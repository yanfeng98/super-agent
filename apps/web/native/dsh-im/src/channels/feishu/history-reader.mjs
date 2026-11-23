import { readNearbyHistory } from './nearby-history.mjs';
import { deletedHistoryEvent, MESSAGE_TYPES, mergedForwardText, normalizeExternalText, senderMatches } from './external-consumer.mjs';

function refuse(code) { throw Object.assign(new Error(code), { code }); }
function identifier(value) {
  return typeof value === 'string' && value.length > 0 && value.length <= 512;
}

function check(response) {
  if ([230027, 99991672, 99991679].includes(response?.code)) refuse('history-permission-denied');
  if (response?.code) refuse('history-unavailable');
}

/** Feishu's page cap for a message listing; a context read and a backfill page share it. */
const HISTORY_LIMIT = 50;

async function checkedSdkRead(operation, signal) {
  let response;
  try { response = await operation(); }
  catch (error) {
    signal?.throwIfAborted();
    if (error?.name === 'AbortError') throw error;
    if (error?.code === 'ABORT_ERR' || error?.code === 'ERR_CANCELED') refuse('cancelled');
    // The SDK rejects HTTP 403 before returning the provider's JSON payload.
    // Never expose its raw Axios error (which can contain credential headers).
    check(error?.response?.data);
    refuse('history-unavailable');
  }
  signal?.throwIfAborted();
  check(response);
  return response;
}

/**
 * True when a fetched message is the one a retained route names: same chat, same author, same topic links.
 * A deleted message never matches, so a route to it stops resolving.
 *
 * Feishu opens a topic on a message after it was sent — a reply inside the Thread, including the bot's own, does it —
 * so a route captured before that keeps naming the same message even though the message now reports a `thread_id`.
 * Only that one-way transition is tolerated, and only when the message became the topic's own root; a route that names a
 * different topic, or a route without one for a message inside someone else's topic, still fails.
 */
export function anchorMatches(source, route) {
  if (!source || source.deleted === true || source.chat_id !== route.conversationId
    || !senderMatches(source, route.actorId)) return false;
  const threadId = source.thread_id || undefined;
  const rootId = source.root_id || undefined;
  const parentId = source.parent_id || undefined;
  // Feishu opens a topic on a message after it was sent — a reply inside the Thread, including the bot's own, does it.
  // A route captured before that still names the same message as long as the message became that topic's own root.
  const topicOpened = threadId !== undefined && route.threadId === undefined && route.rootId === undefined
    && (rootId === undefined || rootId === source.message_id);
  if (topicOpened) return parentId === route.parentId;
  return threadId === route.threadId && rootId === route.rootId && parentId === route.parentId;
}

/**
 * Reads the sub-messages of one merged forward. Feishu returns only a fixed placeholder body for the message itself, so
 * the contents are the items of a second read that name it as their `upper_message_id`.
 */
export async function readMergedForward(client, messageId, signal) {
  if (!identifier(messageId)) return [];
  const response = await checkedSdkRead(() => client.im.v1.message.get({
    path: { message_id: messageId }, params: { with_sender_name: true },
  }, { signal }), signal);
  const items = Array.isArray(response?.data?.items) ? response.data.items : [];
  const linked = items.filter(item => item?.upper_message_id === messageId);
  if (linked.length) return linked;
  // An item linked to another upper message belongs to that one, not to this forward; only an unlinked response is taken.
  if (items.some(item => item?.message_id !== messageId && item?.upper_message_id !== undefined)) return [];
  return items.filter(item => item?.message_id !== messageId);
}

/** Replaces a merged forward's placeholder body with its sub-messages; an unreadable one keeps the placeholder. */
async function expandMergedForward(client, item, signal) {
  if (item?.msg_type !== 'merge_forward') return item;
  let items;
  try { items = await readMergedForward(client, item.message_id, signal); }
  catch { signal?.throwIfAborted(); return item; }
  const text = mergedForwardText(items);
  return text ? { ...item, body: { ...(item.body ?? {}), content: JSON.stringify({ text }) } } : item;
}

// Application-defined checked read: destination is derived from an authenticated source.
// Lark has Chat/time-range listing and Thread listing, not a native around-message API.
export async function readExternalHistory(client, identity, route, query, signal) {
  if (!route || !identifier(route.messageId) || !identifier(route.conversationId)
    || !identifier(route.actorId) || ['threadId', 'rootId', 'parentId'].some(key => route[key] !== undefined && !identifier(route[key])) || !['group', 'nearby', 'thread'].includes(query?.scope)
    || !Number.isInteger(query.limit) || query.limit < 1 || query.limit > HISTORY_LIMIT
    || (query.includeBots !== undefined && typeof query.includeBots !== 'boolean')
    || (query.includeDeleted !== undefined && typeof query.includeDeleted !== 'boolean')
    || (query.cursor !== undefined && (typeof query.cursor !== 'string' || !query.cursor || query.cursor.length > 4096)))
    refuse('bad-request');
  if (query.scope === 'nearby' && [query.beforeCount ?? 10, query.afterCount ?? 5]
    .some(value => !Number.isInteger(value) || value < 0 || value > 20)) refuse('bad-request');
  signal?.throwIfAborted();
  const reference = await checkedSdkRead(() => client.im.v1.message.get({ path: { message_id: route.messageId }, params: { with_sender_name: true } }, { signal }), signal);
  signal?.throwIfAborted();
  const source = (Array.isArray(reference?.data?.items) ? reference.data.items : []).find(item => item?.message_id === route.messageId);
  if (!anchorMatches(source, route)) refuse('stale-route');
  if (query.scope === 'thread' && !route.threadId) refuse('thread-unavailable');
  signal?.throwIfAborted();
  const seconds = Math.floor(Number(source.create_time) / 1000);
  if (!Number.isFinite(seconds) || seconds <= 0) refuse('stale-route');
  const readPage = async params => {
    const response = await checkedSdkRead(() => client.im.v1.message.list({ params }, { signal }), signal);
    signal?.throwIfAborted();
    const items = response?.data?.items;
    if (!Array.isArray(items) || items.length > query.limit || typeof response.data.has_more !== 'boolean')
      refuse('history-unavailable');
    const events = [];
    let omitted = 0;
    for (const item of items) {
      if (!item || item.chat_id !== route.conversationId
        || (query.scope === 'thread' && item.thread_id !== route.threadId)) refuse('untrusted-source');
      const senderVisible = item.sender?.sender_type === 'user'
        || (query.includeBots === true && ['app', 'bot'].includes(item.sender?.sender_type));
      if (item.deleted) {
        // A context read shows a recalled message in place; a read that did not ask for it counts it as omitted.
        const recalled = senderVisible && query.includeDeleted === true
          ? deletedHistoryEvent(item, identity, { chatId: route.conversationId }) : undefined;
        if (!recalled) { ++omitted; continue; }
        events.push(recalled);
        continue;
      }
      if (!MESSAGE_TYPES.includes(item.msg_type) || !senderVisible) { ++omitted; continue; }
      if (item.mentions !== undefined && (!Array.isArray(item.mentions) || item.mentions.length > 100)) { ++omitted; continue; }
      let event;
      try { event = historyEvent(await expandMergedForward(client, item, signal), identity,
        { chatId: route.conversationId, includeBots: query.includeBots === true }); } catch (error) {
        if (error?.code !== 'invalid-inbound') throw error;
        ++omitted; continue;
      }
      if (!event) { ++omitted; continue; }
      events.push(event);
    }
    const nextCursor = response.data.has_more ? response.data.page_token : undefined;
    if (response.data.has_more && (typeof nextCursor !== 'string' || !nextCursor || nextCursor.length > 4096))
      refuse('history-unavailable');
    return {events, omitted, hasMore:response.data.has_more, ...(nextCursor ? {nextCursor} : {})};
  };
  if (query.scope === 'nearby') return readNearbyHistory(readPage, route, query, Number(source.create_time));
  if (query.beforeCount !== undefined || query.afterCount !== undefined) refuse('bad-request');
  const page = await readPage({container_id_type:query.scope === 'thread' ? 'thread' : 'chat',
    container_id:query.scope === 'thread' ? route.threadId : route.conversationId,
    sort_type:'ByCreateTimeDesc', page_size:query.limit, with_sender_name:true,
    ...(query.cursor === undefined ? {} : {page_token:query.cursor})});
  return {version:1, scope:query.scope, ...page, coverage:'provider-visible-human-text'};
}

/** Builds consumer evidence from one history item; `includeBots` admits bot-authored messages. */
function historyEvent(item, identity, { chatId, threadId, direct = false, includeBots = false }) {
  if (!item || item.chat_id !== chatId || (threadId !== undefined && item.thread_id !== threadId))
    refuse('untrusted-source');
  return normalizeExternalText({
    event_id: `history:${item.message_id}`,
    sender: { sender_type: item.sender?.sender_type, sender_id: { open_id: item.sender?.id }, sender_name: item.sender?.sender_name },
    message: { ...item, chat_type: direct ? 'p2p' : 'group', message_type: item.msg_type, content: item.body?.content,
      mentions: (item.mentions ?? []).map(mention => ({ ...mention, id: { open_id: mention?.id } })) },
  }, identity, { includeBots });
}

function historyQuery(query) {
  if (!query || typeof query !== 'object') refuse('bad-request');
  const { since, until, cursor, includeBots, direct, includeSelf, includeDeleted } = query;
  const limit = query.limit;
  if (!Number.isInteger(limit) || limit < 1 || limit > HISTORY_LIMIT) refuse('bad-request');
  for (const value of [since, until]) {
    if (value !== undefined && (typeof value !== 'string' || !Number.isFinite(Date.parse(value)))) refuse('bad-request');
  }
  if (cursor !== undefined && (typeof cursor !== 'string' || !cursor || cursor.length > 4096)) refuse('bad-request');
  for (const value of [includeBots, direct, includeSelf, includeDeleted]) {
    if (value !== undefined && typeof value !== 'boolean') refuse('bad-request');
  }
  return { since, until, cursor, includeBots: includeBots === true, direct: direct === true,
    includeSelf: includeSelf === true, includeDeleted: includeDeleted === true, limit };
}

async function readHistoryPage(client, identity, options, signal) {
  const { containerIdType, containerId, chatId, threadId, direct, includeBots, includeSelf, includeDeleted,
    limit, since, until, cursor } = options;
  const params = {
    container_id_type: containerIdType, container_id: containerId, sort_type: 'ByCreateTimeAsc',
    page_size: limit, with_sender_name: true,
    ...(since ? { start_time: String(Math.floor(Date.parse(since) / 1000)) } : {}),
    ...(until ? { end_time: String(Math.ceil(Date.parse(until) / 1000)) } : {}),
    ...(cursor ? { page_token: cursor } : {}),
  };
  const response = await checkedSdkRead(() => client.im.v1.message.list({ params }, { signal }), signal);
  const items = response?.data?.items;
  if (!Array.isArray(items) || items.length > limit || typeof response.data.has_more !== 'boolean')
    refuse('history-unavailable');
  const events = [];
  let omitted = 0;
  for (const item of items) {
    if (!item || item.chat_id !== chatId || (threadId !== undefined && item.thread_id !== threadId)) refuse('untrusted-source');
    // A bot's own messages are never inbound work for its consumer, so a backfill skips them without counting them as
    // unreadable; a context read asks for them, since its reader shows what the bot itself said.
    const senderId = item.sender?.id;
    if (includeSelf !== true && senderId && ((senderId === identity.appId && item.sender?.sender_type === 'app')
      || (senderId === identity.botOpenId && item.sender?.sender_type === 'bot'))) continue;
    const senderVisible = item.sender?.sender_type === 'user'
      || (includeBots === true && ['app', 'bot'].includes(item.sender?.sender_type));
    if (item.deleted) {
      // A recalled message keeps its place in a context window; a backfill skips it, since there is no body to receive.
      const recalled = senderVisible && includeDeleted === true
        ? deletedHistoryEvent(item, identity, { chatId, direct }) : undefined;
      if (!recalled) { ++omitted; continue; }
      events.push(recalled);
      continue;
    }
    if (!MESSAGE_TYPES.includes(item.msg_type) || !senderVisible) { ++omitted; continue; }
    let event;
    try { event = historyEvent(await expandMergedForward(client, item, signal), identity, { chatId, threadId, direct, includeBots }); } catch (error) {
      if (error?.code !== 'invalid-inbound') throw error;
      ++omitted; continue;
    }
    if (!event) { ++omitted; continue; }
    events.push(event);
  }
  const nextCursor = response.data.has_more ? response.data.page_token : undefined;
  if (response.data.has_more && (typeof nextCursor !== 'string' || !nextCursor || nextCursor.length > 4096))
    refuse('history-unavailable');
  return { events, omitted, hasMore: response.data.has_more, ...(nextCursor ? { nextCursor } : {}) };
}

/**
 * Reads one chat's messages between two times, oldest first, with an explicit continuation cursor.
 * `includeSelf` keeps the bot's own messages, which a backfill always skips; `includeDeleted` keeps a recalled message as
 * a bodiless, `deleted` event instead of counting it as omitted. A bot-authored message still needs `includeBots`.
 */
export async function readChatHistory(client, identity, query, signal) {
  const chatId = typeof query?.chatId === 'string' && identifier(query.chatId) ? query.chatId : undefined;
  if (!chatId) refuse('bad-request');
  const options = historyQuery(query);
  const page = await readHistoryPage(client, identity, { containerIdType: 'chat', containerId: chatId, chatId, ...options }, signal);
  return { version: 1, scope: 'chat', chatId, ...page, coverage: 'provider-visible-messages' };
}

/**
 * Reads one topic thread's messages between two times, oldest first, with an explicit continuation cursor.
 * `includeSelf` keeps the bot's own messages, which a backfill always skips; `includeDeleted` keeps a recalled message as
 * a bodiless, `deleted` event instead of counting it as omitted. A bot-authored message still needs `includeBots`.
 */
export async function readThreadHistory(client, identity, query, signal) {
  const threadId = typeof query?.threadId === 'string' && identifier(query.threadId) ? query.threadId : undefined;
  const chatId = typeof query?.chatId === 'string' && identifier(query.chatId) ? query.chatId : undefined;
  if (!threadId || !chatId) refuse('bad-request');
  const options = historyQuery(query);
  const page = await readHistoryPage(client, identity,
    { containerIdType: 'thread', containerId: threadId, chatId, threadId, ...options }, signal);
  return { version: 1, scope: 'thread', chatId, threadId, ...page, coverage: 'provider-visible-messages' };
}

/**
 * Reads one message by its retained route, including the AppLink that opens it in Feishu. `targetId` reads another
 * message of the same conversation, such as the message a reply answers: the route must still resolve to its own
 * message, and the target must belong to the same conversation.
 */
export async function readExternalMessage(client, identity, route, signal, targetId) {
  if (!route || !identifier(route.messageId) || !identifier(route.conversationId) || !identifier(route.actorId)
    || (targetId !== undefined && !identifier(targetId)))
    refuse('bad-request');
  const fetch = async id => {
    const reference = await checkedSdkRead(() => client.im.v1.message.get({
      path: { message_id: id }, params: { with_sender_name: true },
    }, { signal }), signal);
    return (Array.isArray(reference?.data?.items) ? reference.data.items : [])
      .find(entry => entry?.message_id === id);
  };
  // A target read still proves the retained route: a fabricated route must not reach any message of a chat.
  if (targetId !== undefined && !anchorMatches(await fetch(route.messageId), route)) refuse('stale-route');
  const wanted = targetId ?? route.messageId;
  const item = await fetch(wanted);
  if (!item) refuse('message-unavailable');
  // The message must belong to the routed conversation before its state is reported, so a caller cannot learn whether
  // a message of another chat exists.
  if (item.chat_id !== route.conversationId
    || (targetId === undefined && item.sender?.id !== route.actorId)) refuse('stale-route');
  // A recalled message keeps its record and loses its body, so a consumer reports it apart from an unreadable one.
  if (item.deleted) refuse('message-deleted');
  const event = historyEvent(await expandMergedForward(client, item, signal), identity, { chatId: route.conversationId, includeBots: true });
  if (!event) refuse('message-unavailable');
  return { version: 1, messageId: event.messageId, chatId: event.conversation.id, at: event.at, text: event.text,
    messageType: event.messageType, images: event.images, sender: event.actor,
    ...(event.reply.threadId ? { threadId: event.reply.threadId } : {}),
    ...(event.reply.rootId ? { rootId: event.reply.rootId } : {}),
    ...(event.reply.parentId ? { parentId: event.reply.parentId } : {}),
    ...(identifier(item.message_app_link) ? { appLink: item.message_app_link } : {}), deleted: false };
}

/** Reads one chat's member list as open_id plus name, following pagination to the end. */
export async function readChatMembers(client, chatId, signal) {
  if (!identifier(chatId)) refuse('bad-request');
  const users = [];
  let pageToken;
  let pages = 0;
  do {
    const response = await checkedSdkRead(() => client.im.v1.chatMembers.get({
      path: { chat_id: chatId },
      params: { member_id_type: 'open_id', page_size: 100, ...(pageToken ? { page_token: pageToken } : {}) },
    }, { signal }), signal);
    const items = response?.data?.items;
    if (!Array.isArray(items) || typeof response.data.has_more !== 'boolean') refuse('members-unavailable');
    for (const item of items) {
      const id = typeof item?.member_id === 'string' && identifier(item.member_id) ? item.member_id : undefined;
      if (id) users.push({ id, ...(typeof item.name === 'string' && identifier(item.name) ? { name: item.name } : {}) });
    }
    pageToken = response.data.has_more && typeof response.data.page_token === 'string'
      && identifier(response.data.page_token) ? response.data.page_token : undefined;
    if (response.data.has_more && !pageToken) refuse('members-unavailable');
    if (++pages > 50) refuse('members-unavailable');
  } while (pageToken);
  return { version: 1, chatId, users };
}

/** Reads the chats the bot has joined as chat_id plus name, following pagination to the end. */
export async function readChatList(client, signal) {
  const chats = [];
  let pageToken;
  let pages = 0;
  do {
    const response = await checkedSdkRead(() => client.im.v1.chat.list({
      params: { page_size: 100, user_id_type: 'open_id', ...(pageToken ? { page_token: pageToken } : {}) },
    }, { signal }), signal);
    const items = response?.data?.items;
    if (!Array.isArray(items) || typeof response.data.has_more !== 'boolean') refuse('chats-unavailable');
    for (const item of items) {
      const chatId = typeof item?.chat_id === 'string' && identifier(item.chat_id) ? item.chat_id : undefined;
      if (!chatId) continue;
      const name = typeof item.name === 'string' && item.name.trim() ? item.name.trim().slice(0, 200) : undefined;
      chats.push({ chatId, ...(name ? { name } : {}) });
    }
    pageToken = response.data.has_more && typeof response.data.page_token === 'string'
      && identifier(response.data.page_token) ? response.data.page_token : undefined;
    if (response.data.has_more && !pageToken) refuse('chats-unavailable');
    if (++pages > 50) refuse('chats-unavailable');
  } while (pageToken);
  return { version: 1, chats };
}

import { readNearbyHistory } from './nearby-history.mjs';
import { MESSAGE_TYPES, normalizeExternalText, senderMatches } from './external-consumer.mjs';

function refuse(code) { throw Object.assign(new Error(code), { code }); }
function identifier(value) {
  return typeof value === 'string' && value.length > 0 && value.length <= 512;
}

function check(response) {
  if ([230027, 99991672, 99991679].includes(response?.code)) refuse('history-permission-denied');
  if (response?.code) refuse('history-unavailable');
}

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

// Application-defined checked read: destination is derived from an authenticated source.
// Lark has Chat/time-range listing and Thread listing, not a native around-message API.
export async function readExternalHistory(client, identity, route, query, signal) {
  if (!route || !identifier(route.messageId) || !identifier(route.conversationId)
    || !identifier(route.actorId) || ['threadId', 'rootId', 'parentId'].some(key => route[key] !== undefined && !identifier(route[key])) || !['group', 'nearby', 'thread'].includes(query?.scope)
    || !Number.isInteger(query.limit) || query.limit < 1 || query.limit > 20
    || (query.includeBots !== undefined && typeof query.includeBots !== 'boolean')
    || (query.cursor !== undefined && (typeof query.cursor !== 'string' || !query.cursor || query.cursor.length > 4096)))
    refuse('bad-request');
  if (query.scope === 'nearby' && [query.beforeCount ?? 10, query.afterCount ?? 5]
    .some(value => !Number.isInteger(value) || value < 0 || value > 20)) refuse('bad-request');
  signal?.throwIfAborted();
  const reference = await checkedSdkRead(() => client.im.v1.message.get({ path: { message_id: route.messageId }, params: { with_sender_name: true } }, { signal }), signal);
  signal?.throwIfAborted();
  const source = (Array.isArray(reference?.data?.items) ? reference.data.items : []).find(item => item?.message_id === route.messageId);
  if (!source || source.deleted || source.chat_id !== route.conversationId
    || !senderMatches(source, route.actorId) || (source.thread_id || undefined) !== route.threadId
    || (source.root_id || undefined) !== route.rootId || (source.parent_id || undefined) !== route.parentId)
    refuse('stale-route');
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
      if (item.deleted || !MESSAGE_TYPES.includes(item.msg_type)
        || (item.sender?.sender_type !== 'user'
          && !(query.includeBots === true && ['app', 'bot'].includes(item.sender?.sender_type)))) { ++omitted; continue; }
      if (item.mentions !== undefined && (!Array.isArray(item.mentions) || item.mentions.length > 100)) { ++omitted; continue; }
      let event;
      try { event = historyEvent(item, identity, { chatId: route.conversationId, includeBots: query.includeBots === true }); } catch (error) {
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

const HISTORY_LIMIT = 50;

function historyQuery(query) {
  if (!query || typeof query !== 'object') refuse('bad-request');
  const { since, until, cursor, includeBots, direct } = query;
  const limit = query.limit;
  if (!Number.isInteger(limit) || limit < 1 || limit > HISTORY_LIMIT) refuse('bad-request');
  for (const value of [since, until]) {
    if (value !== undefined && (typeof value !== 'string' || !Number.isFinite(Date.parse(value)))) refuse('bad-request');
  }
  if (cursor !== undefined && (typeof cursor !== 'string' || !cursor || cursor.length > 4096)) refuse('bad-request');
  if (includeBots !== undefined && typeof includeBots !== 'boolean') refuse('bad-request');
  if (direct !== undefined && typeof direct !== 'boolean') refuse('bad-request');
  return { since, until, cursor, includeBots: includeBots === true, direct: direct === true, limit };
}

async function readHistoryPage(client, identity, options, signal) {
  const { containerIdType, containerId, chatId, threadId, direct, includeBots, limit, since, until, cursor } = options;
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
    if (item.deleted || !MESSAGE_TYPES.includes(item.msg_type)
      || (item.sender?.sender_type !== 'user'
        && !(includeBots && ['app', 'bot'].includes(item.sender?.sender_type)))) { ++omitted; continue; }
    let event;
    try { event = historyEvent(item, identity, { chatId, threadId, direct, includeBots }); } catch (error) {
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

/** Reads one chat's messages between two times, oldest first, with an explicit continuation cursor. */
export async function readChatHistory(client, identity, query, signal) {
  const chatId = typeof query?.chatId === 'string' && identifier(query.chatId) ? query.chatId : undefined;
  if (!chatId) refuse('bad-request');
  const options = historyQuery(query);
  const page = await readHistoryPage(client, identity, { containerIdType: 'chat', containerId: chatId, chatId, ...options }, signal);
  return { version: 1, scope: 'chat', chatId, ...page, coverage: 'provider-visible-messages' };
}

/** Reads one topic thread's messages between two times, oldest first, with an explicit continuation cursor. */
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
 * message of the same conversation, such as the message a reply answers; the retained route still proves the caller may
 * read that conversation, and the target must belong to it.
 */
export async function readExternalMessage(client, identity, route, signal, targetId) {
  if (!route || !identifier(route.messageId) || !identifier(route.conversationId) || !identifier(route.actorId)
    || (targetId !== undefined && !identifier(targetId)))
    refuse('bad-request');
  const wanted = targetId ?? route.messageId;
  const reference = await checkedSdkRead(() => client.im.v1.message.get({
    path: { message_id: wanted }, params: { with_sender_name: true },
  }, { signal }), signal);
  const item = (Array.isArray(reference?.data?.items) ? reference.data.items : [])
    .find(entry => entry?.message_id === wanted);
  if (!item || item.deleted || item.chat_id !== route.conversationId
    || (wanted === route.messageId && item.sender?.id !== route.actorId)) refuse('stale-route');
  const event = historyEvent(item, identity, { chatId: route.conversationId, includeBots: true });
  if (!event) refuse('message-unavailable');
  return { version: 1, messageId: event.messageId, chatId: event.conversation.id, at: event.at, text: event.text,
    messageType: event.messageType, images: event.images, sender: event.actor,
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

function refusal(code) {
  const error = new Error(code);
  error.code = code;
  return error;
}

function identifier(value) {
  return typeof value === 'string' && value.length > 0 && value.length <= 512 ? value : undefined;
}

const TEXT_LIMIT = 16000;
const TRUNCATED = '\n[内容过长，已截断]';

/** Keeps a body bounded: a message longer than the limit keeps its beginning and says it was cut, instead of vanishing. */
function bounded(text) {
  if (text.length <= TEXT_LIMIT) return text;
  let cut = text.slice(0, TEXT_LIMIT);
  // The cut must not split a surrogate pair, or the kept text ends with half an emoji.
  if (/[\uD800-\uDBFF]$/.test(cut)) cut = cut.slice(0, -1);
  return `${cut}${TRUNCATED}`;
}

/**
 * Message types a consumer receives. Text, post and image bodies carry model-readable content; the rest are rendered as a
 * one-line placeholder that names the resource and its key, so a consumer knows the message exists and can read the
 * resource back instead of the message vanishing from history.
 */
export const MESSAGE_TYPES = Object.freeze([
  'text', 'post', 'image', 'file', 'audio', 'media', 'sticker',
  'interactive', 'share_chat', 'share_user', 'merge_forward',
]);

/** Human-readable rich-text values are unbounded; `identifier` bounds ids, not paragraphs. */
const textValue = value => (typeof value === 'string' && value.length > 0 ? value : undefined);

/** Renders one Feishu rich-text (post) element into plain text; image elements also report their keys. */
function renderPostElement(element, images, mentioned) {
  if (!element || typeof element !== 'object') return '';
  if (element.tag === 'at') {
    const id = identifier(element.user_id) ?? identifier(element.open_id);
    if (id) mentioned.push(id);
    const name = identifier(element.user_name) ?? identifier(element.user_id);
    return name ? `@${name}` : '@';
  }
  if (element.tag === 'img') {
    const key = identifier(element.image_key);
    if (!key) return '[图片]';
    images.push(key);
    // Keep the key beside the placeholder so a consumer can read the image back from the message.
    return `[图片] ${key}`;
  }
  if (element.tag === 'a') return [textValue(element.text), identifier(element.href)].filter(Boolean).join(' ');
  if (element.tag === 'code_block' || element.tag === 'text' || element.tag === 'md') return textValue(element.text) ?? '';
  if (element.tag === 'emotion') return '[表情]';
  return '';
}

/** Renders a Feishu post body (localized or flat) into text plus the image keys it references. */
function renderPost(content) {
  let body;
  try { body = JSON.parse(content); } catch { return undefined; }
  if (!body || typeof body !== 'object') return undefined;
  const localized = body.zh_cn ?? body.en_us ?? body.ja_jp
    ?? Object.values(body).find(value => value && typeof value === 'object' && Array.isArray(value.content));
  const post = localized && typeof localized === 'object' && Array.isArray(localized.content) ? localized : body;
  if (!Array.isArray(post.content)) return undefined;
  const images = [];
  const mentioned = [];
  const lines = [];
  if (textValue(post.title)) lines.push(post.title);
  for (const paragraph of post.content) {
    if (!Array.isArray(paragraph)) continue;
    lines.push(paragraph.map(element => renderPostElement(element, images, mentioned)).join(''));
  }
  const text = lines.join('\n').trim();
  return { text: text || (images.length ? '[图片]' : ''), images, mentioned };
}

const CARD_TEXT_KEYS = new Set(['text', 'content', 'title']);
/** Collects the human-readable strings of a card body; other card fields are layout and styling. */
function cardText(value, depth = 0, found = []) {
  if (depth > 8 || found.length > 200) return found;
  if (typeof value === 'string') {
    if (value.trim()) found.push(value.trim());
    return found;
  }
  if (Array.isArray(value)) {
    for (const item of value) cardText(item, depth + 1, found);
    return found;
  }
  if (!value || typeof value !== 'object') return found;
  for (const [key, entry] of Object.entries(value)) {
    if (typeof entry === 'string') {
      if (CARD_TEXT_KEYS.has(key) && entry.trim()) found.push(entry.trim());
    } else cardText(entry, depth + 1, found);
  }
  return found;
}

/** Renders a resource, card, or shared-contact body into one line that names its type and key. */
function resourceBody(messageType, body) {
  const fileKey = identifier(body.file_key);
  const fileName = identifier(body.file_name) ?? identifier(body.name);
  switch (messageType) {
    case 'file': return fileKey ? `[文件] ${fileName ?? fileKey} ${fileKey}` : undefined;
    case 'audio': {
      const seconds = Math.round(Number(body.duration) / 1000);
      const length = Number.isFinite(seconds) && seconds > 0 ? `${seconds}s ` : '';
      return fileKey ? `[语音] ${length}${fileKey}` : undefined;
    }
    case 'media': return fileKey ? `[视频] ${fileName ?? fileKey} ${fileKey}` : undefined;
    case 'sticker': return fileKey ? `[表情] ${fileKey}` : '[表情]';
    case 'interactive': {
      const text = cardText(body).join(' ').replace(/\s+/g, ' ').trim().slice(0, 1000);
      return text ? `[卡片] ${text}` : '[卡片]';
    }
    case 'share_chat': return identifier(body.chat_id) ? `[群名片] ${body.chat_id}` : '[群名片]';
    case 'share_user': return identifier(body.user_id) ? `[个人名片] ${body.user_id}` : '[个人名片]';
    case 'merge_forward': return '[合并转发]';
    default: return undefined;
  }
}

/** Reads the text, message type and image keys of one Feishu message body. */
function messageBody(messageType, content) {
  let body;
  try { body = JSON.parse(content); } catch { throw refusal('invalid-inbound'); }
  if (!body || typeof body !== 'object') throw refusal('invalid-inbound');
  if (messageType === 'text') {
    if (typeof body.text !== 'string' || !body.text.trim()) throw refusal('invalid-inbound');
    return { text: bounded(body.text), images: [], mentioned: [] };
  }
  if (messageType === 'image') {
    const key = identifier(body.image_key);
    if (!key) throw refusal('invalid-inbound');
    return { text: `[图片] ${key}`, images: [key], mentioned: [] };
  }
  if (messageType === 'post') {
    const rendered = renderPost(content);
    if (!rendered || !rendered.text) throw refusal('invalid-inbound');
    return { text: bounded(rendered.text), images: rendered.images, mentioned: rendered.mentioned };
  }
  // A merged forward arrives with its fixed placeholder body; a consumer that read the sub-messages replaces it with their text.
  if (messageType === 'merge_forward' && typeof body.text === 'string' && body.text.trim())
    return { text: bounded(body.text), images: [], mentioned: [] };
  const text = resourceBody(messageType, body);
  if (!text) throw refusal('invalid-inbound');
  return { text: bounded(text), images: [], mentioned: [] };
}

/**
 * Renders the sub-messages of a merged forward as `sender: text` lines. Feishu returns only a fixed placeholder body
 * for a merged forward, so the contents come from a second read of the same message; an unreadable sub-message is skipped.
 */
export function mergedForwardText(items) {
  const lines = [];
  let length = 0;
  for (const item of Array.isArray(items) ? items : []) {
    if (!item || item.deleted === true) continue;
    let text = '';
    try {
      const body = JSON.parse(item.body?.content ?? '');
      if (item.msg_type === 'text') text = typeof body?.text === 'string' ? body.text : '';
      else if (item.msg_type === 'post') text = renderPost(item.body.content)?.text ?? '';
      else if (item.msg_type === 'image') text = `[图片] ${identifier(body?.image_key) ?? ''}`.trim();
      else if (MESSAGE_TYPES.includes(item.msg_type)) text = resourceBody(item.msg_type, body) ?? '';
    } catch { continue; }
    if (!text) continue;
    const sender = identifier(item.sender_name) ?? identifier(item.sender?.sender_name) ?? '';
    const line = sender ? `${sender}: ${text}` : text;
    if (length + line.length > TEXT_LIMIT) break;
    lines.push(line);
    length += line.length + 1;
  }
  return lines.join('\n').trim();
}

/**
 * Whether a live event was authored by the handing-over bot itself. Its mirrored Session turns and its replies carry new
 * message ids, so the consumer's trigger-id dedupe cannot catch them; the live ingress drops them instead.
 */
export function isOwnBotEvent(event, { appId, botOpenId }) {
  const senderType = event?.sender?.sender_type;
  if (senderType !== 'app' && senderType !== 'bot') return false;
  const id = identifier(event?.sender?.sender_id?.open_id) ?? identifier(event?.sender?.sender_id?.app_id);
  return !!id && (id === appId || id === botOpenId);
}

/**
 * Normalizes one Feishu event into the exclusive consumer's evidence. Human text is the default; `includeBots`
 * also admits bot-authored messages, and post/image bodies are rendered so a consumer receives text plus image keys.
 * The runtime adds `voiceTranscript` to an audio event once the channel's voice capability transcribed it: that text
 * replaces the placeholder naming the audio resource, while the message type and the provider body stay unchanged.
 */
export function normalizeExternalText(event, { botId, appId, botOpenId, fingerprint }, { includeBots = false } = {}) {
  const message = event?.message;
  const senderType = event?.sender?.sender_type;
  const actorId = identifier(event?.sender?.sender_id?.open_id)
    ?? (includeBots && (senderType === 'app' || senderType === 'bot')
      ? identifier(event?.sender?.sender_id?.app_id) : undefined);
  const messageId = identifier(message?.message_id);
  const conversationId = identifier(message?.chat_id);
  const eventId = identifier(event?.event_id ?? event?.header?.event_id);
  const eventApp = event?.app_id ?? event?.header?.app_id;
  if (eventApp !== undefined && eventApp !== appId) throw refusal('account-changed');
  if (senderType !== 'user' && !(includeBots && (senderType === 'app' || senderType === 'bot'))) return null;
  if (!['p2p', 'group'].includes(message?.chat_type)
    || !MESSAGE_TYPES.includes(message?.message_type)) return null;
  if (message.chat_type === 'group' && !identifier(botOpenId)) throw refusal('account-unverified');
  if (message.mentions !== undefined && (!Array.isArray(message.mentions) || message.mentions.length > 100))
    throw refusal('invalid-inbound');
  const mentions = Object.freeze((message.mentions ?? []).map(mention => {
    const id = identifier(mention?.id?.open_id ?? mention?.open_id);
    const key = identifier(mention?.key);
    if (!id || !key) throw refusal('invalid-inbound');
    return Object.freeze({ id, key, ...(identifier(mention.name) ? { name: mention.name } : {}) });
  }));
  if (!actorId || !messageId || !conversationId || !eventId
    || !/^[a-f0-9]{64}$/.test(fingerprint ?? '')) throw refusal('invalid-inbound');
  const body = messageBody(message.message_type, message.content);
  // A voice message carries no readable text of its own, so the runtime transcribes it first and hands the words over
  // here; without a transcript the placeholder that names the audio resource stays the body.
  const transcript = message.message_type === 'audio' ? textValue(event?.voiceTranscript)?.trim() : undefined;
  const milliseconds = Number(message.create_time);
  const timestamp = new Date(milliseconds);
  if (!Number.isFinite(milliseconds) || milliseconds <= 0 || !Number.isFinite(timestamp.getTime()))
    throw refusal('invalid-inbound');
  const route = Object.freeze({
    messageId, conversationId, actorId,
    ...(identifier(message.thread_id) ? { threadId: message.thread_id } : {}),
    ...(identifier(message.root_id) ? { rootId: message.root_id } : {}),
    ...(identifier(message.parent_id) ? { parentId: message.parent_id } : {}),
  });
  return Object.freeze({
    version: 1, channel: 'feishu', botId, fingerprint, eventId, messageId,
    actor: Object.freeze({ kind: senderType === 'user' ? 'user' : 'bot', id: actorId,
      ...(identifier(event.sender.sender_name) ? { name: event.sender.sender_name } : {}) }),
    conversation: Object.freeze({ kind: message.chat_type === 'group' ? 'group' : 'dm', id: conversationId }),
    mentions, mentionedAccount: mentions.some(mention => mention.id === botOpenId)
      || body.mentioned.includes(botOpenId),
    messageType: message.message_type, images: Object.freeze(body.images),
    at: timestamp.toISOString(), text: transcript ? bounded(transcript) : body.text, reply: route,
    ...(identifier(message.message_app_link) ? { appLink: message.message_app_link } : {}),
    replay: Object.freeze({ kind: 'provider-redelivery', resumeCursor: false, gapPossible: true }),
  });
}

/**
 * Builds the event a context read keeps for a recalled message: it names the sender and the time, carries the reply
 * route, and has an empty `text` plus `deleted: true`, so a reader shows that a message was withdrawn instead of
 * dropping it from the window. Returns undefined when the provider no longer returns the ids an event needs.
 */
export function deletedHistoryEvent(item, identity, { chatId, direct = false } = {}) {
  if (!item || item.chat_id !== chatId) return undefined;
  const senderType = item.sender?.sender_type;
  if (senderType !== 'user' && !['app', 'bot'].includes(senderType)) return undefined;
  const actorId = identifier(item.sender?.id);
  const messageId = identifier(item.message_id);
  const conversationId = identifier(item.chat_id);
  const milliseconds = Number(item.create_time);
  const timestamp = new Date(milliseconds);
  if (!actorId || !messageId || !conversationId || !Number.isFinite(milliseconds) || milliseconds <= 0
    || !Number.isFinite(timestamp.getTime()) || !/^[a-f0-9]{64}$/.test(identity?.fingerprint ?? '')) return undefined;
  const route = Object.freeze({
    messageId, conversationId, actorId,
    ...(identifier(item.thread_id) ? { threadId: item.thread_id } : {}),
    ...(identifier(item.root_id) ? { rootId: item.root_id } : {}),
    ...(identifier(item.parent_id) ? { parentId: item.parent_id } : {}),
  });
  return Object.freeze({
    version: 1, channel: 'feishu', botId: identity.botId, fingerprint: identity.fingerprint,
    eventId: `history:${messageId}`, messageId,
    actor: Object.freeze({ kind: senderType === 'user' ? 'user' : 'bot', id: actorId,
      ...(identifier(item.sender?.sender_name) ? { name: item.sender.sender_name } : {}) }),
    conversation: Object.freeze({ kind: direct ? 'dm' : 'group', id: conversationId }),
    mentions: Object.freeze([]), mentionedAccount: false, images: Object.freeze([]),
    at: timestamp.toISOString(), text: '', reply: route, deleted: true,
    replay: Object.freeze({ kind: 'provider-redelivery', resumeCursor: false, gapPossible: true }),
  });
}

/**
 * Whether one fetched message's sender is the actor a retained route names. Users are matched by `open_id`; app and bot
 * messages carry the app id instead, so only the id is compared.
 */
export function senderMatches(item, actorId) {
  const sender = item?.sender;
  if (!sender || sender.id !== actorId) return false;
  if (sender.sender_type === 'user') return sender.id_type === 'open_id';
  return sender.sender_type === 'app' || sender.sender_type === 'bot';
}

export class ExclusiveInboundConsumers {
  #entries = new Map();

  register(botId, { fingerprint, onEvent, signal, includeBots = false }) {
    if (this.#entries.has(botId)) throw refusal('consumer-conflict');
    if (!/^[a-f0-9]{64}$/.test(fingerprint ?? '') || typeof onEvent !== 'function'
      || typeof includeBots !== 'boolean')
      throw refusal('bad-request');
    signal?.throwIfAborted();
    const controller = new AbortController();
    const entry = { fingerprint, onEvent, controller, includeBots, dispose: undefined };
    const dispose = () => {
      if (this.#entries.get(botId) === entry) this.#entries.delete(botId);
      controller.abort(refusal('consumer-unavailable'));
      signal?.removeEventListener('abort', dispose);
    };
    entry.dispose = dispose;
    this.#entries.set(botId, entry);
    signal?.addEventListener('abort', dispose, { once: true });
    return dispose;
  }

  async accept(botId, evidence, signal) {
    const entry = this.#entries.get(botId);
    if (!entry) throw refusal('consumer-unavailable');
    if (entry.fingerprint !== evidence.fingerprint) throw refusal('account-changed');
    const deliverySignal = signal
      ? AbortSignal.any([signal, entry.controller.signal]) : entry.controller.signal;
    deliverySignal.throwIfAborted();
    let abort;
    const interrupted = new Promise((_, reject) => {
      abort = () => reject(deliverySignal.reason);
      deliverySignal.addEventListener('abort', abort, { once: true });
      if (deliverySignal.aborted) abort();
    });
    let result;
    try {
      result = await Promise.race([entry.onEvent(evidence, { signal: deliverySignal }), interrupted]);
    } finally {
      deliverySignal.removeEventListener('abort', abort);
    }
    deliverySignal.throwIfAborted();
    if (this.#entries.get(botId) !== entry) throw refusal('consumer-unavailable');
    if (result?.accepted !== true) throw refusal('ingress-not-accepted');
    return { accepted: true };
  }

  has(botId) {
    return this.#entries.has(botId);
  }

  /** Whether the registered consumer asked for bot-authored messages too. */
  includeBotsFor(botId) {
    return this.#entries.get(botId)?.includeBots === true;
  }

  signalFor(botId, fingerprint) {
    const entry = this.#entries.get(botId);
    if (!entry) throw refusal('consumer-unavailable');
    if (entry.fingerprint !== fingerprint) throw refusal('account-changed');
    return entry.controller.signal;
  }

  remove(botId) {
    const entry = this.#entries.get(botId);
    entry?.dispose();
  }

  close() {
    for (const botId of this.#entries.keys()) this.remove(botId);
  }
}

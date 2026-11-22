function refusal(code) {
  const error = new Error(code);
  error.code = code;
  return error;
}

function identifier(value) {
  return typeof value === 'string' && value.length > 0 && value.length <= 512 ? value : undefined;
}

const TEXT_LIMIT = 16000;

/** Renders one Feishu rich-text (post) element into plain text; image elements also report their keys. */
function renderPostElement(element, images) {
  if (!element || typeof element !== 'object') return '';
  if (element.tag === 'at') {
    const name = identifier(element.user_name) ?? identifier(element.user_id);
    return name ? `@${name}` : '@';
  }
  if (element.tag === 'img') {
    const key = identifier(element.image_key);
    if (key) images.push(key);
    return '[图片]';
  }
  if (element.tag === 'a') return [element.text, element.href].filter(identifier).join(' ');
  if (element.tag === 'code_block' || element.tag === 'text' || element.tag === 'md') return identifier(element.text) ?? '';
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
  const lines = [];
  if (identifier(post.title)) lines.push(post.title);
  for (const paragraph of post.content) {
    if (!Array.isArray(paragraph)) continue;
    lines.push(paragraph.map(element => renderPostElement(element, images)).join(''));
  }
  const text = lines.join('\n').trim();
  return { text: text || (images.length ? '[图片]' : ''), images };
}

/** Reads the text, message type and image keys of one Feishu message body. */
function messageBody(messageType, content) {
  let body;
  try { body = JSON.parse(content); } catch { throw refusal('invalid-inbound'); }
  if (messageType === 'text') {
    if (typeof body?.text !== 'string' || !body.text.trim() || body.text.length > TEXT_LIMIT) throw refusal('invalid-inbound');
    return { text: body.text, images: [] };
  }
  if (messageType === 'image') {
    const key = identifier(body?.image_key);
    if (!key) throw refusal('invalid-inbound');
    return { text: `[图片] ${key}`, images: [key] };
  }
  const rendered = renderPost(content);
  if (!rendered || !rendered.text || rendered.text.length > TEXT_LIMIT) throw refusal('invalid-inbound');
  return { text: rendered.text, images: rendered.images };
}

/**
 * Normalizes one Feishu event into the exclusive consumer's evidence. Human text is the default; `includeBots`
 * also admits bot-authored messages, and post/image bodies are rendered so a consumer receives text plus image keys.
 */
export function normalizeExternalText(event, { botId, appId, botOpenId, fingerprint }, { includeBots = false } = {}) {
  const message = event?.message;
  const actorId = identifier(event?.sender?.sender_id?.open_id);
  const messageId = identifier(message?.message_id);
  const conversationId = identifier(message?.chat_id);
  const eventId = identifier(event?.event_id ?? event?.header?.event_id);
  const eventApp = event?.app_id ?? event?.header?.app_id;
  if (eventApp !== undefined && eventApp !== appId) throw refusal('account-changed');
  const senderType = event?.sender?.sender_type;
  if (senderType !== 'user' && !(includeBots && (senderType === 'app' || senderType === 'bot'))) return null;
  if (!['p2p', 'group'].includes(message?.chat_type)
    || !['text', 'post', 'image'].includes(message?.message_type)) return null;
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
    mentions, mentionedAccount: mentions.some(mention => mention.id === botOpenId),
    messageType: message.message_type, images: Object.freeze(body.images),
    at: timestamp.toISOString(), text: body.text, reply: route,
    replay: Object.freeze({ kind: 'provider-redelivery', resumeCursor: false, gapPossible: true }),
  });
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

/**
 * Channel-neutral conversation routes plus the single generator of the
 * conversation session keys stored in a channel's state file.
 *
 * The codecs encode the established on-disk shapes so adopting this module
 * never rewrites an existing key: Feishu group chats keep `group:<chatId>` and
 * `group:<chatId>:thread:<threadId>`. Feishu p2p keys are produced by the
 * channel's own `conversationKey` helper until the parity cutover moves them
 * here; the codec therefore refuses to encode a Feishu dm route instead of
 * inventing a second shape.
 */

/**
 * @typedef {Object} ConversationRoute
 * @property {string} channel          Codec selector ('feishu', ...).
 * @property {?string} botId           Bot identity for namespacing; null when the channel key is not bot-scoped.
 * @property {string} chatId           Room / chat id, such as Feishu `chat_id`.
 * @property {?string} senderId        Sender identity, required by dm routes.
 * @property {?string} threadId        Server-side thread root, such as Feishu `thread_id`; null on main-feed routes.
 * @property {'dm'|'group'|'thread'} scope
 */

const FEISHU_CODEC = Object.freeze({
  dm: null,
  group: (route) => (route.threadId ? `group:${route.chatId}:thread:${route.threadId}` : `group:${route.chatId}`),
});

const CODECS = Object.freeze({ feishu: FEISHU_CODEC });

/**
 * Build one frozen conversation route from the channel-neutral parts.
 * @param {Object} parts - route parts; `kind` accepts 'direct'|'dm'|'group'.
 * @returns {ConversationRoute}
 */
export function buildConversationRoute({ channel, botId = null, chatId, senderId = null, threadId = null, kind = 'group' } = {}) {
  if (typeof channel !== 'string' || !channel) throw new TypeError('A conversation route requires a channel');
  if (typeof chatId !== 'string' || !chatId) throw new TypeError('A conversation route requires a chat id');
  const scope = kind === 'direct' || kind === 'dm'
    ? 'dm'
    : (typeof threadId === 'string' && threadId ? 'thread' : 'group');
  return Object.freeze({
    channel,
    botId: typeof botId === 'string' && botId ? botId : null,
    chatId,
    senderId: typeof senderId === 'string' && senderId ? senderId : null,
    threadId: scope === 'thread' ? threadId : null,
    scope,
  });
}

/**
 * The one place conversation session keys are produced.
 * @param {ConversationRoute} route
 * @returns {string} the durable conversation key, e.g. `group:oc_abc:thread:omt_1`.
 */
export function encodeConversationRouteKey(route) {
  const codec = CODECS[route?.channel];
  if (!codec) throw new TypeError(`No conversation key codec is registered for channel "${route?.channel ?? '?'}"`);
  const encode = codec[route.scope === 'dm' ? 'dm' : 'group'];
  if (typeof encode !== 'function') {
    throw new TypeError(`Channel "${route.channel}" cannot encode a ${route.scope} route`);
  }
  return encode(route);
}

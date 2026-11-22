/** Feishu Source backed by dsh-im's exclusive text consumer, with no lark-cli or separate bot credentials. */
import type { Context } from '@deepseek-ai/cordis'
import type { Agent } from '@deepseek-ai/dsh-agent'
import type { ImageAttachmentRef, ImageMediaType } from '@deepseek-ai/dsh-attachment'
import type {} from '@deepseek-ai/dsh-llm'
import { setTimeout as delay } from 'node:timers/promises'
import { z } from 'zod'
import { key, threadKey, type OutboundMessage, type Trigger } from '../core/index.ts'
import { checkedImText } from '../infra/text.ts'
import { admissionImageKeys } from './feishu-image.ts'
import { dshIm, dshImOf, setDshIm, type DshImEvent, type DshImMessage, type DshImService } from './dsh-im.ts'
import { feishuChatLink } from './feishu-channel.ts'
import { withMentionNames } from './feishu-trigger.ts'
import { runFeishuImBackfill } from './feishu-im-loops.ts'
import { KV } from '../kv.ts'
import { sourceContext, type Health, type SourceContext } from './host.ts'
import type { ContextMessage, ContextPart, SourceKind, ThreadContext } from './kind.ts'
import type { SourceRegistry } from './registry.ts'

export const SOURCE = 'feishu'
const POLL_MS = 2_000
let currentConfig: FeishuConfig | undefined
interface CachedBot { botId: string; fingerprint: string; name: string; brand: 'feishu' | 'lark' }
const cachedBots = new Map<string, CachedBot>()
/** Sets the config the Channel uses; the watch loop keeps it current, and tests set it directly. */
export const setFeishuConfig = (value: FeishuConfig | undefined): void => { currentConfig = value }
const HISTORY_LIMIT = 20

const FeishuConfigSchema = z.object({
  botId: z.string().trim().max(128).default(''),
  chats: z.array(z.object({ chatId: z.string().regex(/^oc_[a-zA-Z0-9]+$/), name: z.string().trim().max(60).optional(), listen: z.enum(['mentions', 'all']) }).strict())
    .refine(chats => new Set(chats.map(chat => chat.chatId)).size === chats.length, 'Duplicate chat IDs')
    .default([]),
  directMessages: z.boolean().default(true),
  otherChats: z.enum(['none', 'mentions']).default('none'),
  readPageSize: z.number().int().min(1).max(HISTORY_LIMIT).default(HISTORY_LIMIT),
  linkTimeoutMs: z.number().int().min(1).max(90_000).default(5_000),
})
export type FeishuConfig = z.infer<typeof FeishuConfigSchema>
/** Parses a stored Feishu Source config, ignoring keys the lark-cli Source used. */
export const feishuImConfig = (value: unknown): FeishuConfig => FeishuConfigSchema.parse(value ?? {})

/** Returns the cached account facts of one dsh-im Feishu bot, verifying it only when the cache misses. */
export const feishuBotFor = (im: DshImService, botId: string): Promise<CachedBot> => botFor(im, botId)
const botFor = async (im: DshImService, botId: string): Promise<CachedBot> => {
  const cached = cachedBots.get(botId)
  if (cached) return cached
  const described = await im.describeBot(botId)
  const bot = { botId, fingerprint: described.account.fingerprint, name: described.account.name || botId,
    brand: described.domain === 'lark' ? 'lark' as const : 'feishu' as const }
  cachedBots.set(botId, bot)
  return bot
}
const routeOf = (address: Record<string, string>): DshImEvent['reply'] => {
  const messageId = address.messageId ?? ''
  const conversationId = address.conversationId ?? address.chatId ?? ''
  const actorId = address.actorId ?? ''
  if (!messageId || !conversationId || !actorId) throw new Error('The Feishu reply route is incomplete')
  return { messageId, conversationId, actorId, ...address.threadId ? { threadId: address.threadId } : {},
    ...address.rootId ? { rootId: address.rootId } : {}, ...address.parentId ? { parentId: address.parentId } : {} }
}
const routeAddress = (event: DshImEvent): Record<string, string> => ({
  chatId: event.conversation.id, messageId: event.messageId, actorId: event.actor.id,
  ...event.reply.threadId ? { threadId: event.reply.threadId } : {},
  ...event.reply.rootId ? { rootId: event.reply.rootId } : {},
  ...event.reply.parentId ? { parentId: event.reply.parentId } : {},
})

/** Converts one authenticated dsh-im event into a domain Trigger; its id is the Feishu message id. */
export function imTrigger(event: DshImEvent): Trigger {
  const direct = event.conversation.kind === 'dm'
  const root = event.reply.rootId ?? event.reply.threadId ?? event.messageId
  return {
    id: key(SOURCE, event.messageId), source: SOURCE,
    threads: [{ source: SOURCE, id: `${event.conversation.id}:${root}` }],
    actor: { source: SOURCE, id: event.actor.id }, human: event.actor.kind === 'user',
    ...(direct ? { addressed: 'direct' as const } : event.mentionedAccount ? { addressed: 'mention' as const } : {}),
    at: event.at, text: withMentionNames(event.text, event.mentions),
    channel: { source: SOURCE, thread: `feishu:${event.conversation.id}`, address: { ...routeAddress(event), botId: event.botId, ...direct ? { direct: 'true' } : {} } },
    meta: { botId: event.botId, chatId: event.conversation.id, messageId: event.messageId, direct, mentioned: event.mentionedAccount,
      ...event.messageType ? { messageType: event.messageType } : {},
      ...event.images?.length ? { images: event.images } : {},
      ...event.actor.kind === 'bot' ? { bot: true } : {},
      ...event.reply.threadId ? { threadId: event.reply.threadId } : {},
      ...event.reply.rootId ? { rootId: event.reply.rootId } : {},
      ...event.reply.parentId ? { parentId: event.reply.parentId } : {} },
  }
}
/** The Trigger id of the message a Feishu reply answers, when Feishu supplied the parent id. */
export const imRepliesTo = (trigger: Trigger): string | undefined =>
  typeof trigger.meta.parentId === 'string' ? key(SOURCE, trigger.meta.parentId) : undefined

const eventView = (event: DshImEvent): ContextMessage => ({
  id: event.messageId, from: event.actor.name || event.actor.id, at: event.at,
  text: withMentionNames(event.text, event.mentions),
  ...event.actor.kind === 'bot' ? { bot: true as const } : {},
  ...event.reply.parentId ? { reply_to: event.reply.parentId } : {},
  ...event.appLink ? { url: event.appLink } : {},
})
const messageView = (message: DshImMessage): ContextMessage => ({
  id: message.messageId, from: message.sender.name || message.sender.id, at: message.at, text: message.text,
  ...message.sender.kind === 'bot' ? { bot: true as const } : {},
  ...message.parentId ? { reply_to: message.parentId } : {},
  ...message.appLink ? { url: message.appLink } : {},
})
/** Reads one message of the trigger's chat by id; the retained route proves access to that chat. */
async function readMessage(im: DshImService, botId: string, bot: CachedBot, route: DshImEvent['reply'],
  messageId: string): Promise<DshImMessage | undefined> {
  if (!im.messageChecked) return undefined
  try {
    return await im.messageChecked(botId, route, { expectedFingerprint: bot.fingerprint, messageId,
      signal: AbortSignal.timeout(30_000) })
  } catch (error) {
    const code = (error as { code?: string }).code
    if (code === 'account-changed') cachedBots.delete(botId)
    // A deleted message, or one whose sender is not a person or bot, has no context to show.
    if (code === 'stale-route' || code === 'message-unavailable') return undefined
    throw error
  }
}
const empty = (status: string): ContextPart => ({ messages: [], body: { status }, status })
/** Media type of an image by its magic bytes; Feishu resources carry no extension the caller can trust. */
function imageMediaType(data: Uint8Array): ImageMediaType | undefined {
  const ascii = (start: number, end: number): string => String.fromCharCode(...data.slice(start, end))
  if (data.length >= 8 && data[0] === 0x89 && data[1] === 0x50 && data[2] === 0x4e && data[3] === 0x47) return 'image/png'
  if (data.length >= 3 && data[0] === 0xff && data[1] === 0xd8 && data[2] === 0xff) return 'image/jpeg'
  if (data.length >= 6 && ascii(0, 3) === 'GIF') return 'image/gif'
  if (data.length >= 12 && ascii(0, 4) === 'RIFF' && ascii(8, 12) === 'WEBP') return 'image/webp'
  return undefined
}

/** Reads one image of the Trigger's message through dsh-im and commits it as a model-visible attachment. */
export async function readFeishuImImage(agent: Agent, trigger: Trigger, message: { id: string; text: string },
  key: string, signal: AbortSignal): Promise<{ source_message_id: string; image_key: string; image: ImageAttachmentRef }> {
  if (!admissionImageKeys(message.text).includes(key)) throw new Error('Choose an image key from this source message')
  const im = dshIm()
  const botId = typeof trigger.meta.botId === 'string' ? trigger.meta.botId : currentConfig?.botId ?? ''
  if (!im?.resourceChecked || !botId) throw new Error('Image reading is unavailable: dsh-im does not expose Feishu resources')
  const attachments = agent.ctx.get('attachments')
  if (!attachments) throw new Error('Image reading is unavailable: no attachment service is mounted')
  const route = agent.session.requestHeader()?.config
  const provider = route?.provider ?? agent.options.provider
  const model = route?.model ?? agent.options.model
  const llm = agent.ctx.get('llm')
  if (!llm || !provider || !model) throw new Error('Image reading is unavailable: the current model route is unresolved')
  const info = await llm.resolveModelInfo(provider, model, signal)
  if (!info.inputModalities?.includes('image')) throw new Error(`Model ${model} does not support image input; the image has not been inspected`)
  const bot = await botFor(im, botId)
  const limit = Math.min(attachments.imageLimits.maxImageBytes, attachments.imageLimits.maxMessageImageBytes)
  const resource = await im.resourceChecked(botId, routeOf(trigger.channel?.address ?? {}),
    { fileKey: key, type: 'image', maxBytes: limit },
    { expectedFingerprint: bot.fingerprint, signal: AbortSignal.any([signal, AbortSignal.timeout(60_000)]) })
  const mediaType = imageMediaType(resource.data)
  if (!mediaType) throw new Error('The downloaded resource is not a supported PNG/JPEG/WebP/GIF image')
  const image = await attachments.saveImage({ data: resource.data, mediaType, name: `${key}.${mediaType.split('/')[1]}` })
  return { source_message_id: message.id, image_key: key, image }
}

/** Reads Feishu context through dsh-im's bounded, authenticated text history. */
export const feishuImThreadContext: ThreadContext<FeishuConfig> = {
  messageId: trigger => String(trigger.meta.messageId ?? ''),
  async read(_commands, trigger, part, cursor, config) {
    const im = dshIm()
    if (!im) throw new Error('dsh-im is not mounted')
    const botId = typeof trigger.meta.botId === 'string' ? trigger.meta.botId : config.botId
    if (!botId) throw new Error('Choose an IM Feishu bot first')
    const bot = await botFor(im, botId)
    const route = routeOf(trigger.channel?.address ?? {})
    if (part === 'root' || part === 'parent') {
      const id = part === 'root' ? route.rootId : route.parentId
      const message = id ? await readMessage(im, botId, bot, route, id) : undefined
      return message ? { messages: [messageView(message)], body: { status: 'available', ...message } }
        : empty(`no_${part}`)
    }
    const scope: 'nearby' | 'thread' = route.threadId ? 'thread' : 'nearby'
    let page
    try {
      page = await im.historyChecked(botId, route, {
        scope, limit: config.readPageSize, includeBots: true, ...cursor ? { cursor } : {},
        ...scope === 'nearby' ? { beforeCount: 5, afterCount: 5 } : {},
      }, { expectedFingerprint: bot.fingerprint, signal: AbortSignal.timeout(30_000) })
    } catch (error) {
      if ((error as { code?: string }).code === 'account-changed') cachedBots.delete(botId)
      throw error
    }
    return { messages: page.events.map(eventView), next: page.nextCursor, body: { status: 'available', ...page }, status: 'available' }
  },
  images: text => admissionImageKeys(text),
  image: (agent, _commands, trigger, message, key, _stateDir, signal) => readFeishuImImage(agent, trigger, message, key, signal),
}

/** Sends one queued reply through dsh-im's checked original-message route. */
export async function sendFeishuIm(_host: Pick<SourceContext, 'commands'>, message: OutboundMessage): Promise<'sent' | 'gone'> {
  const config = currentConfig
  const im = dshIm()
  const botId = message.channel.address.botId || config?.botId || ''
  if (!im || !botId) throw new Error('dsh-im is not ready for Feishu replies')
  const bot = await botFor(im, botId)
  try {
    await im.replyChecked(botId, routeOf(message.channel.address), checkedImText(message.text),
      { expectedFingerprint: bot.fingerprint, signal: AbortSignal.timeout(30_000), uuid: message.id,
        format: 'markdown', replyInThread: true })
    return 'sent'
  } catch (error) {
    // `reply-result-unknown` is retryable: the same receipt id is sent as Feishu's `uuid`, so a retry after an
    // ambiguous result is deduplicated instead of producing a second reply.
    if ((error as { code?: string }).code === 'stale-route') return 'gone'
    if ((error as { code?: string }).code === 'account-changed') cachedBots.delete(botId)
    throw error
  }
}
/** Publishes the chat name and chat link on each Thread a Trigger opened, so the panel needs no extra read. */
function describeThreads(full: SourceContext | undefined, event: DshImEvent, trigger: Trigger): void {
  if (!full?.threads?.describe) return
  const labels = full.state.get(KV.sources.labels(SOURCE)) ?? {}
  const chat = event.conversation.kind === 'dm'
    ? { value: { zh: '私聊', en: 'Direct message' } }
    : { value: labels[event.conversation.id] ?? event.conversation.id }
  const url = feishuChatLink(cachedBots.get(event.botId)?.brand ?? 'feishu', event.conversation.id)
  for (const ref of trigger.threads) full.threads.describe(threadKey(ref), { url, facts: { chat } })
}
const feishuLink = (address: Record<string, string>) => ({
  url: feishuChatLink(cachedBots.get(address.botId ?? '')?.brand ?? 'feishu', address.chatId ?? ''),
  label: { zh: '飞书消息', en: 'Feishu message' },
  ...address.messageId ? { triggerId: key(SOURCE, address.messageId) } : {},
})

export const feishuImKind: SourceKind<FeishuConfig> = {
  kind: SOURCE,
  title: { zh: '飞书', en: 'Feishu' },
  description: { zh: '接收 dsh-im 飞书机器人的文本消息；在 dsh-im 里打开“交给 Superagent”后生效。', en: 'Receives text from a dsh-im Feishu bot after its Hand to Superagent switch is on.' },
  schema: FeishuConfigSchema,
  fields: [
    { key: 'botId', type: 'text', choose: { zh: '选择 IM 飞书机器人', en: 'Choose an IM Feishu bot' },
      label: { zh: 'IM 飞书机器人', en: 'IM Feishu bot' },
      hint: { zh: '在 dsh-im 里已经配置好的机器人。它的设置页打开“交给 Superagent”后，这个来源才会接管消息。', en: 'A bot already configured in dsh-im. This Source takes its messages only after Hand to Superagent is on in that bot\'s settings.' } },
    { key: 'chats', type: 'rows', label: { zh: '接收的群', en: 'Chats' },
      hint: { zh: '“仅 @”只接收 @ 机器人的消息和已有工单的线程；“全部消息”也接收其余消息，需要 dsh-im 机器人有读取全部消息的权限。', en: '"@ only" receives messages that mention the bot and Threads that have Tickets; "All" also receives the rest and needs the dsh-im bot permission for all messages.' },
      columns: [
        { key: 'chatId', type: 'text', label: { zh: '群', en: 'Chat' }, pattern: 'oc_[a-zA-Z0-9]+', choose: { zh: '选择 dsh-im 投递目标中的群', en: 'Choose a chat from dsh-im delivery targets' } },
        { key: 'listen', type: 'select', label: { zh: '接收', en: 'Receive' }, options: [{ value: 'mentions', label: { zh: '仅 @', en: '@ only' } }, { value: 'all', label: { zh: '全部消息', en: 'All' } }] },
      ] },
    { key: 'otherChats', type: 'select', label: { zh: '其他群', en: 'Other chats' }, options: [{ value: 'none', label: { zh: '不接收', en: 'Not received' } }, { value: 'mentions', label: { zh: '仅 @', en: '@ only' } }],
      hint: { zh: '机器人已加入、但没列在上面的群。“仅 @”接收那里 @ 机器人的消息；谁的消息能发起工作看策略里的准入。', en: 'Chats the bot has joined that are not listed above. "@ only" receives messages there that mention the bot; whose messages may originate work is the Policy\'s admission.' } },
    { key: 'directMessages', type: 'switch', label: { zh: '接收私聊', en: 'Direct messages' } },
    { key: 'readPageSize', type: 'number', advanced: true, min: 1, max: HISTORY_LIMIT, label: { zh: '每页消息数', en: 'Messages per page' }, hint: { zh: '智能体读取线程和群历史时，每页的消息数。', en: 'Messages per page when agents read Threads and chat history.' } },
    { key: 'linkTimeoutMs', type: 'number', advanced: true, min: 1, max: 90_000, label: { zh: '原消息链接的查询时限 / 毫秒', en: 'Original message link time limit / ms' },
      hint: { zh: '查询原消息链接最多等待的时间；超时后智能体改为直接读取原消息。', en: 'How long to wait for an original message\'s link; after that, agents read the message itself.' } },
  ],
  health: ['feishu', 'history'],
  scope({ meta }, config) {
    if (meta.direct === true) return config.directMessages ? { listened: true, observed: false, row: 'direct' } : { listened: false, reason: 'direct_messages_not_allowed' }
    const chat = config.chats.find(item => item.chatId === meta.chatId)
    if (chat) return { listened: true, observed: chat.listen === 'all', row: chat.chatId }
    return config.otherChats === 'mentions' ? { listened: true, observed: false, row: 'other' } : { listened: false, reason: 'chat_not_allowed' }
  },
  channel: {
    allowed: (address, config) =>
      feishuImKind.scope!({ meta: { chatId: address.chatId ?? '', direct: address.direct === 'true' } }, config).listened,
    send: sendFeishuIm, link: feishuLink,
  },
  threadContext: feishuImThreadContext,
  repliesTo: imRepliesTo,
  find: query => ({ threads: [], terms: [...new Set(query.match(/om_[A-Za-z0-9_-]+/g) ?? [])] }),
  policy: {
    unnamed: { person: { zh: '飞书用户', en: 'Feishu user' }, bot: { zh: '飞书机器人', en: 'Feishu bot' } },
    schema: z.object({
      allowed: z.array(z.object({ id: z.string().regex(/^ou_[a-zA-Z0-9]+$/), name: z.string().trim().max(100).optional() }).strict())
        .refine(users => new Set(users.map(user => user.id)).size === users.length, 'Duplicate open_ids').default([]),
      syncChat: z.string().regex(/^(oc_[a-zA-Z0-9]+)?$/).default(''),
      bots: z.array(z.object({ id: z.string().regex(/^(cli|ou)_[a-zA-Z0-9]+$/), name: z.string().trim().max(100).optional() }).strict())
        .refine(bots => new Set(bots.map(bot => bot.id)).size === bots.length, 'Duplicate bot ids').optional(),
    }).strict(),
    fields: [
      { key: 'allowed', type: 'rows', label: { zh: '能发起工作的飞书用户', en: 'Feishu users who may originate work' },
        columns: [{ key: 'id', type: 'text', label: { zh: 'open_id', en: 'open_id' }, pattern: 'ou_[a-zA-Z0-9]+' }, { key: 'name', type: 'text', label: { zh: '名字', en: 'Name' }, optional: true }] },
      { key: 'syncChat', type: 'text', pattern: '(oc_[a-zA-Z0-9]+)?', label: { zh: '同步来源群', en: 'Sync chat' }, choose: { zh: '不同步', en: 'Do not sync' },
        hint: { zh: '同步时，名单变成这个群里的人：新成员加入，离开的人移除，名字取群里的名字。点同步时进行。', en: 'Syncing makes the list the people in this chat: new members join, people who left are removed, and names follow the chat. It runs on Sync.' } },
      { key: 'bots', type: 'rows', label: { zh: '消息可以成为候选的机器人', en: 'Bots whose messages may become candidates' },
        hint: { zh: '名单里的机器人在接收的群里发的消息成为推断的候选，交给评估；它们不能发起明确的请求，也不能停止工单。', en: 'Messages of a bot on this list, in chats the Source receives, become inferred candidates for an assessment; they never make an explicit request or stop a ticket.' },
        columns: [{ key: 'id', type: 'text', label: { zh: 'App ID 或 open_id', en: 'App ID or open_id' }, pattern: '(cli|ou)_[a-zA-Z0-9]+' },
          { key: 'name', type: 'text', label: { zh: '名字', en: 'Name' }, optional: true }] },
    ],
    explicit: { zh: '名单内的人 @ 机器人或私聊机器人，总是明确的请求，不需要配置。', en: 'Someone on the list who mentions the bot or messages it directly always makes an explicit request; that needs no setting.' },
    rows(stored) {
      const config = FeishuConfigSchema.parse(stored)
      const mentions = { zh: '仅 @', en: '@ only' }
      return [
        ...config.chats.map(chat => ({ id: chat.chatId, label: chat.name || chat.chatId, note: chat.listen === 'all' ? { zh: '全部消息', en: 'All' } : mentions,
          cells: { listedUnaddressed: chat.listen === 'all', unlistedAddressed: true, unlistedUnaddressed: chat.listen === 'all' } })),
        { id: 'other', label: { zh: '其他群', en: 'Other chats' }, note: config.otherChats === 'mentions' ? mentions : { zh: '不接收', en: 'Not received' },
          cells: { listedUnaddressed: false, unlistedAddressed: config.otherChats === 'mentions', unlistedUnaddressed: false } },
        { id: 'direct', label: { zh: '私聊', en: 'Direct messages' }, note: config.directMessages ? { zh: '接收', en: 'Received' } : { zh: '不接收', en: 'Not received' },
          cells: { listedUnaddressed: false, unlistedAddressed: config.directMessages, unlistedUnaddressed: false } },
      ]
    },
    sync: {
      label: { zh: '立即同步', en: 'Sync now' },
      ready: part => !!part.syncChat,
      async run(host, part, signal) {
        const im = dshIm()
        const botId = host.sources.instance<FeishuConfig>(SOURCE)?.config.botId
        if (!im?.chatMembersChecked || !botId) throw new Error('Choose an IM Feishu bot first')
        const described = await im.describeBot(botId)
        const members = await im.chatMembersChecked(botId, { chatId: String(part.syncChat) },
          { expectedFingerprint: described.account.fingerprint, signal: signal ?? AbortSignal.timeout(30_000) })
        const users = new Map(members.users.map(user => [user.id, user.name ?? '']))
        const known = new Set(part.allowed.map(entry => entry.id))
        return { ...part, allowed: [
          ...part.allowed.filter(entry => users.has(entry.id)).map(entry => ({ ...entry, name: users.get(entry.id) || entry.name })),
          ...members.users.filter(user => !known.has(user.id)).map(user => ({ id: user.id, name: user.name ?? '' })),
        ] }
      },
    },
  },
  environment: {
    variables: [
      { key: 'DSH_SUPERAGENT_FEISHU_BOT', description: { zh: 'dsh-im 飞书机器人；回复由 dsh-im 发送', en: 'The dsh-im Feishu bot; dsh-im sends the replies' } },
      { key: 'DSH_SUPERAGENT_FEISHU_CHAT', description: { zh: '接下工单的飞书群 chat_id；工单不是从飞书来的时为空', en: 'chat_id of the Feishu chat that started the ticket; empty when the ticket did not start on Feishu' } },
    ],
    values: ({ config, trigger }) => {
      const botId = trigger?.source === SOURCE && typeof trigger.meta.botId === 'string' ? trigger.meta.botId : config.botId
      return {
        // The bot's name reads better in prompts; the id remains the fallback before its account is read.
        DSH_SUPERAGENT_FEISHU_BOT: cachedBots.get(botId)?.name || botId,
        DSH_SUPERAGENT_FEISHU_CHAT: trigger?.source === SOURCE ? String(trigger.meta.chatId ?? '') : '',
      }
    },
  },
  async refreshLabels(host) {
    const im = dshIm()
    if (!im) return {}
    const state = host.context(feishuImKind).state
    const labels: Record<string, string> = { ...(state.get(KV.sources.labels(SOURCE)) ?? {}) }
    try {
      for (const bot of (await im.listBots()).filter(bot => bot.channel === 'feishu')) {
        try { const described = await im.describeBot(bot.botId); labels[bot.botId] = described.account.name || bot.botId }
        catch { labels[bot.botId] = bot.botId }
      }
    } catch { /* The bot list is a display aid; the Source still shows the saved id. */ }
    const config = host.sources.instance<FeishuConfig>(SOURCE)?.config
    if (config?.botId) {
      try {
        for (const target of await im.listTargets(config.botId)) {
          const chatId = target.route?.chatId
          if (target.kind === 'group' && chatId) labels[chatId] = target.name || chatId
        }
      } catch { /* Targets are a display aid. */ }
    }
    state.set(KV.sources.labels(SOURCE), labels)
    return labels
  },
}

/** Returns the Feishu Source's settings, falling back to defaults while the plugin is disabled. */
export const feishuSettings = (sources: SourceRegistry): FeishuConfig =>
  sources.instance<FeishuConfig>(SOURCE)?.config ?? FeishuConfigSchema.parse({})

/** The subset of `SourceContext` the handoff watcher uses; tests supply it directly. */
export interface FeishuWatchContext {
  readonly ready: Promise<void>
  config(kind: string): FeishuConfig | undefined
  configured(): boolean
  receive(trigger: Trigger): unknown
  report(entry: string, health: Health): void
}
/** Watches dsh-im's handoff switch and owns the exclusive consumer while it is on; `full` enables backfill and polling. */
export async function watchFeishuIm(ctx: FeishuWatchContext, serviceOf: () => DshImService | undefined, signal: AbortSignal,
  full?: SourceContext): Promise<void> {
  await ctx.ready
  let release: (() => void) | undefined
  let controller: AbortController | undefined
  let backfill: AbortController | undefined
  let backfillRun: Promise<void> | undefined
  let botId = ''
  let botName = ''
  const stop = (): void => {
    controller?.abort(); backfill?.abort(); release?.()
    controller = undefined; backfill = undefined; release = undefined; backfillRun = undefined
  }
  try {
    while (!signal.aborted) {
      const im = serviceOf()
      setDshIm(im)
      const config = ctx.config(SOURCE)
      currentConfig = config
      if (!im) {
        stop(); ctx.report('feishu', { state: 'paused', detail: 'dsh-im is not mounted' })
        await delay(POLL_MS, undefined, { signal }).catch(() => {})
        continue
      }
      if (!config) {
        stop(); ctx.report('feishu', { state: 'paused', detail: 'The Feishu Source is off' })
        await delay(POLL_MS, undefined, { signal }).catch(() => {})
        continue
      }
      if (!config.botId) {
        stop(); ctx.report('feishu', { state: 'paused', detail: 'Choose an IM Feishu bot' })
        await delay(POLL_MS, undefined, { signal }).catch(() => {})
        continue
      }
      if (botId !== config.botId) { stop(); botId = config.botId; botName = ''; cachedBots.clear() }
      try {
        // `inboundStatus` is a cheap local read; `describeBot` verifies the account over the network, so only registration pays for it.
        const status = im.inboundStatus ? await im.inboundStatus(config.botId) : await im.describeBot(config.botId)
        if (!status.superagent) {
          stop(); ctx.report('feishu', { state: 'paused', detail: 'Hand to Superagent is off for this bot' })
        } else {
          // dsh-im can release the consumer without aborting our lease (switch off, restart, or a replacement runtime).
          if (release && !status.superagentActive) stop()
          if (!release) {
            if (status.superagentActive) {
              ctx.report('feishu', { state: 'error', detail: 'Another consumer already owns this Feishu bot' })
            } else {
              const bot = await im.describeBot(config.botId)
              const cached = { botId: config.botId, fingerprint: bot.account.fingerprint, name: bot.account.name || config.botId,
                brand: bot.domain === 'lark' ? 'lark' as const : 'feishu' as const }
              cachedBots.set(config.botId, cached)
              botName = cached.name
              controller = new AbortController()
              const lease = AbortSignal.any([signal, controller.signal])
              release = await im.consumeInbound(config.botId, {
                expectedFingerprint: bot.account.fingerprint, signal: lease, superagent: true, includeBots: true,
                onEvent: async (event: DshImEvent) => {
                  const current = ctx.config(SOURCE)
                  if (current && ctx.configured()) {
                    const trigger = imTrigger(event)
                    await ctx.receive(trigger)
                    describeThreads(full, event, trigger)
                  }
                  if (full && event.conversation.kind === 'dm') {
                    const chats = full.state.get(KV.feishu.chats) ?? {}
                    if (chats[event.conversation.id] !== true)
                      full.state.set(KV.feishu.chats, { ...chats, [event.conversation.id]: true })
                  }
                  return { accepted: true }
                },
              })
              if (full) {
                backfill = new AbortController()
                const signal = AbortSignal.any([lease, backfill.signal])
                backfillRun = runFeishuImBackfill({ ctx: full, im, botId: config.botId, fingerprint: bot.account.fingerprint,
                  config: () => ctx.config(SOURCE), toTrigger: imTrigger,
                  describe: (event, trigger) => describeThreads(full, event, trigger),
                  report: (entry, health) => ctx.report(entry, health), signal })
                backfillRun.catch(error => {
                  if (!signal.aborted) ctx.report('history', { state: 'error', detail: String(error) })
                })
              }
              ctx.report('feishu', { state: 'ready', detail: `Listening for ${botName}`, at: new Date().toISOString() })
            }
          } else {
            ctx.report('feishu', { state: 'ready', detail: `Listening for ${botName || config.botId}`, at: new Date().toISOString() })
          }
        }
      } catch (error) {
        if ((error as { code?: string }).code === 'account-changed') cachedBots.clear()
        stop()
        ctx.report('feishu', { state: 'error', detail: String(error) })
      }
      await delay(POLL_MS, undefined, { signal }).catch(() => {})
    }
  } finally {
    const running = backfillRun
    stop()
    await running?.catch(() => {})
    setDshIm(undefined)
    currentConfig = undefined
    cachedBots.clear()
  }
}

export const name = 'dsh-superagent-feishu'
export const inject = ['orchestrator']
export function apply(ctx: Context): void {
  setDshIm(dshImOf(ctx))
  ctx.effect(() => ctx.orchestrator.sources.register(feishuImKind), 'feishu: source kind')
  ctx.effect(() => {
    const controller = new AbortController()
    const full = sourceContext(ctx.orchestrator, feishuImKind)
    const running = watchFeishuIm(full, () => dshImOf(ctx), controller.signal, full)
    return async () => { controller.abort(); await running }
  })
}

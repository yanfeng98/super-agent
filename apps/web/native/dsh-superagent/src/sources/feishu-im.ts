/** Feishu Source backed by dsh-im's exclusive text consumer, with no lark-cli or separate bot credentials. */
import type { Context } from '@deepseek-ai/cordis'
import { setTimeout as delay } from 'node:timers/promises'
import { z } from 'zod'
import { key, type OutboundMessage, type Trigger } from '../core/index.ts'
import { checkedImText } from '../infra/text.ts'
import { dshIm, dshImOf, setDshIm, type DshImEvent, type DshImService } from './dsh-im.ts'
import { feishuChatLink } from './feishu-channel.ts'
import { sourceContext, type Health, type SourceContext } from './host.ts'
import type { ContextMessage, ContextPart, SourceKind, ThreadContext } from './kind.ts'
import type { SourceRegistry } from './registry.ts'

export const SOURCE = 'feishu'
const POLL_MS = 2_000
let currentConfig: FeishuConfig | undefined
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
})
export type FeishuConfig = z.infer<typeof FeishuConfigSchema>
/** Parses a stored Feishu Source config, ignoring keys the lark-cli Source used. */
export const feishuImConfig = (value: unknown): FeishuConfig => FeishuConfigSchema.parse(value ?? {})

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
    actor: { source: SOURCE, id: event.actor.id }, human: true,
    ...(direct ? { addressed: 'direct' as const } : event.mentionedAccount ? { addressed: 'mention' as const } : {}),
    at: event.at, text: event.text,
    channel: { source: SOURCE, thread: `feishu:${event.conversation.id}`, address: { ...routeAddress(event), ...direct ? { direct: 'true' } : {} } },
    meta: { chatId: event.conversation.id, messageId: event.messageId, direct, mentioned: event.mentionedAccount,
      ...event.reply.threadId ? { threadId: event.reply.threadId } : {},
      ...event.reply.rootId ? { rootId: event.reply.rootId } : {},
      ...event.reply.parentId ? { parentId: event.reply.parentId } : {} },
  }
}
/** The Trigger id of the message a Feishu reply answers, when Feishu supplied the parent id. */
export const imRepliesTo = (trigger: Trigger): string | undefined =>
  typeof trigger.meta.parentId === 'string' ? key(SOURCE, trigger.meta.parentId) : undefined

const eventView = (event: DshImEvent): ContextMessage => ({
  id: event.messageId, from: event.actor.name || event.actor.id, at: event.at, text: event.text,
})
const empty = (status: string): ContextPart => ({ messages: [], body: { status }, status })
/** Reads Feishu context through dsh-im's bounded, authenticated text history. */
export const feishuImThreadContext: ThreadContext<FeishuConfig> = {
  messageId: trigger => String(trigger.meta.messageId ?? ''),
  async read(_commands, trigger, part, cursor, config) {
    const im = dshIm()
    if (!im) throw new Error('dsh-im is not mounted')
    if (!config.botId) throw new Error('Choose an IM Feishu bot first')
    const bot = await im.describeBot(config.botId)
    const route = routeOf(trigger.channel?.address ?? {})
    const scope: 'group' | 'nearby' | 'thread' = (part === 'thread' || part === 'root' || part === 'parent') && route.threadId
      ? 'thread' : 'nearby'
    const page = await im.historyChecked(config.botId, route, {
      scope, limit: config.readPageSize, ...cursor ? { cursor } : {},
      ...scope === 'nearby' ? { beforeCount: 5, afterCount: 5 } : {},
    }, { expectedFingerprint: bot.account.fingerprint, signal: AbortSignal.timeout(30_000) })
    if (part === 'root' || part === 'parent') {
      const id = part === 'root' ? route.rootId : route.parentId
      const found = id ? page.events.find(event => event.messageId === id) : undefined
      return found ? { messages: [eventView(found)], body: { status: 'available', ...page } }
        : empty(`no_${part}`)
    }
    return { messages: page.events.map(eventView), next: page.nextCursor, body: { status: 'available', ...page }, status: 'available' }
  },
}

/** Sends one queued reply through dsh-im's checked original-message route. */
export async function sendFeishuIm(_host: Pick<SourceContext, 'commands'>, message: OutboundMessage): Promise<'sent' | 'gone'> {
  const config = currentConfig
  const im = dshIm()
  if (!im || !config?.botId) throw new Error('dsh-im is not ready for Feishu replies')
  const bot = await im.describeBot(config.botId)
  try {
    await im.replyChecked(config.botId, routeOf(message.channel.address), checkedImText(message.text),
      { expectedFingerprint: bot.account.fingerprint, signal: AbortSignal.timeout(30_000) })
    return 'sent'
  } catch (error) {
    if ((error as { code?: string }).code === 'stale-route') return 'gone'
    throw error
  }
}
const feishuLink = (address: Record<string, string>) => ({
  url: feishuChatLink('feishu', address.chatId ?? ''), label: { zh: '飞书消息', en: 'Feishu message' },
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
  ],
  health: ['feishu'],
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
  policy: {
    unnamed: { person: { zh: '飞书用户', en: 'Feishu user' }, bot: { zh: '飞书机器人', en: 'Feishu bot' } },
    schema: z.object({
      allowed: z.array(z.object({ id: z.string().regex(/^ou_[a-zA-Z0-9]+$/), name: z.string().trim().max(100).optional() }).strict())
        .refine(users => new Set(users.map(user => user.id)).size === users.length, 'Duplicate open_ids').default([]),
    }),
    fields: [
      { key: 'allowed', type: 'rows', label: { zh: '能发起工作的飞书用户', en: 'Feishu users who may originate work' },
        columns: [{ key: 'id', type: 'text', label: { zh: 'open_id', en: 'open_id' }, pattern: 'ou_[a-zA-Z0-9]+' }, { key: 'name', type: 'text', label: { zh: '名字', en: 'Name' }, optional: true }] },
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
  },
  environment: {
    variables: [
      { key: 'DSH_SUPERAGENT_FEISHU_BOT', description: { zh: 'dsh-im 飞书机器人；回复由 dsh-im 发送', en: 'The dsh-im Feishu bot; dsh-im sends the replies' } },
      { key: 'DSH_SUPERAGENT_FEISHU_CHAT', description: { zh: '接下工单的飞书群 chat_id；工单不是从飞书来的时为空', en: 'chat_id of the Feishu chat that started the ticket; empty when the ticket did not start on Feishu' } },
    ],
    values: ({ config, trigger }) => ({ DSH_SUPERAGENT_FEISHU_BOT: config.botId,
      DSH_SUPERAGENT_FEISHU_CHAT: trigger?.source === SOURCE ? String(trigger.meta.chatId ?? '') : '' }),
  },
  async refreshLabels(host) {
    const im = dshIm()
    if (!im) return {}
    const labels: Record<string, string> = {}
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
/** Watches dsh-im's handoff switch and owns the exclusive consumer while it is on. */
export async function watchFeishuIm(ctx: FeishuWatchContext, serviceOf: () => DshImService | undefined, signal: AbortSignal): Promise<void> {
  await ctx.ready
  let release: (() => void) | undefined
  let controller: AbortController | undefined
  let botId = ''
  const stop = (): void => { controller?.abort(); release?.(); controller = undefined; release = undefined }
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
      if (botId !== config.botId) { stop(); botId = config.botId }
      try {
        const bot = await im.describeBot(config.botId)
        if (!bot.superagent) {
          stop(); ctx.report('feishu', { state: 'paused', detail: 'Hand to Superagent is off for this bot' })
        } else {
          // dsh-im can release the consumer without aborting our lease (switch off, restart, or a replacement runtime).
          if (release && !bot.superagentActive) stop()
          if (!release) {
            if (bot.superagentActive) {
              ctx.report('feishu', { state: 'error', detail: 'Another consumer already owns this Feishu bot' })
            } else {
              controller = new AbortController()
              const lease = AbortSignal.any([signal, controller.signal])
              release = await im.consumeInbound(config.botId, {
                expectedFingerprint: bot.account.fingerprint, signal: lease, superagent: true,
                onEvent: async (event: DshImEvent) => {
                  const current = ctx.config(SOURCE)
                  if (current && ctx.configured()) await ctx.receive(imTrigger(event))
                  return { accepted: true }
                },
              })
              ctx.report('feishu', { state: 'ready', detail: `Listening for ${bot.account.name || config.botId}`, at: new Date().toISOString() })
            }
          } else {
            ctx.report('feishu', { state: 'ready', detail: `Listening for ${bot.account.name || config.botId}`, at: new Date().toISOString() })
          }
        }
      } catch (error) {
        stop()
        ctx.report('feishu', { state: 'error', detail: String(error) })
      }
      await delay(POLL_MS, undefined, { signal }).catch(() => {})
    }
  } finally {
    stop()
    setDshIm(undefined)
    currentConfig = undefined
  }
}

export const name = 'dsh-superagent-feishu'
export const inject = ['orchestrator']
export function apply(ctx: Context): void {
  ctx.effect(() => ctx.orchestrator.sources.register(feishuImKind), 'feishu: source kind')
  ctx.effect(() => {
    const controller = new AbortController()
    const running = watchFeishuIm(sourceContext(ctx.orchestrator, feishuImKind), () => dshImOf(ctx), controller.signal)
    return async () => { controller.abort(); await running }
  })
}

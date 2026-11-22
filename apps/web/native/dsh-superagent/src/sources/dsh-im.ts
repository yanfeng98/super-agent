/** The same-Host dsh-im Feishu intake service, typed for the superagent Feishu Source. */
import type { Context } from '@deepseek-ai/cordis'

/** One authenticated inbound text event delivered by dsh-im's exclusive Feishu consumer. */
export interface DshImEvent {
  version: 1
  channel: 'feishu'
  botId: string
  fingerprint: string
  eventId: string
  messageId: string
  actor: { kind: 'user' | 'bot'; id: string; name?: string }
  conversation: { kind: 'group' | 'dm'; id: string }
  mentions: { id: string; key: string; name?: string }[]
  mentionedAccount: boolean
  /** How dsh-im rendered the body: plain text, rich text (post), or an image placeholder carrying its key. */
  messageType?: 'text' | 'post' | 'image'
  /** Image keys referenced by the message, in order. */
  images?: string[]
  at: string
  text: string
  reply: { messageId: string; conversationId: string; actorId: string; threadId?: string; rootId?: string; parentId?: string }
}

/** A bot dsh-im owns, as listed for a Source picker. */
export interface DshImBot { botId: string; channel: string }
/** A dsh-im delivery target; only group targets name a Feishu chat this Source can receive. */
export interface DshImTarget { targetId: string; kind: string; name?: string; route?: { chatId?: string; openId?: string } }
/** The authenticated account facts and handoff state of one bot. */
export interface DshImBotDescription {
  version: 1
  botId: string
  channel: string
  domain?: 'feishu' | 'lark'
  account: { fingerprint: string; name?: string }
  connected: boolean
  superagent: boolean
  superagentActive: boolean
  capabilities: string[]
}
/** One page of bounded history, in the same event shape as live intake. */
export interface DshImHistory {
  version: 1
  scope: 'group' | 'nearby' | 'thread' | 'chat'
  events: DshImEvent[]
  omitted: number
  hasMore: boolean
  nextCursor?: string
  coverage: string
}
/** The dsh-im Host service consumed by this plugin; no browser or HTTP surface. */
export interface DshImService {
  contractVersion: 1
  inboundVersion: 1
  listBots(): Promise<DshImBot[]>
  listTargets(botId: string): Promise<DshImTarget[]>
  describeBot(botId: string): Promise<DshImBotDescription>
  /** Cheap handoff state; absent on an older dsh-im, where the watcher falls back to `describeBot`. */
  inboundStatus?(botId: string): Promise<{ version: 1; botId: string; superagent: boolean; superagentActive: boolean }>
  consumeInbound(botId: string, options: {
    expectedFingerprint: string
    signal: AbortSignal
    onEvent(event: DshImEvent, context: { signal: AbortSignal }): Promise<{ accepted: true }>
    superagent?: true
    includeBots?: boolean
  }): Promise<() => void>
  historyChecked(botId: string, route: DshImEvent['reply'], query: {
    scope: 'group' | 'nearby' | 'thread'
    limit: number
    cursor?: string
    beforeCount?: number
    afterCount?: number
    includeBots?: boolean
  }, options: { expectedFingerprint: string; signal: AbortSignal }): Promise<DshImHistory>
  replyChecked(botId: string, route: DshImEvent['reply'], text: string,
    options: { expectedFingerprint: string; signal: AbortSignal; uuid?: string }): Promise<{ sent: true; messageId: string }>
  /** Reads one chat between two times, oldest first; `direct` marks a direct chat. */
  chatHistoryChecked?(botId: string, query: {
    chatId: string; limit: number; since?: string; until?: string; cursor?: string
    includeBots?: boolean; direct?: boolean
  }, options: { expectedFingerprint: string; signal: AbortSignal }): Promise<DshImHistory>
  /** Reads one topic thread between two times, oldest first. */
  threadHistoryChecked?(botId: string, query: {
    chatId: string; threadId: string; limit: number; since?: string; until?: string; cursor?: string; includeBots?: boolean
  }, options: { expectedFingerprint: string; signal: AbortSignal }): Promise<DshImHistory>
  /** Reads one retained message plus the AppLink that opens it in Feishu. */
  messageChecked?(botId: string, route: DshImEvent['reply'], options: { expectedFingerprint: string; signal: AbortSignal }): Promise<DshImMessage>
  /** Reads one chat's members as open_id plus name. */
  chatMembersChecked?(botId: string, query: { chatId: string },
    options: { expectedFingerprint: string; signal: AbortSignal }): Promise<{ version: 1; chatId: string; users: { id: string; name?: string }[] }>
  /** Downloads one image or file of a retained message. */
  resourceChecked?(botId: string, route: DshImEvent['reply'], query: { fileKey: string; type: 'image' | 'file'; maxBytes?: number },
    options: { expectedFingerprint: string; signal: AbortSignal }): Promise<DshImResource>
}
/** One retained message as dsh-im reads it. */
export interface DshImMessage {
  version: 1
  messageId: string
  chatId: string
  at: string
  text: string
  messageType?: 'text' | 'post' | 'image'
  images?: string[]
  sender: { kind: 'user' | 'bot'; id: string; name?: string }
  appLink?: string
  deleted: boolean
}
/** One downloaded Feishu resource; `data` is raw bytes. */
export interface DshImResource {
  version: 1
  messageId: string
  fileKey: string
  type: 'image' | 'file'
  size: number
  data: Uint8Array
}

let service: DshImService | undefined
/** Records the service this plugin resolved from its Cordis context; the Source loops read it through `dshIm()`. */
export const setDshIm = (value: DshImService | undefined): void => { service = value }
/** Returns the current dsh-im service, or undefined while dsh-im is not mounted. */
export const dshIm = (): DshImService | undefined => service
/** Reads the optional same-Host service without requiring it at plugin load. */
export const dshImOf = (ctx: Context): DshImService | undefined => ctx.get('dshIm') as DshImService | undefined

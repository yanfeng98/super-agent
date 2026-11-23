/** The same-Host dsh-im Feishu intake service, typed for the superagent Feishu Source. */
import type { Context } from '@deepseek-ai/cordis'

/** How dsh-im rendered a message body: model-readable text, or a placeholder that names the resource and its key. */
export type DshImMessageType = 'text' | 'post' | 'image' | 'file' | 'audio' | 'media' | 'sticker'
  | 'interactive' | 'share_chat' | 'share_user' | 'merge_forward' | 'system'
/** One authenticated inbound event delivered by dsh-im's exclusive Feishu consumer. */
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
  /** How dsh-im rendered the body: plain text, rich text (post), or a placeholder carrying the resource key. */
  messageType?: DshImMessageType
  /** Image keys referenced by the message, in order. */
  images?: string[]
  at: string
  text: string
  reply: { messageId: string; conversationId: string; actorId: string; threadId?: string; rootId?: string; parentId?: string }
  /** Feishu AppLink that opens the message, present on history reads. */
  appLink?: string
  /** True for a recalled message a context read keeps in place: its sender and time stay, and `text` is empty. */
  deleted?: true
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
  /** Unix-second bounds of a `nearby` window, `start` inclusive and `end` exclusive; history beyond them is not in this page. */
  window?: { start: number; end: number }
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
    /** Keeps a recalled message as a bodiless event instead of counting it as omitted. */
    includeDeleted?: boolean
  }, options: { expectedFingerprint: string; signal: AbortSignal }): Promise<DshImHistory>
  replyChecked(botId: string, route: DshImEvent['reply'], text: string,
    options: {
      expectedFingerprint: string; signal: AbortSignal; uuid?: string
      /** `markdown` sends a rich-text card; `text` (default) sends plain text. */
      format?: 'text' | 'markdown'
      /** Answers inside the message's topic, starting one when it has none. */
      replyInThread?: boolean
    }): Promise<{ sent: true; messageId: string }>
  /** Reads one chat between two times, oldest first; `direct` marks a direct chat. */
  chatHistoryChecked?(botId: string, query: {
    chatId: string; limit: number; since?: string; until?: string; cursor?: string
    includeBots?: boolean; direct?: boolean
  }, options: { expectedFingerprint: string; signal: AbortSignal }): Promise<DshImHistory>
  /** Reads one topic thread between two times, oldest first; `direct` marks a direct chat. */
  threadHistoryChecked?(botId: string, query: {
    chatId: string; threadId: string; limit: number; since?: string; until?: string; cursor?: string; includeBots?: boolean
    direct?: boolean
    /** Keeps the bot's own messages, which a backfill skips; a bot-authored message still needs `includeBots`. */
    includeSelf?: boolean
    /** Keeps a recalled message as a bodiless event instead of counting it as omitted. */
    includeDeleted?: boolean
  }, options: { expectedFingerprint: string; signal: AbortSignal }): Promise<DshImHistory>
  /**
   * Reads one retained message plus the AppLink that opens it; `messageId` reads another message of the same chat.
   * Throws `message-deleted` when that other message was recalled, `message-unavailable` when its body cannot be
   * rendered, and `stale-route` when the retained route no longer resolves — including when the route's own message,
   * which every read of it names by id, was recalled.
   */
  messageChecked?(botId: string, route: DshImEvent['reply'],
    options: { expectedFingerprint: string; signal: AbortSignal; messageId?: string }): Promise<DshImMessage>
  /** Reads one chat's members as open_id plus name. */
  chatMembersChecked?(botId: string, query: { chatId: string },
    options: { expectedFingerprint: string; signal: AbortSignal }): Promise<{ version: 1; chatId: string; users: { id: string; name?: string }[] }>
  /** Reads the chats the bot has joined, as chat_id plus name. */
  chatListChecked?(botId: string,
    options: { expectedFingerprint: string; signal: AbortSignal }): Promise<{ version: 1; chats: { chatId: string; name?: string }[] }>
  /** Downloads one image or file of a retained message; `messageId` downloads from another message of the same chat. */
  resourceChecked?(botId: string, route: DshImEvent['reply'], query: { fileKey: string; type: 'image' | 'file'; maxBytes?: number },
    options: { expectedFingerprint: string; signal: AbortSignal; messageId?: string }): Promise<DshImResource>
}
/** One retained message as dsh-im reads it. */
export interface DshImMessage {
  version: 1
  messageId: string
  chatId: string
  at: string
  text: string
  messageType?: DshImMessageType
  images?: string[]
  sender: { kind: 'user' | 'bot'; id: string; name?: string }
  threadId?: string
  rootId?: string
  parentId?: string
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

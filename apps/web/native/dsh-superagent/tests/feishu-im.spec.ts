import { afterEach, describe, expect, it } from 'vitest'
import { key } from '../src/core/index.ts'
import { setDshIm, type DshImEvent, type DshImService } from '../src/sources/dsh-im.ts'
import { feishuImKind, imTrigger, sendFeishuIm, setFeishuConfig, watchFeishuIm, type FeishuConfig, type FeishuWatchContext } from '../src/sources/feishu-im.ts'

const event = (overrides: Partial<DshImEvent> = {}): DshImEvent => ({
  version: 1, channel: 'feishu', botId: 'bot_1', fingerprint: 'f'.repeat(64), eventId: 'event_1',
  messageId: 'om_1', actor: { kind: 'user', id: 'ou_alice', name: 'Alice' },
  conversation: { kind: 'group', id: 'oc_team' }, mentions: [], mentionedAccount: true,
  at: '2026-11-21T10:00:00.000Z', text: 'fix the login page',
  reply: { messageId: 'om_1', conversationId: 'oc_team', actorId: 'ou_alice', threadId: 'omt_1', rootId: 'om_root', parentId: 'om_parent' },
  ...overrides,
})
const config = (overrides: Partial<FeishuConfig> = {}): FeishuConfig => ({
  botId: 'bot_1', chats: [{ chatId: 'oc_team', listen: 'mentions' }], directMessages: true, otherChats: 'none', readPageSize: 20,
  ...overrides,
})
const service = (overrides: Partial<DshImService> = {}): DshImService => ({
  contractVersion: 1, inboundVersion: 1,
  listBots: async () => [{ botId: 'bot_1', channel: 'feishu' }],
  listTargets: async () => [{ targetId: 'team', kind: 'group', name: 'Team', route: { chatId: 'oc_team' } }],
  describeBot: async () => ({ version: 1, botId: 'bot_1', channel: 'feishu', account: { fingerprint: 'f'.repeat(64), name: 'Bot' },
    connected: true, superagent: true, superagentActive: false, capabilities: ['exclusive-text-consumer'] }),
  consumeInbound: async () => () => {},
  historyChecked: async () => ({ version: 1, scope: 'thread', events: [event()], omitted: 0, hasMore: false, coverage: 'provider-visible-human-text' }),
  replyChecked: async () => ({ sent: true, messageId: 'om_reply' }),
  ...overrides,
})

afterEach(() => { setDshIm(undefined); setFeishuConfig(undefined) })

describe('the dsh-im-backed Feishu Source', () => {
  it('maps an authenticated event to a Trigger with the original reply route', () => {
    const trigger = imTrigger(event())
    expect(trigger.id).toBe(key('feishu', 'om_1'))
    expect(trigger.threads).toEqual([{ source: 'feishu', id: 'oc_team:om_root' }])
    expect(trigger.actor).toEqual({ source: 'feishu', id: 'ou_alice' })
    expect(trigger.addressed).toBe('mention')
    expect(trigger.channel).toMatchObject({ source: 'feishu', thread: 'feishu:oc_team', address: {
      chatId: 'oc_team', messageId: 'om_1', actorId: 'ou_alice', threadId: 'omt_1', rootId: 'om_root', parentId: 'om_parent',
    } })
  })

  it('marks a direct message addressed and keeps the chat scope settings', () => {
    const trigger = imTrigger(event({ conversation: { kind: 'dm', id: 'oc_dm' }, mentions: [], mentionedAccount: false }))
    expect(trigger.addressed).toBe('direct')
    expect(trigger.channel?.address.direct).toBe('true')
    expect(feishuImKind.scope!({ meta: { chatId: 'oc_team', direct: false } }, config()).listened).toBe(true)
    expect(feishuImKind.scope!({ meta: { chatId: 'oc_other', direct: false } }, config()).listened).toBe(false)
    expect(feishuImKind.scope!({ meta: { chatId: 'oc_dm', direct: true } }, config()).listened).toBe(true)
  })

  it('drops legacy policy keys that the lark-cli Source used', () => {
    const parsed = feishuImKind.policy!.schema.parse({ allowed: [], syncChat: 'oc_team', bots: [{ id: 'ou_bot' }] })
    expect(parsed).toEqual({ allowed: [] })
  })

  it('replies through the checked original-message route and reports a stale route as gone', async () => {
    const calls: [string, Record<string, string>, string, unknown][] = []
    setFeishuConfig(config())
    setDshIm(service({
      replyChecked: async (...args) => { calls.push(args); return { sent: true, messageId: 'om_reply' } },
    }))
    const message = { id: 'reply_1', channel: { source: 'feishu', thread: 'feishu:oc_team', address: {
      chatId: 'oc_team', messageId: 'om_1', actorId: 'ou_alice', threadId: 'omt_1',
    } }, text: 'done', kind: 'reply' as const }
    await expect(sendFeishuIm({ commands: {} as never }, message)).resolves.toBe('sent')
    expect(calls[0]?.[0]).toBe('bot_1')
    expect(calls[0]?.[1]).toMatchObject({ messageId: 'om_1', conversationId: 'oc_team', actorId: 'ou_alice', threadId: 'omt_1' })
    setDshIm(service({ replyChecked: async () => { throw Object.assign(new Error('gone'), { code: 'stale-route' }) } }))
    await expect(sendFeishuIm({ commands: {} as never }, message)).resolves.toBe('gone')
  })

  it('reads thread context through dsh-im history and filters a root by its message id', async () => {
    setDshIm(service())
    const trigger = imTrigger(event())
    const thread = await feishuImKind.threadContext!.read({} as never, trigger, 'thread', undefined, config())
    expect(thread.messages).toHaveLength(1)
    expect(thread.messages[0]).toMatchObject({ id: 'om_1', from: 'Alice', text: 'fix the login page' })
    setDshIm(service({ historyChecked: async () => ({ version: 1, scope: 'group', events: [event({ messageId: 'om_root' })], omitted: 0, hasMore: false, coverage: 'provider-visible-human-text' }) }))
    const root = await feishuImKind.threadContext!.read({} as never, trigger, 'root', undefined, config())
    expect(root.messages[0]?.id).toBe('om_root')
    expect(feishuImKind.threadContext!.images).toBeUndefined()
  })

  it('registers the exclusive consumer while the handoff switch is on and releases it on abort', async () => {
    let release = 0
    let accept: ((event: DshImEvent, context: { signal: AbortSignal }) => Promise<unknown>) | undefined
    const received: string[] = []
    const reports: { state: string }[] = []
    const imService = service({
      consumeInbound: async (_botId, options) => {
        accept = options.onEvent
        return () => { release += 1 }
      },
    })
    setDshIm(imService)
    const controller = new AbortController()
    const ctx: FeishuWatchContext = {
      ready: Promise.resolve(), config: () => config(), configured: () => true,
      receive: trigger => { received.push(trigger.id) },
      report: (_entry, health) => { reports.push(health) },
    }
    const running = watchFeishuIm(ctx, () => imService, controller.signal)
    for (let attempt = 0; attempt < 20 && !accept; attempt += 1) await new Promise(resolve => setTimeout(resolve, 0))
    expect(accept).toBeTypeOf('function')
    expect(reports.at(-1)?.state).toBe('ready')
    await accept!(event(), { signal: new AbortController().signal })
    expect(received).toEqual([key('feishu', 'om_1')])
    controller.abort()
    await running
    expect(release).toBe(1)
  })

  it('exposes the bot and chat labels dsh-im owns', async () => {
    setDshIm(service())
    const host = { sources: { instance: () => ({ config: config() }) } }
    await expect(feishuImKind.refreshLabels!(host as never)).resolves.toEqual({ bot_1: 'Bot', oc_team: 'Team' })
  })
})

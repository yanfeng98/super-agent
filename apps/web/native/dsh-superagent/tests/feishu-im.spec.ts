import { afterEach, describe, expect, it, vi } from 'vitest'
import { key } from '../src/core/index.ts'
import { KV } from '../src/kv.ts'
import { sourceLink } from '../src/app/links.ts'
import { feishuImPage } from '../src/app/resources.ts'
import { setDshIm, type DshImEvent, type DshImService } from '../src/sources/dsh-im.ts'
import { feishuImKind, imTrigger, sendFeishuIm, setFeishuConfig, watchFeishuIm, type FeishuConfig, type FeishuWatchContext } from '../src/sources/feishu-im.ts'
import { runFeishuImBackfill } from '../src/sources/feishu-im-loops.ts'

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
  linkTimeoutMs: 5000,
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

  it('uses the Thread resource instead of a lark-cli link for a dsh-im Feishu trigger', async () => {
    const trigger = imTrigger(event())
    await expect(sourceLink({} as never, trigger)).resolves.toEqual({ source_url_read: `feishu-im/${trigger.id}/thread` })
  })

  it('serves the feishu-im resource from the stored Source config', async () => {
    setDshIm(service())
    const host = { commands: { withSignal: () => ({}) }, store: { settings: () => ({ sources: { feishu: { config: { botId: 'bot_1' } } } }) } }
    const page = await feishuImPage(host as never, imTrigger(event()), 'thread')
    expect(page.messages.map(message => message.id)).toEqual(['om_1'])
  })

  it('keeps the bot and member-sync policy fields', () => {
    const parsed = feishuImKind.policy!.schema.parse({ allowed: [], syncChat: 'oc_team', bots: [{ id: 'cli_ci' }] })
    expect(parsed).toEqual({ allowed: [], syncChat: 'oc_team', bots: [{ id: 'cli_ci' }] })
  })

  it('syncs the allowed list from dsh-im chat members', async () => {
    setDshIm(service({ chatMembersChecked: async () => ({ version: 1, chatId: 'oc_team',
      users: [{ id: 'ou_alice', name: 'Alice' }, { id: 'ou_bob' }] }) }))
    const host = { sources: { instance: () => ({ config: { botId: 'bot_1' } }) } }
    const part = { allowed: [{ id: 'ou_alice', name: 'Old' }, { id: 'ou_gone', name: 'Gone' }], syncChat: 'oc_team' }
    const result = await feishuImKind.policy!.sync!.run(host as never, part as never, new AbortController().signal)
    expect(result.allowed).toEqual([{ id: 'ou_alice', name: 'Alice' }, { id: 'ou_bob', name: '' }])
  })

  it('stores refreshed labels for the chat picker', async () => {
    setDshIm(service())
    const state = new Map<string, unknown>()
    const host = { sources: { instance: () => ({ config: config() }) }, context: () => ({ state: {
      get: (name: string) => state.get(name), set: (name: string, value: unknown) => { state.set(name, value) } } }) }
    const labels = await feishuImKind.refreshLabels!(host as never)
    expect(labels).toMatchObject({ bot_1: 'Bot', oc_team: 'Team' })
    expect(state.get(KV.sources.labels('feishu'))).toEqual(labels)
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
    // Replies render as rich text in the source message's topic, and the receipt id deduplicates a retry.
    expect(calls[0]?.[3]).toMatchObject({ uuid: 'reply_1', format: 'markdown', replyInThread: true })
    setDshIm(service({ replyChecked: async () => { throw Object.assign(new Error('gone'), { code: 'stale-route' }) } }))
    await expect(sendFeishuIm({ commands: {} as never }, message)).resolves.toBe('gone')
  })

  it('reads thread context through dsh-im history and a root or parent message by its id', async () => {
    setDshIm(service())
    const trigger = imTrigger(event())
    const thread = await feishuImKind.threadContext!.read({} as never, trigger, 'thread', undefined, config())
    expect(thread.messages).toHaveLength(1)
    expect(thread.messages[0]).toMatchObject({ id: 'om_1', from: 'Alice', text: 'fix the login page' })
    const asked: unknown[][] = []
    setDshIm(service({ messageChecked: async (...args: unknown[]) => {
      asked.push(args)
      return { version: 1, messageId: 'om_root', chatId: 'oc_team', at: '2026-11-20T09:00:00.000Z',
        text: 'root message', sender: { kind: 'user', id: 'ou_bob', name: 'Bob' }, deleted: false }
    } }))
    const root = await feishuImKind.threadContext!.read({} as never, trigger, 'root', undefined, config())
    expect(root.messages[0]).toMatchObject({ id: 'om_root', from: 'Bob', text: 'root message' })
    expect(asked[0]?.[2]).toMatchObject({ messageId: 'om_root' })
    setDshIm(service({ messageChecked: async () => { throw Object.assign(new Error('gone'), { code: 'stale-route' }) } }))
    const parent = await feishuImKind.threadContext!.read({} as never, trigger, 'parent', undefined, config())
    expect(parent.status).toBe('no_parent')
    expect(parent.messages).toEqual([])
    setDshIm(service({ historyChecked: async () => ({ version: 1, scope: 'thread', omitted: 0, hasMore: false,
      coverage: 'provider-visible-human-text', events: [event({ actor: { kind: 'bot', id: 'cli_ci', name: 'CI' },
        text: '@_user_1 build ok', mentions: [{ id: 'ou_bot', key: '@_user_1', name: '机器人' }] })] }) }))
    const bots = await feishuImKind.threadContext!.read({} as never, trigger, 'thread', undefined, config())
    expect(bots.messages[0]).toMatchObject({ from: 'CI', bot: true, text: '@机器人 build ok' })
    expect(feishuImKind.threadContext!.images!('see [图片] img_abc and img_def')).toEqual(['img_abc', 'img_def'])
  })

  it('keeps the message AppLink as the context message URL', async () => {
    const link = 'https://applink.feishu.cn/client/message/open?message_id=om_1'
    setDshIm(service({ historyChecked: async () => ({ version: 1, scope: 'thread', omitted: 0, hasMore: false,
      coverage: 'provider-visible-human-text', events: [event({ appLink: link })] }) }))
    const page = await feishuImKind.threadContext!.read({} as never, imTrigger(event()), 'thread', undefined, config())
    expect(page.messages[0]?.url).toBe(link)
  })

  it('registers the exclusive consumer while the handoff switch is on and releases it on abort', async () => {
    let release = 0
    let accept: ((event: DshImEvent, context: { signal: AbortSignal }) => Promise<unknown>) | undefined
    const received: string[] = []
    const reports: { state: string }[] = []
    let active = false
    let described = 0
    const imService = service({
      inboundStatus: async () => ({ version: 1, botId: 'bot_1', superagent: true, superagentActive: active }),
      describeBot: async () => {
        described += 1
        return { version: 1, botId: 'bot_1', channel: 'feishu', account: { fingerprint: 'f'.repeat(64), name: 'Bot' },
          connected: true, superagent: true, superagentActive: active, capabilities: ['exclusive-text-consumer'] }
      },
      consumeInbound: async (_botId, options) => {
        accept = options.onEvent
        active = true
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
    expect(described).toBe(1)
  })

  it('exposes the bot and chat labels dsh-im owns', async () => {
    setDshIm(service())
    const state = new Map<string, unknown>()
    const host = { sources: { instance: () => ({ config: config() }) },
      context: () => ({ state: { get: (name: string) => state.get(name),
        set: (name: string, value: unknown) => { state.set(name, value) } } }) }
    await expect(feishuImKind.refreshLabels!(host as never)).resolves.toEqual({ bot_1: 'Bot', oc_team: 'Team' })
  })
})

describe('dsh-im Feishu recovery and identity', () => {
  it('renders mention names and keeps bot and image metadata on the Trigger', () => {
    const trigger = imTrigger(event({ mentions: [{ id: 'ou_bot', key: '@_user_1', name: '机器人' }],
      text: '@_user_1 修一下', messageType: 'image', images: ['img_1'] }))
    expect(trigger.text).toBe('@机器人 修一下')
    expect(trigger.meta.images).toEqual(['img_1'])
    expect(trigger.meta.messageType).toBe('image')
    expect(imTrigger(event({ actor: { kind: 'bot', id: 'cli_ci' } })).human).toBe(false)
    expect(imTrigger(event()).human).toBe(true)
  })

  it('retries an ambiguous reply with the same uuid instead of dropping it', async () => {
    const calls: unknown[][] = []
    setFeishuConfig(config())
    setDshIm(service({ replyChecked: async (...args) => {
      calls.push(args)
      throw Object.assign(new Error('reply-result-unknown'), { code: 'reply-result-unknown' })
    } }))
    const message = { id: 'reply_1', channel: { source: 'feishu', thread: 'feishu:oc_team', address: {
      chatId: 'oc_team', messageId: 'om_1', actorId: 'ou_alice', threadId: 'omt_1' } }, text: 'done', kind: 'reply' as const }
    await expect(sendFeishuIm({} as never, message)).rejects.toMatchObject({ code: 'reply-result-unknown' })
    expect(calls[0]![3]).toMatchObject({ uuid: 'reply_1' })
  })

  it('backfills chat history from the stored cursor and polls subscribed threads', async () => {
    const received: string[] = []
    const state = new Map<string, unknown>()
    const page = { version: 1 as const, scope: 'chat' as const, events: [event()], omitted: 0, hasMore: false, coverage: 'provider-visible-messages' }
    const im = service({
      chatHistoryChecked: async () => page,
      threadHistoryChecked: async () => ({ ...page, scope: 'thread' as const }),
    })
    const thread = { key: 'feishu:oc_team:om_root', ref: { source: 'feishu', id: 'oc_team:om_root' },
      meta: { chatId: 'oc_team', threadId: 'omt_1' }, createdAt: new Date().toISOString(), subscriptions: [] }
    const ctx = {
      ready: Promise.resolve(),
      config: () => config(),
      configured: () => true,
      receive: async (trigger: { id: string }) => { received.push(trigger.id) },
      report: () => {},
      state: { get: (name: string) => state.get(name), set: (name: string, value: unknown) => { state.set(name, value) }, keys: () => [] },
      threads: { subscribed: () => [thread], patch: () => {}, get: () => undefined, describe: () => {} },
      triggers: { latest: () => undefined, get: () => undefined },
      tickets: { get: () => undefined },
    }
    const controller = new AbortController()
    const run = runFeishuImBackfill({ ctx: ctx as never, im, botId: 'bot_1', fingerprint: 'f'.repeat(64),
      config: () => config(), toTrigger: imTrigger, report: () => {}, signal: controller.signal })
    await vi.waitFor(() => expect(received.length).toBeGreaterThanOrEqual(2))
    controller.abort()
    await run
    expect(new Set(received)).toEqual(new Set([key('feishu', 'om_1')]))
    expect(state.get(KV.sources.labels('feishu'))).toEqual({ oc_team: 'Team' })
    expect(typeof state.get(KV.feishu.history('oc_team'))).toBe('string')
  })
})

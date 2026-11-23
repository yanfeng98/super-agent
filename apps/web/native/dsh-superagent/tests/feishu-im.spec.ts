import { afterEach, describe, expect, it, vi } from 'vitest'
import { key } from '../src/core/index.ts'
import { KV } from '../src/kv.ts'
import { sourceLink } from '../src/app/links.ts'
import { feishuImPage } from '../src/app/resources.ts'
import { setDshIm, type DshImEvent, type DshImService } from '../src/sources/dsh-im.ts'
import { feishuImKind, imTrigger, sendFeishuIm, setFeishuConfig, watchFeishuIm, type FeishuConfig, type FeishuWatchContext } from '../src/sources/feishu-im.ts'
import type { SourceContext } from '../src/sources/host.ts'
import { backfillChat, runFeishuImBackfill } from '../src/sources/feishu-im-loops.ts'

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
    // The assessing Session reads a mapped message, not the record dsh-im keeps for its own route checks.
    expect(root.body).toMatchObject({ status: 'available', message: root.messages[0] })
    expect(asked[0]?.[2]).toMatchObject({ messageId: 'om_root' })
    // A route that stopped resolving reports no content, which is a different answer from a reply that names no parent.
    setDshIm(service({ messageChecked: async () => { throw Object.assign(new Error('gone'), { code: 'stale-route' }) } }))
    const parent = await feishuImKind.threadContext!.read({} as never, trigger, 'parent', undefined, config())
    expect(parent.status).toBe('unavailable')
    expect(parent.messages).toEqual([])
    const unparented = imTrigger(event({ reply: { messageId: 'om_1', conversationId: 'oc_team', actorId: 'ou_alice' } }))
    const absent = await feishuImKind.threadContext!.read({} as never, unparented, 'parent', undefined, config())
    expect(absent.status).toBe('no_parent')
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

  it('keeps a message whose handling failed in the Source\'s health until the next message is handled', async () => {
    let accept: ((event: DshImEvent, context: { signal: AbortSignal }) => Promise<unknown>) | undefined
    let active = false
    let fail = true
    const reports: Parameters<FeishuWatchContext['report']>[1][] = []
    const imService = service({
      inboundStatus: async () => ({ version: 1, botId: 'bot_1', superagent: true, superagentActive: active }),
      consumeInbound: async (_botId, options) => { accept = options.onEvent; active = true; return () => {} },
    })
    setDshIm(imService)
    const controller = new AbortController()
    const ctx: FeishuWatchContext = { ready: Promise.resolve(), config: () => config(), configured: () => true,
      receive: () => { if (fail) throw new Error('store is down') }, report: (_entry, health) => { reports.push(health) } }
    const running = watchFeishuIm(ctx, () => imService, controller.signal)
    for (let attempt = 0; attempt < 20 && !accept; attempt += 1) await new Promise(resolve => setTimeout(resolve, 0))
    // dsh-im logs the rejection; the Source reports it too, because the message is not retried.
    await expect(accept!(event(), { signal: new AbortController().signal })).rejects.toThrow('store is down')
    expect(reports.at(-1)).toMatchObject({ state: 'error', detail: expect.stringContaining(`Handling ${key('feishu', 'om_1')} failed: store is down`) })
    // Poll instead of sleeping a fixed poll interval, so a loaded machine does not decide the outcome.
    const until = async (done: () => boolean) => {
      for (let attempt = 0; attempt < 100 && !done(); attempt += 1) await new Promise(resolve => setTimeout(resolve, 50))
      expect(done()).toBe(true)
    }
    // The loop's ready report repeats it instead of hiding it behind "Listening".
    await until(() => reports.filter(report => report.state === 'error').length > 1)
    // The next message that is handled clears it, so the Source returns to ready.
    fail = false
    await accept!(event({ messageId: 'om_2' }), { signal: new AbortController().signal })
    await until(() => reports.at(-1)?.state === 'ready')
    controller.abort()
    await running
  }, 15_000)

  it('describes each Thread with its chat name and link as messages arrive', async () => {
    let accept: ((event: DshImEvent, context: { signal: AbortSignal }) => Promise<unknown>) | undefined
    const described: { key: string; description: Record<string, unknown> }[] = []
    const imService = service({
      inboundStatus: async () => ({ version: 1, botId: 'bot_1', superagent: true, superagentActive: false }),
      consumeInbound: async (_botId, options) => { accept = options.onEvent; return () => {} },
    })
    setDshIm(imService)
    const controller = new AbortController()
    const ctx: FeishuWatchContext = { ready: Promise.resolve(), config: () => config(), configured: () => true,
      receive: () => undefined, report: () => {} }
    const full = { ...ctx,
      state: { get: () => ({ oc_team: 'Team' }), set: () => undefined, keys: () => [] },
      threads: { describe: (key: string, description: never) => { described.push({ key, description }) } },
    } as unknown as SourceContext
    const running = watchFeishuIm(ctx, () => imService, controller.signal, full)
    for (let attempt = 0; attempt < 20 && !accept; attempt += 1) await new Promise(resolve => setTimeout(resolve, 0))
    await accept!(event(), { signal: new AbortController().signal })
    expect(described[0]?.key).toBe('feishu:oc_team:om_root')
    expect(described[0]?.description).toMatchObject({
      url: 'https://applink.feishu.cn/client/chat/open?openChatId=oc_team', facts: { chat: { value: 'Team' } } })
    controller.abort()
    await running
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
    const state = new Map<string, unknown>([['feishu-history:oc_team', '2026-09-01T00:00:00.000Z']])
    const page = { version: 1 as const, scope: 'chat' as const, events: [event()], omitted: 0, hasMore: false, coverage: 'provider-visible-messages' }
    const im = service({
      chatHistoryChecked: async () => page,
      threadHistoryChecked: async () => ({ ...page, scope: 'thread' as const }),
      chatListChecked: async () => ({ version: 1 as const, chats: [{ chatId: 'oc_other', name: 'Other' }] }),
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
    // Both the chats the bot joined and dsh-im's delivery targets name chats for the picker.
    expect(state.get(KV.sources.labels('feishu'))).toEqual({ oc_team: 'Team', oc_other: 'Other' })
    expect(typeof state.get(KV.feishu.history('oc_team'))).toBe('string')
  })

  /** Backfill dependencies for a fake dsh-im, recording what they delivered and what the history health said. */
  const backfillDeps = (im: DshImService, state: Map<string, unknown>, threads: { meta: Record<string, string>; createdAt: string }[] = []) => {
    const received: { id: string; threads?: { id: string }[]; channel?: { address: Record<string, string> } }[] = []
    const reports: string[] = []
    const ctx = {
      ready: Promise.resolve(), config: () => config(), configured: () => true,
      receive: async (trigger: { id: string; threads?: { id: string }[]; channel?: { address: Record<string, string> } }) => {
        received.push(trigger)
      }, report: () => {},
      state: { get: (name: string) => state.get(name), set: (name: string, value: unknown) => { state.set(name, value) }, keys: () => [] },
      threads: { subscribed: () => threads.map(thread => ({ ...thread, key: 'feishu:oc_team:om_root',
        ref: { source: 'feishu', id: 'oc_team:om_root' }, subscriptions: [] })),
      patch: () => {}, get: () => undefined, describe: () => {} },
      triggers: { latest: () => undefined, get: () => undefined },
      tickets: { get: () => undefined },
    }
    const controller = new AbortController()
    const deps = { ctx: ctx as never, im, botId: 'bot_1', fingerprint: 'f'.repeat(64), config: () => config(), toTrigger: imTrigger,
      report: (_entry: string, value: { detail: string }) => { reports.push(value.detail) }, signal: controller.signal }
    return { received, reports, deps, controller }
  }
  /** Runs one backfill round against a fake dsh-im. */
  const backfill = (im: DshImService, state: Map<string, unknown>, threads: { meta: Record<string, string>; createdAt: string }[] = []) => {
    const { received, reports, deps } = backfillDeps(im, state, threads)
    const controller = new AbortController()
    const run = runFeishuImBackfill({ ...deps, signal: controller.signal })
    /** Waits for the round to finish reporting, then stops the loop before its next round. */
    const round = async () => {
      await vi.waitFor(() => expect(reports.some(detail => detail.startsWith('Recovered'))).toBe(true))
      controller.abort()
      await run
    }
    return { received, reports, round }
  }
  const chatPage = (events: DshImEvent[]) => ({ version: 1 as const, scope: 'chat' as const, events, omitted: 0, hasMore: false,
    coverage: 'provider-visible-messages' })

  it('recovers the replies of a topic chat history leaves out', async () => {
    const base = Date.now() - 3_600_000
    const at = (minutes: number): string => new Date(base + minutes * 60_000).toISOString()
    // The chat container returns the message a topic was opened on and names its topic; the replies are only in the thread.
    const root = event({ messageId: 'om_root', at: at(0), reply: { messageId: 'om_root', conversationId: 'oc_team',
      actorId: 'ou_alice', threadId: 'omt_9', rootId: 'om_root' } })
    const asked: Record<string, unknown>[] = []
    const im = service({
      chatHistoryChecked: async () => chatPage([root]),
      threadHistoryChecked: async (_botId: string, query: Record<string, unknown>) => {
        asked.push(query)
        return { version: 1 as const, scope: 'thread' as const, chatId: 'oc_team', threadId: 'omt_9', omitted: 0, hasMore: false,
          coverage: 'provider-visible-messages', events: [
            event({ messageId: 'om_reply_1', at: at(10), text: 'it is the lockfile', reply: { messageId: 'om_reply_1',
              conversationId: 'oc_team', actorId: 'ou_bob', threadId: 'omt_9', rootId: 'om_root' } }),
            // Feishu may leave root_id off a topic message; its Trigger still joins the Topic's Thread.
            event({ messageId: 'om_reply_2', at: at(20), text: 'fixed', reply: { messageId: 'om_reply_2',
              conversationId: 'oc_team', actorId: 'ou_bob', threadId: 'omt_9' } }),
          ] }
      },
    })
    const state = new Map<string, unknown>([['feishu-history:oc_team', at(-600)]])
    const { received, reports, round } = backfill(im, state)
    await round()
    expect(asked).toContainEqual(expect.objectContaining({ chatId: 'oc_team', threadId: 'omt_9', direct: false }))
    // Recovery is inbound work only: it never asks for the bot's own messages or recalled placeholders, so a reply
    // cannot come back as a new Trigger and a withdrawal cannot open one.
    expect(asked[0]).not.toHaveProperty('includeSelf')
    expect(asked[0]).not.toHaveProperty('includeDeleted')
    expect(new Set(received.map(trigger => trigger.id)))
      .toEqual(new Set([key('feishu', 'om_root'), key('feishu', 'om_reply_1'), key('feishu', 'om_reply_2')]))
    expect(received.find(trigger => trigger.id === key('feishu', 'om_reply_2'))?.threads?.[0]?.id).toBe('oc_team:om_root')
    // The route stays exactly as dsh-im reported it: a fabricated root would make every later read and reply fail.
    expect(received.find(trigger => trigger.id === key('feishu', 'om_reply_2'))?.channel?.address).not.toHaveProperty('rootId')
    expect(state.get(KV.feishu.imTopics('oc_team'))).toMatchObject({ omt_9: { at: at(20), rootId: 'om_root' } })
    // The Topic was read to the end of the round's window, so the next round starts there.
    expect(Date.parse((state.get(KV.feishu.imTopics('oc_team')) as Record<string, { readThrough: string }>).omt_9!.readThrough))
      .toBeGreaterThanOrEqual(Date.parse(at(20)))
    expect(reports).toContainEqual(expect.stringContaining('Recovered 3 messages'))
  })

  it('resumes a topic from the last message it delivered when its page budget runs out', async () => {
    const start = Date.now() - 7_200_000
    const at = (minutes: number): string => new Date(start + minutes * 60_000).toISOString()
    const asked: Record<string, unknown>[] = []
    const im = service({
      chatHistoryChecked: async () => chatPage([event({ messageId: 'om_root', at: at(0), reply: { messageId: 'om_root',
        conversationId: 'oc_team', actorId: 'ou_alice', threadId: 'omt_9', rootId: 'om_root' } })]),
      threadHistoryChecked: async (_botId: string, query: Record<string, unknown>) => {
        asked.push(query)
        // Every page claims another one, so each round stops at its page budget instead of draining the Topic.
        const page = asked.length
        return { version: 1 as const, scope: 'thread' as const, chatId: 'oc_team', threadId: 'omt_9', omitted: 0, hasMore: true,
          nextCursor: `cursor_${page}`, coverage: 'provider-visible-messages', events: [event({ messageId: `om_${page}`, at: at(page) })] }
      },
    })
    const state = new Map<string, unknown>([['feishu-history:oc_team', at(-1)]])
    const { deps } = backfillDeps(im, state)
    const errors: string[] = []
    await backfillChat(deps, 'oc_team', false, 50, errors)
    // The first round delivered the page budget and kept the time of its last message.
    expect(asked).toHaveLength(40)
    expect(errors).toEqual([])
    await backfillChat(deps, 'oc_team', false, 50, errors)
    // The second round continues from there, five seconds back, instead of the chat's own window.
    expect(asked).toHaveLength(80)
    expect(Date.parse(String(asked[40]?.since))).toBe(Date.parse(at(40)) - 5000)
  })

  it('keeps a topic\u2019s read position when the read fails before delivering anything', async () => {
    const start = Date.now() - 3_600_000
    const at = (minutes: number): string => new Date(start + minutes * 60_000).toISOString()
    const im = service({
      chatHistoryChecked: async () => chatPage([event({ messageId: 'om_root', at: at(0), reply: { messageId: 'om_root',
        conversationId: 'oc_team', actorId: 'ou_alice', threadId: 'omt_9', rootId: 'om_root' } })]),
      threadHistoryChecked: async () => { throw Object.assign(new Error('Feishu is unavailable'), { code: 'history-unavailable' }) },
    })
    const state = new Map<string, unknown>([['feishu-history:oc_team', at(-1)]])
    const { deps } = backfillDeps(im, state)
    const errors: string[] = []
    await backfillChat(deps, 'oc_team', false, 50, errors)
    // The window stays where it was, so the next round retries it instead of skipping the Topic's replies.
    expect(state.get(KV.feishu.imTopics('oc_team'))).toMatchObject({ omt_9: { readThrough: at(-1) } })
    expect(errors).toEqual([expect.stringContaining('omt_9')])
  })

  it('keeps a topic\u2019s Thread on the message that opened it', () => {
    // Feishu names the Topic on the message someone replied to, and its replies name that message as their root.
    const root = imTrigger(event({ messageId: 'om_root',
      reply: { messageId: 'om_root', conversationId: 'oc_team', actorId: 'ou_alice', threadId: 'omt_9' } }))
    expect(root.threads[0]?.id).toBe('oc_team:om_root')
    const reply = imTrigger(event({ messageId: 'om_reply', reply: { messageId: 'om_reply', conversationId: 'oc_team',
      actorId: 'ou_bob', threadId: 'omt_9', rootId: 'om_root' } }))
    expect(reply.threads[0]?.id).toBe(root.threads[0]?.id)
  })

  it('records a topic before the chat cursor moves past the message that named it', async () => {
    let stop: (() => void) | undefined
    const im = service({
      chatHistoryChecked: async () => chatPage([event({ messageId: 'om_root', reply: { messageId: 'om_root',
        conversationId: 'oc_team', actorId: 'ou_alice', threadId: 'omt_9', rootId: 'om_root' } })]),
      threadHistoryChecked: async () => { stop?.(); throw new Error('stopped') },
    })
    const state = new Map<string, unknown>([['feishu-history:oc_team', new Date(Date.now() - 300_000).toISOString()]])
    const { deps, controller } = backfillDeps(im, state)
    stop = () => controller.abort()
    await backfillChat(deps, 'oc_team', false, 50, [])
    // The round stopped inside the Topic read, after the chat cursor moved; the Topic still has to be on disk.
    expect(state.get(KV.feishu.imTopics('oc_team'))).toMatchObject({ omt_9: { rootId: 'om_root' } })
  })

  it('advances the topics read longest ago first, so none is starved', async () => {
    const state = new Map<string, unknown>([['feishu-history:oc_team', new Date(Date.now() - 300_000).toISOString()]])
    const base = Date.now() - 3_600_000
    const at = (minutes: number): string => new Date(base + minutes * 60_000).toISOString()
    const topics: Record<string, { at: string; rootId: string; readThrough: string }> = {}
    for (let index = 0; index < 21; index++) topics[`omt_${index}`] = { at: at(index), rootId: `om_${index}`, readThrough: at(index) }
    state.set(KV.feishu.imTopics('oc_team'), topics)
    const asked: Record<string, unknown>[] = []
    const im = service({
      chatHistoryChecked: async () => chatPage([]),
      threadHistoryChecked: async (_botId: string, query: Record<string, unknown>) => {
        asked.push(query)
        return { version: 1 as const, scope: 'thread' as const, chatId: 'oc_team', threadId: String(query.threadId), omitted: 0,
          hasMore: false, coverage: 'provider-visible-messages', events: [] }
      },
    })
    const { deps } = backfillDeps(im, state)
    await backfillChat(deps, 'oc_team', false, 50, [])
    // Twenty reads, oldest watermark first; the most recently read Topic waits for the next round.
    expect(asked.map(query => query.threadId)).toEqual(Array.from({ length: 20 }, (_, index) => `omt_${index}`))
  })

  it('stops a round at its time budget and keeps its progress', async () => {
    const asked: unknown[][] = []
    const im = service({
      chatHistoryChecked: async (...args: unknown[]) => { asked.push(args); return chatPage([event()]) },
    })
    const state = new Map<string, unknown>([['feishu-history:oc_team', new Date(Date.now() - 300_000).toISOString()]])
    const { deps } = backfillDeps(im, state)
    const before = state.get(KV.feishu.history('oc_team'))
    await backfillChat(deps, 'oc_team', false, 50, [], Date.now() - 1)
    expect(asked).toEqual([])
    // Nothing was read, so the cursor stays where it was and the next round reads the same window.
    expect(state.get(KV.feishu.history('oc_team'))).toBe(before)
  })

  it('reads a topic whose subscribed Thread is past its polling horizon', async () => {
    const started = new Date(Date.now() - 30 * 86_400_000).toISOString()
    const asked: unknown[][] = []
    const im = service({
      chatHistoryChecked: async () => chatPage([event({ messageId: 'om_root', reply: { messageId: 'om_root',
        conversationId: 'oc_team', actorId: 'ou_alice', threadId: 'omt_1', rootId: 'om_root' } })]),
      threadHistoryChecked: async (...args: unknown[]) => {
        asked.push(args)
        return { version: 1 as const, scope: 'thread' as const, chatId: 'oc_team', threadId: 'omt_1', events: [], omitted: 0,
          hasMore: false, coverage: 'provider-visible-messages' }
      },
    })
    const state = new Map<string, unknown>([['feishu-history:oc_team', new Date(Date.now() - 300_000).toISOString()]])
    const { round } = backfill(im, state, [{ meta: { chatId: 'oc_team', threadId: 'omt_1' }, createdAt: started }])
    await round()
    // The Thread poller leaves this Thread alone, so the Topic has to be read here or its replies are never recovered.
    expect(asked).toHaveLength(1)
  })

  it('leaves a topic its subscribed Thread already polls to the Thread poller', async () => {
    const started = new Date().toISOString()
    const asked: unknown[][] = []
    const im = service({
      chatHistoryChecked: async () => chatPage([event({ messageId: 'om_root', at: started, reply: { messageId: 'om_root',
        conversationId: 'oc_team', actorId: 'ou_alice', threadId: 'omt_1', rootId: 'om_root' } })]),
      threadHistoryChecked: async (...args: unknown[]) => {
        asked.push(args)
        return { version: 1 as const, scope: 'thread' as const, chatId: 'oc_team', threadId: 'omt_1', events: [], omitted: 0,
          hasMore: false, coverage: 'provider-visible-messages' }
      },
    })
    const state = new Map<string, unknown>([['feishu-history:oc_team', new Date(Date.now() - 86_400_000).toISOString()]])
    const { round } = backfill(im, state, [{ meta: { chatId: 'oc_team', threadId: 'omt_1' }, createdAt: started }])
    await round()
    // One read: the Thread poller's. The topic scan skips it instead of reading the same topic twice.
    expect(asked).toHaveLength(1)
  })

  it('learns a Thread topic from its root message before polling it', async () => {
    const state = new Map<string, unknown>([['feishu-history:oc_team', '2026-09-01T00:00:00.000Z']])
    const asked: string[] = []
    const patched: unknown[] = []
    const page = { version: 1 as const, scope: 'chat' as const, events: [], omitted: 0, hasMore: false, coverage: 'provider-visible-messages' }
    const im = service({
      chatHistoryChecked: async () => page,
      messageChecked: async () => { asked.push('message'); return { version: 1 as const, messageId: 'om_root', chatId: 'oc_team',
        at: '2026-09-01T00:00:00.000Z', text: 'root', sender: { kind: 'user' as const, id: 'ou_bob' }, threadId: 'omt_learned', deleted: false } },
      threadHistoryChecked: async () => { asked.push('thread'); return { ...page, scope: 'thread' as const } },
    })
    const thread = { key: 'feishu:oc_team:om_root', ref: { source: 'feishu', id: 'oc_team:om_root' },
      meta: { chatId: 'oc_team' }, createdAt: new Date().toISOString(), subscriptions: [] }
    const trigger = { meta: {}, channel: { address: { chatId: 'oc_team', messageId: 'om_root', actorId: 'ou_bob' } } }
    const ctx = {
      ready: Promise.resolve(), config: () => config(), configured: () => true, receive: async () => {}, report: () => {},
      state: { get: (name: string) => state.get(name), set: (name: string, value: unknown) => { state.set(name, value) }, keys: () => [] },
      threads: { subscribed: () => [thread], patch: (_key: string, patch: unknown) => { patched.push(patch) }, get: () => undefined, describe: () => {} },
      triggers: { latest: () => trigger, get: () => undefined },
      tickets: { get: () => undefined },
    }
    const controller = new AbortController()
    const run = runFeishuImBackfill({ ctx: ctx as never, im, botId: 'bot_1', fingerprint: 'f'.repeat(64),
      config: () => config(), toTrigger: imTrigger, report: () => {}, signal: controller.signal })
    await vi.waitFor(() => expect(asked).toContain('thread'))
    controller.abort()
    await run
    expect(asked[0]).toBe('message')
    expect(patched).toContainEqual({ threadId: 'omt_learned' })
    expect(state.get(KV.feishu.topic('feishu:oc_team:om_root'))).toMatchObject({ threadId: 'omt_learned' })
  })

  it('remembers a Thread whose source message was recalled as gone instead of rereading it every round', async () => {
    const state = new Map<string, unknown>([['feishu-history:oc_team', '2026-09-01T00:00:00.000Z']])
    const asked: string[] = []
    const page = { version: 1 as const, scope: 'chat' as const, events: [], omitted: 0, hasMore: false, coverage: 'provider-visible-messages' }
    const im = service({
      chatHistoryChecked: async () => page,
      messageChecked: async () => { asked.push('message'); throw Object.assign(new Error('recalled'), { code: 'message-deleted' }) },
      threadHistoryChecked: async () => { asked.push('thread'); return { ...page, scope: 'thread' as const } },
    })
    const createdAt = new Date().toISOString()
    const thread = { key: 'feishu:oc_team:om_root', ref: { source: 'feishu', id: 'oc_team:om_root' },
      meta: { chatId: 'oc_team' }, createdAt, subscriptions: [] }
    const trigger = { meta: {}, channel: { address: { chatId: 'oc_team', messageId: 'om_root', actorId: 'ou_bob' } } }
    const ctx = {
      ready: Promise.resolve(), config: () => config(), configured: () => true, receive: async () => {}, report: () => {},
      state: { get: (name: string) => state.get(name), set: (name: string, value: unknown) => { state.set(name, value) }, keys: () => [] },
      threads: { subscribed: () => [thread], patch: () => {}, get: () => undefined, describe: () => {} },
      triggers: { latest: () => trigger, get: () => undefined },
      tickets: { get: () => undefined },
    }
    const controller = new AbortController()
    const run = runFeishuImBackfill({ ctx: ctx as never, im, botId: 'bot_1', fingerprint: 'f'.repeat(64),
      config: () => config(), toTrigger: imTrigger, report: () => {}, signal: controller.signal })
    // The recalled message stays gone: the lookup is remembered, so the Thread does not reread it on every round.
    await vi.waitFor(() => expect(state.get(KV.feishu.topic('feishu:oc_team:om_root'))).toEqual({ at: createdAt }))
    controller.abort()
    await run
    expect(asked).toEqual(['message'])
  })

  it('counts the messages dsh-im could not render in the history health', async () => {
    const reports: { detail?: string }[] = []
    const state = new Map<string, unknown>([['feishu-history:oc_team', '2026-09-01T00:00:00.000Z']])
    // A message outside a topic, so this round reads one chat page and counts only its unreadable items.
    const untopiced = event({ reply: { messageId: 'om_1', conversationId: 'oc_team', actorId: 'ou_alice' } })
    const page = { version: 1 as const, scope: 'chat' as const, events: [untopiced], omitted: 2, hasMore: false, coverage: 'provider-visible-messages' }
    const im = service({ chatHistoryChecked: async () => page,
      threadHistoryChecked: async () => ({ ...page, scope: 'thread' as const }) })
    const ctx = {
      ready: Promise.resolve(), config: () => config(), configured: () => true,
      receive: async () => {}, report: (_entry: string, value: { detail?: string }) => { reports.push(value) },
      state: { get: (name: string) => state.get(name), set: (name: string, value: unknown) => { state.set(name, value) }, keys: () => [] },
      threads: { subscribed: () => [], patch: () => {}, get: () => undefined, describe: () => {} },
      triggers: { latest: () => undefined, get: () => undefined },
      tickets: { get: () => undefined },
    }
    const controller = new AbortController()
    const run = runFeishuImBackfill({ ctx: ctx as never, im, botId: 'bot_1', fingerprint: 'f'.repeat(64),
      config: () => config(), toTrigger: imTrigger, report: (_entry, value) => { reports.push(value) }, signal: controller.signal })
    await vi.waitFor(() => expect(reports.some(report => report.detail?.includes('2 unreadable messages skipped'))).toBe(true))
    controller.abort()
    await run
  })

  it('records where it took over instead of replaying the chat on the first round', async () => {
    const received: string[] = []
    const state = new Map<string, unknown>()
    let reads = 0
    const page = { version: 1 as const, scope: 'chat' as const, events: [event()], omitted: 0, hasMore: false, coverage: 'provider-visible-messages' }
    const im = service({ chatHistoryChecked: async () => { reads++; return page },
      threadHistoryChecked: async () => ({ ...page, scope: 'thread' as const }) })
    const ctx = {
      ready: Promise.resolve(), config: () => config(), configured: () => true,
      receive: async (trigger: { id: string }) => { received.push(trigger.id) }, report: () => {},
      state: { get: (name: string) => state.get(name), set: (name: string, value: unknown) => { state.set(name, value) }, keys: () => [] },
      threads: { subscribed: () => [], patch: () => {}, get: () => undefined, describe: () => {} },
      triggers: { latest: () => undefined, get: () => undefined },
      tickets: { get: () => undefined },
    }
    const controller = new AbortController()
    const run = runFeishuImBackfill({ ctx: ctx as never, im, botId: 'bot_1', fingerprint: 'f'.repeat(64),
      config: () => config(), toTrigger: imTrigger, report: () => {}, signal: controller.signal })
    await vi.waitFor(() => expect(typeof state.get(KV.feishu.history('oc_team'))).toBe('string'))
    controller.abort()
    await run
    expect(reads).toBe(0)
    expect(received).toEqual([])
  })
})

describe('dsh-im Feishu context parts and images', () => {
  it('reads the chat around a message and the message itself', async () => {
    const asked: unknown[][] = []
    setDshIm(service({
      historyChecked: async (...args: unknown[]) => {
        asked.push(args)
        return { version: 1, scope: 'nearby', omitted: 0, hasMore: false, coverage: 'provider-visible-human-text', events: [event()] }
      },
      messageChecked: async (...args: unknown[]) => {
        asked.push(args)
        return { version: 1, messageId: 'om_1', chatId: 'oc_team', at: '2026-11-21T10:00:00.000Z', text: 'fix the login page',
          sender: { kind: 'user', id: 'ou_alice', name: 'Alice' }, parentId: 'om_parent', deleted: false }
      },
    }))
    const trigger = imTrigger(event())
    // `around` reads neighbours even when the message belongs to a topic, and asks for everything a nearby read may
    // return before the message, since dsh-im cannot page further back than that window.
    const around = await feishuImKind.threadContext!.read({} as never, trigger, 'around', undefined, config())
    expect(around.messages.map(message => message.id)).toEqual(['om_1'])
    expect(asked[0]?.[2]).toMatchObject({ scope: 'nearby', beforeCount: 500, afterCount: 5, includeDeleted: true })
    const own = await feishuImKind.threadContext!.read({} as never, trigger, 'message', undefined, config())
    expect(own.messages[0]).toMatchObject({ id: 'om_1', reply_to: 'om_parent' })
    expect(asked[1]?.[2]).toMatchObject({ messageId: 'om_1' })
  })

  it('answers a neighbouring window whose source message no longer resolves with unavailable', async () => {
    setDshIm(service({ historyChecked: async () => { throw Object.assign(new Error('gone'), { code: 'stale-route' }) } }))
    const page = await feishuImKind.threadContext!.read({} as never, imTrigger(event()), 'around', undefined, config())
    // The window names the Trigger's own message, so refusing its route means that message cannot be read: an answer
    // with no messages, not a failed read.
    expect(page.status).toBe('unavailable')
    expect(page.messages).toEqual([])
    expect(page.body).toEqual({ status: 'unavailable' })
    // A provider failure stays a failure instead of looking like a message that was withdrawn.
    setDshIm(service({ historyChecked: async () => { throw Object.assign(new Error('unreadable'), { code: 'history-unavailable' }) } }))
    await expect(feishuImKind.threadContext!.read({} as never, imTrigger(event()), 'around', undefined, config()))
      .rejects.toThrow('unreadable')
  })

  it('reports a recalled message as deleted and keeps dsh-im internals out of the page body', async () => {
    const trigger = imTrigger(event())
    setDshIm(service({ messageChecked: async () => { throw Object.assign(new Error('recalled'), { code: 'message-deleted' }) } }))
    const root = await feishuImKind.threadContext!.read({} as never, trigger, 'root', undefined, config())
    expect(root.status).toBe('deleted')
    expect(root.messages).toEqual([])
    // The page an assessment reads carries the mapped messages, never dsh-im's event records or the account fingerprint.
    setDshIm(service())
    const page = await feishuImKind.threadContext!.read({} as never, trigger, 'thread', undefined, config())
    expect(page.body).toMatchObject({ status: 'available', scope: 'thread', messages: page.messages })
    expect(page.body).not.toHaveProperty('events')
    expect(JSON.stringify(page.body)).not.toContain('f'.repeat(64))
  })

  it('reads a topic oldest-first so the message that started it is on the first page', async () => {
    const asked: unknown[][] = []
    setDshIm(service({
      historyChecked: async () => { throw new Error('a topic is read through its own history') },
      threadHistoryChecked: async (...args: unknown[]) => {
        asked.push(args)
        return { version: 1, scope: 'thread', chatId: 'oc_team', threadId: 'omt_1', omitted: 0, hasMore: true, nextCursor: 'cursor_2',
          coverage: 'provider-visible-messages', events: [event({ messageId: 'om_first' })] }
      },
    }))
    const page = await feishuImKind.threadContext!.read({} as never, imTrigger(event()), 'thread', 'cursor_1', config())
    // A context window is the conversation the reader saw: the Source's own answers stay in it, and a recalled message
    // keeps its place as a placeholder.
    expect(asked[0]?.[1]).toEqual({ chatId: 'oc_team', threadId: 'omt_1', limit: 20, includeBots: true,
      includeSelf: true, includeDeleted: true, cursor: 'cursor_1' })
    expect(page.messages.map(message => message.id)).toEqual(['om_first'])
    expect(page.next).toBe('cursor_2')
  })

  it('keeps the Source own answers and a recalled message in a topic window', async () => {
    setDshIm(service({
      threadHistoryChecked: async () => ({ version: 1, scope: 'thread', omitted: 0, hasMore: false,
        coverage: 'provider-visible-messages', events: [
          event({ messageId: 'om_q', text: 'why did the build fail?' }),
          event({ messageId: 'om_a', text: 'the lint gate failed', actor: { kind: 'bot', id: 'cli_bot', name: 'Bot' } }),
          event({ messageId: 'om_gone', text: '', deleted: true,
            reply: { messageId: 'om_gone', conversationId: 'oc_team', actorId: 'ou_alice' } }),
        ] }),
    }))
    const page = await feishuImKind.threadContext!.read({} as never, imTrigger(event()), 'thread', undefined, config())
    expect(page.messages.map(message => message.id)).toEqual(['om_q', 'om_a', 'om_gone'])
    expect(page.messages[1]).toMatchObject({ from: 'Bot', bot: true })
    // A recalled message stays in place with no text, so an assessment sees that something was withdrawn.
    expect(page.messages[2]).toMatchObject({ id: 'om_gone', deleted: true, text: '' })
    expect(page.body).toMatchObject({ status: 'available', messages: page.messages })
  })

  it('reads the chat around a message that sits in no topic', async () => {
    const asked: unknown[][] = []
    setDshIm(service({
      historyChecked: async (...args: unknown[]) => {
        asked.push(args)
        return { version: 1, scope: 'nearby', omitted: 0, hasMore: false, coverage: 'provider-visible-human-text', events: [event()] }
      },
      threadHistoryChecked: async () => { throw new Error('a message without a topic has no thread to read') },
    }))
    // Feishu reported no topic for this message, so `/thread` is the chat around it, not a thread history read.
    const reply = { messageId: 'om_1', conversationId: 'oc_team', actorId: 'ou_alice' }
    const page = await feishuImKind.threadContext!.read({} as never, imTrigger(event({ reply })), 'thread', undefined, config())
    expect(asked[0]?.[2]).toMatchObject({ scope: 'nearby', beforeCount: 5, afterCount: 5 })
    expect(page.messages).toHaveLength(1)
    // The window holds no Topic replies, so the status names it instead of letting an assessment read it as the Thread.
    expect(page.status).toBe('chat_neighbours')
    expect(page.body).toMatchObject({ status: 'chat_neighbours', scope: 'nearby', messages: page.messages })
  })

  it('downloads an image from the message that carries it', async () => {
    const asked: unknown[][] = []
    setDshIm(service({ resourceChecked: async (...args: unknown[]) => {
      asked.push(args)
      return { version: 1, messageId: 'om_parent', fileKey: 'img_x', type: 'image', size: 8,
        data: new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]) }
    } }))
    const saved: unknown[] = []
    const agent = {
      ctx: { get: (name: string) => name === 'attachments' ? { imageLimits: { maxImageBytes: 1_000, maxMessageImageBytes: 1_000 },
        saveImage: async (input: unknown) => { saved.push(input); return { id: 'image_1' } } } : name === 'llm'
        ? { resolveModelInfo: async () => ({ inputModalities: ['image'] }) } : undefined },
      session: { requestHeader: () => ({ config: { provider: 'p', model: 'm' } }) },
      options: { provider: 'p', model: 'm' },
    }
    const { readFeishuImImage } = await import('../src/sources/feishu-im.ts')
    const result = await readFeishuImImage(agent as never, imTrigger(event()), { id: 'om_parent', text: '[图片] img_x' },
      'img_x', new AbortController().signal)
    expect(result.source_message_id).toBe('om_parent')
    // The route still names the Trigger's own message; the download names the message that carries the image.
    expect(asked[0]?.[1]).toMatchObject({ messageId: 'om_1' })
    expect(asked[0]?.[3]).toMatchObject({ messageId: 'om_parent' })
    expect(saved).toHaveLength(1)
  })

  it('downloads a file, voice message, or video and hands the model a saved path', async () => {
    const asked: unknown[][] = []
    setDshIm(service({ resourceChecked: async (...args: unknown[]) => {
      asked.push(args)
      return { version: 1, messageId: 'om_parent', fileKey: 'file_x', type: 'file', size: 5, data: new Uint8Array([104, 101, 108, 108, 111]) }
    } }))
    const saved: unknown[] = []
    const agent = { ctx: { get: (name: string) => name === 'attachments'
      ? { saveFile: async (input: unknown) => { saved.push(input); return { attachmentId: 'sha256:abc', name: 'file_x', bytes: 5 } } } : undefined } }
    const { readFeishuImFile } = await import('../src/sources/feishu-im.ts')
    const result = await readFeishuImFile(agent as never, imTrigger(event()), { id: 'om_parent', text: '[文件] notes.txt file_x' },
      'file_x', new AbortController().signal)
    expect(asked[0]?.[2]).toMatchObject({ fileKey: 'file_x', type: 'file' })
    // The route still names the Trigger's own message; the download names the message that carries the file.
    expect(asked[0]?.[1]).toMatchObject({ messageId: 'om_1' })
    expect(asked[0]?.[3]).toMatchObject({ messageId: 'om_parent' })
    expect(saved).toEqual([{ data: new Uint8Array([104, 101, 108, 108, 111]), name: 'file_x' }])
    expect(result).toMatchObject({ source_message_id: 'om_parent', file_key: 'file_x' })
    // Only keys the handoff rendered into that message are accepted.
    await expect(readFeishuImFile(agent as never, imTrigger(event()), { id: 'om_parent', text: 'no attachment here' },
      'file_x', new AbortController().signal)).rejects.toThrow('Choose a file key')
    expect(feishuImKind.threadContext!.files!('[文件] notes.txt file_x [图片] img_y')).toEqual(['file_x'])
  })

  it('keeps a profile saved with the lark-cli page size', () => {
    expect(feishuImKind.schema.parse({ botId: 'bot_1', readPageSize: 50 })).toMatchObject({ readPageSize: 50 })
    expect(feishuImKind.schema.parse({ botId: 'bot_1', readPageSize: 7 })).toMatchObject({ readPageSize: 7 })
  })

  it('keeps the AppLink dsh-im already read on the Trigger', () => {
    const link = 'https://applink.feishu.cn/client/message/open?message_id=om_1'
    expect(imTrigger(event({ appLink: link })).url).toBe(link)
  })
})

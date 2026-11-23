import { afterEach, describe, expect, it } from 'vitest'
import { z } from 'zod'
import { SettingsSchema } from '../src/policy.ts'
import { SourceRegistry } from '../src/sources/registry.ts'
import type { SourceKind } from '../src/sources/kind.ts'
import type { Agent } from '@deepseek-ai/dsh-agent'
import type { OutboundMessage } from '../src/core/index.ts'
import { Commands } from '../src/infra/commands.ts'
import { HealthBoard } from '../src/infra/health.ts'
import { sourceContextOf, type SourceContextDeps } from '../src/app/source-context.ts'
import { feishuImKind, imTrigger } from '../src/sources/feishu-im.ts'
import { feishuTriggerId } from '../src/sources/feishu-shared.ts'
import { setDshIm, type DshImEvent, type DshImService } from '../src/sources/dsh-im.ts'
import { BOT_ID, FINGERPRINT, feishuEvent } from './fixtures/host.ts'

const probe: SourceKind<{ interval: number }> = {
  kind: 'probe', title: { zh: '探针', en: 'Probe' }, description: { zh: '', en: '' }, health: [],
  schema: z.object({ interval: z.number().int().min(1).default(5) }).strict(),
  fields: [{ key: 'interval', type: 'number', min: 1, max: 60, label: { zh: '间隔', en: 'Interval' } }],
}
/** A dsh-im service whose account read describes the fixture's Feishu bot. */
const dshImService = (overrides: Partial<DshImService> = {}): DshImService => ({
  contractVersion: 1, inboundVersion: 1,
  listBots: async () => [{ botId: BOT_ID, channel: 'feishu' }],
  listTargets: async () => [],
  describeBot: async () => ({ version: 1, botId: BOT_ID, channel: 'feishu', domain: 'feishu',
    account: { fingerprint: FINGERPRINT, name: 'Widgets Bot' }, connected: true, superagent: true, superagentActive: true,
    capabilities: ['exclusive-text-consumer'] }),
  consumeInbound: async () => () => {},
  historyChecked: async () => ({ version: 1, scope: 'thread', events: [], omitted: 0, hasMore: false,
    coverage: 'provider-visible-human-text' }),
  replyChecked: async () => ({ sent: true, messageId: 'om_reply' }),
  ...overrides,
})
afterEach(() => { setDshIm(undefined) })

describe('Source registry', () => {
  it('lists every installed kind with defaults, in id order, followed by stored instances whose kind is not installed', () => {
    const settings = SettingsSchema.parse({ sources: { probe: { enabled: false, config: { interval: 9 } }, retired: { enabled: true, config: { x: 1 } } } })
    const sources = new SourceRegistry(() => settings)
    sources.register(probe); sources.register(feishuImKind)
    expect(sources.instances()).toEqual([
      { id: 'feishu', kind: 'feishu', enabled: true, config: { botId: '', chats: [], directMessages: true, otherChats: 'none',
        readPageSize: 20, linkTimeoutMs: 5000 }, installed: true },
      { id: 'probe', kind: 'probe', enabled: false, config: { interval: 9 }, installed: true },
      { id: 'retired', kind: 'retired', enabled: true, config: { x: 1 }, installed: false },
    ])
  })
  it('validates submitted instances with the kind schema and refuses kinds that are not installed', () => {
    const sources = new SourceRegistry(() => SettingsSchema.parse({}))
    sources.register(probe)
    expect(sources.validate('probe', { enabled: true, config: {} })).toEqual({ enabled: true, config: { interval: 5 } })
    expect(() => sources.validate('probe', { enabled: true, config: { interval: 0 } })).toThrow()
    expect(() => sources.validate('missing', { enabled: true, config: {} })).toThrow('not installed')
  })
  it('sends, links and reads a kind\'s Threads through its own channel and context, and nothing for a kind without them', async () => {
    const sources = new SourceRegistry(() => SettingsSchema.parse({ sources: { probe: { enabled: true, config: { interval: 7 } } } }))
    const sent: OutboundMessage[] = []
    const read: unknown[] = []
    sources.register({ ...probe,
      channel: { send: async (_host, message) => { sent.push(message); return 'sent' },
        link: address => ({ url: `https://probe/${address.room}`, label: { zh: '探针', en: 'Probe' } }) },
      threadContext: { messageId: trigger => String(trigger.meta.id), read: async (_commands, trigger, part, cursor, config) => {
        read.push([trigger.id, part, cursor, config.interval])
        return { messages: [{ id: 'm1', from: 'u', at: '1', text: '' }],
          body: { page: cursor ?? 'first' } }
      } } })
    const commands = {} as Commands
    const message = (source: string): OutboundMessage => ({ id: 'r1', channel: { source, thread: 'probe:1', address: { room: 'r' } },
      text: 'hi', kind: 'reply' })
    expect(await sources.send(message('probe'), commands)).toBe('sent')
    expect(sent).toEqual([message('probe')])
    // A kind that is not installed keeps its replies waiting; links to it have no page.
    expect(await sources.send(message('retired'), commands)).toBe('waiting')
    expect(sources.link(message('probe').channel)).toEqual({ url: 'https://probe/r', label: { zh: '探针', en: 'Probe' } })
    expect(sources.link(message('retired').channel)).toEqual({ url: '', label: { zh: 'retired', en: 'retired' } })
    const trigger = { id: 't1', source: 'probe', threads: [{ source: 'probe', id: '1' }], actor: { source: 'probe', id: 'u' }, human: true,
      at: '', text: 'hi', meta: { id: 'm1' } }
    const context = sources.threadContext(trigger)!
    expect(context.messageId).toBe('m1')
    expect(await context.read(commands, 'thread', '2')).toMatchObject({ body: { page: '2' } })
    expect(read).toEqual([['t1', 'thread', '2', 7]])
    expect(context.images('img_x')).toEqual([])
    await expect(context.image({} as Agent, commands, { id: 'm1', text: '' }, 'img_x', '',
      new AbortController().signal)).rejects.toThrow('no images')
    // A kind without a channel or a context has neither.
    sources.register({ ...probe, kind: 'quiet' })
    expect(await sources.send(message('quiet'), commands)).toBe('gone')
    expect(sources.threadContext({ ...trigger, source: 'quiet' })).toBeUndefined()
  })
  it('refuses a kind that declares a health entry another kind or dsh-superagent itself reports, and gives up its entries at unregister', () => {
    const board = new HealthBoard()
    // Pre-claim built-in health entries; sync:* entries are reserved for settings sync.
    for (const name of ['runtime', 'reply']) board.claim(name)
    const sources = new SourceRegistry(() => SettingsSchema.parse({}), board)
    const unregister = sources.register(feishuImKind)
    expect(() => sources.register({ ...probe, health: ['feishu'] })).toThrow('declares the health entry feishu')
    for (const name of ['runtime', 'reply', 'sync:feishu']) expect(() => sources.register({ ...probe, health: ['probe', name] })).toThrow(name)
    expect(sources.kind('probe')).toBeUndefined()
    sources.register({ ...probe, health: ['probe'] })
    const context = (kind: { kind: string; health: readonly string[] }) => sourceContextOf({ sources } as unknown as SourceContextDeps, kind)
    const first = context(feishuImKind)
    first.report('feishu', { state: 'ready', detail: 'ok' })
    expect(board.all.feishu).toEqual({ state: 'ready', detail: 'ok' })
    expect(() => first.report('probe', { state: 'ready', detail: '' })).toThrow('has no health entry probe')
    // A report sent after its kind unregisters (such as from an exiting CLI listener) is ignored and does not affect a later registration.
    unregister()
    expect(() => first.report('feishu', { state: 'error', detail: 'WARN exiting' })).not.toThrow()
    expect(board.all.feishu).toBeUndefined()
    sources.register(feishuImKind)
    first.report('feishu', { state: 'error', detail: 'late' })
    expect(board.all.feishu).toEqual({ state: 'starting', detail: '' })
    context(feishuImKind).report('feishu', { state: 'ready', detail: 'again' })
    expect(board.all.feishu).toEqual({ state: 'ready', detail: 'again' })
  })
  it('names the Trigger whose message a Trigger\'s message answers, where its kind can', () => {
    const sources = new SourceRegistry(() => SettingsSchema.parse({}))
    sources.register(feishuImKind)
    const event = feishuEvent('2', 'yes')
    const reply = imTrigger({ ...event, reply: { ...event.reply, parentId: 'om_1' } })
    expect(sources.repliesTo(reply)).toBe(feishuTriggerId('om_1'))
    expect(sources.repliesTo({ ...reply, meta: { ...reply.meta, parentId: undefined } })).toBeUndefined()
    expect(sources.repliesTo({ ...reply, source: 'probe' })).toBeUndefined()
  })
  it('finds what a search query names in each kind\'s terms', () => {
    const sources = new SourceRegistry(() => SettingsSchema.parse({}))
    sources.register(feishuImKind)
    expect(sources.find('#12')).toEqual({ threads: [], terms: [] })
    expect(sources.find('https://applink.feishu.cn/client/message/link/open?token=x&amp;om_abc')).toEqual({ threads: [], terms: ['om_abc'] })
    expect(sources.find('see om_abc and om_abc, om_def')).toEqual({ threads: [], terms: ['om_abc', 'om_def'] })
    expect(sources.find('login fails')).toEqual({ threads: [], terms: [] })
    expect(feishuImKind.policy!.unnamed).toEqual({ person: { zh: '飞书用户', en: 'Feishu user' }, bot: { zh: '飞书机器人', en: 'Feishu bot' } })
  })
  it('refuses a second registration of one kind and forgets a kind after its disposer runs', () => {
    const sources = new SourceRegistry(() => SettingsSchema.parse({}))
    const dispose = sources.register(probe)
    expect(() => sources.register(probe)).toThrow('already registered')
    dispose()
    expect(sources.instance('probe')).toBeUndefined()
  })
  it('validates each Source\'s part of the Policy with its kind, keeping parts of kinds that are not installed as stored', () => {
    const sources = new SourceRegistry(() => SettingsSchema.parse({ policy: { sources: { feishu: { allowed: [{ id: 'ou_alice' }] } } } }))
    sources.register(feishuImKind)
    expect(sources.validatePolicy({ feishu: { allowed: [{ id: 'ou_alice', name: 'Alice' }] }, retired: { allowed: [], x: 1 } }))
      .toEqual({ feishu: { allowed: [{ id: 'ou_alice', name: 'Alice' }], syncChat: '', admission: {} }, retired: { allowed: [], x: 1 } })
    expect(() => sources.validatePolicy({ feishu: { allowed: [{ id: 'alice' }] } })).toThrow()
    expect(sources.policyPart('feishu')).toEqual({ allowed: [{ id: 'ou_alice' }], syncChat: '', admission: {} })
    expect(sources.policyPart('retired')).toBeUndefined()
    sources.register(probe)
    expect(sources.policyPart('probe')).toBeUndefined()
  })
  it('allows a reply only on a Channel whose Source still listens to its chat', () => {
    const settings = SettingsSchema.parse({ sources: { feishu: { enabled: true, config: { chats: [{ chatId: 'oc_team', listen: 'mentions' }], directMessages: false } } } })
    const sources = new SourceRegistry(() => settings)
    expect(sources.channelAllowed({ source: 'feishu', address: { chatId: 'oc_team' } })).toBe(false)
    sources.register(feishuImKind)
    expect(sources.channelAllowed({ source: 'feishu', address: { chatId: 'oc_team' } })).toBe(true)
    expect(sources.channelAllowed({ source: 'feishu', address: { chatId: 'oc_other' } })).toBe(false)
    // A direct chat's id starts with oc_ too; the address says it is direct, and with direct messages off it gets no reply.
    expect(sources.channelAllowed({ source: 'feishu', address: { chatId: 'oc_dm', messageId: 'om_1', actorId: 'ou_alice', direct: 'true' } })).toBe(false)
    expect(sources.channelAllowed({ source: 'probe', address: {} })).toBe(false)
  })
  it('delivers a reply to a direct message under the default settings', async () => {
    const sources = new SourceRegistry(() => SettingsSchema.parse({}))
    sources.register(feishuImKind)
    const event: DshImEvent = { ...feishuEvent('dm', 'fix it', { chat_id: 'oc_dm' }),
      conversation: { kind: 'dm', id: 'oc_dm' }, mentions: [], mentionedAccount: false }
    const trigger = imTrigger(event)
    const channel = trigger.channel!
    expect(channel.address).toEqual({ chatId: 'oc_dm', messageId: 'om_dm', actorId: 'ou_alice', botId: BOT_ID, direct: 'true' })
    expect(sources.channelAllowed(channel)).toBe(true)
    const replies: { botId: string; route: DshImEvent['reply']; text: string }[] = []
    setDshIm(dshImService({ replyChecked: async (botId, route, text) => {
      replies.push({ botId, route, text }); return { sent: true, messageId: 'om_reply' }
    } }))
    const reply: OutboundMessage = { id: 'reply-1', channel, text: 'Done', kind: 'reply' }
    expect(await sources.send(reply, {} as Commands)).toBe('sent')
    expect(replies).toEqual([{ botId: BOT_ID, route: expect.objectContaining({ messageId: 'om_dm', conversationId: 'oc_dm', actorId: 'ou_alice' }),
      text: 'Done' }])
  })
})

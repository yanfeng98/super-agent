/** Concurrency, retry, and validation tests for settings saves, pause toggling, and policy synchronization. */
import { afterEach, expect, it, vi } from 'vitest'
import { UserError } from '../src/core/index.ts'
import { Setup } from '../src/app/setup.ts'
import { SettingsConflict } from '../src/app/settings.ts'
import { setDshIm, type DshImService } from '../src/sources/dsh-im.ts'
import { BOT_ID, CHAT, FINGERPRINT, MEMBER, startRuntime, type Host } from './fixtures/host.ts'

let h: Host | undefined
afterEach(async () => { await h?.stop(); h = undefined })
function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>(settle => { resolve = settle })
  return { promise, resolve }
}
const model = { provider: 'worker-provider', model: 'worker-model' }
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
/** The fixture bot's account read, with the fields a test changes. */
const described = (overrides: Partial<Awaited<ReturnType<DshImService['describeBot']>>> = {}) => ({
  version: 1 as const, botId: BOT_ID, channel: 'feishu', domain: 'feishu' as const,
  account: { fingerprint: FINGERPRINT, name: 'Widgets Bot' }, connected: true, superagent: true, superagentActive: true,
  capabilities: ['exclusive-text-consumer'], ...overrides })

it('refuses a save of a block that another save changed during its model check, and saves nothing of it', async () => {
  h = await startRuntime()
  const { settings, store } = h.runtime
  const llm = (h.ctx as unknown as { llm: { resolveCallConfig: ReturnType<typeof vi.fn> } }).llm
  const check = deferred<unknown>()
  llm.resolveCallConfig.mockImplementationOnce(() => check.promise)
  const executor = store.settings().executor
  const revision = store.blockRevision('executor')
  const slow = settings.save('executor', { ...executor, model, maxWorkers: 5 }, revision, 'panel')
  await settings.save('executor', { ...executor, maxWorkers: 2 }, revision, 'panel')
  check.resolve(model)
  await expect(slow).rejects.toThrow(SettingsConflict)
  expect(store.settings().executor).toMatchObject({ maxWorkers: 2, model: executor.model })
})

it('refuses a Policy sync that a Policy save outran, reports it on sync:feishu, and keeps the save', async () => {
  h = await startRuntime()
  const { settings, store } = h.runtime
  const members = deferred<void>()
  setDshIm(dshImService({ chatMembersChecked: async () => {
    await members.promise
    return { version: 1, chatId: CHAT, users: [{ id: MEMBER, name: 'Alice' }, { id: 'ou_new', name: 'New' }] }
  } }))
  const syncing = settings.sync('feishu')
  const policy = store.settings().policy
  const rules = [{ id: 'docs', name: 'Docs', enabled: true, instruction: 'Fix docs' }]
  await settings.save('policy', { ...policy, rules }, store.blockRevision('policy'), 'panel')
  members.resolve()
  await expect(syncing).rejects.toThrow(SettingsConflict)
  expect(h.runtime.health['sync:feishu']).toMatchObject({ state: 'error',
    detail: expect.stringContaining('Policy changed during synchronization') })
  expect(store.settings().policy).toMatchObject({ rules, sources: { feishu: { allowed: [{ id: 'ou_alice', name: 'Alice' }] } } })
})

it('syncs a Policy part again after a failed sync, a minute later and then after a wait that doubles, until it succeeds', async () => {
  h = await startRuntime()
  const { settings } = h.runtime
  const chatMembersChecked = vi.fn(async () => ({ version: 1 as const, chatId: CHAT,
    users: [{ id: MEMBER, name: 'Alice' }] }))
  setDshIm(dshImService({ chatMembersChecked }))
  // A sync that succeeds first leaves the Source ready and no retry pending.
  await settings.sync('feishu')
  chatMembersChecked.mockClear()
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
  try {
    chatMembersChecked.mockRejectedValueOnce(new Error('network down')).mockRejectedValueOnce(new Error('network down'))
    await expect(settings.sync('feishu')).rejects.toThrow('network down')
    expect(h.runtime.health['sync:feishu']).toMatchObject({ state: 'error', detail: expect.stringContaining('network down') })
    const calls = () => chatMembersChecked.mock.calls.length
    const before = calls()
    await vi.advanceTimersByTimeAsync(60_000)
    // The first retry fails too, so the next waits two minutes.
    expect(calls()).toBe(before + 1)
    chatMembersChecked.mockResolvedValueOnce({ version: 1, chatId: CHAT, users: [{ id: MEMBER, name: 'Alice' }] })
    await vi.advanceTimersByTimeAsync(60_000)
    expect(calls()).toBe(before + 1)
    await vi.advanceTimersByTimeAsync(60_000)
    await vi.waitFor(() => expect(h!.runtime.health['sync:feishu']).toMatchObject({ state: 'ready' }))
    expect(calls()).toBe(before + 2)
    // A success ends the retries.
    await vi.advanceTimersByTimeAsync(3_600_000)
    expect(calls()).toBe(before + 2)
  } finally { vi.useRealTimers() }
})
it('records nothing for a pause that changes nothing, refuses a sync while setup waits, and names an unknown block', async () => {
  h = await startRuntime()
  const { settings, store } = h.runtime
  const revision = store.blockRevision('pause')
  const inject = h.agents.get(h.runtime.masterId)!.inject
  inject.mockClear()
  expect(await settings.pause(false, 'panel')).toEqual({ paused: false })
  await h.runtime.idle()
  expect([store.blockRevision('pause'), inject.mock.calls.length, h.runtime.effects.pending('notice')]).toEqual([revision, 0, false])
  vi.spyOn(Setup.prototype, 'configured').mockReturnValue(false)
  await expect(settings.sync('feishu')).rejects.toThrow(new UserError('nothingToSync', { source: 'feishu' }))
  vi.mocked(Setup.prototype.configured).mockRestore()
  expect(() => settings.block('source:x')).toThrow(new UserError('unknownBlock', { block: 'source:x' }))
})

it('adopts the handoff bot\'s owners into the allowed list, so turning the handoff on needs no second list', async () => {
  h = await startRuntime()
  const { settings, store } = h.runtime
  const policy = store.settings().policy
  await settings.save('policy', { ...policy, sources: { ...policy.sources, feishu: { allowed: [{ id: 'ou_bob', name: 'Bob' }], syncChat: '' } } }, store.blockRevision('policy'), 'panel')
  // The owner list is the bot's own; only open_ids can originate work, and '*' or an app id names no person.
  setDshIm(dshImService({ describeBot: async () => described({ ownerOpenIds: [MEMBER, '*', 'cli_not_a_user'] }) }))
  expect(await settings.sync('feishu')).toEqual({ allowed: [{ id: 'ou_bob', name: 'Bob' }, { id: MEMBER }], syncChat: '',
    admission: {}, adoptedOwners: [MEMBER] })
  expect(h.runtime.health['sync:feishu']).toMatchObject({ state: 'ready', detail: '2 allowed' })
  const revision = store.get('settings-revision')
  await settings.sync('feishu')
  expect(store.get('settings-revision')).toBe(revision)
})

it('never re-adds an owner a person removed from the list, while an owner the bot gained later still joins', async () => {
  h = await startRuntime()
  const { settings, store } = h.runtime
  const policy = store.settings().policy
  await settings.save('policy', { ...policy, sources: { ...policy.sources, feishu: { allowed: [], syncChat: '' } } }, store.blockRevision('policy'), 'panel')
  setDshIm(dshImService({ describeBot: async () => described({ ownerOpenIds: [MEMBER] }) }))
  await settings.sync('feishu')
  // The panel keeps `adoptedOwners` when a person removes the owner from the list, as its Policy save carries the whole part.
  const adopted = store.settings().policy
  await settings.save('policy', { ...adopted, sources: { ...adopted.sources,
    feishu: { allowed: [], syncChat: '', adoptedOwners: [MEMBER] } } }, store.blockRevision('policy'), 'panel')
  expect(await settings.sync('feishu')).toEqual({ allowed: [], syncChat: '', admission: {}, adoptedOwners: [MEMBER] })
  // An owner added to the bot in dsh-im after the first adoption still joins at the next sync, once.
  setDshIm(dshImService({ describeBot: async () => described({ ownerOpenIds: [MEMBER, 'ou_new'] }) }))
  expect(await settings.sync('feishu')).toEqual({ allowed: [{ id: 'ou_new' }], syncChat: '', admission: {},
    adoptedOwners: [MEMBER, 'ou_new'] })
})

it('keeps the handoff bot\'s owner beside a sync chat\'s members on every sync, so the roster governs there', async () => {
  h = await startRuntime()
  const { settings, store } = h.runtime
  const policy = store.settings().policy
  await settings.save('policy', { ...policy, sources: { ...policy.sources, feishu: { allowed: [{ id: 'ou_owner' }], syncChat: CHAT } } }, store.blockRevision('policy'), 'panel')
  setDshIm(dshImService({
    describeBot: async () => described({ ownerOpenIds: ['ou_owner'] }),
    chatMembersChecked: async () => ({ version: 1, chatId: CHAT, users: [{ id: MEMBER, name: 'Alice' }] }),
  }))
  expect(await settings.sync('feishu')).toEqual({ allowed: [{ id: MEMBER, name: 'Alice' }, { id: 'ou_owner' }], syncChat: CHAT,
    admission: {}, adoptedOwners: ['ou_owner'] })
  // The roster rebuilds on every sync, and the owner stands beside its members each time.
  expect(await settings.sync('feishu')).toEqual({ allowed: [{ id: MEMBER, name: 'Alice' }, { id: 'ou_owner' }], syncChat: CHAT,
    admission: {}, adoptedOwners: ['ou_owner'] })
})

it('adopts the handoff bot\'s owners from the local handoff status, without verifying the account', async () => {
  h = await startRuntime()
  const { settings, store } = h.runtime
  const policy = store.settings().policy
  await settings.save('policy', { ...policy, sources: { ...policy.sources, feishu: { allowed: [], syncChat: '' } } }, store.blockRevision('policy'), 'panel')
  const describeBot = vi.fn(async () => described())
  setDshIm(dshImService({ describeBot, inboundStatus: async () => ({ version: 1, botId: BOT_ID, superagent: true, superagentActive: true, ownerOpenIds: [MEMBER] }) }))
  expect(await settings.sync('feishu')).toEqual({ allowed: [{ id: MEMBER }], syncChat: '', admission: {}, adoptedOwners: [MEMBER] })
  expect(describeBot).not.toHaveBeenCalled()
})

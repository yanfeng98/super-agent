import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { Store } from '../src/app/store.ts'
import { SettingsSchema } from '../src/policy.ts'
import { SourceRegistry } from '../src/sources/registry.ts'
import { feishuKind } from '../src/sources/feishu.ts'
import { localKind } from '../src/sources/local.ts'
import { githubKind } from '../src/sources/github.ts'
import { snapshot, threadSummary, ticketSummary, triggerView } from '../src/app/views/records.ts'
import { basisFacts, ticketDetail, ticketList, ticketTags } from '../src/app/views/tickets.ts'
import { search, threadDetail } from '../src/app/views/threads.ts'
import { droppedReplies, sourcesStatus } from '../src/app/views/sources.ts'
import { overview } from '../src/app/views/overview.ts'
import { markdown, plain } from '../src/app/views/text.ts'
import { policyHistory } from '../src/app/settings-history.ts'
import { KV } from '../src/kv.ts'
import { feishuLink } from '../src/sources/feishu-channel.ts'
import { feishuChannel, feishuTriggerId } from '../src/sources/feishu-trigger.ts'
import { githubLink } from '../src/sources/github-channel.ts'
import { githubChannel } from '../src/sources/github-trigger.ts'
import { followupOwners, type ChannelRef } from '../src/core/index.ts'

let directory: string
let store: Store
beforeEach(() => {
  directory = mkdtempSync(join(tmpdir(), 'dsh-superagent-views-'))
  store = new Store(join(directory, 'orchestrator.sqlite'))
})
afterEach(() => { store.close(); rmSync(directory, { recursive: true, force: true }) })
const links = (channel: ChannelRef) => (channel.source === 'feishu' ? feishuLink : githubLink)(channel.address, {})
const drop = (id: string, channel: ChannelRef, ticketId?: string) => {
  const message = { id, channel, text: id, kind: 'reply' as const, ...ticketId ? { ticketId } : {} }
  store.receipts.add(message)
  store.receipts.settle(message, 'channelGone')
}

it('names the Thread of each dropped message: the one of the Trigger its Channel answers, or the Channel\'s own', () => {
  // A Feishu Channel names its chat; the message it answers belongs to the Thread of the topic's root message.
  store.triggers.save({ id: feishuTriggerId('om_reply'), source: 'feishu', threads: [{ source: 'feishu', id: 'oc_1:om_root' }],
    actor: { source: 'feishu', id: 'ou_a' }, human: true, at: new Date().toISOString(), text: 'please', meta: {} })
  store.threads.save({ key: 'github:42', ref: { source: 'github', id: '42' }, createdAt: '', subscriptions: [], correlated: [], pending: [],
    meta: {} })
  drop('a', feishuChannel('oc_1', 'om_reply'))
  drop('b', githubChannel(42))
  // A message whose Trigger and Thread the store no longer has links nowhere.
  drop('c', feishuChannel('oc_2', 'om_gone'))
  expect(Object.fromEntries(droppedReplies({ store, links }).map(item => [item.id, item.thread])))
    .toEqual({ a: 'feishu:oc_1:om_root', b: 'github:42', c: undefined })
})

function views() {
  store.saveSettings(SettingsSchema.parse({ policy: { sources: { feishu: { allowed: [{ id: 'ou_alice', name: 'Alice' }] },
    github: { allowed: [{ id: 'alice' }] } },
    rules: [{ id: 'copy', name: 'Copy fixes', enabled: true, instruction: 'Fix copy' }] } }))
  const sources = new SourceRegistry(() => store.settings())
  sources.register(feishuKind); sources.register(githubKind)
  return { store, sources, links }
}
const at = new Date().toISOString()
/** A live ticket with no Assignment, as a follow-up's owner needs one. */
const live = (id: string) => store.tickets.save({ id, title: id, decisionId: 'd', channels: [], undelivered: [], createdAt: at,
  updatedAt: at,
  mandate: { strength: 'explicit', policyVersion: 'v', basis: { via: 'listed' }, actor: { source: 'feishu', id: 'ou_alice' }, scope: '' },
  origin: { triggerId: 'e0', triggerIds: ['e0'] }, assignments: [], inputs: [] })
const message = (id: string, actor: { source: string; id: string }, human = true) => ({ id, source: actor.source,
  threads: [{ source: 'feishu', id: 'oc_1:om_root' }], actor, human, at, text: `message ${id}`, meta: {} })

it('names each sender as the Policy lists it, by its id where the id is readable, and as its kind names one it does not list', () => {
  const view = snapshot(views())
  expect([view.actor({ source: 'feishu', id: 'OU_ALICE' }), view.actor({ source: 'github', id: 'alice' }), view.actor({ source: 'github',
    id: 'bob' }),
    view.actor({ source: 'feishu', id: 'ou_bob' }), view.actor({ source: 'feishu', id: 'ou_bot' }, false), view.actor({ source: 'feishu',
      id: '' })])
    .toEqual([{ source: 'feishu', id: 'OU_ALICE', name: { zh: 'Alice', en: 'Alice' } }, { source: 'github', id: 'alice',
      name: { zh: 'alice', en: 'alice' } },
      { source: 'github', id: 'bob', name: { zh: 'bob', en: 'bob' } }, { source: 'feishu', id: 'ou_bob',
        name: feishuKind.policy!.unnamed!.person },
      { source: 'feishu', id: 'ou_bot', name: feishuKind.policy!.unnamed!.bot }, { source: 'feishu', id: '' }])
  expect(view.rule('copy')).toEqual({ id: 'copy', name: 'Copy fixes' })
  expect(view.rule('gone')).toEqual({ id: 'gone', name: 'gone' })
})

it('classifies a Thread by what it is doing, and offers a filtered Trigger of a person to Eval again', () => {
  const host = views()
  store.threads.save({ key: 'feishu:oc_1:om_root', ref: { source: 'feishu', id: 'oc_1:om_root' }, createdAt: at, subscriptions: [],
    correlated: [],
    pending: ['e2'], meta: {} })
  store.triggers.save(message('e1', { source: 'feishu', id: 'ou_alice' }))
  store.triggers.save(message('e2', { source: 'feishu', id: 'ou_bob' }))
  store.triggers.save(message('e3', { source: 'feishu', id: 'ou_bot' }, false))
  for (const id of ['e1', 'e3']) store.triggers.saveDisposition(id, { filtered: { code: 'not_a_member' }, fanout: [], at })
  const view = snapshot(host)
  expect(threadSummary(view, store.threads.get('feishu:oc_1:om_root')!).stage).toBe('queued')
  const rows = ['e1', 'e2', 'e3'].map(id => triggerView(view, store.triggers.get(id)!))
  // A queued Trigger waits for Eval already, and an automated one is not offered.
  expect(rows.map(row => row.actions.requeue)).toEqual([true, false, false])
  expect(rows[0]!.actor.name).toEqual({ zh: 'Alice', en: 'Alice' })
})

it('states each Source by the first state that holds', () => {
  const host = views()
  const states = (setup: Parameters<typeof sourcesStatus>[2], health: Parameters<typeof sourcesStatus>[1] = {}) =>
    Object.fromEntries(sourcesStatus(host, health, setup).map(source => [source.id, source.state]))
  expect(states({})).toEqual({ feishu: 'starting', github: 'starting' })
  expect(states({}, { feishu: { state: 'ready', detail: '' }, history: { state: 'paused', detail: '' }, github: { state: 'error',
    detail: 'x' } }))
    .toEqual({ feishu: 'paused', github: 'error' })
  const problem = { place: 'deployment', code: 'stateDirUnset', values: {}, detail: '' } as never
  expect(states({ problem }, { feishu: { state: 'ready', detail: '' }, history: { state: 'ready', detail: '' }, github: { state: 'ready',
    detail: '' } }))
    .toEqual({ feishu: 'setup', github: 'setup' })
})

it('follows the Ticket\'s stage: a blocked Ticket a person answered since is working, with no retry, note or turn end', () => {
  const host = views()
  const blocked = { state: 'blocked' as const, summary: 'Needs the design file', at: '2026-09-22T00:01:00Z' }
  const ticket = { id: 't1', title: 'Fix', decisionId: 'd', channels: [], undelivered: [], createdAt: at, updatedAt: at,
    mandate: { strength: 'explicit' as const, policyVersion: 'v', basis: { via: 'listed' as const }, actor: { source: 'feishu',
      id: 'ou_alice' }, scope: '' },
    origin: { triggerId: 'e1', triggerIds: ['e1'] }, assignments: [{ id: 'a1', state: 'running' as const, createdAt: at, startedAt: at,
      report: blocked }],
    inputs: [] as { id: string; kind: 'trigger'; triggerId: string; at: string; human?: true }[] }
  store.tickets.save(ticket)
  const row = ticketSummary(store.tickets.get('t1')!)
  expect(row).toMatchObject({ stage: 'person', action: 'blocked', actions: { retry: true }, note: { text: 'Needs the design file',
    error: false } })
  expect(ticketDetail(host, 't1')!.assignments[0]).toMatchObject({ turnEndedAt: blocked.at })
  store.tickets.save({ ...ticket, inputs: [{ id: 'i1', kind: 'trigger', triggerId: 'e2', at: '2026-09-22T00:02:00Z', human: true }] })
  const answered = ticketSummary(store.tickets.get('t1')!)
  expect(answered).toMatchObject({ stage: 'working', actions: { retry: false } })
  expect(answered).not.toHaveProperty('note')
  expect(ticketDetail(host, 't1')!.assignments[0]).not.toHaveProperty('turnEndedAt')
})

it('does not show a follow-up that reached a ticket as waiting for an Eval, though an older version queued it', () => {
  const host = views()
  live('t1')
  store.triggers.save(message('f1', { source: 'feishu', id: 'ou_alice' }))
  store.triggers.saveDisposition('f1', { fanout: ['t1'], at })
  store.triggers.save(message('f2', { source: 'feishu', id: 'ou_bob' }))
  store.triggers.saveDisposition('f2', { fanout: ['t1'], requeued: true, at })
  store.threads.save({ key: 'feishu:oc_1:om_root', ref: { source: 'feishu', id: 'oc_1:om_root' }, createdAt: at, subscriptions: ['t1'],
    correlated: [],
    pending: ['f1'], meta: {} })
  const view = snapshot(host)
  expect(threadSummary(view, store.threads.get('feishu:oc_1:om_root')!)).toMatchObject({ stage: 'subscribed', pending: 0 })
  expect(triggerView(view, store.triggers.get('f1')!).disposition.queued).toBe(false)
  // One a person sent to Eval again waits for it.
  store.threads.save({ ...store.threads.get('feishu:oc_1:om_root')!, pending: ['f1', 'f2'] })
  expect(threadSummary(snapshot(host), store.threads.get('feishu:oc_1:om_root')!)).toMatchObject({ stage: 'queued', pending: 1 })
})

const load = { running: 0, queued: 0 }
const problems = (host: ReturnType<typeof views>, health: Parameters<typeof overview>[1] = {}) =>
  overview(host, health, 'v1', load).problems

it('counts a Source kind\'s default allowed senders, so a fresh local-ticket install is not told that nobody may ask', () => {
  store.saveSettings(SettingsSchema.parse({}))
  const withLocal = new SourceRegistry(() => store.settings())
  withLocal.register(localKind)
  expect(problems({ store, sources: withLocal, links })).not.toContainEqual({ kind: 'no-allowed' })
  // Without a kind that defaults to allowed senders, the problem is still reported.
  expect(problems({ store, sources: new SourceRegistry(() => store.settings()), links })).toContainEqual({ kind: 'no-allowed' })
})

it('lists an Eval past its deadline, and as dsh-superagent\'s own errors only parts no Source or Policy sync reports', () => {
  const host = views()
  const evaluating = { id: 'e1', threadKey: 'github:1', triggerIds: ['t1'], strength: 'explicit' as const, actor: { source: 'github',
    id: 'alice' },
    basis: { via: 'addressed' as const, addressed: 'mention' as const }, policyVersion: 'v1', startedAt: '2026-01-01T00:00:00Z',
    deadline: '2026-01-01T00:02:00Z' }
  store.threads.save({ key: 'github:1', ref: { source: 'github', id: '1' }, createdAt: at, subscriptions: [], correlated: [], pending: [],
    evaluating, meta: {} })
  const error = (detail: string) => ({ state: 'error' as const, detail })
  const health = { relay: error('relay failing'), reply: error('reply failing'), feishu: error('socket closed'),
    history: error('read failed'),
    'sync:feishu': error('sync failed'), evals: error('unavailable'), launch: { state: 'ready' as const, detail: '' } }
  const found = problems(host, health)
  expect(found).toContainEqual({ kind: 'eval-overdue', thread: expect.objectContaining({ key: 'github:1', stage: 'evaluating' }) })
  // Effects are dsh-superagent's own; a Source shows its parts and its Policy sync, and the Evals are listed one by one.
  expect(found.filter(item => item.kind === 'runtime-error')).toEqual([{ kind: 'runtime-error', name: 'relay', detail: 'relay failing' },
    { kind: 'runtime-error', name: 'reply', detail: 'reply failing' }])
  expect(found.filter(item => item.kind === 'source-error').map(item => item.source.health.name)).toEqual(['feishu', 'history'])
})

it('says no Source runs when every Source is off, and states a Source missing or off', () => {
  const host = views()
  expect(problems(host).map(item => item.kind)).not.toContain('no-source')
  const settings = store.settings()
  store.saveSettings({ ...settings, sources: { feishu: { enabled: false, config: {} }, github: { enabled: false, config: {} },
    jira: { enabled: true, config: { site: 'acme' } } } })
  expect(problems(host).map(item => item.kind)).toContain('no-source')
  expect(Object.fromEntries(sourcesStatus(host, {}, {}).map(source => [source.id, [source.state, source.installed]])))
    .toEqual({ feishu: ['off', true], github: ['off', true], jira: ['missing', false] })
})

it('sends text without HTML, entity-encoded tags included, and with images named', () => {
  const text = '<div>Hello <b>there</b></div> &lt;script&gt;alert(1)&lt;/script&gt; <!-- note --> ![shot](https://x/y.png) [Image: diagram]'
  expect(plain(text)).toBe('Hello there alert(1) [image] [image]')
  expect(markdown(`**Fix** ${text}\n\n- one`)).toBe('**Fix** Hello there alert(1)  [image] [image]\n\n- one')
  expect(plain('[the docs](https://example.com) and `code` &amp; more')).toBe('the docs and code & more')
})

it('words each settings save by what it changed from the one before, and nothing after a gap in the history', () => {
  const host = views()
  const first = store.settings()
  const allowed = (entries: { id: string; name?: string }[]) => ({ ...first.policy, sources: { ...first.policy.sources,
    feishu: { ...first.policy.sources.feishu!, allowed: entries } } })
  const alice = { id: 'ou_alice', name: 'Alice Liu' }, bob = { id: 'ou_bob', name: 'Bob' }
  store.saveSettings({ ...first, policy: allowed([alice, bob, { id: 'ou_carol' }]), paused: true,
    sources: { ...first.sources, feishu: { enabled: false, config: {} } },
    executor: { ...first.executor, directory: '/srv/work', prepare: 'npm ci', blockedCommands: ['rm', 'curl'] } })
  const second = store.settings()
  store.saveSettings({ ...second, policy: allowed([alice, { id: 'ou_carol' }]), paused: false, sources: { ...second.sources,
    feishu: { enabled: true, config: {} } } })
  const changes = (revision: number) => policyHistory(store, host.sources, 'v').items.find(item => item.revision === revision)!.changes
  const revision = store.get(KV.settings.revision)!
  expect(changes(revision - 1)).toEqual([
    { code: 'sourceRowsAdded', source: 'feishu', part: true, field: 'allowed', items: ['Bob', 'ou_carol'] },
    { code: 'sourceRowsChanged', source: 'feishu', part: true, field: 'allowed', items: ['Alice Liu'] },
    { code: 'paused' }, { code: 'executionDirectory', from: '', to: '/srv/work' }, { code: 'prepareCommand' },
    { code: 'blockedCommands', from: '', to: 'rm curl' }, { code: 'sourceDisabled', source: 'feishu' }])
  expect(changes(revision)).toEqual([{ code: 'sourceRowsRemoved', source: 'feishu', part: true, field: 'allowed', items: ['Bob'] },
    { code: 'resumed' }, { code: 'sourceEnabled', source: 'feishu' }])
  // A save whose predecessor is missing from the history has nothing to compare with.
  store.delete(KV.settings.version(revision))
  store.saveSettings({ ...store.settings(), paused: true })
  expect(changes(revision + 1)).toBeNull()
})

it('names a listed bot as the Policy lists it, and gives a bot\'s Mandate a basis fact of its own', () => {
  store.saveSettings(SettingsSchema.parse({ policy: { sources: { feishu: { allowed: [], bots: [{ id: 'cli_review', name: '评审机器人' }] } } } }))
  const sources = new SourceRegistry(() => store.settings())
  sources.register(feishuKind)
  const view = snapshot({ store, sources, links })
  expect(view.actor({ source: 'feishu', id: 'cli_review' }, false).name).toEqual({ zh: '评审机器人', en: '评审机器人' })
  expect(basisFacts({ via: 'bot', sender: '评审机器人', addressed: 'mention', observation: { mode: 'auto' } }, view.rule).facts)
    .toEqual([{ name: 'sender', value: '评审机器人' }, { name: 'bot' }, { name: 'addressed', value: 'mention' }, { name: 'mode', value: 'auto' }])
})

it('shows a follow-up that reached a ticket as delivered, the tickets that handed a Trigger back with why, and the Threads a ticket follows for context', () => {
  const host = views()
  live('t1')
  store.threads.save({ key: 'feishu:oc_1:om_root', ref: { source: 'feishu', id: 'oc_1:om_root' }, createdAt: at, subscriptions: ['t1'],
    correlated: [],
    pending: [], meta: {} })
  store.triggers.save(message('f1', { source: 'feishu', id: 'ou_alice' }))
  store.triggers.saveDisposition('f1', { fanout: ['t1'], at })
  store.triggers.save(message('f2', { source: 'feishu', id: 'ou_alice' }))
  store.triggers.saveDisposition('f2', { fanout: ['t1'], requeued: true, handbacks: [{ ticketId: 't1', reason: 'Asks for a new feature', at }], at })
  const view = snapshot(host)
  expect(triggerView(view, store.triggers.get('f1')!).disposition).toMatchObject({ delivered: true, queued: false, handbacks: [] })
  // A Trigger handed back goes to Eval as any message on its Thread; the Ticket that handed it back is named by its title once it has one.
  expect(triggerView(view, store.triggers.get('f2')!).disposition).toMatchObject({ delivered: false,
    handbacks: [{ ticketId: 't1', title: 't1', reason: 'Asks for a new feature' }] })
  store.threads.save({ key: 'github:7', ref: { source: 'github', id: '7' }, createdAt: at, subscriptions: ['t1'], context: ['t1'],
    correlated: [], pending: [], meta: {} })
  expect(threadDetail(host, 'github:7', undefined, 50)?.context).toEqual(['t1'])
  store.tickets.save({ id: 't1', title: 'Fix', decisionId: 'd', channels: [], undelivered: [], createdAt: at, updatedAt: at,
    mandate: { strength: 'explicit', policyVersion: 'v', basis: { via: 'listed' }, actor: { source: 'feishu', id: 'ou_alice' }, scope: '' },
    origin: { triggerId: 'e1', triggerIds: ['e1'] }, assignments: [],
    inputs: [{ id: 'i1', kind: 'trigger', triggerId: 'f1', at }, { id: 'i2', kind: 'trigger', triggerId: 'f2', at }] })
  const page = ticketDetail(host, 't1')!
  expect(Object.fromEntries(page.threads.map(thread => [thread.key, !!thread.context]))).toEqual({ 'feishu:oc_1:om_root': false,
    'github:7': true })
  expect(page.inputs.map(input => input.kind === 'trigger' && input.trigger?.handback)).toEqual([{ reason: 'Asks for a new feature', at },
    undefined])
  expect(triggerView(snapshot(host), store.triggers.get('f2')!).disposition.handbacks[0]?.title).toBe('Fix')
})

it('names a bot\'s Mandate as a bot once the Policy no longer lists it', () => {
  const host = views()
  const mandate = { strength: 'inferred' as const, policyVersion: 'v', basis: { via: 'bot' as const,
    observation: { mode: 'auto' as const } },
    actor: { source: 'feishu', id: 'cli_removed' }, scope: '' }
  store.tickets.save({ id: 't1', title: 'Fix', decisionId: 'd', channels: [], undelivered: [], createdAt: at, updatedAt: at, mandate,
    origin: { triggerId: 'e1', triggerIds: ['e1'] }, assignments: [], inputs: [] })
  expect(ticketDetail(host, 't1')!.mandate.actor).toEqual({ source: 'feishu', id: 'cli_removed', name: feishuKind.policy!.unnamed!.bot })
})

it('offers a person to send a follow-up to Eval while a ticket decides it, or once nothing did', () => {
  const host = views()
  live('t1')
  store.threads.save({ key: 'feishu:oc_1:om_root', ref: { source: 'feishu', id: 'oc_1:om_root' }, createdAt: at, subscriptions: ['t1'],
    correlated: [],
    pending: [], meta: {} })
  store.triggers.save(message('f1', { source: 'feishu', id: 'ou_alice' }))
  store.triggers.saveDisposition('f1', { fanout: ['t1'], at })
  const row = () => triggerView(snapshot(host), store.triggers.get('f1')!)
  expect(row()).toMatchObject({ actions: { requeue: true }, disposition: { delivered: true } })
  // A ticket that ended after its worker had it leaves it to nobody: a person may still send it to Eval.
  store.tickets.save({ ...store.tickets.get('t1')!, ended: { at, by: 'executor', reason: 'done', outcome: 'completed' } })
  expect(row()).toMatchObject({ actions: { requeue: true }, disposition: { delivered: false } })
  // Not once its worker acknowledged it, which took it up; a stop on it decided only the ticket's work.
  store.triggers.saveDisposition('f1', { fanout: ['t1'], acked: ['t1'], at })
  expect(row().actions.requeue).toBe(false)
  store.decisions.save({ id: 'stop', evalId: 'e', threadKey: 'feishu:oc_1:om_root', triggerIds: ['f1'], policyVersion: 'v', at, kind: 'stop',
    reason: [{ code: 'humanStop' }], strength: 'explicit', ticketId: 't1' })
  store.triggers.saveDisposition('f1', { fanout: ['t1'], decisionId: 'stop', at })
  expect(row().actions.requeue).toBe(true)
  // So once the stop ended the ticket, which let go of the message, as the Engine records it.
  store.triggers.saveDisposition('f1', { fanout: [], decisionId: 'stop', at })
  expect(row().actions.requeue).toBe(true)
  // Once sent, it waits for the Eval.
  store.threads.save({ ...store.threads.get('feishu:oc_1:om_root')!, pending: ['f1'] })
  store.triggers.saveDisposition('f1', { fanout: ['t1'], requeued: true, at })
  expect(row()).toMatchObject({ actions: { requeue: false }, disposition: { queued: true } })
})

it('reads each ticket a page\'s follow-ups reached once, and nothing for a message no ticket can decide', () => {
  const host = views()
  live('t1')
  store.threads.save({ key: 'feishu:oc_1:om_root', ref: { source: 'feishu', id: 'oc_1:om_root' }, createdAt: at, subscriptions: ['t1'],
    correlated: [], pending: [], meta: {} })
  for (const id of ['f1', 'f2', 'f3']) {
    store.triggers.save(message(id, { source: 'feishu', id: 'ou_alice' }))
    store.triggers.saveDisposition(id, { fanout: ['t1'], at })
  }
  const view = snapshot(host)
  const tickets = vi.spyOn(store.tickets, 'get')
  expect(['f1', 'f2', 'f3'].map(id => triggerView(view, store.triggers.get(id)!).disposition.delivered)).toEqual([true, true, true])
  expect(tickets).toHaveBeenCalledOnce()
  tickets.mockRestore()
  // A message the ticket handed back is decided by no ticket, which needs no read.
  store.triggers.saveDisposition('f3', { fanout: ['t1'], handbacks: [{ ticketId: 't1', reason: 'Another change', at }], at })
  const records = { tickets: { get: vi.fn() }, threads: { get: vi.fn() }, triggers: store.triggers }
  expect(followupOwners(store.triggers.get('f3')!, records)).toEqual([])
  expect([records.tickets.get, records.threads.get].map(read => read.mock.calls.length)).toEqual([0, 0])
})

it('numbers tickets in creation order and keeps the number across edits', () => {
  const host = views()
  live('t1'); live('t2')
  expect(ticketDetail(host, 't1')!.number).toBe(1)
  expect(ticketDetail(host, 't2')!.number).toBe(2)
  store.tickets.save({ ...store.tickets.get('t1')!, title: '改名之后' })
  expect(ticketDetail(host, 't1')!.number).toBe(1)
})

it('offers merge, discard and rebase only on an ended ticket with a review branch, and cleanup once merged', () => {
  const host = views()
  live('t1')
  const ended = { ...store.tickets.get('t1')!, execution: { name: 't1', directory: '/w/t1', assignment: '' },
    ended: { at, by: 'executor' as const, outcome: 'completed' as const },
    review: { capturedAt: at, branch: 'agent/t1', base: 'a'.repeat(40), head: 'b'.repeat(40), commits: [], files: [], dirty: [] } }
  store.tickets.save(ended)
  expect(ticketDetail(host, 't1')!.actions).toMatchObject({ reopen: true, merge: true, discard: true, rebase: true, cleanup: false })
  store.tickets.save({ ...ended, merged: { at, branch: 'agent/t1', target: 'main' } })
  expect(ticketDetail(host, 't1')!.actions).toMatchObject({ reopen: false, merge: false, discard: false, rebase: false, cleanup: true })
  store.tickets.save({ ...ended, merged: { at, branch: 'agent/t1', target: 'main' }, cleaned: { at } })
  expect(ticketDetail(host, 't1')!.actions).toMatchObject({ cleanup: false, reopen: false })
  store.tickets.save({ ...ended, discarded: { at } })
  expect(ticketDetail(host, 't1')!.actions).toMatchObject({ reopen: false, merge: false, discard: false, cleanup: false })
  // Undoing a merge is offered while it stands, and not for one a revert already undid.
  const merged = { ...ended, merged: { at, branch: 'agent/t1', target: 'main', root: '/repo' } }
  store.tickets.save(merged)
  expect(ticketDetail(host, 't1')!.actions).toMatchObject({ revert: true })
  store.tickets.save({ ...merged, reverted: { at: '2999-01-01T00:00:00.000Z', commit: 'c'.repeat(40), target: 'main',
    merge: 'b'.repeat(40) } })
  expect(ticketDetail(host, 't1')!.actions).toMatchObject({ revert: false })
})

it('offers a check run on an ended ticket with a workspace when a check command is configured, and shows the result', () => {
  const host = views()
  live('t1')
  const settings = store.settings()
  store.saveSettings(SettingsSchema.parse({ ...settings, executor: { ...settings.executor, check: 'pnpm test' } }))
  const ended = { ...store.tickets.get('t1')!, execution: { name: 't1', directory: '/w/t1', assignment: '' },
    ended: { at, by: 'executor' as const, outcome: 'completed' as const } }
  store.tickets.save(ended)
  expect(ticketDetail(host, 't1')!.actions).toMatchObject({ check: true })
  expect(ticketDetail(host, 't1')!.checks).toBeUndefined()
  store.tickets.save({ ...ended, checks: { command: 'pnpm test', ok: false, exitCode: 1, at, output: '1 failed' } })
  expect(ticketDetail(host, 't1')!.checks).toMatchObject({ ok: false, exitCode: 1, output: '1 failed' })
  // An open ticket has no workspace check yet.
  store.tickets.save({ ...store.tickets.get('t1')!, ended: undefined, checks: undefined })
  expect(ticketDetail(host, 't1')!.actions).toMatchObject({ check: false })
})

it('lists the person actions on a ticket in time order', () => {
  const host = views()
  live('t1')
  store.tickets.save({ ...store.tickets.get('t1')!,
    approved: { at: '2026-09-20T00:00:00Z' },
    renamed: { at: '2026-09-21T00:00:00Z', title: '新的标题' },
    rebased: { at: '2026-09-22T00:00:00Z', branch: 'agent/t1', target: 'main', base: 'c'.repeat(40) },
    merged: { at: '2026-09-23T00:00:00Z', branch: 'agent/t1', target: 'main' },
    cleaned: { at: '2026-09-24T00:00:00Z' } })
  expect(ticketDetail(host, 't1')!.history).toEqual([
    { code: 'approved', at: '2026-09-20T00:00:00Z' },
    { code: 'renamed', at: '2026-09-21T00:00:00Z', title: '新的标题' },
    { code: 'rebased', at: '2026-09-22T00:00:00Z', branch: 'agent/t1', target: 'main' },
    { code: 'merged', at: '2026-09-23T00:00:00Z', branch: 'agent/t1', target: 'main' },
    { code: 'cleaned', at: '2026-09-24T00:00:00Z' }])
})

it('lists the ended Tickets a person still has to review, and drops the approved ones', () => {
  const host = views()
  const ended = (id: string, patch: (ticket: NonNullable<ReturnType<typeof store.tickets.get>>) => void) => {
    live(id)
    const ticket = store.tickets.get(id)!
    ticket.ended = { at, by: 'executor', outcome: 'completed' }
    patch(ticket)
    store.tickets.save(ticket)
  }
  ended('open-review', () => { /* no approval yet */ })
  ended('approved', ticket => { ticket.reviewed = { at, decision: 'approved', via: 'panel' } })
  ended('merged', ticket => { ticket.merged = { at, branch: 'agent/m', target: 'main', head: 'abc' } })
  live('working')
  expect(ticketList(host, 'review', 0, 20).items.map(item => item.id)).toEqual(['open-review'])
  // An approved Ticket comes back to the list only after a new commit clears the decision.
  const ticket = store.tickets.get('approved')!
  delete ticket.reviewed
  store.tickets.save(ticket)
  expect(ticketList(host, 'review', 0, 20).items.map(item => item.id).sort()).toEqual(['approved', 'open-review'])
})

it('lists review notes with their file, line and text, and marks one the changes moved past', () => {
  const host = views()
  live('t1')
  store.tickets.save({ ...store.tickets.get('t1')!, notes: [{ id: 'n1', file: 'src/a.ts', line: 12, text: '改成可选', at,
    commit: 'old' }], review: { capturedAt: at, head: 'new', commits: [], files: [], dirty: [] } })
  expect(ticketDetail(host, 't1')!.notes).toEqual([{ id: 'n1', file: 'src/a.ts', line: 12, text: '改成可选', at, commit: 'old',
    outdated: true }])
})

it('lists related tickets with their number and title', () => {
  const host = views()
  live('t1'); live('t2')
  store.tickets.save({ ...store.tickets.get('t1')!, related: ['t2', 'gone'] })
  expect(ticketDetail(host, 't1')!.related).toEqual([{ id: 't2', number: 2, title: 't2' }])
  expect(ticketDetail(host, 't2')!.related).toEqual([])
})

it('shows the full request text and the archive file on a local ticket\'s page', () => {
  const host = views()
  live('t1')
  const text = `# 整理首页\n\n${'细节 '.repeat(80)}`
  const channel = { source: 'local', thread: 'local:req_1', address: { request: 'req_1', file: '/tmp/tickets/req_1.md' } }
  store.triggers.save({ id: 'e9', source: 'local', threads: [{ source: 'local', id: 'req_1' }], actor: { source: 'local', id: 'operator' },
    human: true, at, text, channel, meta: { requestId: 'req_1', messageId: 'req_1', title: '整理首页' } })
  store.tickets.save({ ...store.tickets.get('t1')!, origin: { triggerId: 'e9', triggerIds: ['e9'] }, channels: [channel],
    inputs: [{ id: 'i1', kind: 'trigger', triggerId: 'e9', at }] })
  const page = ticketDetail(host, 't1')!
  expect(page.archive).toBe('/tmp/tickets/req_1.md')
  // A preview stays short for compact rows, while the page carries the request in full.
  expect(page.request).toMatchObject({ at })
  expect(page.request!.text).toContain('整理首页')
  expect(page.request!.text.length).toBeGreaterThan(160)
})

it('lists the tags on requests and filters Tickets by one', () => {
  const host = views()
  live('t1'); live('t2')
  store.tickets.save({ ...store.tickets.get('t2')!, origin: { triggerId: 'e1', triggerIds: ['e1'] } })
  store.triggers.save({ id: 'e0', source: 'local', threads: [{ source: 'local', id: 'req' }], actor: { source: 'local', id: 'operator' },
    human: true, at, text: '带标签', meta: { tags: ['前端', 'p1'] } })
  store.triggers.save({ id: 'e1', source: 'local', threads: [{ source: 'local', id: 'req2' }], actor: { source: 'local', id: 'operator' },
    human: true, at, text: '另一个', meta: { tags: ['后端'] } })
  expect(new Set(ticketTags(store))).toEqual(new Set(['p1', '前端', '后端']))
  expect(ticketList(host, 'open', 0, 20, '前端').items.map(item => item.id)).toEqual(['t1'])
  expect(ticketList(host, 'open', 0, 20, '后端').items.map(item => item.id)).toEqual(['t2'])
  expect(ticketList(host, 'open', 0, 20).items.map(item => item.id).sort()).toEqual(['t1', 't2'])
  // Labels a person edited on the Ticket replace the request's, and an empty list clears them instead of falling back.
  store.tickets.save({ ...store.tickets.get('t1')!, tags: ['设置'], tagged: { at } })
  expect(new Set(ticketTags(store))).toEqual(new Set(['后端', '设置']))
  expect(ticketList(host, 'open', 0, 20, '前端').items).toEqual([])
  expect(ticketList(host, 'open', 0, 20, '设置').items.map(item => item.id)).toEqual(['t1'])
  expect(ticketList(host, 'open', 0, 20, '后端').items.map(item => item.id)).toEqual(['t2'])
  expect(ticketDetail(host, 't1')!.history).toContainEqual(expect.objectContaining({ code: 'tagged', tags: ['设置'] }))
  // An empty list clears the Ticket's labels instead of falling back to the request's.
  store.tickets.save({ ...store.tickets.get('t2')!, tags: [] })
  expect(ticketDetail(host, 't2')!.tags).toEqual([])
  expect(ticketList(host, 'open', 0, 20, '后端').items).toEqual([])
})

it('narrows Ticket search with is, tag and author qualifiers', () => {
  const host = views()
  host.sources.register(localKind)
  live('t1'); live('t2')
  const ended = { at, by: 'human' as const, outcome: 'stopped' as const }
  store.tickets.save({ ...store.tickets.get('t2')!, ended,
    mandate: { ...store.tickets.get('t2')!.mandate, actor: { source: 'feishu', id: 'ou_bob' } },
    origin: { triggerId: 'e1', triggerIds: ['e1'] } })
  store.triggers.save({ id: 'e0', source: 'local', threads: [{ source: 'local', id: 'r1' }], actor: { source: 'local', id: 'operator' },
    human: true, at, text: '首页按钮', meta: { tags: ['p1'] } })
  store.triggers.save({ id: 'e1', source: 'local', threads: [{ source: 'local', id: 'r2' }], actor: { source: 'local', id: 'operator' },
    human: true, at, text: '设置页面', meta: {} })
  const ids = (query: string) => search(host, query, new Map()).tickets.map(item => item.id).sort()
  expect(ids('is:open')).toEqual(['t1'])
  expect(ids('is:ended')).toEqual(['t2'])
  expect(ids('tag:p1')).toEqual(['t1'])
  expect(ids('label:p1')).toEqual(['t1'])
  const day = at.slice(0, 10)
  expect(ids(`created:${day}`)).toEqual(['t1', 't2'])
  expect(ids(`created:<${day}`)).toEqual([])
  expect(ids(`created:>=${day}`)).toEqual(['t1', 't2'])
  expect(ids(`updated:<=${day}`)).toEqual(['t1', 't2'])
  expect(ids('created:not-a-date')).toEqual([])
  expect(ids('author:Alice')).toEqual(['t1'])
  expect(ids('author:ou_bob')).toEqual(['t2'])
  expect(ids('is:open 首页')).toEqual(['t1'])
  expect(ids('is:ended 首页')).toEqual([])
  expect(ids('tag:p1 首页')).toEqual(['t1'])
  // An unknown `is:` value stays free text rather than silently matching everything.
  expect(ids('is:merged')).toEqual([])
})

it('finds a ticket by a reply in its archive as well as by its title', () => {
  const host = views()
  host.sources.register(localKind)
  live('t1')
  const ticket = store.tickets.get('t1')!
  store.tickets.save({ ...ticket, title: '文案修复' })
  store.receipts.add({ id: 'r1', channel: { source: 'local', thread: 'local:r', address: { request: 'r' } }, kind: 'reply',
    ticketId: 't1', text: '把按钮文案改成保存' })
  expect(search(host, '按钮文案', new Map()).tickets.map(item => item.id)).toEqual(['t1'])
  // A tag on the request is searchable too.
  store.triggers.save({ id: 'e0', source: 'local', threads: [{ source: 'local', id: 'req' }], actor: { source: 'local', id: 'operator' },
    human: true, at, text: '文案修复', meta: { tags: ['前端'] } })
  expect(search(host, '前端', new Map()).tickets.map(item => item.id)).toEqual(['t1'])
  expect(search(host, '文案修复', new Map()).tickets.map(item => item.id)).toEqual(['t1'])
  expect(search(host, '没有这个词', new Map()).tickets).toEqual([])
  // A person's request edit replaces the text the page shows and search matches.
  store.tickets.save({ ...store.tickets.get('t1')!, request: { text: '把按钮文案改成保存并加图标', at } })
  expect(ticketDetail(host, 't1')!.request).toMatchObject({ text: '把按钮文案改成保存并加图标', editedAt: at })
  expect(search(host, '加图标', new Map()).tickets.map(item => item.id)).toEqual(['t1'])
  expect(search(host, '旧的请求正文', new Map()).tickets).toEqual([])
})

it('pages matching Tickets so more than the first fifty are reachable', () => {
  const host = views()
  for (let index = 0; index < 55; index += 1) live(`t${index}`)
  const first = search(host, 't', new Map())
  expect(first.tickets).toHaveLength(50)
  expect(first.next_offset).toBe(50)
  const next = search(host, 't', new Map(), 50)
  expect(next.tickets).toHaveLength(5)
  expect(next.next_offset).toBeNull()
})

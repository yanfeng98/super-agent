import { describe, expect, it, vi } from 'vitest'
import { Engine, type Assessor, type Executor, type Ticket, type Trigger } from '../src/core/index.ts'
import { feishuImKind } from '../src/sources/feishu-im.ts'
import { SourceRegistry } from '../src/sources/registry.ts'
import { Store } from '../src/app/store.ts'
import { Commands } from '../src/infra/commands.ts'
import { HealthBoard } from '../src/infra/health.ts'
import { testDeployment } from './fixtures/deployment.ts'
import { SettingsSchema, settingsPolicy } from '../src/policy.ts'
import { sourceContextOf, type SourceContextDeps } from '../src/app/source-context.ts'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

function host() {
  const store = new Store(join(mkdtempSync(join(tmpdir(), 'sources-')), 'db.sqlite'))
  const settings = SettingsSchema.parse({ policy: { sources: { feishu: { allowed: [{ id: 'ou_alice', name: 'Alice' }], syncChat: 'oc_team' } } }, eval: { mode: 'auto' }, sources: { feishu: { enabled: true, config: { chats: [{ chatId: 'oc_team', listen: 'mentions' }, { chatId: 'oc_observed', listen: 'all' }], directMessages: true } } } })
  store.saveSettings(settings)
  const board = new HealthBoard()
  const sources = new SourceRegistry(() => store.settings(), board)
  const deployment = testDeployment()
  sources.register(feishuImKind)
  const assessor: Assessor = { request: vi.fn(async () => {}) }
  const executor: Executor = { start: vi.fn(async () => {}), deliver: vi.fn(async () => {}), stop: vi.fn(async () => {}) }
  const outbox = { send: () => {}, status: () => undefined, withdraw: () => {} }
  const engine = new Engine({ repo: store, scope: sources, policy: () => settingsPolicy(store.settings(), sources), assessor, executor, outbox })
  const commands = new Commands({}, new AbortController().signal)
  const receive = vi.fn(async (trigger: Trigger) => { const result = engine.receive(trigger); engine.evaluateDue(); return result })
  // Record outbound messages in the store without sending them.
  const parts: SourceContextDeps = { store, engine, outbox: { send: message => { store.receipts.add(message) } }, commands, sources, deployment,
    ready: Promise.resolve(), configured: () => true, receive, stopThreadWork: async () => undefined }
  // The Sources' functions take the context; plugin entries and kind callbacks take the host, which builds each kind's context.
  const value = { ...parts, context: (kind: { kind: string; health: readonly string[] }) => sourceContextOf(parts, kind) }
  const context = sourceContextOf(parts, { kind: 'feishu', health: ['feishu', 'history'] })
  return { ...value, ...context, health: board.all, store, engine, commands: parts.commands, receive, executor }
}
const openTicket = (id: string): Ticket => ({ id, title: id, decisionId: `d-${id}`, mandate: { strength: 'explicit', policyVersion: '',
  basis: { via: 'listed' }, actor: { source: 'feishu', id: 'ou_alice' }, scope: '' },
  channels: [], assignments: [], inputs: [], undelivered: [], createdAt: '', updatedAt: '', origin: { triggerId: `source-${id}`, triggerIds: [] } })
const execution = (name: string) => ({ name, directory: `/w/${name}`, assignment: '' })
describe('source context', () => {
  it('lets a kind report only its own health entries', () => {
    const h = host()
    const feishu = h.context(feishuImKind)
    feishu.report('feishu', { state: 'ready', detail: 'ok' })
    expect(h.health.feishu).toEqual({ state: 'ready', detail: 'ok' })
    expect(() => feishu.report('probe', { state: 'error', detail: 'x' })).toThrow('no health entry probe')
    h.store.close()
  })
  it('patches a Thread\'s meta as it is now, so a change another writer made meanwhile stays', () => {
    const h = host()
    h.store.threads.save({ key: 'feishu:7', ref: { source: 'feishu', id: '7' }, createdAt: '', subscriptions: [], correlated: [],
      pending: [], meta: { title: 'old', pollError: 'boom' } })
    const snapshot = h.store.threads.get('feishu:7')!
    h.store.threads.save({ ...snapshot, subscriptions: ['t'], meta: { ...snapshot.meta, label: '#7 old' } })
    h.threads.patch('feishu:7', { title: 'new', pollError: undefined })
    expect(h.store.threads.get('feishu:7')).toMatchObject({ subscriptions: ['t'], meta: { title: 'new', label: '#7 old' } })
    expect(h.store.threads.get('feishu:7')!.meta).not.toHaveProperty('pollError')
    // A Thread that does not exist is not created.
    h.threads.patch('feishu:8', { title: 'x' })
    expect(h.store.threads.get('feishu:8')).toBeUndefined()
    h.store.close()
  })
  it('reads the configuration of an instance that is on, removes a record set to undefined, and merges ticket meta', () => {
    const h = host()
    expect(h.config('feishu')).toMatchObject({ directMessages: true })
    h.store.saveSettings({ ...h.store.settings(), sources: { ...h.store.settings().sources, feishu: { enabled: false, config: {} } } })
    expect(h.config('feishu')).toBeUndefined()
    h.state.set('probe-retry', { 1: {} })
    h.state.set('probe-retry', undefined)
    expect(h.store.keys('probe-retry')).toEqual([])
    h.store.tickets.save({ ...openTicket('t1'), execution: execution('one') })
    h.tickets.bindThread('t1', { source: 'feishu', id: '4' }, true)
    // The ticket follows the Thread; its other fields stay as they were.
    expect(h.store.tickets.get('t1')).toMatchObject({ follows: [{ thread: 'feishu:4', own: true }], execution: execution('one'),
      origin: { triggerId: 'source-t1' } })
    h.store.close()
  })
})

import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { Store } from '../src/app/store.ts'
import { rekeyFeishuTopicThreads } from '../src/app/repair.ts'
import { KV } from '../src/kv.ts'
import type { Decision, Thread, Ticket, Trigger } from '../src/core/index.ts'

let directory: string
beforeEach(() => { directory = mkdtempSync(join(tmpdir(), 'orchestrator-repair-')) })
afterEach(() => { rmSync(directory, { recursive: true, force: true }) })

const thread = (id: string, fields: Partial<Thread> = {}): Thread => ({
  key: `feishu:${id}`, ref: { source: 'feishu', id }, createdAt: '2026-01-01T00:00:00.000Z',
  subscriptions: [], correlated: [], pending: [], meta: {}, ...fields,
})
const trigger = (id: string, threadId: string, at: string): Trigger => ({
  id, source: 'feishu', threads: [{ source: 'feishu', id: threadId }], actor: { source: 'feishu', id: 'ou_alice' },
  human: true, at, text: 'opened the topic', meta: { messageId: id, chatId: 'oc_team', threadId },
})
const ticket = (id: string, fields: Partial<Ticket> = {}): Ticket => ({
  id, title: id, decisionId: '', mandate: { strength: 'explicit', policyVersion: '', basis: { via: 'listed' }, actor: { source: 'x', id: '' }, scope: '' },
  channels: [], assignments: [], inputs: [], undelivered: [], createdAt: '', updatedAt: '', origin: { triggerId: '', triggerIds: [] }, ...fields,
})
const decision = (id: string, threadKey: string): Decision => ({
  id, evalId: `eval-${id}`, threadKey, triggerIds: ['om_root'], policyVersion: 'p', at: '2026-01-01T00:00:00.000Z',
  kind: 'accept', reason: [], strength: 'explicit',
})

it('re-keys a Topic Thread to the message that opened it, so its Ticket receives the Topic again', () => {
  const store = new Store(join(directory, 'orchestrator.sqlite'))
  const oldKey = 'feishu:oc_team:omt_1'
  store.threads.save(thread('oc_team:omt_1', { subscriptions: ['ticket_1'], pending: ['om_root'],
    lastAt: '2026-01-02T00:00:00.000Z', meta: { threadId: 'omt_1', label: 'Topic' } }))
  // The opening message is the oldest Trigger on the Thread; Feishu gives it a thread_id but no root_id.
  store.triggers.save(trigger('om_root', 'oc_team:omt_1', '2026-01-01T00:00:00.000Z'))
  store.triggers.save(trigger('om_later', 'oc_team:omt_1', '2026-01-02T00:00:00.000Z'))
  store.tickets.save(ticket('ticket_1', { follows: [{ thread: oldKey, own: true }] }))
  store.decisions.save(decision('d1', oldKey))
  store.set(KV.feishu.history(`thread:${oldKey}`), '2026-01-02T00:00:00.000Z')
  store.set(KV.feishu.topic(oldKey), { at: '2026-01-02T00:00:00.000Z', threadId: 'omt_1' })
  expect(rekeyFeishuTopicThreads(store)).toBe(1)
  const key = 'feishu:oc_team:om_root'
  expect(store.threads.get(oldKey)).toBeUndefined()
  expect(store.threads.get(key)).toMatchObject({ key, ref: { source: 'feishu', id: 'oc_team:om_root' },
    subscriptions: ['ticket_1'], pending: ['om_root'], meta: { threadId: 'omt_1', label: 'Topic' } })
  // The Ticket follows the merged Thread, and both Triggers are found on it.
  expect(store.tickets.get('ticket_1')!.follows).toEqual([{ thread: key, own: true }])
  expect(store.read.threadTriggers(key).map(item => item.id)).toEqual(['om_root', 'om_later'])
  expect(store.triggers.get('om_later')!.threads).toEqual([{ source: 'feishu', id: 'oc_team:om_root' }])
  // Its Decisions and read records moved with it.
  expect(store.decisions.byThread(key).map(item => item.id)).toEqual(['d1'])
  expect(store.get(KV.feishu.history(`thread:${key}`))).toBe('2026-01-02T00:00:00.000Z')
  expect(store.get(KV.feishu.topic(oldKey))).toBeUndefined()
  expect(store.get(KV.feishu.topic(key))).toMatchObject({ threadId: 'omt_1' })
  // A second run has nothing left to do.
  expect(rekeyFeishuTopicThreads(store)).toBe(0)
  store.close()
})

it('merges into a Thread already recorded for the opening message and keeps one follow', () => {
  const store = new Store(join(directory, 'orchestrator.sqlite'))
  const oldKey = 'feishu:oc_team:omt_1'
  store.threads.save(thread('oc_team:omt_1', { subscriptions: ['ticket_1'], pending: ['om_root'], meta: { threadId: 'omt_1' } }))
  store.threads.save(thread('oc_team:om_root', { subscriptions: ['ticket_2'], pending: ['om_other'],
    meta: { threadId: 'omt_1', label: 'Kept' }, lastAt: '2026-01-03T00:00:00.000Z' }))
  store.triggers.save(trigger('om_root', 'oc_team:omt_1', '2026-01-01T00:00:00.000Z'))
  store.tickets.save(ticket('ticket_1', { follows: [{ thread: oldKey, own: true }] }))
  store.tickets.save(ticket('ticket_2', { follows: [{ thread: 'feishu:oc_team:om_root', own: false }] }))
  store.tickets.save(ticket('ticket_3', { follows: [{ thread: oldKey, own: false }, { thread: 'feishu:oc_team:om_root', own: true }] }))
  expect(rekeyFeishuTopicThreads(store)).toBe(1)
  const merged = store.threads.get('feishu:oc_team:om_root')!
  expect(merged.subscriptions).toEqual(['ticket_2', 'ticket_1'])
  expect(merged.pending).toEqual(['om_other', 'om_root'])
  // The record already on the new key keeps its own meta and activity time.
  expect(merged.meta.label).toBe('Kept')
  expect(merged.lastAt).toBe('2026-01-03T00:00:00.000Z')
  expect(store.tickets.get('ticket_3')!.follows).toEqual([{ thread: 'feishu:oc_team:om_root', own: true }])
  store.close()
})

it('leaves a Topic Thread alone when no opening message is known', () => {
  const store = new Store(join(directory, 'orchestrator.sqlite'))
  store.threads.save(thread('oc_team:omt_9', { subscriptions: ['ticket_1'], meta: { threadId: 'omt_9' } }))
  expect(rekeyFeishuTopicThreads(store)).toBe(0)
  expect(store.threads.get('feishu:oc_team:omt_9')).toBeDefined()
  store.close()
})

it('leaves a Thread it cannot re-key for the next start instead of failing the start', () => {
  const store = new Store(join(directory, 'orchestrator.sqlite'))
  const oldKey = 'feishu:oc_team:omt_7'
  store.threads.save(thread('oc_team:omt_7', { subscriptions: ['ticket_1'], meta: { threadId: 'omt_7' } }))
  store.triggers.save(trigger('om_seven', 'oc_team:omt_7', '2026-01-01T00:00:00.000Z'))
  const failure = vi.spyOn(store.tickets, 'list').mockImplementation(() => { throw new Error('disk busy') })
  const reported: string[] = []
  expect(rekeyFeishuTopicThreads(store, (error, failed) => reported.push(`${failed.key}: ${String(error)}`))).toBe(0)
  expect(reported).toEqual([`${oldKey}: Error: disk busy`])
  // Nothing was rewritten, so the next start tries again.
  expect(store.threads.get(oldKey)).toBeDefined()
  expect(store.threads.get('feishu:oc_team:om_seven')).toBeUndefined()
  failure.mockRestore()
  store.close()
})

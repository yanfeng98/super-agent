import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, expect, it } from 'vitest'
import { Store } from '../src/app/store.ts'
import { notifications, readAllNotifications, readNotification, recordNotification } from '../src/app/notifications.ts'

let directory: string
let store: Store
beforeEach(() => {
  directory = mkdtempSync(join(tmpdir(), 'notifications-'))
  store = new Store(join(directory, 'orchestrator.sqlite'))
})
afterEach(() => { store.close(); rmSync(directory, { recursive: true, force: true }) })

it('reads an ending recorded as text into its fields, and leaves other records as written', () => {
  recordNotification(store, { at: '2026-11-18T01:00:00Z', kind: 'ended', ticketId: 't1', title: '修复回归', text: 'completed' })
  recordNotification(store, { at: '2026-11-18T02:00:00Z', kind: 'ended', ticketId: 't2', title: '另一个', text: 'stopped: 需求取消' })
  recordNotification(store, { at: '2026-11-18T03:00:00Z', kind: 'merged', ticketId: 't2', title: '另一个',
    text: 'Merged T-0002 into main at aaaa1111' })
  recordNotification(store, { at: '2026-11-18T04:00:00Z', kind: 'ended', ticketId: 't3', title: '第三个', text: 'completed',
    ending: { outcome: 'completed', reason: '工作已完成' } })
  const items = notifications(store).items
  expect(items.find(item => item.ticketId === 't1')!.ending).toEqual({ outcome: 'completed' })
  expect(items.find(item => item.ticketId === 't2' && item.kind === 'ended')!.ending).toEqual({ outcome: 'stopped',
    reason: '需求取消' })
  expect(items.find(item => item.kind === 'merged')!.merge).toEqual({ target: 'main', head: 'aaaa1111' })
  expect(items.find(item => item.kind === 'ended' && item.ticketId === 't1')!.merge).toBeUndefined()
  // A record that already carries fields keeps them as recorded.
  expect(items.find(item => item.ticketId === 't3')!.ending).toEqual({ outcome: 'completed', reason: '工作已完成' })
})

it('keeps notifications newest first, counts unread ones, and marks one or all read', () => {
  recordNotification(store, { at: '2026-11-18T01:00:00Z', kind: 'attention', ticketId: 't1', title: '修复回归', text: '用哪种颜色？' })
  recordNotification(store, { at: '2026-11-18T02:00:00Z', kind: 'ended', ticketId: 't1', title: '修复回归', text: 'completed' })
  recordNotification(store, { at: '2026-11-18T03:00:00Z', kind: 'ended', ticketId: 't2', title: '另一个', text: 'stopped: person' })
  expect(notifications(store)).toMatchObject({ unread: 3, items: [{ ticketId: 't2' }, { at: '2026-11-18T02:00:00Z' },
    { at: '2026-11-18T01:00:00Z' }] })
  const first = notifications(store).items[0]!
  expect(readNotification(store, first.id)).toBe(true)
  expect(readNotification(store, 'missing')).toBe(false)
  expect(notifications(store).unread).toBe(2)
  expect(readAllNotifications(store)).toBe(2)
  expect(notifications(store).unread).toBe(0)
  expect(readAllNotifications(store)).toBe(0)
})

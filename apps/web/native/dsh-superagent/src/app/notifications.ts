/** Persistent panel notifications for work that needs a person: a question, an ending, or a merge event. */
import { key } from '../core/index.ts'
import { family } from '../kv.ts'
import type { Store } from './store.ts'

/** One notification, kept after the event so a person can catch up when the panel was closed. */
export interface Notification {
  id: string
  at: string
  /** A question (`attention`), an ending (`ended`), or a merge outcome a person may need to see. */
  kind: 'attention' | 'ended' | 'merged' | 'reverted' | 'review' | 'ci' | 'automerge' | 'check' | 'push'
  ticketId: string
  title: string
  /** The question, the ending outcome and reason, or the merge event in one sentence. */
  text: string
  read?: true
}
/** Notification records by id; the store owns the prefix, this module owns the value. */
const NOTIFICATION = family<Notification, [id: string]>('notification:')

/**
 * Records a notification for a Ticket event, replacing one with the same Ticket, kind and time.
 * @param store - The store that keeps the panel's records.
 * @param input - The event to record, without its derived id.
 */
export function recordNotification(store: Store, input: Omit<Notification, 'id'>): void {
  const id = key('notification', input.ticketId, input.kind, input.at)
  store.set(NOTIFICATION(id), { ...input, id })
}

/**
 * Lists the newest notifications and how many are unread.
 * @param store - The store that keeps the panel's records.
 * @param limit - The maximum number of notifications to return.
 * @returns The newest notifications, newest first, and the unread count over every stored notification.
 */
export function notifications(store: Store, limit = 50): { items: Notification[]; unread: number } {
  const all = store.keys(NOTIFICATION).flatMap(name => {
    const value = store.get(name)
    return value ? [value] : []
  })
  all.sort((a, b) => b.at.localeCompare(a.at) || b.id.localeCompare(a.id))
  return { items: all.slice(0, limit), unread: all.filter(item => !item.read).length }
}

/**
 * Marks one notification read.
 * @returns Whether it existed.
 */
export function readNotification(store: Store, id: string): boolean {
  const name = NOTIFICATION(id)
  const value = store.get(name)
  if (!value) return false
  if (!value.read) store.set(name, { ...value, read: true })
  return true
}

/**
 * Marks every notification read.
 * @returns How many were unread.
 */
export function readAllNotifications(store: Store): number {
  let changed = 0
  for (const name of store.keys(NOTIFICATION)) {
    const value = store.get(name)
    if (!value || value.read) continue
    store.set(name, { ...value, read: true })
    changed++
  }
  return changed
}

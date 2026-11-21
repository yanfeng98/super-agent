/** Desktop notifications for tickets that start waiting on a person or end, while the Web app is open. */
import type { TicketRow } from './api.ts'
import { en, zh } from './locales.ts'

const KEY = 'dsh-superagent/notifications'
const POLL_MS = 30_000
type Watched = Pick<TicketRow, 'id' | 'title' | 'state' | 'stage'> & { action?: string }
type Dictionary = typeof zh
const dictionary = (): Dictionary => typeof navigator !== 'undefined' && navigator.language.toLowerCase().startsWith('zh') ? zh : en
/** What a ticket is doing now, so a repeated poll of the same state notifies once. */
const marker = (ticket: Watched): string => ticket.state === 'ended' ? 'ended' : `${ticket.stage}:${ticket.action ?? ''}`
/** Whether the browser can raise a desktop notification at all. */
export const notificationsSupported = (): boolean => typeof Notification !== 'undefined'
/** Whether the operator turned desktop notifications on in this browser. */
export const notificationsEnabled = (): boolean => {
  try { return notificationsSupported() && localStorage.getItem(KEY) === '1' }
  catch (error) { /* A browser that blocks storage keeps notifications off rather than breaking the panel. */ return false }
}
/**
 * Decides whether a ticket's move deserves a notification: it started waiting on a person, or it ended.
 * @param previous - The marker last seen for the ticket; undefined on first sight, which reports nothing.
 * @param ticket - Its current row.
 * @returns The localized title and body, or undefined when nothing changed or the ticket still waits the same way.
 */
export function notificationFor(previous: string | undefined, ticket: Watched): { title: string; body: string } | undefined {
  const current = marker(ticket)
  if (previous === undefined || previous === current) return undefined
  const dict = dictionary()
  if (current === 'ended') return { title: dict.notifyEnded, body: ticket.title }
  if (ticket.stage !== 'person') return undefined
  const stage = dict[`stage.${ticket.action ?? 'review'}` as keyof Dictionary]
  return { title: dict.notifyAttention, body: stage ? `${ticket.title} · ${stage}` : ticket.title }
}
/** The kinds the inbox records that the ticket poller cannot see, because the Ticket itself does not move. */
const MERGE_EVENTS = new Set(['merged', 'reverted', 'ci', 'automerge', 'check', 'push'])
/** One panel notification as the desktop watcher reads it. */
export interface InboxItem { id: string; kind: string; title: string; text: string }
/**
 * Decides whether an inbox notification deserves a desktop announcement: a merge outcome, a failing check, or a
 * standing merge the host gave up on. A question or an ending is announced from the ticket's own state instead.
 * @param item - The notification the panel recorded.
 * @returns The localized title and body, or undefined for a kind the ticket poller already covers.
 */
export function eventNotificationFor(item: InboxItem): { title: string; body: string } | undefined {
  if (!MERGE_EVENTS.has(item.kind)) return undefined
  const dict = dictionary()
  const title = dict[`notification.${item.kind}` as keyof Dictionary]
  if (typeof title !== 'string') return undefined
  return { title, body: `${item.title} · ${item.text}` }
}
let timer: ReturnType<typeof setInterval> | undefined
let polling = false
const seen = new Map<string, string>()
const announced = new Set<string>()
async function load(state: 'open' | 'ended'): Promise<Watched[]> {
  const response = await fetch(`/orchestrator/api/tickets?state=${state}&limit=50`, { headers: { 'x-orchestrator-request': '1' } })
  if (!response.ok) return []
  const body = await response.json() as { items?: Watched[] }
  return body.items ?? []
}
async function loadInbox(): Promise<InboxItem[]> {
  const response = await fetch('/orchestrator/api/notifications?limit=20', { headers: { 'x-orchestrator-request': '1' } })
  if (!response.ok) return []
  const body = await response.json() as { items?: InboxItem[] }
  return body.items ?? []
}
async function poll(first: boolean): Promise<void> {
  if (polling) return
  polling = true
  try {
    const tickets = [...await load('open'), ...await load('ended')]
    for (const ticket of tickets) {
      const previous = seen.get(ticket.id)
      seen.set(ticket.id, marker(ticket))
      // The first poll records what already needs a person without replaying it as a new event.
      if (first) continue
      const notice = notificationFor(previous, ticket)
      if (!notice) continue
      try { new Notification(notice.title, { body: notice.body, tag: `dsh-superagent:${ticket.id}` }) }
      catch (error) { /* The browser can refuse while hidden or after permission changed; the next poll tries again. */ }
    }
    // The inbox carries what the ticket poller cannot see: a merge, a revert, a failed check, a push.
    for (const item of await loadInbox()) {
      if (announced.has(item.id)) continue
      announced.add(item.id)
      if (first) continue
      const notice = eventNotificationFor(item)
      if (!notice) continue
      try { new Notification(notice.title, { body: notice.body, tag: `dsh-superagent:${item.id}` }) }
      catch (error) { /* The browser can refuse while hidden or after permission changed; the next poll tries again. */ }
    }
  } catch (error) { /* An unreachable host is reported by the panel; polling resumes on the next tick. */ }
  finally { polling = false }
}
/** Starts polling for tickets that need a person; repeated calls keep one timer. */
export function startNotifications(): void {
  if (!notificationsEnabled() || timer !== undefined) return
  void poll(true)
  timer = setInterval(() => { void poll(false) }, POLL_MS)
}
/** Stops the watcher, such as when the plugin unloads. */
export function stopNotifications(): void {
  if (timer !== undefined) clearInterval(timer)
  timer = undefined
  seen.clear()
  announced.clear()
}
/**
 * Asks the browser for permission and turns the watcher on.
 * @returns whether notifications are now on.
 */
export async function enableNotifications(): Promise<boolean> {
  if (!notificationsSupported()) return false
  const permission = Notification.permission === 'granted' ? 'granted' : await Notification.requestPermission()
  if (permission !== 'granted') return false
  try { localStorage.setItem(KEY, '1') }
  catch (error) { /* The watcher can run for this page even when the setting cannot be stored. */ }
  startNotifications()
  return true
}

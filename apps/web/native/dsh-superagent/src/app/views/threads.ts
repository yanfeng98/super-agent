/** Thread list, timeline detail, cross-entity search, and decision list views. */
import { latestReport, threadKey, type Thread, type Ticket, type Trigger } from '../../core/index.ts'
import type { ThreadFilter } from '../api.ts'
import type { TextCache } from '../cache.ts'
import { awaitingEval, channelLink, dayAgo, decisionView, newestFirst, page, snapshot, threadSummary, threadTitle, ticketLine,
  ticketSummary, triggerView, type ViewHost } from './records.ts'
import { attribute } from './text.ts'

export function threadList(host: ViewHost, filter: ThreadFilter, offset: number, limit: number) {
  const { store } = host
  const view = snapshot(host)
  const since = dayAgo()
  const open = new Set(store.tickets.open().map(ticket => ticket.id))
  const live = (thread: Thread) => thread.subscriptions.some(id => open.has(id))
  const lastAt = (thread: Thread) => thread.lastAt ?? thread.createdAt
  const kept = (thread: Thread) => filter === 'recent' ? lastAt(thread) >= since
    : filter === 'all' || (filter === 'subscribed' ? live(thread) : !!thread.evaluating || awaitingEval(view, thread).length > 0)
  const candidates = filter === 'subscribed' ? store.read.subscribedThreads()
    : filter === 'evaluating' ? store.threads.active() : store.threads.list()
  const threads = candidates.filter(kept).sort((a, b) => lastAt(b).localeCompare(lastAt(a)))
  const result = page(threads, offset, limit)
  return { ...result, items: result.items.map(thread => threadSummary(view, thread)) }
}
export type ThreadListView = ReturnType<typeof threadList>
/**
 * Thread timeline with triggers ordered oldest-first, inline decisions, and subscribed tickets.
 * `before` pages backward; `around` anchors the page on a specific trigger.
 */
export function threadDetail(host: ViewHost, key: string, before: string | undefined, limit: number, around?: string) {
  const { store } = host
  const thread = store.threads.get(key)
  if (!thread) return undefined
  const view = snapshot(host)
  const target = around ? store.triggers.get(around) : undefined
  // Cursors pair timestamp and id so triggers sharing the same second page deterministically.
  const [at = '', id = ''] = before?.split('|') ?? []
  const triggers = target ? store.read.threadTriggers(key, limit + 1, target, true)
    : store.read.threadTriggers(key, limit + 1, before ? { at, id } : undefined)
  const decisions = store.decisions.byThread(key)
  const more = triggers.length > limit
  const shown = more ? triggers.slice(1) : triggers
  const first = store.read.firstTrigger(key)
  const url = attribute(thread.meta.url) ?? first?.url ?? (first?.channel ? channelLink(view, first.channel).url : undefined)
  // `context` lists tickets whose Executor follows this thread for context only (messages reach them without granting authority).
  return { ...threadSummary(view, thread), url, context: thread.context ?? [],
    correlated: thread.correlated.map(other => {
      const value = store.threads.get(other)
      return { key: other, title: value ? threadTitle(store, value) : other }
    }),
    tickets: thread.subscriptions.flatMap(id => { const ticket = store.tickets.get(id); return ticket ? [ticketLine(view, ticket)] : [] }),
    decisions: decisions.slice().reverse().map(decision => decisionView(view, decision)),
    timeline: shown.map(trigger => triggerView(view, trigger, decisions)), earlier: more ? `${shown[0]!.at}|${shown[0]!.id}` : null }
}
export type ThreadDetailView = NonNullable<ReturnType<typeof threadDetail>>
/** A comparison against a ticket timestamp: `>=2026-01-01`, `<2026-02-01`, or a plain date for that day. */
export interface SearchDate { op: '>=' | '<=' | '>' | '<' | '='; value: string; dateOnly: boolean }
/** Search qualifiers parsed from a query: `is:open`, `tag:p1` (or `label:p1`), `author:alice`, `created:>=2026-01-01`. */
export interface SearchQuery { text: string; is?: 'open' | 'ended'; tag?: string; author?: string; created?: SearchDate; updated?: SearchDate }
/**
 * Splits a search query into the free text and Source qualifiers.
 * @param query - The text a person typed.
 * @returns The remaining words and the qualifiers they narrowed; an unknown `is:` value or a malformed date stays as text.
 */
export function parseSearchQuery(query: string): SearchQuery {
  const values: Partial<SearchQuery> = {}
  const text = query.replace(/(?:^|\s)(is|tag|label|author|created|updated):("[^"]*"|\S+)/gi, (_match, name: string, raw: string) => {
    const value = raw.replace(/^"|"$/g, '')
    const qualifier = name.toLowerCase()
    if (qualifier === 'is') {
      const state = value.toLowerCase()
      if (state === 'open' || state === 'ended') values.is = state
      else return _match
    } else if (qualifier === 'tag' || qualifier === 'label') values.tag = value
    else if (qualifier === 'author') values.author = value
    else {
      const date = parseSearchDate(value)
      if (!date) return _match
      if (qualifier === 'created' || qualifier === 'updated') values[qualifier] = date
    }
    return ''
  }).trim()
  return { text, ...values }
}
/**
 * Reads a ticket timestamp comparison, accepting a date or a full timestamp after an optional operator.
 * @param value - The qualifier's value, such as `>=2026-01-01` or `2026-02-03T04:05:06Z`.
 * @returns The comparison, or undefined when the value is not a date or timestamp.
 */
export function parseSearchDate(value: string): SearchDate | undefined {
  const match = /^(>=|<=|>|<|=)?(\d{4}-\d{2}-\d{2})(?:T(\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?))?$/.exec(value)
  if (!match) return undefined
  const day = match[2]!
  const timestamp = match[3] ? `${day}T${match[3]}` : day
  if (Number.isNaN(Date.parse(timestamp))) return undefined
  return { op: (match[1] as SearchDate['op'] | undefined) ?? '=', value: match[3] ? new Date(timestamp).toISOString() : day,
    dateOnly: !match[3] }
}
/** Whether a ticket timestamp satisfies a parsed comparison; a date compares the day, a timestamp the instant. */
const within = (at: string, bound: SearchDate | undefined): boolean => {
  if (!bound) return true
  const left = bound.dateOnly ? at.slice(0, 10) : at
  if (bound.op === '>=') return left >= bound.value
  if (bound.op === '<=') return left <= bound.value
  if (bound.op === '>') return left > bound.value
  if (bound.op === '<') return left < bound.value
  return left === bound.value
}
/**
 * Searches triggers, threads, and tickets by text, ID, source-extracted references (e.g., `#N`, URLs), or cached Feishu AppLinks.
 * Qualifiers narrow Tickets only: `is:open` or `is:ended`, `tag:<name>`, and `author:<id or name>`.
 * @param links - Feishu message AppLinks cached since startup, keyed by chat and message ID.
 * @param offset - How many matching Tickets to skip, so the panel can append the next page.
 */
export function search(host: ViewHost, query: string, links: Pick<TextCache, 'entries'>, offset = 0) {
  const { store, sources } = host
  const parts = parseSearchQuery(query)
  const value = parts.text
  const narrowed = !!(parts.is || parts.tag || parts.author || parts.created || parts.updated)
  if (!value && !narrowed) return { tickets: [], items: [], threads: [], more: false, next_offset: null }
  const view = snapshot(host)
  const named = value ? sources.find(value) : { terms: [], threads: [] }
  const terms = new Set([value, ...named.terms])
  if (value) for (const [name, url] of links.entries()) if (url === value) terms.add(name.split(':').at(-1)!)
  const found = new Map<string, Trigger>()
  if (value) for (const term of terms) for (const trigger of store.read.search(term)) found.set(trigger.id, trigger)
  const threads = named.threads.flatMap(key => { const thread = store.threads.get(key); return thread ? [threadSummary(view, thread)]
    : [] })
  const lower = value.toLowerCase()
  // Match tickets by threads they own or subscribe to, by title, by their latest result, and by a reply
  // whose text the archive also holds, so searching a phrase an agent wrote finds the ticket.
  const reaches = (ticket: Ticket) => [...(ticket.follows ?? []).map(follow => follow.thread),
    ...view.threadsOf(ticket.id).map(thread => thread.key)]
  const replied = new Set(value ? store.receipts.ticketsMatching(value) : [])
  const tags = (ticket: Ticket): string[] => {
    if (ticket.tags) return ticket.tags
    const value = store.triggers.get(ticket.origin.triggerId)?.meta.tags
    return Array.isArray(value) ? value.filter((tag): tag is string => typeof tag === 'string') : []
  }
  const actor = (ticket: Ticket) => {
    const person = view.actor(ticket.mandate.actor, ticket.mandate.basis.via !== 'bot')
    return [ticket.mandate.actor.id, ...person.name ? [person.name.zh, person.name.en] : []].map(name => name.toLowerCase())
  }
  const state = (ticket: Ticket) => !parts.is || (parts.is === 'open' ? !ticket.ended : !!ticket.ended)
  const tagged = (ticket: Ticket) => !parts.tag || tags(ticket).some(tag => tag.toLowerCase() === parts.tag!.toLowerCase())
  const authored = (ticket: Ticket) => !parts.author || actor(ticket).includes(parts.author.toLowerCase())
  const dated = (ticket: Ticket) => within(ticket.createdAt, parts.created) && within(ticket.updatedAt, parts.updated)
  // The request text is what a person wrote, so a phrase in it finds the Ticket even when its title differs.
  const request = (ticket: Ticket) => (ticket.request?.text
    ?? ticket.origin.triggerIds.map(id => store.triggers.get(id)?.text ?? '').join('\n')).toLowerCase()
  const text = (ticket: Ticket) => !value || reaches(ticket).some(key => named.threads.includes(key))
    || ticket.title.toLowerCase().includes(lower) || request(ticket).includes(lower)
    || (latestReport(ticket)?.summary ?? '').toLowerCase().includes(lower)
    || replied.has(ticket.id) || tags(ticket).some(tag => tag.toLowerCase().includes(lower))
  const matched = store.tickets.list().filter(ticket => state(ticket) && tagged(ticket) && authored(ticket) && dated(ticket) && text(ticket))
    .sort(newestFirst)
  const tickets = matched.slice(offset, offset + 50).map(ticket => ticketSummary(ticket, view.number(ticket.id)))
  // Omit automated triggers from threads already included in `threads`.
  const shownThreads = new Set(threads.map(thread => thread.key))
  const item = (trigger: Trigger) => {
    const home = store.threads.get(threadKey(trigger.threads[0]!))
    const { text: _text, ...row } = triggerView(view, trigger)
    // Triggers filtered by their Source have no Thread record; surface the filter reason instead.
    const filtered = home ? {} : { filtered: store.triggers.disposition(trigger.id)?.filtered ?? '' }
    return { ...row, threadTitle: home ? threadTitle(store, home) : undefined, ...filtered }
  }
  const items = [...found.values()].filter(trigger => trigger.human || !shownThreads.has(threadKey(trigger.threads[0]!)))
  const shown = items.sort((a, b) => b.at.localeCompare(a.at))
  const nextOffset = matched.length > offset + 50 ? offset + 50 : null
  return { tickets, items: shown.slice(0, 30).map(item), threads, more: nextOffset !== null || shown.length > 30, next_offset: nextOffset }
}
export type SearchView = ReturnType<typeof search>
export function decisionList(host: ViewHost, kinds: string[], offset: number, limit: number, since = '') {
  const { items, more } = host.store.read.decisions({ kinds, since }, offset, limit)
  const view = snapshot(host)
  return { items: items.map(decision => decisionView(view, decision)), next_offset: more ? offset + limit : null }
}
export type DecisionListView = ReturnType<typeof decisionList>

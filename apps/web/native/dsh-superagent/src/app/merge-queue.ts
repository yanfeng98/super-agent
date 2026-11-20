/** The host's merge queue: the Tickets a person asked to merge one after another, in the order they asked. */
import type { MergeStrategy } from '../core/index.ts'
import { KV } from '../kv.ts'
import type { Store } from './store.ts'

/** Why a queued merge did not land: a gate another Ticket or the person opens, or one that needs new work. */
export type MergeFailureCode = 'dependency' | 'review' | 'notes' | 'other'

/** One Ticket waiting for its turn; the options travel with it so a restart merges it the way the person asked. */
export interface QueuedMerge {
  ticketId: string
  at: string
  strategy?: MergeStrategy
  cleanup?: boolean
  push?: boolean
  /**
   * Why its last attempt did not merge; an entry with an error waits for a retry instead of running again. `code` names the
   * gate, so a drain can retry an entry whose dependency merged or whose review was approved on its own.
   */
  error?: { at: string; message: string; code?: MergeFailureCode }
}

/** The queue as the store holds it, oldest first. */
export function queuedMerges(store: Store): QueuedMerge[] { return store.get(KV.runtime.mergeQueue) ?? [] }

/**
 * Appends Tickets to the queue in the order given; one already queued keeps its place and takes the new options, which also
 * clears a failure so the person asking again retries it.
 * @returns The queue after the addition.
 */
export function enqueueMerges(store: Store, entries: readonly QueuedMerge[]): QueuedMerge[] {
  return store.atomic(() => {
    const queue = queuedMerges(store)
    for (const entry of entries) {
      const at = queue.findIndex(item => item.ticketId === entry.ticketId)
      // Replacing the entry rather than merging into it is what clears a recorded failure.
      if (at < 0) queue.push(entry)
      else queue[at] = entry
    }
    store.set(KV.runtime.mergeQueue, queue)
    return queue
  })
}

/** Records why an entry did not merge, keeping it in the queue for a retry or a removal. */
export function markMergeFailed(store: Store, ticketId: string, message: string, code: MergeFailureCode = 'other'): void {
  store.atomic(() => {
    const queue = queuedMerges(store)
    const at = queue.findIndex(item => item.ticketId === ticketId)
    if (at < 0) return
    queue[at] = { ...queue[at]!, error: { at: new Date().toISOString(), message: message.slice(0, 1000), code } }
    store.set(KV.runtime.mergeQueue, queue)
  })
}

/**
 * The failed entries whose gate has opened since, such as a dependency that merged or a review that was approved, so a
 * drain tries them again without the person asking.
 * @returns The Tickets to retry, in queue order.
 */
export function recoverableMerges(store: Store): string[] {
  return queuedMerges(store).flatMap(entry => {
    const code = entry.error?.code
    if (!code || code === 'other') return []
    const ticket = store.tickets.get(entry.ticketId)
    if (!ticket?.ended || ticket.merged || ticket.discarded || ticket.cleaned) return []
    const open = code === 'dependency' ? (ticket.blockedBy ?? []).every(id => store.tickets.get(id)?.merged)
      : code === 'review' ? ticket.reviewed?.decision === 'approved'
      : !(ticket.notes ?? []).some(note => !note.resolved)
    return open ? [entry.ticketId] : []
  })
}

/**
 * Clears the recorded failure of Tickets in the queue, so the next pass tries them again.
 * @returns How many entries were waiting on a retry.
 */
export function retryMerges(store: Store, ticketIds: readonly string[]): number {
  return store.atomic(() => {
    const queue = queuedMerges(store)
    let retried = 0
    for (const entry of queue) {
      if (!entry.error || !ticketIds.includes(entry.ticketId)) continue
      delete entry.error
      retried++
    }
    store.set(KV.runtime.mergeQueue, queue)
    return retried
  })
}

/** The first entry whose last attempt did not fail, in queue order; one that failed waits for a retry. */
export function nextQueuedMerge(store: Store): QueuedMerge | undefined {
  return queuedMerges(store).find(entry => !entry.error)
}

/**
 * Removes Tickets from the queue; one that is not queued is left alone.
 * @returns How many entries were removed, and the queue afterwards.
 */
export function dequeueMerges(store: Store, ticketIds: readonly string[]): { removed: number; queue: QueuedMerge[] } {
  return store.atomic(() => {
    const before = queuedMerges(store)
    const queue = before.filter(item => !ticketIds.includes(item.ticketId))
    store.set(KV.runtime.mergeQueue, queue)
    return { removed: before.length - queue.length, queue }
  })
}

/** A Ticket's place in the queue, counted from one, or undefined when it does not wait there. */
export function mergePosition(store: Store, ticketId: string): number | undefined {
  const at = queuedMerges(store).findIndex(item => item.ticketId === ticketId)
  return at >= 0 ? at + 1 : undefined
}

/** Whether the host holds the queue, such as after a check failed on a merged target. */
export function mergeQueuePaused(store: Store): boolean { return store.get(KV.runtime.mergeQueuePaused) === true }

/**
 * Holds or releases the queue; a paused queue keeps its order and starts no new Ticket.
 * @returns The state afterwards.
 */
export function setMergeQueuePaused(store: Store, paused: boolean): boolean {
  store.set(KV.runtime.mergeQueuePaused, paused)
  return paused
}

/** The queue as the panel shows it: its state and the Tickets waiting, in order, with their titles. */
export interface MergeQueueView {
  paused: boolean
  items: { ticket_id: string; number?: number; title: string; position: number; strategy?: MergeStrategy;
    error?: { at: string; message: string; code?: MergeFailureCode } }[]
}
/**
 * Reads the queue for the panel, leaving out an entry whose Ticket no longer exists.
 * @param store - The store that keeps the queue and the Tickets.
 * @returns The paused state and the entries waiting, oldest first.
 */
export function mergeQueueView(store: Store): MergeQueueView {
  return { paused: mergeQueuePaused(store), items: queuedMerges(store).flatMap((entry, index) => {
    const ticket = store.tickets.get(entry.ticketId)
    if (!ticket) return []
    const number = store.tickets.number(entry.ticketId)
    return [{ ticket_id: entry.ticketId, title: ticket.title, position: index + 1,
      ...number ? { number } : {}, ...entry.strategy ? { strategy: entry.strategy } : {}, ...entry.error ? { error: entry.error } : {} }]
  }) }
}

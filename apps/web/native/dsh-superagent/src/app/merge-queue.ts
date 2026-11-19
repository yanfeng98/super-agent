/** The host's merge queue: the Tickets a person asked to merge one after another, in the order they asked. */
import type { MergeStrategy } from '../core/index.ts'
import { KV } from '../kv.ts'
import type { Store } from './store.ts'

/** One Ticket waiting for its turn; the options travel with it so a restart merges it the way the person asked. */
export interface QueuedMerge {
  ticketId: string
  at: string
  strategy?: MergeStrategy
  cleanup?: boolean
  push?: boolean
}

/** The queue as the store holds it, oldest first. */
export function queuedMerges(store: Store): QueuedMerge[] { return store.get(KV.runtime.mergeQueue) ?? [] }

/**
 * Appends Tickets to the queue in the order given, leaving one that is already queued where it is.
 * @returns The queue after the addition.
 */
export function enqueueMerges(store: Store, entries: readonly QueuedMerge[]): QueuedMerge[] {
  return store.atomic(() => {
    const queue = queuedMerges(store)
    for (const entry of entries) if (!queue.some(item => item.ticketId === entry.ticketId)) queue.push(entry)
    store.set(KV.runtime.mergeQueue, queue)
    return queue
  })
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

/**
 * Removes the first entry, which a drain took up, when it is still the one the drain named.
 * @returns The entry that was removed, or undefined when the queue changed meanwhile.
 */
export function shiftQueuedMerge(store: Store, ticketId: string): QueuedMerge | undefined {
  return store.atomic(() => {
    const [first, ...rest] = queuedMerges(store)
    if (first?.ticketId !== ticketId) return undefined
    store.set(KV.runtime.mergeQueue, rest)
    return first
  })
}

/** A Ticket's place in the queue, counted from one, or undefined when it does not wait there. */
export function mergePosition(store: Store, ticketId: string): number | undefined {
  const at = queuedMerges(store).findIndex(item => item.ticketId === ticketId)
  return at >= 0 ? at + 1 : undefined
}

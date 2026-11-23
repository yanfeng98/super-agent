/** Startup repairs for records an earlier version wrote in a shape this build no longer produces. */
import { threadKey, type Follow, type Thread, type ThreadRef, type Trigger } from '../core/index.ts'
import { KV } from '../kv.ts'
import type { Store } from './store.ts'

/** A Feishu Topic id. An older build keyed a Thread by one instead of by the message that opened the Topic. */
const TOPIC_ID = /^omt[_-][A-Za-z0-9_-]+$/

/** The chat and Topic a Thread was keyed by, when it was keyed by a Topic rather than by its opening message. */
interface TopicKey { chatId: string; topicId: string }

const topicOf = (thread: Thread): TopicKey | undefined => {
  if (thread.ref.source !== 'feishu') return undefined
  const at = thread.ref.id.indexOf(':')
  if (at < 1) return undefined
  const chatId = thread.ref.id.slice(0, at)
  const topicId = thread.ref.id.slice(at + 1)
  return TOPIC_ID.test(topicId) ? { chatId, topicId } : undefined
}

const text = (value: unknown): string | undefined => typeof value === 'string' && value ? value : undefined
const union = (left: readonly string[] = [], right: readonly string[] = []): string[] => [...new Set([...left, ...right])]
const later = (left?: string, right?: string): string | undefined =>
  left === undefined ? right : right === undefined ? left : left > right ? left : right
const earlier = (left: string, right?: string): string => right === undefined ? left : left < right ? left : right

/**
 * The message that opened a Topic: the Thread's own record, the bot's Topic map, or the oldest Trigger on it. A Topic
 * opens on a message Feishu gives a `thread_id` but no `root_id`, which is the message that opened it.
 */
function topicRoot(store: Store, thread: Thread, topic: TopicKey): string | undefined {
  const recorded = text(thread.meta.rootId)
  if (recorded) return recorded
  const mapped = store.get(KV.feishu.imTopics(topic.chatId))?.[topic.topicId]?.rootId
  if (text(mapped)) return mapped
  const row = store.db.prepare(`SELECT t.value FROM trigger_thread x JOIN trigger t ON t.id = x.id
    WHERE x.thread = ? ORDER BY x.at, x.rowid LIMIT 1`).get(thread.key) as { value?: string } | undefined
  if (!row?.value) return undefined
  return text((JSON.parse(row.value) as Trigger).meta?.messageId)
}

/** Follows the merged Thread, keeping `own` when either record named it and merging a recorded close. */
function mergeFollows(follows: readonly Follow[], from: string, to: string): Follow[] {
  const merged: Follow[] = []
  for (const follow of follows) {
    const moved = follow.thread === from ? { ...follow, thread: to } : follow
    const same = merged.find(item => item.thread === moved.thread)
    if (!same) { merged.push({ ...moved }); continue }
    if (moved.own) same.own = true
    if (!same.closed && moved.closed) same.closed = moved.closed
  }
  return merged
}

/** Moves the per-Thread read records that would otherwise be left behind on the old key. */
function moveThreadState(store: Store, from: string, to: string): void {
  const previous = store.get(KV.feishu.history(`thread:${from}`))
  const current = store.get(KV.feishu.history(`thread:${to}`))
  if (previous !== undefined && (current === undefined || previous < current)) store.set(KV.feishu.history(`thread:${to}`), previous)
  store.delete(KV.feishu.history(`thread:${from}`))
  const lookup = store.get(KV.feishu.topic(from))
  if (lookup !== undefined) {
    if (store.get(KV.feishu.topic(to)) === undefined) store.set(KV.feishu.topic(to), lookup)
    store.delete(KV.feishu.topic(from))
  }
}

/** Re-keys one Topic Thread to its opening message and merges anything already recorded there. */
function rekey(store: Store, old: Thread, target: ThreadRef): void {
  const key = threadKey(target)
  const existing = store.threads.get(key)
  // The record already on the new key comes first: it is the one the Topic has been using since the fix.
  const context = union(existing?.context, old.context)
  const lastAt = later(old.lastAt, existing?.lastAt)
  const expired = Math.max(old.expired ?? 0, existing?.expired ?? 0)
  const evaluating = existing?.evaluating ?? (old.evaluating ? { ...old.evaluating, threadKey: key } : undefined)
  store.threads.save({
    key, ref: target, createdAt: earlier(old.createdAt, existing?.createdAt),
    ...(lastAt === undefined ? {} : { lastAt }),
    subscriptions: union(existing?.subscriptions, old.subscriptions),
    ...(context.length ? { context } : {}),
    correlated: union(existing?.correlated, old.correlated).filter(item => item !== key && item !== old.key),
    pending: union(existing?.pending, old.pending),
    ...(evaluating ? { evaluating } : {}),
    ...(expired ? { expired } : {}),
    meta: { ...old.meta, ...existing?.meta },
  })
  for (const ticket of store.tickets.list()) {
    if (!ticket.follows?.some(follow => follow.thread === old.key)) continue
    store.tickets.save({ ...ticket, follows: mergeFollows(ticket.follows, old.key, key) })
  }
  // A Trigger indexed on the old Thread still names it, and a person's follow-up there has to reach the merged Thread.
  const indexed = store.db.prepare('SELECT id FROM trigger_thread WHERE thread = ?').all(old.key)
  for (const row of indexed) {
    const trigger = store.triggers.get(String(row.id))
    if (!trigger?.threads.some(ref => ref.source === target.source && ref.id === old.ref.id)) continue
    store.triggers.save({ ...trigger, threads: trigger.threads.map(ref =>
      ref.source === target.source && ref.id === old.ref.id ? target : ref) })
  }
  for (const decision of store.decisions.byThread(old.key)) store.decisions.save({ ...decision, threadKey: key })
  for (const other of store.threads.list()) {
    if (other.key === key || !other.correlated.includes(old.key)) continue
    store.threads.save({ ...other, correlated: [...new Set(other.correlated.map(item => item === old.key ? key : item))]
      .filter(item => item !== other.key) })
  }
  moveThreadState(store, old.key, key)
  store.db.prepare('DELETE FROM thread WHERE key = ?').run(old.key)
}

/**
 * Re-keys the Feishu Topic Threads an older build recorded under their Topic id to the message that opened the Topic,
 * merging them with a Thread already recorded there, so a Ticket following one receives the Topic's replies again.
 * The repair is idempotent: a re-keyed Thread no longer has a Topic id, and a Thread whose opening message is unknown is
 * left alone.
 * @param store - The opened store, after schema migrations.
 * @returns How many Threads it re-keyed.
 */
export function rekeyFeishuTopicThreads(store: Store): number {
  let rekeyed = 0
  for (const thread of store.threads.list()) {
    const topic = topicOf(thread)
    if (!topic) continue
    const rootId = topicRoot(store, thread, topic)
    if (!rootId) continue
    const target = { source: 'feishu', id: `${topic.chatId}:${rootId}` }
    if (threadKey(target) === thread.key) continue
    store.atomic(() => rekey(store, thread, target))
    rekeyed += 1
  }
  return rekeyed
}

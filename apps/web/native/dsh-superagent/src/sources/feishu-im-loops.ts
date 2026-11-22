/** dsh-im Feishu background loops: chat history backfill and subscribed-thread polling. */
import type { Thread, Trigger } from '../core/index.ts'
import { KV } from '../kv.ts'
import { SOURCE } from './feishu-trigger.ts'
import type { DshImEvent, DshImService } from './dsh-im.ts'
import type { FeishuConfig } from './feishu-im.ts'
import { POLL_HORIZON_MS, cursorAt, pause, pollThreads } from './poll.ts'
import { health, type Health, type SourceContext } from './host.ts'

const HISTORY_INTERVAL_MS = 60_000
/** dsh-im's per-page cap for chat and thread history; the backfill reads the largest page it allows. */
const HISTORY_PAGE = 50
/** 2 000 messages per chat per round, so a long outage catches up over a few rounds instead of hours of them. */
const MAX_PAGES = 40
/** Topics read per chat per round, most recent first: enough for a busy chat without re-reading every old topic each minute. */
const TOPIC_READS_PER_ROUND = 20
/** Topic failures kept in one round's health detail; the rest are counted. */
const REPORTED_TOPIC_ERRORS = 3
const READ_TIMEOUT_MS = 30_000

export interface BackfillDeps {
  ctx: SourceContext
  im: DshImService
  botId: string
  fingerprint: string
  config: () => FeishuConfig | undefined
  toTrigger: (event: DshImEvent) => Trigger
  /** Publishes the Thread's chat name and link for a recovered message, as the live path does. */
  describe?: (event: DshImEvent, trigger: Trigger) => void
  report: (entry: 'feishu' | 'history', health: Health) => void
  signal: AbortSignal
}

const signalOf = (signal: AbortSignal): AbortSignal => AbortSignal.any([signal, AbortSignal.timeout(READ_TIMEOUT_MS)])

/** Merges the chats the bot joined and dsh-im's group targets into the stored label map; both reads are cheap, so they run every round. */
async function refreshChatLabels(deps: BackfillDeps): Promise<void> {
  const labels: Record<string, string> = { ...(deps.ctx.state.get(KV.sources.labels(SOURCE)) ?? {}) }
  let changed = false
  if (deps.im.chatListChecked) {
    try {
      const listed = await deps.im.chatListChecked(deps.botId,
        { expectedFingerprint: deps.fingerprint, signal: signalOf(deps.signal) })
      for (const chat of listed.chats) {
        if (!chat.name || labels[chat.chatId] === chat.name) continue
        labels[chat.chatId] = chat.name
        changed = true
      }
    } catch { /* Labels are a display aid; the stored map stays until a later read succeeds. */ }
  }
  if (deps.im.listTargets) {
    try {
      for (const target of await deps.im.listTargets(deps.botId)) {
        const chatId = target.route?.chatId
        if (target.kind !== 'group' || !chatId || !target.name || labels[chatId] === target.name) continue
        labels[chatId] = target.name
        changed = true
      }
    } catch { /* Labels are a display aid; the stored map stays until a later read succeeds. */ }
  }
  if (changed) deps.ctx.state.set(KV.sources.labels(SOURCE), labels)
}

/** What one chat or Thread round delivered and what dsh-im could not render. */
interface Recovered { recovered: number; omitted: number }

/** One chat's messages since the stored cursor, page by page; the cursor only advances after every page is received. */
async function backfillChat(deps: BackfillDeps, chatId: string, direct: boolean, limit: number,
  errors: string[]): Promise<Recovered> {
  const read = deps.im.chatHistoryChecked
  if (!read) return { recovered: 0, omitted: 0 }
  const key = KV.feishu.history(chatId)
  const now = new Date().toISOString()
  const stored = deps.ctx.state.get(key)
  // The first round only records where the Source took over: messages before the handoff were dsh-im's own conversation.
  if (stored === undefined) {
    deps.ctx.state.set(key, cursorAt(now))
    return { recovered: 0, omitted: 0 }
  }
  const since = stored
  // A message carrying a topic is how a topic is discovered: its later replies live in the thread container, not the chat.
  const topicsKey = KV.feishu.imTopics(chatId)
  const topics: Record<string, { at: string; rootId: string }> = { ...deps.ctx.state.get(topicsKey) }
  let cursor: string | undefined
  let count = 0
  let omitted = 0
  let drained = false
  let lastAt: string | undefined
  for (let page = 0; page < MAX_PAGES; page++) {
    if (deps.signal.aborted) return { recovered: count, omitted }
    const result = await read(deps.botId,
      { chatId, limit, since, until: now, direct, includeBots: true, ...cursor ? { cursor } : {} },
      { expectedFingerprint: deps.fingerprint, signal: signalOf(deps.signal) })
    if (!deps.config()) return { recovered: count, omitted }
    omitted += result.omitted
    for (const event of result.events) {
      const trigger = deps.toTrigger(event)
      await deps.ctx.receive(trigger)
      deps.describe?.(event, trigger)
      count++
      lastAt = event.at
      const topicId = event.reply.threadId
      const known = topicId ? topics[topicId] : undefined
      // The first message seen for a topic roots it — its own reply's root when it has one; a later one only moves its
      // last activity.
      if (topicId && !known) topics[topicId] = { at: event.at, rootId: event.reply.rootId ?? event.messageId }
      else if (topicId && known && Date.parse(event.at) > Date.parse(known.at))
        topics[topicId] = { at: event.at, rootId: known.rootId }
    }
    cursor = result.nextCursor
    if (!cursor) { drained = true; break }
  }
  // Advance only as far as this round actually read: a page cap must not skip the rest of the window.
  if (!deps.signal.aborted) {
    if (drained) deps.ctx.state.set(key, cursorAt(now))
    else if (lastAt) deps.ctx.state.set(key, cursorAt(lastAt))
  }
  const replies = deps.signal.aborted ? { recovered: 0, omitted: 0 }
    : await recoverTopics(deps, chatId, direct, since, now, limit, topics, errors)
  if (!deps.signal.aborted) deps.ctx.state.set(topicsKey, topics)
  return { recovered: count + replies.recovered, omitted: omitted + replies.omitted }
}

/**
 * Reads the replies Feishu keeps out of chat history: the chat container returns the message a topic was opened on, while
 * its replies are only in the thread container. Each topic this chat had activity in is read within the round's window, so
 * a reply missed while the Host was down is recovered instead of waiting for the next live event. A topic a subscribed
 * Thread already polls is skipped, and one quiet for the Thread polling horizon is forgotten.
 */
async function recoverTopics(deps: BackfillDeps, chatId: string, direct: boolean, since: string, until: string, limit: number,
  topics: Record<string, { at: string; rootId: string }>, errors: string[]): Promise<Recovered> {
  const read = deps.im.threadHistoryChecked
  if (!read) return { recovered: 0, omitted: 0 }
  const followed = new Set(deps.ctx.threads.subscribed(SOURCE)
    .map(thread => String((thread.meta as { threadId?: string }).threadId ?? '')))
  for (const [topicId, topic] of Object.entries(topics)) {
    if (Date.parse(until) - Date.parse(topic.at) >= POLL_HORIZON_MS) delete topics[topicId]
  }
  const readable = Object.entries(topics).filter(([topicId]) => !followed.has(topicId))
    .sort(([, left], [, right]) => Date.parse(right.at) - Date.parse(left.at)).slice(0, TOPIC_READS_PER_ROUND)
  let recovered = 0
  let omitted = 0
  let failures = 0
  for (const [topicId, topic] of readable) {
    if (deps.signal.aborted) break
    let cursor: string | undefined
    let newest = topic.at
    try {
      for (let page = 0; page < MAX_PAGES; page++) {
        const result = await read(deps.botId,
          { chatId, threadId: topicId, limit, since, until, direct, includeBots: true, ...cursor ? { cursor } : {} },
          { expectedFingerprint: deps.fingerprint, signal: signalOf(deps.signal) })
        if (!deps.config()) return { recovered, omitted }
        omitted += result.omitted
        for (const event of result.events) {
          // Feishu may leave root_id off a topic's messages; the Thread opened here must be the one the live stream opens
          // for the same reply, and the message that carried the topic is its root.
          const rooted = event.reply.rootId ? event : { ...event, reply: { ...event.reply, rootId: topic.rootId } }
          const trigger = deps.toTrigger(rooted)
          await deps.ctx.receive(trigger)
          deps.describe?.(rooted, trigger)
          recovered++
          if (Date.parse(event.at) > Date.parse(newest)) newest = event.at
        }
        cursor = result.nextCursor
        if (!cursor) break
      }
    } catch (error) {
      // One unreadable topic must not stall the chat's cursor; it is tried again next round until it goes quiet.
      if (!deps.signal.aborted) {
        failures++
        if (failures <= REPORTED_TOPIC_ERRORS) errors.push(`topic ${topicId}: ${String(error)}`)
      }
    }
    topics[topicId] = { at: newest, rootId: topic.rootId }
  }
  if (failures > REPORTED_TOPIC_ERRORS) errors.push(`${failures - REPORTED_TOPIC_ERRORS} more topics failed`)
  return { recovered, omitted }
}

/**
 * Learns a Thread's topic id from its latest Trigger's root message when the Thread was opened before Feishu reported one.
 * A Thread without a topic stays on chat backfill, so the answer is remembered per Thread activity and read again only
 * after the Thread moved on.
 */
async function learnThreadId(deps: BackfillDeps, thread: Thread, chatId: string): Promise<string | undefined> {
  const read = deps.im.messageChecked
  const key = KV.feishu.topic(thread.key)
  const at = thread.lastAt ?? thread.createdAt
  const known = deps.ctx.state.get(key)
  if (known?.at === at) return known.threadId
  const address = deps.ctx.triggers.latest(thread.key)?.channel?.address
  if (!read || !address?.messageId || !address.actorId) return undefined
  try {
    const message = await read(deps.botId, {
      messageId: address.messageId, conversationId: address.conversationId ?? chatId, actorId: address.actorId,
      ...address.threadId ? { threadId: address.threadId } : {}, ...address.rootId ? { rootId: address.rootId } : {},
      ...address.parentId ? { parentId: address.parentId } : {},
    }, { expectedFingerprint: deps.fingerprint, signal: signalOf(deps.signal), messageId: address.rootId ?? address.messageId })
    const threadId = message.threadId
    deps.ctx.state.set(key, { at, ...threadId ? { threadId } : {} })
    if (threadId) deps.ctx.threads.patch(thread.key, { threadId })
    return threadId
  } catch (error) {
    const code = (error as { code?: string }).code
    // A source message that is gone stays gone; any other failure is tried again on the next round.
    if (code === 'stale-route' || code === 'message-unavailable') deps.ctx.state.set(key, { at })
    return undefined
  }
}

/** One subscribed thread's new messages since its stored cursor; thread ids come from the Thread, its latest Trigger, or its root message. */
async function pollThread(deps: BackfillDeps, thread: Thread): Promise<Recovered> {
  const read = deps.im.threadHistoryChecked
  if (!read) return { recovered: 0, omitted: 0 }
  const meta = thread.meta as { chatId?: string; threadId?: string }
  const latest = deps.ctx.triggers.latest(thread.key)
  const chatId = meta.chatId ?? (typeof latest?.meta.chatId === 'string' ? latest.meta.chatId : undefined)
  if (!chatId) return { recovered: 0, omitted: 0 }
  const threadId = meta.threadId ?? (typeof latest?.meta.threadId === 'string' ? latest.meta.threadId : undefined)
    ?? await learnThreadId(deps, thread, chatId)
  if (!threadId) return { recovered: 0, omitted: 0 }
  const key = KV.feishu.history(`thread:${thread.key}`)
  const now = new Date().toISOString()
  const since = deps.ctx.state.get(key) ?? new Date(Date.parse(thread.createdAt) - 5000).toISOString()
  const limit = deps.config()?.readPageSize ?? 20
  let cursor: string | undefined
  let count = 0
  let omitted = 0
  let drained = false
  let lastAt: string | undefined
  for (let page = 0; page < MAX_PAGES; page++) {
    if (deps.signal.aborted) return { recovered: count, omitted }
    const result = await read(deps.botId,
      { chatId, threadId, limit, since, until: now, includeBots: true, ...cursor ? { cursor } : {} },
      { expectedFingerprint: deps.fingerprint, signal: signalOf(deps.signal) })
    if (!deps.config()) return { recovered: count, omitted }
    omitted += result.omitted
    for (const event of result.events) {
      const trigger = deps.toTrigger(event)
      await deps.ctx.receive(trigger)
      deps.describe?.(event, trigger)
      count++
      lastAt = event.at
    }
    cursor = result.nextCursor
    if (!cursor) { drained = true; break }
  }
  if (!deps.signal.aborted) {
    if (drained) deps.ctx.state.set(key, cursorAt(now))
    else if (lastAt) deps.ctx.state.set(key, cursorAt(lastAt))
  }
  return { recovered: count, omitted }
}

/**
 * Recovers messages the live consumer missed — while superagent restarted, the Host was down, or the bot was handed off —
 * by reading each configured chat's history between stored cursors and polling subscribed threads. Re-received messages
 * deduplicate on their deterministic Trigger id, so overlap with the live stream is safe.
 */
export async function runFeishuImBackfill(deps: BackfillDeps): Promise<void> {
  await deps.ctx.ready
  if (!deps.im.chatHistoryChecked || !deps.im.threadHistoryChecked) {
    deps.report('history', health('paused', 'imBackfillMissing'))
    return
  }
  while (!deps.signal.aborted) {
    const config = deps.config()
    if (!config) {
      deps.report('history', health('paused', 'sourceOff'))
      await pause(HISTORY_INTERVAL_MS, deps.signal)
      continue
    }
    await refreshChatLabels(deps)
    const errors: string[] = []
    let recovered = 0
    let omitted = 0
    const chats = new Map<string, boolean>()
    for (const chat of config.chats) chats.set(chat.chatId, false)
    for (const [chatId, direct] of Object.entries(deps.ctx.state.get(KV.feishu.chats) ?? {})) {
      if (direct && config.directMessages) chats.set(chatId, true)
    }
    for (const [chatId, direct] of chats) {
      if (deps.signal.aborted) break
      try {
        const round = await backfillChat(deps, chatId, direct, HISTORY_PAGE, errors)
        recovered += round.recovered
        omitted += round.omitted
      } catch (error) { if (!deps.signal.aborted) errors.push(`${chatId}: ${String(error)}`) }
    }
    if (!deps.signal.aborted) {
      try {
        await pollThreads(deps.ctx, 'feishu', () => deps.signal.aborted, thread => pollThread(deps, thread).then(round => {
          recovered += round.recovered
          omitted += round.omitted
        }))
      } catch (error) { if (!deps.signal.aborted) errors.push(`threads: ${String(error)}`) }
    }
    if (!deps.signal.aborted) {
      deps.report('history', errors.length
        ? { state: 'error', detail: errors.join('\n') }
        : { state: 'ready', detail: `Recovered ${recovered} messages from chats, topics and subscribed threads`
          + (omitted ? `; ${omitted} unreadable messages skipped` : ''), at: new Date().toISOString() })
    }
    await pause(HISTORY_INTERVAL_MS, deps.signal)
  }
}

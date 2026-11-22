/** dsh-im Feishu background loops: chat history backfill and subscribed-thread polling. */
import type { Thread, Trigger } from '../core/index.ts'
import { KV } from '../kv.ts'
import { SOURCE } from './feishu-trigger.ts'
import type { DshImEvent, DshImService } from './dsh-im.ts'
import type { FeishuConfig } from './feishu-im.ts'
import { POLL_HORIZON_MS, cursorAt, pause, pollThreads, polled } from './poll.ts'
import { health, type Health, type SourceContext } from './host.ts'

const HISTORY_INTERVAL_MS = 60_000
/** dsh-im's per-page cap for chat and thread history; the backfill reads the largest page it allows. */
const HISTORY_PAGE = 50
/** 2 000 messages per chat per round, so a long outage catches up over a few rounds instead of hours of them. */
const MAX_PAGES = 40
/** Topics read per chat per round, least recently read first: enough for a busy chat without re-reading every old topic each minute. */
const TOPIC_READS_PER_ROUND = 20
/** Topic failures kept in one round's health detail; the rest are counted. */
const REPORTED_TOPIC_ERRORS = 3
/** Wall-clock budget of one round, so a slow remote makes the loop take longer instead of never catching up. */
const ROUND_MS = 5 * 60_000
const READ_TIMEOUT_MS = 30_000

export interface BackfillDeps {
  ctx: SourceContext
  im: DshImService
  botId: string
  fingerprint: string
  config: () => FeishuConfig | undefined
  /** Builds a Trigger; `root` names the message that opened the event's Topic when Feishu left `root_id` off it. */
  toTrigger: (event: DshImEvent, root?: string) => Trigger
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
/**
 * A Topic the backfill recovers: its last activity, the message that roots it, and how far it has been read. An entry an
 * earlier build wrote has no watermark, and starts again from the chat window's beginning.
 */
interface Topic { at: string; rootId: string; readThrough?: string }

/**
 * One chat's messages since the stored cursor, page by page, then the replies of the Topics those messages show. The chat
 * cursor only advances after every page is received.
 *
 * @param deps Backfill dependencies: the dsh-im service, the Source context, the account fingerprint and the abort signal.
 * @param chatId Chat to read.
 * @param direct Whether the chat is a direct message chat, which its Triggers must record.
 * @param limit Page size, at most what dsh-im accepts for history reads.
 * @param errors Collects the failures this round reports; a failing Topic does not stop the others.
 * @returns How many Triggers this round delivered and how many messages dsh-im could not render.
 */
export async function backfillChat(deps: BackfillDeps, chatId: string, direct: boolean, limit: number,
  errors: string[], deadline = Date.now() + ROUND_MS): Promise<Recovered> {
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
  const topics: Record<string, Topic> = { ...deps.ctx.state.get(topicsKey) }
  let cursor: string | undefined
  let count = 0
  let omitted = 0
  let drained = false
  let lastAt: string | undefined
  for (let page = 0; page < MAX_PAGES; page++) {
    if (deps.signal.aborted) return { recovered: count, omitted }
    // A round that runs out of time keeps its progress: the cursor below moves only as far as this round read.
    if (Date.now() > deadline) break
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
      // The first message seen for a topic roots it — its own reply's root when it has one — and starts reading from the
      // chat window's beginning; a later one only moves its last activity.
      if (topicId && !known)
        topics[topicId] = { at: event.at, rootId: event.reply.rootId ?? event.messageId, readThrough: since }
      else if (topicId && known && Date.parse(event.at) > Date.parse(known.at))
        topics[topicId] = { ...known, at: event.at }
    }
    cursor = result.nextCursor
    if (!cursor) { drained = true; break }
  }
  // Persist what this round discovered before the cursor moves past the message that named each Topic: after a restart the
  // chat history no longer returns it, so an unrecorded Topic is never read.
  if (!deps.signal.aborted) deps.ctx.state.set(topicsKey, topics)
  // Advance only as far as this round actually read: a page cap must not skip the rest of the window.
  if (!deps.signal.aborted) {
    if (drained) deps.ctx.state.set(key, cursorAt(now))
    else if (lastAt) deps.ctx.state.set(key, cursorAt(lastAt))
  }
  let replies: Recovered = { recovered: 0, omitted: 0 }
  if (!deps.signal.aborted) {
    try { replies = await recoverTopics(deps, chatId, direct, since, now, limit, topics, errors, deadline) } catch (error) {
      // The chat's own messages were delivered and its cursor advanced; only this round's Topic recovery is lost.
      if (!deps.signal.aborted) errors.push(`topics: ${String(error)}`)
    }
  }
  if (!deps.signal.aborted) deps.ctx.state.set(topicsKey, topics)
  return { recovered: count + replies.recovered, omitted: omitted + replies.omitted }
}

/**
 * Reads the replies Feishu keeps out of chat history: the chat container returns the message a topic was opened on, while
 * its replies are only in the thread container. Each topic this chat had activity in is read from its own watermark, so a
 * reply missed while the Host was down is recovered instead of waiting for the next live event. A topic the Thread poller
 * reads this round is skipped, and one quiet for the Thread polling horizon is forgotten.
 */
async function recoverTopics(deps: BackfillDeps, chatId: string, direct: boolean, since: string, until: string, limit: number,
  topics: Record<string, Topic>, errors: string[], deadline: number): Promise<Recovered> {
  const read = deps.im.threadHistoryChecked
  if (!read) return { recovered: 0, omitted: 0 }
  // A Topic is left to the Thread poller only while that poller actually runs: a Thread past its polling horizon is skipped
  // there, and skipping the Topic here as well would leave it unread until it is forgotten.
  const followed = new Set<string>()
  for (const thread of deps.ctx.threads.subscribed(SOURCE)) {
    if (!polled(deps.ctx, thread)) continue
    const topicId = (thread.meta as { threadId?: string }).threadId
      ?? deps.ctx.triggers.latest(thread.key)?.meta.threadId
    if (typeof topicId === 'string') followed.add(topicId)
  }
  for (const [topicId, topic] of Object.entries(topics)) {
    if (Date.parse(until) - Date.parse(topic.at) >= POLL_HORIZON_MS) delete topics[topicId]
  }
  // Least recently read first, so a Topic that dropped out of the rotation still moves forward every round instead of
  // starving until it is forgotten with its replies unread.
  const readable = Object.entries(topics).filter(([topicId]) => !followed.has(topicId))
    .sort(([, left], [, right]) => Date.parse(left.readThrough ?? since) - Date.parse(right.readThrough ?? since))
    .slice(0, TOPIC_READS_PER_ROUND)
  let recovered = 0
  let omitted = 0
  let failures = 0
  for (const [topicId, topic] of readable) {
    if (deps.signal.aborted || Date.now() > deadline) break
    const from = topic.readThrough ?? since
    let cursor: string | undefined
    let newest = topic.at
    let readThrough = from
    let drained = false
    try {
      for (let page = 0; page < MAX_PAGES; page++) {
        if (Date.now() > deadline) break
        const result = await read(deps.botId,
          { chatId, threadId: topicId, limit, since: cursorAt(from), until, direct, includeBots: true,
            ...cursor ? { cursor } : {} },
          { expectedFingerprint: deps.fingerprint, signal: signalOf(deps.signal) })
        if (!deps.config()) return { recovered, omitted }
        omitted += result.omitted
        for (const event of result.events) {
          // The reply route stays exactly as dsh-im reported it — dsh-im refuses a read or reply whose route does not match
          // the message — while the Topic root only decides which Thread the Trigger joins.
          const trigger = deps.toTrigger(event, event.reply.rootId ?? topic.rootId)
          await deps.ctx.receive(trigger)
          deps.describe?.(event, trigger)
          recovered++
          readThrough = event.at
          if (Date.parse(event.at) > Date.parse(newest)) newest = event.at
        }
        cursor = result.nextCursor
        if (!cursor) { drained = true; break }
      }
    } catch (error) {
      // One unreadable topic must not stall the chat's cursor; it is tried again next round until it goes quiet.
      if (!deps.signal.aborted) {
        failures++
        if (failures <= REPORTED_TOPIC_ERRORS) errors.push(`topic ${topicId}: ${String(error)}`)
      }
    }
    // Resume where delivery stopped; a drained read has caught up to `until`. A read that stopped on its page or time
    // budget, or failed, keeps the window it did not finish, so nothing is skipped.
    const progressed = drained ? until : readThrough
    topics[topicId] = { at: newest, rootId: topic.rootId, readThrough: progressed }
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
    if (code === 'stale-route' || code === 'message-unavailable' || code === 'message-deleted') deps.ctx.state.set(key, { at })
    return undefined
  }
}

/** One subscribed thread's new messages since its stored cursor; thread ids come from the Thread, its latest Trigger, or its root message. */
async function pollThread(deps: BackfillDeps, thread: Thread, deadline: number): Promise<Recovered> {
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
  // The round's time budget applies here too; the stored cursor resumes the Thread next round.
  if (Date.now() > deadline) return { recovered: 0, omitted: 0 }
  for (let page = 0; page < MAX_PAGES; page++) {
    if (deps.signal.aborted) return { recovered: count, omitted }
    if (Date.now() > deadline) break
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
    // One budget for the whole round: a slow remote stretches the interval instead of starving later chats.
    const deadline = Date.now() + ROUND_MS
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
        const round = await backfillChat(deps, chatId, direct, HISTORY_PAGE, errors, deadline)
        recovered += round.recovered
        omitted += round.omitted
      } catch (error) { if (!deps.signal.aborted) errors.push(`${chatId}: ${String(error)}`) }
    }
    if (!deps.signal.aborted) {
      try {
        await pollThreads(deps.ctx, 'feishu', () => deps.signal.aborted || Date.now() > deadline,
          thread => pollThread(deps, thread, deadline).then(round => {
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

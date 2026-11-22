/** dsh-im Feishu background loops: chat history backfill and subscribed-thread polling. */
import type { Thread, Trigger } from '../core/index.ts'
import { KV } from '../kv.ts'
import { SOURCE } from './feishu-trigger.ts'
import type { DshImEvent, DshImService } from './dsh-im.ts'
import type { FeishuConfig } from './feishu-im.ts'
import { cursorAt, pause, pollThreads } from './poll.ts'
import type { Health, SourceContext } from './host.ts'

const HISTORY_INTERVAL_MS = 60_000
const MAX_PAGES = 20
const FIRST_LOOKBACK_MS = 10 * 60_000
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

/** Merges dsh-im's known group targets into the stored label map; the read is local, so it runs every round. */
async function refreshChatLabels(deps: BackfillDeps): Promise<void> {
  if (!deps.im.listTargets) return
  try {
    const labels: Record<string, string> = { ...(deps.ctx.state.get(KV.sources.labels(SOURCE)) ?? {}) }
    let changed = false
    for (const target of await deps.im.listTargets(deps.botId)) {
      const chatId = target.route?.chatId
      if (target.kind !== 'group' || !chatId || !target.name || labels[chatId] === target.name) continue
      labels[chatId] = target.name
      changed = true
    }
    if (changed) deps.ctx.state.set(KV.sources.labels(SOURCE), labels)
  } catch { /* Labels are a display aid; the stored map stays until a later read succeeds. */ }
}

/** One chat's messages since the stored cursor, page by page; the cursor only advances after every page is received. */
async function backfillChat(deps: BackfillDeps, chatId: string, direct: boolean, limit: number): Promise<number> {
  const read = deps.im.chatHistoryChecked
  if (!read) return 0
  const key = KV.feishu.history(chatId)
  const now = new Date().toISOString()
  const since = deps.ctx.state.get(key) ?? new Date(Date.now() - FIRST_LOOKBACK_MS).toISOString()
  let cursor: string | undefined
  let count = 0
  let drained = false
  let lastAt: string | undefined
  for (let page = 0; page < MAX_PAGES; page++) {
    if (deps.signal.aborted) return count
    const result = await read(deps.botId,
      { chatId, limit, since, until: now, direct, includeBots: true, ...cursor ? { cursor } : {} },
      { expectedFingerprint: deps.fingerprint, signal: signalOf(deps.signal) })
    if (!deps.config()) return count
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
  // Advance only as far as this round actually read: a page cap must not skip the rest of the window.
  if (!deps.signal.aborted) {
    if (drained) deps.ctx.state.set(key, cursorAt(now))
    else if (lastAt) deps.ctx.state.set(key, cursorAt(lastAt))
  }
  return count
}

/** One subscribed thread's new messages since its stored cursor; thread ids come from the Thread or its latest Trigger. */
async function pollThread(deps: BackfillDeps, thread: Thread): Promise<number> {
  const read = deps.im.threadHistoryChecked
  if (!read) return 0
  const meta = thread.meta as { chatId?: string; threadId?: string }
  const latest = deps.ctx.triggers.latest(thread.key)
  const chatId = meta.chatId ?? (typeof latest?.meta.chatId === 'string' ? latest.meta.chatId : undefined)
  const threadId = meta.threadId ?? (typeof latest?.meta.threadId === 'string' ? latest.meta.threadId : undefined)
  if (!chatId || !threadId) return 0
  const key = KV.feishu.history(`thread:${thread.key}`)
  const now = new Date().toISOString()
  const since = deps.ctx.state.get(key) ?? new Date(Date.parse(thread.createdAt) - 5000).toISOString()
  const limit = deps.config()?.readPageSize ?? 20
  let cursor: string | undefined
  let count = 0
  let drained = false
  let lastAt: string | undefined
  for (let page = 0; page < MAX_PAGES; page++) {
    if (deps.signal.aborted) return count
    const result = await read(deps.botId,
      { chatId, threadId, limit, since, until: now, includeBots: true, ...cursor ? { cursor } : {} },
      { expectedFingerprint: deps.fingerprint, signal: signalOf(deps.signal) })
    if (!deps.config()) return count
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
  return count
}

/**
 * Recovers messages the live consumer missed — while superagent restarted, the Host was down, or the bot was handed off —
 * by reading each configured chat's history between stored cursors and polling subscribed threads. Re-received messages
 * deduplicate on their deterministic Trigger id, so overlap with the live stream is safe.
 */
export async function runFeishuImBackfill(deps: BackfillDeps): Promise<void> {
  await deps.ctx.ready
  if (!deps.im.chatHistoryChecked || !deps.im.threadHistoryChecked) {
    deps.report('history', { state: 'paused', detail: 'This dsh-im does not expose Feishu backfill' })
    return
  }
  while (!deps.signal.aborted) {
    const config = deps.config()
    if (!config) {
      deps.report('history', { state: 'paused', detail: 'The Feishu Source is off' })
      await pause(HISTORY_INTERVAL_MS, deps.signal)
      continue
    }
    await refreshChatLabels(deps)
    const errors: string[] = []
    let recovered = 0
    const chats = new Map<string, boolean>()
    for (const chat of config.chats) chats.set(chat.chatId, false)
    for (const [chatId, direct] of Object.entries(deps.ctx.state.get(KV.feishu.chats) ?? {})) {
      if (direct && config.directMessages) chats.set(chatId, true)
    }
    for (const [chatId, direct] of chats) {
      if (deps.signal.aborted) break
      try { recovered += await backfillChat(deps, chatId, direct, config.readPageSize) }
      catch (error) { if (!deps.signal.aborted) errors.push(`${chatId}: ${String(error)}`) }
    }
    if (!deps.signal.aborted) {
      try {
        await pollThreads(deps.ctx, 'feishu', () => deps.signal.aborted, thread => pollThread(deps, thread).then(count => { recovered += count }))
      } catch (error) { if (!deps.signal.aborted) errors.push(`threads: ${String(error)}`) }
    }
    if (!deps.signal.aborted) {
      deps.report('history', errors.length
        ? { state: 'error', detail: errors.join('\n') }
        : { state: 'ready', detail: `Recovered ${recovered} messages from chat history and subscribed threads`, at: new Date().toISOString() })
    }
    await pause(HISTORY_INTERVAL_MS, deps.signal)
  }
}

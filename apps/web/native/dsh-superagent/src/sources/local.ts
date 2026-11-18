/** Local ticket Source: requests created from the management panel, archived as Markdown, with the agent's replies appended to the same file. */
import { appendFile, mkdir, readFile, readdir, rename, rm, writeFile } from 'node:fs/promises'
import { randomUUID } from 'node:crypto'
import { dirname, join } from 'node:path'
import type { Context } from '@deepseek-ai/cordis'
import { z } from 'zod'
import { key, latestReport, type OutboundMessage, type Ticket, type TicketReview, type Trigger } from '../core/index.ts'
import { family } from '../kv.ts'
import type { Settings } from '../policy.ts'
import { sourceContext, type SourceContext, type SourceHost } from './host.ts'
import type { Localized, SourceKind } from './kind.ts'
import type { SourceRegistry } from './registry.ts'

export const SOURCE = 'local'
/** Actor id of the person who creates a ticket in the panel; the saved Policy part lists it by default. */
export const OPERATOR = 'operator'
const absolute = (value: string): boolean => value === '' || /^(?:\/|[A-Za-z]:[\\/]|\\\\)/.test(value)

const LocalConfigSchema = z.object({
  archive: z.string().trim().refine(absolute, 'An absolute path, or empty for <stateDir>/tickets')
    .default(''),
}).strict()
export type LocalConfig = z.infer<typeof LocalConfigSchema>
/** A panel request; `key` is the client's idempotency key, so a retry after a lost response returns the first result. */
export type LocalRequest = { title: string; text: string; key?: string }
export interface LocalRequestResult { request: string; thread: string; trigger: string; file: string }
/** Stored result per client idempotency key, so retrying a creation returns the ticket it already made. */
const REQUEST_RESULTS = family<LocalRequestResult, [key: string]>('local-request:')
/** In-flight creations by idempotency key, so two simultaneous submissions make one archive file and one Trigger. */
const pendingRequests = new Map<string, Promise<LocalRequestResult>>()
export type LocalFollowup = { thread: string; text: string }
export interface LocalFollowupResult { trigger: string; thread: string }

/** Directory holding one Markdown file per request: the configured one, or `tickets` under the state directory. */
export const archiveDir = (deployment: { stateDir: string }, config: LocalConfig): string =>
  config.archive || join(deployment.stateDir, 'tickets')
export const archiveFile = (dir: string, request: string): string => join(dir, `${request}.md`)
export const requestId = (at: string): string => `local-${at.replace(/[-:]/g, '').replace(/\..*$/, '')}-${randomUUID().slice(0, 8)}`
const REQUEST_LABELS: Record<string, Localized> = {
  acknowledgement: { zh: '已接单', en: 'Accepted' },
  reply: { zh: '结果', en: 'Result' },
  notice: { zh: '通知', en: 'Notice' },
}

/**
 * Writes the request that starts a ticket as a Markdown file, refusing to overwrite an existing one.
 * @param file - Absolute path of the ticket's archive file.
 */
export async function writeRequest(file: string, request: { id: string; title: string; text: string; at: string }): Promise<void> {
  await mkdir(dirname(file), { recursive: true })
  // Front matter is one line per field, and the title also becomes the heading, so a pasted line break becomes a space.
  const title = request.title.replace(/\s+/g, ' ').trim()
  await writeFile(file, `---\nid: ${request.id}\ntitle: ${JSON.stringify(title)}\nsource: ${SOURCE}\ncreated: ${request.at}\n---\n\n# ${title}\n\n${request.text.trim()}\n`, { flag: 'wx' })
}
/** One writer at a time per archive file, so a read-check-append pair cannot interleave with another write to it. */
const archiveTails = new Map<string, Promise<unknown>>()
async function serialized<T>(file: string, work: () => Promise<T>): Promise<T> {
  const previous = archiveTails.get(file) ?? Promise.resolve()
  const started = previous.then(work, work)
  const tail = started.then(() => undefined, () => undefined)
  archiveTails.set(file, tail)
  try { return await started }
  finally { if (archiveTails.get(file) === tail) archiveTails.delete(file) }
}
/** Idempotency marker of one outbound message, so a delivery retried after a crash does not append its text twice. */
const replyMarker = (id: string): string => `<!-- dsh-superagent:${id} -->`
/**
 * Appends one outbound message to the ticket's Markdown file, as a section named for its purpose.
 * The message's ID marks the section; appending the same message again leaves the file unchanged.
 */
export async function appendReply(file: string, entry: { id: string; at: string; kind: string; text: string }): Promise<void> {
  const label = REQUEST_LABELS[entry.kind] ?? { zh: entry.kind, en: entry.kind }
  const marker = replyMarker(entry.id)
  await serialized(file, async () => {
    const existing = await readFile(file, 'utf8').catch(() => '')
    if (existing.includes(marker)) return
    await appendFile(file, `\n## ${entry.at} · ${label.zh} / ${label.en}\n\n${marker}\n${entry.text.trim()}\n`)
  })
}
/** Appends a person's follow-up to the ticket's Markdown file, in the same conversation as the replies. */
export async function appendFollowup(file: string, entry: { at: string; text: string }): Promise<void> {
  await serialized(file, () => appendFile(file, `\n## ${entry.at} · 追加 / Follow-up\n\n${entry.text.trim()}\n`))
}
/** The archive file of a ticket accepted from a local request, taken from its Channel address. */
export const ticketArchive = (ticket: Pick<Ticket, 'channels'>): string | undefined =>
  ticket.channels.flatMap(channel => channel.source === SOURCE ? [channel.address.file] : []).find((file): file is string => !!file)
const oneLine = (text: string, limit = 2000): string => text.replace(/\s+/g, ' ').trim().slice(0, limit)
/** A ticket's display number as the panel and the archive write it, such as `T-0001`. */
export const displayNumber = (number: number | undefined): string | undefined => number ? `T-${String(number).padStart(4, '0')}` : undefined
/** Git facts of one ended ticket as archive lines; omitted fields do not appear. */
function reviewLines(review: TicketReview): string[] {
  return [
    ...(review.error ? [`- review_error: ${oneLine(review.error)}`] : []),
    ...(review.branch ? [`- branch: ${review.branch}`] : []),
    ...(review.base ? [`- base: ${review.base}`] : []),
    ...(review.head ? [`- head: ${review.head}`] : []),
    ...(review.stat ? [`- diffstat: ${oneLine(review.stat)}`] : []),
    `- commits: ${review.commits.length}`,
    ...review.commits.slice(0, 10).map(commit => `  - ${commit.sha} ${oneLine(commit.subject, 200)}`),
    `- changed_files: ${review.files.length}`,
    ...review.files.slice(0, 20).map(item => `  - ${item.status} ${item.path}`),
    `- uncommitted: ${review.dirty.length}`,
    ...review.dirty.slice(0, 20).map(item => `  - ${item.status} ${item.path}`),
  ]
}
/**
 * Appends a ticket's ending, its summary, and the review captured for it to its archive, once per end.
 * A missing archive file, or one without a local Channel, is left alone.
 * @param ticket - The ended ticket.
 * @param review - Git facts of its workspace, when it had any.
 * @param number - The ticket's stable display number, when the store knows it.
 */
export async function closeArchive(ticket: Ticket, review?: TicketReview, number?: number): Promise<void> {
  const file = ticketArchive(ticket)
  const ended = ticket.ended
  if (!file || !ended) return
  const marker = `<!-- dsh-superagent:closed:${ended.at} -->`
  await serialized(file, async () => {
    const existing = await readFile(file, 'utf8').catch(() => undefined)
    if (existing === undefined || existing.includes(marker)) return
    const summary = latestReport(ticket)?.summary ?? ''
    await appendFile(file, ['', `## ${ended.at} · 关闭 / Closed`, '', marker, '- status: closed', `- outcome: ${ended.outcome}`,
      `- by: ${ended.by}`, ...(displayNumber(number) ? [`- ticket: ${displayNumber(number)}`] : []),
      ...(ended.reason ? [`- reason: ${oneLine(ended.reason)}`] : []),
      ...(summary ? [`- summary: ${oneLine(summary)}`] : []), ...(review ? reviewLines(review) : []), ''].join('\n'))
  })
}
/**
 * Appends a reopening to a ticket's archive, once per `at`, so the file states its status rather than ending at the close.
 * @param ticket - The reopened ticket.
 * @param at - The reopening time, which also marks the section.
 */
export async function reopenArchive(ticket: Pick<Ticket, 'channels'>, at: string): Promise<void> {
  const file = ticketArchive(ticket)
  if (!file) return
  const marker = `<!-- dsh-superagent:reopened:${at} -->`
  await serialized(file, async () => {
    const existing = await readFile(file, 'utf8').catch(() => undefined)
    if (existing === undefined || existing.includes(marker)) return
    await appendFile(file, ['', `## ${at} · 重新打开 / Reopened`, '', marker, '- status: open', `- by: ${OPERATOR}`, ''].join('\n'))
  })
}
/** Channel adapter: the archive file is the destination, so a sent message is already recorded. */
export async function sendLocal(_host: Pick<SourceContext, 'commands'>, message: OutboundMessage): Promise<'sent' | 'gone'> {
  const file = message.channel.address.file ?? ''
  if (!file) return 'gone'
  await appendReply(file, { id: message.id, at: new Date().toISOString(), kind: message.kind, text: message.text })
  return 'sent'
}

export const localKind: SourceKind<LocalConfig> = {
  kind: SOURCE,
  title: { zh: '本地工单', en: 'Local tickets' },
  description: { zh: '在面板里新建的工单；请求和代理的回复都写进归档目录里的 Markdown 文件。', en: 'Tickets created in the panel; each request and the agent\'s replies are written to a Markdown file in the archive directory.' },
  schema: LocalConfigSchema,
  fields: [
    { key: 'archive', type: 'text', label: { zh: '归档目录', en: 'Archive directory' },
      hint: { zh: '绝对路径；留空用状态目录下的 tickets。每个工单一个 Markdown 文件，请求、回复和结果都追加在里面。', en: 'An absolute path; empty uses tickets under the state directory. One Markdown file per ticket holds the request, replies and results.' } },
  ],
  // Intake is request-driven: nothing polls, so the kind reports no health entry.
  health: [],
  scope: () => ({ listened: true, observed: false, row: 'requests' }),
  channel: {
    allowed: () => true,
    send: sendLocal,
    link: () => ({ url: '', label: { zh: '本地工单', en: 'Local ticket' } }),
  },
  threadContext: {
    // Each Trigger of a Thread is one person's message, so its own id names it (several follow-ups share a request id).
    messageId: trigger => String(trigger.meta.messageId ?? trigger.meta.requestId ?? ''),
    read: async (_commands, trigger) => {
      const message = { id: String(trigger.meta.messageId ?? trigger.meta.requestId ?? ''), from: OPERATOR, at: trigger.at,
        text: trigger.text }
      return { messages: [message], body: { messages: [message] } }
    },
  },
  find: query => {
    const found = query.match(/\blocal-[A-Za-z0-9-]+/g) ?? []
    return { threads: [...new Set(found.map(id => `${SOURCE}:${id}`))], terms: [...new Set(found)] }
  },
  policy: {
    schema: z.object({ allowed: z.array(z.object({ id: z.string().min(1), name: z.string().trim().max(100).optional() }).strict())
      .default([{ id: OPERATOR, name: 'Operator' }]) }).strict(),
    fields: [{ key: 'allowed', type: 'rows', label: { zh: '能在面板里新建工单的人', en: 'People who may create tickets in the panel' },
      columns: [{ key: 'id', type: 'text', label: { zh: '标识', en: 'Id' } }, { key: 'name', type: 'text', label: { zh: '名字', en: 'Name' }, optional: true }] }],
    explicit: { zh: '在面板里新建的工单总是明确的请求，不需要名单以外的授权。', en: 'A ticket created in the panel is always an explicit request; nothing beyond the list authorizes it.' },
    rows: () => [{ id: 'requests', label: { zh: '面板新建', en: 'Created in the panel' }, note: { zh: '总是接收', en: 'Always received' },
      cells: { listedUnaddressed: false, unlistedAddressed: true, unlistedUnaddressed: false } }],
  },
}

/**
 * Saves the kind's default Policy part when the Source has none yet, so a fresh install admits the operator without a first save.
 * An existing part is left as saved, including an empty allowed list, which then refuses new tickets.
 */
export async function seedPolicy(host: { store: { settings(): Settings; blockRevision(block: 'policy'): number }
  settings: { save(block: 'policy', value: unknown, revision: number, origin: 'panel'): Promise<unknown> }
  sources: Pick<SourceRegistry, 'policyPart'> }): Promise<void> {
  const settings = host.store.settings()
  if (settings.policy.sources[SOURCE]) return
  const part = host.sources.policyPart(SOURCE)
  if (!part?.allowed.length) return
  // Another save can move the revision between reading and writing; the next request seeds then.
  await host.settings.save('policy', { ...settings.policy, sources: { ...settings.policy.sources, [SOURCE]: part } },
    host.store.blockRevision('policy'), 'panel').catch(() => { /* A concurrent save moved the revision; the next request seeds again. */ })
}

/**
 * Writes a panel request to its archive file and ingests the Trigger that starts its Ticket. A request with an idempotency
 * key returns the result an earlier one stored, so a retry after a lost response does not create a second ticket.
 * @throws when the Source is off, the archive cannot be written, or the Source's scope filters the Trigger.
 */
export async function createLocalRequest(host: SourceHost, input: LocalRequest): Promise<LocalRequestResult> {
  const idempotency = input.key
  if (!idempotency) return createLocalRequestOnce(host, input)
  const running = pendingRequests.get(idempotency)
  if (running) return running
  const started = createLocalRequestOnce(host, input, idempotency).finally(() => pendingRequests.delete(idempotency))
  pendingRequests.set(idempotency, started)
  return started
}
async function createLocalRequestOnce(host: SourceHost, input: LocalRequest, idempotency?: string): Promise<LocalRequestResult> {
  const ctx = sourceContext(host, localKind)
  const config = ctx.config<LocalConfig>(SOURCE)
  if (!config) throw new Error('The local Source is off; turn it on in Settings → Sources')
  if (idempotency) {
    const stored = ctx.state.get(REQUEST_RESULTS(idempotency))
    if (stored) return stored
  }
  const at = new Date().toISOString()
  const id = requestId(at)
  const thread = `${SOURCE}:${id}`
  // A title is one line in the front matter, the heading and the Thread label.
  const title = input.title.replace(/\s+/g, ' ').trim()
  const file = archiveFile(archiveDir(ctx.deployment, config), id)
  await writeRequest(file, { id, title, text: input.text, at })
  const trigger: Trigger = {
    id: key(SOURCE, id), source: SOURCE, threads: [{ source: SOURCE, id }],
    actor: { source: SOURCE, id: OPERATOR }, human: true, addressed: 'direct', at,
    // The person's title leads the request text, so the Session that accepts it can keep that wording.
    text: `${title}\n\n${input.text}`,
    channel: { source: SOURCE, thread, address: { request: id, file } },
    meta: { requestId: id, messageId: id, title, ...idempotency ? { idempotency } : {} },
  }
  // Describe the Thread after the intake created it; describing an absent Thread is a no-op.
  const received = await ctx.receive(trigger)
  if (received !== 'filtered') ctx.threads.describe(thread, { label: title })
  if (received === 'filtered') {
    // No Ticket will read the file, so remove it rather than leave an archive no part of the panel shows.
    await rm(file, { force: true })
    throw new Error('The local Source filtered the request; check the Source and the Policy')
  }
  const result = { request: id, thread, trigger: trigger.id, file }
  if (idempotency) ctx.state.set(REQUEST_RESULTS(idempotency), result)
  return result
}

/**
 * Adds a person's message to an existing local ticket's Thread, where the Ticket's worker reads it as an Input (and an ended Ticket resumes).
 * @throws when the Source is off, `thread` is not a local Thread, or the Thread has no archived request.
 */
export async function createLocalFollowup(host: SourceHost, input: LocalFollowup): Promise<LocalFollowupResult> {
  const ctx = sourceContext(host, localKind)
  if (!ctx.config<LocalConfig>(SOURCE)) throw new Error('The local Source is off; turn it on in Settings → Sources')
  const thread = ctx.threads.get(input.thread)
  if (!thread || thread.ref.source !== SOURCE) throw new Error('The local ticket Thread is unknown')
  const previous = ctx.triggers.latest(input.thread)
  const request = String(previous?.meta.requestId ?? '')
  const file = previous?.channel?.address.file ?? ''
  if (!request || !file) throw new Error('The local ticket has no archive file')
  const at = new Date().toISOString()
  const id = requestId(at)
  // A failed intake must not leave the message in the archive as if the Ticket had received it.
  const before = await readFile(file, 'utf8').catch(() => undefined)
  const restore = async (): Promise<void> => { if (before !== undefined) await writeFile(file, before).catch(() => { /* The archive is a record; its loss must not hide the intake failure. */ }) }
  await appendFollowup(file, { at, text: input.text })
  const trigger: Trigger = {
    id: key(SOURCE, id), source: SOURCE, threads: [thread.ref], actor: { source: SOURCE, id: OPERATOR }, human: true,
    addressed: 'direct', at, text: input.text, channel: { source: SOURCE, thread: input.thread, address: { request, file } },
    meta: { requestId: request, messageId: id, followup: true },
  }
  let received: Awaited<ReturnType<SourceContext['receive']>>
  try { received = await ctx.receive(trigger) } catch (error) { await restore(); throw error }
  if (received === 'filtered') { await restore(); throw new Error('The local Source filtered the follow-up; check the Source and the Policy') }
  return { trigger: trigger.id, thread: input.thread }
}

/** Directory a command-line client drops ticket requests into, under the state directory. */
export const inboxDir = (deployment: { stateDir: string }): string => join(deployment.stateDir, 'inbox')
const INBOX_REQUEST = z.object({ title: z.string().trim().min(1).max(120), text: z.string().trim().min(1).max(20_000),
  key: z.string().trim().min(8).max(200).optional() }).strict()
/**
 * Ingests the JSON requests a command-line client drops into `<stateDir>/inbox`, writing each result to
 * `done/<file>` and moving invalid or refused files to `rejected/`. A request without a key uses its file name, so a
 * retried file returns the ticket the first attempt made.
 * @returns The number of requests created or recognized.
 */
export async function drainInbox(host: SourceHost): Promise<number> {
  const ctx = sourceContext(host, localKind)
  if (!ctx.config<LocalConfig>(SOURCE)) return 0
  const directory = inboxDir(ctx.deployment)
  const files = await readdir(directory).catch(() => undefined)
  if (!files) return 0
  let handled = 0
  for (const file of files.filter(name => name.endsWith('.json')).sort()) {
    const path = join(directory, file)
    const content = await readFile(path, 'utf8').catch(() => undefined)
    if (content === undefined) continue
    let request: LocalRequest
    try { request = INBOX_REQUEST.parse(JSON.parse(content)) }
    catch (error) {
      await mkdir(join(directory, 'rejected'), { recursive: true })
      await rename(path, join(directory, 'rejected', file))
      continue
    }
    try {
      const result = await createLocalRequest(host, { ...request, key: request.key ?? file.replace(/\.json$/, '') })
      await mkdir(join(directory, 'done'), { recursive: true })
      await writeFile(join(directory, 'done', file), JSON.stringify(result))
      await rm(path, { force: true })
      handled++
    } catch (error) {
      await mkdir(join(directory, 'rejected'), { recursive: true })
      await writeFile(join(directory, 'rejected', `${file}.error`), String(error))
      await rm(path, { force: true })
    }
  }
  return handled
}
/** Appends one lifecycle section to the archive under a marker that keeps a retried operation from duplicating it. */
async function appendLifecycle(ticket: Pick<Ticket, 'channels'>, marker: string, heading: string, lines: string[]): Promise<void> {
  const file = ticketArchive(ticket)
  if (!file) return
  await serialized(file, async () => {
    const existing = await readFile(file, 'utf8').catch(() => undefined)
    if (existing === undefined || existing.includes(marker)) return
    await appendFile(file, ['', `## ${heading}`, '', marker, ...lines, ''].join('\n'))
  })
}
/**
 * Appends the merge of a ticket's branch into the repository to its archive.
 * @param ticket - The ticket whose branch was merged.
 * @param merged - When, which branch, and the branch it moved.
 */
export async function recordMerge(ticket: Pick<Ticket, 'channels'>, merged: { at: string; branch: string; target: string }): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:merged:${merged.at} -->`, `${merged.at} · 已合并 / Merged`,
    ['- status: merged', `- branch: ${merged.branch}`, `- target: ${merged.target}`, `- by: ${OPERATOR}`])
}
/**
 * Appends the discard of a ticket's worktree and branch to its archive.
 * @param ticket - The ticket whose worktree was discarded.
 * @param at - When it was discarded.
 */
export async function recordDiscard(ticket: Pick<Ticket, 'channels'>, at: string): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:discarded:${at} -->`, `${at} · 已丢弃 / Discarded`,
    ['- status: discarded', `- by: ${OPERATOR}`])
}
export const name = 'dsh-superagent-local'
export const inject = ['orchestrator']
export function apply(ctx: Context): void {
  ctx.effect(() => ctx.orchestrator.sources.register(localKind), 'local: source kind')
  // A command-line client drops JSON files into the inbox; the running service turns them into tickets.
  ctx.effect(() => {
    const drain = (): void => { void drainInbox(ctx.orchestrator).catch(() => { /* The next tick retries; a refusal is kept in rejected/. */ }) }
    const timer = setInterval(drain, 2000)
    drain()
    return () => clearInterval(timer)
  }, 'local: command-line inbox')
}

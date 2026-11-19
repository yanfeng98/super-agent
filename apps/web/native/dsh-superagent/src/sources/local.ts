/** Local ticket Source: requests created from the management panel, archived as Markdown, with the agent's replies appended to the same file. */
import { appendFile, mkdir, readFile, readdir, rename, rm, writeFile } from 'node:fs/promises'
import { randomUUID } from 'node:crypto'
import { dirname, join } from 'node:path'
import type { Context } from '@deepseek-ai/cordis'
import { z } from 'zod'
import { key, latestReport, type ActionOrigin, type OutboundMessage, type Ticket, type TicketCheck, type TicketReview, type Trigger }
  from '../core/index.ts'
import { family } from '../kv.ts'
import type { Settings, SettingsOrigin } from '../policy.ts'
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
  webhook: z.string().trim().refine(value => value === '' || /^https?:\/\/\S+$/.test(value), 'An http(s) URL, or empty').default(''),
  projects: z.array(z.object({
    name: z.string().trim().min(1).max(60).regex(/^[A-Za-z0-9][A-Za-z0-9_.-]*$/, 'Letters, digits, dot, dash or underscore'),
    root: z.string().trim().refine(absolute, 'An absolute path, or empty').default(''),
    prepare: z.string().trim().max(4000).default(''),
  }).strict()).max(20).default([]),
}).strict()
export type LocalConfig = z.infer<typeof LocalConfigSchema>
/** One file sent with a panel request, as its name and base64 content. */
export type LocalAttachment = { name: string; data: string }
/** A panel request; `key` is the client's idempotency key, `project` a label a prepare command may route on, and `tags` labels for people. */
export type LocalRequest = { title: string; text: string; key?: string; project?: string; tags?: string[]; files?: LocalAttachment[];
  hold?: boolean }
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
/** The local Source's notification webhook, or empty when the Source is off or none is configured. */
export const webhookFor = (settings: Settings): string => {
  const part = settings.sources[SOURCE]
  if (!part?.enabled) return ''
  const parsed = LocalConfigSchema.safeParse(part.config)
  return parsed.success ? parsed.data.webhook : ''
}
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
export async function writeRequest(file: string, request: { id: string; title: string; text: string; at: string; project?: string;
  tags?: string[]; hold?: boolean }): Promise<void> {
  await mkdir(dirname(file), { recursive: true })
  // Front matter is one line per field, and the title also becomes the heading, so a pasted line break becomes a space.
  const title = request.title.replace(/\s+/g, ' ').trim()
  const project = request.project?.trim() ? `project: ${JSON.stringify(request.project.trim())}\n` : ''
  const tags = request.tags?.length ? `tags: ${JSON.stringify(request.tags)}\n` : ''
  const hold = request.hold ? 'hold: true\n' : ''
  await writeFile(file, `---\nid: ${request.id}\ntitle: ${JSON.stringify(title)}\nsource: ${SOURCE}\n${project}${tags}${hold}created: ${request.at}\n---\n\n# ${title}\n\n${request.text.trim()}\n`, { flag: 'wx' })
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
/**
 * Writes a request's attachments beside its archive file, under `<request id>.files`, with sanitized unique names.
 * @returns The absolute paths a worker may read.
 */
async function writeAttachments(directory: string, request: string, files: LocalAttachment[]): Promise<string[]> {
  if (!files.length) return []
  const target = join(directory, `${request}.files`)
  await mkdir(target, { recursive: true, mode: 0o700 })
  const used = new Set<string>()
  const paths: string[] = []
  for (const [index, file] of files.entries()) {
    let name = file.name.replace(/[\\/\u0000-\u001f\u007f]/g, '_').replace(/^\.+/, '').slice(0, 80).trim() || `file-${index + 1}`
    if (used.has(name)) name = `${index + 1}-${name}`
    used.add(name)
    const path = join(target, name)
    await writeFile(path, Buffer.from(file.data, 'base64'), { mode: 0o600 })
    paths.push(path)
  }
  return paths
}
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
/** Whether an ended local ticket's archive still lacks the close section for its ending, so a restart can complete it. */
export async function archiveNeedsClose(ticket: Pick<Ticket, 'channels' | 'ended'>): Promise<boolean> {
  const file = ticketArchive(ticket)
  const ended = ticket.ended
  if (!file || !ended) return false
  const existing = await readFile(file, 'utf8').catch(() => undefined)
  return existing !== undefined && !existing.includes(`<!-- dsh-superagent:closed:${ended.at} -->`)
}
/**
 * Appends a ticket's ending, its summary, the review, and the check captured for it to its archive, once per end.
 * A missing archive file, or one without a local Channel, is left alone.
 * @param ticket - The ended ticket.
 * @param review - Git facts of its workspace, when it had any.
 * @param number - The ticket's stable display number, when the store knows it.
 * @param check - The workspace check run at the end, when one is configured.
 */
export async function closeArchive(ticket: Ticket, review?: TicketReview, number?: number, check?: TicketCheck): Promise<void> {
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
      ...(summary ? [`- summary: ${oneLine(summary)}`] : []), ...(review ? reviewLines(review) : []),
      ...(check ? [`- check: ${check.ok ? 'passed' : 'failed'}${check.exitCode === null ? '' : ` (exit ${check.exitCode})`}`,
        `- check_command: ${oneLine(check.command, 500)}`] : []), ''].join('\n'))
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
    { key: 'webhook', type: 'text', label: { zh: '通知 Webhook', en: 'Notification webhook' },
      hint: { zh: '工单结束或需要人回答时向这个 http(s) 地址 POST JSON；留空关闭。', en: 'POST JSON to this http(s) address when a ticket ends or needs a person; empty turns it off.' } },
    { key: 'projects', type: 'rows', label: { zh: '项目', en: 'Projects' },
      hint: { zh: '给请求里的项目名配置执行根目录和 prepare 命令；留空的项沿用 Execution 设置。', en: 'Give a project label its own execution root and prepare command; empty fields keep the Execution settings.' },
      columns: [{ key: 'name', type: 'text', label: { zh: '项目名', en: 'Name' } },
        { key: 'root', type: 'text', label: { zh: '根目录', en: 'Root' }, optional: true },
        { key: 'prepare', type: 'text', label: { zh: 'prepare 命令', en: 'Prepare command' }, optional: true }] },
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
  settings: { save(block: 'policy', value: unknown, revision: number, origin: SettingsOrigin): Promise<unknown> }
  sources: Pick<SourceRegistry, 'policyPart'> }, origin: SettingsOrigin = 'panel'): Promise<void> {
  const settings = host.store.settings()
  if (settings.policy.sources[SOURCE]) return
  const part = host.sources.policyPart(SOURCE)
  if (!part?.allowed.length) return
  // Another save can move the revision between reading and writing; the next request seeds then.
  await host.settings.save('policy', { ...settings.policy, sources: { ...settings.policy.sources, [SOURCE]: part } },
    host.store.blockRevision('policy'), origin).catch(() => { /* A concurrent save moved the revision; the next request seeds again. */ })
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
  // A project label may carry its own root and prepare command; a label without a row keeps the global settings.
  const project = input.project ? config.projects.find(item => item.name === input.project) : undefined
  const projectMeta = project ? { ...project.root ? { projectRoot: project.root } : {},
    ...project.prepare ? { projectPrepare: project.prepare } : {} } : {}
  const file = archiveFile(archiveDir(ctx.deployment, config), id)
  await writeRequest(file, { id, title, text: input.text, at, ...input.project ? { project: input.project } : {},
    ...input.tags?.length ? { tags: input.tags } : {}, ...input.hold ? { hold: true } : {} })
  let attachments: string[] = []
  try { attachments = await writeAttachments(dirname(file), id, input.files ?? []) }
  catch (error) { await rm(file, { force: true }); throw error }
  const attachmentList = attachments.length ? `\n\nAttachments:\n${attachments.map(path => `- ${path}`).join('\n')}` : ''
  if (attachments.length) await appendFile(file, `\n## 附件 / Attachments\n\n${attachments.map(path => `- ${path}`).join('\n')}\n`)
  const trigger: Trigger = {
    id: key(SOURCE, id), source: SOURCE, threads: [{ source: SOURCE, id }],
    actor: { source: SOURCE, id: OPERATOR }, human: true, addressed: 'direct', at,
    // The person's title leads the request text, so the Session that accepts it can keep that wording.
    text: `${title}\n\n${input.text}${attachmentList}`,
    channel: { source: SOURCE, thread, address: { request: id, file } },
    meta: { requestId: id, messageId: id, title, ...input.project ? { project: input.project } : {},
      ...projectMeta, ...input.tags?.length ? { tags: input.tags } : {}, ...input.hold ? { hold: true } : {},
      ...idempotency ? { idempotency } : {} },
  }
  let received: Awaited<ReturnType<typeof ctx.receive>>
  try { received = await ctx.receive(trigger) }
  catch (error) {
    // A failed intake leaves neither an archive nor attachments that no Ticket will read.
    await rm(file, { force: true })
    if (attachments.length) await rm(join(dirname(file), `${id}.files`), { recursive: true, force: true })
    throw error
  }
  // Describe the Thread after the intake created it; describing an absent Thread is a no-op.
  if (received !== 'filtered') ctx.threads.describe(thread, { label: title })
  if (received === 'filtered') {
    // No Ticket will read the file, so remove it rather than leave an archive no part of the panel shows.
    await rm(file, { force: true })
    if (attachments.length) await rm(join(dirname(file), `${id}.files`), { recursive: true, force: true })
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
export async function createLocalFollowup(host: SourceHost, input: LocalFollowup,
  options: { mainSession?: boolean } = {}): Promise<LocalFollowupResult> {
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
    meta: { requestId: request, messageId: id, followup: true, ...options.mainSession ? { mainSession: true } : {} },
  }
  let received: Awaited<ReturnType<SourceContext['receive']>>
  try { received = await ctx.receive(trigger) } catch (error) { await restore(); throw error }
  if (received === 'filtered') { await restore(); throw new Error('The local Source filtered the follow-up; check the Source and the Policy') }
  return { trigger: trigger.id, thread: input.thread }
}

/** Directory a command-line client drops ticket requests into, under the state directory. */
export const inboxDir = (deployment: { stateDir: string }): string => join(deployment.stateDir, 'inbox')
const INBOX_REQUEST = z.object({ title: z.string().trim().min(1).max(120), text: z.string().trim().min(1).max(20_000),
  key: z.string().trim().min(8).max(200).optional(), project: z.string().trim().min(1).max(60).optional(),
  tags: z.array(z.string().trim().min(1).max(24)).max(8).optional(), hold: z.boolean().optional() }).strict()
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
 * Appends a person's approval of a held ticket to its archive, once per approval.
 * @param ticket - The approved ticket.
 * @param at - When it was approved.
 */
export async function recordApproval(ticket: Pick<Ticket, 'channels'>, at: string): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:approved:${at} -->`, `${at} · 批准 / Approved`,
    ['- status: approved', `- by: ${OPERATOR}`])
}
/**
 * Appends a ticket's new title to its archive, so the file records the rename instead of rewriting the request.
 * @param ticket - The renamed ticket.
 * @param at - When it was renamed.
 * @param title - The new title.
 */
export async function recordRename(ticket: Pick<Ticket, 'channels'>, at: string, title: string): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:renamed:${at} -->`, `${at} · 标题 / Title`,
    ['- status: renamed', `- title: ${JSON.stringify(title)}`, `- by: ${OPERATOR}`])
}
/**
 * Appends a ticket's labels to its archive, so edits are recorded instead of rewriting the request's front matter.
 * @param ticket - The ticket whose labels changed.
 * @param at - When they changed.
 * @param tags - The labels after the change; an empty list records that the ticket has none.
 */
export async function recordTags(ticket: Pick<Ticket, 'channels'>, at: string, tags: string[]): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:tags:${at} -->`, `${at} · 标签 / Tags`,
    ['- status: tagged', ...tags.map(tag => `- tag: ${JSON.stringify(tag)}`), `- by: ${OPERATOR}`])
}
/**
 * Appends a person's replacement of the request text to its archive, so the edit is recorded next to the original.
 * @param ticket - The edited ticket.
 * @param request - When it was edited and the text after the edit.
 */
export async function recordRequestEdit(ticket: Pick<Ticket, 'channels'>, request: { at: string; text: string }): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:request:${request.at} -->`, `${request.at} · 请求正文 / Request`,
    ['- status: request-edited', `- by: ${OPERATOR}`, '', request.text])
}
/**
 * Appends a person's review of a ticket's changes to its archive.
 * @param ticket - The reviewed ticket.
 * @param reviewed - When it was reviewed, the decision, any note, the commit it covered, and the interface it came from.
 */
export async function recordReview(ticket: Pick<Ticket, 'channels'>, reviewed: { at: string;
  decision: 'approved' | 'changes-requested'; note?: string; head?: string; via?: ActionOrigin }): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:reviewed:${reviewed.at} -->`, `${reviewed.at} · 评审 / Review`,
    ['- status: reviewed', `- decision: ${reviewed.decision}`, ...(reviewed.head ? [`- commit: ${reviewed.head}`] : []),
      ...(reviewed.note ? [`- note: ${oneLine(reviewed.note)}`] : []), `- by: ${OPERATOR}`,
      ...(reviewed.via ? [`- via: ${reviewed.via}`] : [])])
}
/**
 * Appends the merge of a ticket's branch into the repository to its archive.
 * @param ticket - The ticket whose branch was merged.
 * @param merged - When, which branch, the branch it moved, the commit it left, and the interface that asked.
 */
export async function recordMerge(ticket: Pick<Ticket, 'channels'>, merged: { at: string; branch: string; target: string;
  strategy?: 'ff' | 'squash' | 'merge'; head?: string; via?: ActionOrigin }): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:merged:${merged.at} -->`, `${merged.at} · 已合并 / Merged`,
    ['- status: merged', `- branch: ${merged.branch}`, `- target: ${merged.target}`,
      ...(merged.head ? [`- commit: ${merged.head}`] : []), ...(merged.strategy ? [`- strategy: ${merged.strategy}`] : []),
      `- by: ${OPERATOR}`, ...(merged.via ? [`- via: ${merged.via}`] : [])])
}
/**
 * Appends the commit that undid a merge to its archive.
 * @param ticket - The ticket whose merge was reverted.
 * @param reverted - When it was reverted, the reverting commit, the branch, the merge it undoes, and the interface that asked.
 */
export async function recordRevert(ticket: Pick<Ticket, 'channels'>, reverted: { at: string; target: string; commit: string;
  merge: string; via?: ActionOrigin }): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:reverted:${reverted.at} -->`, `${reverted.at} · 已回退 / Reverted`,
    ['- status: reverted', `- branch: ${reverted.target}`, `- commit: ${reverted.commit}`, `- merge: ${reverted.merge}`,
      `- by: ${OPERATOR}`, ...(reverted.via ? [`- via: ${reverted.via}`] : [])])
}
/**
 * Appends the push that followed a merge to its archive, which says so either way since the merge stands on its own.
 * @param ticket - The ticket whose merged branch was pushed.
 * @param push - When it was pushed, whether the remote accepted it, and why not.
 */
export async function recordPush(ticket: Pick<Ticket, 'channels'>, push: { at: string; ok: boolean; error?: string }): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:pushed:${push.at} -->`, `${push.at} · 推送 / Push`,
    ['- status: ' + (push.ok ? 'pushed' : 'push-failed'), `- by: ${OPERATOR}`,
      ...(push.error ? [`- error: ${oneLine(push.error)}`] : [])])
}
/**
 * Appends a rebase of the ticket branch to its archive, once per time.
 * @param ticket - The ticket whose branch was rebased.
 * @param rebased - When it was rebased, the branch, the target, and the new base.
 */
export async function recordRebase(ticket: Pick<Ticket, 'channels'>, rebased: { at: string; branch: string; target: string;
  base: string; via?: ActionOrigin }): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:rebased:${rebased.at} -->`, `${rebased.at} · 已变基 / Rebased`,
    ['- status: rebased', `- branch: ${rebased.branch}`, `- target: ${rebased.target}`, `- base: ${rebased.base}`,
      ...(rebased.via ? [`- via: ${rebased.via}`] : []), `- by: ${OPERATOR}`])
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
/**
 * Appends the removal of a merged ticket's worktree and branch to its archive.
 * @param ticket - The ticket whose workspace was cleaned up.
 * @param at - When it was cleaned up.
 */
export async function recordCleanup(ticket: Pick<Ticket, 'channels'>, at: string, via?: ActionOrigin): Promise<void> {
  await appendLifecycle(ticket, `<!-- dsh-superagent:cleaned:${at} -->`, `${at} · 已清理 / Cleaned up`,
    ['- status: cleaned', `- by: ${OPERATOR}`, ...(via ? [`- via: ${via}`] : [])])
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

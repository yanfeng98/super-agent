/** Management HTTP routes for the web panel: they follow the bound Web host and enforce same-origin requests. */
import { isLoopbackHost } from '@deepseek-ai/dsh-host-webserver'
import type { Context } from '@deepseek-ai/cordis'
import type { IncomingMessage, ServerResponse } from 'node:http'
import { z } from 'zod'
import type { Notification } from './notifications.ts'
import type { DiffFile } from './review.ts'
import type { MergeQueueView } from './merge-queue.ts'
import { THREAD_FILTERS, TICKET_FILTERS, type Api, type ApiResponse, type RouteParams } from './api.ts'
import { sourceSkills } from './context.ts'
import type { ModelCatalog } from '@deepseek-ai/dsh-api-session-controller/types'
import { PromptConflict, SKILL_NAME, SKILL_PREFIX, TEAM_SECTIONS, builtInBase, builtInParts, defaultPrompts, savePrompts, type Prompts }
  from './prompts.ts'
import { policyVersion } from '../policy.ts'
import { SettingsConflict, type Settings } from './settings.ts'
import { SetupError, type SetupProblem } from '../setup.ts'
import { UserError, type EngineForPeople, type Ticket, type TicketCheck, type TicketNote } from '../core/index.ts'
import type { Deployment, Machine } from '../deployment.ts'
import type { DshExecutor } from '../executor/dsh.ts'
import type { Health } from '../sources/host.ts'
import type { SourceRegistry } from '../sources/registry.ts'
import type { Sessions } from './sessions.ts'
import type { Store } from './store.ts'
import type { TextCache } from './cache.ts'
import type { LinkTarget, ViewHost } from './views/records.ts'
import type { Variable } from './execution-environment.ts'
import { createLocalFollowup, createLocalRequest, OPERATOR, seedPolicy, type LocalFollowup, type LocalFollowupResult,
  type LocalRequest, type LocalRequestResult } from '../sources/local.ts'
import { policyHistory } from './settings-history.ts'
import { overview } from './views/overview.ts'
import { settingsView } from './views/settings.ts'
import { sourcesStatus, type SetupState } from './views/sources.ts'
import { decisionList, search, threadDetail, threadList } from './views/threads.ts'
import { executorLoad, ticketDetail, ticketList, ticketTags } from './views/tickets.ts'

export interface HttpHost {
  readonly store: Store
  /** Feishu message links resolved since startup, used by search to match AppLink URLs. */
  readonly sourceLinks: Pick<TextCache, 'entries'>
  readonly sources: SourceRegistry
  readonly prompts: Pick<Prompts, 'current' | 'version' | 'history' | 'preview' | 'save' | 'defaultChanged'>
  readonly sessions: Pick<Sessions, 'status' | 'notify' | 'delivery'>
  readonly settings: Pick<Settings, 'view' | 'save' | 'pause' | 'sync' | 'refreshLabels'>
  readonly engine: EngineForPeople
  readonly executor: Pick<DshExecutor, 'activeCount'>
  readonly health: Record<string, Health>
  readonly machine: Machine
  readonly masterId: string
  /** Creates a ticket request made in the panel, archives it, and ingests its Trigger. */
  readonly localRequest: (input: LocalRequest) => Promise<LocalRequestResult>
  /** Adds a message to an existing local ticket's Thread. */
  readonly localFollowup: (input: LocalFollowup) => Promise<LocalFollowupResult>
  /** Resolves once startup finishes or setup determines what is still missing. */
  readonly ready: Promise<void>
  /** Resolved deployment; throws if read while waiting for setup (`!configured()`). */
  readonly deployment: Deployment
  readonly setupProblem?: SetupProblem | undefined
  readonly feishuProblem?: SetupProblem | undefined
  configured(): boolean
  /** Reads a ticket workspace's committed diff since its base, for the panel's Changes section. */
  ticketDiff(id: string, since?: string): Promise<{ diff: string; truncated: boolean; files: DiffFile[]; base?: string; error?: string }>
  /** Merges an ended ticket's branch into the repository and records the merge. */
  mergeTicket(id: string, strategy?: 'ff' | 'squash' | 'merge'): Promise<{ branch: string; target: string }>
  /** Removes an ended ticket's linked worktree and branch. */
  discardTicket(id: string): Promise<void>
  /** Rebases an ended ticket's branch onto the repository's checked-out branch. */
  rebaseTicket(id: string): Promise<{ branch: string; target: string }>
  /** Runs the executor's check command in an ended ticket's worktree. */
  checkTicket(id: string): Promise<TicketCheck>
  /** Links or unlinks another ticket, named by id or display number. */
  linkTicket(id: string, reference: string, linked: boolean): { related: { id: string; number?: number; title: string }[] }
  /** Undoes a merged ticket's merge with a reverting commit on the branch it landed. */
  revertTicket(id: string): Promise<{ branch: string; target: string; commit: string }>
  /** Pushes the branch a merge moved again, for a push that failed. */
  pushTicket(id: string): Promise<{ ticket_id: string; pushed: true }>
  /** Takes a ticket out of the merge queue. */
  dequeueMerges(ids: readonly string[]): { removed: number; queue: { ticket_id: string; position: number }[] }
  /** Clears the recorded failure of queued tickets, so the next pass tries them again. */
  retryMerges(ids: readonly string[]): { retried: number; queue: { ticket_id: string; position: number }[] }
  /** The merge queue as the panel shows it. */
  mergeQueue(): MergeQueueView
  /** Holds or releases the merge queue. */
  pauseMergeQueue(paused: boolean): boolean
  /** Clears a ticket's standing merge request, which the main Session records. */
  cancelAutoMerge(id: string): Ticket
  /** Records or clears that a ticket waits for another ticket's merge. */
  blockTicket(id: string, reference: string, blocked: boolean):
    { ticket_id: string; blocked_by: { id: string; number?: number; title: string; merged: boolean }[] }
  /** Adds a review note anchored to a file and optional line. */
  addNote(id: string, input: { file: string; line?: number; endLine?: number; text: string }): TicketNote[]
  /** Removes a review note. */
  removeNote(id: string, noteId: string): TicketNote[]
  /** Marks a review note resolved, or open again. */
  resolveNote(id: string, noteId: string, resolved: boolean): TicketNote[]
  /** Adds a reply to a review note's discussion. */
  replyToNote(id: string, noteId: string, text: string): TicketNote[]
  /** Records a review of an ended ticket's changes. */
  reviewTicket(id: string, decision: 'approved' | 'changes-requested', note?: string): Ticket
  /** The newest panel notifications and the unread count. */
  notifications(): { items: Notification[]; unread: number }
  /** Marks one notification read. */
  readNotification(id: string): boolean
  /** Marks every notification read. */
  readAllNotifications(): number
  /** Replaces the request text the ticket shows. */
  editRequest(id: string, text: string): Ticket
  /** Removes a merged ticket's worktree and branch. */
  cleanupTicket(id: string): Promise<void>
  /** Renames a ticket without changing the recorded request text. */
  renameTicket(id: string, title: string): Ticket
  /** Replaces a ticket's labels. */
  setTags(id: string, tags: string[]): Ticket
  /** Approves a ticket held for a person's approval. */
  approveTicket(id: string): Ticket
  links(): LinkTarget
  ticketEnvironment(ticket: Ticket): Variable[]
  checkSetup(): void
  requestPass(): void
  models(): Promise<ModelCatalog>
}
/**
 * Represents a client-facing 400 or 404 error raised during management route parsing or lookup.
 * Route handlers throw it when request bodies are malformed or target records are missing, and registerHttp serializes its status and message in the HTTP response.
 */
class RouteError extends Error { constructor(readonly status: 400 | 404, message: string) { super(message) } }
async function readBody(req: IncomingMessage, maxBytes = 128 * 1024): Promise<unknown> {
  let size = 0; const chunks: Buffer[] = []
  for await (const chunk of req) {
    const bytes = Buffer.from(chunk); size += bytes.length
    if (size > maxBytes) throw new RouteError(400, `Request exceeds ${maxBytes} bytes`)
    chunks.push(bytes)
  }
  try { return JSON.parse(Buffer.concat(chunks).toString('utf8')) as unknown }
  catch { throw new RouteError(400, 'The request body is not JSON') }
}
function json(res: ServerResponse, status: number, value: unknown): void {
  res.writeHead(status, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store', 'x-content-type-options': 'nosniff' }); res.end(JSON.stringify(value))
}
export function sameOrigin(req: Pick<IncomingMessage, 'headers'>, serverHost: string, port: number): boolean {
  const host = req.headers.host
  // The panel is served from the bound host, so its own authority must pass; loopback aliases name the same local server.
  const authority = serverHost.includes(':') ? `[${serverHost}]:${port}` : `${serverHost}:${port}`
  const aliases = isLoopbackHost(serverHost) ? [`127.0.0.1:${port}`, `localhost:${port}`, `[::1]:${port}`] : []
  if (host !== authority && !aliases.includes(host ?? '')) return false
  return req.headers['x-orchestrator-request'] === '1' && (req.headers.origin === undefined || req.headers.origin === `http://${host}`)
}
/** Maps errors to HTTP status codes and structured payloads that the panel localizes in the interface language. */
function failure(error: unknown): [number, Record<string, unknown>] {
  if (error instanceof z.ZodError) {
    const issues = error.issues.map(issue => ({ path: issue.path.join('.'), message: issue.message }))
    return [400, { error: `Invalid settings: ${issues.map(issue => issue.path).join(', ')}`, issues }]
  }
  if (error instanceof SetupError) return [400, { error: error.message, setup: error.problem }]
  if (error instanceof RouteError) return [error.status, { error: error.message }]
  if (error instanceof UserError) return [error.missing ? 404 : 400, { error: error.message, code: error.code, values: error.values }]
  if (error instanceof SettingsConflict || error instanceof PromptConflict) return [409, { error: error.message }]
  return [500, { error: error instanceof Error ? error.message : String(error) }]
}

function machineView({ stateDir, promptMaxChars, admissionContextMaxPages, admissionContextTimeoutMs }: Machine) {
  return { stateDir, promptMaxChars: promptMaxChars.get(), admissionContextMaxPages: admissionContextMaxPages.get(),
    admissionContextTimeoutMs: admissionContextTimeoutMs.get() }
}
function setupView(runtime: HttpHost) {
  const resolved = runtime.configured() ? runtime.deployment : undefined
  return {
    machine: machineView(runtime.machine), problem: runtime.setupProblem ?? null, feishuProblem: runtime.feishuProblem ?? null,
    accounts: resolved ? { repository: resolved.repository, agent: resolved.github.login, feishu: resolved.feishu ? { appId: resolved.feishu.appId, brand: resolved.feishu.brand, botId: resolved.feishu.botId, name: resolved.feishu.name } : null } : null,
  }
}
/** Build the `/orchestrator/api/state` response consumed by the management panel and `scripts/dsh-superagent status` (`docs/006-deployment.md`). */
export function stateView(runtime: HttpHost) {
  const settings = runtime.store.settings()
  return { setup: setupView(runtime), load: { ...executorLoad(runtime.store, runtime.executor.activeCount()),
    maxWorkers: settings.executor.maxWorkers },
    masterId: runtime.masterId, paused: settings.paused, health: runtime.health }
}
const views = (runtime: HttpHost): ViewHost => ({ store: runtime.store, sources: runtime.sources, links: runtime.links() })
const setupState = (runtime: HttpHost): SetupState => ({ deployment: runtime.configured() ? runtime.deployment : undefined,
  problem: runtime.setupProblem, feishuProblem: runtime.feishuProblem })
export type StateView = ReturnType<typeof stateView>
/** Builds a ticket's detail view; `environment` is empty while waiting for setup because it requires the resolved deployment. */
function ticketPage(runtime: HttpHost, id: string) {
  const detail = ticketDetail(views(runtime), id), ticket = runtime.store.tickets.get(id)
  if (!detail || !ticket) throw new RouteError(404, 'Ticket not found')
  return { ...detail, environment: runtime.configured() ? runtime.ticketEnvironment(ticket) : [] }
}
export type TicketPageView = ReturnType<typeof ticketPage>
function promptsView(runtime: HttpHost, revision: number | undefined, offset: number) {
  const current = runtime.prompts.current()
  const shown = revision === undefined || revision === current.revision ? current : runtime.prompts.version(revision)
  // Each built-in part with the key of its text, which an override records as its base, and for the overrides shown whose
  // built-in text changed since, the text they were edited from when known.
  const parts = Object.entries(builtInParts()).map(([key, part]) => ({ key, ...part, base: builtInBase(part.text) }))
  return { masterId: runtime.masterId, current: shown, defaults: defaultPrompts(), sections: TEAM_SECTIONS, skillName: SKILL_NAME.source,
    builtin: Object.keys(sourceSkills), skillPrefix: SKILL_PREFIX, parts, defaultChanged: runtime.prompts.defaultChanged(shown.config),
    ...runtime.sessions.status(), ...runtime.prompts.history(offset) }
}
export type PromptsView = ReturnType<typeof promptsView>

interface Input<K extends keyof Api> {
  params: RouteParams<K>; query: Record<string, string>; body: (maxBytes?: number) => Promise<unknown>
}
type Routes = { [K in keyof Api]: (input: Input<K>) => ApiResponse<K> | Promise<ApiResponse<K>> }
const offset = z.coerce.number().int().nonnegative().catch(0)
const limit = (fallback: number, max: number) => z.coerce.number().int().positive().catch(fallback).transform(value => Math.min(max, value))
const LOCAL_BODY_BYTES = 256 * 1024
/** Local requests may carry up to five base64 attachments, within this decoded total. */
const LOCAL_REQUEST_BYTES = 12 * 1024 * 1024
const LOCAL_FILES_BYTES = 8 * 1024 * 1024
const LocalAttachmentSchema = z.object({
  name: z.string().trim().min(1).max(120).refine(name => !/[\\/\u0000-\u001f\u007f]/.test(name) && name !== '.' && name !== '..',
    'A file name without a path or control characters'),
  data: z.string().min(1).regex(/^[A-Za-z0-9+/]+={0,2}$/, 'Base64 content'),
}).strict()
const LocalRequestSchema = z.object({ title: z.string().trim().min(1).max(120), text: z.string().trim().min(1).max(20_000),
  key: z.string().trim().min(8).max(200).optional(),
  project: z.string().trim().min(1).max(60).regex(/^[A-Za-z0-9][A-Za-z0-9_.-]*$/, 'Letters, digits, dot, dash or underscore').optional(),
  tags: z.array(z.string().trim().min(1).max(24)).max(8).optional(),
  files: z.array(LocalAttachmentSchema).max(5).optional(),
  hold: z.boolean().optional(),
}).strict()
const LocalFollowupSchema = z.object({ thread: z.string().min(1), text: z.string().trim().min(1).max(20_000) }).strict()
const TicketsQuery = z.object({ state: z.enum(TICKET_FILTERS).catch('open'), offset, limit: limit(20, 1000),
  tag: z.string().trim().max(24).optional() })
const ThreadsQuery = z.object({ filter: z.enum(THREAD_FILTERS).catch('subscribed'), offset, limit: limit(20, 1000) })
const ThreadQuery = z.object({ limit: limit(50, 100), before: z.string().optional(), around: z.string().optional() })
const DecisionsQuery = z.object({ kind: z.string().default(''), window: z.literal('24h').optional().catch(undefined), offset,
  limit: limit(30, 200) })
const count = z.coerce.number().int().nonnegative()
const PromptsQuery = z.object({ revision: count.optional(), offset: count.default(0) })
const SourceBlock = z.templateLiteral(['source:', z.string().regex(/^[a-z][a-z0-9-]*$/)])
const SettingsBlockSchema = z.union([z.enum(['policy', 'eval', 'executor']), SourceBlock])
const SaveRequest = z.object({ value: z.unknown(), revision: z.number().int().nonnegative() })

/**
 * Refuses local ticket requests while setup is pending, the Source is off, or nobody may create tickets.
 * The first call saves the Source's default Policy part, so an unconfigured installation is not filtered for want of an operator.
 */
async function checkLocalSource(runtime: HttpHost): Promise<void> {
  if (!runtime.configured()) throw new RouteError(400, 'dsh-superagent is waiting for the setup check')
  if (!runtime.sources.instance('local')?.enabled) throw new RouteError(400, 'The local Source is off; turn it on in Settings → Sources')
  await seedPolicy(runtime)
  if (!runtime.store.settings().policy.sources.local?.allowed.some(entry => entry.id === OPERATOR)) {
    throw new RouteError(400, 'The local Source allows nobody; add the operator under Policy → On each Source')
  }
}
function routes(runtime: HttpHost): Routes {
  const { store } = runtime
  const ticket = (id: string) => { if (!store.tickets.get(id)) throw new RouteError(404, 'Ticket not found'); return id }
  const promptBody = (input: { body: (maxBytes?: number) => Promise<unknown> }) => input.body(4 * runtime.machine.promptMaxChars.get() + 4096)
  return {
    'GET state': () => stateView(runtime),
    'GET overview': () => overview(views(runtime), runtime.health, policyVersion(store.settings()),
      executorLoad(store, runtime.executor.activeCount()), runtime.setupProblem ?? runtime.feishuProblem),
    'GET notifications': () => runtime.notifications(),
    'POST notifications/:id/read': ({ params }) => {
      if (!runtime.readNotification(params.id)) throw new RouteError(404, 'Notification not found')
      return { read: true }
    },
    'POST notifications/read-all': () => ({ read: runtime.readAllNotifications() }),
    'GET search': ({ query }) => search(views(runtime), query.q ?? '', runtime.sourceLinks,
      Math.max(0, Math.trunc(Number(query.offset)) || 0)),
    'GET sources': () => sourcesStatus(views(runtime), runtime.health, setupState(runtime)),
    'GET settings': () => settingsView(store, runtime.sources, runtime.health, setupState(runtime), runtime.machine.stateDir),
    'POST sources/:id/labels': ({ params }) => runtime.settings.refreshLabels(params.id),
    'GET threads': ({ query }) => {
      const { filter, offset, limit } = ThreadsQuery.parse(query)
      return threadList(views(runtime), filter, offset, limit)
    },
    'GET threads/:key': ({ params, query }) => {
      const { limit, before, around } = ThreadQuery.parse(query)
      const detail = threadDetail(views(runtime), params.key, before, limit, around)
      if (!detail) throw new RouteError(404, 'Thread not found')
      return detail
    },
    'POST threads/:key/unsubscribe': async ({ params }) => {
      const removed = runtime.engine.unsubscribe(params.key)
      return { removed, thread: params.key, context_delivery: await runtime.sessions.delivery(`subscription:${params.key}`) }
    },
    'POST triggers/:id/requeue': ({ params }) => {
      const queued = runtime.engine.reevaluate(params.id)
      runtime.requestPass()
      return { queued }
    },
    'GET decisions': ({ query }) => {
      const { kind, window, offset, limit } = DecisionsQuery.parse(query)
      const since = window ? new Date(Date.now() - 86_400_000).toISOString() : ''
      return decisionList(views(runtime), kind.split(',').filter(Boolean), offset, limit, since)
    },
    'POST decisions/:id/dismiss': ({ params }) => { runtime.engine.dismiss(params.id); return { dismissed: true } },
    'GET tickets': ({ query }) => {
      const { state, offset, limit, tag } = TicketsQuery.parse(query)
      return ticketList(views(runtime), state, offset, limit, tag)
    },
    'GET tags': () => ({ tags: ticketTags(store) }),
    'GET tickets/:id': ({ params }) => ticketPage(runtime, params.id),
    'POST tickets/:id/retry': ({ params }) => { const id = ticket(params.id); runtime.engine.retryTicket(id); return { retried: true } },
    'POST tickets/:id/stop': ({ params }) => { const id = ticket(params.id); runtime.engine.stopTicket(id); return { stopped: true } },
    'POST tickets/:id/reopen': ({ params }) => { const id = ticket(params.id); runtime.engine.reopenTicket(id); return { reopened: true } },
    'POST tickets/:id/approve': ({ params }) => { const id = ticket(params.id); runtime.approveTicket(id); return { approved: true } },
    'GET tickets/:id/diff': async ({ params, query }) => {
      const id = ticket(params.id)
      const since = z.string().regex(/^[0-9a-f]{7,40}$/i, 'A commit id').optional().parse(query.since)
      return runtime.ticketDiff(id, since)
    },
    'POST tickets/:id/merge': async ({ params, query }) => {
      const id = ticket(params.id)
      const strategy = z.enum(['ff', 'squash', 'merge']).default('ff').parse(query.strategy)
      try { return { merged: true as const, ...await runtime.mergeTicket(id, strategy) } }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/title': async ({ params, body }) => {
      const id = ticket(params.id)
      const { title } = z.object({ title: z.string().trim().min(1).max(120) }).strict().parse(await body())
      return { title: runtime.renameTicket(id, title).title }
    },
    'POST tickets/:id/review': async ({ params, body }) => {
      const id = ticket(params.id)
      const { decision, note } = z.object({ decision: z.enum(['approved', 'changes-requested']),
        note: z.string().trim().max(2000).optional() }).strict().parse(await body())
      try {
        const reviewed = runtime.reviewTicket(id, decision, note).reviewed
        return { reviewed: true as const, decision, at: reviewed?.at ?? '' }
      } catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/request': async ({ params, body }) => {
      const id = ticket(params.id)
      const { text } = z.object({ text: z.string().trim().min(1).max(20_000) }).strict().parse(await body())
      try {
        const ticket = runtime.editRequest(id, text)
        return { request: ticket.request ?? { text, at: '' } }
      } catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/tags': async ({ params, body }) => {
      const id = ticket(params.id)
      const { tags } = z.object({ tags: z.array(z.string().trim().min(1).max(24)).max(8) }).strict().parse(await body())
      try { return { tags: runtime.setTags(id, tags).tags ?? [] } }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/revert': async ({ params }) => {
      const id = ticket(params.id)
      try { return await runtime.revertTicket(id) }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/push': async ({ params }) => {
      const id = ticket(params.id)
      try { return await runtime.pushTicket(id) }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/dequeue': ({ params }) => {
      const id = ticket(params.id)
      return runtime.dequeueMerges([id])
    },
    'GET merge-queue': () => runtime.mergeQueue(),
    'POST merge-queue/retry': async ({ body }) => {
      const { ticket_ids: ids } = z.object({ ticket_ids: z.array(z.string().trim().min(1).max(200)).min(1).max(20) }).strict()
        .parse(await body())
      return runtime.retryMerges(ids)
    },
    'POST merge-queue/pause': async ({ body }) => {
      const { paused } = z.object({ paused: z.boolean() }).strict().parse(await body())
      return { paused: runtime.pauseMergeQueue(paused) }
    },
    'POST tickets/:id/discard': async ({ params }) => {
      const id = ticket(params.id)
      try { await runtime.discardTicket(id); return { discarded: true as const } }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/rebase': async ({ params }) => {
      const id = ticket(params.id)
      try { return { rebased: true as const, ...await runtime.rebaseTicket(id) } }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/notes': async ({ params, body }) => {
      const id = ticket(params.id)
      const note = z.object({ file: z.string().trim().min(1).max(500), line: z.number().int().min(1).max(1_000_000).optional(),
        endLine: z.number().int().min(1).max(1_000_000).optional(), text: z.string().trim().min(1).max(2_000) }).strict()
        .parse(await body())
      try { return { notes: runtime.addNote(id, note) } }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/notes/:noteId/remove': ({ params }) => {
      const id = ticket(params.id)
      try { return { notes: runtime.removeNote(id, params.noteId) } }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/notes/:noteId/replies': async ({ params, body }) => {
      const id = ticket(params.id)
      const { text } = z.object({ text: z.string().trim().min(1).max(2000) }).strict().parse(await body())
      try { return { notes: runtime.replyToNote(id, params.noteId, text) } }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/notes/:noteId/resolve': async ({ params, body }) => {
      const id = ticket(params.id)
      const { resolved } = z.object({ resolved: z.boolean() }).strict().parse(await body())
      try { return { notes: runtime.resolveNote(id, params.noteId, resolved) } }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/automerge': async ({ params, body }) => {
      const id = ticket(params.id)
      const { cancel } = z.object({ cancel: z.boolean() }).strict().parse(await body())
      try {
        const ticket = cancel ? runtime.cancelAutoMerge(id) : runtime.store.tickets.get(id)
        return { auto_merge: !!ticket?.autoMerge }
      } catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/blockers': async ({ params, body }) => {
      const id = ticket(params.id)
      const { ticket: reference, blocked } = z.object({ ticket: z.string().trim().min(1).max(200), blocked: z.boolean() }).strict()
        .parse(await body())
      try { return runtime.blockTicket(id, reference, blocked) }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/related': async ({ params, body }) => {
      const id = ticket(params.id)
      const { ticket: reference, linked } = z.object({ ticket: z.string().trim().min(1).max(200), linked: z.boolean() }).strict()
        .parse(await body())
      try { return runtime.linkTicket(id, reference, linked) }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/check': async ({ params }) => {
      const id = ticket(params.id)
      try { return { checks: await runtime.checkTicket(id) } }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST tickets/:id/cleanup': async ({ params }) => {
      const id = ticket(params.id)
      try { await runtime.cleanupTicket(id); return { cleaned: true as const } }
      catch (error) { throw new RouteError(400, error instanceof Error ? error.message : String(error)) }
    },
    'POST local/requests': async input => {
      const request = LocalRequestSchema.parse(await input.body(LOCAL_REQUEST_BYTES))
      const bytes = (request.files ?? []).reduce((total, file) => total + Math.floor(file.data.length * 3 / 4), 0)
      if (bytes > LOCAL_FILES_BYTES) throw new RouteError(400, 'Attachments exceed 8 MiB')
      await checkLocalSource(runtime)
      return runtime.localRequest(request)
    },
    'POST local/followups': async input => {
      const followup = LocalFollowupSchema.parse(await input.body(LOCAL_BODY_BYTES))
      await checkLocalSource(runtime)
      // A caller names the Thread, so an unknown one is bad input rather than a server fault.
      if (runtime.store.threads.get(followup.thread)?.ref.source !== 'local') {
        throw new RouteError(400, 'The local ticket Thread is unknown')
      }
      return runtime.localFollowup(followup)
    },
    'POST executor/pause': async ({ body }) => runtime.settings.pause(z.object({ paused: z.boolean() }).parse(await body()).paused, 'panel'),
    'PUT settings/:block': async ({ params, body }) => {
      const block = SettingsBlockSchema.parse(params.block)
      const known = !block.startsWith('source:') || runtime.sources.instances().some(instance => `source:${instance.id}` === block)
      if (!known) throw new RouteError(404, 'Source not found')
      const request = SaveRequest.parse(await body())
      return runtime.settings.save(block, request.value, request.revision, 'panel')
    },
    'GET policy/history': () => policyHistory(store, runtime.sources, policyVersion(store.settings())),
    'POST policy-sync/:id': async ({ params }) => ({ policy: await runtime.settings.sync(params.id) }),
    'POST setup/check': () => { runtime.checkSetup(); return { restarting: true } },
    'GET models': () => runtime.models(),
    'GET prompts': ({ query }) => { const { revision, offset } = PromptsQuery.parse(query); return promptsView(runtime, revision, offset) },
    'PUT prompts': async input => {
      const request = z.object({ config: z.unknown(), baseRevision: z.number().int().nonnegative() }).parse(await promptBody(input))
      return savePrompts(runtime, request.config, request.baseRevision, 'panel')
    },
    'POST prompts/preview': async input => runtime.prompts.preview(await promptBody(input)),
  }
}
function decoded(segment: string): string {
  try { return decodeURIComponent(segment) } catch { throw new RouteError(400, 'A path segment has a malformed escape') }
}
function match(table: Routes, method: string, segments: string[]) {
  for (const [route, run] of Object.entries(table)) {
    const [verb, path] = route.split(' ') as [string, string]
    const pattern = path.split('/')
    const fits = pattern.length === segments.length && pattern.every((part, index) => part.startsWith(':') || part === segments[index])
    if (verb !== method || !fits) continue
    const params: Record<string, string> = {}
    pattern.forEach((part, index) => { if (part.startsWith(':')) params[part.slice(1)] = decoded(segments[index]!) })
    return { run: run as (input: Input<keyof Api>) => unknown, params }
  }
  return undefined
}
export function registerHttp(ctx: Context, runtime: HttpHost): () => void {
  // The panel is served from the bound host, so the management API follows it; DSH's Web authentication and the
  // same-origin header checks below protect it wherever the operator chose to bind.
  const table = routes(runtime)
  const disposers = [
    ctx.webServer.register({ kind: 'prefix', path: '/orchestrator', handler: async (req, res) => {
      const url = new URL(req.url ?? '/', 'http://localhost')
      if (!url.pathname.startsWith('/orchestrator/api/')) { res.writeHead(404); res.end(); return }
      if (!sameOrigin(req, ctx.webServer.host, ctx.webServer.port ?? -1)) { json(res, 403, { error: 'Same-origin management request required' }); return }
      try {
        await runtime.ready
        const route = match(table, req.method ?? 'GET', url.pathname.slice('/orchestrator/api/'.length).split('/'))
        if (!route) throw new RouteError(404, 'Unknown operation')
        const query = Object.fromEntries(url.searchParams)
        json(res, 200, await route.run({ params: route.params as never, query, body: maxBytes => readBody(req, maxBytes) }))
      } catch (error) { json(res, ...failure(error)) }
    } })]
  return () => { for (const dispose of disposers) dispose() }
}

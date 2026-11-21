/** Application composition root and lifecycle manager for dsh-superagent. */
import { Service, type Context } from '@deepseek-ai/cordis'
import type { Agent } from '@deepseek-ai/dsh-agent'
import type { SessionId } from '@deepseek-ai/dsh-session'
import type {} from '@deepseek-ai/dsh-api-session-controller'
import type {} from '@deepseek-ai/dsh-session-persistence'
import type {} from '@deepseek-ai/dsh-workspace'
import type {} from '@deepseek-ai/dsh-permission-presets'
import type {} from '@deepseek-ai/dsh-system-prompt'
import type {} from '@deepseek-ai/dsh-tools'
import type {} from '@deepseek-ai/dsh-skill'
import type {} from '@deepseek-ai/dsh-subprocess'
import { mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { lockSync } from 'proper-lockfile'
import { Engine, key, latestAssignment, latestReport, liveTicket, reasonText, type ActionOrigin, type DomainEvent, type EngineForSettings, type ReceiveResult, type Ticket,
  type TicketCheck, type TicketCheckRun, type TicketNote, type TicketReview, type Trigger } from '../core/index.ts'
import { allowedSender, policyVersion, settingsPolicy } from '../policy.ts'
import { SourceRegistry } from '../sources/registry.ts'
import { Commands } from '../infra/commands.ts'
import type { Deployment, Machine } from '../deployment.ts'
import { environmentOf, ticketEnvironment, type Variable } from './execution-environment.ts'
import type { SetupProblem } from '../setup.ts'
import { DshExecutor, sourceTrigger, triggerMessage, workerSessionId } from '../executor/dsh.ts'
import { ticketCard } from './cards.ts'
import { notifications as listNotifications, readAllNotifications, readNotification, recordNotification } from './notifications.ts'
import { dequeueMerges, enqueueMerges, markMergeFailed, mergeQueuePaused, mergeQueueView, nextQueuedMerge, queuedMerges,
  recoverableMerges, retryMerges, setMergeQueuePaused, type MergeFailureCode, type MergeQueueView, type QueuedMerge } from './merge-queue.ts'
import { MasterAssessor } from './assessment.ts'
import { KV } from '../kv.ts'
import { Store } from './store.ts'
import { evalsHealth } from './views/overview.ts'
import { TextCache } from './cache.ts'
import { Effects } from './effects.ts'
import { Prompts } from './prompts.ts'
import { registerHttp, type HttpHost } from './http.ts'
import { newestFirst, type LinkTarget } from './views/records.ts'
import { candidateCard, repliedTo, sourceLink, type LinkHost } from './links.ts'
import { mergeBlockedByCheck, parseChecks, workspaceChecks } from './check.ts'
import { captureReview, discardWorkspace, mergeWorkspace, patchFingerprint, pushWorkspace, RebaseConflictError, rebaseWorkspace, remapNoteAnchors, RevertConflictError, revertWorkspace, targetContained, workingBase, workspaceDiff, workspaceHead, workspaceRepo, type DiffFile, type MergeStrategy, type NoteAnchor } from './review.ts'
import { sendWebhook, type WebhookPayload } from './webhook.ts'
import { eventNotice, followupNotice, receiptNotice } from './notices.ts'
import { archiveNeedsClose, closeArchive, createLocalFollowup, createLocalRequest, OPERATOR, recordApproval, recordCleanup, recordDiscard,
  recordHandback, recordMerge, recordPush, recordRebase, recordRename, recordRequestEdit, recordReview, recordRevert, recordTags, reopenArchive, seedPolicy, ticketArchive, webhookFor,
  type LocalRequest, type LocalRequestResult } from '../sources/local.ts'
import { HealthBoard } from '../infra/health.ts'
import { Outbox } from './outbox.ts'
import { Passes } from './passes.ts'
import { Setup } from './setup.ts'
import { feishuSettings } from '../sources/feishu.ts'
import { health, type Health, type SourceContext, type SourceHost } from '../sources/host.ts'
import { Sessions } from './sessions.ts'
import { Settings } from './settings.ts'
import { sourceContextOf, type SourceContextDeps } from './source-context.ts'
import type { ResourceHost } from './resources.ts'
import { mainTools, type MainToolsHost } from './tools/main.ts'
import { executorTools, type ExecutorToolsHost } from './tools/executor.ts'

// Source plugins interact with dsh-superagent exclusively through the `SourceHost` service interface.
declare module '@deepseek-ai/cordis' { interface Context { orchestrator: SourceHost } }
const now = (): string => new Date().toISOString()
/** Attempts a standing merge request gets before the host gives up and tells the person which gate stayed closed. */
const AUTO_MERGE_ATTEMPTS = 8
/** How long an automatic attempt trusts a check that already failed for the same ending, instead of running it again. */
const CHECK_REUSE_MS = 10 * 60_000
/** What keeps an ended Ticket from merging, as a standing request and the chat's status report read it. */
type MergeGate = { code: 'merge' } | { code: 'dependency' | 'review' | 'notes'; message: string; counts: boolean }
/** The merge or revert a push acted on, so an outcome written after another action cannot land on the wrong record. */
type PushTarget = { merge: string } | { revert: string }

/**
 * Composition root of dsh-superagent that connects `Store`, `Setup`, `Engine`, `SourceRegistry`, `Effects`, `Outbox`, `Passes`, `Sessions`, `Settings`, `Prompts`, `DshExecutor`, and `MasterAssessor`.
 * Exposes the `SourceHost` service on the Cordis context for source plugins and coordinates startup, setup checks, HTTP route registration, and graceful shutdown.
 * Locks the state directory before opening SQLite so concurrent processes cannot corrupt or double-migrate state.
 */
export class Runtime extends Service implements SourceHost {
  readonly store: Store
  readonly setup: Setup
  readonly prompts: Prompts
  readonly effects: Effects
  readonly sources: SourceRegistry
  readonly commands: Commands
  readonly sessions: Sessions
  readonly settings: Settings
  readonly executor: DshExecutor
  readonly assessor: MasterAssessor
  readonly outbox: Outbox
  readonly engine: Engine
  readonly passes: Passes
  readonly ready: Promise<void>
  private readonly board = new HealthBoard()
  private readonly runtimeHealth = this.board.claim('runtime', { state: 'starting', detail: '' })
  readonly masterId: string
  private readonly sourceLinks = new TextCache(200_000)
  private readonly stopped = new AbortController()
  private readonly disposers: (() => void)[] = []
  private releaseLock?: () => Promise<void>
  private closingFlag = false
  private stopping?: Promise<void>
  private readonly sourceDeps: SourceContextDeps
  private readonly toolHost: MainToolsHost & ExecutorToolsHost
  /** Non-zero while a main-Session tool is executing an Engine command, suppressing self-triggered domain event notices. */
  private quiet = 0
  /** Tickets whose merge is running, so a standing request and a person's cannot move one branch at the same time. */
  private readonly merging = new Set<string>()
  /** Tickets whose revert is running, so a second request cannot revert the same merge twice. */
  private readonly reverting = new Set<string>()
  /** Ending captures still running, so a discard does not remove the worktree a capture reads. */
  private readonly finishing = new Map<string, Promise<void>>()
  /** Operations that move a repository run one at a time, so two merges never share the git index of one checkout. */
  private repoLock: Promise<unknown> = Promise.resolve()
  /** The auto-merge attempt in flight, so passes do not start a second one over the same repository. */
  private autoMerging?: Promise<void>
  /** The merge-queue item in flight, so passes do not start a second one over the same repository. */
  private queueMerging?: Promise<void>
  constructor(ctx: Context, readonly machine: Machine, restart: () => void) {
    super(ctx, 'orchestrator')
    const runtime = this
    // Lock the state directory before opening the database so another process cannot migrate or write concurrently.
    mkdirSync(machine.stateDir, { recursive: true, mode: 0o700 })
    const compromised = (error: Error) => { this.runtimeHealth.set({ state: 'error', detail: error.message }); this.stopped.abort() }
    const release = lockSync(machine.stateDir, { stale: 30_000, onCompromised: compromised })
    this.releaseLock = async () => { release() }
    let opened: Store | undefined
    try {
      const report = (error: unknown) => { ctx.logger.warn('dsh-superagent: %s', String(error)) }
      const store = opened = this.store = new Store(join(machine.stateDir, 'orchestrator.sqlite'), report)
      const setup = this.setup = new Setup({ context: ctx, machine, lifetime: this.lifetime, restart })
      const deployed = () => setup.configured()
        ? environmentOf({ deployment: setup.deployment, sources: this.sources, settings: store.settings() }) : undefined
      const feishu = () => setup.configured() && !!setup.deployment.feishu
      const prompts = this.prompts = new Prompts(store, () => machine.promptMaxChars.get(), deployed, feishu)
      const masterId = this.masterId = store.get(KV.runtime.masterId) ?? `orchestrator-${key(machine.stateDir)}`
      store.set(KV.runtime.masterId, masterId)
      const commands = this.commands = new Commands(() => setup.deployment, setup.commandEnvironment, this.lifetime)
      const board = this.board
      const effects = this.effects = new Effects(store, board)
      const sources = this.sources = new SourceRegistry(() => store.settings(), board, kind => {
        // Sync source member lists once startup completes when a source plugin loads dynamically.
        if (kind.policy?.sync) void this.ready?.then(() => this.settings.syncable().includes(kind.kind) ? this.settings.sync(kind.kind) : undefined)
          .catch(() => { /* sync() records its failure in health. */ })
        return () => {}
      })
      const links: LinkHost = { store, links: this.sourceLinks, commands, sources, lifetime: this.lifetime }
      const link = (trigger: Trigger, signal?: AbortSignal) => sourceLink(links, trigger, signal)
      const environment = (ticket: Ticket) => this.ticketEnvironment(ticket)
      const resources: ResourceHost = { store, pages: new TextCache(4_000_000), prompts, commands, masterId, context: ctx,
        promptVersion: id => this.sessions.promptVersion(id), mayRead: (id, revision) => this.sessions.mayRead(id, revision),
        feishuPageSize: () => feishuSettings(sources).readPageSize, sourceLink: link, ticketDiff: id => this.ticketDiff(id) }
      const sessions = this.sessions = new Sessions({ context: ctx, store, prompts, effects, health: board.claim('models'), masterId,
        lifetime: this.lifetime,
        closing: () => this.closing(), environment,
        tools: (agent, ticket) => ticket ? executorTools(this.toolHost, ticket) : mainTools(this.toolHost, agent) })
      const policyChanged: EngineForSettings['policyChanged'] = (from, to, effects) => { this.engine.policyChanged(from, to, effects) }
      this.settings = new Settings({ store, sources, engine: { policyChanged }, health: board, context: ctx, lifetime: this.lifetime, sessions,
        sourceHost: this, setup, requestPass: () => { this.passes.request() } })
      this.executor = new DshExecutor({ context: ctx, get engine() { return runtime.engine }, store, effects, health: board.claim('worker'), machine,
        lifetime: this.lifetime, sessions, closing: () => this.closing(), paused: () => this.paused(), environment, sourceLink: link,
        repliedTo: trigger => repliedTo({ store, sources }, trigger) })
      this.assessor = new MasterAssessor({ get engine() { return runtime.engine }, commands, get deployment() { return setup.deployment }, effects,
        settings: () => store.settings(), paused: () => this.paused(), threadContext: trigger => sources.threadContext(trigger),
        deliver: (id, text) => sessions.prompt(masterId, id, text, 'steer'), candidateCard: candidate => candidateCard(links, candidate) })
      const outbox = this.outbox = new Outbox({ store, effects, sources, commands, paused: () => this.paused() })
      const { assessor, executor } = this
      const policy = () => settingsPolicy(store.settings(), sources)
      const engine = this.engine = new Engine({ repo: store, scope: sources, policy, assessor, executor,
        outbox, prepareTicket: (ticket, outcome) => executor.prepare(ticket, outcome), emit: event => {
          // Notify the main Session only about domain events it did not initiate itself.
          const notice = this.quiet ? undefined : eventNotice(event, id => store.tickets.get(id))
          if (notice) sessions.notify(...notice)
          // A person's message that reached an open ticket is the ticket's; the main Session hears of it in the same transaction,
          // and of one for an ended ticket once it resumes, unless it routed the message there itself.
          const followed = !this.quiet && (event.type === 'trigger.received' || event.type === 'ticket.resumed' || event.type === 'trigger.followup')
          const followup = followed ? followupNotice(store, sources, event.data.triggerId) : undefined
          if (followup) sessions.relay(followup.id, followup.text)
          // Side effects outside the database wait for the transaction to commit, so a rolled-back question or
          // ending sends no webhook and writes no archive section.
          if (event.type === 'ticket.attention' || event.type === 'ticket.ended') {
            const notified = store.tickets.get(event.data.ticketId)
            const kind = event.type === 'ticket.attention' ? 'attention' as const : 'ended' as const
            const text = event.type === 'ticket.attention' ? event.data.question
              : [event.data.outcome, event.data.reason].filter(Boolean).join(': ')
            this.store.afterCommit(() => {
              // An inbox write must not roll back the Ticket change it reports, so it fails alone.
              try { if (notified) recordNotification(store, { at: event.at, kind, ticketId: notified.id, title: notified.title, text }) }
              catch (error) { ctx.logger.warn('dsh-superagent: recording a notification failed: %s', String(error)) }
              this.notifyWebhook(event)
            })
          }
          // Record the ending in the archive and the workspace review, and a reopening the operator made afterwards.
          const ended = event.type === 'ticket.ended' ? store.tickets.get(event.data.ticketId) : undefined
          if (ended) this.store.afterCommit(() => {
            const task = this.finishTicket(ended, ctx).catch((error: unknown) => {
              ctx.logger.warn('dsh-superagent: completing the ending of ticket %s failed: %s', ended.id, String(error))
            })
            this.finishing.set(ended.id, task)
            void task.finally(() => { if (this.finishing.get(ended.id) === task) this.finishing.delete(ended.id) })
          })
          if (event.type === 'ticket.reopened') {
            const reopened = store.tickets.get(event.data.ticketId)
            if (reopened) this.store.afterCommit(() => {
              void reopenArchive(reopened, event.at).catch((error: unknown) => {
                ctx.logger.warn('dsh-superagent: recording a reopening in the archive failed: %s', String(error))
              })
            })
          }
        } })
      // Update the `evals` health entry on every pass if an evaluation failed as unavailable within the last hour.
      const evals = board.claim('evals', { state: 'ready', detail: 'No unavailable Eval in the last hour' })
      const evaluateDue = () => { engine.evaluateDue(); evals.set(evalsHealth(store)) }
      const passes = this.passes = new Passes({ engine: { receive: trigger => engine.receive(trigger), evaluateDue }, effects,
        health: this.runtimeHealth, lifetime: this.lifetime, configured: () => setup.configured(), closing: () => this.closing(),
        automerge: () => { void this.runAutoMerge() }, mergeQueue: () => { void this.runMergeQueue() },
        })
      this.sourceDeps = { store, engine, outbox, commands, sources, get ready() { return runtime.ready },
        get deployment() { return setup.deployment }, configured: () => setup.configured(),
        get setupProblem() { return setup.problem }, get feishuProblem() { return setup.feishuProblem }, receive: trigger => passes.receive(trigger) }
      this.toolHost = { context: ctx, engine, store, prompts, sessions, settings: this.settings, assessor, resources,
        localRequest: input => this.localRequest(input),
        localFollowup: (input, options) => createLocalFollowup(runtime, input, options),
        handbackTicket: (id, input, origin) => runtime.handbackTicket(id, input, origin),
        mergeTicket: (id, strategy, options) => runtime.mergeTicket(id, strategy, options, 'chat'),
        autoMergeTicket: (id, options) => runtime.autoMergeTicket(id, options),
        cancelAutoMerge: id => runtime.cancelAutoMerge(id),
        revertTicket: (id, options) => runtime.revertTicket(id, 'chat', options),
        queueMerges: (ids, options) => runtime.queueMerges(ids, options),
        dequeueMerges: ids => runtime.dequeueMerges(ids), retryMerges: ids => runtime.retryMerges(ids),
        pauseMergeQueue: paused => runtime.pauseMergeQueue(paused),
        pendingMerges: () => runtime.pendingMerges(),
        ticketId: reference => runtime.ticketId(reference),
        stopTicket: (id, options) => runtime.stopTicket(id, options),
        pushTicket: id => runtime.pushTicket(id, 'chat'),
        cleanupTicket: id => runtime.cleanupTicket(id, 'chat'),
        reviewTicket: (id, decision, note) => runtime.reviewTicket(id, decision, note, 'chat'),
        resolveNotes: (id, resolved) => runtime.resolveAllNotes(id, resolved),
        checkTicket: id => runtime.checkTicket(id),
        blockTicket: (id, reference, blocked, start) => runtime.blockTicket(id, reference, blocked, start),
        requestPass: () => { passes.request() }, sendReplies: () => outbox.sent(),
        quietly: run => { this.quiet++; try { return run() } finally { this.quiet-- } } }
      this.ready = this.initialize()
    } catch (error) {
      // Release the database and state lock if initialization fails so an in-process restart can reacquire them.
      try { opened?.close() } finally { release() }
      throw error
    }
  }
  get health(): Readonly<Record<string, Health>> { return this.board.all }
  /** Aborted when dsh-superagent stops or loses its state directory lock, cancelling running reads and subprocesses. */
  get lifetime(): AbortSignal { return this.stopped.signal }
  context(kind: { kind: string; health: readonly string[] }): SourceContext { return sourceContextOf(this.sourceDeps, kind) }
  /** Resolved deployment; throws while waiting for setup (`!configured()`). */
  get deployment(): Deployment { return this.setup.deployment }
  configured(): boolean { return this.setup.configured() }
  get setupProblem(): SetupProblem | undefined { return this.setup.problem }
  /** Why the Feishu bot is unavailable even though the Feishu Source configures a CLI. */
  get feishuProblem(): SetupProblem | undefined { return this.setup.feishuProblem }
  links(): LinkTarget {
    const resolved = this.setup.configured() ? this.setup.deployment : undefined
    const links = resolved?.feishu ? { feishu: resolved.feishu } : {}
    return channel => this.sources.link(channel, links)
  }
  ticketEnvironment(ticket: Ticket): Variable[] {
    const trigger = sourceTrigger(this, ticket)
    const { setup: { deployment }, sources } = this
    return ticketEnvironment({ deployment, sources, settings: this.store.settings(), ticket, ...trigger ? { trigger } : {} })
  }
  /**
   * Reads a ticket workspace's committed diff since its recorded base, on demand from the panel.
   * @returns The diff, whether it was truncated, and why git could not provide it, if it could not.
   */
  /**
   * The committed diff of a ticket's changes.
   * @param id - The ticket, by id or display number.
   * @param since - A commit to diff from instead of the ticket's base, such as the one a review covered, so the panel
   * shows only what changed after that review.
   * @returns The diff, whether it was truncated, one entry per file, and why it could not be read.
   */
  async ticketDiff(id: string, since?: string):
    Promise<{ diff: string; truncated: boolean; files: DiffFile[]; base?: string; error?: string }> {
    const ticket = this.store.tickets.get(id)
    const directory = ticket?.execution?.directory
    const base = since ?? ticket?.execution?.base
    if (!directory) return { diff: '', truncated: false, files: [], error: 'This ticket has no workspace' }
    if (!base) return { diff: '', truncated: false, files: [], error: 'No base commit was recorded for this ticket' }
    try { return { ...await workspaceDiff(this.ctx.subprocess, directory, base, this.stopped.signal), base } }
    catch (error) { return { diff: '', truncated: false, files: [], base, error: String(error) } }
  }
  /**
   * POSTs a local ticket's question or ending to the local Source's webhook, when one is configured. Best effort: a
   * failure is logged and the ticket is unaffected.
   */
  private notifyWebhook(event: Extract<DomainEvent, { type: 'ticket.attention' } | { type: 'ticket.ended' }>): void {
    const ticket = this.store.tickets.get(event.data.ticketId)
    if (!ticket) return
    const base = { at: event.at, ticket: { id: ticket.id, number: this.store.tickets.number(ticket.id) ?? 0, title: ticket.title } }
    this.postWebhook(ticket, event.type === 'ticket.ended'
      ? { event: 'ended', ...base, outcome: event.data.outcome, reason: event.data.reason ?? event.data.outcome }
      : { event: 'attention', ...base, question: event.data.question })
  }
  /** POSTs one event about a local ticket, the Source with a webhook configured; a failure is logged and changes nothing. */
  private postWebhook(ticket: Ticket, payload: WebhookPayload): void {
    if (!ticket.channels.some(channel => channel.source === 'local')) return
    const url = webhookFor(this.store.settings())
    if (!url) return
    void sendWebhook(url, payload, this.stopped.signal).catch((error: unknown) => {
      this.ctx.logger.warn('dsh-superagent: webhook notification failed: %s', String(error))
    })
  }
  /** The label a person reads for a ticket: its display number, such as `T-0007`, or its id when it has none yet. */
  private ticketLabel(ticket: Pick<Ticket, 'id'>): string {
    const number = this.store.tickets.number(ticket.id)
    return number ? `T-${String(number).padStart(4, '0')}` : ticket.id
  }
  /**
   * Records a panel notification for an event a person may need to act on, and posts it to a local Source's webhook.
   * @param kind - The event a person should see, by the same name in both interfaces.
   * @param text - One sentence stating what happened.
   */
  private ticketNotice(ticket: Ticket, kind: 'merged' | 'reverted' | 'review' | 'automerge' | 'check' | 'push', text: string): void {
    const at = now()
    try { recordNotification(this.store, { at, kind, ticketId: ticket.id, title: ticket.title, text }) }
    catch (error) { this.ctx.logger.warn('dsh-superagent: recording a notification failed: %s', String(error)) }
    this.postWebhook(ticket, { event: kind, at, text,
      ticket: { id: ticket.id, number: this.store.tickets.number(ticket.id) ?? 0, title: ticket.title } })
  }
  /**
   * Approves a held ticket, letting its pending Assignment launch, and records the approval in its local archive.
   * @throws UserError when the ticket is unknown, ended, not held, or has no pending Assignment.
   */
  approveTicket(id: string): Ticket {
    const ticket = this.engine.approveTicket(id)
    const at = ticket.approved?.at ?? now()
    void recordApproval(ticket, at).catch((error: unknown) => {
      this.ctx.logger.warn('dsh-superagent: recording an approval in the archive failed: %s', String(error))
    })
    return ticket
  }
  /**
   * Renames an ended or open ticket, updates the label of its local origin Thread, and appends the new title to its local
   * archive without rewriting the original request.
   * @throws UserError when the ticket is unknown or the title is blank.
   */
  renameTicket(id: string, title: string): Ticket {
    const ticket = this.engine.renameTicket(id, title)
    const ref = this.store.triggers.get(ticket.origin.triggerId)?.threads[0]
    if (ref?.source === 'local') this.store.threads.describe(`${ref.source}:${ref.id}`, { label: ticket.title })
    const at = ticket.renamed?.at ?? now()
    void recordRename(ticket, at, ticket.title).catch((error: unknown) => {
      this.ctx.logger.warn('dsh-superagent: recording a rename in the archive failed: %s', String(error))
    })
    return ticket
  }
  /**
   * Replaces a ticket's labels and appends them to its local archive without rewriting the request's front matter.
   * @returns The ticket with its labels.
   * @throws UserError when the ticket is unknown, or the labels are too many or too long.
   */
  setTags(id: string, tags: string[]): Ticket {
    const ticket = this.engine.setTags(id, tags)
    if (ticket.tagged) void recordTags(ticket, ticket.tagged.at, ticket.tags ?? []).catch((error: unknown) => {
      this.ctx.logger.warn('dsh-superagent: recording labels in the archive failed: %s', String(error))
    })
    return ticket
  }
  /**
   * Creates a local ticket request from the main Session's conversation, seeding the local Source's Policy part first so an
   * installation that never opened the panel still admits the operator.
   * @returns The request, Thread, Trigger and archive file the local Source recorded.
   * @throws when setup is pending, the local Source is off, or its Policy allows nobody.
   */
  private async localRequest(input: LocalRequest): Promise<LocalRequestResult> {
    if (!this.configured()) throw new Error('dsh-superagent is waiting for the setup check')
    if (!this.sources.instance('local')?.enabled) throw new Error('The local Source is off; turn it on in Settings → Sources')
    // The tool is the caller, so its Policy seed must not notify the main Session of its own change.
    await seedPolicy(this, 'tool')
    if (!this.store.settings().policy.sources.local?.allowed.some(entry => entry.id === OPERATOR)) {
      throw new Error('The local Source allows nobody; add the operator under Policy → On each Source')
    }
    return createLocalRequest(this, input)
  }
  /**
   * Completes archive records a crash may have left half-written: the close section of ended local tickets, and the merge or
   * discard section of tickets that already recorded one. Runs once after startup; each write is idempotent.
   * @returns The number of close sections added.
   */
  async reconcileArchives(): Promise<number> {
    let repaired = 0
    for (const ticket of this.store.tickets.list()) {
      if (!ticketArchive(ticket)) continue
      try {
        if (ticket.approved) await recordApproval(ticket, ticket.approved.at)
        if (ticket.renamed) await recordRename(ticket, ticket.renamed.at, ticket.renamed.title)
        if (ticket.tagged) await recordTags(ticket, ticket.tagged.at, ticket.tags ?? [])
        if (ticket.reviewed) await recordReview(ticket, ticket.reviewed)
        if (ticket.request) await recordRequestEdit(ticket, ticket.request)
        if (!ticket.ended) continue
        if (await archiveNeedsClose(ticket)) {
          let review = ticket.review
          if (!review) {
            review = await captureReview(this.ctx.subprocess, ticket, this.stopped.signal)
            if (review) this.store.atomic(() => {
              const current = this.store.tickets.get(ticket.id)
              if (!current?.ended || current.ended.at !== ticket.ended?.at) return
              current.review = review
              this.store.tickets.save(current)
            })
          }
          // A follow-up may have resumed the ticket while the review was captured.
          const current = this.store.tickets.get(ticket.id)
          if (!current?.ended || current.ended.at !== ticket.ended?.at) continue
          const checks = current.checks && (!current.ended || current.checks.at >= current.ended.at) ? current.checks : undefined
          await closeArchive(ticket, review, this.store.tickets.number(ticket.id), checks)
          repaired++
        }
        if (ticket.rebased) await recordRebase(ticket, ticket.rebased)
        if (ticket.merged) await recordMerge(ticket, ticket.merged)
        if (ticket.cleaned) await recordCleanup(ticket, ticket.cleaned.at, ticket.cleaned.via)
        if (ticket.discarded) await recordDiscard(ticket, ticket.discarded.at)
      } catch (error) { this.ctx.logger.warn('dsh-superagent: completing the archive of ticket %s failed: %s', ticket.id, String(error)) }
    }
    return repaired
  }
  /**
   * Merges an ended ticket's branch into the repository's checked-out branch, fast-forwarding, squashing, or making a merge
   * commit, then records the merge on the ticket and in its local archive. A branch the repository moved past is rebased
   * first, and a configured check that has not passed since the ending runs again, so a person who asked to merge needs no
   * other step. With `cleanup`, the merged worktree and branch are removed afterwards; with `push`, the merged branch goes to
   * the upstream its repository names. A branch that gained commits after its review was captured is captured again, with the
   * check run on the new tree, unless approvals are required.
   * @throws when the ticket is unknown or open, has no workspace, was discarded, a review requested changes, an approval is
   * required and missing, an open review note remains, the configured check fails, a rebase conflicts and the worker cannot
   * take it, or git cannot fast-forward. `origin` is the interface that asked, recorded on the merge.
   */
  async mergeTicket(id: string, strategy: MergeStrategy = 'ff', options: { cleanup?: boolean; push?: boolean } = {},
    origin: ActionOrigin = 'panel'):
    Promise<{ branch: string; target: string; head?: string; rebased?: true; recaptured?: true; cleaned?: true; pushed?: true;
      pushError?: string; targetCheck?: TicketCheck }> {
    // The person and the main Session may name a ticket by its display number, such as T-0001.
    const ticketId = this.ticketReference(id) ?? id
    // One merge at a time for a ticket, so a person's request and a standing one cannot run git on its worktree at once.
    if (this.merging.has(ticketId)) throw new Error('A merge of this ticket is already running')
    this.merging.add(ticketId)
    try { return await this.serialized(() => this.mergeOnce(ticketId, strategy, options, origin)) }
    catch (error) {
      // The report reaches whoever asked; the Ticket keeps it too, for the panel that is the record.
      this.recordMergeError(ticketId, error instanceof Error ? error.message : String(error), origin)
      throw error
    } finally { this.merging.delete(ticketId) }
  }
  /** Records why the latest merge attempt did not land, which the ticket page shows until a merge succeeds. */
  private recordMergeError(ticketId: string, error: string, origin: ActionOrigin): void {
    this.store.atomic(() => {
      const current = this.store.tickets.get(ticketId)
      if (!current) return
      current.mergeError = { at: now(), error: error.slice(0, 2000), via: origin }
      this.store.tickets.save(current)
    })
  }
  /** Runs one repository operation after the others, so two merges or reverts never move one checkout together. */
  private serialized<T>(run: () => Promise<T>): Promise<T> {
    const task = this.repoLock.then(run, run)
    this.repoLock = task.then(() => undefined, () => undefined)
    return task
  }
  /** Merges with the per-ticket guard already held; `mergeTicket` is the public entry point. */
  private async mergeOnce(id: string, strategy: MergeStrategy, options: { cleanup?: boolean; push?: boolean }, origin: ActionOrigin):
    Promise<{ branch: string; target: string; head?: string; rebased?: true; recaptured?: true; cleaned?: true; pushed?: true;
      pushError?: string; targetCheck?: TicketCheck }> {
    const ticket = this.store.tickets.get(id)
    if (!ticket) throw new Error('Unknown ticket')
    if (!ticket.ended) throw new Error('Only an ended ticket can be merged')
    if (ticket.discarded) throw new Error('The ticket worktree was discarded')
    if (ticket.merged) throw new Error('The ticket branch was already merged')
    // A person's stop withdraws standing and queued merges; only an explicit request merges the work afterwards.
    if (origin === 'auto' && ticket.ended.by === 'human') {
      throw new Error('A person stopped this ticket; merge it explicitly if you still want its work')
    }
    // A ticket that waits on another branch merges only once that branch has landed, so order is a recorded fact rather
    // than something the person has to remember.
    const waiting = (ticket.blockedBy ?? []).filter(value => !this.store.tickets.get(value)?.merged)
    if (waiting.length) {
      const names = waiting.map(value => {
        const other = this.store.tickets.get(value)
        return other ? `${value} (${other.title}${other.discarded ? ', discarded' : ''})` : value
      })
      // A discarded blocker never merges, so naming it as a step to take would send the person in a circle.
      const discarded = waiting.some(value => this.store.tickets.get(value)?.discarded)
      throw new Error(discarded
        ? `Blocked by ${names.join(', ')}; a discarded Ticket never merges, so remove that dependency first`
        : `Blocked by ${names.join(', ')}; merge ${waiting.length === 1 ? 'it' : 'them'} first`)
    }
    const directory = ticket.execution?.directory
    if (!directory) throw new Error('This ticket has no workspace')
    if (ticket.reviewed?.decision === 'changes-requested') {
      throw new Error('A review requested changes; address them and approve the review before merging')
    }
    if (this.store.settings().executor.requireReview && ticket.reviewed?.decision !== 'approved') {
      throw new Error('Review approval is required before merging')
    }
    const open = ticket.notes?.filter(note => !note.resolved).length ?? 0
    if (open) throw new Error(`${open} review note${open === 1 ? '' : 's'} still open; resolve them before merging`)
    // The repository may have moved on since the ticket ended; rebasing here keeps a person's "merge it" from needing a
    // second instruction, and the rebase captures the review again from the rebased commits. It also clears an approval, so
    // a deployment that requires one rebases from the ticket page instead, where the new review can be approved.
    let rebased: true | undefined
    if (await targetContained(this.ctx.subprocess, directory, this.stopped.signal) === false) {
      if (this.store.settings().executor.requireReview) {
        throw new Error(`${ticket.execution?.target ?? 'The repository'} has commits the Ticket branch does not; rebase it from the ticket page and approve the review again`)
      }
      await this.rebaseTicket(id, origin)
      rebased = true
    }
    const current = this.store.tickets.get(id)
    if (!current?.ended) throw new Error('Only an ended ticket can be merged')
    if (current.reviewed?.decision === 'changes-requested') {
      throw new Error('A review requested changes; address them and approve the review before merging')
    }
    // A commit made after the ending leaves the review and check describing an older tree. Capture them again instead of
    // merging work no recorded review covers; an approval requirement keeps that decision in the panel.
    let recaptured: true | undefined
    const head = await workspaceHead(this.ctx.subprocess, directory, this.stopped.signal)
    if (head && current.review?.head && current.review.head !== head) {
      if (this.store.settings().executor.requireReview) {
        throw new Error('The Ticket branch changed after its review; approve the new review from the ticket page before merging')
      }
      const endedAt = current.ended.at
      const review = await captureReview(this.ctx.subprocess, current, this.stopped.signal)
      const anchors = await this.noteFreshness(current, review)
      const saved = this.store.atomic(() => {
        const latest = this.store.tickets.get(id)
        // A Ticket that ended again meanwhile owns a newer review, which this capture must not overwrite.
        if (!latest?.ended || latest.merged || latest.ended.at !== endedAt) return false
        if (review) {
          latest.review = review
          this.applyNoteFreshness(latest, review, anchors)
        }
        // The check ran on the tree the review described, so it waits for a fresh run on this one.
        delete latest.checks
        this.store.tickets.save(latest)
        return true
      })
      if (!saved) throw new Error('The ticket changed while its review was captured; merge it again')
      recaptured = true
    }
    const blocked = mergeBlockedByCheck(parseChecks(this.store.settings().executor.check).map(check => check.command).join(' && '),
      this.store.tickets.get(id)?.checks, current.ended.at)
    if (blocked) {
      // An automatic attempt that already saw this ending's check fail waits for the code to change instead of running the
      // same minutes again on every retry; a person asking themselves gets a fresh run.
      const recorded = this.store.tickets.get(id)?.checks
      const recent = origin === 'auto' && recorded && !recorded.ok && recorded.at >= current.ended.at
        && Date.parse(recorded.at) > Date.now() - CHECK_REUSE_MS
      if (recent) {
        throw new Error(`The configured workspace check failed (exit ${recorded.exitCode ?? 'unknown'}) at ${recorded.at}; `
          + 'it is not run again until the Ticket changes')
      }
      // A rebase clears the check, and a ticket may have ended before the command was configured; run it here rather than
      // sending the person to the ticket page for another step.
      const check = await this.checkTicket(id)
      if (!check.ok) {
        const output = check.output.length > 800 ? `…${check.output.slice(-800)}` : check.output
        throw new Error(`The configured workspace check failed (exit ${check.exitCode ?? 'unknown'}); fix it before merging.${output ? `\n${output}` : ''}`)
      }
    }
    // A squash or merge commit carries what the Ticket says it did, as a commit message would.
    const number = this.store.tickets.number(current.id)
    const message = [current.title, latestReport(current)?.summary.trim(),
      `${number ? `Ticket ${number}` : `Ticket ${current.id}`}`].filter(Boolean).join('\n\n').slice(0, 4_000)
    const result = await mergeWorkspace(this.ctx.subprocess, directory, this.stopped.signal, strategy, current.execution?.target, message)
    const at = now()
    this.store.atomic(() => {
      const latest = this.store.tickets.get(id)
      if (!latest) return
      latest.merged = { at, ...result, via: origin, ...strategy === 'ff' ? {} : { strategy } }
      delete latest.mergeError
      this.store.tickets.save(latest)
    })
    try { await recordMerge(current, { at, ...result, via: origin, ...strategy === 'ff' ? {} : { strategy } }) }
    catch (error) { this.ctx.logger.warn('dsh-superagent: recording a merge in the archive failed: %s', String(error)) }
    // The push is the person's request too, but the remote's answer is not this runtime's: a rejected or missing upstream
    // leaves the merge recorded and says why, so one report carries both facts.
    let pushed: true | undefined
    let pushError: string | undefined
    if (options.push) {
      const pushedAt = now()
      let outcome: { at: string; ok: boolean; error?: string; via: ActionOrigin }
      try {
        await pushWorkspace(this.ctx.subprocess, directory, this.stopped.signal, result.target)
        pushed = true
        outcome = { at: pushedAt, ok: true, via: origin }
      } catch (error) {
        pushError = error instanceof Error ? error.message : String(error)
        outcome = { at: pushedAt, ok: false, error: pushError, via: origin }
      }
      // The merge stands either way, so the remote's answer is recorded where the panel and the archive both keep it.
      this.savePush(id, outcome, { merge: at })
      try { await recordPush(this.store.tickets.get(id) ?? current, outcome) }
      catch (error) { this.ctx.logger.warn('dsh-superagent: recording a push in the archive failed: %s', String(error)) }
    }
    // A person may be away from the chat that asked; the panel and a local Source's webhook keep the outcome too.
    const decided = this.store.tickets.get(id) ?? current
    this.ticketNotice(decided, 'merged', `Merged ${this.ticketLabel(decided)} into ${result.target}`
      + `${result.head ? ` at ${result.head}` : ''}${pushError ? `; the push failed: ${pushError}` : pushed ? ', pushed to the upstream' : ''}`)
    // The merge moved the target branch, so the configured check now says whether the merged tree passes; a failure is the
    // person's to see, and the merge can be undone.
    let targetCheck: TicketCheck | undefined
    const configured = parseChecks(this.store.settings().executor.check)
    // A fast-forward merge leaves the target with exactly the tree the Ticket branch was checked on, so the gate's check
    // already answered for it; running the same command again would only cost the same minutes twice.
    const sameTree = strategy === 'ff' && !!result.head && result.head === this.store.tickets.get(id)?.review?.head
    if (configured.length && result.root && !sameTree) {
      let targetRuns: TicketCheckRun[] | undefined
      try {
        const runs = await workspaceChecks(this.ctx.subprocess, result.root, configured, this.stopped.signal,
          Object.fromEntries(this.ticketEnvironment(decided).map(variable => [variable.key, variable.value])))
        targetCheck = runs.aggregate
        targetRuns = runs.runs
      } catch (error) { this.ctx.logger.warn('dsh-superagent: checking the merged target failed: %s', String(error)) }
      if (targetCheck) {
        const run = targetCheck
        this.store.atomic(() => {
          const latest = this.store.tickets.get(id)
          if (!latest?.merged) return
          latest.targetCheck = run
          this.store.tickets.save(latest)
        })
        // A check command that writes into the checkout it runs in leaves the next merge a dirty repository.
        const dirty = await workspaceRepo(this.ctx.subprocess, result.root, this.stopped.signal)
          .then(repo => repo?.dirty === true).catch(() => false)
        if (dirty) {
          this.ticketNotice(decided, 'check', `The configured check left tracked changes in ${result.root}; `
            + 'commit or discard them before the next merge')
        }
        if (!targetCheck.ok) {
          // A red target branch is no place to merge the next Ticket from a queue.
          const queued = queuedMerges(this.store).length > 0
          if (queued) setMergeQueuePaused(this.store, true)
          const failing = targetRuns?.filter(run => !run.ok).map(run => run.name).join(', ')
          this.ticketNotice(decided, 'check', `The configured check${failing ? ` ${failing}` : ''} failed on `
            + `${result.target} after merging ${this.ticketLabel(decided)} (exit ${targetCheck.exitCode ?? 'unknown'}); `
            + 'the merge can be undone' + (queued ? ', and the merge queue is paused' : ''))
        }
      }
    }
    // The merge that lands the last blocker starts the Tickets that were waiting to begin on it.
    this.startWaiting()
    let cleaned: true | undefined
    if (options.cleanup) {
      try {
        await this.cleanupTicket(id, origin)
        cleaned = true
      } catch (error) {
        // The merge already moved the branch, so the report says so rather than presenting this as a failed merge.
        throw new Error(`Merged ${result.branch} into ${result.target}, but removing its worktree failed: ${error instanceof Error ? error.message : String(error)}`)
      }
    }
    return { branch: result.branch, target: result.target, ...result.head ? { head: result.head } : {},
      ...rebased ? { rebased } : {}, ...recaptured ? { recaptured } : {}, ...cleaned ? { cleaned } : {},
      ...pushed ? { pushed } : {}, ...pushError ? { pushError } : {}, ...targetCheck ? { targetCheck } : {} }
  }
  /**
   * Undoes a merged ticket's merge with a reverting commit on the branch it landed, recorded on the ticket and in its archive.
   * The revert runs where the merge did, so it works after the worktree was cleaned up. With `push`, the reverting commit goes
   * to the upstream its repository names, and a failed push leaves the revert in place with its reason recorded.
   * @returns The branch reverted, its commit after the revert, that commit, and the push outcome when one was asked for.
   * @throws when the ticket was not merged or was already reverted, the merge recorded no commit or repository, the
   * repository is on another branch or has tracked changes, the merge commit is gone, or the revert conflicts.
   */
  async revertTicket(id: string, origin: ActionOrigin = 'panel', options: { push?: boolean } = {}):
    Promise<{ ticket_id: string; branch: string; target: string; commit: string; pushed?: true; pushError?: string }> {
    const ticketId = this.ticketReference(id) ?? id
    const ticket = this.store.tickets.get(ticketId)
    if (!ticket) throw new Error('Unknown ticket')
    const merged = ticket.merged
    if (!merged) throw new Error('Only a merged ticket can be reverted')
    // A record from before a revert cleared `merged` still holds both; its merge is already undone.
    if (ticket.reverted && ticket.reverted.at > merged.at) throw new Error('The merge of this ticket was already reverted')
    const commit = merged.head
    if (!commit) throw new Error('The merge recorded no commit to revert; undo it with git in the repository')
    const directory = merged.root ?? ticket.execution?.directory
    if (!directory) throw new Error('The merged repository is unknown; undo the merge with git in the repository')
    if (this.reverting.has(ticketId)) throw new Error('A revert of this ticket is already running')
    this.reverting.add(ticketId)
    try {
      // A fast-forward merge moved the target through the Ticket's commits, so undoing it undoes each of them.
      const from = merged.strategy === undefined ? merged.base : undefined
      const result = await this.serialized(() => revertWorkspace(this.ctx.subprocess, directory, this.stopped.signal, commit,
        merged.target, from))
        .catch((error: unknown) => {
          if (!(error instanceof RevertConflictError)) throw error
          throw new Error(`Reverting the merge of ${merged.branch} into ${merged.target} conflicts in ${error.files.join(', ')}; `
            + 'resolve it with git in the repository, which keeps its commits')
        })
      const at = now()
      this.store.atomic(() => {
        const current = this.store.tickets.get(ticketId)
        if (!current?.merged) return
        // The merge is undone, so the Ticket may merge again; the revert keeps which commit it undid and where, so the
        // reverting commit can still be pushed after the worktree is gone.
        current.reverted = { at, commit: result.head, target: result.target, merge: commit, root: directory, via: origin }
        delete current.merged
        this.store.tickets.save(current)
      })
      try { await recordRevert(ticket, { at, target: result.target, commit: result.head, merge: commit, via: origin }) }
      catch (error) { this.ctx.logger.warn('dsh-superagent: recording a revert in the archive failed: %s', String(error)) }
      const current = this.store.tickets.get(ticketId) ?? ticket
      this.ticketNotice(current, 'reverted', `Reverted the merge of ${merged.branch} into ${merged.target} with ${result.head}`)
      const pushed = options.push ? await this.pushAfterRevert(current, directory, result.target, result.head, origin) : undefined
      return { ticket_id: ticketId, branch: merged.branch, target: result.target, commit: result.head,
        ...pushed?.pushed ? { pushed: true as const } : {}, ...pushed?.pushError ? { pushError: pushed.pushError } : {} }
    } finally { this.reverting.delete(ticketId) }
  }
  /**
   * Pushes the branch a revert moved, recording the remote's answer on the revert; a failure leaves the revert in place and
   * tells the person, since undoing the revert would need the merge again.
   */
  private async pushAfterRevert(ticket: Ticket, directory: string, target: string, commit: string, origin: ActionOrigin):
    Promise<{ pushed?: true; pushError?: string }> {
    const at = now()
    const label = this.ticketLabel(ticket)
    try {
      await this.serialized(() => pushWorkspace(this.ctx.subprocess, directory, this.stopped.signal, target))
      this.savePush(ticket.id, { at, ok: true, via: origin }, { revert: commit })
      try { await recordPush(ticket, { at, ok: true, via: origin }) }
      catch (error) { this.ctx.logger.warn('dsh-superagent: recording a push in the archive failed: %s', String(error)) }
      this.ticketNotice(ticket, 'push', `Pushed the revert of ${label} to the upstream its repository names`)
      return { pushed: true }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      this.savePush(ticket.id, { at, ok: false, error: message, via: origin }, { revert: commit })
      try { await recordPush(ticket, { at, ok: false, error: message, via: origin }) }
      catch (failure) { this.ctx.logger.warn('dsh-superagent: recording a push in the archive failed: %s', String(failure)) }
      this.ticketNotice(ticket, 'push', `Pushing the revert of ${label} failed: ${message}`)
      return { pushError: message }
    }
  }
  /**
   * Records a person's request to merge a ticket as soon as its gates pass, which the passes carry out when it is due.
   * @returns The ticket with the recorded request.
   * @throws when the ticket is unknown, still open, or already merged, discarded, or cleaned.
   */
  autoMergeTicket(id: string, options: { strategy?: MergeStrategy; cleanup?: boolean; push?: boolean }): Ticket {
    const ticketId = this.ticketReference(id) ?? id
    if (this.store.tickets.get(ticketId)?.ended?.by === 'human') {
      throw new Error('A person stopped this ticket; merge it explicitly')
    }
    const ticket = this.engine.autoMerge(ticketId, options)
    this.passes.request()
    return ticket
  }
  /**
   * Clears a person's standing merge request.
   * @returns The ticket without the request.
   */
  cancelAutoMerge(id: string): Ticket { return this.engine.cancelAutoMerge(id) }
  /**
   * Queues Tickets to merge one after another, which a pass drains, so a batch the person asked for survives a restart and
   * reports each Ticket on its own.
   * @returns The Tickets queued with their places, and the queue afterwards.
   * @throws when a Ticket is unknown, or not an ended one that was neither merged, discarded, nor cleaned.
   */
  queueMerges(ids: readonly string[], options: { strategy?: MergeStrategy; cleanup?: boolean; push?: boolean }):
    { queued: { ticket_id: string; position: number }[]; queue: { ticket_id: string; position: number }[] } {
    const entries: QueuedMerge[] = []
    const at = now()
    for (const id of ids) {
      const ticketId = this.ticketReference(id) ?? id
      const ticket = this.store.tickets.get(ticketId)
      if (!ticket) throw new Error(`Unknown ticket: ${id}`)
      if (!ticket.ended || ticket.merged || ticket.discarded || ticket.cleaned) {
        throw new Error(`${id} cannot be queued: only an ended Ticket that is not merged, discarded or cleaned merges`)
      }
      if (ticket.ended.by === 'human') throw new Error(`${id} cannot be queued: a person stopped it; merge it explicitly`)
      entries.push({ ticketId, at, ...options })
    }
    const queue = enqueueMerges(this.store, entries)
    const line = (ticketId: string): { ticket_id: string; position: number } => ({
      ticket_id: this.ticketLabel({ id: ticketId }), position: queue.findIndex(item => item.ticketId === ticketId) + 1 })
    return { queued: entries.map(entry => line(entry.ticketId)), queue: queue.map(item => line(item.ticketId)) }
  }
  /**
   * Clears the recorded failure of queued Tickets, so the next pass tries them again.
   * @returns How many entries were waiting on a retry, and the queue afterwards.
   */
  retryMerges(ids: readonly string[]): { retried: number; queue: { ticket_id: string; position: number }[] } {
    const ticketIds = ids.map(id => this.ticketReference(id) ?? id)
    const retried = retryMerges(this.store, ticketIds)
    return { retried, queue: queuedMerges(this.store).map((item, index) => ({ ticket_id: this.ticketLabel({ id: item.ticketId }),
      position: index + 1 })) }
  }
  /**
   * Takes Tickets out of the merge queue, which a pass drains; one already merging finishes.
   * @returns How many entries were removed, and the queue afterwards.
   */
  dequeueMerges(ids: readonly string[]): { removed: number; queue: { ticket_id: string; position: number }[] } {
    const ticketIds = ids.map(id => this.ticketReference(id) ?? id)
    const { removed, queue } = dequeueMerges(this.store, ticketIds)
    return { removed, queue: queue.map((item, index) => ({ ticket_id: this.ticketLabel({ id: item.ticketId }), position: index + 1 })) }
  }
  /**
   * Merges the next queued Ticket and reports how it went to the main Session. A pass calls this without waiting, so a slow
   * check or merge cannot hold up the pass.
   */
  async runMergeQueue(): Promise<void> {
    if (this.queueMerging) return this.queueMerging
    if (this.paused() || this.closing() || !this.setup.configured() || mergeQueuePaused(this.store)) return
    this.pruneMergeQueue()
    this.recoverMergeQueue()
    if (!nextQueuedMerge(this.store)) return
    const task = this.drainMergeQueue()
      .catch((error: unknown) => { this.ctx.logger.warn('dsh-superagent: a queued merge failed: %s', String(error)) })
      .finally(() => { this.queueMerging = undefined; this.passes.request() })
    this.queueMerging = task
    return task
  }
  /**
   * Retries failed entries whose gate opened since, such as a dependency that merged or a review that was approved, so a
   * person does not have to ask again. A failure that needs new work keeps waiting for `retryMerges`.
   */
  private recoverMergeQueue(): void {
    const retried = recoverableMerges(this.store)
    if (retried.length) retryMerges(this.store, retried)
  }
  /**
   * Drops failed entries whose Ticket moved on, such as one a person merged from the chat while it waited: a retry could
   * never merge it, and the entry would otherwise keep its stale reason forever.
   */
  private pruneMergeQueue(): void {
    const stale = queuedMerges(this.store).filter(item => {
      if (!item.error) return false
      const ticket = this.store.tickets.get(item.ticketId)
      // An open Ticket may end again, such as one resumed to resolve a conflict, so its entry keeps waiting.
      return !ticket || !!ticket.merged || !!ticket.discarded || !!ticket.cleaned
    })
    if (stale.length) dequeueMerges(this.store, stale.map(item => item.ticketId))
  }
  /** Merges the whole queue in order, one Ticket at a time, until it is empty or the deployment stops taking work. */
  private async drainMergeQueue(): Promise<void> {
    let previous: string | undefined
    for (;;) {
      if (this.paused() || this.closing() || mergeQueuePaused(this.store)) return
      // An entry whose last attempt failed waits for a retry rather than running again on its own.
      const entry = nextQueuedMerge(this.store)
      if (!entry) return
      // A drain that cannot make progress on the same entry stops instead of spinning on it.
      if (entry.ticketId === previous) return
      previous = entry.ticketId
      await this.drainQueuedMerge(entry)
    }
  }
  /** Merges one queued Ticket, reports that one result, and takes it off the queue either way. */
  private async drainQueuedMerge(entry: QueuedMerge): Promise<void> {
    const ticket = this.store.tickets.get(entry.ticketId)
    const label = ticket ? this.ticketLabel(ticket) : entry.ticketId
    // How many wait after this one, counted once it left the queue.
    const rest = (): number => queuedMerges(this.store).length
    // A person may have taken it out, or the Ticket may have moved on, since the pass read the queue.
    if (!queuedMerges(this.store).some(item => item.ticketId === entry.ticketId)) return
    if (!ticket?.ended || ticket.merged || ticket.discarded || ticket.cleaned) {
      dequeueMerges(this.store, [entry.ticketId])
      this.sessions.notify(...receiptNotice({ merge_queue: { ticket_id: entry.ticketId, skipped: 'not an ended Ticket that can merge' } },
        `Skipped ${label} in the merge queue: it is no longer an ended Ticket that can merge`))
      return
    }
    try {
      const result = await this.mergeTicket(entry.ticketId, entry.strategy ?? 'ff',
        { ...entry.cleanup === undefined ? {} : { cleanup: entry.cleanup }, ...entry.push === undefined ? {} : { push: entry.push } }, 'auto')
      dequeueMerges(this.store, [entry.ticketId])
      const left = rest()
      this.sessions.notify(...receiptNotice({ merge_queue: { ticket_id: entry.ticketId, ...result, remaining: left } },
        `Merged ${label} from the merge queue${left ? `; ${left} left` : ''}`))
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      // The entry stays at its place with the reason, so a person can retry it or take it out; the gate it names lets a
      // later pass retry it on its own once that gate opens.
      const after = this.store.tickets.get(entry.ticketId)
      markMergeFailed(this.store, entry.ticketId, message, after ? this.mergeFailureCode(after) : 'other')
      const left = rest()
      this.sessions.notify(...receiptNotice({ merge_queue: { ticket_id: entry.ticketId, error: message, remaining: left,
        retryable: true } },
        `Did not merge ${label} from the merge queue: ${message}${left ? `; ${left} left` : ''}. Retry it with orchestrator_merge_queue.`))
    }
  }
  /** The merge queue as the panel shows it. */
  mergeQueue(): MergeQueueView { return mergeQueueView(this.store) }
  /**
   * The queue and the ended Tickets that are not queued but still wait for a merge or a person's decision, so the main
   * Session can answer what is left instead of the person opening the panel.
   * @returns The queue in order and up to twenty waiting Tickets, newest first, each with its ending outcome and the gate it waits on.
   */
  pendingMerges(): { queue: MergeQueueView; waiting: { ticket_id: string; title: string; outcome: string; needs: string; reason?: string;
    blocked_by?: string[] }[] } {
    const queued = new Set(queuedMerges(this.store).map(entry => entry.ticketId))
    const waiting = this.store.tickets.list()
      .filter(ticket => ticket.ended && !ticket.merged && !ticket.discarded && !ticket.cleaned && !queued.has(ticket.id))
      .sort(newestFirst)
      .slice(0, 20)
      .map(ticket => {
        const gate = this.mergeGate(ticket)
        return { ticket_id: this.ticketLabel(ticket), title: ticket.title, outcome: ticket.ended!.outcome, needs: gate.code,
          ...gate.code === 'merge' ? {} : { reason: gate.message },
          ...ticket.blockedBy?.length ? { blocked_by: ticket.blockedBy.map(id => this.ticketLabel({ id })) } : {} }
      })
    return { queue: this.mergeQueue(), waiting }
  }
  /**
   * Holds or releases the merge queue, which keeps its order while paused.
   * @returns The state afterwards.
   */
  pauseMergeQueue(paused: boolean): boolean { return setMergeQueuePaused(this.store, paused) }
  /**
   * Pushes the branch a merge or a revert moved to its upstream again, for one whose push failed or that was never pushed.
   * @returns The Ticket pushed.
   * @throws when the Ticket was neither merged nor reverted, the repository is unknown, or git cannot push; the failure is
   * recorded on the merge or the revert.
   */
  async pushTicket(id: string, origin: ActionOrigin = 'panel'): Promise<{ ticket_id: string; pushed: true }> {
    const ticketId = this.ticketReference(id) ?? id
    const ticket = this.store.tickets.get(ticketId)
    // A revert removes `merged`, so the reverting commit keeps its own target and repository for this push; a record that
    // still holds both, written before that change, pushes the revert that is its current state.
    const reverted = ticket?.reverted && (!ticket.merged || ticket.reverted.at > ticket.merged.at) ? ticket.reverted : undefined
    const merged = reverted ? undefined : ticket?.merged
    const target = merged?.target ?? reverted?.target
    const record: PushTarget | undefined = merged ? { merge: merged.at } : reverted ? { revert: reverted.commit } : undefined
    const directory = merged?.root ?? reverted?.root ?? ticket?.execution?.directory
    if (!ticket || !target || !record) throw new Error('Only a merged or reverted ticket can be pushed')
    // A merged ticket normally records the repository its merge moved; an old record without one needs git by hand.
    if (!directory) throw new Error('The repository the merge or revert moved is unknown; push it with git in the repository')
    const at = now()
    const label = this.ticketLabel(ticket)
    try {
      await this.serialized(() => pushWorkspace(this.ctx.subprocess, directory, this.stopped.signal, target))
      this.savePush(ticketId, { at, ok: true, via: origin }, record)
      try { await recordPush(ticket, { at, ok: true, via: origin }) }
      catch (error) { this.ctx.logger.warn('dsh-superagent: recording a push in the archive failed: %s', String(error)) }
      this.ticketNotice(ticket, 'push', `Pushed ${label} to the upstream its repository names`)
      return { ticket_id: ticketId, pushed: true }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      this.savePush(ticketId, { at, ok: false, error: message, via: origin }, record)
      try { await recordPush(ticket, { at, ok: false, error: message, via: origin }) }
      catch (failure) { this.ctx.logger.warn('dsh-superagent: recording a push in the archive failed: %s', String(failure)) }
      this.ticketNotice(ticket, 'push', `Pushing ${label} failed again: ${message}`)
      throw error
    }
  }
  /** Records the remote's answer on the merge or revert the push acted on, which a later retry replaces. */
  private savePush(ticketId: string, outcome: { at: string; ok: boolean; error?: string; via?: ActionOrigin },
    target: PushTarget): void {
    this.store.atomic(() => {
      const current = this.store.tickets.get(ticketId)
      if (!current) return
      if ('merge' in target) {
        if (current.merged?.at !== target.merge) return
        current.merged.push = outcome
      } else {
        if (current.reverted?.commit !== target.revert) return
        current.reverted.push = outcome
      }
      this.store.tickets.save(current)
    })
  }
  /**
   * What keeps an ended Ticket from merging, for a standing request that must not use up its attempts on a gate the person
   * or another Ticket opens, and for the status the main Session reports.
   */
  private mergeGate(ticket: Ticket): MergeGate {
    const waiting = (ticket.blockedBy ?? []).filter(value => !this.store.tickets.get(value)?.merged)
    if (waiting.length) {
      const names = waiting.map(value => {
        const other = this.store.tickets.get(value)
        return other ? `${this.ticketLabel(other)} (${other.title}${other.discarded ? ', discarded' : ''})` : value
      })
      const discarded = waiting.some(value => this.store.tickets.get(value)?.discarded)
      return { code: 'dependency', counts: discarded, message: discarded
        ? `Blocked by ${names.join(', ')}; a discarded Ticket never merges, so remove that dependency first`
        : `Blocked by ${names.join(', ')}; merge ${waiting.length === 1 ? 'it' : 'them'} first` }
    }
    if (ticket.reviewed?.decision === 'changes-requested') {
      return { code: 'review', counts: false, message: 'A review requested changes; address them and approve the review before merging' }
    }
    if (this.store.settings().executor.requireReview && ticket.reviewed?.decision !== 'approved') {
      return { code: 'review', counts: false, message: 'Review approval is required before merging' }
    }
    const open = ticket.notes?.filter(note => !note.resolved).length ?? 0
    if (open) {
      return { code: 'notes', counts: false,
        message: `${open} review note${open === 1 ? '' : 's'} still open; resolve them before merging` }
    }
    return { code: 'merge' }
  }
  /** The gate a failed merge left closed, so a queued entry can be retried once that gate opens. */
  private mergeFailureCode(ticket: Ticket): MergeFailureCode {
    const gate = this.mergeGate(ticket)
    return gate.code === 'merge' ? 'other' : gate.code
  }
  /** Ends a standing request that ran out of attempts, telling both the conversation and the panel. */
  private autoMergeGaveUp(ticket: Ticket, label: string, message: string): void {
    this.engine.autoMergeDone(ticket.id)
    this.sessions.notify(...receiptNotice({ auto_merge_gave_up: { ticket_id: ticket.id, error: message } },
      `Gave up merging ${label} after ${AUTO_MERGE_ATTEMPTS} attempts: ${message}`))
    this.ticketNotice(ticket, 'automerge', `Gave up merging ${label} after ${AUTO_MERGE_ATTEMPTS} attempts: ${message}`)
  }
  /**
   * Runs one due standing merge request and waits for it. A pass calls this without waiting, so the check or merge it runs
   * cannot hold up the pass, while a test or a restart can run one directly.
   */
  async runAutoMerge(): Promise<void> {
    if (this.autoMerging) return this.autoMerging
    if (this.paused() || this.closing() || !this.setup.configured()) return
    const ticket = this.engine.dueAutoMerge()[0]
    if (!ticket) return
    const attempt = this.attemptAutoMerge(ticket.id)
      .catch((error: unknown) => { this.ctx.logger.warn('dsh-superagent: an auto-merge attempt failed: %s', String(error)) })
      .finally(() => { this.autoMerging = undefined; this.passes.request() })
    this.autoMerging = attempt
    return attempt
  }
  /** Attempts one standing merge request, recording why it did not merge and telling the main Session once per gate. */
  private async attemptAutoMerge(id: string): Promise<void> {
    // The request may have been cancelled, or the Ticket merged, between the pass choosing it and this attempt.
    const current = this.store.tickets.get(id)
    const request = current?.autoMerge
    if (!current || !request || current.merged || current.discarded || current.cleaned) return
    const ticket = current
    const number = this.store.tickets.number(ticket.id)
    const label = number ? `T-${String(number).padStart(4, '0')}` : ticket.id
    // A gate that another Ticket or the person opens waits without using up the request, so a long dependency does not
    // cancel a merge the person asked for.
    const gate = this.mergeGate(ticket)
    if (gate.code !== 'merge') {
      this.engine.autoMergeBlocked(ticket.id, gate.message, gate.counts)
      const pending = this.store.tickets.get(ticket.id)?.autoMerge
      // A gate that counts still ends the request when its attempts run out, such as a dependency that was discarded and
      // can never open.
      if (gate.counts && pending && pending.attempts >= AUTO_MERGE_ATTEMPTS) {
        this.autoMergeGaveUp(ticket, label, gate.message)
        return
      }
      // The person hears each gate the first time it stops the request; later passes retry quietly.
      if (!request.last || request.last.error !== gate.message) {
        this.sessions.notify(...receiptNotice({ auto_merge_blocked: { ticket_id: ticket.id, error: gate.message } },
          `Did not merge ${label} yet: ${gate.message}`))
      }
      return
    }
    try {
      const result = await this.mergeTicket(ticket.id, request.strategy ?? 'ff',
        { ...request.cleanup === undefined ? {} : { cleanup: request.cleanup }, ...request.push === undefined ? {} : { push: request.push } },
        'auto')
      this.engine.autoMergeDone(ticket.id)
      const merged = this.store.tickets.get(ticket.id)
      this.sessions.notify(...receiptNotice({ auto_merged: merged ? ticketCard(merged) : ticket.id, ...result },
        `Merged ${label} on the person's standing request, as its gates passed`))
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      this.engine.autoMergeBlocked(ticket.id, message)
      const pending = this.store.tickets.get(ticket.id)?.autoMerge
      if (pending && pending.attempts >= AUTO_MERGE_ATTEMPTS) {
        this.autoMergeGaveUp(ticket, label, message)
      } else if (!request.last || request.last.error !== message) {
        // Each gate is the one the person needs to hear about once; later passes retry quietly.
        this.sessions.notify(...receiptNotice({ auto_merge_blocked: { ticket_id: ticket.id, error: message } },
          `Did not merge ${label} yet: ${message}`))
      }
    }
  }
  /**
   * Adds a person's note on a file and an optional line or range of a ticket's changes.
   * @returns The ticket's notes after the addition.
   */
  addNote(id: string, input: { file: string; line?: number; endLine?: number; text: string }): TicketNote[] {
    return this.engine.addNote(id, input).notes ?? []
  }
  /**
   * Removes a person's note from a ticket.
   * @returns The ticket's notes after the removal.
   */
  removeNote(id: string, noteId: string): TicketNote[] {
    return this.engine.removeNote(id, noteId).notes ?? []
  }
  /**
   * Marks a review note resolved, or open again.
   * @returns The ticket's notes after the change.
   */
  resolveNote(id: string, noteId: string, resolved: boolean): TicketNote[] {
    return this.engine.resolveNote(id, noteId, resolved).notes ?? []
  }
  /**
   * Adds a reply to a review note's discussion.
   * @returns The ticket's notes after the reply.
   * @throws UserError when the ticket or note is unknown, or the reply is blank.
   */
  replyToNote(id: string, noteId: string, text: string): TicketNote[] {
    return this.engine.replyToNote(id, noteId, text).notes ?? []
  }
  /**
   * Marks every review note resolved, or opens every resolved one again.
   * @returns The ticket's notes after the change.
   */
  resolveAllNotes(id: string, resolved: boolean): TicketNote[] {
    return this.engine.resolveAllNotes(id, resolved).notes ?? []
  }
  /**
   * Hands a ticket back to its worker with a person's text, its unsent review notes, or its failing check, resuming an
   * ended ticket so the worker can act on it.
   * @param input - The text to send, whether to send the unsent notes, and whether to send the failing check.
   * @returns The ticket after the hand-back, whether it resumed, and how many notes were sent.
   * @throws When the ticket is unknown, the selected source has nothing to send, or an ended ticket cannot resume.
   */
  handbackTicket(id: string, input: { text?: string; notes?: boolean; check?: boolean }, origin: ActionOrigin = 'panel'):
    { ticket: Ticket; resumed: boolean; sent: number } {
    const ticket = this.store.tickets.get(id)
    if (!ticket) throw new Error('Unknown ticket')
    if (!input.text && !input.notes && !input.check) throw new Error('Name text, notes or check to hand back')
    if (input.check && !ticket.ended) throw new Error('The ticket is still open; a check runs when it ends')
    const notes = input.notes ? (ticket.notes ?? []).filter(note => !note.resolved && !note.sent) : []
    if (input.notes && !notes.length) throw new Error('No unsent review notes to hand back')
    const check = input.check ? this.failingCheck(ticket) : undefined
    if (input.check && !check) throw new Error('The ticket has no failing check to hand back')
    const parts: string[] = []
    if (input.text) parts.push(input.text)
    if (notes.length) parts.push(`Review notes on the changes:\n${notes.map(note => {
      const range = note.line ? `:${note.line}${note.endLine ? `-${note.endLine}` : ''}` : ''
      const replies = note.replies?.length ? `\n  ${note.replies.map(reply => reply.text).join('\n  ')}` : ''
      return `- ${note.file}${range}: ${note.text}${replies}`
    }).join('\n')}`)
    if (check) parts.push(`The configured check failed:\n${check.command}\n${check.output}`)
    const text = parts.join('\n\n').slice(0, 8_000)
    const live = liveTicket(ticket)
    if (live) this.engine.note(id, text)
    else if (!this.engine.resumeWithNote(id, text)) {
      throw new Error('The ticket cannot resume: it ended by a person, was merged, discarded, or cleaned, or processing is paused')
    }
    if (notes.length) this.engine.markNotesSent(id, notes.map(note => note.id))
    const updated = this.store.tickets.get(id)!
    void recordHandback(updated, { at: now(), text, via: origin }).catch((error: unknown) => {
      this.ctx.logger.warn('dsh-superagent: recording a hand-back in the archive failed: %s', String(error))
    })
    return { ticket: updated, resumed: !live, sent: notes.length }
  }
  /** The failing check of an ended ticket, when its result belongs to the current ending. */
  private failingCheck(ticket: Ticket): TicketCheck | undefined {
    const check = ticket.checks
    return check && !check.ok && (!ticket.ended || check.at >= ticket.ended.at) ? check : undefined
  }
  /** Moves each note's anchor to the current lines after a review capture, marking a note stale when its own lines changed. */
  private applyNoteFreshness(ticket: Ticket, review: TicketReview, anchors: Map<string, NoteAnchor> | undefined): void {
    if (!review.head) return
    for (const note of ticket.notes ?? []) {
      if (!note.commit) continue
      if (note.commit === review.head) {
        note.checkedHead = review.head
        delete note.stale
        continue
      }
      const anchor = anchors?.get(note.id)
      if (!anchor) continue
      note.checkedHead = review.head
      if (anchor.stale) {
        note.stale = { at: review.capturedAt, head: review.head }
        continue
      }
      delete note.stale
      note.commit = review.head
      if (anchor.line === undefined) delete note.line
      else note.line = anchor.line
      if (anchor.endLine === undefined) delete note.endLine
      else note.endLine = anchor.endLine
    }
  }
  /** Moves note anchors against a captured review, leaving notes unchecked when git cannot answer. */
  private async noteFreshness(ticket: Ticket, review: TicketReview | undefined): Promise<Map<string, NoteAnchor> | undefined> {
    if (!review) return undefined
    try { return await remapNoteAnchors(this.ctx.subprocess, ticket, review, this.stopped.signal) }
    catch (error) {
      this.ctx.logger.warn('dsh-superagent: moving review-note anchors failed: %s', String(error))
      return undefined
    }
  }
  /** The newest panel notifications and the unread count. */
  notifications(): ReturnType<typeof listNotifications> { return listNotifications(this.store) }
  /** Marks one notification read; false when its id is unknown. */
  readNotification(id: string): boolean { return readNotification(this.store, id) }
  /** Marks every notification read and returns how many were unread. */
  readAllNotifications(): number { return readAllNotifications(this.store) }
  /**
   * Records a person's review of an ended ticket's changes and appends it to the local archive.
   * @returns The ticket with the review.
   * @throws UserError when the ticket is unknown, still open, or its workspace is gone or merged.
   */
  reviewTicket(id: string, decision: 'approved' | 'changes-requested', note?: string, origin: ActionOrigin = 'panel'): Ticket {
    const ticket = this.engine.reviewTicket(id, decision, note, origin, OPERATOR)
    if (ticket.reviewed) void recordReview(ticket, ticket.reviewed).catch((error: unknown) => {
      this.ctx.logger.warn('dsh-superagent: recording a review in the archive failed: %s', String(error))
    })
    return ticket
  }
  /**
   * Replaces the request text the ticket shows and appends it to the local archive.
   * @returns The ticket with the replacement.
   * @throws UserError when the ticket is unknown or the text is blank.
   */
  editRequest(id: string, text: string): Ticket {
    const ticket = this.engine.editRequest(id, text)
    if (ticket.request) void recordRequestEdit(ticket, ticket.request).catch((error: unknown) => {
      this.ctx.logger.warn('dsh-superagent: recording a request edit in the archive failed: %s', String(error))
    })
    return ticket
  }
  /** Starts pending first Assignments whose start dependencies have all merged. */
  private startWaiting(): void {
    for (const ticket of this.store.tickets.open()) {
      if (!ticket.startAfter?.length) continue
      if (ticket.assignments.some(item => item.startedAt)) continue
      if (ticket.startAfter.some(id => !this.store.tickets.get(id)?.merged)) continue
      const assignment = latestAssignment(ticket)
      if (assignment?.state !== 'pending') continue
      this.executor.start(ticket, assignment)
    }
  }
  /**
   * Links or unlinks another ticket, named by its id or `T-000N`, on both tickets' pages.
   * @returns The changed ticket's related tickets with their numbers and titles.
   * @throws when the reference names no ticket, or either ticket link operation is refused.
   */
  linkTicket(id: string, reference: string, linked: boolean): { related: { id: string; number?: number; title: string }[] } {
    const target = this.ticketReference(reference)
    if (!target) throw new Error('Unknown ticket')
    const ticket = this.engine.linkTicket(id, target, linked)
    return { related: (ticket.related ?? []).flatMap(value => {
      const other = this.store.tickets.get(value)
      const number = this.store.tickets.number(value)
      return other ? [{ id: value, title: other.title, ...number ? { number } : {} }] : []
    }) }
  }
  /**
   * Records or clears that a ticket waits for another ticket's branch to merge first. Both may be named by id or by the
   * display number `T-0001`, as a person and the main Session quote them.
   * @returns The blocked ticket's blockers, with their titles and display numbers.
   * @throws when either ticket is unknown, they are the same, or the dependency would close a cycle.
   */
  blockTicket(id: string, reference: string, blocked: boolean, start = false):
    { ticket_id: string; blocked_by: { id: string; number?: number; title: string; merged: boolean; discarded: boolean; start: boolean }[] } {
    const ticketId = this.ticketReference(id), target = this.ticketReference(reference)
    if (!ticketId || !target) throw new Error('Unknown ticket')
    const current = this.store.tickets.get(ticketId)
    if (blocked && start) {
      if (current && !liveTicket(current)) throw new Error('Only an open Ticket can wait to start')
      if (current?.assignments.some(item => item.startedAt)) {
        throw new Error('The Ticket already started; a start dependency cannot hold it now')
      }
    }
    const ticket = this.engine.blockTicket(ticketId, target, blocked, start)
    // A dependency set after its blocker already merged does not hold the first Assignment back.
    this.startWaiting()
    return { ticket_id: ticket.id, blocked_by: (ticket.blockedBy ?? []).flatMap(value => {
      const other = this.store.tickets.get(value)
      const number = this.store.tickets.number(value)
      return other ? [{ id: value, title: other.title, merged: !!other.merged, discarded: !!other.discarded,
        start: (ticket.startAfter ?? []).includes(value), ...number ? { number } : {} }] : []
    }) }
  }
  /**
   * The id a person's ticket reference names, such as `T-0007`.
   * @param reference - The ticket id or display number.
   * @returns The ticket id, or the trimmed reference when no ticket has it.
   */
  ticketId(reference: string): string { return this.ticketReference(reference) ?? reference.trim() }
  /** Resolves a ticket id or display number (`T-0001`) to its id. */
  private ticketReference(reference: string): string | undefined {
    const value = reference.trim()
    if (this.store.tickets.get(value)) return value
    const match = /^T-?(\d+)$/i.exec(value)
    if (!match) return undefined
    const number = Number(match[1])
    for (const [id, seq] of this.store.tickets.numbering()) if (seq === number) return id
    return undefined
  }
  /**
   * Runs the executor's check command in an ended ticket's worktree and records its outcome on the ticket.
   * @throws when the ticket is unknown, still open, has no workspace, or no check command is configured.
   */
  async checkTicket(id: string): Promise<TicketCheck> {
    const ticket = this.store.tickets.get(id)
    if (!ticket) throw new Error('Unknown ticket')
    if (!ticket.ended) throw new Error('Only an ended ticket can run its check')
    if (ticket.merged) throw new Error('The ticket branch was already merged; its check no longer gates anything')
    if (ticket.discarded || ticket.cleaned) throw new Error('The ticket workspace was removed')
    const directory = ticket.execution?.directory
    if (!directory) throw new Error('This ticket has no workspace')
    const configured = parseChecks(this.store.settings().executor.check)
    if (!configured.length) throw new Error('No workspace check command is configured')
    const env = Object.fromEntries(this.ticketEnvironment(ticket).map(variable => [variable.key, variable.value]))
    const endedAt = ticket.ended.at
    const base = ticket.execution?.base
    const { aggregate, runs } = await workspaceChecks(this.ctx.subprocess, directory, configured, this.stopped.signal, env)
    const saved = this.store.atomic(() => {
      const current = this.store.tickets.get(id)
      if (!current?.ended || current.ended.at !== endedAt || current.merged || current.discarded || current.cleaned
        || current.execution?.base !== base) return false
      this.recordCheck(current, aggregate, runs)
      return true
    })
    if (!saved) throw new Error('The ticket changed while its check ran; run the check again')
    return aggregate
  }
  /** Records a check run as the ticket's latest and appends it to the capped history. */
  private recordCheck(ticket: Ticket, check: TicketCheck, runs?: TicketCheckRun[]): void {
    ticket.checks = check
    ticket.checkHistory = [...(ticket.checkHistory ?? []), check].slice(-10)
    if (runs) ticket.checkRuns = runs
    this.store.tickets.save(ticket)
  }
  /**
   * Rebases an ended ticket's branch onto the repository's checked-out branch, so a later merge can fast-forward.
   * The recorded base moves to the target's commit, so the review and diff exclude upstream commits, and a prior check is
   * cleared because the branch now runs on a different tree. A rebase that conflicts resumes the ticket with the conflict,
   * which its worker resolves in its own worktree.
   * @throws when the ticket is not ended, was merged or discarded, has no workspace, the rebase cannot finish cleanly, or
   * the conflict could not be handed to the worker.
   */
  async rebaseTicket(id: string, origin: ActionOrigin = 'panel'): Promise<{ branch: string; target: string }> {
    const ticket = this.store.tickets.get(id)
    if (!ticket) throw new Error('Unknown ticket')
    if (!ticket.ended) throw new Error('Only an ended ticket can be rebased')
    if (ticket.merged) throw new Error('The ticket branch was already merged')
    if (ticket.discarded) throw new Error('The ticket worktree was discarded')
    const directory = ticket.execution?.directory
    if (!directory) throw new Error('This ticket has no workspace')
    // A rebase moves the Ticket's changes onto a newer commit without changing them; the fingerprint proves whether that
    // is what happened, so an approval of them can survive it.
    const changesBefore = ticket.execution?.base
      ? await patchFingerprint(this.ctx.subprocess, directory, ticket.execution.base, this.stopped.signal) : undefined
    const result = await rebaseWorkspace(this.ctx.subprocess, directory, this.stopped.signal, ticket.execution?.target)
      .catch((error: unknown) => {
        if (error instanceof RebaseConflictError) throw this.handBackConflict(error, ticket)
        throw error
      })
    const rebased = { ...ticket, execution: ticket.execution ? { ...ticket.execution, base: result.base } : undefined }
    const review = await captureReview(this.ctx.subprocess, rebased, this.stopped.signal)
    const anchors = await this.noteFreshness(rebased, review)
    const changesAfter = await patchFingerprint(this.ctx.subprocess, directory, result.base, this.stopped.signal)
    const at = now()
    this.store.atomic(() => {
      const current = this.store.tickets.get(id)
      if (!current) return
      if (current.execution) current.execution = { ...current.execution, base: result.base }
      if (review) {
        current.review = review
        this.applyNoteFreshness(current, review, anchors)
      }
      current.rebased = { at, ...result, via: origin }
      // Upstream commits changed the tree the check ran on, so it waits for a fresh pass.
      delete current.checks
      // The approval covers the changes, not the commit they sat on: keep it when the rebase moved them unchanged.
      if (changesBefore && changesBefore === changesAfter && review && current.reviewed) {
        current.reviewed = { ...current.reviewed, ...review.head ? { head: review.head } : {} }
      } else delete current.reviewed
      this.store.tickets.save(current)
    })
    try { await recordRebase(ticket, { at, ...result, via: origin }) }
    catch (error) { this.ctx.logger.warn('dsh-superagent: recording a rebase in the archive failed: %s', String(error)) }
    return { branch: result.branch, target: result.target }
  }
  /**
   * Hands a rebase conflict to the ticket's worker, which resolves it in its own worktree, so the person who asked to merge
   * need not send the work back themselves. A ticket that cannot resume, such as a stopped or paused one, keeps the plain
   * conflict report.
   */
  private handBackConflict(error: RebaseConflictError, ticket: Ticket): Error {
    const files = error.files.join(', ')
    const note = `Rebasing the ticket branch onto ${error.target} to merge it conflicts in ${files}; the branch keeps its `
      + `commits. In your worktree, run "git rebase ${error.target}", resolve `
      + `${error.files.length === 1 ? 'that file' : 'those files'}, and continue with "git add" and "git rebase --continue"; `
      + 'report ready again once the branch is rebased.'
    if (!this.engine.resumeWithNote(ticket.id, note)) return error
    return new Error(`Rebasing onto ${error.target} conflicts in ${files}; the worker was told and is resolving it. `
      + 'Ask to merge again once it reports ready.')
  }
  /**
   * Removes a merged ticket's worktree and deletes its branch; a squash merge's branch is force-deleted because its
   * commits are represented by the squash commit rather than being ancestors of the target.
   * @throws when the ticket is not merged, its workspace was already removed, or git cannot remove it.
   */
  async cleanupTicket(id: string, origin: ActionOrigin = 'panel'): Promise<void> {
    const ticket = this.store.tickets.get(id)
    if (!ticket) throw new Error('Unknown ticket')
    if (!ticket.merged) throw new Error('Only a merged ticket can be cleaned up')
    if (ticket.discarded || ticket.cleaned) throw new Error('The ticket workspace was already removed')
    const directory = ticket.execution?.directory
    if (!directory) throw new Error('This ticket has no workspace')
    await discardWorkspace(this.ctx.subprocess, directory, this.stopped.signal,
      { merged: true, squash: ticket.merged.strategy === 'squash' })
    const at = now()
    this.store.atomic(() => {
      const current = this.store.tickets.get(id)
      if (!current) return
      current.cleaned = { at, via: origin }
      this.store.tickets.save(current)
    })
    try { await recordCleanup(ticket, at, origin) }
    catch (error) { this.ctx.logger.warn('dsh-superagent: recording a workspace cleanup in the archive failed: %s', String(error)) }
  }
  /**
   * Stops a live ticket on the person's explicit instruction and withdraws its queued or standing merge, which a stop also
   * does for an ended ticket; with `discard` it removes the worktree and branch of a ticket that has ended.
   * @returns The ticket after the change, whether it stopped, and whether it discarded the worktree.
   * @throws when the ticket is unknown, merged, already discarded or cleaned, or its worktree cannot be removed.
   */
  async stopTicket(id: string, options: { discard?: boolean; reason?: string } = {}):
    Promise<{ ticket: Ticket; stopped: boolean; discarded: boolean }> {
    const ticketId = this.ticketReference(id) ?? id
    const ticket = this.store.tickets.get(ticketId)
    if (!ticket) throw new Error('Unknown ticket')
    if (ticket.merged) throw new Error('The ticket branch was already merged; its work is in the target branch')
    if (ticket.discarded) throw new Error('The ticket worktree was already discarded')
    if (ticket.cleaned) throw new Error('The ticket worktree was already cleaned')
    const stopped = !ticket.ended
    // `Tickets.stop` ends a live Ticket and clears a standing merge request; an ended Ticket keeps its ending and loses
    // only the request, so the same call withdraws both kinds of merge.
    this.engine.stopTicket(ticketId, options.reason)
    this.dequeueMerges([ticketId])
    if (options.discard) await this.discardTicket(ticketId)
    return { ticket: this.store.tickets.get(ticketId)!, stopped, discarded: !!options.discard }
  }
  /**
   * Removes an ended ticket's linked worktree and branch, then records the discard on the ticket and in its archive.
   * @throws when the ticket is unknown or open, has no workspace, was merged or discarded, or either removal fails.
   */
  async discardTicket(id: string): Promise<void> {
    const ticketId = this.ticketReference(id) ?? id
    const ticket = this.store.tickets.get(ticketId)
    if (!ticket) throw new Error('Unknown ticket')
    if (!ticket.ended) throw new Error('Only an ended ticket can be discarded')
    if (ticket.merged) throw new Error('The ticket branch was merged; discarding it now would delete the merged branch')
    if (ticket.discarded) throw new Error('The ticket worktree was already discarded')
    if (this.merging.has(ticketId) || this.reverting.has(ticketId)) {
      throw new Error('A merge or revert of this ticket is running; discard it after that finishes')
    }
    const directory = ticket.execution?.directory
    if (!directory) throw new Error('This ticket has no workspace')
    // An ending capture reads the worktree; wait for it before removing what it reads.
    await this.finishing.get(ticketId)
    await discardWorkspace(this.ctx.subprocess, directory, this.stopped.signal)
    const at = now()
    this.store.atomic(() => {
      const current = this.store.tickets.get(ticketId)
      if (!current) return
      // A discarded ticket has no branch left to merge, so a standing merge request goes with it.
      delete current.autoMerge
      current.discarded = { at }
      this.store.tickets.save(current)
    })
    try { await recordDiscard(ticket, at) }
    catch (error) { this.ctx.logger.warn('dsh-superagent: recording a discard in the archive failed: %s', String(error)) }
  }
  /**
   * Captures an ended ticket's workspace facts into `ticket.review` for the panel, and appends the ending, summary, and
   * review to its local archive. Runs after the ending transaction, so a capture failure leaves the ending intact.
   */
  private async finishTicket(ticket: Ticket, ctx: Context): Promise<void> {
    // A worker may merge or rebase the repository's branch into its own before reporting, which leaves the recorded launch
    // base outside the branch; re-anchoring it at the fork point keeps the review to this ticket's own commits.
    const launchBase = ticket.execution?.base
    const base = await workingBase(ctx.subprocess, ticket, this.stopped.signal)
    if (base && launchBase && base !== launchBase) {
      const anchored = this.store.atomic(() => {
        const current = this.store.tickets.get(ticket.id)
        if (!current?.execution || current.execution.base !== launchBase) return false
        current.execution.base = base
        this.store.tickets.save(current)
        return true
      })
      if (!anchored) return
      ticket = { ...ticket, execution: ticket.execution ? { ...ticket.execution, base } : undefined }
    }
    // A follow-up may resume the ticket while the review is captured; its close then belongs to the next ending.
    const stillEnded = (): boolean => {
      const current = this.store.tickets.get(ticket.id)
      return !!current?.ended && current.ended.at === ticket.ended?.at
        && current.execution?.base === ticket.execution?.base
    }
    if (!stillEnded()) return
    let review: TicketReview | undefined
    try { review = await captureReview(ctx.subprocess, ticket, this.stopped.signal) }
    catch (error) { ctx.logger.warn('dsh-superagent: capturing the workspace review failed: %s', String(error)) }
    if (!stillEnded()) return
    const anchors = await this.noteFreshness(ticket, review)
    if (!stillEnded()) return
    if (review) this.store.atomic(() => {
      const current = this.store.tickets.get(ticket.id)
      if (!current?.ended || current.ended.at !== ticket.ended?.at || current.merged || current.discarded
        || current.cleaned || current.execution?.base !== ticket.execution?.base) return
      current.review = review
      this.applyNoteFreshness(current, review, anchors)
      this.store.tickets.save(current)
    })
    // The configured checks run once per ending, so a merge can require their result instead of trusting the agent's report.
    const configured = parseChecks(this.store.settings().executor.check)
    let checks: TicketCheck | undefined
    let checkRuns: TicketCheckRun[] | undefined
    if (configured.length && ticket.execution?.directory) {
      try {
        const runs = await workspaceChecks(ctx.subprocess, ticket.execution.directory, configured, this.stopped.signal,
          Object.fromEntries(this.ticketEnvironment(ticket).map(variable => [variable.key, variable.value])))
        checks = runs.aggregate
        checkRuns = runs.runs
      } catch (error) { ctx.logger.warn('dsh-superagent: running the workspace check failed: %s', String(error)) }
      if (checks && stillEnded()) {
        const run = checks, detail = checkRuns
        this.store.atomic(() => {
          const current = this.store.tickets.get(ticket.id)
          if (!current?.ended || current.ended.at !== ticket.ended?.at || current.merged || current.discarded
            || current.cleaned || current.execution?.base !== ticket.execution?.base) return
          this.recordCheck(current, run, detail)
        })
      }
    }
    if (!stillEnded()) return
    try { await closeArchive(ticket, review, this.store.tickets.number(ticket.id), checks) }
    catch (error) { ctx.logger.warn('dsh-superagent: recording the ending in the archive failed: %s', String(error)) }
    // A check that failed at the ending is the person's to see too, since it keeps the merge from passing.
    const ended = this.store.tickets.get(ticket.id)
    if (ended?.ended?.outcome === 'completed' && checks && !checks.ok) {
      const failing = checkRuns?.filter(run => !run.ok).map(run => run.name).join(', ')
      this.ticketNotice(ended, 'check', `The configured check${failing ? ` ${failing}` : ''} failed at the ending of `
        + `${this.ticketLabel(ended)} (exit ${checks.exitCode ?? 'unknown'}); the merge waits for a passing run`)
    }
    // A deployment that requires an approval gates the merge on a person, who hears of it without opening the panel.
    if (ended?.ended?.outcome === 'completed' && this.store.settings().executor.requireReview
      && ended.reviewed?.decision !== 'approved') {
      this.ticketNotice(ended, 'review', `${this.ticketLabel(ended)} needs an approval of its changes before it can merge`)
    }
  }
  checkSetup(): void { this.setup.check() }
  receive(trigger: Trigger): ReceiveResult { return this.passes.receive(trigger) }
  idle(): Promise<void> { return this.passes.idle() }
  pump(): Promise<void> { return this.passes.pump() }
  sendReplies(): Promise<void> { return this.outbox.sent() }
  private closing(): boolean { return this.closingFlag }
  /** Hold evaluation, dispatch, and outbound delivery while paused or awaiting setup; sources still record incoming triggers. */
  private paused(): boolean { return !this.setup.configured() || this.store.settings().paused }
  private async initialize(): Promise<void> {
    this.dropRemovedSourceData()
    // Register management HTTP routes before setup completes so operators can fix setup problems in the UI.
    this.disposers.push(registerHttp(this.ctx, this.httpHost()))
    if (!await this.setup.run(this.store.settings())) {
      this.runtimeHealth.set({ ...health('paused', 'setupPending'), at: now() })
      return
    }
    this.sessions.install()
    this.executor.install()
    await this.sessions.ensure(this.masterId, this.machine.stateDir, 'dsh-superagent orchestrator', 'orchestrator')
    for (const id of this.settings.syncable()) await this.settings.sync(id).catch(() => { /* sync() records its failure in health. */ })
    // If stop() was called while awaiting setup, session creation, or policy sync, do not start background loops.
    if (this.closing()) return
    this.executor.resume()
    this.passes.start()
    this.effects.start()
    this.runtimeHealth.set({ state: 'ready', detail: 'Orchestrator is ready', at: now() })
    // A crash between an ending and its archive write leaves the archive behind; complete those records once at startup.
    void this.reconcileArchives().catch((error: unknown) => {
      this.ctx.logger.warn('dsh-superagent: completing archive records failed: %s', String(error))
    })
  }
  /** Drop the removed GitHub Sources' settings, Policy parts, read cursors, and ticket bindings from a database that predates their removal. */
  private dropRemovedSourceData(): void {
    const removed = new Set(['github', 'github-issue'])
    const stale = new Set<string>()
    // A ticket whose GitHub channels or followed threads are gone would wait forever for a Source that no longer exists.
    for (const ticket of this.store.tickets.list()) {
      const origin = ticket.origin?.triggerId ? this.store.triggers.get(ticket.origin.triggerId)?.source : undefined
      const githubOrigin = origin !== undefined && removed.has(origin)
      const githubFollow = !!ticket.follows?.some(follow => follow.thread.startsWith('github:'))
      if (!githubFollow && !ticket.ciNotice && !githubOrigin && !(ticket.channels ?? []).some(channel => removed.has(channel.source))) continue
      this.store.atomic(() => {
        const current = this.store.tickets.get(ticket.id)
        if (!current) return
        const channels = (current.channels ?? []).filter(channel => !removed.has(channel.source))
        const followsGitHub = !!current.follows?.some(follow => follow.thread.startsWith('github:'))
        const bound = githubOrigin || followsGitHub || (current.channels ?? []).some(channel => removed.has(channel.source))
        if (bound) stale.add(current.id)
        if (!bound && !current.ciNotice && channels.length === (current.channels ?? []).length) return
        // Only GitHub follows are removed; a custom Source's thread bindings stay.
        if (current.follows) {
          const follows = current.follows.filter(follow => !follow.thread.startsWith('github:'))
          if (follows.length) current.follows = follows
          else delete current.follows
        }
        // A standing merge from a GitHub-bound ticket must not act on the local repository; a local standing merge stays.
        if (bound && current.autoMerge) delete current.autoMerge
        if (current.ciNotice) delete current.ciNotice
        current.channels = channels
        this.store.tickets.save(current)
      })
    }
    const queue = this.store.get(KV.runtime.mergeQueue) ?? []
    const kept = queue.filter(entry => {
      if (stale.has(entry.ticketId)) return false
      const ticket = this.store.tickets.get(entry.ticketId)
      if (!ticket) return false
      const origin = ticket.origin?.triggerId ? this.store.triggers.get(ticket.origin.triggerId)?.source : undefined
      return !(origin !== undefined && removed.has(origin))
    })
    if (kept.length !== queue.length) {
      this.store.set(KV.runtime.mergeQueue, kept)
      // An old queue that paused itself leaves nothing to release; a remaining local queue keeps its pause.
      if (!kept.length) this.store.set(KV.runtime.mergeQueuePaused, false)
    }
    // A pending reply to a removed Source can never be delivered; settle it rather than retrying it forever.
    for (const receipt of this.store.receipts.waiting()) if (removed.has(receipt.channel.source)) this.store.receipts.settle(receipt, 'channelGone')
    // Old CI notifications describe a Source this installation no longer has.
    for (const name of this.store.keys('notification:')) {
      if (this.store.get<{ kind?: string }>(name)?.kind === 'ci') this.store.delete(name)
    }
    // Read cursors and block revisions are removed even when a partial cleanup already dropped the settings.
    for (const prefix of ['github-', 'thread-since:github:', 'thread-since:github-issue:']) for (const name of this.store.keys(prefix)) this.store.delete(name)
    for (const id of removed) {
      this.store.delete(KV.settings.block(`source:${id}`))
      this.store.delete(KV.sources.labels(id))
    }
    const settings = this.store.settings()
    const holds = (record: Record<string, unknown>) => Object.keys(record).some(id => removed.has(id))
    if (!holds(settings.sources) && !holds(settings.policy.sources)) return
    const without = <T>(record: Record<string, T>) => Object.fromEntries(Object.entries(record).filter(([id]) => !removed.has(id)))
    const next = { ...settings, sources: without(settings.sources), policy: { ...settings.policy, sources: without(settings.policy.sources) } }
    this.store.saveSettings(next, { source: 'internal', block: 'policy', policyVersion: policyVersion(next), policyChanged: true, notified: [] })
  }
  private httpHost(): HttpHost {
    const runtime = this
    const { store, sources, prompts, sessions, settings, engine, executor, health, machine, masterId, ready, sourceLinks, setup, passes } = this
    return { store, sources, prompts, sessions, settings, engine, executor, health, machine, masterId, ready, sourceLinks,
      revertTicket: id => runtime.revertTicket(id), pushTicket: id => runtime.pushTicket(id),
      dequeueMerges: ids => runtime.dequeueMerges(ids), retryMerges: ids => runtime.retryMerges(ids),
      mergeQueue: () => runtime.mergeQueue(),
      pauseMergeQueue: paused => runtime.pauseMergeQueue(paused),
      get deployment() { return setup.deployment },
      get setupProblem() { return setup.problem }, get feishuProblem() { return setup.feishuProblem },
      configured: () => setup.configured(), links: () => runtime.links(), ticketEnvironment: ticket => runtime.ticketEnvironment(ticket),
      checkSetup: () => { setup.check() }, requestPass: () => { passes.request() }, models: () => this.ctx.sessionController.modelCatalog(),
      localRequest: input => createLocalRequest(runtime, input), localFollowup: input => createLocalFollowup(runtime, input),
      ticketDiff: id => runtime.ticketDiff(id), mergeTicket: (id, strategy) => runtime.mergeTicket(id, strategy),
      discardTicket: id => runtime.discardTicket(id), rebaseTicket: id => runtime.rebaseTicket(id), checkTicket: id => runtime.checkTicket(id),
      cancelAutoMerge: id => runtime.cancelAutoMerge(id),
      linkTicket: (id, reference, linked) => runtime.linkTicket(id, reference, linked),
      blockTicket: (id, reference, blocked, start) => runtime.blockTicket(id, reference, blocked, start),
      addNote: (id, input) => runtime.addNote(id, input), removeNote: (id, noteId) => runtime.removeNote(id, noteId),
      resolveNote: (id, noteId, resolved) => runtime.resolveNote(id, noteId, resolved),
      resolveAllNotes: (id, resolved) => runtime.resolveAllNotes(id, resolved),
      replyToNote: (id, noteId, text) => runtime.replyToNote(id, noteId, text),
      handbackTicket: (id, input) => runtime.handbackTicket(id, input),
      reviewTicket: (id, decision, note) => runtime.reviewTicket(id, decision, note),
      notifications: () => runtime.notifications(), readNotification: id => runtime.readNotification(id),
      readAllNotifications: () => runtime.readAllNotifications(),
      editRequest: (id, text) => runtime.editRequest(id, text),
      setTags: (id, tags) => runtime.setTags(id, tags),
      cleanupTicket: id => runtime.cleanupTicket(id), renameTicket: (id, title) => runtime.renameTicket(id, title),
      approveTicket: id => runtime.approveTicket(id) }
  }
  /** Idempotent stop; concurrent or repeated calls share the same shutdown promise. */
  stop(): Promise<void> { return this.stopping ??= this.shutdown() }
  private async shutdown(): Promise<void> {
    this.closingFlag = true; const passes = this.passes.stop(); this.stopped.abort()
    try {
      this.assessor.stop()
      const ids = [this.masterId, ...this.store.tickets.list().map(ticket => workerSessionId(ticket.id))]
      const agents = ids.map(id => this.ctx.agents.get(id as SessionId)).filter((agent): agent is Agent => agent !== undefined)
      for (const agent of agents) agent.cancel({ kind: 'user' }, { keepInbox: true })
      await Promise.allSettled([passes, this.effects.drain(), ...agents.map(agent => agent.whenIdle())])
      // The workers' jobs and subagents stop too: their ends would start worker turns without the tool limits released below.
      await this.executor.shutdown()
      this.executor.dispose()
      this.sessions.dispose()
      for (const dispose of this.disposers.reverse()) dispose()
      await this.ctx.sessionPersistence.flush()
    } finally {
      // Close the database before releasing the directory lock so another process cannot open the database mid-write; a compromised lock may already be released.
      this.setup.restore()
      try { this.store.close() } finally {
        const unreleased = (error: unknown) => { this.ctx.logger.warn('dsh-superagent could not release its state directory lock: %s',
          String(error)) }
        await this.releaseLock?.().catch(unreleased)
      }
    }
  }
}

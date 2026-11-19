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
import { Engine, key, reasonText, type DomainEvent, type EngineForSettings, type ReceiveResult, type Ticket, type TicketCheck,
  type TicketNote, type TicketReview, type Trigger } from '../core/index.ts'
import { allowedSender, settingsPolicy } from '../policy.ts'
import { SourceRegistry } from '../sources/registry.ts'
import { Commands } from '../infra/commands.ts'
import type { Deployment, Machine } from '../deployment.ts'
import { environmentOf, ticketEnvironment, type Variable } from './execution-environment.ts'
import type { SetupProblem } from '../setup.ts'
import { DshExecutor, sourceTrigger, triggerMessage, workerSessionId } from '../executor/dsh.ts'
import { ticketCard } from './cards.ts'
import { notifications as listNotifications, readAllNotifications, readNotification, recordNotification } from './notifications.ts'
import { MasterAssessor } from './assessment.ts'
import { KV } from '../kv.ts'
import { Store } from './store.ts'
import { evalsHealth } from './views/overview.ts'
import { TextCache } from './cache.ts'
import { Effects } from './effects.ts'
import { Prompts } from './prompts.ts'
import { registerHttp, type HttpHost } from './http.ts'
import { threadFacts, type LinkTarget } from './views/records.ts'
import { candidateCard, repliedTo, sourceLink, type LinkHost } from './links.ts'
import { mergeBlockedByCheck, workspaceCheck } from './check.ts'
import { captureReview, discardWorkspace, mergeWorkspace, rebaseWorkspace, targetContained, workspaceDiff, type MergeStrategy } from './review.ts'
import { sendWebhook } from './webhook.ts'
import { eventNotice, followupNotice } from './notices.ts'
import { pullEndNotices } from '../sources/github-pull.ts'
import { archiveNeedsClose, closeArchive, createLocalFollowup, createLocalRequest, OPERATOR, recordApproval, recordCleanup, recordDiscard,
  recordMerge, recordRebase, recordRename, recordRequestEdit, recordReview, recordTags, reopenArchive, seedPolicy, ticketArchive, webhookFor,
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
        feishuPageSize: () => feishuSettings(sources).readPageSize, sourceLink: link }
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
          // Notify the ticket's conversations whenever it ends via a pull request.
          const ended = event.type === 'ticket.ended' ? store.tickets.get(event.data.ticketId) : undefined
          if (ended && setup.configured()) {
            const request = store.triggers.get(ended.origin.triggerId)?.text ?? ''
            for (const message of pullEndNotices(ended, request, setup.deployment.repository)) outbox.send(message)
          }
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
          if (ended) this.store.afterCommit(() => {
            void this.finishTicket(ended, ctx).catch((error: unknown) => {
              ctx.logger.warn('dsh-superagent: completing the ending of ticket %s failed: %s', ended.id, String(error))
            })
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
        health: this.runtimeHealth, lifetime: this.lifetime, configured: () => setup.configured(), closing: () => this.closing() })
      this.sourceDeps = { store, engine, outbox, commands, sources, get ready() { return runtime.ready },
        get deployment() { return setup.deployment }, configured: () => setup.configured(),
        get setupProblem() { return setup.problem }, get feishuProblem() { return setup.feishuProblem }, receive: trigger => passes.receive(trigger) }
      this.toolHost = { context: ctx, engine, store, prompts, sessions, settings: this.settings, assessor, resources,
        localRequest: input => this.localRequest(input),
        localFollowup: (input, options) => createLocalFollowup(runtime, input, options),
        mergeTicket: (id, strategy, options) => runtime.mergeTicket(id, strategy, options),
        get github() { return runtime.context({ kind: 'github', health: ['github'] }) },
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
    const links = resolved ? { repository: resolved.repository, ...(resolved.feishu ? { feishu: resolved.feishu } : {}) } : {}
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
  async ticketDiff(id: string): Promise<{ diff: string; truncated: boolean; error?: string }> {
    const ticket = this.store.tickets.get(id)
    const directory = ticket?.execution?.directory
    const base = ticket?.execution?.base
    if (!directory) return { diff: '', truncated: false, error: 'This ticket has no workspace' }
    if (!base) return { diff: '', truncated: false, error: 'No base commit was recorded for this ticket' }
    try { return await workspaceDiff(this.ctx.subprocess, directory, base, this.stopped.signal) }
    catch (error) { return { diff: '', truncated: false, error: String(error) } }
  }
  /**
   * POSTs a local ticket's question or ending to the local Source's webhook, when one is configured. Best effort: a
   * failure is logged and the ticket is unaffected.
   */
  private notifyWebhook(event: Extract<DomainEvent, { type: 'ticket.attention' } | { type: 'ticket.ended' }>): void {
    const ticket = this.store.tickets.get(event.data.ticketId)
    if (!ticket || !ticket.channels.some(channel => channel.source === 'local')) return
    const url = webhookFor(this.store.settings())
    if (!url) return
    const base = { at: event.at, ticket: { id: ticket.id, number: this.store.tickets.number(ticket.id) ?? 0, title: ticket.title } }
    const payload = event.type === 'ticket.ended'
      ? { event: 'ended' as const, ...base, outcome: event.data.outcome, reason: event.data.reason ?? event.data.outcome }
      : { event: 'attention' as const, ...base, question: event.data.question }
    void sendWebhook(url, payload, this.stopped.signal).catch((error: unknown) => {
      this.ctx.logger.warn('dsh-superagent: webhook notification failed: %s', String(error))
    })
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
        if (ticket.cleaned) await recordCleanup(ticket, ticket.cleaned.at)
        if (ticket.discarded) await recordDiscard(ticket, ticket.discarded.at)
      } catch (error) { this.ctx.logger.warn('dsh-superagent: completing the archive of ticket %s failed: %s', ticket.id, String(error)) }
    }
    return repaired
  }
  /**
   * Merges an ended ticket's branch into the repository's checked-out branch, fast-forwarding, squashing, or making a merge
   * commit, then records the merge on the ticket and in its local archive. A branch the repository moved past is rebased
   * first, and a configured check that has not passed since the ending runs again, so a person who asked to merge needs no
   * other step. With `cleanup`, the merged worktree and branch are removed afterwards.
   * @throws when the ticket is unknown or open, has no workspace, was discarded, a review requested changes, an approval is
   * required and missing, an open review note remains, the configured check fails, a rebase conflicts, or git cannot
   * fast-forward.
   */
  async mergeTicket(id: string, strategy: MergeStrategy = 'ff', options: { cleanup?: boolean } = {}):
    Promise<{ branch: string; target: string; rebased?: true; cleaned?: true }> {
    const ticket = this.store.tickets.get(id)
    if (!ticket) throw new Error('Unknown ticket')
    if (!ticket.ended) throw new Error('Only an ended ticket can be merged')
    if (ticket.discarded) throw new Error('The ticket worktree was discarded')
    if (ticket.merged) throw new Error('The ticket branch was already merged')
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
    // A bound pull request's CI is a fact on its Thread; a failed run blocks merging the ticket's work.
    for (const follow of ticket.follows ?? []) {
      const thread = this.store.threads.get(follow.thread)
      const ci = thread ? threadFacts(this.store, thread).facts.ci : undefined
      if (ci?.problem) throw new Error(`The pull request on ${follow.thread} has failing CI; fix it before merging`)
    }
    // The repository may have moved on since the ticket ended; rebasing here keeps a person's "merge it" from needing a
    // second instruction, and the rebase captures the review again from the rebased commits. It also clears an approval, so
    // a deployment that requires one rebases from the ticket page instead, where the new review can be approved.
    let rebased: true | undefined
    if (await targetContained(this.ctx.subprocess, directory, this.stopped.signal) === false) {
      if (this.store.settings().executor.requireReview) {
        throw new Error(`${ticket.execution?.target ?? 'The repository'} has commits the Ticket branch does not; rebase it from the ticket page and approve the review again`)
      }
      await this.rebaseTicket(id)
      rebased = true
    }
    const current = this.store.tickets.get(id)
    if (!current?.ended) throw new Error('Only an ended ticket can be merged')
    if (current.reviewed?.decision === 'changes-requested') {
      throw new Error('A review requested changes; address them and approve the review before merging')
    }
    const blocked = mergeBlockedByCheck(this.store.settings().executor.check, current.checks, current.ended.at)
    if (blocked) {
      // A rebase clears the check, and a ticket may have ended before the command was configured; run it here rather than
      // sending the person to the ticket page for another step.
      const check = await this.checkTicket(id)
      if (!check.ok) {
        const output = check.output.length > 800 ? `…${check.output.slice(-800)}` : check.output
        throw new Error(`The configured workspace check failed (exit ${check.exitCode ?? 'unknown'}); fix it before merging.${output ? `\n${output}` : ''}`)
      }
    }
    const result = await mergeWorkspace(this.ctx.subprocess, directory, this.stopped.signal, strategy, current.execution?.target)
    const at = now()
    this.store.atomic(() => {
      const latest = this.store.tickets.get(id)
      if (!latest) return
      latest.merged = { at, ...result, ...strategy === 'ff' ? {} : { strategy } }
      this.store.tickets.save(latest)
    })
    try { await recordMerge(current, { at, ...result, ...strategy === 'ff' ? {} : { strategy } }) }
    catch (error) { this.ctx.logger.warn('dsh-superagent: recording a merge in the archive failed: %s', String(error)) }
    let cleaned: true | undefined
    if (options.cleanup) {
      try {
        await this.cleanupTicket(id)
        cleaned = true
      } catch (error) {
        // The merge already moved the branch, so the report says so rather than presenting this as a failed merge.
        throw new Error(`Merged ${result.branch} into ${result.target}, but removing its worktree failed: ${error instanceof Error ? error.message : String(error)}`)
      }
    }
    return { ...result, ...rebased ? { rebased } : {}, ...cleaned ? { cleaned } : {} }
  }
  /**
   * Adds a person's note on a file and line of a ticket's changes.
   * @returns The ticket's notes after the addition.
   */
  addNote(id: string, input: { file: string; line?: number; text: string }): TicketNote[] {
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
  reviewTicket(id: string, decision: 'approved' | 'changes-requested', note?: string): Ticket {
    const ticket = this.engine.reviewTicket(id, decision, note)
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
    const command = this.store.settings().executor.check.trim()
    if (!command) throw new Error('No workspace check command is configured')
    const env = Object.fromEntries(this.ticketEnvironment(ticket).map(variable => [variable.key, variable.value]))
    const endedAt = ticket.ended.at
    const base = ticket.execution?.base
    const checks = await workspaceCheck(this.ctx.subprocess, directory, command, this.stopped.signal, env)
    const saved = this.store.atomic(() => {
      const current = this.store.tickets.get(id)
      if (!current?.ended || current.ended.at !== endedAt || current.merged || current.discarded || current.cleaned
        || current.execution?.base !== base) return false
      this.recordCheck(current, checks)
      return true
    })
    if (!saved) throw new Error('The ticket changed while its check ran; run the check again')
    return checks
  }
  /** Records a check run as the ticket's latest and appends it to the capped history. */
  private recordCheck(ticket: Ticket, check: TicketCheck): void {
    ticket.checks = check
    ticket.checkHistory = [...(ticket.checkHistory ?? []), check].slice(-10)
    this.store.tickets.save(ticket)
  }
  /**
   * Rebases an ended ticket's branch onto the repository's checked-out branch, so a later merge can fast-forward.
   * The recorded base moves to the target's commit, so the review and diff exclude upstream commits, and a prior check is
   * cleared because the branch now runs on a different tree.
   * @throws when the ticket is not ended, was merged or discarded, has no workspace, or the rebase cannot finish cleanly.
   */
  async rebaseTicket(id: string): Promise<{ branch: string; target: string }> {
    const ticket = this.store.tickets.get(id)
    if (!ticket) throw new Error('Unknown ticket')
    if (!ticket.ended) throw new Error('Only an ended ticket can be rebased')
    if (ticket.merged) throw new Error('The ticket branch was already merged')
    if (ticket.discarded) throw new Error('The ticket worktree was discarded')
    const directory = ticket.execution?.directory
    if (!directory) throw new Error('This ticket has no workspace')
    const result = await rebaseWorkspace(this.ctx.subprocess, directory, this.stopped.signal, ticket.execution?.target)
    const rebased = { ...ticket, execution: ticket.execution ? { ...ticket.execution, base: result.base } : undefined }
    const review = await captureReview(this.ctx.subprocess, rebased, this.stopped.signal)
    const at = now()
    this.store.atomic(() => {
      const current = this.store.tickets.get(id)
      if (!current) return
      if (current.execution) current.execution = { ...current.execution, base: result.base }
      if (review) current.review = review
      current.rebased = { at, ...result }
      // Upstream commits changed the tree the check ran on and the review approved, so both wait for a fresh pass.
      delete current.checks
      delete current.reviewed
      this.store.tickets.save(current)
    })
    try { await recordRebase(ticket, { at, ...result }) }
    catch (error) { this.ctx.logger.warn('dsh-superagent: recording a rebase in the archive failed: %s', String(error)) }
    return { branch: result.branch, target: result.target }
  }
  /**
   * Removes a merged ticket's worktree and deletes its branch; a squash merge's branch is force-deleted because its
   * commits are represented by the squash commit rather than being ancestors of the target.
   * @throws when the ticket is not merged, its workspace was already removed, or git cannot remove it.
   */
  async cleanupTicket(id: string): Promise<void> {
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
      current.cleaned = { at }
      this.store.tickets.save(current)
    })
    try { await recordCleanup(ticket, at) }
    catch (error) { this.ctx.logger.warn('dsh-superagent: recording a workspace cleanup in the archive failed: %s', String(error)) }
  }
  /**
   * Removes an ended ticket's linked worktree and branch, then records the discard on the ticket and in its archive.
   * @throws when the ticket is unknown or open, has no workspace, was merged or discarded, or either removal fails.
   */
  async discardTicket(id: string): Promise<void> {
    const ticket = this.store.tickets.get(id)
    if (!ticket) throw new Error('Unknown ticket')
    if (!ticket.ended) throw new Error('Only an ended ticket can be discarded')
    if (ticket.merged) throw new Error('The ticket branch was merged; discarding it now would delete the merged branch')
    if (ticket.discarded) throw new Error('The ticket worktree was already discarded')
    const directory = ticket.execution?.directory
    if (!directory) throw new Error('This ticket has no workspace')
    await discardWorkspace(this.ctx.subprocess, directory, this.stopped.signal)
    const at = now()
    this.store.atomic(() => {
      const current = this.store.tickets.get(id)
      if (!current) return
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
    if (review) this.store.atomic(() => {
      const current = this.store.tickets.get(ticket.id)
      if (!current?.ended || current.ended.at !== ticket.ended?.at || current.merged || current.discarded
        || current.cleaned || current.execution?.base !== ticket.execution?.base) return
      current.review = review
      this.store.tickets.save(current)
    })
    // The configured check runs once per ending, so a merge can require its result instead of trusting the agent's report.
    const command = this.store.settings().executor.check.trim()
    let checks: TicketCheck | undefined
    if (command && ticket.execution?.directory) {
      try {
        checks = await workspaceCheck(ctx.subprocess, ticket.execution.directory, command, this.stopped.signal,
          Object.fromEntries(this.ticketEnvironment(ticket).map(variable => [variable.key, variable.value])))
      } catch (error) { ctx.logger.warn('dsh-superagent: running the workspace check failed: %s', String(error)) }
      if (checks && stillEnded()) {
        const run = checks
        this.store.atomic(() => {
          const current = this.store.tickets.get(ticket.id)
          if (!current?.ended || current.ended.at !== ticket.ended?.at || current.merged || current.discarded
            || current.cleaned || current.execution?.base !== ticket.execution?.base) return
          this.recordCheck(current, run)
        })
      }
    }
    if (!stillEnded()) return
    try { await closeArchive(ticket, review, this.store.tickets.number(ticket.id), checks) }
    catch (error) { ctx.logger.warn('dsh-superagent: recording the ending in the archive failed: %s', String(error)) }
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
  private httpHost(): HttpHost {
    const runtime = this
    const { store, sources, prompts, sessions, settings, engine, executor, health, machine, masterId, ready, sourceLinks, setup, passes } = this
    return { store, sources, prompts, sessions, settings, engine, executor, health, machine, masterId, ready, sourceLinks,
      get deployment() { return setup.deployment },
      get setupProblem() { return setup.problem }, get feishuProblem() { return setup.feishuProblem },
      configured: () => setup.configured(), links: () => runtime.links(), ticketEnvironment: ticket => runtime.ticketEnvironment(ticket),
      checkSetup: () => { setup.check() }, requestPass: () => { passes.request() }, models: () => this.ctx.sessionController.modelCatalog(),
      localRequest: input => createLocalRequest(runtime, input), localFollowup: input => createLocalFollowup(runtime, input),
      ticketDiff: id => runtime.ticketDiff(id), mergeTicket: (id, strategy) => runtime.mergeTicket(id, strategy),
      discardTicket: id => runtime.discardTicket(id), rebaseTicket: id => runtime.rebaseTicket(id), checkTicket: id => runtime.checkTicket(id),
      linkTicket: (id, reference, linked) => runtime.linkTicket(id, reference, linked),
      addNote: (id, input) => runtime.addNote(id, input), removeNote: (id, noteId) => runtime.removeNote(id, noteId),
      resolveNote: (id, noteId, resolved) => runtime.resolveNote(id, noteId, resolved),
      replyToNote: (id, noteId, text) => runtime.replyToNote(id, noteId, text),
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

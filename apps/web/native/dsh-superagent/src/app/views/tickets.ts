/** Ticket list, ticket detail, mandate basis formatting, and executor load views. */
import { liveTicket, threadKey, ticketStage, type AcceptMode, type ActionOrigin, type AdmissionCell, type MandateBasis,
  type Ticket, type TicketStage } from '../../core/index.ts'
import { workerSessionId } from '../../executor/dsh.ts'
import type { Store } from '../store.ts'
import { channelLink, decisionView, newestFirst, page, snapshot, threadSummary, ticketLine, type Snapshot,
  type ViewHost } from './records.ts'
import { mergePosition } from '../merge-queue.ts'
import { settingsPolicy } from '../../policy.ts'
import { markdown, plain, preview, readable } from './text.ts'

export type BasisFact =
  | { name: 'addressed'; value: 'mention' | 'direct' } | { name: 'followup' } | { name: 'admission'; value: AdmissionCell } | { name: 'observation' }
  | { name: 'mode'; value: AcceptMode } | { name: 'rules'; value: { id: string; name: string }[] } | { name: 'sender'; value: string }
  | { name: 'legacyAuthority'; value: string } | { name: 'bot' }
/**
 * Extracts displayable mandate admission facts in reading order (omitting the receiving scope, which is stored
 * only for re-validation), marking pre-scope records that only have `authority` as `legacy`.
 */
export function basisFacts(basis: MandateBasis, rule: Snapshot['rule']): { facts: BasisFact[]; legacy: boolean } {
  const facts: BasisFact[] = []
  if (basis.sender) facts.push({ name: 'sender', value: basis.sender })
  if (basis.via === 'bot') facts.push({ name: 'bot' })
  if (basis.via !== 'followup' && basis.addressed) facts.push({ name: 'addressed', value: basis.addressed })
  // Imported bases already record their follow-up in `authority`.
  if (basis.via === 'followup' && !basis.authority) facts.push({ name: 'followup' })
  if (basis.via === 'admission') facts.push({ name: 'admission', value: basis.cell })
  const observation = basis.via === 'admission' || basis.via === 'listed' || basis.via === 'bot' ? basis.observation : undefined
  // Listed bot messages are inferred candidates regardless of where received, so only the accept mode is shown from `observation`.
  if (observation && basis.via !== 'bot') facts.push({ name: 'observation' })
  if (observation?.mode) facts.push({ name: 'mode', value: observation.mode })
  if (basis.rules?.length) facts.push({ name: 'rules', value: basis.rules.map(rule) })
  const authority = basis.via === 'followup' || basis.via === 'listed' ? basis.authority : undefined
  if (authority) facts.push({ name: 'legacyAuthority', value: authority })
  return { facts, legacy: authority !== undefined }
}
const tagsOf = (store: Pick<Store, 'triggers'>, ticket: Pick<Ticket, 'origin' | 'tags'>): string[] => {
  if (ticket.tags) return ticket.tags
  const value = store.triggers.get(ticket.origin.triggerId)?.meta.tags
  return Array.isArray(value) ? value.filter((tag): tag is string => typeof tag === 'string') : []
}
/** Every tag on any ticket's request, sorted, for the Execution filter. */
export function ticketTags(store: Pick<Store, 'tickets' | 'triggers'>): string[] {
  const all = new Set<string>()
  for (const ticket of store.tickets.list()) for (const tag of tagsOf(store, ticket)) all.add(tag)
  return [...all].sort((a, b) => a.localeCompare(b))
}
export function ticketList(host: ViewHost, stage: 'open' | TicketStage['stage'] | 'review', offset: number, limit: number, tag?: string) {
  const view = snapshot(host)
  const { store } = view
  // The review worklist is the ended Tickets a person still has to decide on: not merged, and not approved.
  const pending = (ticket: Ticket): boolean => !liveTicket(ticket) && !ticket.merged && !ticket.discarded && !ticket.cleaned
    && ticket.reviewed?.decision !== 'approved'
  const shown = (ticket: Ticket) => stage === 'open' ? liveTicket(ticket)
    : stage === 'review' ? pending(ticket) : ticketStage(ticket).stage === stage
  const tickets = (stage === 'ended' || stage === 'review' ? store.tickets.list() : store.tickets.open()).filter(shown)
    .filter(ticket => !tag || tagsOf(store, ticket).includes(tag)).sort(newestFirst)
  const result = page(tickets, offset, limit)
  return { ...result, items: result.items.map(ticket => ticketRow(view, ticket)) }
}
export type TicketListView = ReturnType<typeof ticketList>
export function ticketRow(view: Snapshot, ticket: Ticket) {
  const line = ticketLine(view, ticket)
  const channel = ticket.channels[0]
  const reply = line.stage === 'person' && channel ? channelLink(view, channel) : undefined
  return { ...line, ...reply ? { reply } : {} }
}
/** Executor slot utilization: `running` worker Sessions in an active turn (which occupy slots and are interrupted by a restart), and tickets `queued` for a slot. */
export interface ExecutorLoad { running: number; queued: number }
export function executorLoad(store: Store, running: number): ExecutorLoad {
  const queued = (ticket: Ticket) => { const stage = ticketStage(ticket); return stage.stage === 'working' && stage.action === 'queued' }
  return { running, queued: store.tickets.open().filter(queued).length }
}
export function ticketDetail(host: ViewHost, id: string) {
  const { store } = host
  const ticket = store.tickets.get(id)
  if (!ticket) return undefined
  const view = snapshot(host)
  const { stage } = ticketStage(ticket)
  const decision = store.decisions.get(ticket.decisionId)
  const input = (value: Ticket['inputs'][number]) => {
    const trigger = value.kind === 'trigger' ? store.triggers.get(value.triggerId) : undefined
    const handback = trigger && store.triggers.disposition(trigger.id)?.handbacks?.find(item => item.ticketId === id)
    const shown = trigger && { id: trigger.id, actor: view.actor(trigger.actor, trigger.human), human: trigger.human,
      preview: preview(trigger.text), thread: threadKey(trigger.threads[0]!),
      ...handback ? { handback: { reason: handback.reason, at: handback.at } } : {} }
    const text = (value.kind === 'text' || value.kind === 'note') && value.text ? { text: markdown(value.text) } : {}
    return { ...value, ...text, delivered: !ticket.undelivered.includes(value.id), ...shown ? { trigger: shown } : {} }
  }
  // Pass `human = false` for bot-originated mandates so the Source kind still labels the actor as a bot if removed from the policy list.
  const human = ticket.mandate.basis.via !== 'bot'
  const line = ticketLine(view, ticket)
  // Reopening needs the current Policy to still hold the ticket's Mandate, as the reopen command itself checks.
  const reopen = !liveTicket(ticket) && !ticket.discarded && !ticket.cleaned && !ticket.merged
    && settingsPolicy(store.settings(), host.sources).revalidate(ticket.mandate)
  // Ending a ticket's workspace state is offered once it ended, has a review, and was neither merged nor discarded.
  const finish = !liveTicket(ticket) && !!ticket.review?.branch && !ticket.merged && !ticket.discarded
  const rebase = finish && !!ticket.execution?.directory
  const check = !liveTicket(ticket) && !ticket.merged && !ticket.discarded && !ticket.cleaned
    && !!ticket.execution?.directory && !!store.settings().executor.check.trim()
  const cleanup = !liveTicket(ticket) && !!ticket.merged && !ticket.discarded && !ticket.cleaned && !!ticket.execution?.directory
  // A merge a revert already undid (a record from before the revert cleared `merged`) is not undone twice.
  const undone = !!ticket.merged && !!ticket.reverted && ticket.reverted.at > ticket.merged.at
  // Undoing a merge needs the repository the merge recorded, which survives the removal of its worktree.
  const revert = !!ticket.merged && !undone && !!(ticket.merged.root ?? ticket.execution?.directory)
  // A revert of a merge that a later merge replaced is history, not the ticket's current state.
  const reverted = ticket.reverted && ticket.reverted.at > (ticket.merged?.at ?? '') ? ticket.reverted : undefined
  // A merge or a revert that was never pushed, or whose push failed, can be pushed where it ran, from the panel.
  const pushTarget = reverted ?? ticket.merged
  const push = !!pushTarget && pushTarget.push?.ok !== true && !!(pushTarget.root ?? ticket.execution?.directory)
  // A queued Ticket shows its place and can leave the queue from its page.
  const queuePosition = mergePosition(store, ticket.id)
  const reviewDecision = !liveTicket(ticket) && !ticket.merged && !ticket.discarded && !ticket.cleaned
  // The decisions a person made on the Ticket, in the order they happened, with the interface each came from.
  const history: { code: 'approved' | 'renamed' | 'tagged' | 'reviewed' | 'rebased' | 'merged' | 'reverted' | 'cleaned' | 'discarded'; at: string;
    title?: string; tags?: string[]; decision?: 'approved' | 'changes-requested'; note?: string; branch?: string; target?: string;
    via?: ActionOrigin; by?: string; head?: string; merge?: string; push?: { ok: boolean; error?: string } }[] = []
  if (ticket.approved) history.push({ code: 'approved', at: ticket.approved.at })
  if (ticket.renamed) history.push({ code: 'renamed', at: ticket.renamed.at, title: ticket.renamed.title })
  if (ticket.tagged) history.push({ code: 'tagged', at: ticket.tagged.at, tags: ticket.tags ?? [] })
  // Every review is shown, including rounds a resume or a rebase cleared from the current decision.
  for (const review of ticket.reviews ?? (ticket.reviewed ? [ticket.reviewed] : [])) {
    history.push({ code: 'reviewed', at: review.at, decision: review.decision, ...(review.note ? { note: review.note } : {}),
      ...(review.head ? { head: review.head } : {}), ...(review.via ? { via: review.via } : {}),
      ...(review.by ? { by: review.by } : {}) })
  }
  if (ticket.rebased) history.push({ code: 'rebased', at: ticket.rebased.at, branch: ticket.rebased.branch,
    target: ticket.rebased.target, ...(ticket.rebased.via ? { via: ticket.rebased.via } : {}) })
  if (ticket.merged) history.push({ code: 'merged', at: ticket.merged.at, branch: ticket.merged.branch, target: ticket.merged.target,
    ...(ticket.merged.head ? { head: ticket.merged.head } : {}), ...(ticket.merged.via ? { via: ticket.merged.via } : {}),
    ...(ticket.merged.push ? { push: { ok: ticket.merged.push.ok, ...(ticket.merged.push.error ? { error: ticket.merged.push.error } : {}) } } : {}) })
  if (ticket.reverted) history.push({ code: 'reverted', at: ticket.reverted.at, head: ticket.reverted.commit,
    merge: ticket.reverted.merge, ...(ticket.reverted.push ? { push: { ok: ticket.reverted.push.ok,
      ...(ticket.reverted.push.error ? { error: ticket.reverted.push.error } : {}) } } : {}),
    ...(ticket.reverted.via ? { via: ticket.reverted.via } : {}) })
  if (ticket.cleaned) history.push({ code: 'cleaned', at: ticket.cleaned.at, ...(ticket.cleaned.via ? { via: ticket.cleaned.via } : {}) })
  if (ticket.discarded) history.push({ code: 'discarded', at: ticket.discarded.at })
  history.sort((a, b) => a.at.localeCompare(b.at))
  const origin = store.triggers.get(ticket.origin.triggerId)
  const request = origin ? { text: plain(readable(ticket.request?.text ?? origin.text), true), at: origin.at,
    ...(ticket.request ? { editedAt: ticket.request.at } : {}) } : undefined
  // A check from before the current ending belongs to work that was resumed and must not be shown as this ending's.
  const checks = ticket.checks && (!ticket.ended || ticket.checks.at >= ticket.ended.at) ? ticket.checks : undefined
  const local = ticket.channels.find(channel => channel.source === 'local')
  return { ...line, actions: { ...line.actions, reopen, approve: liveTicket(ticket) && !!ticket.hold, merge: finish, discard: finish,
    rebase, cleanup, check, reviewDecision, revert, push, dequeue: queuePosition !== undefined },
    ...local?.address.file ? { archive: local.address.file } : {},
    history,
    // A note keeps the commit it was written on, so the panel can say when the changes it described have moved.
    notes: (ticket.notes ?? []).map(note => ({ ...note,
      outdated: !!note.commit && !!ticket.review?.head && note.commit !== ticket.review.head })),
    ...ticket.autoMerge ? { autoMerge: ticket.autoMerge } : {},
    ...request ? { request } : {},
    related: (ticket.related ?? []).flatMap(id => {
      const other = store.tickets.get(id)
      return other ? [{ id, number: view.number(other.id), title: other.title }] : []
    }),
    blockedBy: (ticket.blockedBy ?? []).flatMap(id => {
      const other = store.tickets.get(id)
      return other ? [{ id, number: view.number(other.id), title: other.title, merged: !!other.merged }] : []
    }),
    ...checks ? { checks } : {},
    ...ticket.reviewed ? { reviewed: ticket.reviewed } : {},
    ...ticket.checkHistory?.length ? { checkHistory: ticket.checkHistory } : {},
    ...ticket.checkRuns?.length ? { checkRuns: ticket.checkRuns } : {},
    review: ticket.review,
    merged: ticket.merged, reverted, discarded: ticket.discarded, cleaned: ticket.cleaned,
    ...queuePosition !== undefined ? { queuePosition } : {},
    ...ticket.targetCheck ? { targetCheck: ticket.targetCheck } : {},
    ...ticket.mergeError ? { mergeError: ticket.mergeError } : {},
    mandate: { ...ticket.mandate, actor: view.actor(ticket.mandate.actor, human) },
    basis: basisFacts(ticket.mandate.basis, view.rule), paused: store.settings().paused,
    channels: ticket.channels.map(channel => channelLink(view, channel)),
    decision: decision ? decisionView(view, decision) : undefined,
    // Every started Assignment ran in the ticket's one worker Session. A running assignment whose worker reported a non-progress state has finished its turn unless a human response resumed the ticket.
    assignments: ticket.assignments.map(assignment => ({ ...assignment, ...assignment.startedAt ? { ref: workerSessionId(ticket.id) } : {},
      ...assignment.state === 'running' && assignment.report && assignment.report.state !== 'progress' && stage !== 'working'
        ? { turnEndedAt: assignment.report.at } : {} })),
    inputs: ticket.inputs.slice().reverse().map(input),
    // What the ticket sent to its Channel, which the panel otherwise shows only where the Source's own UI can.
    replies: store.receipts.ofTicket(id).map(receipt => ({ id: receipt.id, kind: receipt.kind, status: receipt.status,
      text: markdown(receipt.text), at: receipt.at ?? '', ...receipt.dropped ? { dropped: receipt.dropped } : {} })),
    // Threads followed for context only (e.g., related issues) grant no authority and are marked with `context: true`.
    threads: view.threadsOf(id).map(thread => ({ ...threadSummary(view, thread),
      ...thread.context?.includes(id) ? { context: true as const } : {} })) }
}
export type TicketDetailView = NonNullable<ReturnType<typeof ticketDetail>>

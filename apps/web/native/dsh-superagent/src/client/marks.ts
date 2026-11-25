/**
 * Pure mappers from domain states (Tickets, Threads, Sources, Decisions) to UI status indicators.
 *
 * A Ticket has one primary mark from exactly one of three families: a run state (`stage.queued`/`stage.working`/
 * `stage.idle`), a person wait (`stage.approval`/`answer`/`review`/`blocked`/`failed`/`delivery`), or a terminal outcome
 * (`outcome.merged`/`completedUnmerged`/`completedSettled`/`closed`/`stopped`/`dropped`). `ticketMark` picks exactly one, so the families
 * never overlap. Merge and push are separate axes: the merge landing is in the primary mark's hover detail, and the push
 * result is an independent `pushMark` tag shown beside it on every Ticket row, with the same wording on Trigger rows.
 */
import type { DecisionRow, SourceStatusRow, ThreadRow, TicketStage, TicketSummary } from './api.ts'
import type { DshSuperagentKey } from './locales.ts'
import { endedLabel, valueLabel, when, type T } from './text.ts'
import type { DotState, IconName, TagTone } from './controls.tsx'

/** A row's status indicator; `title` carries the meaning and any detail a person gets by hovering it. */
export interface Mark { label: string; tone: TagTone; dot: DotState; icon: IconName; title?: string }

/** Hover text: the mark's meaning first, then any recorded time, merge landing or push detail. */
function hint(...lines: (string | undefined)[]): string {
  return lines.filter(Boolean).join('\n')
}

/** What a Ticket mark reads: its stage, its ending, and where a recorded merge landed. */
export type TicketMarkInput = TicketStage & Pick<TicketSummary, 'ended'>
  & { mergeRequested?: boolean; merged?: { at: string; target?: string; branch?: string; head?: string; push?: { ok: boolean; error?: string } } }

export function ticketMark(t: T, ticket: TicketMarkInput): Mark {
  switch (ticket.stage) {
    // A merge is the Ticket's strongest end state, so it labels the row even when the work itself was reported completed first.
    case 'ended': {
      const ended = endedLabel(t, ticket.ended)
      if (ticket.merged || ticket.ended?.outcome === 'merged') {
        const merge = ticket.merged
        const where = merge?.target && merge.branch
          ? [t('mergedHint', { target: merge.target, branch: merge.branch }),
            ...merge.head ? [t('mergedCommit', { commit: merge.head.slice(0, 8) })] : []].join('\n')
          : ''
        return { label: t('outcome.merged'), tone: 'success', dot: 'done', icon: 'pull',
          title: hint(t('markMergedHint'), where, ticket.ended?.reason ? ended : undefined) }
      }
      if (ticket.ended?.outcome === 'completed') return ticket.mergeRequested
        ? { label: t('outcome.completedUnmerged'), tone: 'success', dot: 'done', icon: 'decision',
          title: hint(t('markCompletedMergeHint'), ticket.ended.at
            ? t('markCompletedTimeHint', { time: when(ticket.ended.at, t('lang')) }) : undefined) }
        : { label: t('outcome.completedSettled'), tone: 'success', dot: 'done', icon: 'decision',
          title: hint(t('markCompletedHint'), ticket.ended.at
            ? t('markCompletedTimeHint', { time: when(ticket.ended.at, t('lang')) }) : undefined) }
      // A person's stop ends the work for good; the dismiss icon and the word "abandoned" keep it apart from the
      // ordinary in-progress marks and from completed, merged and reverted work.
      if (ticket.ended?.outcome === 'stopped') return { label: t('outcome.stopped'), tone: 'neutral', dot: 'idle', icon: 'dismiss',
        title: hint(t('markAbandonedMeaningHint'), ticket.ended.at
          ? t('markAbandonedHint', { time: when(ticket.ended.at, t('lang')) }) : undefined,
        ticket.ended.reason ? ended : undefined) }
      const meaning = ticket.ended?.outcome === 'closed' ? t('markClosedHint')
        : ticket.ended?.outcome === 'dropped' ? t('markDroppedHint') : undefined
      return { label: valueLabel(t, 'outcome', ticket.ended?.outcome), tone: 'neutral', dot: 'idle', icon: 'stop',
        title: hint(meaning, ticket.ended?.reason ? ended : undefined) }
    }
    case 'person': {
      const action = ticket.action
      const icon: IconName = action === 'review' ? 'review' : action === 'failed' ? 'alert' : action === 'blocked' ? 'blocked'
        : action === 'delivery' ? 'inbox' : action === 'approval' ? 'decision' : 'ask'
      return { label: t(`stage.${action}`), tone: action === 'failed' ? 'danger' : 'warning', dot: action === 'failed' ? 'error'
        : 'warning', icon, title: t(`markStage.${action}Hint`) }
    }
    case 'working': return ticket.action === 'queued'
      ? { label: t('stage.queued'), tone: 'neutral', dot: 'idle', icon: 'queue', title: t('markQueuedHint') }
      : { label: t('stage.working'), tone: 'info', dot: 'ongoing', icon: 'running', title: t('markWorkingHint') }
    default: return { label: t('stage.idle'), tone: 'neutral', dot: 'idle', icon: 'clock', title: t('markIdleHint') }
  }
}
/** The push result is orthogonal to the ticket's own state, so it is shown as a separate mark beside it. */
export function pushMark(t: T, push: { ok: boolean; error?: string } | undefined): Mark | undefined {
  if (!push) return undefined
  return push.ok
    ? { label: t('pushed'), tone: 'success', dot: 'done', icon: 'inbox', title: hint(t('pushOk'), t('pushedHint')) }
    : { label: t('pushFailedShort'), tone: 'danger', dot: 'error', icon: 'alert',
      title: hint(t('pushFailedShortHint'), t('pushFailed', { error: push.error ?? '' })) }
}

export const reportDot = (report: string | undefined): DotState => report === 'failed' ? 'error'
  : report === 'blocked' || report === 'waiting' ? 'warning' : report === 'ready' ? 'done' : 'ongoing'

const THREAD_MARKS: Record<ThreadRow['stage'], Omit<Mark, 'label' | 'tone'> & { label: DshSuperagentKey }> = {
  error: { dot: 'error', icon: 'alert', label: 'threadPollFailed' },
  evaluating: { dot: 'ongoing', icon: 'running', label: 'threadEvaluating' },
  queued: { dot: 'warning', icon: 'queue', label: 'threadQueued' },
  subscribed: { dot: 'done', icon: 'inbox', label: 'threadSubscribed' },
  dormant: { dot: 'idle', icon: 'clock', label: 'threadDormantShort' },
  unsubscribed: { dot: 'idle', icon: 'hash', label: 'threadUnsubscribed' },
}
export function threadMark(t: T, thread: Pick<ThreadRow, 'stage' | 'pending'>): Omit<Mark, 'tone'> {
  const { dot, icon, label } = THREAD_MARKS[thread.stage]
  return { dot, icon, label: t(label, { count: thread.pending }) }
}

const SOURCE_MARKS: Record<SourceStatusRow['state'], { dot: DotState; label: DshSuperagentKey }> = {
  missing: { dot: 'idle', label: 'sourceNotInstalled' }, off: { dot: 'idle', label: 'sourceOff' },
  error: { dot: 'error', label: 'sourceState.error' }, setup: { dot: 'idle', label: 'sourceState.setup' },
  starting: { dot: 'ongoing', label: 'sourceState.ongoing' }, paused: { dot: 'warning', label: 'sourceState.warning' },
  ready: { dot: 'done', label: 'sourceState.done' },
}
export const sourceMark = (t: T, state: SourceStatusRow['state']) => ({ dot: SOURCE_MARKS[state].dot, label: t(SOURCE_MARKS[state].label) })

type Outcome = DecisionRow['outcome']
const DECISION_MARKS: Record<Outcome, Omit<Mark, 'label'>> = {
  accept: { dot: 'done', icon: 'decision', tone: 'success' }, reopen: { dot: 'done', icon: 'refresh', tone: 'success' },
  stop: { dot: 'idle', icon: 'stop', tone: 'danger' }, ignore: { dot: 'idle', icon: 'dismiss', tone: 'neutral' },
  unavailable: { dot: 'warning', icon: 'alert', tone: 'warning' }, wouldAccept: { dot: 'idle', icon: 'dismiss', tone: 'warning' },
}
/** Maps a Decision outcome to its visual mark, distinguishing `wouldAccept` (an ignore that observe mode would have accepted). */
export function decisionMark(t: T, outcome: Outcome): Mark {
  return { ...DECISION_MARKS[outcome], label: outcome === 'wouldAccept' ? t('wouldAccept') : valueLabel(t, 'decision', outcome) }
}

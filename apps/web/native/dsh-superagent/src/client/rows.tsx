/** Shared list rows for Tickets, Threads, and Decisions, plus Source fact chips and automated-message run folding. */
import { useState } from 'react'
import type { DecisionRow, ThreadFacts, ThreadRow, TicketFacts, TicketRow } from './api.ts'
import { Icon, Tag } from './controls.tsx'
import { decisionMark, threadMark, ticketMark } from './marks.ts'
import { usePanel } from './provider.tsx'
import { actorName, localized, readableMarkers, reasonLabel, ticketNumber, valueLabel } from './text.ts'
import { ActionLink, ConfirmButton, LabelValue, SessionButton, StageMark, When } from './widgets.tsx'

/** How many Ticket associations one row shows before folding the rest into a count. */
const TICKET_ASSOCIATIONS = 2

export function TicketLine({ ticket, question, onChanged, context, select }: {
  ticket: TicketRow; question?: boolean; onChanged?: () => Promise<void>; context?: boolean
  select?: { checked: boolean; onToggle: () => void }
}) {
  const { t, call, navigate } = usePanel()
  const status = ticketMark(t, ticket)
  const after = async (request: Promise<unknown>) => { await request; await onChanged?.() }
  const note = question && ticket.note && { ...ticket.note, text: readableMarkers(t, ticket.note.text) }
  const answer = ticket.reply && ticket.stage === 'person' && ticket.action === 'answer' ? ticket.reply : undefined
  return <li className="dt-row dt-link dt-ticket" data-state={question ? status.dot : undefined}
    data-select={select ? '' : undefined}>
    {select && <label className="dt-check"><input type="checkbox" checked={select.checked} onChange={select.onToggle}
      aria-label={t('selectTicket', { title: ticket.title })} /></label>}
    <div className="dt-ticket-stage"><StageMark state={status.dot} icon={status.icon} label={status.label} title={status.title} /></div>
    <div className="dt-ticket-body">
      <div className="dt-row-title">{ticket.number ? <span className="dt-ticket-number">{ticketNumber(ticket.number)}</span> : null}
        <button type="button" className="dt-open" onClick={() => navigate({ page: 'execution', id: ticket.id })}>{ticket.title}</button></div>
      <div className="dt-row-meta dt-ticket-meta">
        {ticket.tags.map(tag => <Tag key={tag} tone="quiet">{tag}</Tag>)}
        {ticket.threads.map(thread => <ThreadChip key={thread.key} thread={thread} />)}
        {context && <ContextTag />}
        {ticket.undelivered > 0 && <LabelValue label={t('ticketUndeliveredLabel')} value={ticket.undelivered} />}
        {note ? <span className={note.error ? 'dt-ticket-note dt-ticket-note-error' : 'dt-ticket-note'} title={note.text}>{note.text}</span>
          : <When at={ticket.updatedAt} />}
      </div>
      {/* Always render the actions container so every Ticket row keeps a uniform three-line height. */}
      <div className="dt-ticket-actions">{onChanged && <>
        {answer?.url && <ActionLink href={answer.url} icon="message">{t('answerIn', { source: valueLabel(t, 'source',
          answer.source) })}</ActionLink>}
        {ticket.actions.retry && <ConfirmButton variant="outline" icon="refresh" label={t('retry')} title={t('retryTitle')}
          message={t('retryConfirm', { title: ticket.title })}
          onConfirm={() => after(call('POST tickets/:id/retry', { id: ticket.id }))} />}
        {ticket.actions.retry && ticket.sessionId && <SessionButton variant="outline" sessionId={ticket.sessionId} />}
        {ticket.actions.stop && <ConfirmButton variant="outline" icon="stop" label={t('stop')} title={t('stopConfirmTitle')}
          message={t('stopConfirm', { title: ticket.title })}
          onConfirm={() => after(call('POST tickets/:id/stop', { id: ticket.id }))} />}
      </>}</div>
    </div>
  </li>
}

/** One Ticket a conversation started, with its stage mark and the remote's answer when the merge asked for a push. */
export function TicketAssociation({ ticket }: { ticket: TicketFacts }) {
  const { t, navigate } = usePanel()
  const mark = ticketMark(t, ticket)
  const push = ticket.merged?.push
  return <span className="dt-ticket-assoc">
    <button type="button" className="dt-textlink" title={mark.title}
      onClick={() => navigate({ page: 'execution', id: ticket.id })}>{ticketNumber(ticket.number)}</button>
    <Tag tone={mark.tone} title={mark.title}>{mark.label}</Tag>
    {push && (push.ok ? <Tag tone="success" title={t('pushOk')}>{t('pushed')}</Tag>
      : <Tag tone="danger" title={t('pushFailed', { error: push.error ?? '' })}>{t('pushFailedShort')}</Tag>)}
  </span>
}

function ContextTag() {
  const { t } = usePanel()
  return <Tag tone="quiet" title={t('contextOnlyHint')}>{t('contextOnly')}</Tag>
}
/** Thread list row; on a Ticket page, `ticketId` omits that Ticket from fanout targets and `context` marks context-only threads. */
export function ThreadLine({ thread, ticketId, context }: { thread: ThreadRow; ticketId?: string; context?: boolean }) {
  const { t, navigate } = usePanel()
  const shown = thread.tickets.filter(ticket => ticket.id !== ticketId)
  const { dot, icon, label } = threadMark(t, thread)
  return <li className="dt-row dt-link dt-ticket">
    <div className="dt-ticket-stage"><StageMark state={dot} icon={icon} label={label} /></div>
    <div className="dt-ticket-body">
      <div className="dt-row-title">
        <button type="button" className="dt-open" title={thread.pollError} onClick={() => navigate({ page: 'trigger',
          id: thread.key })}>{readableMarkers(t, thread.title)}</button>
      </div>
      <div className="dt-row-quote dt-line">
        {thread.latest && <>{actorName(t, thread.latest.actor)}{t('labelSeparator')}{readableMarkers(t, thread.latest.preview)
          || t('empty')}</>}
      </div>
      <div className="dt-row-meta dt-line">
        <ThreadChip thread={thread} />
        {context && <ContextTag />}
        {thread.pollError ? <span className="dt-line-text dt-error-text">{thread.pollError}</span>
          // On a Ticket's page its own Ticket is omitted; an empty list still means the Thread has one.
          : shown.length > 0 ? <span className="dt-ticket-assoc">
            <span className="dt-chip-label">{t('term.ticket')}</span>
            {shown.slice(0, TICKET_ASSOCIATIONS).map(ticket => <TicketAssociation key={ticket.id} ticket={ticket} />)}
            {shown.length > TICKET_ASSOCIATIONS && <span className="dt-line-text">{t('andMore', { count: shown.length - TICKET_ASSOCIATIONS })}</span>}
          </span>
            : thread.tickets.length > 0 ? null : <Tag tone="quiet" title={t('noTicketHint')}>{t('noTicket')}</Tag>}
        <When at={thread.lastAt} />
      </div>
    </div>
  </li>
}

export function DecisionLine({ decision }: { decision: DecisionRow }) {
  const { t, navigate } = usePanel()
  const matched = decision.rules.filter(rule => rule.matched).map(rule => rule.name)
  const mark = decisionMark(t, decision.outcome)
  const preview = readableMarkers(t, decision.preview)
  const reason = decision.reason ? reasonLabel(t, decision.reason) : ''
  const open = () => decision.ticketId ? navigate({ page: 'execution', id: decision.ticketId })
    : navigate({ page: 'trigger', id: decision.thread, query: { trigger: decision.triggerIds[0] ?? '' } })
  return <li className="dt-row dt-link dt-ticket">
    <div className="dt-ticket-stage"><StageMark state={mark.dot} icon={mark.icon} label={mark.label} /></div>
    <div className="dt-ticket-body">
      <div className="dt-row-title">
        <button type="button" className="dt-open" title={reason || undefined} onClick={open}>
          {decision.ticketTitle ?? (decision.threadTitle ? readableMarkers(t, decision.threadTitle) : preview || t('empty'))}
        </button>
      </div>
      <div className="dt-row-quote dt-line">{preview}</div>
      <div className="dt-row-meta dt-line">
        {reason && <span className="dt-line-text">{reason}</span>}
        {decision.ticket ? <TicketAssociation ticket={decision.ticket} />
          : <Tag tone="quiet" title={t('noTicketHint')}>{t('noTicket')}</Tag>}
        {matched.length > 0 && <LabelValue label={t('ruleLabel')} value={matched.join(t('listSeparator'))} />}
        <When at={decision.at} />
      </div>
    </div>
  </li>
}

function ThreadChip({ thread }: { thread: Pick<ThreadFacts, 'source' | 'facts' | 'url' | 'title'> }) {
  const { t } = usePanel()
  const cells = <>
    <span className="dt-chip-source">{valueLabel(t, 'source', thread.source)}</span>
    {Object.entries(thread.facts).map(([id, fact]) => <span key={id} className={fact.problem ? 'dt-chip-fact dt-chip-problem'
      : 'dt-chip-fact'}>
      {fact.label && <span className="dt-chip-label">{localized(t, fact.label)}</span>}{localized(t, fact.value)}
    </span>)}
  </>
  return thread.url ? <a className="dt-chip dt-chip-link" href={thread.url} target="_blank" rel="noopener noreferrer" title={thread.title}>
    {cells}<span className="dt-chip-open"><Icon name="external" size={11} /></span>
  </a> : <span className="dt-chip">{cells}</span>
}

type Run<I> = { item: I; earlier: I[] }
/** Group consecutive automated items from the same sender so only the newest is shown by default. */
export function automatedRuns<I>(items: I[], sender: (item: I) => string | undefined): Run<I>[] {
  const runs: Run<I>[] = []
  for (const item of items) {
    const last = runs.at(-1), from = sender(item)
    if (last && from !== undefined && sender(last.item) === from) last.earlier.push(item)
    else runs.push({ item, earlier: [] })
  }
  return runs
}
export function Earlier({ items }: { items: { id: string; at: string; preview: string }[] }) {
  const { t } = usePanel()
  const [open, setOpen] = useState(false)
  if (!items.length) return null
  return <div>
    <button type="button" className="dt-textlink" aria-expanded={open} onClick={() => setOpen(!open)}>
      {open ? t('collapse') : t('automatedEarlier', { count: items.length })}
    </button>
    {open && <ul className="dt-plain-list dt-hint">{items.map(item => <li key={item.id}><When
      at={item.at} /> · {readableMarkers(t, item.preview) || t('empty')}</li>)}</ul>}
  </div>
}

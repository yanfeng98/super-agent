/** Ticket detail page: stage and actions, Mandate details, undelivered/delivered Inputs, subscribed Threads, and Assignments. */
import { useState, type FormEvent, type ReactNode } from 'react'
import { ApiError, usePoll, type TicketPageView } from './api.ts'
import { Button, DisclosureRow, Dot, Icon, Input, Modal, Tag, type IconName } from './controls.tsx'
import { decisionMark, reportDot, ticketMark } from './marks.ts'
import { usePanel } from './provider.tsx'
import { settingsRoute } from './route.ts'
import { Earlier, ThreadLine, automatedRuns } from './rows.tsx'
import { actorName, errorText, localized, readableMarkers, reasonLabel, ticketNumber as ticketRef, valueLabel, when } from './text.ts'
import { ActionLink, ClampText, ConfirmButton, ExternalLink, Hint, KeyValues, LabelValue, LoadState, MarkdownLite, PageHead, Section,
  SessionButton, StageMark, When } from './widgets.tsx'

type Input = TicketPageView['inputs'][number]
function InputLine({ input, earlier = [] }: { input: Input; earlier?: { id: string; at: string; preview: string }[] }) {
  const { t, navigate } = usePanel()
  const trigger = input.kind === 'trigger' ? input.trigger : undefined
  return <li className="dt-row">
    <Dot state={input.delivered ? 'idle' : 'warning'} label={input.delivered ? t('inputDelivered') : t('inputUndelivered')} />
    <div className="dt-row-main">
      {trigger ? <div className="dt-row-title"><button type="button" className="dt-open" onClick={() => navigate({ page: 'trigger',
        id: trigger.thread, query: { trigger: trigger.id } })}>
        {actorName(t, trigger.actor)}{t('labelSeparator')}{readableMarkers(t, trigger.preview) || t('empty')}
      </button></div> : <div className="dt-row-title">{valueLabel(t, 'input', input.kind)}</div>}
      <div className="dt-row-meta">{input.kind === 'trigger' && <span>{valueLabel(t, 'input', input.kind)}</span>}<When
        at={input.at} /></div>
      {(input.kind === 'text' || input.kind === 'note') && input.text && <div className="dt-row-quote"><MarkdownLite text={input.text} /></div>}
      {trigger?.handback && <Hint>{t('inputHandedBack', { reason: trigger.handback.reason })}</Hint>}
      <Earlier items={earlier} />
    </div>
  </li>
}
function InputList({ inputs }: { inputs: Input[] }) {
  const sender = (input: Input) => input.kind === 'trigger' && input.trigger && !input.trigger.human
    ? `${input.trigger.actor.source}:${input.trigger.actor.id}` : undefined
  const shown = (input: Input) => ({ id: input.id, at: input.at, preview: input.kind === 'trigger' ? input.trigger?.preview ?? '' : '' })
  return <ul className="dt-list">{automatedRuns(inputs, sender).map(run => <InputLine key={run.item.id} input={run.item}
    earlier={run.earlier.map(shown)} />)}</ul>
}

/** Adds a person's message to a local ticket's Thread, which its worker reads as an Input and an ended ticket resumes on. */
function LocalReply({ thread, onDone }: { thread: string; onDone: () => Promise<void> }) {
  const { t, call, notify, navigate } = usePanel()
  const [text, setText] = useState('')
  const [busy, setBusy] = useState(false)
  const submit = async (event: FormEvent) => {
    event.preventDefault()
    setBusy(true)
    try {
      await call('POST local/followups', { body: { thread, text: text.trim() } })
      setText('')
      await onDone()
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error))
    } finally { setBusy(false) }
  }
  return <form className="dt-form" onSubmit={submit}>
    <label className="dt-field"><span className="dt-field-head">{t('localReplyLabel')}</span>
      <textarea className="dt-textarea" rows={3} required maxLength={20000} aria-label={t('localReplyLabel')} value={text}
        placeholder={t('localReplyPlaceholder')} onChange={event => setText(event.target.value)} /></label>
    <div className="dt-row-buttons"><Button variant="primary" type="submit" disabled={!text.trim() || busy}>{t('localReplySend')}</Button></div>
  </form>
}
function Details({ icon, title, open = false, children }: { icon: IconName; title: ReactNode; open?: boolean; children: ReactNode }) {
  const [shown, setShown] = useState(open)
  return <DisclosureRow icon={<Icon name={icon} />} title={title} open={shown} onToggle={() => setShown(!shown)}>
    <div className="dt-disclosure-body">{children}</div>
  </DisclosureRow>
}
function Assignments({ ticket }: { ticket: TicketPageView }) {
  const { t } = usePanel()
  const report = (state: string) => t('assignmentReportShort', { state: valueLabel(t, 'report', state) })
  return <ul className="dt-list">{ticket.assignments.slice().reverse().map((item, index) => <li key={item.id} className="dt-row">
    <Dot state={item.state === 'running' ? reportDot(item.report?.state) : 'idle'} label={valueLabel(t, 'assignment', item.state)} />
    <div className="dt-row-main">
      <div className="dt-row-title">{t('assignmentNumber', { number: ticket.assignments.length - index })}</div>
      <div className="dt-row-meta">
        {/* A running Assignment whose worker reported a non-progress state has ended its turn; show that report as its tag and end its span there. */}
        {item.turnEndedAt && item.report ? <Tag tone="neutral">{report(item.report.state)}</Tag> : <Tag tone={item.state === 'running'
          ? 'info'
          : 'neutral'}>{valueLabel(t, 'assignment', item.state)}</Tag>}
        {item.state !== 'running' && item.report && <span>{report(item.report.state)}</span>}
        <span>{t('assignmentSpan', { from: when(item.startedAt ?? item.createdAt, t('lang')), to: when(item.endedAt ?? item.turnEndedAt,
          t('lang')) || t('now') })}</span>
        {item.endedReason && <span>{reasonLabel(t, item.endedReason)}</span>}
      </div>
    </div>
    {item.ref && item.ref !== ticket.sessionId && <div className="dt-row-actions"><SessionButton variant="outline"
      sessionId={item.ref} /></div>}
  </li>)}</ul>
}
function MandateDetails({ ticket, actor }: { ticket: TicketPageView; actor: string }) {
  const { t, navigate } = usePanel()
  const { mandate, decision } = ticket
  const fact = (item: TicketPageView['basis']['facts'][number]): string => {
    if (item.name === 'legacyAuthority') return valueLabel(t, 'legacy', item.value)
    if (item.name === 'mode' || item.name === 'addressed') return valueLabel(t, item.name, item.value)
    if (item.name === 'admission') return valueLabel(t, 'admissionCell', item.value)
    if (item.name === 'rules') return item.value.map(rule => rule.name).join(t('listSeparator'))
    return item.name === 'sender' ? item.value : t('yes')
  }
  const { legacy } = ticket.basis
  // Omit `sender` when it matches `actor`, which is already shown on the status line.
  const facts = ticket.basis.facts.filter(item => !(item.name === 'sender' && item.value === actor))
  const direction = decision?.direction
  const rows: [string, ReactNode][] = decision ? [
    [t('term.trigger'), <button type="button" className="dt-trigger-link" onClick={() => navigate({ page: 'trigger', id: decision.thread,
      query: { trigger: decision.triggerIds[0] ?? '' } })}>
      <span className="dt-trigger-text">{readableMarkers(t, decision.preview) || t('empty')}</span>
      <span className="dt-row-meta"><span>{valueLabel(t, 'source', mandate.actor.source)} · {actor}</span><When at={decision.at} /></span>
    </button>],
    [t('term.decision'), <div className="dt-kv-stack">
      <span>{decisionMark(t, decision.outcome).label}</span>
      <Hint>{reasonLabel(t, decision.reason)}</Hint>
      {decision.rules.map(rule => <Hint key={rule.id}>{t(rule.matched ? 'ruleMatched' : 'ruleUnmatched', { rule: rule.name,
        reason: rule.reason })}</Hint>)}
      {direction && <Hint>{t('directionLine', { intent: t(direction.implementation_requested ? 'intentYes' : 'intentNo'),
        basis: valueLabel(t, 'targetBasis', direction.target_basis),
        design: t(direction.requires_design_decision ? 'designNeeded' : 'designNotNeeded') })} {t('directionEvidence',
          { evidence: direction.evidence })}</Hint>}
    </div>],
  ] : []
  const version = <button type="button" className="dt-textlink dt-mono" onClick={() => navigate(settingsRoute('eval',
    { version: mandate.policyVersion }))}>{mandate.policyVersion.slice(0, 8)}</button>
  return <Section title={t('mandateDetails')} icon="shield">
    <KeyValues rows={[...rows, [t('term.scope'), legacy ? t('legacyScopeShort') : mandate.scope || t('empty')],
      ...facts.map((item): [string, string] => [valueLabel(t, 'basis', item.name), fact(item)]),
      ...legacy ? [] : [[t('policyVersion'), version] as [string, ReactNode]]]} />
  </Section>
}

export function TicketPage({ id }: { id: string }) {
  const { t, call, notify, navigate } = usePanel()
  const { data, error, reload } = usePoll(() => call('GET tickets/:id', { id }), [id])
  const [diff, setDiff] = useState<{ diff: string; truncated: boolean; error?: string }>()
  const [diffBusy, setDiffBusy] = useState(false)
  const [checkBusy, setCheckBusy] = useState(false)
  const [relatedRef, setRelatedRef] = useState('')
  const [relatedBusy, setRelatedBusy] = useState(false)
  const [noteFile, setNoteFile] = useState('')
  const [noteLine, setNoteLine] = useState('')
  const [noteText, setNoteText] = useState('')
  const [notesBusy, setNotesBusy] = useState(false)
  const [reviewBusy, setReviewBusy] = useState(false)
  const [mergeStrategy, setMergeStrategy] = useState<'ff' | 'squash' | 'merge'>('ff')
  const [replyNoteId, setReplyNoteId] = useState<string>()
  const [replyDraft, setReplyDraft] = useState('')
  const [replyBusy, setReplyBusy] = useState(false)
  const [editingRequest, setEditingRequest] = useState(false)
  const [requestDraft, setRequestDraft] = useState('')
  const [requestBusy, setRequestBusy] = useState(false)
  const [renaming, setRenaming] = useState(false)
  const [titleDraft, setTitleDraft] = useState('')
  const [renamingBusy, setRenamingBusy] = useState(false)
  const [editingTags, setEditingTags] = useState(false)
  const [tagsDraft, setTagsDraft] = useState('')
  const [tagsBusy, setTagsBusy] = useState(false)
  const after = async (request: Promise<unknown>) => { await request; await reload() }
  const relate = async (reference: string, linked: boolean) => {
    setRelatedBusy(true)
    try { await after(call('POST tickets/:id/related', { id, body: { ticket: reference, linked } })) }
    catch (failure) { notify(errorText(failure)) }
    finally { setRelatedBusy(false) }
  }
  const addRelated = async (event: FormEvent) => {
    event.preventDefault()
    if (!relatedRef.trim()) return
    await relate(relatedRef.trim(), true)
    setRelatedRef('')
  }
  const runCheck = async () => {
    setCheckBusy(true)
    try { await after(call('POST tickets/:id/check', { id })) }
    catch (failure) { notify(errorText(failure)) }
    finally { setCheckBusy(false) }
  }
  const saveTags = async (event: FormEvent) => {
    event.preventDefault()
    setTagsBusy(true)
    try {
      const tags = [...new Set(tagsDraft.split(',').map(tag => tag.trim()).filter(Boolean))]
      await after(call('POST tickets/:id/tags', { id, body: { tags } }))
      setEditingTags(false)
    } catch (failure) { notify(errorText(failure)) }
    finally { setTagsBusy(false) }
  }
  const rename = async (event: FormEvent) => {
    event.preventDefault()
    setRenamingBusy(true)
    try {
      await call('POST tickets/:id/title', { id, body: { title: titleDraft.trim() } })
      setRenaming(false)
      await reload()
    } catch (failure) { notify(errorText(failure)) }
    finally { setRenamingBusy(false) }
  }
  const loadDiff = async () => {
    setDiffBusy(true)
    try { setDiff(await call('GET tickets/:id/diff', { id })) }
    catch (failure) { notify(errorText(failure)) }
    finally { setDiffBusy(false) }
  }
  if (!data) return <><PageHead
    title={t('term.ticket')} />{error instanceof ApiError && error.missing ? <Hint>{t('ticketNotFound')}</Hint> : <LoadState
      error={error} loading />}</>
  const status = ticketMark(t, data)
  const actor = actorName(t, data.mandate.actor)
  const { legacy } = data.basis
  const undelivered = data.inputs.filter(input => !input.delivered)
  const failedText = /^(?:\w+Error|Error)\b[:：]/.test(data.summary)
  const failed = data.stage === 'person' && data.action === 'failed'
  const opening = data.threads.find(thread => thread.url)
  const ended = data.ended && (() => {
    const outcome = valueLabel(t, 'outcome', data.ended.outcome), reason = reasonLabel(t, data.ended.reason), time = when(data.ended.at,
      t('lang'))
    return reason ? t('endedAt', { time, outcome, reason }) : t('endedAtShort', { time, outcome })
  })()
  const channel = data.channels[0]
  const review = data.review
  // Only the Ticket's own local Thread takes a reply; a context-only Thread belongs to another Ticket.
  const localThread = data.threads.find(thread => thread.source === 'local' && !thread.context)
  const notes = data.notes
  const addNote = async (event: FormEvent) => {
    event.preventDefault()
    setNotesBusy(true)
    try {
      const line = Number(noteLine)
      await call('POST tickets/:id/notes', { id, body: { file: noteFile.trim(), text: noteText.trim(),
        ...noteLine.trim() && Number.isInteger(line) && line > 0 ? { line } : {} } })
      setNoteFile(''); setNoteLine(''); setNoteText('')
      await reload()
    } catch (failure) { notify(errorText(failure)) }
    finally { setNotesBusy(false) }
  }
  const removeNote = async (noteId: string) => {
    setNotesBusy(true)
    try { await after(call('POST tickets/:id/notes/:noteId/remove', { id, noteId })) }
    catch (failure) { notify(errorText(failure)) }
    finally { setNotesBusy(false) }
  }
  const saveRequest = async (event: FormEvent) => {
    event.preventDefault()
    setRequestBusy(true)
    try {
      await after(call('POST tickets/:id/request', { id, body: { text: requestDraft.trim() } }))
      setEditingRequest(false)
    } catch (failure) { notify(errorText(failure)) }
    finally { setRequestBusy(false) }
  }
  const reviewChanges = async (decision: 'approved' | 'changes-requested') => {
    setReviewBusy(true)
    try { await after(call('POST tickets/:id/review', { id, body: { decision } })) }
    catch (failure) { notify(errorText(failure)) }
    finally { setReviewBusy(false) }
  }
  const replyToNote = async (event: FormEvent, noteId: string) => {
    event.preventDefault()
    setReplyBusy(true)
    try {
      await after(call('POST tickets/:id/notes/:noteId/replies', { id, noteId, body: { text: replyDraft.trim() } }))
      setReplyNoteId(undefined); setReplyDraft('')
    } catch (failure) { notify(errorText(failure)) }
    finally { setReplyBusy(false) }
  }
  const resolveNote = async (noteId: string, resolved: boolean) => {
    setNotesBusy(true)
    try { await after(call('POST tickets/:id/notes/:noteId/resolve', { id, noteId, body: { resolved } })) }
    catch (failure) { notify(errorText(failure)) }
    finally { setNotesBusy(false) }
  }
  const sendNotes = async () => {
    if (!localThread) return
    const text = `${t('noteMessage')}\n\n${notes.filter(note => !note.resolved).map(note => `- ${note.file}${note.line ? `:${note.line}` : ''}: ${note.text}`).join('\n')}`
    setNotesBusy(true)
    try { await after(call('POST local/followups', { body: { thread: localThread.key, text } })) }
    catch (failure) { notify(errorText(failure)) }
    finally { setNotesBusy(false) }
  }
  return <>
    <PageHead title={data.title}>
      {opening?.url && <ActionLink href={opening.url}>{t('openThread', { source: valueLabel(t, 'source', opening.source) })}</ActionLink>}
      {data.actions.retry && <ConfirmButton variant="outline" icon="refresh" label={t('retry')} title={t('retryTitle')}
        message={t('retryConfirm', { title: data.title })}
        onConfirm={() => after(call('POST tickets/:id/retry', { id }))} />}
      {data.actions.approve && <Button variant="primary" icon={<Icon name="decision" />}
        onClick={() => after(call('POST tickets/:id/approve', { id }))}>{t('approveTicket')}</Button>}
      <Button variant="outline" icon={<Icon name="settings" />} onClick={() => { setTitleDraft(data.title); setRenaming(true) }}>
        {t('renameTicket')}</Button>
      {data.actions.reopen && <ConfirmButton variant="outline" icon="refresh" label={t('reopen')} title={t('reopenTitle')}
        message={t('reopenConfirm', { title: data.title })}
        onConfirm={() => after(call('POST tickets/:id/reopen', { id }))} />}
      {data.sessionId && <SessionButton variant="outline" sessionId={data.sessionId} />}
    </PageHead>
    {error && <LoadState error={error} loading={false} />}
    <div className="dt-row-meta dt-status-line">
      <StageMark state={status.dot} icon={status.icon} label={status.label} />
      {data.number ? <LabelValue label={t('ticketNumberLabel')} value={ticketRef(data.number)} /> : null}
      {data.tags.map(tag => <Tag key={tag} tone="quiet">{tag}</Tag>)}
      <Button variant="ghost" size="sm" onClick={() => { setTagsDraft(data.tags.join(', ')); setEditingTags(true) }}>{t('tagsEdit')}</Button>
      <LabelValue label={t('ticketMandateLabel')} value={valueLabel(t, 'strength', data.mandate.strength)} />
      <LabelValue label={t('ticketActorLabel')} value={`${valueLabel(t, 'source', data.mandate.actor.source)} · ${actor}`} />
      <LabelValue label={t('ticketUpdatedLabel')} value={when(data.updatedAt, t('lang'))} />
    </div>
    {editingTags && <form className="dt-form" onSubmit={saveTags}>
      <div className="dt-row-meta">
        <Input value={tagsDraft} maxLength={200} aria-label={t('tagsEdit')} placeholder={t('newTicketTagsPlaceholder')}
          onChange={event => setTagsDraft(event.target.value)} />
        <Button variant="outline" type="submit" disabled={tagsBusy}>{t('tagsSave')}</Button>
        <Button variant="ghost" type="button" onClick={() => setEditingTags(false)}>{t('cancel')}</Button>
      </div>
    </form>}
    {ended && <Hint>{ended}</Hint>}
    {data.attention && <div className="dt-banner"><span className="dt-banner-label">{t('attentionPrefix')}</span><MarkdownLite
      text={data.attention.question} />
      {channel && <div className="dt-banner-actions">
        {channel.url ? <ActionLink href={channel.url} icon="message">{t('answerIn', { source: valueLabel(t, 'source', channel.source) })}</ActionLink>
          : <span className="dt-hint">{t('localReplyHint')}</span>}
      </div>}</div>}
    {/* Ordinary progress stays in the Session/Channels; only a report that halted work (blocked/failed) is shown here. */}
    {data.actions.retry && data.summary && <p className={`dt-banner${failed ? ' dt-danger' : ''}${failedText ? ' dt-mono' : ''}`}>
      <span className="dt-banner-label">{t(failed ? 'failedPrefix' : 'blockedPrefix')}</span>{data.summary}
    </p>}
    {data.deliveryError && <p className="dt-banner dt-danger dt-mono">
      <span className="dt-banner-label">{t('deliveryFailedPrefix')}</span>{data.deliveryError}
    </p>}
    {data.request && <Section title={t('requestTitle')} hint={t('requestHint')}>
      <div className="dt-row-meta"><When at={data.request.at} />
        {data.request.editedAt && <span>{t('requestEditedAt', { time: when(data.request.editedAt, t('lang')) })}</span>}</div>
      {editingRequest ? <form className="dt-form" onSubmit={saveRequest}>
        <textarea className="dt-textarea" rows={6} maxLength={20_000} value={requestDraft} aria-label={t('requestEdit')}
          onChange={event => setRequestDraft(event.target.value)} />
        <div className="dt-row-buttons">
          <Button variant="outline" type="submit" disabled={requestBusy || !requestDraft.trim()}>{t('requestSave')}</Button>
          <Button variant="ghost" type="button" onClick={() => setEditingRequest(false)}>{t('cancel')}</Button>
        </div>
      </form> : <>
        <ClampText text={data.request.text} />
        <div className="dt-row-actions">
          <Button variant="ghost" size="sm" onClick={() => { setRequestDraft(data.request!.text); setEditingRequest(true) }}>
            {t('requestEdit')}</Button>
        </div>
      </>}
    </Section>}
    {data.replies.length > 0 && <Section title={t('repliesTitle')} count={data.replies.length} hint={t('repliesHint')}>
      <ul className="dt-list">{data.replies.slice().reverse().map(reply => <li key={reply.id} className="dt-row">
        <div className="dt-row-main">
          <div className="dt-row-title">{valueLabel(t, 'reply', reply.kind)}</div>
          <div className="dt-row-meta">
            {reply.status !== 'sent' && <Tag tone={reply.status === 'dropped' ? 'danger' : 'neutral'}>{valueLabel(t, 'replyStatus',
              reply.status)}</Tag>}
            {reply.at && <When at={reply.at} />}
            {reply.dropped && <span>{reasonLabel(t, reply.dropped.reason)}</span>}
          </div>
          <div className="dt-row-quote"><MarkdownLite text={reply.text} /></div>
        </div>
      </li>)}</ul>
    </Section>}
    {localThread && <Section title={t('localReplyTitle')} hint={t('localReplyHint')}>
      <LocalReply thread={localThread.key} onDone={reload} />
    </Section>}
    {review && <Section title={t('reviewTitle')} hint={t('reviewHint')}>
      {review.error && <Hint>{review.error}</Hint>}
      <KeyValues rows={[[t('reviewBranch'), <span className="dt-mono">{review.branch ?? t('empty')}</span>],
        [t('reviewBase'), <span className="dt-mono">{review.base?.slice(0, 12) ?? t('empty')}</span>],
        [t('reviewHead'), <span className="dt-mono">{review.head?.slice(0, 12) ?? t('empty')}</span>],
        ...review.stat ? [[t('reviewDiffstat'), <span className="dt-mono">{review.stat}</span>] as [string, ReactNode]] : []]} />
      {review.commits.length > 0 && <>
        <p className="dt-hint">{t('reviewCommits', { count: review.commits.length })}</p>
        <ul className="dt-plain-list dt-mono">{review.commits.map((commit, index) => <li key={`${commit.sha}:${index}`}>{commit.sha} {commit.subject}</li>)}</ul>
      </>}
      {review.files.length > 0 && <>
        <p className="dt-hint">{t('reviewFiles', { count: review.files.length })}</p>
        <ul className="dt-plain-list dt-mono">{review.files.map((file, index) => <li key={`${file.path}:${index}`}>{file.status} {file.path}</li>)}</ul>
      </>}
      {review.dirty.length > 0 && <>
        <p className="dt-hint">{t('reviewDirty', { count: review.dirty.length })}</p>
        <ul className="dt-plain-list dt-mono">{review.dirty.map((file, index) => <li key={`${file.path}:${index}`}>{file.status} {file.path}</li>)}</ul>
      </>}
      <div className="dt-row-actions">
        <Button variant="outline" size="sm" onClick={loadDiff} disabled={diffBusy}>{t('reviewViewDiff')}</Button>
      </div>
      {diff?.error && <Hint>{diff.error}</Hint>}
      {diff && !diff.error && (diff.diff ? <pre className="dt-workspace-diff">{diff.diff}</pre> : <Hint>{t('reviewDiffEmpty')}</Hint>)}
      {diff?.truncated && <Hint>{t('reviewDiffTruncated')}</Hint>}
      {data.actions.reviewDecision && <div className="dt-row-actions">
        <Button variant={data.reviewed?.decision === 'approved' ? 'primary' : 'outline'} disabled={reviewBusy}
          onClick={() => reviewChanges('approved')}>{t('reviewApprove')}</Button>
        <Button variant={data.reviewed?.decision === 'changes-requested' ? 'primary' : 'outline'} disabled={reviewBusy}
          onClick={() => reviewChanges('changes-requested')}>{t('reviewRequestChanges')}</Button>
        {data.reviewed && <Tag tone={data.reviewed.decision === 'approved' ? 'success' : 'warning'}>
          {t(`review.${data.reviewed.decision}`)}</Tag>}
      </div>}
      {data.reviewed?.note && <Hint>{data.reviewed.note}</Hint>}
      <div className="dt-form">
        {notes.some(note => !note.resolved) && <Hint>{t('notesUnresolved', { count: notes.filter(note => !note.resolved).length })}</Hint>}
        {notes.length > 0 && <ul className="dt-list">{notes.map(note => <li className="dt-row" key={note.id}>
          <div className="dt-row-main">
            <div className="dt-row-meta"><span className="dt-mono">{note.file}{note.line ? `:${note.line}` : ''}</span><When at={note.at} />
              {note.resolved && <Tag tone="success">{t('noteResolvedTag')}</Tag>}</div>
            <div className="dt-row-quote">{note.text}</div>
            {note.replies?.map(reply => <div className="dt-row-quote" key={reply.id}>
              <span className="dt-hint">{t('noteReply')} · <When at={reply.at} /></span>
              <div>{reply.text}</div>
            </div>)}
            {replyNoteId === note.id && <form className="dt-form" onSubmit={event => replyToNote(event, note.id)}>
              <Input value={replyDraft} maxLength={2000} aria-label={t('noteReply')} placeholder={t('noteReplyPlaceholder')}
                onChange={event => setReplyDraft(event.target.value)} />
              <div className="dt-row-buttons">
                <Button variant="outline" type="submit" disabled={replyBusy || !replyDraft.trim()}>{t('noteReplySend')}</Button>
                <Button variant="ghost" type="button" onClick={() => setReplyNoteId(undefined)}>{t('cancel')}</Button>
              </div>
            </form>}
          </div>
          <div className="dt-row-actions">
            <Button variant="ghost" size="sm" disabled={notesBusy}
              onClick={() => { setReplyNoteId(note.id); setReplyDraft('') }}>{t('noteReply')}</Button>
            <Button variant="ghost" size="sm" disabled={notesBusy}
              onClick={() => resolveNote(note.id, !note.resolved)}>{note.resolved ? t('noteReopen') : t('noteResolve')}</Button>
            <Button variant="ghost" size="sm" disabled={notesBusy}
              onClick={() => removeNote(note.id)}>{t('remove')}</Button>
          </div>
        </li>)}</ul>}
        <form className="dt-form" onSubmit={addNote}>
          <div className="dt-row-meta">
            <Input className="dt-mono" value={noteFile} placeholder={t('noteFilePlaceholder')} aria-label={t('noteFilePlaceholder')}
              onChange={event => setNoteFile(event.target.value)} />
            <Input className="dt-mono" type="number" min={1} value={noteLine} placeholder={t('noteLinePlaceholder')}
              aria-label={t('noteLinePlaceholder')} onChange={event => setNoteLine(event.target.value)} />
          </div>
          <textarea className="dt-textarea" rows={2} maxLength={2000} value={noteText} placeholder={t('noteTextPlaceholder')}
            aria-label={t('noteTextPlaceholder')} onChange={event => setNoteText(event.target.value)} />
          <div className="dt-row-buttons">
            <Button variant="outline" type="submit" disabled={notesBusy || !noteFile.trim() || !noteText.trim()}>{t('noteAdd')}</Button>
            {localThread && notes.some(note => !note.resolved)
              && <Button variant="primary" disabled={notesBusy} onClick={sendNotes}>{t('noteSend')}</Button>}
          </div>
        </form>
      </div>
      {(data.actions.rebase || data.actions.merge || data.actions.discard) && <div className="dt-row-actions">
        {data.actions.rebase && <ConfirmButton variant="outline" icon="refresh" label={t('ticketRebase')} title={t('rebaseTitle')}
          message={t('rebaseConfirm', { title: data.title })} onConfirm={() => after(call('POST tickets/:id/rebase', { id }))} />}
        {data.actions.merge && <>
          <label className="dt-field-inline"><span className="dt-hint">{t('mergeStrategy')}</span>
            <select className="dt-select" aria-label={t('mergeStrategy')} value={mergeStrategy}
              onChange={event => setMergeStrategy(event.target.value as 'ff' | 'squash' | 'merge')}>
              <option value="ff">{t('merge.ff')}</option>
              <option value="squash">{t('merge.squash')}</option>
              <option value="merge">{t('merge.merge')}</option>
            </select></label>
          <ConfirmButton variant="outline" icon="decision" label={t('ticketMerge')} title={t('mergeTitle')}
            message={t('mergeConfirm', { title: data.title })}
            onConfirm={() => after(call('POST tickets/:id/merge', { id, query: { strategy: mergeStrategy } }))} />
        </>}
        {data.actions.discard && <ConfirmButton variant="outline" icon="trash" label={t('ticketDiscard')} title={t('discardTitle')}
          message={t('discardConfirm', { title: data.title })} onConfirm={() => after(call('POST tickets/:id/discard', { id }))} />}
      </div>}
    </Section>}
    {(data.checks || data.actions.check) && <Section title={t('checksTitle')} hint={t('checksHint')}>
      {data.checks ? <>
        <KeyValues rows={[[t('statusTitle'), <span className="dt-inline-link"><Dot state={data.checks.ok ? 'done' : 'error'}
          label={data.checks.ok ? t('checksPassed') : t('checksFailedUnknown')} />{data.checks.ok ? t('checksPassed')
          : data.checks.exitCode === null ? t('checksFailedUnknown') : t('checksFailed', { code: data.checks.exitCode })}</span>],
          [t('checkCommand'), <span className="dt-mono">{data.checks.command}</span>],
          [t('checksAt'), when(data.checks.at, t('lang'))]]} />
        {data.checks.output && <pre className="dt-workspace-diff">{data.checks.output}</pre>}
        {data.checks.truncated && <Hint>{t('checksTruncated')}</Hint>}
      </> : <Hint>{t('checksNotRun')}</Hint>}
      {data.checkHistory && data.checkHistory.length > 1 && <Details icon="queue"
        title={`${t('checksHistory')} · ${data.checkHistory.length}`}>
        <ul className="dt-list">{data.checkHistory.slice().reverse().map((run, index) => <li className="dt-row" key={`${run.at}:${index}`}>
          <div className="dt-row-main">
            <div className="dt-row-meta"><Dot state={run.ok ? 'done' : 'error'} label={run.ok ? t('checksPassed') : t('checksFailedUnknown')} />
              <When at={run.at} /><span className="dt-mono">{run.command}</span></div>
          </div>
        </li>)}</ul>
      </Details>}
      {data.actions.check && <div className="dt-row-actions">
        <Button variant="outline" size="sm" disabled={checkBusy} onClick={runCheck}>{t('checksRun')}</Button>
      </div>}
    </Section>}
    {data.merged && <Hint>{t('mergedHint', { branch: data.merged.branch, target: data.merged.target })}</Hint>}
    {data.actions.cleanup && <div className="dt-row-actions">
      <ConfirmButton variant="outline" icon="trash" label={t('cleanupWorktree')} title={t('cleanupTitle')}
        message={t('cleanupConfirm', { title: data.title })} onConfirm={() => after(call('POST tickets/:id/cleanup', { id }))} />
    </div>}
    {data.cleaned && <Hint>{t('cleanedHint')}</Hint>}
    {data.discarded && <Hint>{t('discardedHint')}</Hint>}
    {data.history.length > 0 && <Section title={t('historyTitle')} hint={t('historyHint')}>
      <ul className="dt-list">{data.history.map(item => <li className="dt-row" key={`${item.code}:${item.at}`}>
        <div className="dt-row-main">
          <div className="dt-row-title">{t(`history.${item.code}`)}</div>
          <div className="dt-row-meta"><When at={item.at} />{item.title && <span>{item.title}</span>}
            {item.tags && <span>{item.tags.join(', ') || t('empty')}</span>}
            {item.decision && <span>{t(`review.${item.decision}`)}</span>}
            {item.note && <span>{item.note}</span>}
            {item.branch && <span className="dt-mono">{item.branch} → {item.target}</span>}</div>
        </div>
      </li>)}</ul>
    </Section>}
    <MandateDetails ticket={data} actor={actor} />
    {undelivered.length > 0 && <Section title={t('undeliveredInputs')} count={undelivered.length}>
      <Hint>{data.deliveryError ? t('inputRetryHint') : data.paused ? t('inputPausedHint') : t('inputWaitingHint')}</Hint>
      <InputList inputs={undelivered} />
    </Section>}
    {data.threads.length > 0 && <Section title={t('subscribedThreads')} count={data.threads.length}>
      <ul className="dt-list">{data.threads.map(thread => <ThreadLine key={thread.key} thread={thread} ticketId={data.id}
        context={thread.context} />)}</ul>
    </Section>}
    <div className="dt-details">
      {/* A single Assignment adds nothing beyond the status line; list Assignments only once work has been resumed or handed over. */}
      {data.assignments.length > 1 && <Details icon="queue" title={`${t('term.assignment')} · ${data.assignments.length}`} open><Assignments
        ticket={data} /></Details>}
      {data.environment.length > 0 && <Details icon="settings" title={t('term.environment')}>
        <table className="dt-table"><tbody>{data.environment.map(variable => <tr key={variable.key} title={localized(t,
          variable.description)}>
          <td className="dt-mono">{variable.key}</td><td className="dt-mono">{variable.value || <span
            className="dt-hint">{t('empty')}</span>}</td>
        </tr>)}</tbody></table>
      </Details>}
      <Details icon="message" title={t('term.channel')}>
        <ul className="dt-plain-list">{data.channels.map((item, index) => <li key={index}>
          {item.url ? <ExternalLink href={item.url} icon={item.source === 'github' ? 'pull' : 'message'}>{localized(t, item.label)}</ExternalLink>
            : <span className="dt-inline-link dt-icon-link"><Icon name="message" size={13} />{localized(t, item.label)}</span>}
        </li>)}</ul>
      </Details>
      {data.inputs.length > undelivered.length && <Details icon="inbox" title={t('deliveredInputs',
        { count: data.inputs.length - undelivered.length })}>
        <InputList inputs={data.inputs.filter(input => input.delivered)} />
      </Details>}
      <Details icon="hash" title={t('relatedTitle')}>
        <Hint>{t('relatedHint')}</Hint>
        {data.related.length > 0 && <ul className="dt-list">{data.related.map(item => <li className="dt-row" key={item.id}>
          <div className="dt-row-main"><div className="dt-row-title">{item.number
            ? <span className="dt-ticket-number">{ticketRef(item.number)}</span> : null}
            <button type="button" className="dt-open" onClick={() => navigate({ page: 'execution', id: item.id })}>{item.title}</button>
          </div></div>
          <div className="dt-row-actions"><Button variant="ghost" size="sm" disabled={relatedBusy}
            onClick={() => relate(item.id, false)}>{t('remove')}</Button></div>
        </li>)}</ul>}
        <form className="dt-form" onSubmit={addRelated}>
          <div className="dt-row-meta">
            <Input className="dt-mono" value={relatedRef} placeholder={t('relatedPlaceholder')} aria-label={t('relatedPlaceholder')}
              onChange={event => setRelatedRef(event.target.value)} />
            <Button variant="outline" type="submit" disabled={!relatedRef.trim() || relatedBusy}>{t('relatedAdd')}</Button>
          </div>
        </form>
      </Details>
      <Details icon="hash" title={t('identifiers')}>
        <KeyValues rows={[[t('ticketId'), <span className="dt-mono">{data.id}</span>], [t('executionDirectory'), <span
          className="dt-mono">{data.directory ?? t('empty')}</span>],
          ...data.archive ? [[t('archiveFile'), <span className="dt-mono">{data.archive}</span>] as [string, ReactNode]] : [],
          ...legacy && data.mandate.scope ? [[t('legacyAssignment'), <span
            className="dt-mono dt-clamp">{data.mandate.scope}</span>] as [string, ReactNode]] : []]} />
      </Details>
    </div>
    {renaming && <Modal open onClose={() => setRenaming(false)} title={t('renameTicketTitle')} closeLabel={t('close')}
      description={t('renameTicketHint')} footer={<><Button variant="outline" onClick={() => setRenaming(false)}>{t('cancel')}</Button>
        <Button variant="primary" type="submit" form="dt-rename-ticket" disabled={!titleDraft.trim() || renamingBusy}>{t('save')}</Button></>}>
      <form id="dt-rename-ticket" className="dt-form" onSubmit={rename}>
        <label className="dt-field"><span className="dt-field-head">{t('newTicketTitle')}</span>
          <Input value={titleDraft} maxLength={120} required aria-label={t('newTicketTitle')}
            onChange={event => setTitleDraft(event.target.value)} /></label>
      </form>
    </Modal>}
    {data.actions.stop && <div className="dt-danger-zone">
      <span className="dt-hint">{t('stopHint')}</span>
      <ConfirmButton variant="outline" label={t('stop')} title={t('stopConfirmTitle')} message={t('stopConfirm', { title: data.title })}
        onConfirm={() => after(call('POST tickets/:id/stop', { id }))} />
    </div>}
  </>
}

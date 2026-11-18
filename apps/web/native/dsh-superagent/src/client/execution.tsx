/** Execution status view (active Tickets and pause control) and Execution settings group (Executor capacity and workspace environment). */
import { useRef, useState, type FormEvent } from 'react'
import { TICKET_FILTERS, type TicketFilter } from '../app/api.ts'
import { LIMITS } from '../limits.ts'
import { usePoll, type SettingsView, type SourceView, type StateView } from './api.ts'
import { Button, DisclosureRow, Icon, InfoTip, Input, Modal, Pill } from './controls.tsx'
import { SaveBar } from './draft.tsx'
import { usePanel } from './provider.tsx'
import { enableNotifications, notificationsEnabled, notificationsSupported } from './notify.ts'
import { TicketLine } from './rows.tsx'
import { ConfigHistory, Field, Fields, ModelField, numberOf, useBlockDraft, useSettings } from './settings-kit.tsx'
import { localized } from './text.ts'
import { ConfirmButton, Hint, LabelValue, LoadMore, LoadState, Section, StatusHead } from './widgets.tsx'

/** Reads a picked file as base64, without the data-URL prefix. */
const readAttachment = (file: File): Promise<string> => new Promise((resolve, reject) => {
  const reader = new FileReader()
  reader.onload = () => { const result = String(reader.result ?? ''); resolve(result.slice(result.indexOf(',') + 1)) }
  reader.onerror = () => reject(reader.error ?? new Error('Could not read the attachment'))
  reader.readAsDataURL(file)
})
const requestKey = (): string => globalThis.crypto?.randomUUID?.() ?? `local-${Date.now()}-${Math.random().toString(16).slice(2)}`
/** Creates a local ticket: the request is archived as Markdown and its Trigger starts a Ticket. */
function NewTicket({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t, call, navigate, notify } = usePanel()
  const [title, setTitle] = useState('')
  const [project, setProject] = useState('')
  const [tags, setTags] = useState('')
  const [text, setText] = useState('')
  const [files, setFiles] = useState<File[]>([])
  const [hold, setHold] = useState(false)
  const [busy, setBusy] = useState(false)
  const fileInput = useRef<HTMLInputElement>(null)
  // One key per unchanged draft, so a retry after a lost response returns the ticket the first attempt made.
  const idempotency = useRef<string | undefined>(undefined)
  const submit = async (event: FormEvent) => {
    event.preventDefault()
    setBusy(true)
    idempotency.current ??= requestKey()
    try {
      const labels = [...new Set(tags.split(',').map(tag => tag.trim()).filter(Boolean))]
      if (files.reduce((total, file) => total + file.size, 0) > 8 * 1024 * 1024) throw new Error(t('attachmentsTooLarge'))
      const attachments = await Promise.all(files.map(async file => ({ name: file.name, data: await readAttachment(file) })))
      const created = await call('POST local/requests', { body: { title: title.trim(), text: text.trim(), key: idempotency.current,
        ...project.trim() ? { project: project.trim() } : {}, ...labels.length ? { tags: labels } : {},
        ...attachments.length ? { files: attachments } : {}, ...hold ? { hold: true } : {} } })
      setTitle(''); setProject(''); setTags(''); setText(''); setFiles([]); setHold(false); idempotency.current = undefined
      if (fileInput.current) fileInput.current.value = ''
      setBusy(false); onClose()
      navigate({ page: 'trigger', id: created.thread, query: { trigger: created.trigger } })
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error))
      setBusy(false)
    }
  }
  const ready = !!title.trim() && !!text.trim() && !busy
  return <Modal open={open} onClose={onClose} title={t('newTicket')} closeLabel={t('close')} description={t('newTicketHint')}
    footer={<><Button variant="outline" onClick={onClose}>{t('cancel')}</Button>
      <Button variant="primary" type="submit" form="dt-new-ticket" disabled={!ready}>{t('createTicket')}</Button></>}>
    <form id="dt-new-ticket" className="dt-form" onSubmit={submit}>
      <label className="dt-field"><span className="dt-field-head">{t('newTicketTitle')}</span>
        <Input value={title} maxLength={120} required aria-label={t('newTicketTitle')} placeholder={t('newTicketTitlePlaceholder')}
          onChange={event => { idempotency.current = undefined; setTitle(event.target.value) }} /></label>
      <label className="dt-field"><span className="dt-field-head">{t('newTicketProject')}</span>
        <Input value={project} maxLength={60} aria-label={t('newTicketProject')} placeholder={t('newTicketProjectPlaceholder')}
          onChange={event => { idempotency.current = undefined; setProject(event.target.value) }} /></label>
      <label className="dt-field"><span className="dt-field-head">{t('newTicketTags')}</span>
        <Input value={tags} maxLength={200} aria-label={t('newTicketTags')} placeholder={t('newTicketTagsPlaceholder')}
          onChange={event => { idempotency.current = undefined; setTags(event.target.value) }} /></label>
      <label className="dt-check"><input type="checkbox" checked={hold}
        onChange={event => { idempotency.current = undefined; setHold(event.target.checked) }} /> {t('newTicketHold')}</label>
      <label className="dt-field"><span className="dt-field-head">{t('newTicketFiles')}</span>
        <input ref={fileInput} type="file" multiple className="dt-file" aria-label={t('newTicketFiles')}
          onChange={event => { idempotency.current = undefined; setFiles([...event.target.files ?? []].slice(0, 5)) }} />
        <Hint>{t('newTicketFilesHint')}</Hint></label>
      <label className="dt-field"><span className="dt-field-head">{t('newTicketText')}</span>
        <textarea className="dt-textarea" rows={6} maxLength={20000} required aria-label={t('newTicketText')}
          placeholder={t('newTicketTextPlaceholder')} value={text} onChange={event => { idempotency.current = undefined; setText(event.target.value) }} /></label>
    </form>
  </Modal>
}
export function ExecutionStatus({ filter: current, state, reload }: {
  filter: TicketFilter; state: StateView; reload: () => Promise<void>
}) {
  const { t, call, navigate, notify } = usePanel()
  const [limit, setLimit] = useState(20)
  const [tag, setTag] = useState('')
  const [creating, setCreating] = useState(false)
  const [notifying, setNotifying] = useState(notificationsEnabled)
  const enableNotify = async () => {
    if (await enableNotifications()) setNotifying(true)
    else notify(t('notificationsDenied'))
  }
  const { data, error, reload: reloadTickets } = usePoll(() => call('GET tickets', {
    query: { state: current, limit, ...tag ? { tag } : {} } }), [current, limit, tag])
  const { data: tagData } = usePoll(() => call('GET tags'), [])
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const { paused, load: { maxWorkers } } = state
  // Pausing uses its own block revision so toggling pause does not invalidate an open Execution settings draft.
  const pause = async () => { await call('POST executor/pause', { body: { paused: !paused } }); await reload() }
  const status = state.setup.problem ? { tone: 'warning', label: t('executorWaitingSetup') } : paused ? { tone: 'warning',
    label: t('processingPaused') }
    : { tone: 'ongoing', label: t('executorRunning') }
  const action = paused ? 'resumeProcessing' : 'pauseProcessing'
  const toggle = (id: string) => setSelected(current => {
    const next = new Set(current)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    return next
  })
  const chosen = data?.items.filter(item => selected.has(item.id)) ?? []
  const stopSelected = chosen.filter(item => item.state === 'open').map(item => item.id)
  const cleanupSelected = chosen.filter(item => item.actions.cleanup).map(item => item.id)
  const runBulk = async (verb: 'stop' | 'cleanup', ids: string[]) => {
    const request: (id: string) => Promise<unknown> = verb === 'stop' ? (id: string) => call('POST tickets/:id/stop', { id })
      : (id: string) => call('POST tickets/:id/cleanup', { id })
    const results = await Promise.allSettled(ids.map(request))
    const failed = results.filter(result => result.status === 'rejected').length
    notify(failed ? t('bulkFailed', { failed, total: ids.length }) : t('bulkDone', { count: ids.length }))
    setSelected(new Set())
    await reload()
    await reloadTickets()
  }
  return <>
    <StatusHead hint={t('executionStatusIntro')} settings="execution" />
    <div className="dt-row-meta dt-status-line">
      <span className="dt-status-text" data-state={status.tone}>{status.label}</span>
      {!state.setup.problem && <>
        <LabelValue label={t('executingLabel')}
          value={`${state.load.running} / ${maxWorkers}`} /><LabelValue label={t('queuedLabel')} value={state.load.queued} />
      </>}
      <Button variant="outline" icon={<Icon name="plus" />} onClick={() => setCreating(true)}>{t('newTicket')}</Button>
      {notificationsSupported() && !notifying && <Button variant="outline" icon={<Icon name="message" />}
        onClick={enableNotify}>{t('notificationsEnable')}</Button>}
      {notifying && <span className="dt-hint">{t('notificationsOn')}</span>}
      <ConfirmButton variant="outline" icon={paused ? 'play' : 'pause'} label={t(action)} title={t(action)} message={t(paused
        ? 'resumeProcessingConfirm' : 'pauseProcessingConfirm')}
        onConfirm={pause} />
    </div>
    {!!tagData?.tags.length && <div className="dt-row-meta dt-status-line">
      <label className="dt-field-inline"><span className="dt-hint">{t('tagFilter')}</span>
        <select className="dt-select" aria-label={t('tagFilter')} value={tag}
          onChange={event => { setLimit(20); setTag(event.target.value) }}>
          <option value="">{t('tagFilterAll')}</option>
          {tagData.tags.map(item => <option key={item} value={item}>{item}</option>)}
        </select></label>
    </div>}
    <div className="dt-filters">
      {TICKET_FILTERS.map(value => <Pill key={value} active={current === value}
        onClick={() => { setLimit(20); navigate({ page: 'execution', query: value === 'open' ? {} : { filter: value } },
          true) }}>{t(`ticketFilter.${value}`)}</Pill>)}
    </div>
    {current === 'idle' && <Hint>{t('idleHint')}</Hint>}
    <LoadState error={error} loading={!data} />
    {data && <>
      {chosen.length > 0 && <div className="dt-row-meta dt-status-line">
        <span className="dt-hint">{t('selectedCount', { count: chosen.length })}</span>
        <Button variant="ghost" size="sm" onClick={() => setSelected(new Set(data.items.map(item => item.id)))}>{t('selectAll')}</Button>
        <Button variant="ghost" size="sm" onClick={() => setSelected(new Set())}>{t('clearSelection')}</Button>
        {stopSelected.length > 0 && <ConfirmButton variant="outline" icon="stop" label={t('bulkStop', { count: stopSelected.length })}
          title={t('bulkStop')} message={t('bulkStopConfirm', { count: stopSelected.length })}
          onConfirm={() => runBulk('stop', stopSelected)} />}
        {cleanupSelected.length > 0 && <ConfirmButton variant="outline" icon="trash" label={t('bulkCleanup', { count: cleanupSelected.length })}
          title={t('bulkCleanup')} message={t('bulkCleanupConfirm', { count: cleanupSelected.length })}
          onConfirm={() => runBulk('cleanup', cleanupSelected)} />}
      </div>}
      {data.items.length ? <ul className="dt-list">{data.items.map(ticket => ticket.stage === 'person'
        ? <TicketLine key={ticket.id} ticket={ticket} question onChanged={reloadTickets}
          select={{ checked: selected.has(ticket.id), onToggle: () => toggle(ticket.id) }} />
        : <TicketLine key={ticket.id} ticket={ticket}
          select={{ checked: selected.has(ticket.id), onToggle: () => toggle(ticket.id) }} />)}</ul> : <Hint>{t('none')}</Hint>}
    </>}
    {data?.next_offset != null && <LoadMore onMore={() => setLimit(limit + 20)} />}
    <NewTicket open={creating} onClose={() => setCreating(false)} />
  </>
}

const firstSentence = (text: string): string => /^[^。；;.]*(?:[。；;]|\.(?=\s|$))?/.exec(text)?.[0] ?? text
function EnvironmentVariables({ groups, sources,
  onPick }: { groups: SettingsView['environment']['variables']; sources: SourceView[]; onPick: (key: string) => void }) {
  const { t } = usePanel()
  const [open, setOpen] = useState(false)
  const owner = (id: string) => id === 'executor' ? t('term.executor') : localized(t, sources.find(source => source.id === id)?.title ?? id)
  const count = groups.reduce((sum, group) => sum + group.variables.length, 0)
  return <DisclosureRow icon={<Icon name="hash" />} title={<>{t('environmentVariables')} <span className="dt-count">{count}</span></>}
    open={open} onToggle={() => setOpen(!open)}>
    <div className="dt-disclosure-body">
      <Hint>{t('environmentPick')}</Hint>
      <table className="dt-table">
        <thead><tr><th>{t('environmentKey')}</th><th>{t('environmentDescription')}</th><th>{t('environmentOwner')}</th></tr></thead>
        <tbody>{groups.flatMap(group => group.variables.map(variable => <tr key={variable.key}>
          <td><button type="button" className="dt-textlink dt-mono" onClick={() => onPick(variable.key)}>{variable.key}</button></td>
          <td title={localized(t, variable.description)}>{firstSentence(localized(t, variable.description))}</td>
          <td>{owner(group.owner)}{!group.enabled && <span className="dt-hint"> · {t('sourceOff')}</span>}</td>
        </tr>))}</tbody>
      </table>
    </div>
  </DisclosureRow>
}
export function ExecutionSettings() {
  const { t } = usePanel()
  const { blocks, environment, sources } = useSettings().settings
  // Keep raw user text (such as separators between blocked commands) in the draft and parse only on save.
  const executor = blocks.executor.value
  const saved = { ...executor, maxWorkers: String(executor.maxWorkers), blockedCommands: executor.blockedCommands.join(' ') }
  const draft = useBlockDraft('executor', saved, blocks.executor.revision,
    value => ({ ...value, maxWorkers: numberOf(value.maxWorkers), blockedCommands: value.blockedCommands.split(/[\s,]+/).filter(Boolean) }))
  const value = draft.value
  const edit = (next: Partial<typeof value>) => draft.edit({ ...value, ...next })
  const prepare = useRef<HTMLTextAreaElement>(null)
  const insert = (key: string) => {
    const field = prepare.current
    const at = field?.selectionStart ?? value.prepare.length, to = field?.selectionEnd ?? at
    edit({ prepare: `${value.prepare.slice(0, at)}"$${key}"${value.prepare.slice(to)}` })
    requestAnimationFrame(() => { field?.focus(); const caret = at + key.length + 3; field?.setSelectionRange(caret, caret) })
  }
  return <div className="dt-form">
    <form className="dt-form" onSubmit={event => { void draft.submit(event) }}>
      <Fields disabled={draft.busy}>
        <div className="dt-form dt-block">
          <h2 className="dt-block-title">{t('term.executor')}<InfoTip text={t('executorIntro')} /></h2>
          <Section title={t('executorModel')} hint={t('executorModelHint')}><ModelField id="dt-worker-model" value={value.model}
            onChange={model => edit({ model })} /></Section>
          <Field label={t('maxWorkers')} hint={t('maxWorkersHint')}>
            <Input type="number" min={LIMITS.maxWorkers.min} max={LIMITS.maxWorkers.max} required value={value.maxWorkers}
              onChange={event => edit({ maxWorkers: event.target.value })} />
          </Field>
        </div>
        <div className="dt-form dt-block">
          <h2 className="dt-block-title">{t('term.environment')}<InfoTip text={t('environmentIntro')} /></h2>
          <Field label={t('executionDirectory')} hint={t('executionDirectoryHint')} note={value.directory ? undefined
            : t('executionDirectoryDefault', { root: environment.root })}>
            <Input className="dt-mono" value={value.directory} placeholder={t('executionDirectoryEmpty')} spellCheck={false} pattern="/.*"
              onChange={event => edit({ directory: event.target.value })} />
          </Field>
          <Field label={t('prepareCommand')} hint={t('prepareCommandHint')}>
            <textarea ref={prepare} className="dt-textarea dt-mono" rows={3} maxLength={LIMITS.prepareCommand} spellCheck={false}
              placeholder={t('prepareCommandEmpty')} value={value.prepare}
              onChange={event => edit({ prepare: event.target.value })} />
          </Field>
          <Field label={t('checkCommand')} hint={t('checkCommandHint')}>
            <textarea className="dt-textarea dt-mono" rows={2} maxLength={LIMITS.prepareCommand} spellCheck={false}
              placeholder={t('checkCommandEmpty')} value={value.check}
              onChange={event => edit({ check: event.target.value })} />
          </Field>
          <EnvironmentVariables groups={environment.variables} sources={sources} onPick={insert} />
          <Field label={t('blockedCommands')} hint={t('blockedCommandsHint')}>
            <Input className="dt-mono" value={value.blockedCommands} placeholder={t('blockedCommandsNone')} spellCheck={false}
              onChange={event => edit({ blockedCommands: event.target.value })} />
          </Field>
        </div>
      </Fields>
      <SaveBar {...draft} onDiscard={draft.discard} />
    </form>
    <ConfigHistory blocks={['executor', 'pause']} />
  </div>
}

/** Trigger status view listing Threads filtered by subscription and evaluation activity. */
import { useState } from 'react'
import { THREAD_FILTERS, type ThreadFilter } from '../app/api.ts'
import { usePoll, type ThreadRow } from './api.ts'
import { Pill } from './controls.tsx'
import { usePanel } from './provider.tsx'
import { ThreadLine } from './rows.tsx'
import { Hint, LoadMore, LoadState, Section, StatusHead } from './widgets.tsx'

export function TriggerStatus({ filter }: { filter: ThreadFilter }) {
  const { t, call, navigate } = usePanel()
  const [limit, setLimit] = useState(20)
  const { data, error } = usePoll(() => call('GET threads', { query: { filter, limit } }), [filter, limit])
  const go = (next: ThreadFilter) => { setLimit(20); navigate({ page: 'trigger', query: next !== 'subscribed' ? { filter: next } : {} },
    true) }
  const list = (threads: ThreadRow[]) => <ul className="dt-list">{threads.map(thread => <ThreadLine key={thread.key} thread={thread} />)}</ul>
  // The host already orders unfinished Threads first; splitting on the same fact keeps that grouping visible when a page of
  // the list is read on its own.
  const unfinished = data?.items.filter(thread => thread.unfinished) ?? []
  const others = data?.items.filter(thread => !thread.unfinished) ?? []
  return <>
    <StatusHead hint={t('triggerIntro')} />
    <div className="dt-filters">
      {THREAD_FILTERS.map(value => <Pill key={value} active={filter === value}
        onClick={() => go(value)}>{t(`threadFilter.${value}`)}</Pill>)}
    </div>
    <LoadState error={error} loading={!data} />
    {data && (data.items.length ? (unfinished.length > 0 ? <>
      <Section title={t('threadsUnfinished')} count={unfinished.length} hint={t('threadsUnfinishedHint')}>{list(unfinished)}</Section>
      {others.length > 0 && <Section title={t('threadsOther')} count={others.length}>{list(others)}</Section>}
    </> : list(others)) : <Hint>{t('threadsNone')}</Hint>)}
    {data?.next_offset != null && <LoadMore onMore={() => setLimit(limit + 20)} />}
  </>
}

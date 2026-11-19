// @vitest-environment jsdom
// The Execution status, a Ticket, and the Execution settings.
import { act } from 'react'
import { expect, it } from 'vitest'
import type { TicketPageView } from '../src/client/api.ts'
import { en, zh } from '../src/client/locales.ts'
import { usePanel, mount, remount, all, find, tab, click, type, text, field, at } from './fixtures/panel.tsx'
import { MEMBER, server, ticketSummary, threadSummary } from './fixtures/panel-server.ts'

usePanel()
it('shows unread notifications in the header and opens their Ticket', async () => {
  server.notifications = { unread: 1, items: [{ id: 'n1', at: '2026-11-18T00:00:00Z', kind: 'attention', ticketId: 't1',
    title: '修复回归', text: '需要确认按钮颜色' }] }
  await mount()
  const bell = document.querySelector('button[aria-label="通知"]') as HTMLButtonElement
  expect(bell.textContent).toContain('1')
  await click(bell)
  const dialog = document.querySelector('[role="dialog"]')!
  expect(dialog.textContent).toContain('需要确认按钮颜色')
  await click([...dialog.querySelectorAll('.dt-open')].find(item => item.textContent === '修复回归') as HTMLElement)
  expect(server.requests.some(request => request.path === 'notifications/n1/read' && request.method === 'POST')).toBe(true)
  expect(server.requests.some(request => request.path === 'tickets/t1')).toBe(true)
})

it('lists Tickets under Execution by who they wait on, with the Executor state and its pause as an action', async () => {
  server.tickets.open = [ticketSummary(), ticketSummary({ id: 't2', title: '执行中的工单', threads: [{ key: 'github:7',
    source: 'github', title: '#7 x', facts: { number: { value: '#7' }, autoMerge: { label: { zh: '自动合并', en: 'Auto-merge' },
      value: { zh: '已开启', en: 'on' } } } }] })]
  await mount()
  await click(zh['tab.execution'])
  expect(document.querySelector('.dt-status-title')?.textContent).toBe(zh.statusTitle)
  expect(server.requests.some(request => request.path === 'tickets?state=open&limit=20')).toBe(true)
  expect(text()).toContain(zh.executorRunning)
  expect(all('.dt-chip').map(chip => chip.textContent)).toContain('GitHub#7自动合并已开启')
  await click(zh.idleLabel)
  expect(server.requests.some(request => request.path === 'tickets?state=idle&limit=20')).toBe(true)
  expect(text()).toContain('执行器结束了当前回合')
  await click(zh['ticket.ended'])
  expect(text()).toContain('旧工单')
  expect(text()).toContain(zh['outcome.stopped'])
  expect(all('.dt-status-line .dt-lv').map(item => item.textContent)).toEqual(['执行中 2 / 3', '排队 0'])
  // The pause is its own action: an Execution settings draft started before it still saves.
  await click(find(zh['tab.settings'], '.dt-status-head button'))
  expect(tab(zh['tab.settings']).getAttribute('aria-selected')).toBe('true')
  expect(document.querySelector('.dt-settings-link[aria-current="page"]')?.textContent).toBe(zh['promptPart.runtime/work'])
  await type(field<HTMLInputElement>(zh.maxWorkers), '4')
  await click(tab(zh['tab.execution']))
  await click(zh.pauseProcessing)
  await click(zh.confirm)
  expect(server.requests.find(request => request.path === 'executor/pause')).toMatchObject({ method: 'POST', body: { paused: true } })
  expect(text()).toContain(zh.processingPaused)
  expect(find(zh.resumeProcessing).textContent).toBe(zh.resumeProcessing)
  await click(tab(zh['tab.settings']))
  await click(zh.save)
  expect(server.settings).toMatchObject({ paused: true, executor: { maxWorkers: 4 } })
})

it('shows the merge queue with its pause and takes a Ticket out of it', async () => {
  server.mergeQueue = { paused: true, items: [{ ticket_id: 't1', number: 1, title: '修复回归', position: 1 },
    { ticket_id: 't2', number: 2, title: '另一个工单', position: 2, strategy: 'squash' }] }
  await mount()
  await click(zh['tab.execution'])
  expect(server.requests.some(request => request.path === 'merge-queue')).toBe(true)
  expect(text()).toContain(zh.mergeQueueTitle)
  expect(text()).toContain(zh.mergeQueuePaused)
  expect(text()).toContain('T-0001 修复回归')
  expect(text()).toContain('squash')
  // The queue resumes as one action, and an entry leaves it on its own.
  await click(zh.mergeQueueResume)
  expect(server.requests.find(request => request.path === 'merge-queue/pause')).toMatchObject({ method: 'POST', body: { paused: false } })
  // The first entry's own way out, which names it on its row.
  await click(zh.queueCancel)
  expect(server.requests.find(request => request.path === 'tickets/t1/dequeue')).toMatchObject({ method: 'POST' })
})

it('filters Tickets by a tag', async () => {
  server.tickets.open = [ticketSummary({ id: 't1', tags: ['前端'] }), ticketSummary({ id: 't2', title: '另一个工单', tags: [] })]
  await mount()
  await click(zh['tab.execution'])
  await type(field<HTMLSelectElement>(zh.tagFilter), '前端')
  expect(server.requests.some(request => request.path.includes('tag=') && decodeURIComponent(request.path).includes('tag=前端'))).toBe(true)
})

it('edits the labels on a Ticket from its page', async () => {
  at({ page: 'execution', id: 't1' })
  server.ticketDetails.t1 = { ...server.ticketDetails.t1!, tags: ['前端'] }
  await mount()
  expect(text()).toContain('前端')
  await click(zh.tagsEdit)
  await type(field<HTMLInputElement>(zh.tagsEdit), '前端, p1')
  await click(zh.tagsSave)
  expect(server.requests.find(request => request.path === 'tickets/t1/tags')).toMatchObject({ method: 'POST',
    body: { tags: ['前端', 'p1'] } })
})

it('approves a held ticket from its page', async () => {
  at({ page: 'execution', id: 't1' })
  server.ticketDetails.t1 = { ...server.ticketDetails.t1!, stage: 'person', action: 'approval',
    actions: { ...server.ticketDetails.t1!.actions, approve: true } } as TicketPageView
  await mount()
  expect(text()).toContain(zh['stage.approval'])
  await click(zh.approveTicket)
  expect(server.requests.some(request => request.path === 'tickets/t1/approve' && request.method === 'POST')).toBe(true)
})

it('stops an open Ticket from its page after confirmation', async () => {
  await mount()
  await click('修复回归')
  await click(zh.stop)
  expect(server.requests.some(request => request.path.endsWith('/stop'))).toBe(false)
  expect(document.querySelector('[role="dialog"]')?.textContent).toContain('停止“修复回归”')
  await click(zh.confirm)
  expect(server.requests.some(request => request.path === 'tickets/t1/stop' && request.method === 'POST')).toBe(true)
})

it('stops or cleans up the selected Tickets from the Execution list in one action', async () => {
  server.tickets.open = [ticketSummary(), ticketSummary({ id: 't2', title: '执行中的工单' })]
  server.tickets.ended = [ticketSummary({ id: 't3', title: '旧工单', number: 9, state: 'ended',
    actions: { retry: false, stop: false, cleanup: true } }, { stage: 'ended' })]
  await mount()
  await click(zh['tab.execution'])
  const boxes = all('input[type="checkbox"]')
  expect(boxes).toHaveLength(2)
  for (const box of boxes) await click(box)
  await click(all('button').find(button => button.textContent === zh.bulkStop.replace('{count}', '2'))!)
  await click(zh.confirm)
  expect(server.requests.filter(request => request.method === 'POST' && request.path.endsWith('/stop')).map(request => request.path))
    .toEqual(['tickets/t1/stop', 'tickets/t2/stop'])
  // Selection clears after the run, so the next bulk action starts from an empty set.
  expect(text()).not.toContain(zh.selectedCount.replace('{count}', '2'))
  await click(zh['ticket.ended'])
  const ended = all('input[type="checkbox"]')
  await click(ended[0]!)
  const cleanup = all('button').find(button => button.textContent === zh.bulkCleanup.replace('{count}', '1'))
  await click(cleanup!)
  await click(zh.confirm)
  expect(server.requests.some(request => request.method === 'POST' && request.path === 'tickets/t3/cleanup')).toBe(true)
})

it('sets labels on the selected Tickets from the Execution list', async () => {
  server.tickets.open = [ticketSummary(), ticketSummary({ id: 't2', title: '执行中的工单' })]
  await mount()
  await click(zh['tab.execution'])
  for (const box of all('input[type="checkbox"]')) await click(box)
  const label = zh.bulkTags.replace('{count}', '2')
  await type(field<HTMLInputElement>(label), '前端, p1')
  await click(label)
  expect(server.requests.filter(request => request.path.endsWith('/tags')).map(request => request.body))
    .toEqual([{ tags: ['前端', 'p1'] }, { tags: ['前端', 'p1'] }])
  expect(server.requests.filter(request => request.path.endsWith('/tags')).map(request => request.path))
    .toEqual(['tickets/t1/tags', 'tickets/t2/tags'])
})

it('shows a Ticket with its stage, its Mandate details with the Trigger and Decision behind it, its Threads, and the rest behind disclosures', async () => {
  await mount()
  await click(all('.dt-open').find(button => button.textContent === '修复回归')!)
  expect(document.querySelector('.dt-page-title h2')?.textContent).toBe('修复回归')
  // Progress is in the Channels and the Session, so the page has no report section; replies come from the outbox receipts.
  expect(all('.dt-section-title').map(title => title.querySelector('.dt-section-name')?.textContent)).toEqual(['原始请求', '回复与结果', '授权详情',
    '未送达的输入', '订阅的线程'])
  expect(document.querySelector('.dt-status-line .dt-stage')?.textContent).toBe(zh.executingLabel)
  expect(document.querySelector('.dt-status-line .dt-stage .dt-spinner')).not.toBeNull()
  // A Thread this Ticket follows reads as subscribed on its page, though no other Ticket follows it.
  expect(all('.dt-list .dt-ticket .dt-stage').map(stage => stage.textContent)).toContain(zh.threadSubscribed)
  expect(all('.dt-list .dt-ticket .dt-stage').map(stage => stage.textContent)).not.toContain(zh.threadUnsubscribed)
  expect(all('.dt-status-line .dt-lv').map(item => item.textContent)).toEqual(['授权 明确', '发起 飞书 · Alice', expect.stringMatching(/^更新 /)])
  for (const expected of ['修复登录页回归', '点名方式@ 智能体', '名单里的人明确请求，由主 Session 接单', '未送达：等待派工运行',
    'Alice：顺便改标题']) expect(text()).toContain(expected)
  expect(all('.dt-chip').map(chip => chip.textContent)).toContain('飞书团队群')
  await click(zh['term.channel'])
  expect(all('.dt-disclosure-body a').map(link => [link.textContent, link.getAttribute('href')])).toEqual([['飞书消息',
    'https://applink.feishu.cn/client/chat/open?openChatId=oc_1'], ['GitHub #42', 'https://github.com/acme/widgets/issues/42']])
  await click(zh['term.environment'])
  expect(all('.dt-disclosure-body .dt-table tr').map(row => row.textContent)).toEqual(['DSH_SUPERAGENT_DIR/w/t1',
    'DSH_SUPERAGENT_GITHUB_NUMBER—'])
  await click(document.querySelector<HTMLElement>('.dt-trigger-link')!)
  expect(tab(zh['tab.trigger']).getAttribute('aria-selected')).toBe('true')
})

it('shows the full request body on a Ticket page', async () => {
  at({ page: 'execution', id: 't1' })
  server.ticketDetails.t1 = { ...server.ticketDetails.t1!,
    request: { text: `短预览\n\n${'细节 '.repeat(100)}结尾标记`, at: '2026-09-22T00:03:00Z' } }
  await mount()
  expect(text()).toContain('结尾标记')
})

it('links and unlinks related Tickets from a Ticket page', async () => {
  at({ page: 'execution', id: 't1' })
  server.ticketDetails.t1 = { ...server.ticketDetails.t1!, related: [{ id: 't2', number: 2, title: '相关工单' }] }
  await mount()
  await click(zh.relatedTitle)
  expect(text()).toContain('T-0002')
  await click(all('button').find(button => button.textContent === zh.remove)!)
  expect(server.requests.find(request => request.path === 'tickets/t1/related')).toMatchObject({ method: 'POST',
    body: { ticket: 't2', linked: false } })
  await type(field<HTMLInputElement>(zh.relatedPlaceholder), 'T-0003')
  await click(zh.relatedAdd)
  expect(server.requests.filter(request => request.path === 'tickets/t1/related').at(-1)).toMatchObject({ method: 'POST',
    body: { ticket: 'T-0003', linked: true } })
})

it('blocks a ticket on another ticket\'s merge from its page', async () => {
  at({ page: 'execution', id: 't1' })
  server.ticketDetails.t1 = { ...server.ticketDetails.t1!,
    blockedBy: [{ id: 't2', number: 2, title: '前置工单', merged: false }] }
  await mount()
  await click(zh.blockedByTitle)
  expect(text()).toContain('T-0002')
  expect(text()).toContain(zh.blockerWaiting)
  await click(all('button').find(button => button.textContent === zh.remove)!)
  expect(server.requests.find(request => request.path === 'tickets/t1/blockers')).toMatchObject({ method: 'POST',
    body: { ticket: 't2', blocked: false } })
  await type(field<HTMLInputElement>(zh.blockedByPlaceholder), 'T-0003')
  await click(zh.blockedByAdd)
  expect(server.requests.filter(request => request.path === 'tickets/t1/blockers').at(-1)).toMatchObject({ method: 'POST',
    body: { ticket: 'T-0003', blocked: true } })
})

it('adds a line-anchored review note and sends the notes to the executor as one follow-up', async () => {
  at({ page: 'execution', id: 't1' })
  server.ticketDetails.t1 = { ...server.ticketDetails.t1!,
    review: { capturedAt: '2026-11-18T00:00:00Z', branch: 'agent/t1', base: 'a'.repeat(40), head: 'b'.repeat(40), commits: [],
      files: [], dirty: [] },
    notes: [{ id: 'n1', file: 'src/a.ts', line: 12, text: '改成可选', at: '2026-11-18T00:00:00Z', outdated: true,
      commit: 'c'.repeat(40) }],
    threads: [{ ...threadSummary(), key: 'local:r1', source: 'local', title: '本地线程' }] }
  await mount()
  expect(text()).toContain('src/a.ts:12')
  // A note written on an older commit is marked, so an approval of the current changes is not mistaken for it.
  expect(text()).toContain(zh.noteOutdated)
  await type(field<HTMLInputElement>(zh.noteFilePlaceholder), 'src/b.ts')
  await type(field<HTMLInputElement>(zh.noteLinePlaceholder), '4')
  await type(field<HTMLTextAreaElement>(zh.noteTextPlaceholder), '补个测试')
  await click(zh.noteAdd)
  expect(server.requests.find(request => request.path === 'tickets/t1/notes')).toMatchObject({ method: 'POST',
    body: { file: 'src/b.ts', line: 4, text: '补个测试' } })
  await click(zh.noteSend)
  const followup = server.requests.filter(request => request.path === 'local/followups').at(-1)
  expect(followup?.body).toMatchObject({ thread: 'local:r1' })
  expect(String((followup?.body as { text?: string }).text)).toContain('src/a.ts:12: 改成可选')
  await click(zh.noteReply)
  await type(field<HTMLInputElement>(zh.noteReply), '已经处理')
  await click(zh.noteReplySend)
  expect(server.requests.find(request => request.path === 'tickets/t1/notes/n1/replies')).toMatchObject({ method: 'POST',
    body: { text: '已经处理' } })
  await click(zh.noteResolve)
  expect(server.requests.find(request => request.path === 'tickets/t1/notes/n1/resolve')).toMatchObject({ method: 'POST',
    body: { resolved: true } })
})

it('edits the request text from the Ticket page', async () => {
  at({ page: 'execution', id: 't1' })
  await mount()
  await click(zh.requestEdit)
  await type(field<HTMLTextAreaElement>(zh.requestEdit), '新的请求正文')
  await click(zh.requestSave)
  expect(server.requests.find(request => request.path === 'tickets/t1/request')).toMatchObject({ method: 'POST',
    body: { text: '新的请求正文' } })
})

it('shows the review captured when a ticket ended and lets a person reopen it', async () => {
  const fields = { id: 't5', title: '记录结束', number: 7, state: 'ended' as const, actions: { retry: false, stop: false },
    ended: { at: '2026-11-18T00:00:00Z', by: 'human' as const, outcome: 'stopped' as const } }
  server.tickets.ended = [ticketSummary(fields, { stage: 'ended' }), ...server.tickets.ended]
  server.ticketDetails.t5 = { ...server.ticketDetails.t1!, ...fields, stage: 'ended', actions: { retry: false, stop: false, reopen: true, approve: false, merge: true, discard: true, rebase: true, cleanup: false, check: true,
      reviewDecision: true, revert: false, push: false, dequeue: false },
    checks: { command: 'pnpm test', ok: false, exitCode: 1, at: '2026-11-18T00:00:00Z', output: '1 test failed' },
    history: [{ code: 'renamed', at: '2026-11-18T00:01:00Z', title: '新的标题', via: 'chat' },
      { code: 'reviewed', at: '2026-11-18T00:01:20Z', decision: 'approved', by: 'operator' },
      { code: 'rebased', at: '2026-11-18T00:01:30Z', branch: 'agent/t5', target: 'main', via: 'panel' },
      { code: 'merged', at: '2026-11-18T00:02:00Z', branch: 'agent/t5', target: 'main', via: 'auto',
        head: 'c'.repeat(40), push: { ok: false, error: 'no upstream' } }],
    review: { capturedAt: '2026-11-18T00:00:00Z', branch: 'agent/t5', base: 'a'.repeat(40), head: 'b'.repeat(40),
      commits: [{ sha: 'b'.repeat(7), subject: '修复窄屏' }], files: [{ status: 'M', path: 'src/a.ts' }], dirty: [] } }
  await mount()
  await click(zh['tab.execution'])
  await click(zh['ticket.ended'])
  await click('记录结束')
  expect(text()).toContain('T-0007')
  expect(text()).toContain('agent/t5')
  expect(text()).toContain('修复窄屏')
  expect(text()).toContain('src/a.ts')
  // The configured check's failure is visible on the page with its output.
  expect(text()).toContain('pnpm test')
  expect(text()).toContain('1 test failed')
  expect(text()).toContain(zh.checksRun)
  expect(text()).toContain(zh['history.rebased'])
  // The record says which interface carried each action out, and keeps the commit and the remote's answer.
  expect(text()).toContain(zh['via.chat'])
  expect(text()).toContain(zh['via.auto'])
  expect(text()).toContain('c'.repeat(8))
  expect(text()).toContain('no upstream')
  expect(text()).toContain(zh['history.merged'])
  expect(text()).toContain('评审人：operator')
  expect(text()).toContain('agent/t5 → main')
  await click(zh.renameTicket)
  await type(field<HTMLInputElement>(zh.newTicketTitle), '新的标题')
  await click(zh.save)
  expect(server.requests.find(request => request.path === 'tickets/t5/title')).toMatchObject({ method: 'POST',
    body: { title: '新的标题' } })
  await click(zh.reviewViewDiff)
  // The diff is listed file by file, each with its counts, opened on demand and marked reviewed.
  expect(text()).toContain('src/a.ts')
  expect(text()).toContain('+1 −1')
  expect(text()).not.toContain('+new')
  await click('src/a.ts')
  expect(text()).toContain('+new')
  await click(zh.diffViewed)
  expect(Object.keys(localStorage).some(key => key.startsWith('dsh-superagent/viewed/t5/'))).toBe(true)
  await click(zh.reviewApprove)
  expect(server.requests.find(request => request.path === 'tickets/t5/review')).toMatchObject({ method: 'POST',
    body: { decision: 'approved' } })
  await click(zh.ticketMerge)
  await click(zh.confirm)
  expect(server.requests.some(request => request.path.startsWith('tickets/t5/merge') && request.method === 'POST'
    && request.path.includes('strategy=ff'))).toBe(true)
  await click(zh.ticketDiscard)
  await click(zh.confirm)
  expect(server.requests.some(request => request.path === 'tickets/t5/discard' && request.method === 'POST')).toBe(true)
  await click(zh.reopen)
  await click(zh.confirm)
  expect(server.requests.some(request => request.path === 'tickets/t5/reopen' && request.method === 'POST')).toBe(true)
})

it('reports a missing Ticket instead of rendering an empty page', async () => {
  at({ page: 'execution', id: 'missing' })
  await mount()
  expect(document.querySelector('.dt-hint')?.textContent).toBe(zh.ticketNotFound)
})

it('links a question to its Channel, folds consecutive automated messages among Inputs, and shows field explanations in a hover tooltip',
  async () => {
  const ci = (id: string, at: string, failed: number) => ({ id, kind: 'trigger' as const, triggerId: `c${id}`, at, delivered: true,
    trigger: { id: `c${id}`, actor: { source: 'github', id: '' }, human: false,
      preview: `CI of PR #42 at abc1234: 10 checks, ${failed} failed, 0 running.`,
      body: `CI of PR #42 at abc1234: 10 checks, ${failed} failed, 0 running.`, thread: 'github:42' } })
  const t1 = server.ticketDetails.t1!
  server.ticketDetails.t1 = { ...t1, stage: 'person', action: 'answer', attention: { id: 'q1', question: '需要确认按钮颜色' }, inputs: [ci('i3',
    '2026-09-22T00:06:00Z', 1), ci('i2', '2026-09-22T00:05:00Z', 0)] }
  await mount()
  await click(all('.dt-open').find(button => button.textContent === '修复回归')!)
  expect(all('.dt-banner a').map(link => [link.textContent, link.getAttribute('href')])).toEqual([['去飞书回答',
    'https://applink.feishu.cn/client/chat/open?openChatId=oc_1']])
  await click('已送达的输入 · 2')
  expect(all('.dt-disclosure-body .dt-row-title').map(title => title.textContent)).toEqual(['自动通知：CI of PR #42 at abc1234: 10 checks, 1 failed, 0 running.'])
  await click('同一发送者之前还有 1 条自动消息')
  expect(text()).toContain('CI of PR #42 at abc1234: 10 checks, 0 failed, 0 running.')
  await click(zh['tab.eval'])
  await click(find(zh['tab.settings'], '.dt-status-head button'))
  expect(text()).not.toContain('到期后记为未完成')
  const tip = field(zh.evalTimeout).closest('.dt-field')!.querySelector<HTMLElement>('.dt-info')!
  await act(async () => { tip.dispatchEvent(new MouseEvent('mouseover', { bubbles: true })) })
  expect(document.querySelector('[role="tooltip"]')?.textContent).toContain('到期后记为未完成')
  expect(tip.closest('.dt-field')!.textContent).not.toContain('到期后记为未完成')
  await act(async () => { tip.dispatchEvent(new MouseEvent('mouseout', { bubbles: true })) })
  expect(document.querySelector('[role="tooltip"]')).toBeNull()
})

it('offers a Ticket awaiting review its review place and Session in the page head', async () => {
  const t1 = server.ticketDetails.t1!
  server.ticketDetails.t1 = { ...t1, stage: 'person', action: 'review', threads: [threadSummary(),
    { ...threadSummary({ key: 'github:42', source: 'github', title: '#42 Fix login' }), url: 'https://github.com/acme/widgets/pull/42' }] }
  at({ page: 'execution', id: 't1' })
  await mount()
  expect(document.querySelector('.dt-status-line .dt-stage')?.textContent).toBe(zh['stage.review'])
  expect(all('.dt-page-head a').map(link => [link.textContent, link.getAttribute('href')])).toEqual([['打开 GitHub 原文',
    'https://github.com/acme/widgets/pull/42']])
})

it('keeps what a person types in the Execution settings, converting it once on save', async () => {
  await mount()
  await click(zh['tab.execution'])
  await click(find(zh['tab.settings'], '.dt-status-head button'))
  const blocked = () => field<HTMLInputElement>(zh.blockedCommands), root = () => field<HTMLInputElement>(zh.executionDirectory),
    workers = () => field<HTMLInputElement>(zh.maxWorkers)
  // A separator typed before the next command, a space inside a path and a cleared number stay as typed.
  await type(blocked(), 'gh-personal ')
  expect(blocked().value).toBe('gh-personal ')
  await type(blocked(), 'gh-personal, gh-other')
  await type(root(), '/srv/my work ')
  expect(root().value).toBe('/srv/my work ')
  await type(workers(), '')
  expect(workers().value).toBe('')
  await type(workers(), '2')
  await click(zh.save)
  expect(server.requests.find(request => request.path === 'settings/executor')?.body).toMatchObject({ value: { blockedCommands: ['gh-personal', 'gh-other'], directory: '/srv/my work ', maxWorkers: 2 } })
  expect(server.settings.executor).toMatchObject({ blockedCommands: ['gh-personal', 'gh-other'], directory: '/srv/my work', maxWorkers: 2 })
})

it('says why an Input was not delivered on its Ticket\'s page, in either language', async () => {
  const t1 = server.ticketDetails.t1!
  server.ticketDetails.t1 = { ...t1, stage: 'person', action: 'delivery', deliveryError: 'feishu: message recalled' }
  at({ page: 'execution', id: 't1' })
  await mount()
  expect(document.querySelector('.dt-status-line .dt-stage')?.textContent).toBe(zh['stage.delivery'])
  expect(document.querySelector('.dt-banner.dt-danger')?.textContent).toBe(`${zh.deliveryFailedPrefix}feishu: message recalled`)
  expect(text()).toContain(zh.inputRetryHint)
  expect(text()).not.toContain(zh.inputWaitingHint)
  expect(document.querySelector('.dt-status-line')?.textContent).toMatch(/9月\d+日/)
  await remount(en)
  expect(document.querySelector('.dt-status-line')?.textContent).toMatch(/Sep \d+/)
  expect(document.querySelector('.dt-status-line .dt-stage')?.textContent).toBe(en['stage.delivery'])
  expect(document.querySelector('.dt-banner.dt-danger')?.textContent).toBe(`${en.deliveryFailedPrefix}feishu: message recalled`)
  expect(text()).toContain(en.inputRetryHint)
})

it('tells a Ticket or Thread that does not exist from a dsh-superagent that is not answering', async () => {
  at({ page: 'execution', id: 't9' })
  await mount()
  expect(text()).toContain(zh.ticketNotFound)
  // While dsh-superagent restarts, DSH's web server answers every route with an empty 404.
  at({ page: 'execution', id: 't1' })
  server.down = true
  await remount()
  expect(text()).not.toContain(zh.ticketNotFound)
  expect(text()).toContain(zh.hostNotRunning)
  at({ page: 'trigger', id: 'feishu:oc_1:om_root' })
  await remount()
  expect(text()).not.toContain(zh.threadNotFound)
  expect(text()).toContain(zh.hostNotRunning)
})

it('gives a Ticket a bot\'s message started its basis, marks a Thread it follows for context, and an Input it handed back', async () => {
  const t1 = server.ticketDetails.t1!
  server.ticketDetails.t1 = { ...t1, mandate: { ...t1.mandate, strength: 'inferred', basis: { via: 'bot', sender: '评审机器人',
    observation: { mode: 'auto' } } },
  basis: { facts: [{ name: 'sender', value: '评审机器人' }, { name: 'bot' }, { name: 'mode', value: 'auto' }], legacy: false },
  threads: [threadSummary(), { ...threadSummary({ key: 'github:7', title: '#7 相关 Issue' }), context: true }],
  inputs: [{ id: 'i1', kind: 'trigger', triggerId: 'e2', at: '2026-09-22T00:03:00Z', delivered: true, trigger: { id: 'e2',
    actor: { source: 'feishu', id: MEMBER, name: { zh: 'Alice', en: 'Alice' } }, human: true, preview: '顺便加一个页面',
    thread: 'feishu:oc_1:om_root',
    handback: { reason: 'Asks for a new page', at: '2026-09-22T00:05:00Z' } } }] }
  at({ page: 'execution', id: 't1' })
  await mount()
  const facts = () => all('.dt-kv > div').map(row => [row.querySelector('dt')?.textContent, row.querySelector('dd')?.textContent])
  expect(facts()).toEqual(expect.arrayContaining([[zh['basis.sender'], '评审机器人'], [zh['basis.bot'], zh.yes], [zh['basis.mode'], zh['mode.auto']]]))
  expect(all('.dt-list .dt-ticket').map(row => row.querySelector('.dt-tag')?.textContent)).toEqual([undefined, zh.contextOnly])
  await click(zh.deliveredInputs.replace('{count}', '1'))
  expect(text()).toContain('已交回评估：Asks for a new page')
  await remount(en)
  expect(facts()).toEqual(expect.arrayContaining([[en['basis.bot'], en.yes]]))
  expect(all('.dt-list .dt-ticket').map(row => row.querySelector('.dt-tag')?.textContent)).toEqual([undefined, en.contextOnly])
  await click(en.deliveredInputs.replace('{count}', '1'))
  expect(text()).toContain('Handed back for an Eval: Asks for a new page')
})

it('words an ended Ticket by its outcome, on its row and its page, with a person\'s reason after it', async () => {
  const fields = { id: 't3', title: '已合并的工单', state: 'ended' as const, actions: { retry: false, stop: false },
    ended: { at: '2026-09-21T00:00:00Z', by: 'executor' as const, outcome: 'merged' as const } }
  server.tickets.ended = [ticketSummary(fields, { stage: 'ended' }), ...server.tickets.ended]
  server.ticketDetails.t3 = { ...server.ticketDetails.t1!, ...fields, stage: 'ended', actions: { ...fields.actions, reopen: true, approve: false, merge: false, discard: false, rebase: false, cleanup: true, check: false,
      reviewDecision: false, revert: true, push: true, dequeue: true },
    merged: { at: '2026-09-21T00:00:00Z', branch: 'agent/t3', target: 'main', via: 'chat', head: 'e'.repeat(40),
      push: { at: '2026-09-21T00:00:00Z', ok: false, error: 'no upstream' } },
    queuePosition: 2,
    targetCheck: { command: 'pnpm test', ok: false, exitCode: 1, at: '2026-09-21T00:00:00Z', output: 'merged tree failed' } }
  await mount()
  await click(zh['tab.execution'])
  await click(zh['ticket.ended'])
  expect(all('.dt-ticket-stage').map(stage => stage.textContent)).toEqual([zh['outcome.merged'], zh['outcome.stopped']])
  await click('已合并的工单')
  expect(document.querySelector('.dt-status-line .dt-stage')?.textContent).toBe(zh['outcome.merged'])
  expect(text()).toContain(zh['outcome.merged'])
  expect(text()).not.toContain('执行器结束')
  // A merged ticket offers cleaning up its worktree instead of discarding the branch, and undoing the merge.
  expect(text()).toContain(zh.cleanupWorktree)
  expect(text()).toContain(zh.ticketRevert)
  await click(zh.ticketRevert)
  await click(zh.confirm)
  expect(server.requests.find(request => request.path === 'tickets/t3/revert')).toBeDefined()
  expect(server.requests.find(request => request.path === 'tickets/t3/revert')).toMatchObject({ method: 'POST' })
  // A failed push offers a retry, a queued Ticket shows its place and can leave the queue, and the check on the merged
  // target keeps its result and output.
  expect(text()).toContain(zh.pushRetry)
  expect(text()).toContain('已排入合并队列，第 2 位。')
  expect(text()).toContain(zh.queueCancel)
  expect(text()).toContain(zh.targetCheckTitle)
  expect(text()).toContain('merged tree failed')
  await click(zh.pushRetry)
  expect(server.requests.find(request => request.path === 'tickets/t3/push')).toMatchObject({ method: 'POST' })
  await click(zh.queueCancel)
  expect(server.requests.find(request => request.path === 'tickets/t3/dequeue')).toMatchObject({ method: 'POST' })
  server.ticketDetails.t3 = { ...server.ticketDetails.t3, ended: { at: '2026-09-21T00:00:00Z', by: 'human', outcome: 'stopped',
    reason: '不需要了' } }
  await remount(en)
  expect(all('.dt-hint').map(hint => hint.textContent).find(line => line?.includes(en['outcome.stopped']))).toMatch(/Stopped by a person: 不需要了$/)
})

it('sends one stop for a double click on Confirm', async () => {
  at({ page: 'execution', id: 't1' })
  await mount()
  await click(zh.stop)
  const confirm = find(zh.confirm)
  await act(async () => { confirm.click(); confirm.click() })
  expect(server.requests.filter(request => request.path === 'tickets/t1/stop')).toHaveLength(1)
})

it('creates a local ticket from Execution and opens its Thread', async () => {
  await mount()
  await click(zh['tab.execution'])
  await click(zh.newTicket)
  const dialog = document.querySelector('[role="dialog"]')!
  expect(dialog.textContent).toContain(zh.newTicketHint)
  await type(field<HTMLInputElement>(zh.newTicketTitle), '修复登录页')
  await type(field<HTMLInputElement>(zh.newTicketProject), 'web')
  await type(field<HTMLInputElement>(zh.newTicketTags), '前端, p1')
  const picker = field<HTMLInputElement>(zh.newTicketFiles)
  await act(async () => {
    Object.defineProperty(picker, 'files', { value: [new File(['hello'], 'a.txt', { type: 'text/plain' })], configurable: true })
    picker.dispatchEvent(new Event('change', { bubbles: true }))
  })
  await type(field<HTMLTextAreaElement>(zh.newTicketText), '窄屏下登录按钮溢出，请修好。')
  await click(zh.createTicket)
  const sent = server.requests.find(request => request.path === 'local/requests')!
  expect(sent).toMatchObject({ method: 'POST', body: { title: '修复登录页', text: '窄屏下登录按钮溢出，请修好。', project: 'web', tags: ['前端', 'p1'],
    files: [{ name: 'a.txt', data: expect.any(String) }] } })
  // One idempotency key accompanies the draft, so retrying after a lost response cannot create a second ticket.
  expect((sent.body as { key?: string }).key).toMatch(/^[\da-f-]{8,}$/)
  expect(JSON.parse(sessionStorage.getItem('dsh-superagent/route')!)).toMatchObject({ route: { page: 'trigger', id: 'local:local-1' } })
})

it('does not offer a reply box for a local Thread the Ticket only follows for context', async () => {
  at({ page: 'execution', id: 't1' })
  server.ticketDetails.t1 = { ...server.ticketDetails.t1!, threads: [{ ...threadSummary({ key: 'local:other', source: 'local',
    title: '别人的工单' }), context: true }] }
  await mount()
  expect(text()).not.toContain(zh.localReplyTitle)
})

it('replies to a local ticket from its page', async () => {
  at({ page: 'execution', id: 't1' })
  server.ticketDetails.t1 = { ...server.ticketDetails.t1!, threads: [threadSummary({ key: 'local:req1', source: 'local',
    title: '本地请求' })], channels: [{ source: 'local', url: '', label: { zh: '本地工单', en: 'Local ticket' } }] }
  await mount()
  expect(text()).toContain(zh.localReplyTitle)
  await type(field<HTMLTextAreaElement>(zh.localReplyLabel), '按这个方案做')
  await click(zh.localReplySend)
  expect(server.requests.find(request => request.path === 'local/followups')).toMatchObject({ method: 'POST',
    body: { thread: 'local:req1', text: '按这个方案做' } })
})

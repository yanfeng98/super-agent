import { expect, it } from 'vitest'
import { zh } from '../src/client/locales.ts'
import { decisionMark, pushMark, reportDot, sourceMark, threadMark, ticketMark } from '../src/client/marks.ts'
import { translator } from './fixtures/panel.tsx'

const t = translator(zh)

it('marks each Ticket stage by whom it waits for, and an ended Ticket by its outcome', () => {
  const marks = [{ stage: 'person', action: 'answer' }, { stage: 'person', action: 'review' }, { stage: 'person', action: 'blocked' },
    { stage: 'person', action: 'failed' }, { stage: 'person', action: 'delivery' }, { stage: 'working', action: 'queued' },
    { stage: 'working' }, { stage: 'idle' }] as const
  const open = marks.map(stage => ticketMark(t, { ...stage, ended: undefined }))
  expect(open.map(({ label, tone, dot, icon }) => [label, tone, dot, icon])).toEqual([
    ['待回答', 'warning', 'warning', 'ask'], ['待验收', 'warning', 'warning', 'review'], ['待处理受阻', 'warning', 'warning', 'blocked'],
    ['待处理失败', 'danger', 'error', 'alert'], ['输入未送达', 'warning', 'warning', 'inbox'], ['排队中', 'neutral', 'idle', 'queue'],
    ['执行中', 'info', 'ongoing', 'running'], ['等待触发', 'neutral', 'idle', 'clock']])
  // Every non-ended mark has a hover meaning.
  expect(open.map(mark => mark.title)).toEqual([
    '执行器向人提了问题，等待回答。', '工作做完，等待有人验收变更。', '执行器受阻，需要有人处理才能继续。',
    '执行器失败，需要有人处理才能继续。', '发给执行器的输入还没送达。', '执行器名额已满，工单排队等待开始。',
    '执行器正在这个工单上工作。', '执行器结束了当前回合，没有再问人；订阅的线程收到新触发时会继续。'])
  expect(open.every(mark => Boolean(mark.title))).toBe(true)
  const ended = (['completed', 'merged', 'closed', 'dropped', 'stopped'] as const).map(outcome => ticketMark(t, { stage: 'ended',
    ended: { at: '', by: outcome === 'stopped' ? 'human' : 'executor', outcome } }))
  expect(ended.map(mark => mark.label)).toEqual(['已完成待合并', '已合并', '已关闭', '无需处理', '已放弃'])
  expect(ended.map(({ tone, dot, icon }) => `${tone} ${dot} ${icon}`)).toEqual(['success done decision', 'success done pull',
    'neutral idle stop', 'neutral idle stop', 'neutral idle dismiss'])
  expect(ended.map(mark => mark.title)).toEqual([
    '工作做完、没人要求合并；若要落到仓库，请在工单页合并它的分支。',
    '工作做完，分支已经进入目标分支。',
    '工单已关闭，不再继续工作。',
    '这个工单无需处理。',
    '由人放弃，不再继续。'])
  expect(ended.every(mark => Boolean(mark.title))).toBe(true)
  // A merge labels the end state even when the work had already been reported completed, and its hover details say where
  // and which commit the target holds.
  const merged = { at: '2026-09-21T00:00:00Z', target: 'main', branch: 'agent/t1', head: `${'e'.repeat(8)}${'0'.repeat(32)}` }
  expect(ticketMark(t, { stage: 'ended', merged, ended: { at: '', by: 'executor', outcome: 'completed' } }))
    .toMatchObject({ label: '已合并', tone: 'success', dot: 'done', icon: 'pull',
      title: '工作做完，分支已经进入目标分支。\n已合并到 main（agent/t1）。\n合并后目标分支停在 eeeeeeee。' })
  // Completed work still says what is left to do, and an abandoned Ticket keeps the person's reason in its hover detail.
  expect(ticketMark(t, { stage: 'ended', ended: { at: '2026-09-21T00:00:00Z', by: 'executor', outcome: 'completed' } }).title)
    .toContain('若要落到仓库')
  const abandoned = ticketMark(t, { stage: 'ended', ended: { at: '2026-09-21T00:00:00Z', by: 'human', outcome: 'stopped',
    reason: '需求取消' } })
  expect(abandoned.label).toBe('已放弃')
  expect(abandoned.title).toContain('由人放弃，不再继续')
  expect(abandoned.title).toMatch(/9月21日 \d\d:\d\d 已放弃：需求取消$/)
  expect(['failed', 'blocked', 'waiting', 'ready', 'progress', undefined].map(reportDot)).toEqual(['error', 'warning', 'warning', 'done',
    'ongoing', 'ongoing'])
  // Push is a separate axis from the Ticket's own state, so it has its own mark with the same wording everywhere.
  expect(pushMark(t, undefined)).toBeUndefined()
  expect(pushMark(t, { ok: true })).toMatchObject({ label: '已推送', tone: 'success',
    title: '已推送到上游。\n合并结果已推送到上游仓库。' })
  expect(pushMark(t, { ok: false, error: 'no upstream' })).toMatchObject({ label: '推送失败', tone: 'danger',
    title: '合并已完成，但把结果推到上游失败了；可在工单页重试推送。\n合并已完成，但推送失败：no upstream' })
})

it('marks each Thread stage, Source state and Decision outcome', () => {
  const threads = (['error', 'evaluating', 'queued', 'subscribed', 'dormant', 'unsubscribed'] as const).map(stage => threadMark(t,
    { stage, pending: 2 }))
  expect(threads.map(({ label, dot, icon }) => [label, dot, icon])).toEqual([['轮询失败', 'error', 'alert'], ['评估进行中', 'ongoing', 'running'],
    ['排队 2', 'warning', 'queue'], ['已订阅', 'done', 'inbox'], ['工单已结束', 'idle', 'clock'], ['未订阅', 'idle', 'hash']])
  const sources = (['missing', 'off', 'error', 'setup', 'starting', 'paused', 'ready'] as const).map(state => sourceMark(t, state))
  expect(sources.map(({ label, dot }) => [label, dot])).toEqual([['插件未安装', 'idle'], ['已关闭', 'idle'], ['出错', 'error'],
    ['等待部署检查', 'idle'], ['启动中', 'ongoing'], ['待处理', 'warning'], ['运行中', 'done']])
  const decisions = (['accept', 'reopen', 'stop', 'ignore', 'unavailable', 'wouldAccept'] as const).map(outcome => decisionMark(t, outcome))
  expect(decisions.map(({ label, tone, dot, icon }) => [label, tone, dot, icon])).toEqual([['接单', 'success', 'done', 'decision'],
    ['续做', 'success', 'done', 'refresh'], ['停止', 'danger', 'idle', 'stop'], ['忽略', 'neutral', 'idle', 'dismiss'],
    ['未完成', 'warning', 'warning', 'alert'], ['忽略（可接单，仅观察）', 'warning', 'idle', 'dismiss']])
})

/** Requests and answers the main Session records from its own conversation as local tickets. */
import { existsSync, readFileSync } from 'node:fs'
import { afterEach, beforeEach, expect, it } from 'vitest'
import { UserError } from '../src/core/index.ts'
import { feishu, startRuntime, type Host } from './fixtures/host.ts'

let h: Host
beforeEach(async () => { h = await startRuntime(undefined, { local: true }) })
afterEach(async () => { await h.stop() })

interface Created { request: string; thread: string; trigger: string; file: string; status: string }
interface Answered { trigger: string; thread: string; ticket: { ticket_id: string } }

it('records a request from the main Session as a local ticket and carries the person\'s answer to its worker', async () => {
  const created = await h.runTool(h.runtime.masterId, 'orchestrator_request',
    { title: '修复登录页', text: '窄屏下登录页溢出，请修好。' }) as unknown as Created
  expect(created.request).toMatch(/^local-\d{8}T\d{6}-[0-9a-f]{8}$/)
  expect(created.thread).toBe(`local:${created.request}`)
  expect(created.status).toContain('Queued for Eval')
  expect(existsSync(created.file)).toBe(true)
  expect(readFileSync(created.file, 'utf8')).toContain('窄屏下登录页溢出，请修好。')
  // The first request seeds the local Policy part, so an installation that never opened the panel still admits the operator.
  expect(h.runtime.store.settings().policy.sources.local?.allowed).toEqual([{ id: 'operator', name: 'Operator' }])
  await h.runtime.idle()
  expect(h.candidates()).toContain(created.trigger)
  const routed = await h.route({ kind: 'accept', trigger_id: created.trigger, title: '修复登录页', worktree_name: 'fix-login',
    acknowledgement: '收到，修复登录页。' })
  await h.runtime.idle()
  const ticketId = (routed.ticket as { ticket_id: string }).ticket_id
  await h.runTool(`worker-${ticketId}`, 'orchestrator_report',
    { state: 'progress', summary: 'Need a layout choice.', human_action: '窄屏用单列还是双列？' })
  await h.runtime.idle()
  expect(h.relays(h.runtime.masterId)).toContainEqual(expect.objectContaining({ kind: 'worker_attention' }))
  const answered = await h.runTool(h.runtime.masterId, 'orchestrator_answer',
    { ticket_id: ticketId, text: '用单列布局。' }) as unknown as Answered
  expect(answered).toMatchObject({ thread: created.thread, ticket: { ticket_id: ticketId } })
  expect(readFileSync(created.file, 'utf8')).toContain('用单列布局。')
  expect(h.runtime.store.tickets.get(ticketId)?.inputs).toEqual(expect.arrayContaining([expect.objectContaining({ kind: 'trigger' })]))
  expect(h.relays(h.runtime.masterId).filter(item => item.kind === 'follow_up')).toEqual([])
})

it('refuses to answer a ticket another Source created', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  await expect(h.runTool(h.runtime.masterId, 'orchestrator_answer', { ticket_id: ticket.id, text: '用单列布局。' }))
    .rejects.toThrow(new UserError('notLocalTicket'))
})

it('refuses a request while the local Source is off', async () => {
  const settings = h.runtime.store.settings()
  h.runtime.store.saveSettings({ ...settings, sources: { ...settings.sources, local: { enabled: false, config: {} } } })
  await expect(h.runTool(h.runtime.masterId, 'orchestrator_request', { title: '修复登录页', text: '窄屏下溢出。' }))
    .rejects.toThrow('The local Source is off')
})

it('resumes an ended local ticket when the person answers from the main Session', async () => {
  const created = await h.runTool(h.runtime.masterId, 'orchestrator_request',
    { title: '修复登录页', text: '窄屏下登录页溢出，请修好。' }) as unknown as Created
  await h.runtime.idle()
  const routed = await h.route({ kind: 'accept', trigger_id: created.trigger, title: '修复登录页', worktree_name: 'fix-login',
    acknowledgement: '收到，修复登录页。' })
  await h.runtime.idle()
  const ticketId = (routed.ticket as { ticket_id: string }).ticket_id
  await h.runTool(`worker-${ticketId}`, 'orchestrator_report', { state: 'ready', summary: 'Login page fixed.' })
  await h.runtime.idle()
  expect(h.runtime.store.tickets.get(ticketId)?.ended).toBeTruthy()
  await h.runTool(h.runtime.masterId, 'orchestrator_answer', { ticket_id: ticketId, text: '再加一个窄屏测试。' })
  await h.runtime.idle()
  const ticket = h.runtime.store.tickets.get(ticketId)!
  expect(ticket.ended).toBeUndefined()
  expect(ticket.assignments).toHaveLength(2)
  expect(readFileSync(created.file, 'utf8')).toContain('再加一个窄屏测试。')
})

it('refuses a request when the local Policy allows nobody', async () => {
  const settings = h.runtime.store.settings()
  h.runtime.store.saveSettings({ ...settings, policy: { ...settings.policy,
    sources: { ...settings.policy.sources, local: { allowed: [] } } } })
  await expect(h.runTool(h.runtime.masterId, 'orchestrator_request', { title: '修复登录页', text: '窄屏下溢出。' }))
    .rejects.toThrow('The local Source allows nobody')
})

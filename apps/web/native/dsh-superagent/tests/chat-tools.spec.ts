/** Requests and answers the main Session records from its own conversation as local tickets. */
import { existsSync, readFileSync } from 'node:fs'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
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

it('merges an ended ticket through the main Session with the panel gates', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const merge = vi.spyOn(h.runtime, 'mergeTicket').mockResolvedValue({ branch: 'agent/t', target: 'main' })
  const result = await h.runTool(h.runtime.masterId, 'orchestrator_ticket_merge',
    { ticket_id: ticket.id, strategy: 'squash' }) as unknown as { branch: string; target: string; ticket: { ticket_id: string } }
  // The tool records that a person asked from the main Session, which the ticket's record and archive keep.
  expect(merge).toHaveBeenCalledWith(ticket.id, 'squash', {}, 'chat')
  expect(result).toMatchObject({ branch: 'agent/t', target: 'main', ticket: { ticket_id: ticket.id } })
  // The schema defaults to a fast-forward, and a gate error reaches the model unchanged.
  await h.runTool(h.runtime.masterId, 'orchestrator_ticket_merge', { ticket_id: ticket.id })
  expect(merge).toHaveBeenLastCalledWith(ticket.id, 'ff', {}, 'chat')
  // A person who also asks for the worktree to go gets the panel's cleanup in the same call.
  await h.runTool(h.runtime.masterId, 'orchestrator_ticket_merge', { ticket_id: ticket.id, cleanup: true })
  expect(merge).toHaveBeenLastCalledWith(ticket.id, 'ff', { cleanup: true }, 'chat')
  // A person who asks to push as well gets both steps in one call.
  await h.runTool(h.runtime.masterId, 'orchestrator_ticket_merge', { ticket_id: ticket.id, push: true })
  expect(merge).toHaveBeenLastCalledWith(ticket.id, 'ff', { push: true }, 'chat')
  merge.mockRejectedValueOnce(new Error('Review approval is required before merging'))
  await expect(h.runTool(h.runtime.masterId, 'orchestrator_ticket_merge', { ticket_id: ticket.id }))
    .rejects.toThrow('Review approval is required before merging')
})

it('records a standing merge request from the main Session, and cancels it', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const request = { at: '2026-11-19T00:00:00.000Z', attempts: 0 } as const
  const auto = vi.spyOn(h.runtime, 'autoMergeTicket').mockReturnValue({ ...ticket, autoMerge: { ...request } })
  const recorded = await h.runTool(h.runtime.masterId, 'orchestrator_ticket_automerge',
    { ticket_id: ticket.id, strategy: 'squash', push: true }) as unknown as { auto_merge: boolean; ticket: { ticket_id: string } }
  expect(auto).toHaveBeenCalledWith(ticket.id, { strategy: 'squash', push: true })
  expect(recorded).toMatchObject({ auto_merge: true, ticket: { ticket_id: ticket.id } })
  const cancel = vi.spyOn(h.runtime, 'cancelAutoMerge').mockReturnValue({ ...ticket })
  const cancelled = await h.runTool(h.runtime.masterId, 'orchestrator_ticket_automerge',
    { ticket_id: ticket.id, cancel: true }) as unknown as { auto_merge: boolean }
  expect(cancel).toHaveBeenCalledWith(ticket.id)
  expect(cancelled.auto_merge).toBe(false)
})

it('records a dependency the person states in chat, and clears it the same way', async () => {
  const { ticket: first } = await h.acceptedTicket(feishu('1', '@bot 先做前置'))
  const { ticket: second } = await h.acceptedTicket(feishu('2', '@bot 再做后续'))
  const run = (args: unknown) => h.runTool(h.runtime.masterId, 'orchestrator_ticket_dependency', args)
  expect(await run({ ticket_id: second.id, blocker_id: first.id }))
    .toMatchObject({ blocked_by: [expect.objectContaining({ id: first.id, merged: false })], ticket: { ticket_id: second.id } })
  expect(h.runtime.store.tickets.get(second.id)?.blockedBy).toEqual([first.id])
  expect(await run({ ticket_id: second.id, blocker_id: first.id, blocked: false }))
    .toMatchObject({ ticket_id: second.id, blocked_by: [] })
  expect(h.runtime.store.tickets.get(second.id)?.blockedBy).toBeUndefined()
  // Both Tickets may be named by the display number the ticket page shows, which is how a person quotes them.
  const number = (id: string) => `T-${String(h.runtime.store.tickets.numbering().get(id)).padStart(4, '0')}`
  expect(await run({ ticket_id: number(second.id), blocker_id: number(first.id) }))
    .toMatchObject({ ticket_id: second.id, blocked_by: [expect.objectContaining({ id: first.id })], ticket: { ticket_id: second.id } })
  expect(h.runtime.store.tickets.get(second.id)?.blockedBy).toEqual([first.id])
  await expect(run({ ticket_id: first.id, blocker_id: first.id })).rejects.toThrow('A ticket cannot wait for itself')
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

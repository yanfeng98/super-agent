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

it('hands review notes and a failing check back to the worker from the main Session', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  await h.runTool(`worker-${ticket.id}`, 'orchestrator_report', { state: 'ready', summary: 'done' })
  await h.runtime.idle()
  h.runtime.addNote(ticket.id, { file: 'src/a.ts', line: 3, text: '改成可选' })
  const handed = await h.runTool(h.runtime.masterId, 'orchestrator_ticket_note',
    { ticket_id: ticket.id, review: true }) as unknown as { reason: string; ticket: { ticket_id: string } }
  expect(handed).toMatchObject({ reason: 'Handed the ticket back to its worker', ticket: { ticket_id: ticket.id } })
  const updated = h.runtime.store.tickets.get(ticket.id)!
  expect(updated.ended).toBeUndefined()
  expect(updated.notes?.[0]?.sent?.at).toBeDefined()
  expect(updated.inputs.at(-1)).toMatchObject({ kind: 'text', text: expect.stringContaining('src/a.ts:3: 改成可选') })

  const { ticket: failing } = await h.acceptedTicket(feishu('2', '@bot 再修一个'))
  await h.runTool(`worker-${failing.id}`, 'orchestrator_report', { state: 'ready', summary: 'done' })
  await h.runtime.idle()
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(failing.id)!
    current.checks = { command: 'pnpm test', ok: false, exitCode: 1, at: current.ended!.at, output: 'one test failed' }
    h.runtime.store.tickets.save(current)
  })
  await h.runTool(h.runtime.masterId, 'orchestrator_ticket_note', { ticket_id: failing.id, check: true })
  expect(h.runtime.store.tickets.get(failing.id)!.inputs.at(-1)).toMatchObject({ kind: 'text',
    text: expect.stringContaining('one test failed') })
})

it('stops a live ticket from the main Session, and discards an ended one when asked', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 停一下'))
  const stopped = await h.runTool(h.runtime.masterId, 'orchestrator_ticket_abandon', { ticket_id: ticket.id }) as unknown as
    { stopped: boolean; discarded: boolean; ticket: { ticket_id: string } }
  expect(stopped).toMatchObject({ stopped: true, discarded: false, ticket: { ticket_id: ticket.id } })
  expect(h.runtime.store.tickets.get(ticket.id)?.ended).toMatchObject({ by: 'human', outcome: 'stopped' })

  const { ticket: endedTicket } = await h.acceptedTicket(feishu('2', '@bot 再修一个'))
  await h.runTool(`worker-${endedTicket.id}`, 'orchestrator_report', { state: 'ready', summary: 'done' })
  await h.runtime.idle()
  const discard = vi.spyOn(h.runtime, 'discardTicket').mockResolvedValue(undefined)
  const discarded = await h.runTool(h.runtime.masterId, 'orchestrator_ticket_abandon',
    { ticket_id: 'T-0002', discard: true }) as unknown as { stopped: boolean; discarded: boolean }
  expect(discard).toHaveBeenCalledWith(endedTicket.id)
  expect(discarded).toMatchObject({ stopped: false, discarded: true })

  const { ticket: live } = await h.acceptedTicket(feishu('3', '@bot 又不做了'))
  const both = await h.runTool(h.runtime.masterId, 'orchestrator_ticket_abandon',
    { ticket_id: live.id, discard: true, reason: '不做了' }) as unknown as { stopped: boolean; discarded: boolean }
  expect(both).toMatchObject({ stopped: true, discarded: true })
  expect(discard).toHaveBeenLastCalledWith(live.id)
  expect(h.runtime.store.tickets.get(live.id)?.ended).toMatchObject({ reason: '不做了' })
})

it('records an approval and resolves open notes from the main Session', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  await h.runTool(`worker-${ticket.id}`, 'orchestrator_report', { state: 'ready', summary: 'done' })
  await h.runtime.idle()
  h.runtime.addNote(ticket.id, { file: 'src/a.ts', line: 3, text: '改成可选' })
  const result = await h.runTool(h.runtime.masterId, 'orchestrator_ticket_review',
    { ticket_id: 'T-0001', approve: true, resolve: true }) as unknown as
    { approved: boolean; resolved: boolean; ticket: { ticket_id: string } }
  expect(result).toMatchObject({ approved: true, resolved: true, ticket: { ticket_id: ticket.id } })
  const reviewed = h.runtime.store.tickets.get(ticket.id)!
  expect(reviewed.reviewed).toMatchObject({ decision: 'approved', via: 'chat' })
  expect(reviewed.notes?.every(note => note.resolved)).toBe(true)
})

it('reruns the configured check from the main Session', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  await h.runTool(`worker-${ticket.id}`, 'orchestrator_report', { state: 'ready', summary: 'done' })
  await h.runtime.idle()
  const check = vi.spyOn(h.runtime, 'checkTicket').mockResolvedValue(
    { command: 'pnpm test', ok: true, exitCode: 0, at: '2026-11-19T00:02:00.000Z', output: 'ok' })
  const result = await h.runTool(h.runtime.masterId, 'orchestrator_ticket_check', { ticket_id: 'T-0001' }) as unknown as
    { check: { ok: boolean }; ticket: { ticket_id: string } }
  expect(check).toHaveBeenCalledWith(ticket.id)
  expect(result).toMatchObject({ check: { ok: true }, ticket: { ticket_id: ticket.id } })
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

it('queues several tickets from one instruction, and cancels them', async () => {
  const { ticket: first } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const { ticket: second } = await h.acceptedTicket(feishu('2', '@bot 再修一个'))
  const lines = [{ ticket_id: 'T-0001', position: 1 }, { ticket_id: 'T-0002', position: 2 }]
  const queue = vi.spyOn(h.runtime, 'queueMerges').mockReturnValue({ queued: lines, queue: lines })
  const queued = await h.runTool(h.runtime.masterId, 'orchestrator_merge_queue',
    { ticket_ids: [first.id, second.id], cleanup: true }) as unknown as { cancelled: boolean; queued: { position: number }[] }
  expect(queue).toHaveBeenCalledWith([first.id, second.id], { cleanup: true })
  expect(queued).toMatchObject({ cancelled: false, queued: [{ position: 1 }, { position: 2 }] })
  // A failed entry is retried through the same tool.
  const retry = vi.spyOn(h.runtime, 'retryMerges').mockReturnValue({ retried: 1, queue: [] })
  const retried = await h.runTool(h.runtime.masterId, 'orchestrator_merge_queue',
    { ticket_ids: ['T-0001'], retry: true }) as unknown as { retried: number }
  expect(retry).toHaveBeenCalledWith(['T-0001'])
  expect(retried).toMatchObject({ retried: 1 })
  // The same tool takes Tickets out of the queue.
  const dequeue = vi.spyOn(h.runtime, 'dequeueMerges').mockReturnValue({ removed: 2, queue: [] })
  const cancelled = await h.runTool(h.runtime.masterId, 'orchestrator_merge_queue',
    { ticket_ids: ['T-0001'], cancel: true }) as unknown as { cancelled: boolean; removed: number }
  expect(dequeue).toHaveBeenCalledWith(['T-0001'])
  expect(cancelled).toMatchObject({ cancelled: true, removed: 2 })
  // One Ticket still merges through the merge tool itself, and that tool no longer takes a list.
  const merge = vi.spyOn(h.runtime, 'mergeTicket').mockResolvedValue({ branch: 'agent/a', target: 'main' })
  await h.runTool(h.runtime.masterId, 'orchestrator_ticket_merge', { ticket_id: first.id })
  expect(merge).toHaveBeenCalledWith(first.id, 'ff', {}, 'chat')
})

it('reports the merge queue and the tickets waiting for a merge or a review', async () => {
  const { ticket: queuedTicket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const { ticket: ready } = await h.acceptedTicket(feishu('2', '@bot 再修一个'))
  const { ticket: waiting } = await h.acceptedTicket(feishu('3', '@bot 等前一个'))
  const end = (id: string): void => {
    h.runtime.store.atomic(() => {
      const ticket = h.runtime.store.tickets.get(id)!
      ticket.ended = { at: '2026-11-19T00:00:00.000Z', by: 'executor', outcome: 'completed' }
      h.runtime.store.tickets.save(ticket)
    })
  }
  end(queuedTicket.id); end(ready.id); end(waiting.id)
  h.runtime.blockTicket(waiting.id, queuedTicket.id, true)
  // Pausing keeps the queue still, so the status is what the tool reports rather than what a pass drained.
  h.runtime.pauseMergeQueue(true)
  h.runtime.queueMerges([queuedTicket.id], {})
  const reported = await h.runTool(h.runtime.masterId, 'orchestrator_merge_queue', {}) as unknown as
    { cancelled: boolean; status: { queue: { paused: boolean; items: { ticket_id: string; position: number }[] };
      waiting: { ticket_id: string; needs: string }[] } }
  expect(reported).toMatchObject({ cancelled: false, status: { queue: { paused: true,
    items: [{ ticket_id: queuedTicket.id, position: 1 }] } } })
  expect(reported.status.waiting).toHaveLength(2)
  expect(reported.status.waiting).toEqual(expect.arrayContaining([
    expect.objectContaining({ ticket_id: 'T-0002', needs: 'merge', outcome: 'completed' }),
    expect.objectContaining({ ticket_id: 'T-0003', needs: 'dependency', reason: expect.stringMatching(/Blocked by/) }),
  ]))
  // Cancelling or retrying still needs the Tickets it acts on.
  await expect(h.runTool(h.runtime.masterId, 'orchestrator_merge_queue', { cancel: true })).rejects.toThrow(/Name the Tickets/)
  h.runtime.pauseMergeQueue(false)
})

it('reports the outcome of a stopped ticket in the waiting list', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 停一下'))
  h.runtime.engine.stopTicket(ticket.id)
  h.runtime.pauseMergeQueue(true)
  const reported = await h.runTool(h.runtime.masterId, 'orchestrator_merge_queue', {}) as unknown as
    { status: { waiting: { ticket_id: string; needs: string; outcome: string }[] } }
  expect(reported.status.waiting).toContainEqual(expect.objectContaining({ ticket_id: 'T-0001', needs: 'merge', outcome: 'stopped' }))
  h.runtime.pauseMergeQueue(false)
})

it('undoes a merged ticket from the main Session', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const revert = vi.spyOn(h.runtime, 'revertTicket').mockResolvedValue({ ticket_id: ticket.id, branch: 'agent/t', target: 'main',
    commit: 'c'.repeat(40) })
  const result = await h.runTool(h.runtime.masterId, 'orchestrator_ticket_revert', { ticket_id: 'T-0001' }) as unknown as
    { commit: string; ticket: { ticket_id: string } }
  expect(revert).toHaveBeenCalledWith('T-0001', 'chat', {})
  expect(result).toMatchObject({ commit: 'c'.repeat(40), ticket: { ticket_id: ticket.id } })
  // A person who also asks to push gets the reverting commit sent to the upstream.
  await h.runTool(h.runtime.masterId, 'orchestrator_ticket_revert', { ticket_id: 'T-0001', push: true })
  expect(revert).toHaveBeenLastCalledWith('T-0001', 'chat', { push: true })
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

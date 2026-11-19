/** Merging from the main Session: one request rebases a branch the repository moved past and runs the check it needs. */
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { captureReview, discardWorkspace, mergeWorkspace, pushWorkspace, RebaseConflictError, rebaseWorkspace, targetContained, workspaceHead } from '../src/app/review.ts'
import { feishu, startRuntime, type Host } from './fixtures/host.ts'

vi.mock('../src/app/review.ts', async importOriginal => {
  const actual = await importOriginal<typeof import('../src/app/review.ts')>()
  return { ...actual, mergeWorkspace: vi.fn(), targetContained: vi.fn(), workspaceHead: vi.fn(), captureReview: vi.fn(),
    rebaseWorkspace: vi.fn(), discardWorkspace: vi.fn(), pushWorkspace: vi.fn() }
})

let h: Host
beforeEach(async () => {
  h = await startRuntime()
  vi.mocked(targetContained).mockReset().mockResolvedValue(true)
  vi.mocked(mergeWorkspace).mockReset().mockResolvedValue({ branch: 'agent/t', target: 'main' })
  vi.mocked(workspaceHead).mockReset().mockResolvedValue(undefined)
  vi.mocked(captureReview).mockReset().mockResolvedValue(
    { capturedAt: '2026-11-19T00:03:00.000Z', head: 'new', commits: [], files: [], dirty: [] })
  vi.mocked(rebaseWorkspace).mockReset().mockResolvedValue({ branch: 'agent/t', target: 'main', base: 'base' })
  vi.mocked(discardWorkspace).mockReset().mockResolvedValue(undefined)
  vi.mocked(pushWorkspace).mockReset().mockResolvedValue(undefined)
})
afterEach(async () => { await h.stop() })

/** Ends the accepted ticket without running a worker, which is the state a merge starts from. */
const ended = (id: string): void => {
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(id)!
    current.ended = { at: '2026-11-19T00:00:00.000Z', by: 'executor', outcome: 'completed' }
    h.runtime.store.tickets.save(current)
  })
}
const configureCheck = (command: string): void => {
  const settings = h.runtime.store.settings()
  h.runtime.store.saveSettings({ ...settings, executor: { ...settings.executor, check: command } })
}

it('rebases a branch the repository moved past, runs the configured check, and merges', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  configureCheck('pnpm test')
  vi.mocked(targetContained).mockResolvedValue(false)
  const rebase = vi.spyOn(h.runtime, 'rebaseTicket').mockResolvedValue({ branch: 'agent/t', target: 'main' })
  const check = vi.spyOn(h.runtime, 'checkTicket').mockResolvedValue(
    { command: 'pnpm test', ok: true, exitCode: 0, at: '2026-11-19T00:01:00.000Z', output: 'passed' })
  expect(await h.runtime.mergeTicket(ticket.id)).toEqual({ branch: 'agent/t', target: 'main', rebased: true })
  expect(rebase).toHaveBeenCalledWith(ticket.id)
  expect(check).toHaveBeenCalledWith(ticket.id)
  const order = (mock: { mock: { invocationCallOrder: number[] } }) => mock.mock.invocationCallOrder[0]!
  expect(order(rebase)).toBeLessThan(order(check))
  expect(order(check)).toBeLessThan(order(vi.mocked(mergeWorkspace)))
  expect(h.runtime.store.tickets.get(ticket.id)?.merged).toMatchObject({ branch: 'agent/t', target: 'main' })
})

it('reports a failed check and does not merge the rebased branch', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  configureCheck('pnpm test')
  vi.mocked(targetContained).mockResolvedValue(false)
  vi.spyOn(h.runtime, 'rebaseTicket').mockResolvedValue({ branch: 'agent/t', target: 'main' })
  vi.spyOn(h.runtime, 'checkTicket').mockResolvedValue(
    { command: 'pnpm test', ok: false, exitCode: 1, at: '2026-11-19T00:01:00.000Z', output: 'one test failed' })
  await expect(h.runtime.mergeTicket(ticket.id)).rejects.toThrow(/check failed \(exit 1\).*one test failed/s)
  expect(mergeWorkspace).not.toHaveBeenCalled()
  expect(h.runtime.store.tickets.get(ticket.id)?.merged).toBeUndefined()
})

it('hands a conflicting rebase to the worker instead of leaving the person a second step', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  vi.mocked(targetContained).mockResolvedValue(false)
  vi.mocked(rebaseWorkspace).mockRejectedValue(new RebaseConflictError('main', ['a.txt', 'b.txt']))
  await expect(h.runtime.mergeTicket(ticket.id))
    .rejects.toThrow(/conflicts in a\.txt, b\.txt; the worker was told and is resolving it/)
  // The ticket takes the conflict back under its Mandate, so its worker resumes without a message from the person.
  const resumed = h.runtime.store.tickets.get(ticket.id)!
  expect(resumed.ended).toBeUndefined()
  expect(resumed.reviewed).toBeUndefined()
  expect(resumed.review).toBeUndefined()
  expect(resumed.inputs).toContainEqual(expect.objectContaining({ kind: 'text', text: expect.stringContaining('git rebase main') }))
  expect(resumed.inputs).toContainEqual(expect.objectContaining({ kind: 'text', text: expect.stringContaining('a.txt, b.txt') }))
  // The worker reads the conflict, in a new Assignment, rather than the person carrying it over.
  await h.runtime.idle()
  expect(resumed.assignments).toHaveLength(2)
  expect(h.deliveries.mock.calls.map(([call]) => call.text))
    .toContainEqual(expect.stringContaining('Rebasing the ticket branch onto main to merge it conflicts in a.txt, b.txt'))
  expect(mergeWorkspace).not.toHaveBeenCalled()
  expect(pushWorkspace).not.toHaveBeenCalled()
})

it('keeps the plain conflict report while processing is paused, which starts no work', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  const settings = h.runtime.store.settings()
  h.runtime.store.saveSettings({ ...settings, paused: true })
  vi.mocked(targetContained).mockResolvedValue(false)
  vi.mocked(rebaseWorkspace).mockRejectedValue(new RebaseConflictError('main', ['a.txt']))
  await expect(h.runtime.mergeTicket(ticket.id))
    .rejects.toThrow(/^Rebasing onto main conflicts in a\.txt; the Ticket branch keeps its commits$/)
  expect(h.runtime.store.tickets.get(ticket.id)?.ended).toBeDefined()
})

it('keeps the plain conflict report for a stopped ticket, which takes no new work', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(ticket.id)!
    current.ended = { at: '2026-11-19T00:00:00.000Z', by: 'human', outcome: 'stopped' }
    h.runtime.store.tickets.save(current)
  })
  vi.mocked(targetContained).mockResolvedValue(false)
  vi.mocked(rebaseWorkspace).mockRejectedValue(new RebaseConflictError('main', ['a.txt']))
  await expect(h.runtime.mergeTicket(ticket.id))
    .rejects.toThrow(/^Rebasing onto main conflicts in a\.txt; the Ticket branch keeps its commits$/)
  expect(h.runtime.store.tickets.get(ticket.id)?.ended?.by).toBe('human')
})

it('pushes the merged branch when the person asked for a push, and reports a push it cannot make', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  expect(await h.runtime.mergeTicket(ticket.id, 'ff', { push: true }))
    .toEqual({ branch: 'agent/t', target: 'main', pushed: true })
  expect(pushWorkspace).toHaveBeenCalled()
  const { ticket: other } = await h.acceptedTicket(feishu('2', '@bot 再修一个'))
  ended(other.id)
  vi.mocked(pushWorkspace).mockRejectedValueOnce(new Error('fatal: no upstream branch'))
  // The merge stands; the push's refusal is what the report adds.
  expect(await h.runtime.mergeTicket(other.id, 'ff', { push: true }))
    .toMatchObject({ branch: 'agent/t', target: 'main', pushError: 'fatal: no upstream branch' })
  expect(h.runtime.store.tickets.get(other.id)?.merged).toBeDefined()
})

it('refuses to rebase for a merge when the deployment requires an approval', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(ticket.id)!
    current.reviewed = { at: '2026-11-19T00:02:00.000Z', decision: 'approved' }
    h.runtime.store.tickets.save(current)
  })
  const settings = h.runtime.store.settings()
  h.runtime.store.saveSettings({ ...settings, executor: { ...settings.executor, requireReview: true } })
  vi.mocked(targetContained).mockResolvedValue(false)
  const rebase = vi.spyOn(h.runtime, 'rebaseTicket')
  await expect(h.runtime.mergeTicket(ticket.id)).rejects.toThrow(/rebase it from the ticket page and approve/)
  expect(rebase).not.toHaveBeenCalled()
})

it('removes the worktree when the merge was asked for with cleanup', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  const cleanup = vi.spyOn(h.runtime, 'cleanupTicket').mockResolvedValue()
  expect(await h.runtime.mergeTicket(ticket.id, 'ff', { cleanup: true }))
    .toEqual({ branch: 'agent/t', target: 'main', cleaned: true })
  expect(cleanup).toHaveBeenCalledWith(ticket.id)
})

it('reports a cleanup failure after the merge already moved the branch', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  vi.spyOn(h.runtime, 'cleanupTicket').mockRejectedValue(new Error('git cannot remove it'))
  await expect(h.runtime.mergeTicket(ticket.id, 'ff', { cleanup: true }))
    .rejects.toThrow(/Merged agent\/t into main, but removing its worktree failed: git cannot remove it/)
  expect(h.runtime.store.tickets.get(ticket.id)?.merged).toBeDefined()
})

it('captures the review again when the branch gained commits after its ending, and checks the new tree', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(ticket.id)!
    current.review = { capturedAt: '2026-11-19T00:01:00.000Z', head: 'old', commits: [], files: [], dirty: [] }
    h.runtime.store.tickets.save(current)
  })
  configureCheck('pnpm test')
  vi.mocked(workspaceHead).mockResolvedValue('new')
  const check = vi.spyOn(h.runtime, 'checkTicket').mockResolvedValue(
    { command: 'pnpm test', ok: true, exitCode: 0, at: '2026-11-19T00:04:00.000Z', output: 'passed' })
  expect(await h.runtime.mergeTicket(ticket.id)).toEqual({ branch: 'agent/t', target: 'main', recaptured: true })
  expect(captureReview).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({ id: ticket.id }), expect.anything())
  // The check ran on the tree the old review described, so the merge runs it again.
  expect(check).toHaveBeenCalledWith(ticket.id)
  expect(h.runtime.store.tickets.get(ticket.id)?.review?.head).toBe('new')
})

it('refuses a branch that changed after its review when approvals are required', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(ticket.id)!
    current.review = { capturedAt: '2026-11-19T00:01:00.000Z', head: 'old', commits: [], files: [], dirty: [] }
    current.reviewed = { at: '2026-11-19T00:02:00.000Z', decision: 'approved' }
    h.runtime.store.tickets.save(current)
  })
  const settings = h.runtime.store.settings()
  h.runtime.store.saveSettings({ ...settings, executor: { ...settings.executor, requireReview: true } })
  vi.mocked(workspaceHead).mockResolvedValue('new')
  await expect(h.runtime.mergeTicket(ticket.id)).rejects.toThrow(/changed after its review/)
  expect(captureReview).not.toHaveBeenCalled()
  expect(mergeWorkspace).not.toHaveBeenCalled()
})

it('waits for a ticket it is blocked by to merge first', async () => {
  const { ticket: blocker } = await h.acceptedTicket(feishu('1', '@bot 先做前置'))
  const { ticket: blocked } = await h.acceptedTicket(feishu('2', '@bot 再做后续'))
  h.runtime.blockTicket(blocked.id, blocker.id, true)
  ended(blocked.id)
  await expect(h.runtime.mergeTicket(blocked.id)).rejects.toThrow(new RegExp(`Blocked by ${blocker.id}`))
  expect(mergeWorkspace).not.toHaveBeenCalled()
  // Once the blocker's branch has landed, the blocked ticket merges.
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(blocker.id)!
    current.merged = { at: '2026-11-19T00:05:00.000Z', branch: 'agent/b', target: 'main' }
    h.runtime.store.tickets.save(current)
  })
  expect(await h.runtime.mergeTicket(blocked.id)).toEqual({ branch: 'agent/t', target: 'main' })
})

it('names a discarded blocker as a dependency to remove rather than merge', async () => {
  const { ticket: blocker } = await h.acceptedTicket(feishu('1', '@bot 先做前置'))
  const { ticket: blocked } = await h.acceptedTicket(feishu('2', '@bot 再做后续'))
  h.runtime.blockTicket(blocked.id, blocker.id, true)
  ended(blocked.id)
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(blocker.id)!
    current.discarded = { at: '2026-11-19T00:05:00.000Z' }
    h.runtime.store.tickets.save(current)
  })
  await expect(h.runtime.mergeTicket(blocked.id)).rejects.toThrow(/discarded Ticket never merges/)
})

it('rebases and discards a blocked ticket, which only the merge waits on', async () => {
  const { ticket: blocker } = await h.acceptedTicket(feishu('1', '@bot 先做前置'))
  const { ticket: blocked } = await h.acceptedTicket(feishu('2', '@bot 再做后续'))
  h.runtime.blockTicket(blocked.id, blocker.id, true)
  ended(blocked.id)
  expect(await h.runtime.rebaseTicket(blocked.id)).toEqual({ branch: 'agent/t', target: 'main' })
  expect(rebaseWorkspace).toHaveBeenCalled()
  expect(await h.runtime.discardTicket(blocked.id)).toBeUndefined()
  expect(discardWorkspace).toHaveBeenCalled()
})

it('refuses a ticket that waits for itself', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 先做前置'))
  expect(() => h.runtime.blockTicket(ticket.id, ticket.id, true)).toThrow(/wait for itself/)
})

it('refuses a dependency that would make two tickets wait for each other', async () => {
  const { ticket: first } = await h.acceptedTicket(feishu('1', '@bot 先做前置'))
  const { ticket: second } = await h.acceptedTicket(feishu('2', '@bot 再做后续'))
  h.runtime.blockTicket(second.id, first.id, true)
  expect(() => h.runtime.blockTicket(first.id, second.id, true)).toThrow(/wait for the other/)
  expect(h.runtime.store.tickets.get(first.id)?.blockedBy).toBeUndefined()
})

it('merges a branch the repository already contains without rebasing or checking', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  const rebase = vi.spyOn(h.runtime, 'rebaseTicket')
  expect(await h.runtime.mergeTicket(ticket.id)).toEqual({ branch: 'agent/t', target: 'main' })
  expect(rebase).not.toHaveBeenCalled()
})

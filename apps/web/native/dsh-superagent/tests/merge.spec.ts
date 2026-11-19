/** Merging from the main Session: one request rebases a branch the repository moved past and runs the check it needs. */
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { captureReview, discardWorkspace, mergeWorkspace, patchFingerprint, pushWorkspace, RebaseConflictError, rebaseWorkspace, RevertConflictError, revertWorkspace, targetContained, workspaceHead } from '../src/app/review.ts'
import { workspaceCheck } from '../src/app/check.ts'
import { queuedMerges } from '../src/app/merge-queue.ts'
import { feishu, startRuntime, type Host } from './fixtures/host.ts'

vi.mock('../src/app/check.ts', async importOriginal => {
  const actual = await importOriginal<typeof import('../src/app/check.ts')>()
  return { ...actual, workspaceCheck: vi.fn() }
})
vi.mock('../src/app/review.ts', async importOriginal => {
  const actual = await importOriginal<typeof import('../src/app/review.ts')>()
  return { ...actual, mergeWorkspace: vi.fn(), targetContained: vi.fn(), workspaceHead: vi.fn(), captureReview: vi.fn(),
    rebaseWorkspace: vi.fn(), discardWorkspace: vi.fn(), pushWorkspace: vi.fn(), revertWorkspace: vi.fn(), patchFingerprint: vi.fn() }
})

let h: Host
beforeEach(async () => {
  h = await startRuntime()
  vi.mocked(targetContained).mockReset().mockResolvedValue(true)
  vi.mocked(mergeWorkspace).mockReset().mockResolvedValue({ branch: 'agent/t', target: 'main', root: '/repo' })
  vi.mocked(workspaceHead).mockReset().mockResolvedValue(undefined)
  vi.mocked(captureReview).mockReset().mockResolvedValue(
    { capturedAt: '2026-11-19T00:03:00.000Z', head: 'new', commits: [], files: [], dirty: [] })
  vi.mocked(rebaseWorkspace).mockReset().mockResolvedValue({ branch: 'agent/t', target: 'main', base: 'base' })
  vi.mocked(discardWorkspace).mockReset().mockResolvedValue(undefined)
  vi.mocked(pushWorkspace).mockReset().mockResolvedValue(undefined)
  vi.mocked(revertWorkspace).mockReset().mockResolvedValue({ target: 'main', head: 'undo-sha' })
  vi.mocked(workspaceCheck).mockReset().mockResolvedValue(
    { command: 'pnpm test', ok: true, exitCode: 0, at: '2026-11-19T00:02:00.000Z', output: 'passed' })
  vi.mocked(patchFingerprint).mockReset().mockResolvedValue('same-changes')
})
afterEach(async () => { await h.stop() })
/** Notices injected into the main Session, parsed from the message text. */
const notices = () => h.agents.get(h.runtime.masterId)!.inject.mock.calls
  .map(([message]) => JSON.parse(message.content.map(block => block.type === 'text' ? block.text : '').join('')) as Record<string, unknown>)

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
  expect(await h.runtime.mergeTicket(ticket.id)).toMatchObject({ branch: 'agent/t', target: 'main', rebased: true })
  expect(rebase).toHaveBeenCalledWith(ticket.id, 'panel')
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
  expect(cleanup).toHaveBeenCalledWith(ticket.id, 'panel')
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
  expect(await h.runtime.mergeTicket(ticket.id)).toMatchObject({ branch: 'agent/t', target: 'main', recaptured: true })
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

it('records which interface merged, the commit it left, and the remote answer to a push', async () => {
  vi.mocked(mergeWorkspace).mockResolvedValue({ branch: 'agent/t', target: 'main', head: 'merged-sha', root: '/repo' })
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  vi.mocked(pushWorkspace).mockRejectedValueOnce(new Error('fatal: no upstream branch'))
  await expect(h.runtime.mergeTicket(ticket.id, 'ff', { push: true })).resolves.toMatchObject({ pushError: 'fatal: no upstream branch' })
  // The panel is the default interface, and the failed push is part of the record instead of only of the report.
  expect(h.runtime.store.tickets.get(ticket.id)?.merged).toMatchObject({ via: 'panel', head: 'merged-sha', root: '/repo',
    push: { ok: false, error: 'fatal: no upstream branch', via: 'panel' } })
  // The panel keeps the outcome even when the person has left the chat that asked.
  expect(h.runtime.notifications().items).toContainEqual(expect.objectContaining({ kind: 'merged', ticketId: ticket.id,
    text: expect.stringContaining('the push failed: fatal: no upstream branch') }))
})

it('records that the main Session asked for the merge, which may name the ticket by number', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  await h.runtime.mergeTicket('T-0001', 'ff', {}, 'chat')
  expect(h.runtime.store.tickets.get(ticket.id)?.merged?.via).toBe('chat')
})

it('carries out a standing merge request and clears it once the merge lands', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  expect(h.runtime.autoMergeTicket(ticket.id, {})).toMatchObject({ autoMerge: { attempts: 0 } })
  await h.runtime.runAutoMerge()
  const merged = h.runtime.store.tickets.get(ticket.id)!
  expect(merged.merged).toMatchObject({ branch: 'agent/t', target: 'main', via: 'auto' })
  expect(merged.autoMerge).toBeUndefined()
  await vi.waitFor(() => expect(notices().some(item => 'auto_merged' in item)).toBe(true))
})

it('waits for a dependency, records why, and retries only once the wait is over', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const { ticket: blocker } = await h.acceptedTicket(feishu('2', '@bot 先做这个'))
  ended(ticket.id)
  h.runtime.blockTicket(ticket.id, blocker.id, true)
  h.runtime.autoMergeTicket(ticket.id, {})
  await h.runtime.runAutoMerge()
  const waiting = h.runtime.store.tickets.get(ticket.id)!
  expect(waiting.merged).toBeUndefined()
  expect(waiting.autoMerge).toMatchObject({ attempts: 1, last: { error: expect.stringMatching(/Blocked by/) } })
  expect(waiting.autoMerge?.next).toBeDefined()
  // The person hears the first gate; the next pass waits for the recorded delay rather than running the check again.
  await vi.waitFor(() => expect(notices().some(item => 'auto_merge_blocked' in item)).toBe(true))
  await h.runtime.runAutoMerge()
  expect(h.runtime.store.tickets.get(ticket.id)?.autoMerge?.attempts).toBe(1)
})

it('gives up on a standing merge request after its attempts run out', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const { ticket: blocker } = await h.acceptedTicket(feishu('2', '@bot 先做这个'))
  ended(ticket.id)
  h.runtime.blockTicket(ticket.id, blocker.id, true)
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(ticket.id)!
    current.autoMerge = { at: '2026-11-19T00:00:00.000Z', attempts: 7, last: { at: '2026-11-19T00:00:00.000Z', error: 'Blocked by t2' } }
    h.runtime.store.tickets.save(current)
  })
  await h.runtime.runAutoMerge()
  expect(h.runtime.store.tickets.get(ticket.id)?.autoMerge).toBeUndefined()
  await vi.waitFor(() => expect(notices().some(item => 'auto_merge_gave_up' in item)).toBe(true))
  expect(h.runtime.notifications().items).toContainEqual(expect.objectContaining({ kind: 'automerge', ticketId: ticket.id }))
})

it('cancels a standing merge request, and a person stopping the ticket clears it', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  h.runtime.autoMergeTicket(ticket.id, {})
  expect(h.runtime.cancelAutoMerge(ticket.id).autoMerge).toBeUndefined()
  // No later pass picks the request up again.
  expect(h.runtime.engine.dueAutoMerge()).toEqual([])
  h.runtime.autoMergeTicket(ticket.id, {})
  // A stop withdraws the request with the ticket's mandate.
  h.runtime.engine.stopTicket(ticket.id)
  expect(h.runtime.store.tickets.get(ticket.id)?.autoMerge).toBeUndefined()
  expect(h.runtime.engine.dueAutoMerge()).toEqual([])
})

it('undoes a recorded merge on the branch it moved, and records the reverting commit', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  vi.mocked(mergeWorkspace).mockResolvedValue({ branch: 'agent/t', target: 'main', head: 'merged-sha', base: 'before-sha',
    root: '/repo' })
  await h.runtime.mergeTicket('T-0001')
  const merged = h.runtime.store.tickets.get(ticket.id)!.merged!
  expect(merged).toMatchObject({ root: '/repo', base: 'before-sha' })
  // A fast-forward merge moved the target through the Ticket's commits, so the base travels with the revert.
  expect(merged.strategy).toBeUndefined()
  expect(await h.runtime.revertTicket('T-0001')).toEqual({ ticket_id: ticket.id, branch: 'agent/t', target: 'main', commit: 'undo-sha' })
  expect(revertWorkspace).toHaveBeenCalledWith(expect.anything(), '/repo', expect.anything(), merged.head, 'main',
    merged.base)
  expect(h.runtime.store.tickets.get(ticket.id)?.reverted).toMatchObject({ commit: 'undo-sha', target: 'main',
    merge: merged.head, via: 'panel' })
  expect(h.runtime.notifications().items).toContainEqual(expect.objectContaining({ kind: 'reverted', ticketId: ticket.id }))
  // A merge is undone once; the record says which commit did it.
  await expect(h.runtime.revertTicket(ticket.id)).rejects.toThrow(/already reverted/)
})

it('reports a conflicting revert without changing the branch, and refuses a ticket without a merge', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  vi.mocked(mergeWorkspace).mockResolvedValue({ branch: 'agent/t', target: 'main', head: 'merged-sha', root: '/repo' })
  await h.runtime.mergeTicket(ticket.id)
  vi.mocked(revertWorkspace).mockRejectedValueOnce(new RevertConflictError('a.txt', ['a.txt']))
  await expect(h.runtime.revertTicket(ticket.id)).rejects.toThrow(/conflicts in a\.txt/)
  expect(h.runtime.store.tickets.get(ticket.id)?.reverted).toBeUndefined()
  const { ticket: open } = await h.acceptedTicket(feishu('2', '@bot 还没合'))
  await expect(h.runtime.revertTicket(open.id)).rejects.toThrow(/Only a merged ticket/)
})

it('keeps an approval when a rebase moves the same changes onto a newer commit, and drops it when they change', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  const approve = (): void => {
    h.runtime.store.atomic(() => {
      const current = h.runtime.store.tickets.get(ticket.id)!
      current.execution = { ...current.execution!, base: 'old-base' }
      current.review = { capturedAt: '2026-11-19T00:00:00.000Z', head: 'old-head', commits: [], files: [], dirty: [] }
      h.runtime.store.tickets.save(current)
    })
    h.runtime.reviewTicket(ticket.id, 'approved')
  }
  approve()
  await h.runtime.rebaseTicket(ticket.id)
  // The capture the rebase made names the new commit, and the approval still covers the same changes.
  expect(h.runtime.store.tickets.get(ticket.id)?.reviewed).toMatchObject({ decision: 'approved', head: 'new' })
  expect(h.runtime.store.tickets.get(ticket.id)?.reviews?.at(-1)).toMatchObject({ decision: 'approved', head: 'old-head' })
  // Changes that differ after the rebase leave the approval behind.
  approve()
  vi.mocked(patchFingerprint).mockResolvedValueOnce('before').mockResolvedValueOnce('after')
  await h.runtime.rebaseTicket(ticket.id)
  expect(h.runtime.store.tickets.get(ticket.id)?.reviewed).toBeUndefined()
})

it('does not re-run a check that already failed for the same ending on an automatic attempt', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  configureCheck('pnpm test')
  const at = new Date().toISOString()
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(ticket.id)!
    current.ended = { at, by: 'executor', outcome: 'completed' }
    current.checks = { command: 'pnpm test', ok: false, exitCode: 1, at, output: 'one test failed' }
    h.runtime.store.tickets.save(current)
  })
  vi.mocked(workspaceCheck).mockClear()
  // The automatic attempt reports the recorded failure instead of spending the same minutes again.
  await expect(h.runtime.mergeTicket(ticket.id, 'ff', {}, 'auto')).rejects.toThrow(/not run again until the Ticket changes/)
  expect(workspaceCheck).not.toHaveBeenCalled()
  // A person asking gets a fresh run.
  await expect(h.runtime.mergeTicket(ticket.id, 'ff', {}, 'chat')).resolves.toMatchObject({ target: 'main' })
  expect(workspaceCheck).toHaveBeenCalled()
})

it('announces a followed pull request\'s failing CI once, and again when the result changes', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  h.runtime.store.atomic(() => {
    h.runtime.store.threads.save({ key: 'github:42', ref: { source: 'github', id: '42' }, createdAt: '2026-11-19T00:00:00.000Z',
      subscriptions: [ticket.id], correlated: [], pending: [],
      meta: { facts: { ci: { value: { zh: '2 项检查失败', en: '2 checks failing' }, problem: true } } } })
    const current = h.runtime.store.tickets.get(ticket.id)!
    current.follows = [{ thread: 'github:42', own: true }]
    h.runtime.store.tickets.save(current)
  })
  // A scan per pass is throttled; each step here clears the throttle to exercise the dedupe itself.
  const scan = async (): Promise<void> => {
    (h.runtime as unknown as { ciCheckedAt: number }).ciCheckedAt = 0
    await h.runtime.pump()
  }
  await scan()
  const failing = (): number => h.runtime.notifications().items.filter(item => item.kind === 'ci').length
  expect(failing()).toBe(1)
  expect(h.runtime.notifications().items.find(item => item.kind === 'ci')?.text).toContain('2 checks failing')
  await scan()
  expect(failing()).toBe(1)
  // A changed CI result is news again.
  h.runtime.store.threads.describe('github:42', { facts: { ci: { value: '3 checks failing', problem: true } } })
  await scan()
  expect(failing()).toBe(2)
  // A pass that clears the problem is remembered as recovered, so the same failure later is news again.
  h.runtime.store.threads.describe('github:42', { facts: { ci: { value: 'passed' } } })
  await scan()
  h.runtime.store.threads.describe('github:42', { facts: { ci: { value: '3 checks failing', problem: true } } })
  await scan()
  expect(failing()).toBe(3)
})

it('checks the merged branch on the target after merging, and reports a failure', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  configureCheck('pnpm test')
  // The gate's run passes; the run on the merged target is the one that fails.
  vi.mocked(workspaceCheck)
    .mockResolvedValueOnce({ command: 'pnpm test', ok: true, exitCode: 0, at: '2026-11-19T00:05:00.000Z', output: 'passed' })
    .mockResolvedValueOnce({ command: 'pnpm test', ok: false, exitCode: 1, at: '2026-11-19T00:06:00.000Z', output: '1 test failed' })
  const result = await h.runtime.mergeTicket(ticket.id)
  expect(result).toMatchObject({ targetCheck: { ok: false, exitCode: 1 } })
  expect(h.runtime.store.tickets.get(ticket.id)?.targetCheck).toMatchObject({ ok: false, output: '1 test failed' })
  // The merge stands; the failing merged tree is what a person hears about, along with the way to undo it.
  expect(h.runtime.store.tickets.get(ticket.id)?.merged).toBeDefined()
  expect(h.runtime.notifications().items).toContainEqual(expect.objectContaining({ kind: 'check', ticketId: ticket.id,
    text: expect.stringMatching(/failed on main after merging.*can be undone/) }))
})

it('merges queued tickets one after another, one queue entry at a time', async () => {
  const { ticket: first } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const { ticket: second } = await h.acceptedTicket(feishu('2', '@bot 再修一个'))
  ended(first.id)
  ended(second.id)
  expect(h.runtime.queueMerges(['T-0001', 'T-0002'], {})).toEqual({ queued: [{ ticket_id: 'T-0001', position: 1 },
    { ticket_id: 'T-0002', position: 2 }], queue: [{ ticket_id: 'T-0001', position: 1 }, { ticket_id: 'T-0002', position: 2 }] })
  expect(queuedMerges(h.runtime.store)).toHaveLength(2)
  // One drain works through the whole queue in order.
  await h.runtime.runMergeQueue()
  expect(h.runtime.store.tickets.get(first.id)?.merged).toBeDefined()
  expect(h.runtime.store.tickets.get(second.id)?.merged).toBeDefined()
  expect(queuedMerges(h.runtime.store)).toHaveLength(0)
  // A queued merge is the person's request the host carried out.
  expect(h.runtime.store.tickets.get(first.id)?.merged?.via).toBe('auto')
  await vi.waitFor(() => expect(notices().filter(item => 'merge_queue' in item)
    .map(item => (item.merge_queue as { ticket_id: string }).ticket_id)).toEqual([first.id, second.id]))
})

it('reports a queued merge that cannot land, and still merges the ones after it', async () => {
  const { ticket: blocked } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const { ticket: blocker } = await h.acceptedTicket(feishu('2', '@bot 先做这个'))
  const { ticket: later } = await h.acceptedTicket(feishu('3', '@bot 之后做'))
  ended(blocked.id)
  ended(later.id)
  h.runtime.blockTicket(blocked.id, blocker.id, true)
  h.runtime.queueMerges([blocked.id, later.id], {})
  await h.runtime.runMergeQueue()
  // The blocked Ticket leaves the queue with its reason, and the next one merges.
  expect(h.runtime.store.tickets.get(blocked.id)?.merged).toBeUndefined()
  expect(h.runtime.store.tickets.get(later.id)?.merged).toBeDefined()
  expect(queuedMerges(h.runtime.store)).toHaveLength(0)
  await vi.waitFor(() => expect(notices().some(item => 'merge_queue' in item
    && String((item.merge_queue as { error?: string }).error ?? '').match(/Blocked by/))).toBe(true))
})

it('takes a ticket out of the merge queue, and skips one that moved on meanwhile', async () => {
  const { ticket: first } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const { ticket: second } = await h.acceptedTicket(feishu('2', '@bot 再修一个'))
  ended(first.id)
  ended(second.id)
  h.runtime.queueMerges([first.id, second.id], {})
  expect(h.runtime.dequeueMerges(['T-0001'])).toMatchObject({ removed: 1, queue: [{ position: 1 }] })
  await h.runtime.runMergeQueue()
  expect(h.runtime.store.tickets.get(first.id)?.merged).toBeUndefined()
  expect(h.runtime.store.tickets.get(second.id)?.merged).toBeDefined()
  // A Ticket merged by someone else while it waited is skipped rather than merged twice.
  const { ticket: third } = await h.acceptedTicket(feishu('3', '@bot 再修一个'))
  ended(third.id)
  h.runtime.queueMerges([third.id], {})
  h.runtime.store.atomic(() => {
    const current = h.runtime.store.tickets.get(third.id)!
    current.merged = { at: '2026-11-19T00:06:00.000Z', branch: 'agent/t3', target: 'main' }
    h.runtime.store.tickets.save(current)
  })
  await h.runtime.runMergeQueue()
  expect(queuedMerges(h.runtime.store)).toHaveLength(0)
  await vi.waitFor(() => expect(notices().some(item => 'merge_queue' in item
    && 'skipped' in (item.merge_queue as Record<string, unknown>))).toBe(true))
})

it('refuses to queue a ticket that is not an ended one it could merge', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  expect(() => h.runtime.queueMerges([ticket.id], {})).toThrow(/cannot be queued/)
  ended(ticket.id)
  h.runtime.queueMerges([ticket.id], {})
  expect(() => h.runtime.queueMerges(['T-9999'], {})).toThrow(/Unknown ticket/)
})

it('retries a push that failed on the branch the merge moved, and records the answer', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  vi.mocked(pushWorkspace).mockRejectedValueOnce(new Error('fatal: no upstream branch'))
  await h.runtime.mergeTicket(ticket.id, 'ff', { push: true })
  expect(h.runtime.store.tickets.get(ticket.id)?.merged?.push).toMatchObject({ ok: false, via: 'panel',
    error: 'fatal: no upstream branch' })
  expect(await h.runtime.pushTicket('T-0001')).toEqual({ ticket_id: ticket.id, pushed: true })
  expect(pushWorkspace).toHaveBeenLastCalledWith(expect.anything(), '/repo', expect.anything(), 'main')
  expect(h.runtime.store.tickets.get(ticket.id)?.merged?.push).toMatchObject({ ok: true, via: 'panel' })
  expect(h.runtime.notifications().items.filter(item => item.kind === 'push')).toHaveLength(1)
  // A failure of the retry is recorded and reported too, and a Ticket that never merged cannot be pushed.
  vi.mocked(pushWorkspace).mockRejectedValueOnce(new Error('fatal: no upstream branch'))
  await expect(h.runtime.pushTicket(ticket.id)).rejects.toThrow(/no upstream branch/)
  expect(h.runtime.store.tickets.get(ticket.id)?.merged?.push).toMatchObject({ ok: false })
  expect(h.runtime.notifications().items.filter(item => item.kind === 'push')).toHaveLength(2)
  const { ticket: open } = await h.acceptedTicket(feishu('2', '@bot 还没合'))
  await expect(h.runtime.pushTicket(open.id)).rejects.toThrow(/Only a merged ticket/)
})

it('pauses the queue when the check on the merged target fails, leaving the rest waiting', async () => {
  const { ticket: first } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  const { ticket: second } = await h.acceptedTicket(feishu('2', '@bot 再修一个'))
  ended(first.id)
  ended(second.id)
  configureCheck('pnpm test')
  // The first Ticket's gate passes; the check on the merged target fails.
  vi.mocked(workspaceCheck)
    .mockResolvedValueOnce({ command: 'pnpm test', ok: true, exitCode: 0, at: '2026-11-19T00:05:00.000Z', output: 'passed' })
    .mockResolvedValueOnce({ command: 'pnpm test', ok: false, exitCode: 1, at: '2026-11-19T00:06:00.000Z', output: 'broke main' })
  h.runtime.queueMerges([first.id, second.id], {})
  await h.runtime.runMergeQueue()
  expect(h.runtime.store.tickets.get(first.id)?.targetCheck).toMatchObject({ ok: false })
  // A red target branch holds the queue; the next Ticket waits rather than merging into it.
  expect(h.runtime.mergeQueue()).toMatchObject({ paused: true, items: [{ ticket_id: second.id, title: expect.any(String), position: 1 }] })
  expect(h.runtime.store.tickets.get(second.id)?.merged).toBeUndefined()
  expect(h.runtime.notifications().items).toContainEqual(expect.objectContaining({ kind: 'check',
    text: expect.stringContaining('merge queue is paused') }))
  // Resuming continues with the Ticket that waited.
  vi.mocked(workspaceCheck).mockResolvedValue({ command: 'pnpm test', ok: true, exitCode: 0, at: '2026-11-19T00:07:00.000Z',
    output: 'passed' })
  h.runtime.pauseMergeQueue(false)
  await h.runtime.runMergeQueue()
  expect(h.runtime.store.tickets.get(second.id)?.merged).toBeDefined()
  expect(h.runtime.mergeQueue()).toMatchObject({ paused: false, items: [] })
})

/** Merging from the main Session: one request rebases a branch the repository moved past and runs the check it needs. */
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { mergeWorkspace, targetContained } from '../src/app/review.ts'
import { feishu, startRuntime, type Host } from './fixtures/host.ts'

vi.mock('../src/app/review.ts', async importOriginal => {
  const actual = await importOriginal<typeof import('../src/app/review.ts')>()
  return { ...actual, mergeWorkspace: vi.fn(), targetContained: vi.fn() }
})

let h: Host
beforeEach(async () => {
  h = await startRuntime()
  vi.mocked(targetContained).mockReset().mockResolvedValue(true)
  vi.mocked(mergeWorkspace).mockReset().mockResolvedValue({ branch: 'agent/t', target: 'main' })
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

it('reports a conflicting rebase and leaves the branch unmerged', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  vi.mocked(targetContained).mockResolvedValue(false)
  vi.spyOn(h.runtime, 'rebaseTicket').mockRejectedValue(
    new Error('Rebasing onto main conflicts in a.txt; the Ticket branch keeps its commits'))
  await expect(h.runtime.mergeTicket(ticket.id)).rejects.toThrow(/conflicts in a\.txt/)
  expect(mergeWorkspace).not.toHaveBeenCalled()
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

it('merges a branch the repository already contains without rebasing or checking', async () => {
  const { ticket } = await h.acceptedTicket(feishu('1', '@bot 修一下回归'))
  ended(ticket.id)
  const rebase = vi.spyOn(h.runtime, 'rebaseTicket')
  expect(await h.runtime.mergeTicket(ticket.id)).toEqual({ branch: 'agent/t', target: 'main' })
  expect(rebase).not.toHaveBeenCalled()
})

import { Context } from '@deepseek-ai/cordis'
import { mkdir, mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Ticket } from '../src/core/index.ts'
import { adoptWorktrees, linkWorktree, presentWorktree, removeWorktree, retitleWorktree, sessionGroupWorkspace, ticketSessionGroup } from '../src/app/worktrees.ts'
import { workspaceRegistryStub } from './fixtures/host.ts'

const tickets: string[] = []
afterEach(async () => {
  for (const directory of tickets.splice(0)) await rm(directory, { recursive: true, force: true })
})

/** One Ticket whose checkout is `directory`, with the facts the sidebar presentation reads. */
const ticket = (directory: string, name: string, title = 'Ticket title', ended = false): Ticket =>
  ({ id: 'ticket-1', title, execution: { name, directory, assignment: '' },
    ...ended ? { ended: { at: '2026-01-01T00:00:00.000Z', by: 'executor', outcome: 'completed' } } : {} } as Ticket)

/** A Host context whose Workspace registry is the fixture's in-memory one. */
function context() {
  const ctx = new Context()
  const registry = workspaceRegistryStub()
  for (const [name, service] of Object.entries({ workspaceRegistry: registry })) ctx.reflect.provide(name, service)
  return { ctx, registry }
}

const directory = async (name: string): Promise<string> => {
  const parent = await mkdtemp(join(tmpdir(), 'dsh-superagent-worktree-'))
  tickets.push(parent)
  const target = join(parent, name)
  await mkdir(target, { recursive: true })
  return target
}

describe('ticket worktree presentation', () => {
  it('links a running checkout to the repository Workspace with the Ticket title and checkout subtitle', async () => {
    const { ctx, registry } = context()
    const root = await directory('repo')
    const checkout = await directory('ticket')
    const parent = await registry.create(root)
    await linkWorktree(ctx, ticket(checkout, 'fix-regression-ab12'), root)
    const workspace = registry.list().find(candidate => candidate.path === checkout)!
    expect(workspace.title).toBe('Ticket title')
    expect(workspace.worktree).toEqual({ name: 'fix-regression-ab12', active: true })
    expect(workspace.parentId).toBe(parent.id)
  })

  it('adopts checkouts recorded before the presentation existed and settles a finished one', async () => {
    const { ctx, registry } = context()
    const root = await directory('repo')
    const checkout = await directory('ticket')
    await registry.create(root)
    await registry.create(checkout)
    const live = ticket(checkout, 'fix-regression-ab12')
    const adopt = vi.fn(async () => root)
    expect(await adoptWorktrees(ctx, [live], adopt)).toBe(1)
    let workspace = registry.list().find(candidate => candidate.path === checkout)!
    expect(workspace.worktree).toEqual({ name: 'fix-regression-ab12', active: true })
    expect(registry.list().find(candidate => candidate.path === checkout)?.parentId).toBe(registry.list().find(candidate => candidate.path === root)?.id)
    // A presentation that already matches is left alone, without resolving the repository again.
    const settled = vi.fn(async () => root)
    await linkWorktree(ctx, { ...live, ended: { at: '2026-01-01T00:00:00.000Z', by: 'executor', outcome: 'completed' } }, root, false)
    expect(await adoptWorktrees(ctx, [{ ...live, ended: { at: '2026-01-01T00:00:00.000Z', by: 'executor', outcome: 'completed' } }], settled)).toBe(0)
    expect(settled).not.toHaveBeenCalled()
    workspace = registry.list().find(candidate => candidate.path === checkout)!
    expect(workspace.worktree?.active).toBe(false)
  })

  it('removes the registration of a finished checkout whose directory is gone', async () => {
    const { ctx, registry } = context()
    const checkout = await directory('ticket')
    const presented = ticket(checkout, 'fix-regression-ab12', 'Ticket title', true)
    await registry.create(checkout)
    await rm(checkout, { recursive: true })
    expect(await adoptWorktrees(ctx, [presented], async () => undefined)).toBe(1)
    expect(registry.list()).toEqual([])
  })

  it('keeps an existing parent relationship while its repository Workspace is unregistered', async () => {
    const { ctx, registry } = context()
    const root = await directory('repo')
    const checkout = await directory('ticket')
    const parent = await registry.create(root)
    const presented = ticket(checkout, 'fix-regression-ab12')
    await linkWorktree(ctx, presented, root)
    await registry.delete(parent.id)
    await linkWorktree(ctx, presented, root)
    expect(registry.list().find(candidate => candidate.path === checkout)?.parentId).toBe(parent.id)
    const recreated = await registry.create(root)
    await linkWorktree(ctx, presented, root)
    expect(registry.list().find(candidate => candidate.path === checkout)?.parentId).toBe(recreated.id)
  })

  it('keeps a checkout whose repository Workspace is not registered at the top level', async () => {
    const { ctx, registry } = context()
    const checkout = await directory('orphan')
    await linkWorktree(ctx, ticket(checkout, 'fix-regression-ab12'), undefined)
    const workspace = registry.list().find(candidate => candidate.path === checkout)!
    expect(workspace.parentId).toBeUndefined()
    expect(workspace.worktree?.active).toBe(true)
  })

  it('marks the entry finished and running again, then removes it with the checkout', async () => {
    const { ctx, registry } = context()
    const root = await directory('repo')
    const checkout = await directory('ticket')
    await registry.create(root)
    const presented = ticket(checkout, 'fix-regression-ab12')
    await linkWorktree(ctx, presented, root)
    await presentWorktree(ctx, { ...presented, title: 'Renamed ticket' }, false)
    let workspace = registry.list().find(candidate => candidate.path === checkout)!
    expect(workspace.title).toBe('Renamed ticket')
    expect(workspace.worktree?.active).toBe(false)
    await presentWorktree(ctx, presented, true)
    workspace = registry.list().find(candidate => candidate.path === checkout)!
    expect(workspace.worktree?.active).toBe(true)
    await removeWorktree(ctx, presented)
    expect(registry.list().some(candidate => candidate.path === checkout)).toBe(false)
  })

  it('drops a finished entry whose checkout directory is gone', async () => {
    const { ctx, registry } = context()
    const checkout = await directory('ticket')
    const presented = ticket(checkout, 'fix-regression-ab12')
    await linkWorktree(ctx, presented, undefined)
    await rm(checkout, { recursive: true })
    await presentWorktree(ctx, presented, false)
    expect(registry.list()).toEqual([])
  })

  it('names the repository group and merge state its worker Session records', async () => {
    const { ctx, registry } = context()
    const root = await directory('repo')
    const checkout = await directory('ticket')
    const running = ticket(checkout, 'fix-regression-ab12')
    await registry.create(root)
    expect(ticketSessionGroup(running, root)).toEqual({ path: root, role: 'ticket' })
    const merged = { ...running, execution: { ...running.execution!, root },
      merged: { at: '2026-01-01T00:00:00.000Z', branch: 'fix-regression-ab12', target: 'main', root } }
    expect(ticketSessionGroup(merged, root)).toEqual({ path: root, role: 'ticket', merged: true })
    for (const outcome of ['completed', 'closed', 'dropped', 'stopped'] as const) {
      expect(ticketSessionGroup({ ...running, ended: { at: '2026-01-01T00:00:00.000Z', by: 'human', outcome } }, root))
        .toEqual({ path: root, role: 'ticket', outcome })
    }
    expect(ticketSessionGroup({ ...merged, ended: { at: '2026-01-01T00:00:00.000Z', by: 'executor', outcome: 'completed' } }, root))
      .toEqual({ path: root, role: 'ticket', merged: true })
    // A live checkout's registered parent answers even when the recorded root is unset.
    await linkWorktree(ctx, running, root)
    expect((await sessionGroupWorkspace(ctx, running))?.path).toBe(root)
    // After the checkout registration is gone, the recorded root answers.
    await removeWorktree(ctx, running)
    expect(await sessionGroupWorkspace(ctx, { ...running, execution: { ...running.execution!, root } }))
      .toMatchObject({ path: root })
    expect(registry.list().map(workspace => workspace.path)).toEqual([root])
  })

  it('retitles a worktree entry with its Ticket', async () => {
    const { ctx, registry } = context()
    const checkout = await directory('ticket')
    const presented = ticket(checkout, 'fix-regression-ab12')
    await linkWorktree(ctx, presented, undefined)
    await retitleWorktree(ctx, { ...presented, title: 'Renamed ticket' })
    expect(registry.list().find(candidate => candidate.path === checkout)?.title).toBe('Renamed ticket')
  })
})

/**
 * Presents a Ticket's checkout in DSH's Workspace sidebar: a worktree child of
 * the repository's Workspace while the Ticket runs, and Sessions folded into
 * that parent once it ends. The Workspace registry keeps the facts; this
 * module only derives them from a Ticket's Execution and the checkout.
 * @module @deepseek-ai/dsh-superagent/src/app/worktrees
 */
import type { Context } from '@deepseek-ai/cordis'
import { appendWorkspaceGroup, workspaceGroupOf, type SessionWorkspaceGroup } from '@deepseek-ai/dsh-api-session-controller'
import type { Session } from '@deepseek-ai/dsh-session'
import type { Workspace } from '@deepseek-ai/dsh-workspace'
import { realpathNormalize } from '@deepseek-ai/dsh-workspace'
import type { Ticket } from '../core/index.ts'

/** The registered Workspace at `directory`, matched through the canonical path the registry stores. */
async function workspaceAt(ctx: Context, directory: string): Promise<Workspace | undefined> {
  const registry = ctx.get('workspaceRegistry')
  if (registry === undefined || directory === '') return undefined
  const canonical = await realpathNormalize(directory).catch(() => directory)
  return registry.list().find(workspace => workspace.path === canonical)
}

/** The registered Workspace at `root`, matched through the canonical path the registry stores. */
async function parentAt(ctx: Context, root: string | undefined): Promise<Workspace | undefined> {
  return root === undefined ? undefined : await workspaceAt(ctx, root)
}

/**
 * Registers or refreshes the sidebar presentation of a running Ticket's
 * checkout: its Workspace shows the Ticket title, the worktree name as the
 * subtitle, and displays under the repository's Workspace when that
 * registration exists. A Ticket without a repository Workspace stays a
 * top-level workspace, and a checkout that is not a git worktree keeps no
 * parent.
 * @param ctx - Host context carrying the Workspace registry.
 * @param ticket - Ticket whose checkout is presented.
 * @param root - Repository root the checkout belongs to, when git reported one.
 * @param active - Whether the Ticket runs: a finished entry folds its Sessions into the parent.
 */
export async function linkWorktree(ctx: Context, ticket: Ticket, root: string | undefined, active = true): Promise<void> {
  const registry = ctx.get('workspaceRegistry')
  const execution = ticket.execution
  if (registry === undefined || execution === undefined) return
  const workspace = await registry.create(execution.directory)
  const parent = await parentAt(ctx, root)
  // An existing relationship survives while its parent is unregistered; a registered repository takes over again.
  const parentId = parent === undefined || parent.id === workspace.id ? workspace.parentId : parent.id
  if (workspace.title !== ticket.title) await workspace.setTitle(ticket.title)
  await workspace.setPresentation({
    ...parentId === undefined ? {} : { parentId },
    worktree: { name: execution.name, active },
  })
}

/**
 * Links the checkouts of Tickets recorded before this presentation existed, so
 * the sidebar nests them without waiting for another launch. A finished
 * Ticket's checkout becomes a finished entry, one whose directory is gone
 * loses its registration, and a checkout that still presents correctly is
 * left alone.
 * @param ctx - Host context carrying the Workspace registry.
 * @param tickets - Tickets whose recorded checkouts are adopted.
 * @param rootOf - Resolves the repository root of one Ticket's checkout; undefined outside a repository.
 * @returns how many entries were linked, settled, or removed.
 */
export async function adoptWorktrees(
  ctx: Context,
  tickets: readonly Ticket[],
  rootOf: (ticket: Ticket) => Promise<string | undefined>,
): Promise<number> {
  const registry = ctx.get('workspaceRegistry')
  if (registry === undefined) return 0
  let adopted = 0
  for (const ticket of tickets) {
    const directory = ticket.execution?.directory
    if (directory === undefined) continue
    const workspace = await workspaceAt(ctx, directory)
    if (workspace === undefined) continue
    if (await workspace.status() === 'missing-dir') {
      if (!ticket.ended) continue
      await registry.delete(workspace.id)
      adopted++
      continue
    }
    const active = !ticket.ended
    if (workspace.worktree !== undefined && workspace.worktree.active === active && workspace.title === ticket.title) continue
    await linkWorktree(ctx, ticket, await rootOf(ticket).catch(() => undefined), active)
    adopted++
  }
  return adopted
}

/**
 * Marks a Ticket's worktree entry as finished or running again, keeping the
 * title current. A finished checkout whose directory is gone loses its
 * registration, so no stale entry survives a merge or cleanup the host did
 * not observe.
 * @param ctx - Host context carrying the Workspace registry.
 * @param ticket - Ticket whose worktree entry changes state.
 * @param active - Whether the Ticket runs again: a finished entry folds its Sessions into the parent.
 */
export async function presentWorktree(ctx: Context, ticket: Ticket, active: boolean): Promise<void> {
  const registry = ctx.get('workspaceRegistry')
  const directory = ticket.execution?.directory
  if (registry === undefined || directory === undefined) return
  const workspace = await workspaceAt(ctx, directory)
  if (workspace?.worktree === undefined) return
  if (!active && await workspace.status() === 'missing-dir') {
    await registry.delete(workspace.id)
    return
  }
  if (workspace.title !== ticket.title) await workspace.setTitle(ticket.title)
  await workspace.setPresentation({
    ...workspace.parentId === undefined ? {} : { parentId: workspace.parentId },
    worktree: { name: workspace.worktree.name, active },
  })
}

/**
 * Keeps a worktree entry's title current after a person renames its Ticket.
 * @param ctx - Host context carrying the Workspace registry.
 * @param ticket - Renamed Ticket.
 */
export async function retitleWorktree(ctx: Context, ticket: Ticket): Promise<void> {
  const directory = ticket.execution?.directory
  if (directory === undefined) return
  const workspace = await workspaceAt(ctx, directory)
  if (workspace?.worktree === undefined || workspace.title === ticket.title) return
  await workspace.setTitle(ticket.title)
}

/**
 * Removes a Ticket workspace's sidebar registration after its checkout is
 * discarded or cleaned up, so the entry disappears with the directory.
 * @param ctx - Host context carrying the Workspace registry.
 * @param ticket - Ticket whose checkout no longer exists.
 */
export async function removeWorktree(ctx: Context, ticket: Ticket): Promise<void> {
  const registry = ctx.get('workspaceRegistry')
  const directory = ticket.execution?.directory
  if (registry === undefined || directory === undefined) return
  const workspace = await workspaceAt(ctx, directory)
  if (workspace !== undefined) await registry.delete(workspace.id)
}

/**
 * The registered Workspace a Ticket's worker Session should be browsed under:
 * the repository Workspace its checkout is nested in while that registration
 * exists, else the one registered at the repository root the Ticket recorded.
 * @param ctx - Host context carrying the Workspace registry.
 * @param ticket - Ticket whose repository Workspace is required.
 * @param fallbackRoot - Repository this installation runs from, for a Ticket that recorded none.
 * @returns the parent Workspace, or undefined when none is registered.
 */
export async function sessionGroupWorkspace(ctx: Context, ticket: Ticket, fallbackRoot?: string): Promise<Workspace | undefined> {
  const registry = ctx.get('workspaceRegistry')
  if (registry === undefined) return undefined
  const worktree = ticket.execution?.directory === undefined ? undefined : await workspaceAt(ctx, ticket.execution.directory)
  const parent = worktree?.parentId === undefined ? undefined : registry.list().find(workspace => workspace.id === worktree.parentId)
  if (parent !== undefined) return parent
  // A Ticket recorded before the root was captured still names its repository or, failing that, uses this installation's.
  return parentAt(ctx, ticket.execution?.root ?? ticket.merged?.root ?? ticket.reverted?.root
    ?? ticket.execution?.repository ?? fallbackRoot)
}

/**
 * The durable sidebar group of a Ticket's worker Session: the repository
 * Workspace, a ticket role marker, and whether the Ticket's branch landed.
 * Read when a row is grouped by this hint rather than by Workspace membership,
 * which starts once the Ticket's own checkout registration is gone.
 * @param ticket - Ticket whose Session is presented.
 * @param path - Registered repository Workspace directory.
 * @returns the grouping hint to append.
 */
export function ticketSessionGroup(ticket: Ticket, path: string): SessionWorkspaceGroup {
  return { path, role: 'ticket', ...ticket.merged ? { merged: true as const } : {} }
}

/**
 * Durably groups a Ticket's worker Session under its repository Workspace,
 * with the role marker and merge state the sidebar shows. Repeating the same
 * facts appends nothing, so a caller may run this whenever a Ticket moves.
 * @param ctx - Host context carrying the Workspace registry and projections.
 * @param ticket - Ticket whose Session is presented.
 * @param session - the live worker Session of that Ticket.
 * @returns whether a newer group was appended.
 */
export async function groupTicketSession(ctx: Context, ticket: Ticket, session: Session): Promise<boolean> {
  const workspace = await sessionGroupWorkspace(ctx, ticket)
  if (workspace === undefined) return false
  const next = ticketSessionGroup(ticket, workspace.path)
  const current = workspaceGroupOf(ctx, session)
  if (current?.path === next.path && current.role === next.role && current.merged === next.merged) return false
  appendWorkspaceGroup(session, next)
  return true
}

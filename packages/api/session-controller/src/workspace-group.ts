/**
 * Durable sidebar grouping override: the Workspace directory a Session is
 * browsed under when its stored header cwd names another project.
 *
 * The value is log data rather than a header field because an existing Session
 * must be able to receive it (`session/workspace-group` appended after
 * resume). It never changes Workspace membership, the header cwd, or the
 * execution directory; the sidebar consumes it only as a display group.
 * @module @deepseek-ai/dsh-api-session-controller/src/workspace-group
 */
import { isAbsolute } from 'node:path'
import type { Context } from '@deepseek-ai/cordis'
import type { Session } from '@deepseek-ai/dsh-session'
import type { ProjectionDefinition } from '@deepseek-ai/dsh-session-projection'
import { z } from 'zod'
import type { SessionWorkspaceGroup } from './types.ts'

const workspaceGroupSchema = z.object({
  path: z.string().min(1),
  role: z.enum(['orchestrator', 'ticket']).optional(),
  merged: z.literal(true).optional(),
  outcome: z.enum(['completed', 'closed', 'dropped', 'stopped']).optional(),
}).strict().nullable()

/** Latest `session/workspace-group` value, or null when the Session has no override. */
export const workspaceGroupProjectionDefinition = {
  key: 'workspaceGroup',
  stateVersion: 1,
  stateSchema: workspaceGroupSchema,
  init: () => null,
  apply: (state, event) => event.type === 'session/workspace-group'
    ? event.data
    : state,
  wire: { viewSchema: workspaceGroupSchema, view: state => state },
} satisfies ProjectionDefinition<'workspaceGroup', SessionWorkspaceGroup | null>

/**
 * The sidebar grouping override a Session's own log records.
 * @param ctx - Host context owning the Session projection registry.
 * @param session - Session whose durable log is folded.
 * @returns the latest grouping override, or null when its cwd decides the group.
 */
export function workspaceGroupOf(ctx: Context, session: Session): SessionWorkspaceGroup | null {
  return ctx.sessionProjections.snapshot(session, ['workspaceGroup']).values.workspaceGroup ?? null
}

/**
 * Record the Workspace directory this Session belongs to for sidebar browsing.
 * The latest accepted value wins; the header cwd is untouched.
 * @param session - live Session whose durable log receives the override.
 * @param group - Workspace directory and optional role marker.
 * @throws when the Workspace directory is not an absolute path.
 */
export function appendWorkspaceGroup(session: Session, group: SessionWorkspaceGroup): void {
  if (!isAbsolute(group.path)) {
    throw new TypeError(`session workspace group must be an absolute path: '${group.path}'`)
  }
  session.append('session/workspace-group', group)
}

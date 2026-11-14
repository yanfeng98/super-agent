/**
 * Permanent removal of one ordinary Session: the stored log and every Host
 * reference to it. Deletion refuses while the Session still runs work, is a
 * subagent conversation, or is held by an Agent or storage handle this
 * controller does not own; nothing is removed before those checks pass.
 * @module @deepseek-ai/dsh-api-session-controller/delete
 */

import type { Context } from '@deepseek-ai/cordis'
import type { SessionHeader, SessionId } from '@deepseek-ai/dsh-session'
import {
  SessionPersistenceBusyError,
  SessionPersistenceNotFoundError,
  type SessionPersistence,
} from '@deepseek-ai/dsh-session-persistence'
import type { SessionActivity } from '@deepseek-ai/dsh-workspace'
import { RemoteError } from '@deepseek-ai/dsh-typert-protocol'
import type { ApiSessionAgentController } from './agent.ts'
import type { SessionDeleteRequest, SessionDeleteValue } from './types.ts'

/** Implements the `session/delete` Remote command over Agent, storage, and Workspace services. */
export class SessionDeletion {
  private tail: Promise<void> = Promise.resolve()

  /**
   * @param ctx - Host context carrying Agents, Session persistence, and the Workspace registry.
   * @param agents - sole owner of the Agent handles deletion must release.
   */
  constructor(
    private readonly ctx: Context,
    private readonly agents: ApiSessionAgentController,
  ) {}

  /**
   * Remove one Session's stored log and every Host reference to it. Requests
   * run one at a time so a repeated delete observes the first one's outcome.
   * @param request - Session identity to remove.
   * @returns confirmation that the Session and its artifacts are gone.
   * @throws {RemoteError} `session/not-found`, `session/subagent`,
   *   `session/active` (with the reported activity), `session/in-use`, or
   *   `gateway/internal`.
   */
  delete(request: SessionDeleteRequest): Promise<SessionDeleteValue> {
    const result = this.tail.then(async () => this.remove(request))
    this.tail = result.then(() => undefined, () => undefined)
    return result
  }

  private async remove(request: SessionDeleteRequest): Promise<SessionDeleteValue> {
    const sessionId = request.sessionId
    const header = await this.requireHeader(sessionId)
    if (header.origin === 'subagent') {
      throw new RemoteError(
        'session/subagent',
        `session "${sessionId}" is a subagent conversation and cannot be deleted on its own`,
        { sessionId },
      )
    }
    const activity = await this.activity(sessionId)
    if (activity.length > 0) {
      throw new RemoteError(
        'session/active',
        `cannot delete session "${sessionId}": the session is active (${activity.map(entry => entry.kind).join(', ')})`,
        { sessionId, activity },
      )
    }
    this.agents.beginDeletion(sessionId)
    try {
      const disposed = await this.agents.disposeAgent(sessionId)
      // A remaining Agent or live store entry would outlive its log. The
      // Agent case is a controller it does not own (a development reload);
      // the store case is any consumer that attached the Session directly.
      if (!disposed && (this.ctx.agents.get(sessionId) !== undefined || this.ctx.sessions.get(sessionId) !== undefined)) {
        throw new RemoteError(
          'session/in-use',
          `session "${sessionId}" is still attached to the Host`,
          { sessionId },
        )
      }
      await this.removeStored(sessionId, disposed)
      await this.forget(sessionId)
      // A disposed Agent already reported removal through `session/disposed`;
      // a cold Session has no such edge, so the list change is published here.
      if (!disposed) this.ctx.emit('api-session/removed', sessionId)
      return { deleted: true }
    } finally {
      this.agents.endDeletion(sessionId)
    }
  }

  /**
   * Read the identity witness for the Session, preferring the live store over
   * a persistence stat.
   * @param sessionId - Session whose header is required.
   * @returns the header the existence and origin checks read.
   */
  private async requireHeader(sessionId: SessionId): Promise<SessionHeader> {
    const live = this.ctx.sessions.get(sessionId)
    if (live !== undefined) return live.header
    let snapshot
    try {
      snapshot = await this.persistence().stat(sessionId)
    } catch (error: unknown) {
      throw new RemoteError(
        'gateway/internal',
        `failed to inspect session "${sessionId}" before deleting it: ${String(error)}`,
        { sessionId },
        { cause: error },
      )
    }
    if (snapshot === undefined) {
      throw new RemoteError('session/not-found', `session "${sessionId}" not found`, { sessionId })
    }
    return snapshot.header
  }

  /**
   * Ask every composed provider what still runs for the Session. The same
   * waterfall archive admission reads; an empty answer means deletion may
   * proceed.
   * @param sessionId - Session about to be removed.
   * @returns the reported activity, in provider order.
   */
  private async activity(sessionId: SessionId): Promise<readonly SessionActivity[]> {
    try {
      return await this.ctx.waterfall(
        'workspace/session-activity',
        { sessionId },
        () => Promise.resolve<readonly SessionActivity[]>([]),
      )
    } catch (error: unknown) {
      throw new RemoteError(
        'gateway/internal',
        `failed to inspect running work for session "${sessionId}": ${String(error)}`,
        { sessionId },
        { cause: error },
      )
    }
  }

  /**
   * Delete the stored artifacts. A live Session that never materialized has
   * none; its disposal already removed the only copy, so the missing
   * artifact confirms the deletion instead of failing it.
   * @param sessionId - Session whose artifacts must go.
   * @param disposed - whether this call disposed the Session's live Agent.
   */
  private async removeStored(sessionId: SessionId, disposed: boolean): Promise<void> {
    try {
      await this.persistence().remove(sessionId)
    } catch (error: unknown) {
      if (error instanceof SessionPersistenceBusyError) {
        throw new RemoteError(
          'session/in-use',
          `session "${sessionId}" is still held by an open storage handle`,
          { sessionId },
          { cause: error },
        )
      }
      if (error instanceof SessionPersistenceNotFoundError) {
        if (disposed) return
        throw new RemoteError('session/not-found', `session "${sessionId}" not found`, { sessionId }, { cause: error })
      }
      throw new RemoteError(
        'gateway/internal',
        `failed to delete stored session "${sessionId}": ${String(error)}`,
        { sessionId },
        { cause: error },
      )
    }
  }

  /**
   * Resolve the mandatory persistence backend explicitly so a composition
   * without one reports a business failure instead of a property error.
   * @returns the session-persistence service.
   */
  private persistence(): SessionPersistence {
    const persistence = this.ctx.get('sessionPersistence')
    if (persistence === undefined) {
      throw new RemoteError(
        'gateway/internal',
        'session deletion is unavailable: this deployment mounts no session-persistence backend',
        {},
      )
    }
    return persistence
  }

  /**
   * Drop the Session's Workspace account, archive, pin, and cached header
   * facts. A failure here leaves only invisible references, so it is logged
   * rather than reported after the log is already gone.
   * @param sessionId - Session whose references must go.
   */
  private async forget(sessionId: SessionId): Promise<void> {
    try {
      await this.ctx.workspaceRegistry.forgetSession(sessionId)
    } catch (error: unknown) {
      this.ctx.logger.warn(
        `session "${sessionId}" was deleted but its workspace accounting could not be cleared: ${String(error)}`,
      )
    }
  }
}

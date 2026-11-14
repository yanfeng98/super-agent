// Permanent Session deletion: refusal order, live-Agent release, stored
// artifact removal, and Workspace reference cleanup.
import { Context } from '@deepseek-ai/cordis'
import { brandString } from '@deepseek-ai/dsh-brand'
import SessionStore, { SESSION_FORMAT_VERSION, type SessionId } from '@deepseek-ai/dsh-session'
import type { SessionHeader } from '@deepseek-ai/dsh-session'
import { SessionPersistenceBusyError, SessionPersistenceNotFoundError } from '@deepseek-ai/dsh-session-persistence'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiSessionAgentController } from '../src/agent.ts'
import { SessionDeletion } from '../src/delete.ts'
import { installSessionReadTestServices, testSessionPersistence } from './test-remote.ts'

const roots: Context[] = []

afterEach(async () => {
  await Promise.all(roots.splice(0).map(ctx => ctx.fiber.dispose()))
})

function header(id: string, origin?: 'subagent'): SessionHeader {
  return {
    version: SESSION_FORMAT_VERSION,
    id: brandString<SessionId>(id),
    createdAt: 1,
    isSeeded: false,
    cwd: '/workspace',
    ...(origin === undefined ? {} : { origin }),
  }
}

interface FixtureOptions {
  readonly stored?: readonly SessionHeader[]
  readonly liveAgent?: boolean
  readonly disposeAgent?: boolean
  readonly remove?: (sessionId: SessionId) => Promise<void>
  readonly forget?: (sessionId: SessionId) => Promise<void>
}

/** Context with a controllable persistence double, Agent lookup, and Workspace cleanup. */
async function fixture(options: FixtureOptions = {}) {
  const ctx = new Context()
  roots.push(ctx)
  await ctx.plugin(SessionStore)
  installSessionReadTestServices(ctx)
  const stored = [...options.stored ?? []]
  const remove = vi.fn(options.remove ?? (async (sessionId: SessionId) => {
    const index = stored.findIndex(entry => entry.id === sessionId)
    if (index < 0) throw new SessionPersistenceNotFoundError(sessionId)
    stored.splice(index, 1)
  }))
  ctx.provide('sessionPersistence', testSessionPersistence(ctx, {
    list: async () => stored,
    remove,
  }) as never)
  const forget = vi.fn(options.forget ?? (async () => {}))
  ctx.provide('workspaceRegistry', { forgetSession: forget } as never)
  ctx.provide('agents', {
    get: () => options.liveAgent === true ? { id: 'live' } : undefined,
  } as never)
  const agents = new ApiSessionAgentController(ctx)
  const disposeAgent = vi.spyOn(agents, 'disposeAgent').mockResolvedValue(options.disposeAgent ?? false)
  const began: SessionId[] = []
  const ended: SessionId[] = []
  vi.spyOn(agents, 'beginDeletion').mockImplementation((sessionId: SessionId) => { began.push(sessionId) })
  vi.spyOn(agents, 'endDeletion').mockImplementation((sessionId: SessionId) => { ended.push(sessionId) })
  const removed: SessionId[] = []
  ctx.on('api-session/removed', (sessionId: SessionId) => { removed.push(sessionId) })
  return { ctx, deletion: new SessionDeletion(ctx, agents), remove, forget, disposeAgent, began, ended, removed }
}

describe('session deletion', () => {
  it('removes a cold Session, forgets its Workspace references, and announces the removal once', async () => {
    const f = await fixture({ stored: [header('cold')] })

    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('cold') })).resolves.toEqual({ deleted: true })

    expect(f.remove).toHaveBeenCalledExactlyOnceWith('cold')
    expect(f.forget).toHaveBeenCalledExactlyOnceWith('cold')
    // No retained handle: the ordinary deletion path still asks, then announces the removal itself.
    expect(f.disposeAgent).toHaveBeenCalledExactlyOnceWith('cold')
    expect(f.removed).toEqual(['cold'])
    expect(f.began).toEqual(['cold'])
    expect(f.ended).toEqual(['cold'])
  })

  it('releases the live Agent before removing storage and does not announce twice', async () => {
    const f = await fixture({ stored: [header('live')], liveAgent: true, disposeAgent: true })
    const session = f.ctx.sessions.create(brandString<SessionId>('live'), { meta: { cwd: '/workspace' } })

    await expect(f.deletion.delete({ sessionId: session.id })).resolves.toEqual({ deleted: true })

    expect(f.disposeAgent).toHaveBeenCalledExactlyOnceWith('live')
    // Disposal emits the Host's own removal edge; a second announcement would duplicate the list mutation.
    expect(f.removed).toEqual([])
    expect(f.remove).toHaveBeenCalledExactlyOnceWith('live')
    expect(f.forget).toHaveBeenCalledExactlyOnceWith('live')
  })

  it('accepts a live Session that never materialized: disposal removed its only copy', async () => {
    const f = await fixture({
      liveAgent: true,
      disposeAgent: true,
      remove: async (sessionId: SessionId) => { throw new SessionPersistenceNotFoundError(sessionId) },
    })
    const session = f.ctx.sessions.create(brandString<SessionId>('blank'), { meta: { cwd: '/workspace' } })

    await expect(f.deletion.delete({ sessionId: session.id })).resolves.toEqual({ deleted: true })
    expect(f.forget).toHaveBeenCalledExactlyOnceWith('blank')
  })

  it('refuses while the Session still reports running work and writes nothing', async () => {
    const f = await fixture({ stored: [header('busy')] })
    f.ctx.on('workspace/session-activity', async ({ sessionId }, next) => {
      const rest = await next()
      return sessionId === 'busy' ? [{ kind: 'turn' }, ...rest] : rest
    })

    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('busy') })).rejects.toMatchObject({
      code: 'session/active',
      details: { sessionId: 'busy', activity: [{ kind: 'turn' }] },
    })
    expect(f.remove).not.toHaveBeenCalled()
    expect(f.forget).not.toHaveBeenCalled()
    expect(f.disposeAgent).not.toHaveBeenCalled()
    expect(f.removed).toEqual([])
    // Refusals before any Agent or storage work never take the deletion fence,
    // so a concurrent resume is not blocked by a rejected attempt.
    expect(f.began).toEqual([])
  })

  it('refuses a subagent conversation without inspecting activity or storage', async () => {
    const f = await fixture({ stored: [header('child', 'subagent')] })

    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('child') })).rejects.toMatchObject({
      code: 'session/subagent',
    })
    expect(f.remove).not.toHaveBeenCalled()
  })

  it('reports an unknown Session as not found', async () => {
    const f = await fixture()
    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('ghost') })).rejects.toMatchObject({
      code: 'session/not-found',
    })
    expect(f.remove).not.toHaveBeenCalled()
  })

  it('refuses a live Agent this controller cannot release', async () => {
    const f = await fixture({ stored: [header('foreign')], liveAgent: true })
    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('foreign') })).rejects.toMatchObject({
      code: 'session/in-use',
    })
    expect(f.remove).not.toHaveBeenCalled()
  })

  it('refuses a Session still attached to the store without a releasable Agent', async () => {
    const f = await fixture({ stored: [header('attached')] })
    const session = f.ctx.sessions.create(brandString<SessionId>('attached'), { meta: { cwd: '/workspace' } })

    await expect(f.deletion.delete({ sessionId: session.id })).rejects.toMatchObject({
      code: 'session/in-use',
    })
    expect(f.remove).not.toHaveBeenCalled()
    expect(f.forget).not.toHaveBeenCalled()
    expect(f.removed).toEqual([])
  })

  it('reports storage still held as in use', async () => {
    const f = await fixture({
      stored: [header('held')],
      remove: async (sessionId: SessionId) => { throw new SessionPersistenceBusyError(sessionId) },
    })
    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('held') })).rejects.toMatchObject({
      code: 'session/in-use',
    })
    expect(f.forget).not.toHaveBeenCalled()
    expect(f.removed).toEqual([])
  })

  it('reports a persistence inspection failure as an internal error', async () => {
    const f = await fixture({ stored: [header('unreadable')] })
    const persistence = f.ctx.sessionPersistence
    vi.spyOn(persistence, 'stat').mockRejectedValue(new Error('stat down'))

    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('unreadable') })).rejects.toMatchObject({
      code: 'gateway/internal',
    })
    expect(f.remove).not.toHaveBeenCalled()
  })

  it('reports a failing activity provider as an internal error', async () => {
    const f = await fixture({ stored: [header('unprobeable')] })
    f.ctx.on('workspace/session-activity', () => { throw new Error('provider down') })

    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('unprobeable') })).rejects.toMatchObject({
      code: 'gateway/internal',
    })
    expect(f.remove).not.toHaveBeenCalled()
  })

  it('reports an unexpected storage failure as an internal error', async () => {
    const f = await fixture({
      stored: [header('broken')],
      remove: async () => { throw new Error('disk on fire') },
    })
    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('broken') })).rejects.toMatchObject({
      code: 'gateway/internal',
    })
    expect(f.forget).not.toHaveBeenCalled()
    expect(f.removed).toEqual([])
  })

  it('reports a cold Session whose artifacts vanished as not found', async () => {
    const f = await fixture({
      stored: [header('vanished')],
      remove: async (sessionId: SessionId) => { throw new SessionPersistenceNotFoundError(sessionId) },
    })
    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('vanished') })).rejects.toMatchObject({
      code: 'session/not-found',
    })
  })

  it('reports a composition without a persistence backend as an internal error', async () => {
    const ctx = new Context()
    roots.push(ctx)
    await ctx.plugin(SessionStore)
    installSessionReadTestServices(ctx)
    ctx.provide('workspaceRegistry', { forgetSession: async () => {} } as never)
    ctx.provide('agents', { get: () => undefined } as never)
    const agents = new ApiSessionAgentController(ctx)
    const deletion = new SessionDeletion(ctx, agents)

    await expect(deletion.delete({ sessionId: brandString<SessionId>('no-backend') })).rejects.toMatchObject({
      code: 'gateway/internal',
    })
  })

  it('keeps the deletion successful when Workspace cleanup fails after the log is gone', async () => {
    const f = await fixture({
      stored: [header('residue')],
      forget: async () => { throw new Error('storage down') },
    })
    const warn = vi.spyOn(f.ctx.logger, 'warn').mockImplementation(() => {})

    await expect(f.deletion.delete({ sessionId: brandString<SessionId>('residue') })).resolves.toEqual({ deleted: true })
    expect(f.remove).toHaveBeenCalledOnce()
    expect(warn).toHaveBeenCalledWith(expect.stringContaining('workspace accounting could not be cleared'))
    expect(f.removed).toEqual(['residue'])
  })
})

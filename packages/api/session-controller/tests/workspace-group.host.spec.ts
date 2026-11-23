/** The durable sidebar grouping override projection: path plus optional role, last wins. */

import { afterEach, describe, expect, it } from 'vitest'
import { Context } from '@deepseek-ai/cordis'
import SessionStore, { SessionSeq } from '@deepseek-ai/dsh-session'
import type { SessionEvent } from '@deepseek-ai/dsh-session'
import SessionProjectionRegistry from '@deepseek-ai/dsh-session-projection'
import type { SessionWorkspaceGroup } from '../src/types.ts'
import { appendWorkspaceGroup, workspaceGroupOf, workspaceGroupProjectionDefinition } from '../src/workspace-group.ts'

/** One grouping override, as `appendWorkspaceGroup` records it. */
function grouped(path: string, role?: 'orchestrator', seq = SessionSeq(0)): SessionEvent {
  return {
    type: 'session/workspace-group', seq, time: Number(seq),
    data: { path, ...(role === undefined ? {} : { role }) },
  }
}

/** Contexts this suite must dispose so the registry and store release their effects. */
const contexts = new Set<Context>()
afterEach(async () => {
  await Promise.all([...contexts].map(ctx => ctx.fiber.dispose()))
  contexts.clear()
})

describe('session workspace group projection', () => {
  it('starts without an override and keeps the latest absolute path and role', () => {
    const definition = workspaceGroupProjectionDefinition
    let state: SessionWorkspaceGroup | null = definition.init()
    expect(state).toBeNull()

    state = definition.apply(state, grouped('/projects/super-agent', 'orchestrator', SessionSeq(0)))
    expect(state).toEqual({ path: '/projects/super-agent', role: 'orchestrator' })
    expect(definition.wire.view(state)).toEqual({ path: '/projects/super-agent', role: 'orchestrator' })
    expect(definition.stateSchema.parse(state)).toEqual({ path: '/projects/super-agent', role: 'orchestrator' })

    state = definition.apply(state, grouped('/projects/other', undefined, SessionSeq(1)))
    expect(state).toEqual({ path: '/projects/other' })
  })

  it('leaves the state untouched for unrelated events', () => {
    const definition = workspaceGroupProjectionDefinition
    const initial = { path: '/projects/super-agent' } as const
    expect(definition.apply(initial, {
      type: 'turn/end', seq: SessionSeq(1), time: 1, data: { turn: 1, reason: { kind: 'completed' } },
    })).toBe(initial)
  })

  it('reads back the group an append records and rejects a relative path', async () => {
    const ctx = new Context()
    contexts.add(ctx)
    await ctx.plugin(SessionStore)
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.sessionProjections.register(workspaceGroupProjectionDefinition)
    const session = ctx.sessions.create()
    expect(workspaceGroupOf(ctx, session)).toBeNull()
    appendWorkspaceGroup(session, { path: '/projects/super-agent', role: 'orchestrator' })
    expect(workspaceGroupOf(ctx, session)).toEqual({ path: '/projects/super-agent', role: 'orchestrator' })
    expect(() => { appendWorkspaceGroup(session, { path: 'relative/project' }) }).toThrow(TypeError)
  })

  it('validates the persisted grouping value', () => {
    expect(() => workspaceGroupProjectionDefinition.stateSchema.parse({ path: 'relative' })).not.toThrow()
    expect(() => workspaceGroupProjectionDefinition.stateSchema.parse({ path: '' })).toThrow()
    expect(() => workspaceGroupProjectionDefinition.stateSchema.parse({ path: '/abs', role: 'other' })).toThrow()
    expect(() => workspaceGroupProjectionDefinition.stateSchema.parse({ path: '/abs', extra: true })).toThrow()
  })
})

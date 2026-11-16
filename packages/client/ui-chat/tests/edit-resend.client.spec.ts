import { describe, expect, it, vi } from 'vitest'
import { RemoteError } from '@deepseek-ai/dsh-client-test-runtime'
import { brandString } from '@deepseek-ai/dsh-brand'
import type {
  SessionListState, SessionSummary,
} from '@deepseek-ai/dsh-api-session-controller/client'
import type { SessionRequestId } from '@deepseek-ai/dsh-api-session-controller/types'
import type { WorkspaceId } from '@deepseek-ai/dsh-workspace/types'
import type { SessionId } from '@deepseek-ai/dsh-session/types'
import type { WorkspaceSnapshot } from '@deepseek-ai/dsh-api-workspace-controller/client'
import { resendEditedMessage, type EditResendDeps, type EditResendSession } from '../src/client/chat/edit-resend.ts'

const SID = brandString<SessionId>('source')
const CHILD = brandString<SessionId>('child')
const WID = 'workspace-1' as WorkspaceId
const REQUEST = brandString<SessionRequestId>('request-1')

const summary: SessionSummary = {
  id: SID,
  displayTitle: 'source',
  running: false,
  blank: false,
  updatedAt: 1,
  retainedBy: {},
  cwd: '/workspace/source',
  projectionValues: { agentPreset: 'coder' },
}

function sessionList(): SessionListState {
  return {
    ids: [SID],
    byId: { [SID]: summary },
    phase: 'ready',
    projectionsBySession: {},
  }
}

function workspaceList(): WorkspaceSnapshot {
  return {
    items: [{
      workspaceId: WID,
      path: '/workspace',
      title: 'workspace',
      sessionIds: [SID],
      createdAt: '2026-01-01T00:00:00.000Z',
      updatedAt: '2026-01-01T00:00:00.000Z',
    }],
    archivedSessionIds: [],
    pinnedSessionIds: [],
    phase: 'ready',
    state: 'idle',
    error: null,
  }
}

interface BenchOverrides {
  readonly running?: boolean
  readonly prompt?: () => ReturnType<EditResendSession['prompt']>
  readonly cancel?: () => ReturnType<EditResendSession['cancel']>
}

function bench(overrides: BenchOverrides = {}) {
  const abandon = vi.fn()
  const beginSubmission = vi.fn(() => ({ requestId: REQUEST, abandon }))
  const prompt = vi.fn(async () => overrides.prompt?.() ?? { ok: true as const, value: { accepted: true as const } })
  const cancel = vi.fn(async () => overrides.cancel?.() ?? { ok: true as const, value: { accepted: true as const } })
  const source = {
    getSnapshot: () => ({ running: overrides.running ?? false }),
    beginSubmission,
    prompt,
    cancel,
  }
  const child = {
    getSnapshot: () => ({ running: false }),
    beginSubmission,
    prompt,
    cancel,
  }
  const release = vi.fn()
  const retain = vi.fn(() => ({ binding: { session: child }, release }))
  const binding = vi.fn((id: SessionId) => id === SID
    ? { session: source }
    : id === CHILD ? { session: child } : undefined)
  const create = vi.fn(async () => CHILD)
  const fork = vi.fn(async () => CHILD)
  const openSession = vi.fn()
  const requestDraftInitialization = vi.fn(() => 'applied' as const)
  const deps: EditResendDeps = {
    sessions: {
      list: { getSnapshot: sessionList },
      create,
      fork,
      retain,
      binding,
    },
    workspaces: { list: { getSnapshot: workspaceList, subscribe: () => () => {} } },
    conversation: { input: { requestDraftInitialization } },
    openSession,
  }
  return {
    deps, abandon, beginSubmission, prompt, cancel, source, child, release,
    retain, binding, create, fork, openSession, requestDraftInitialization,
  }
}

describe('resendEditedMessage', () => {
  it('forks at the exact boundary and admits the edited text', async () => {
    const b = bench()
    await expect(resendEditedMessage(b.deps, {
      sessionId: SID,
      text: 'corrected',
      boundarySeq: 7,
      runningTurn: false,
    })).resolves.toBe(CHILD)

    expect(b.fork).toHaveBeenCalledExactlyOnceWith({
      sessionId: SID,
      atSeq: 7,
      increaseTitle: true,
    })
    expect(b.create).not.toHaveBeenCalled()
    expect(b.retain).toHaveBeenCalledExactlyOnceWith(CHILD, { source: 'chatEditResend' })
    expect(b.beginSubmission).toHaveBeenCalledExactlyOnceWith({
      mode: 'queue',
      text: 'corrected',
      attachments: [],
    })
    expect(b.prompt).toHaveBeenCalledExactlyOnceWith(
      [{ type: 'text', text: 'corrected' }],
      'queue',
      undefined,
      REQUEST,
    )
    expect(b.openSession).toHaveBeenCalledExactlyOnceWith(CHILD)
    expect(b.release).toHaveBeenCalledOnce()
    expect(b.requestDraftInitialization).not.toHaveBeenCalled()
  })

  it('creates a new Session in the source Workspace when there is no prefix', async () => {
    const b = bench()
    await expect(resendEditedMessage(b.deps, {
      sessionId: SID,
      text: 'first message',
      runningTurn: false,
    })).resolves.toBe(CHILD)

    expect(b.fork).not.toHaveBeenCalled()
    expect(b.create).toHaveBeenCalledExactlyOnceWith({
      workspaceId: WID,
      cwd: '/workspace/source',
      agentPreset: 'coder',
    })
  })

  it('cancels a running source Turn before creating the branch', async () => {
    const b = bench({
      running: true,
      cancel: async () => ({ ok: true, value: { accepted: true } }),
    })
    await expect(resendEditedMessage(b.deps, {
      sessionId: SID,
      text: 'corrected',
      boundarySeq: 2,
      runningTurn: true,
    })).resolves.toBe(CHILD)

    expect(b.cancel).toHaveBeenCalledOnce()
    expect(b.cancel.mock.invocationCallOrder[0]).toBeLessThan(b.fork.mock.invocationCallOrder[0] ?? 0)
  })

  it('aborts before branching when cancellation fails', async () => {
    const b = bench({
      running: true,
      cancel: async () => ({ ok: false, error: new RemoteError('session/agent-busy', 'busy', { reason: 'busy' }) }),
    })
    await expect(resendEditedMessage(b.deps, {
      sessionId: SID,
      text: 'corrected',
      boundarySeq: 2,
      runningTurn: true,
    })).rejects.toThrow('session/agent-busy: busy')

    expect(b.fork).not.toHaveBeenCalled()
    expect(b.create).not.toHaveBeenCalled()
  })

  it('restores the edited text when the Host rejects the prompt', async () => {
    const b = bench({
      prompt: async () => ({ ok: false, error: new RemoteError('session/agent-busy', 'busy', { reason: 'busy' }) }),
    })
    await expect(resendEditedMessage(b.deps, {
      sessionId: SID,
      text: 'corrected',
      boundarySeq: 2,
      runningTurn: false,
    })).rejects.toThrow('session/agent-busy: busy')

    expect(b.abandon).toHaveBeenCalledOnce()
    expect(b.openSession).toHaveBeenCalledExactlyOnceWith(CHILD)
    expect(b.requestDraftInitialization).toHaveBeenCalledExactlyOnceWith(
      { session: b.child },
      { prompt: 'corrected', whenDraftExists: 'keep' },
    )
    expect(b.release).toHaveBeenCalledOnce()
  })

  it('restores the edited text when prompt admission throws', async () => {
    const b = bench({
      prompt: async () => { throw new Error('transport closed') },
    })
    await expect(resendEditedMessage(b.deps, {
      sessionId: SID,
      text: 'corrected',
      boundarySeq: 2,
      runningTurn: false,
    })).rejects.toThrow('transport closed')

    expect(b.abandon).toHaveBeenCalledOnce()
    expect(b.requestDraftInitialization).toHaveBeenCalledOnce()
  })

  it('preserves the prompt failure when revealing the child fails', async () => {
    const b = bench({
      prompt: async () => ({ ok: false, error: new RemoteError('session/agent-busy', 'busy', { reason: 'busy' }) }),
    })
    b.openSession.mockImplementation(() => { throw new Error('navigation failed') })
    await expect(resendEditedMessage(b.deps, {
      sessionId: SID,
      text: 'corrected',
      boundarySeq: 2,
      runningTurn: false,
    })).rejects.toThrow('session/agent-busy: busy')

    expect(b.requestDraftInitialization).toHaveBeenCalledOnce()
    expect(b.release).toHaveBeenCalledOnce()
  })

  it('leaves the failure recoverable without a child binding', async () => {
    const b = bench({
      prompt: async () => ({ ok: false, error: new RemoteError('session/agent-busy', 'busy', { reason: 'busy' }) }),
    })
    b.binding.mockImplementation(id => id === SID ? { session: b.source } : undefined)
    await expect(resendEditedMessage(b.deps, {
      sessionId: SID,
      text: 'corrected',
      boundarySeq: 2,
      runningTurn: false,
    })).rejects.toThrow('session/agent-busy: busy')

    expect(b.openSession).toHaveBeenCalledOnce()
    expect(b.requestDraftInitialization).not.toHaveBeenCalled()
  })

  it('rejects blank text before touching services', async () => {
    const b = bench()
    await expect(resendEditedMessage(b.deps, {
      sessionId: SID,
      text: '   ',
      boundarySeq: 2,
      runningTurn: false,
    })).rejects.toThrow('requires non-blank text')

    expect(b.cancel).not.toHaveBeenCalled()
    expect(b.fork).not.toHaveBeenCalled()
    expect(b.create).not.toHaveBeenCalled()
    expect(b.retain).not.toHaveBeenCalled()
  })

  it('creates without Workspace or input recovery when those services are absent', async () => {
    const b = bench({
      prompt: async () => ({ ok: false, error: new RemoteError('session/agent-busy', 'busy', { reason: 'busy' }) }),
    })
    const deps: EditResendDeps = {
      sessions: b.deps.sessions,
      openSession: b.openSession,
    }
    await expect(resendEditedMessage(deps, {
      sessionId: SID,
      text: 'first message',
      runningTurn: false,
    })).rejects.toThrow('session/agent-busy: busy')

    expect(b.create).toHaveBeenCalledExactlyOnceWith({
      cwd: '/workspace/source',
      agentPreset: 'coder',
    })
    expect(b.openSession).toHaveBeenCalledExactlyOnceWith(CHILD)
    expect(b.requestDraftInitialization).not.toHaveBeenCalled()
  })

  it('does not cancel when a running Turn has no source binding', async () => {
    const b = bench({ running: true })
    b.binding.mockReturnValue(undefined)
    await expect(resendEditedMessage(b.deps, {
      sessionId: SID,
      text: 'corrected',
      boundarySeq: 2,
      runningTurn: true,
    })).resolves.toBe(CHILD)

    expect(b.cancel).not.toHaveBeenCalled()
  })
})

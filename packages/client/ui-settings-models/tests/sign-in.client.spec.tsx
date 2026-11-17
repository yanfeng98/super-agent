// @vitest-environment jsdom
/** The Models page's sign-in store and dialog over a scripted authorization namespace. */
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { Context } from '@deepseek-ai/cordis'
import type {
  AuthorizationAttemptView, AuthorizationFlowView, RemoteResult,
} from '@deepseek-ai/dsh-api-remotes/client'
import { ProviderSignIn } from '../src/client/ProviderSignIn.tsx'
import { createProviderSignIn } from '../src/client/sign-in.ts'
import type { ProviderSignInState, ProviderSignIn as ProviderSignInHandle } from '../src/client/sign-in.ts'
import { en } from '../src/client/locales.ts'

const KEY = 'llm-pi-ai/openai-codex'
const t = (key: keyof typeof en): string => en[key]

const FLOW: AuthorizationFlowView = {
  key: KEY,
  scope: 'llm-pi-ai',
  id: 'openai-codex',
  label: 'OpenAI Codex',
  methods: [{ id: 'oauth', label: 'OpenAI (ChatGPT Plus/Pro)' }],
  inFlight: false,
}

/** One attempt view in the shape the host publishes. */
function attempt(overrides: Partial<AuthorizationAttemptView> = {}): AuthorizationAttemptView {
  return { key: KEY, phase: 'running', notices: [], prompt: null, ...overrides }
}

function ok<T>(value: T): RemoteResult<T> {
  return { ok: true, value }
}

function fail(message: string): RemoteResult<never> {
  return { ok: false, error: { code: 'gateway/internal', message, details: {} } as never }
}

/** A page context whose authorization namespace answers from the scripted calls. */
function harness(script: {
  state?: () => Promise<RemoteResult<AuthorizationAttemptView | null>>
  begin?: () => Promise<RemoteResult<AuthorizationAttemptView>>
  answer?: () => Promise<RemoteResult<AuthorizationAttemptView>>
  cancel?: () => Promise<RemoteResult<void>>
} = {}) {
  const calls = {
    begin: [] as Array<[string, string | undefined]>,
    answer: [] as Array<[string, number, string]>,
    cancel: [] as string[],
    state: [] as string[],
  }
  const authorization = {
    list: vi.fn(() => Promise.resolve(ok([FLOW]))),
    begin: vi.fn((key: string, method: string | undefined) => {
      calls.begin.push([key, method])
      return (script.begin ?? (() => Promise.resolve(ok(attempt()))))()
    }),
    state: vi.fn((key: string) => {
      calls.state.push(key)
      return (script.state ?? (() => Promise.resolve(ok(attempt()))))()
    }),
    answer: vi.fn((key: string, promptId: number, value: string) => {
      calls.answer.push([key, promptId, value])
      return (script.answer ?? (() => Promise.resolve(ok(attempt()))))()
    }),
    cancel: vi.fn((key: string) => {
      calls.cancel.push(key)
      return (script.cancel ?? (() => Promise.resolve(ok(undefined))))()
    }),
  }
  const ctx = Object.assign(new Context(), { remote: { authorization } })
  return { ctx, authorization, calls }
}

afterEach(() => {
  cleanup()
})

describe('the Models page sign-in store', () => {
  it('starts the flow it is asked for and publishes the attempt’s notices', async () => {
    const { ctx, authorization } = harness({
      state: () => Promise.resolve(ok(null)),
      begin: () => Promise.resolve(ok(attempt({ notices: [{ message: 'Open this page', url: 'https://example.test' }] }))),
    })
    const signIn: ProviderSignInHandle = createProviderSignIn(ctx)
    await signIn.begin(KEY, 'oauth')

    expect(authorization.begin).toHaveBeenCalledWith(KEY, 'oauth')
    const state = signIn.store.getSnapshot()
    expect(state.phase).toBe('running')
    expect(state.notices).toEqual([{ message: 'Open this page', url: 'https://example.test' }])
    signIn.dispose()
  })

  it('adopts an attempt another page left running instead of starting a second one', async () => {
    const { ctx, authorization } = harness({
      state: () => Promise.resolve(ok(attempt({ notices: [{ message: 'Waiting for the browser' }] }))),
    })
    const signIn = createProviderSignIn(ctx)
    await signIn.begin(KEY, 'oauth')

    expect(authorization.begin).not.toHaveBeenCalled()
    expect(signIn.store.getSnapshot().notices).toEqual([{ message: 'Waiting for the browser' }])
    signIn.dispose()
  })

  it('carries an answer back to the host and publishes the question the flow asked next', async () => {
    const { ctx, calls } = harness({
      state: () => Promise.resolve(ok(null)),
      answer: () => Promise.resolve(ok(attempt({
        prompt: { promptId: 1, kind: 'text', message: 'Paste the code' },
      }))),
    })
    const signIn = createProviderSignIn(ctx)
    await signIn.begin(KEY, 'oauth')
    await signIn.answer(0, 'THE-CODE')

    expect(calls.answer).toEqual([[KEY, 0, 'THE-CODE']])
    expect(signIn.store.getSnapshot().prompt).toEqual({ promptId: 1, kind: 'text', message: 'Paste the code' })
    signIn.dispose()
  })

  it('reports a refused call as the copy the card shows', async () => {
    const { ctx } = harness({
      state: () => Promise.resolve(ok(null)),
      begin: () => Promise.resolve(fail('an authorization attempt for this key is already running')),
    })
    const signIn = createProviderSignIn(ctx)
    await signIn.begin(KEY, undefined)

    expect(signIn.store.getSnapshot()).toMatchObject({
      phase: 'failed',
      failure: 'an authorization attempt for this key is already running',
    })
    signIn.dispose()
  })

  it('withdraws the attempt on cancel and forgets it on reset', async () => {
    const { ctx, calls } = harness({ state: () => Promise.resolve(ok(null)) })
    const signIn = createProviderSignIn(ctx)
    await signIn.begin(KEY, undefined)
    await signIn.cancel()
    expect(calls.cancel).toEqual([KEY])
    expect(signIn.store.getSnapshot().phase).toBe('cancelled')

    signIn.reset()
    expect(signIn.store.getSnapshot()).toMatchObject({ key: null, phase: 'idle', notices: [] })
    signIn.dispose()
  })

  it('withdraws an attempt the dialog stopped watching while its start was in flight', async () => {
    let releaseStart: () => void = () => {}
    const startGate = new Promise<void>((resolve) => { releaseStart = resolve })
    const { ctx, calls } = harness({
      state: () => Promise.resolve(ok(null)),
      begin: async () => {
        await startGate
        return ok(attempt())
      },
    })
    const signIn = createProviderSignIn(ctx)
    const starting = signIn.begin(KEY, 'oauth')
    // The human closes the dialog before the Host has answered the start.
    await signIn.cancel()
    releaseStart()
    await starting

    // Whatever the start created still has no watcher, so it is withdrawn.
    expect(calls.cancel).toContain(KEY)
    signIn.dispose()
  })

  it('reports a vanished attempt instead of polling forever behind it', async () => {
    vi.useFakeTimers()
    try {
      let reads = 0
      const { ctx } = harness({
        state: () => {
          reads += 1
          // The Host kept it on the first read, then lost it (a restart).
          return Promise.resolve(ok(reads === 1 ? attempt() : null))
        },
      })
      const signIn = createProviderSignIn(ctx)
      await signIn.begin(KEY, undefined)
      expect(signIn.store.getSnapshot().phase).toBe('running')

      await act(async () => { await vi.advanceTimersByTimeAsync(900) })
      expect(signIn.store.getSnapshot()).toMatchObject({ phase: 'failed', ended: true, failure: null })

      const settledReads = reads
      await act(async () => { await vi.advanceTimersByTimeAsync(2_000) })
      expect(reads).toBe(settledReads)
      signIn.dispose()
    } finally {
      vi.useRealTimers()
    }
  })

  it('treats a refused answer as stale when the attempt itself is still running', async () => {
    const { ctx } = harness({
      state: () => Promise.resolve(ok(attempt({
        prompt: { promptId: 3, kind: 'text', message: 'Paste the code' },
      }))),
      answer: () => Promise.resolve(fail('attempt for this key is not waiting on that question')),
    })
    const signIn = createProviderSignIn(ctx)
    await signIn.begin(KEY, 'oauth')
    await signIn.answer(2, 'THE-CODE')

    // The double submit is refused; the attempt keeps running with its real question.
    expect(signIn.store.getSnapshot()).toMatchObject({
      phase: 'running',
      failure: null,
      ended: false,
      prompt: { promptId: 3 },
    })
    signIn.dispose()
  })

  it('keeps polling a running attempt until it settles', async () => {
    vi.useFakeTimers()
    try {
      let reads = 0
      const { ctx } = harness({
        state: () => {
          reads += 1
          return Promise.resolve(ok(attempt(reads < 2 ? {} : { phase: 'authorized' })))
        },
      })
      const signIn = createProviderSignIn(ctx)
      await signIn.begin(KEY, undefined)
      expect(signIn.store.getSnapshot().phase).toBe('running')

      await act(async () => { await vi.advanceTimersByTimeAsync(900) })
      expect(signIn.store.getSnapshot().phase).toBe('authorized')

      const settledReads = reads
      await act(async () => { await vi.advanceTimersByTimeAsync(2_000) })
      expect(reads).toBe(settledReads)
      signIn.dispose()
    } finally {
      vi.useRealTimers()
    }
  })
})

describe('the sign-in dialog', () => {
  const idle: ProviderSignInState = {
    key: null, phase: 'idle', notices: [], prompt: null, failure: null, ended: false,
  }

  function view(attemptState: ProviderSignInState) {
    return { attempt: attemptState, begin: vi.fn(), answer: vi.fn(), cancel: vi.fn(), reset: vi.fn() }
  }

  function renderDialog(attemptState: ProviderSignInState, onClose = vi.fn()) {
    const signIn = view(attemptState)
    render(
      <ProviderSignIn
        open
        displayName="OpenAI Codex"
        flow={FLOW}
        signIn={signIn}
        onClose={onClose}
        t={t}
        closeLabel={en.close}
      />,
    )
    return { signIn, onClose }
  }

  it('shows the flow’s notices with the page it asks the human to open and the code to type', () => {
    renderDialog({
      ...idle,
      key: KEY,
      phase: 'running',
      notices: [{ message: 'Enter this code to finish signing in.', url: 'https://example.test/device', code: 'ABCD-1234' }],
    })
    expect(screen.getByText('Enter this code to finish signing in.')).toBeTruthy()
    expect(screen.getByText('ABCD-1234')).toBeTruthy()
    expect(screen.getByRole('link', { name: en.signInOpen }).getAttribute('href')).toBe('https://example.test/device')
  })

  it('submits a typed answer with the question id it belongs to', () => {
    const { signIn } = renderDialog({
      ...idle,
      key: KEY,
      phase: 'running',
      prompt: { promptId: 7, kind: 'text', message: 'Paste the code', placeholder: 'code' },
    })
    fireEvent.change(screen.getByLabelText('Paste the code'), { target: { value: 'THE-CODE' } })
    fireEvent.click(screen.getByRole('button', { name: en.signInSubmit }))
    expect(signIn.answer).toHaveBeenCalledWith(7, 'THE-CODE')
  })

  it('masks a secret question and answers a select question with the chosen option id', () => {
    const { signIn } = renderDialog({
      ...idle,
      key: KEY,
      phase: 'running',
      prompt: {
        promptId: 2,
        kind: 'select',
        message: 'Select OpenAI Codex login method:',
        options: [{ id: 'browser', label: 'Browser login' }, { id: 'device_code', label: 'Device code login' }],
      },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Device code login' }))
    expect(signIn.answer).toHaveBeenCalledWith(2, 'device_code')
  })

  it('reports the host’s refusal and offers a retry', () => {
    const { signIn } = renderDialog({ ...idle, key: KEY, phase: 'failed', failure: 'the provider refused the grant' })
    expect(screen.getByRole('alert').textContent).toContain('the provider refused the grant')
    fireEvent.click(screen.getByRole('button', { name: en.signInRetry }))
    expect(signIn.begin).toHaveBeenCalledWith(KEY, 'oauth')
  })

  it('reports a vanished attempt with the page’s own copy', () => {
    renderDialog({ ...idle, key: KEY, phase: 'failed', ended: true })
    expect(screen.getByRole('alert').textContent).toContain(en.signInEnded)
  })

  it('confirms the sign-in and offers no retry for it', () => {
    renderDialog({ ...idle, key: KEY, phase: 'authorized' })
    expect(screen.getByText(en.signInDone)).toBeTruthy()
    expect(screen.queryByRole('button', { name: en.signInRetry })).toBeNull()
  })

  it('cancels a running attempt when the dialog is closed', () => {
    const { signIn, onClose } = renderDialog({ ...idle, key: KEY, phase: 'running' })
    fireEvent.click(screen.getByRole('button', { name: en.cancel }))
    expect(signIn.cancel).toHaveBeenCalled()
    expect(onClose).toHaveBeenCalled()
  })
})

/**
 * The Models page's half of a provider sign-in: one attempt at a time,
 * addressed by the credential key the host flow writes.
 *
 * The host owns the conversation — its notices, its question, its outcome —
 * and this module carries the page's answers back and republishes the attempt
 * for the card that shows it. Polling lives here rather than in a component
 * because the attempt outlives any one render: a card that closes mid-sign-in
 * leaves the flow waiting on the host, and a reloaded card rejoins the same
 * attempt instead of starting a second one.
 *
 * @module @deepseek-ai/dsh-client-ui-settings-models/src/client/sign-in
 */

import type { Context as ClientContext } from '@deepseek-ai/cordis'
import type {
  AuthorizationAttemptView, AuthorizationNoticeView, AuthorizationPromptView,
} from '@deepseek-ai/dsh-api-remotes/client'
import { createSnapshotStore } from '@deepseek-ai/dsh-client-store'
import type { SnapshotStore } from '@deepseek-ai/dsh-client-store'

/**
 * How often a running attempt is re-read. A sign-in waits on a human at a
 * browser and on a device-code poll at the provider, so a sub-second cadence
 * reads as immediate without turning the wait into a request flood.
 */
const POLL_INTERVAL_MS = 800

/** Where one attempt stands, as the card renders it. */
export interface ProviderSignInState {
  /** The credential key the page is driving, or null when it drives none. */
  key: string | null
  /** `idle` before any attempt; the rest mirror the host's phases. */
  phase: 'idle' | 'starting' | 'running' | 'authorized' | 'cancelled' | 'failed'
  /** Everything the flow has reported, oldest first. */
  notices: readonly AuthorizationNoticeView[]
  /** The question the attempt is waiting on, or null when it waits on nothing. */
  prompt: AuthorizationPromptView | null
  /** Copy for a refused or failed attempt. */
  failure: string | null
}

/** The sign-in state and the actions a provider card invokes. */
export interface ProviderSignIn {
  /** The attempt as the card renders it. */
  readonly store: SnapshotStore<ProviderSignInState>
  /**
   * Drive one provider's flow: adopt the attempt it already runs, or start the
   * named method.
   * @param key - joined `<scope>/<id>` credential key of the flow.
   * @param method - method id to run; absent runs the flow's first.
   */
  begin(key: string, method: string | undefined): Promise<void>
  /**
   * Answer the question the attempt is waiting on.
   * @param promptId - id of the question being answered, as last read.
   * @param value - the typed text, or a `select` answer's option id.
   */
  answer(promptId: number, value: string): Promise<void>
  /** Withdraw the attempt this page is driving, if any. */
  cancel(): Promise<void>
  /** Forget the attempt so the next card starts clean. */
  reset(): void
  /** Stop polling and release the attempt; the plugin's fiber owns this. */
  dispose(): void
}

/** Project one host answer onto the page's own state, keeping the key it belongs to. */
function adopt(view: AuthorizationAttemptView): ProviderSignInState {
  return {
    key: view.key,
    phase: view.phase,
    notices: [...view.notices],
    prompt: view.prompt,
    failure: view.error ?? null,
  }
}

/**
 * Bind the sign-in state and actions to one page context.
 * @param ctx - the page plugin's context, which declares `remote.authorization`.
 * @returns the store the card renders from and the actions it invokes.
 */
export function createProviderSignIn(ctx: ClientContext): ProviderSignIn {
  const store = createSnapshotStore<ProviderSignInState>({
    key: null, phase: 'idle', notices: [], prompt: null, failure: null,
  })
  /** Latest attempt's owner; a response for an older one is dropped instead of published. */
  let generation = 0
  let timer: ReturnType<typeof setInterval> | undefined

  const stopPolling = (): void => {
    if (timer === undefined) return
    clearInterval(timer)
    timer = undefined
  }

  const publish = (view: AuthorizationAttemptView): void => {
    store.update((state) => { Object.assign(state, adopt(view)) })
    if (view.phase !== 'running') stopPolling()
  }

  const refresh = async (key: string, owner: number): Promise<void> => {
    const result = await ctx.remote.authorization.state(key)
    if (owner !== generation) return
    if (!result.ok) {
      store.update((state) => {
        state.phase = 'failed'
        state.failure = result.error.message
      })
      stopPolling()
      return
    }
    if (result.value === null) return
    publish(result.value)
  }

  const poll = (key: string, owner: number): void => {
    stopPolling()
    timer = setInterval(() => { void refresh(key, owner) }, POLL_INTERVAL_MS)
  }

  const fail = (message: string): void => {
    store.update((state) => {
      state.phase = 'failed'
      state.failure = message
    })
    stopPolling()
  }

  return {
    store,

    async begin(key, method) {
      const owner = ++generation
      stopPolling()
      store.update((state) => {
        Object.assign(state, {
          key, phase: 'starting', notices: [], prompt: null, failure: null,
        } satisfies ProviderSignInState)
      })
      // An attempt another page started belongs to this one too: adopting it
      // is what keeps a reload from being a second sign-in to the provider.
      const current = await ctx.remote.authorization.state(key)
      if (owner !== generation) return
      if (current.ok && current.value !== null && current.value.phase === 'running') {
        publish(current.value)
        poll(key, owner)
        return
      }
      const started = await ctx.remote.authorization.begin(key, method)
      if (owner !== generation) return
      if (!started.ok) {
        fail(started.error.message)
        return
      }
      publish(started.value)
      if (started.value.phase === 'running') poll(key, owner)
    },

    async answer(promptId, value) {
      const { key, phase } = store.getSnapshot()
      if (key === null || phase === 'idle') return
      const owner = generation
      const answered = await ctx.remote.authorization.answer(key, promptId, value)
      if (owner !== generation) return
      if (!answered.ok) {
        fail(answered.error.message)
        return
      }
      publish(answered.value)
      if (answered.value.phase === 'running' && timer === undefined) poll(key, owner)
    },

    async cancel() {
      const { key } = store.getSnapshot()
      stopPolling()
      if (key === null) return
      await ctx.remote.authorization.cancel(key)
      const owner = generation
      await refresh(key, owner)
      store.update((state) => { state.phase = 'cancelled' })
    },

    reset() {
      generation += 1
      stopPolling()
      store.update((state) => {
        Object.assign(state, {
          key: null, phase: 'idle', notices: [], prompt: null, failure: null,
        } satisfies ProviderSignInState)
      })
    },

    dispose() {
      generation += 1
      stopPolling()
    },
  }
}

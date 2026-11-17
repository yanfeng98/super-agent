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
  /** The Host's own refusal, shown verbatim under the failure line. */
  failure: string | null
  /** The attempt disappeared before it finished, so the page reports the loss itself. */
  ended: boolean
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
    ended: false,
  }
}

/**
 * Bind the sign-in state and actions to one page context.
 * @param ctx - the page plugin's context, which declares `remote.authorization`.
 * @returns the store the card renders from and the actions it invokes.
 */
export function createProviderSignIn(ctx: ClientContext): ProviderSignIn {
  const store = createSnapshotStore<ProviderSignInState>({
    key: null, phase: 'idle', notices: [], prompt: null, failure: null, ended: false,
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
      fail(result.error.message)
      return
    }
    // No attempt under that key: the Host was restarted, or another surface
    // withdrew it. Waiting would poll forever behind a question nobody holds.
    if (result.value === null) {
      failEnded()
      return
    }
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
      state.ended = false
    })
    stopPolling()
  }

  /** The attempt is gone: no Host copy explains it, so the page says it ended. */
  const failEnded = (): void => {
    store.update((state) => {
      state.phase = 'failed'
      state.failure = null
      state.ended = true
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
          key, phase: 'starting', notices: [], prompt: null, failure: null, ended: false,
        } satisfies ProviderSignInState)
      })
      // An attempt another page started belongs to this one too: adopting it
      // is what keeps a reload from being a second sign-in to the provider.
      const current = await ctx.remote.authorization.state(key)
      // The dialog closed while the read was in flight. Whatever this page
      // started (or adopted) would have no watcher and would hold the key until
      // its lifetime ran out, so withdraw it on the way out.
      if (owner !== generation) {
        void ctx.remote.authorization.cancel(key)
        return
      }
      if (current.ok && current.value !== null && current.value.phase === 'running') {
        publish(current.value)
        poll(key, owner)
        return
      }
      const started = await ctx.remote.authorization.begin(key, method)
      if (owner !== generation) {
        void ctx.remote.authorization.cancel(key)
        return
      }
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
        // A second submit, or an answer that arrived after the question was
        // replaced, is refused while the attempt itself is fine: publish what
        // the Host now holds instead of turning a stale click into a failure.
        const latest = await ctx.remote.authorization.state(key)
        if (owner !== generation) return
        if (latest.ok && latest.value !== null) {
          publish(latest.value)
          if (latest.value.phase === 'running' && timer === undefined) poll(key, owner)
        } else if (latest.ok) failEnded()
        else fail(answered.error.message)
        return
      }
      publish(answered.value)
      if (answered.value.phase === 'running' && timer === undefined) poll(key, owner)
    },

    async cancel() {
      const { key } = store.getSnapshot()
      // Bumping the generation is what tells a start still in flight that the
      // attempt it is about to create has no watcher left; that continuation
      // withdraws it once the Host answers.
      const owner = ++generation
      stopPolling()
      if (key === null) return
      await ctx.remote.authorization.cancel(key)
      if (owner !== generation) return
      const latest = await ctx.remote.authorization.state(key)
      if (owner !== generation) return
      if (latest.ok && latest.value !== null) publish(latest.value)
      store.update((state) => {
        state.phase = 'cancelled'
        state.ended = false
      })
    },

    reset() {
      generation += 1
      stopPolling()
      store.update((state) => {
        Object.assign(state, {
          key: null, phase: 'idle', notices: [], prompt: null, failure: null, ended: false,
        } satisfies ProviderSignInState)
      })
    },

    dispose() {
      generation += 1
      stopPolling()
    },
  }
}

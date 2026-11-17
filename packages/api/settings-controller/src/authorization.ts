/**
 * Host owner of the `authorization` Remote namespace: the half of
 * `ctx.authorization` a browser configuration page drives.
 *
 * The seam answers a `begin()` call only after the flow has finished, because
 * its caller is assumed to render notices and prompts in-process. A page cannot
 * hold a request open for the minutes an account sign-in takes, so this
 * controller runs the attempt in the background and republishes its notices,
 * its pending question, and its outcome as pollable state addressed by the
 * credential the attempt authorizes. One attempt per key at a time is the
 * seam's own rule, which is why the key — not an attempt handle — is the
 * address a request names: a reloaded page rejoins the attempt it left behind
 * instead of starting a second one.
 *
 * @module @deepseek-ai/dsh-api-settings-controller/src/authorization
 */

import type { Context } from '@deepseek-ai/cordis'
import type { AuthorizationEntry, AuthorizationNotice, AuthorizationPrompt, AuthorizationService } from '@deepseek-ai/dsh-authorization'
import { credentialKeyId, credentialKeyScope } from '@deepseek-ai/dsh-credentials'
import type { CredentialKey } from '@deepseek-ai/dsh-credentials'
import { Remote, RemoteError, TypertRemoteService } from '@deepseek-ai/dsh-typert-protocol'
import { z } from 'zod'
import type {
  AuthorizationAttemptView, AuthorizationFlowView, AuthorizationNoticeView, AuthorizationPhase, AuthorizationPromptView,
} from './types.ts'

/**
 * Notices one attempt keeps for a late reader. A sign-in reports a handful —
 * an address, a device code, a progress line — so this is far above any real
 * flow and still keeps a chatty one from growing without bound; the oldest
 * entries fall off first, and the newest instruction is what a page needs.
 */
const NOTICE_LIMIT = 64

/**
 * How long an attempt may stay unanswered. A page that navigated away leaves a
 * pending question behind, and an attempt nobody answers would hold its key
 * forever, refusing the sign-in the human comes back to start. Fifteen minutes
 * matches the lifetime of the device code the longest of these flows displays.
 */
const ATTEMPT_LIFETIME_MS = 15 * 60_000

const keyRequestSchema = z.object({ key: z.string().min(1) })
const beginRequestSchema = z.object({
  key: z.string().min(1),
  method: z.string().min(1).nullish(),
})
const answerRequestSchema = z.object({
  key: z.string().min(1),
  promptId: z.number().int().nonnegative(),
  value: z.string(),
})

/** Parse the domain constraints that are more specific than generated TypeScript codecs. */
function parseRequest<T>(method: string, schema: z.ZodType<T>, value: unknown): T {
  const parsed = schema.safeParse(value)
  if (!parsed.success) {
    throw new RemoteError('gateway/bad-request', `invalid payload for ${method}`, { issues: parsed.error.issues })
  }
  return parsed.data
}

/** One question waiting on the page, with the handles that retire it. */
interface PendingPrompt {
  readonly promptId: number
  readonly prompt: AuthorizationPrompt
  readonly resolve: (value: string) => void
  readonly reject: (error: Error) => void
  /** Releases the flow's own withdrawal listener, when the question carried one. */
  detach?: () => void
}

/** One attempt in flight, or the last one that key ran. */
interface Attempt {
  readonly key: CredentialKey
  readonly controller: AbortController
  readonly notices: AuthorizationNoticeView[]
  phase: AuthorizationPhase
  error?: string
  prompt?: PendingPrompt | undefined
  nextPromptId: number
  timer: ReturnType<typeof setTimeout>
}

declare module '@deepseek-ai/cordis' {
  interface Context {
    /** Host owner of the `authorization` Remote namespace. */
    authorizationController: AuthorizationController
  }
}

/**
 * Host service backing the generated `ctx.remote.authorization` namespace. It
 * owns every lifecycle obligation the seam itself does not: the background run
 * whose rejection has no caller, the pending question's release on cancel,
 * replacement, and expiry, and the projection that keeps a `CredentialKey`
 * brand and an `AbortSignal` off the wire.
 */
export class AuthorizationController extends TypertRemoteService {
  private readonly attempts = new Map<CredentialKey, Attempt>()

  /** @param ctx - Host context where an authorization service may be mounted. */
  constructor(ctx: Context) {
    super(ctx, 'authorizationController', { namespace: 'authorization' })
    ctx.effect(() => () => {
      for (const attempt of this.attempts.values()) {
        attempt.controller.abort()
        this.retire(attempt, 'cancelled')
      }
      this.attempts.clear()
    }, 'authorization controller: attempts lifetime')
  }

  /**
   * Every registered flow, for a page listing what can be authorized here.
   * @returns one entry per flow, with the credential's own addressing units.
   * @throws RemoteError when no authorization service is mounted.
   */
  @Remote
  list(): AuthorizationFlowView[] {
    return this.seam().list().map(flow => ({
      key: flow.key,
      scope: credentialKeyScope(flow.key),
      id: credentialKeyId(flow.key),
      label: flow.label,
      methods: flow.methods.map(method => ({ id: method.id, label: method.label })),
      inFlight: flow.inFlight,
    }))
  }

  /**
   * Start one attempt and answer with the state it has already reached. The run
   * continues in the background: the returned view is a snapshot the caller
   * refreshes through {@link state}.
   * @param key - joined `<scope>/<id>` credential key of a registered flow.
   * @param method - method id to run; absent runs the flow's first.
   * @returns the attempt's current state.
   * @throws RemoteError when the key names no registered flow, the method is not offered, an attempt
   *   for that key is already running, or no authorization service is mounted.
   */
  @Remote
  begin(key: string, method: string | undefined): AuthorizationAttemptView {
    const request = parseRequest('authorization.begin', beginRequestSchema, { key, method })
    const seam = this.seam()
    const flow = this.flow(seam, request.key)
    const chosen = request.method ?? flow.methods[0]?.id
    if (chosen === undefined || !flow.methods.some(candidate => candidate.id === chosen)) {
      throw new RemoteError(
        'authorization/unknown-method', `flow "${flow.key}" offers no method "${chosen ?? ''}"`,
        { key: request.key, method: chosen ?? '' })
    }
    if (this.lookup(flow.key)?.phase === 'running') {
      throw new RemoteError(
        'authorization/in-flight', `an authorization attempt for "${flow.key}" is already running`, { key: request.key })
    }
    const attempt: Attempt = {
      key: flow.key,
      controller: new AbortController(),
      notices: [],
      phase: 'running',
      nextPromptId: 0,
      timer: setTimeout(() => {
        this.cancelAttempt(attempt)
        this.retire(attempt, 'failed', 'The sign-in attempt expired before it was answered.')
      }, ATTEMPT_LIFETIME_MS),
    }
    attempt.timer.unref()
    this.attempts.set(flow.key, attempt)
    void seam.begin({
      key: flow.key,
      method: chosen,
      signal: attempt.controller.signal,
      interaction: {
        notify: (notice) => { this.record(attempt, notice) },
        prompt: prompt => this.ask(attempt, prompt),
      },
    }).then(
      (outcome) => { this.retire(attempt, outcome.status === 'authorized' ? 'authorized' : 'cancelled') },
      (error: unknown) => {
        // A withdrawn attempt arrives here too when the flow ignores its
        // signal; the caller withdrew it, so it ended as a cancellation.
        if (attempt.controller.signal.aborted) { this.retire(attempt, 'cancelled'); return }
        this.retire(attempt, 'failed', error instanceof Error ? error.message : String(error))
      },
    )
    return this.view(attempt)
  }

  /**
   * Read the state a key's attempt reached, or the terminal state it settled
   * into. A page that reloaded mid-sign-in rejoins through this call.
   * @param key - joined `<scope>/<id>` credential key.
   * @returns the attempt's current state, or null when this key has not been attempted since the
   *   controller mounted.
   * @throws RemoteError when the request is invalid.
   */
  @Remote
  state(key: string): AuthorizationAttemptView | null {
    const request = parseRequest('authorization.state', keyRequestSchema, { key })
    const attempt = this.lookup(request.key)
    return attempt === undefined ? null : this.view(attempt)
  }

  /**
   * Answer the question an attempt is waiting on. The prompt id guards against
   * a page answering a question that has already been replaced or withdrawn.
   * @param key - joined `<scope>/<id>` credential key.
   * @param promptId - id of the question being answered, as last read.
   * @param value - the typed text, or a `select` answer's option id.
   * @returns the attempt's state after the answer was delivered.
   * @throws RemoteError when the attempt is gone, has no pending question, or the id is stale.
   */
  @Remote
  answer(key: string, promptId: number, value: string): AuthorizationAttemptView {
    const request = parseRequest('authorization.answer', answerRequestSchema, { key, promptId, value })
    const attempt = this.require(request.key)
    const pending = attempt.prompt
    if (attempt.phase !== 'running' || pending === undefined || pending.promptId !== request.promptId) {
      throw new RemoteError(
        'authorization/no-prompt', `attempt for "${request.key}" is not waiting on that question`,
        { key: request.key, promptId: request.promptId })
    }
    attempt.prompt = undefined
    pending.detach?.()
    pending.resolve(request.value)
    return this.view(attempt)
  }

  /**
   * Withdraw the attempt running for a key, if any. Idempotent: cancelling a
   * finished or unknown attempt is not an error, because a page may close after
   * the flow already settled.
   * @param key - joined `<scope>/<id>` credential key.
   * @throws RemoteError when the request is invalid.
   */
  @Remote
  cancel(key: string): void {
    const request = parseRequest('authorization.cancel', keyRequestSchema, { key })
    const attempt = this.lookup(request.key)
    if (attempt !== undefined) this.cancelAttempt(attempt)
  }

  /** Resolve the mounted seam or report how to supply it. */
  private seam(): AuthorizationService {
    const authorization: AuthorizationService | undefined = this.ctx.get('authorization')
    if (authorization === undefined) {
      throw new RemoteError(
        'gateway/internal',
        'authorization service is absent: this deployment mounts no @deepseek-ai/dsh-authorization in its composition',
        {},
      )
    }
    return authorization
  }

  /** The registered flow for a joined key, or the refusal a page shows. */
  private flow(seam: AuthorizationService, key: string): AuthorizationEntry {
    const entry = seam.list().find(candidate => candidate.key === key)
    if (entry === undefined) {
      throw new RemoteError('authorization/unknown-flow', `no authorization flow is registered for "${key}"`, { key })
    }
    return entry
  }

  /** Look one attempt up by the key its caller named. */
  private lookup(key: string): Attempt | undefined {
    for (const [candidate, attempt] of this.attempts) {
      if (candidate === key) return attempt
    }
    return undefined
  }

  /** Look one attempt up by key, refusing a key that has no attempt. */
  private require(key: string): Attempt {
    const attempt = this.lookup(key)
    if (attempt === undefined) {
      throw new RemoteError('authorization/no-attempt', `"${key}" has no authorization attempt`, { key })
    }
    return attempt
  }

  /** Keep one notice, dropping the oldest when the attempt has heard too many. */
  private record(attempt: Attempt, notice: AuthorizationNotice): void {
    if (attempt.phase !== 'running') return
    attempt.notices.push(projectNotice(notice))
    if (attempt.notices.length > NOTICE_LIMIT) attempt.notices.splice(0, attempt.notices.length - NOTICE_LIMIT)
  }

  /**
   * Put one flow question to the page. The flow's own withdrawal signal — how a
   * flow retires the losing question of a race against a browser callback —
   * rejects here as a surface withdrawal, not as a human declining, so the
   * attempt keeps running.
   */
  private ask(attempt: Attempt, prompt: AuthorizationPrompt): Promise<string> {
    if (attempt.phase !== 'running') {
      return Promise.reject(new Error('the authorization attempt is no longer running'))
    }
    if (attempt.prompt !== undefined) {
      this.retirePrompt(attempt.prompt, new Error('the question was replaced'))
    }
    return new Promise<string>((resolve, reject) => {
      const pending: PendingPrompt = { promptId: attempt.nextPromptId++, prompt, resolve, reject }
      const signal = prompt.signal
      if (signal !== undefined) {
        const withdraw = (): void => {
          if (attempt.prompt !== pending) return
          attempt.prompt = undefined
          reject(new Error('the question was withdrawn'))
        }
        signal.addEventListener('abort', withdraw, { once: true })
        pending.detach = () => { signal.removeEventListener('abort', withdraw) }
      }
      attempt.prompt = pending
    })
  }

  /** Reject one pending question so the flow stops waiting on a page that left. */
  private retirePrompt(pending: PendingPrompt, error: Error): void {
    pending.detach?.()
    pending.reject(error)
  }

  /** Abort the running flow; the seam settles the attempt as cancelled. */
  private cancelAttempt(attempt: Attempt): void {
    if (attempt.phase !== 'running') return
    attempt.controller.abort()
  }

  /** Move an attempt to its terminal state and release everything it held. */
  private retire(attempt: Attempt, phase: AuthorizationPhase, error?: string): void {
    if (attempt.phase !== 'running') return
    clearTimeout(attempt.timer)
    attempt.phase = phase
    if (error !== undefined) attempt.error = error
    const pending = attempt.prompt
    attempt.prompt = undefined
    if (pending !== undefined) this.retirePrompt(pending, new Error('the authorization attempt ended'))
  }

  /** Project one attempt onto the wire vocabulary. */
  private view(attempt: Attempt): AuthorizationAttemptView {
    return {
      key: attempt.key,
      phase: attempt.phase,
      notices: [...attempt.notices],
      prompt: attempt.prompt === undefined ? null : projectPrompt(attempt.prompt),
      ...attempt.error === undefined ? {} : { error: attempt.error },
    }
  }
}

/** Project one notice onto the wire vocabulary, field by field. */
function projectNotice(notice: AuthorizationNotice): AuthorizationNotice {
  return {
    message: notice.message,
    ...notice.url === undefined ? {} : { url: notice.url },
    ...notice.code === undefined ? {} : { code: notice.code },
  }
}

/** Project one question onto the wire vocabulary; its withdrawal signal stays on this side. */
function projectPrompt(pending: PendingPrompt): AuthorizationPromptView {
  const prompt = pending.prompt
  switch (prompt.kind) {
    case 'select':
      return {
        promptId: pending.promptId,
        kind: 'select',
        message: prompt.message,
        options: prompt.options.map(option => ({
          id: option.id,
          label: option.label,
          ...option.description === undefined ? {} : { description: option.description },
        })),
      }
    case 'secret':
      return {
        promptId: pending.promptId,
        kind: 'secret',
        message: prompt.message,
        ...prompt.placeholder === undefined ? {} : { placeholder: prompt.placeholder },
      }
    default:
      return {
        promptId: pending.promptId,
        kind: 'text',
        message: prompt.message,
        ...prompt.placeholder === undefined ? {} : { placeholder: prompt.placeholder },
      }
  }
}

export default AuthorizationController

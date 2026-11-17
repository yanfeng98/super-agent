import { describe, expect, it, vi } from 'vitest'
import { Context } from '@deepseek-ai/cordis'
import { credentialKey } from '@deepseek-ai/dsh-credentials'
import AuthorizationService from '@deepseek-ai/dsh-authorization'
import type { AuthorizationFlow, AuthorizationSession } from '@deepseek-ai/dsh-authorization'
import { remoteErrorOf, remoteMethods } from '@deepseek-ai/dsh-typert-protocol'
import AuthorizationController from '../src/authorization.ts'
import type { AuthorizationAttemptView } from '../src/types.ts'
import { MemoryCredentials } from '../../../credentials/credentials/tests/memory.ts'

const KEY = 'llm-pi-ai/openai-codex'
const OTHER = 'llm-pi-ai/anthropic'

/** A host composition with the record store a sign-in commits through. */
async function harness(): Promise<{ ctx: Context; controller: AuthorizationController }> {
  const ctx = new Context()
  await ctx.plugin(MemoryCredentials)
  await ctx.plugin(AuthorizationService)
  await ctx.plugin(AuthorizationController)
  return { ctx, controller: ctx.authorizationController }
}

/** A flow that commits its key after running `steps`, so a test drives the conversation. */
function flow(ctx: Context, key = credentialKey('llm-pi-ai', 'openai-codex'), steps?: (session: AuthorizationSession) => Promise<void>): AuthorizationFlow {
  return {
    key,
    label: 'ChatGPT',
    methods: [{ id: 'oauth', label: 'Sign in with ChatGPT' }, { id: 'api-key', label: 'Paste a key' }],
    async run(session) {
      await steps?.(session)
      await ctx.credentials.modifyRecord(key, () => Promise.resolve({ kind: 'grant', payload: { token: 'granted' } }))
    },
  }
}

/** Wait for the attempt to put a question to the page. */
async function questioned(controller: AuthorizationController, key = KEY): Promise<AuthorizationAttemptView> {
  return await vi.waitFor(async () => {
    const view = controller.state(key)
    if (view === null || view.prompt === null) throw new Error('the attempt has not asked a question yet')
    return view
  })
}

/** Wait for the attempt to reach a terminal phase. */
async function settled(controller: AuthorizationController, key = KEY): Promise<AuthorizationAttemptView> {
  return await vi.waitFor(async () => {
    const view = controller.state(key)
    if (view === null || view.phase === 'running') throw new Error('the attempt has not settled yet')
    return view
  })
}

/** Recover a refused call — thrown synchronously or rejected — without asserting on the class. */
async function refusal(call: () => unknown): Promise<unknown> {
  try {
    return await call()
  } catch (error: unknown) {
    return error
  }
}

/** Wait for the attempt to report a notice, without requiring a question. */
async function noticed(controller: AuthorizationController, key = KEY): Promise<AuthorizationAttemptView> {
  return await vi.waitFor(async () => {
    const view = controller.state(key)
    if (view === null || view.notices.length === 0) throw new Error('the attempt has not reported anything yet')
    return view
  })
}

describe('the authorization Remote namespace a configuration surface calls', () => {
  it('publishes the authorization namespace from its own service key', async () => {
    const { controller } = await harness()
    expect(controller.typertRemote.serviceKey).toBe('authorizationController')
    expect(controller.typertRemote.namespace).toBe('authorization')
    expect(remoteMethods(controller)).toEqual([
      { method: 'list', invocation: { kind: 'direct' } },
      { method: 'begin', invocation: { kind: 'direct' } },
      { method: 'state', invocation: { kind: 'direct' } },
      { method: 'answer', invocation: { kind: 'direct' } },
      { method: 'cancel', invocation: { kind: 'direct' } },
    ])
  })

  it('lists a registered flow with the credential key split into its own addressing units', async () => {
    const { ctx, controller } = await harness()
    ctx.authorization.registerFlow(flow(ctx))

    expect(controller.list()).toEqual([{
      key: KEY,
      scope: 'llm-pi-ai',
      id: 'openai-codex',
      label: 'ChatGPT',
      methods: [{ id: 'oauth', label: 'Sign in with ChatGPT' }, { id: 'api-key', label: 'Paste a key' }],
      inFlight: false,
    }])
  })

  it('runs the flow in the background, republishing notices, the question, and the committed outcome', async () => {
    const { ctx, controller } = await harness()
    let answered: string | undefined
    ctx.authorization.registerFlow(flow(ctx, undefined, async (session) => {
      session.notify({ message: 'Open this page', url: 'https://example.test/auth', code: 'ABCD-1234' })
      answered = await session.prompt({ kind: 'text', message: 'Paste the code', placeholder: 'code' })
    }))

    const started = controller.begin(KEY, undefined)
    expect(started.phase).toBe('running')

    const asking = await questioned(controller)
    expect(asking.notices).toEqual([
      { message: 'Open this page', url: 'https://example.test/auth', code: 'ABCD-1234' },
    ])
    expect(asking.prompt).toMatchObject({ kind: 'text', message: 'Paste the code', placeholder: 'code' })

    controller.answer(KEY, asking.prompt!.promptId, 'THE-CODE')
    const finished = await settled(controller)
    expect(finished.phase).toBe('authorized')
    expect(finished.prompt).toBeNull()
    expect(answered).toBe('THE-CODE')
    expect(await ctx.credentials.readRecord(credentialKey('llm-pi-ai', 'openai-codex')))
      .toEqual({ kind: 'grant', payload: { token: 'granted' } })
  })

  it('carries a select question through unanswered until the page picks an option id', async () => {
    const { ctx, controller } = await harness()
    let answered: string | undefined
    ctx.authorization.registerFlow(flow(ctx, undefined, async (session) => {
      answered = await session.prompt({
        kind: 'select',
        message: 'Choose a method',
        options: [{ id: 'browser', label: 'Browser login' }, { id: 'device', label: 'Device code', description: 'headless' }],
      })
    }))

    controller.begin(KEY, 'oauth')
    const asking = await questioned(controller)
    expect(asking.prompt).toEqual({
      promptId: 0,
      kind: 'select',
      message: 'Choose a method',
      options: [{ id: 'browser', label: 'Browser login' }, { id: 'device', label: 'Device code', description: 'headless' }],
    })

    controller.answer(KEY, 0, 'device')
    expect((await settled(controller)).phase).toBe('authorized')
    expect(answered).toBe('device')
  })

  it('rejoins an attempt a reloaded page left behind instead of starting a second one', async () => {
    const { ctx, controller } = await harness()
    ctx.authorization.registerFlow(flow(ctx, undefined, async (session) => {
      session.notify({ message: 'Waiting for the browser' })
      await new Promise(() => {})
    }))

    controller.begin(KEY, undefined)
    await noticed(controller)
    expect(controller.list()[0]?.inFlight).toBe(true)
    expect((controller.state(KEY))?.notices).toEqual([{ message: 'Waiting for the browser' }])

    const failure = await refusal(() => controller.begin(KEY, undefined))
    expect(remoteErrorOf(failure)).toMatchObject({ code: 'authorization/in-flight', details: { key: KEY } })
  })

  it('reports a key no flow writes and a method a flow does not offer', async () => {
    const { ctx, controller } = await harness()
    ctx.authorization.registerFlow(flow(ctx))

    expect(remoteErrorOf(await refusal(() => controller.begin(OTHER, undefined))))
      .toMatchObject({ code: 'authorization/unknown-flow', details: { key: OTHER } })
    expect(remoteErrorOf(await refusal(() => controller.begin(KEY, 'saml'))))
      .toMatchObject({ code: 'authorization/unknown-method', details: { key: KEY, method: 'saml' } })
    expect(controller.state(KEY)).toBeNull()
  })

  it('refuses an answer no question is waiting on, and one whose id was replaced', async () => {
    const { ctx, controller } = await harness()
    ctx.authorization.registerFlow(flow(ctx, undefined, async (session) => {
      await session.prompt({ kind: 'text', message: 'First' })
      await session.prompt({ kind: 'text', message: 'Second' })
    }))

    controller.begin(KEY, undefined)
    const first = await questioned(controller)
    controller.answer(KEY, first.prompt!.promptId, 'one')
    const second = await questioned(controller)
    expect(second.prompt?.message).toBe('Second')

    expect(remoteErrorOf(await refusal(() => controller.answer(KEY, first.prompt!.promptId, 'stale'))))
      .toMatchObject({ code: 'authorization/no-prompt', details: { key: KEY, promptId: first.prompt!.promptId } })
    controller.answer(KEY, second.prompt!.promptId, 'two')
    expect((await settled(controller)).phase).toBe('authorized')
  })

  it('settles a cancelled attempt as cancelled and aborts the flow', async () => {
    const { ctx, controller } = await harness()
    let aborted = false
    ctx.authorization.registerFlow(flow(ctx, undefined, async (session) => {
      session.signal.addEventListener('abort', () => { aborted = true }, { once: true })
      await new Promise((resolve) =>{  session.signal.addEventListener('abort', () => { resolve(undefined) }, { once: true }) })
      throw new Error('the flow stopped')
    }))

    controller.begin(KEY, undefined)
    controller.cancel(KEY)
    expect((await settled(controller)).phase).toBe('cancelled')
    expect(aborted).toBe(true)
    // Idempotent: a page closing after the flow settled must not fail.
    expect(() => { controller.cancel(KEY) }).not.toThrow()
    expect(() => { controller.cancel(OTHER) }).not.toThrow()
  })

  it('settles a failing flow as failed with the flow’s own message', async () => {
    const { ctx, controller } = await harness()
    ctx.authorization.registerFlow(flow(ctx, undefined, async () => {
      throw new Error('the provider refused the grant')
    }))

    controller.begin(KEY, undefined)
    const finished = await settled(controller)
    expect(finished.phase).toBe('failed')
    expect(finished.error).toBe('the provider refused the grant')
  })

  it('withdraws the pending question when the attempt is cancelled', async () => {
    const { ctx, controller } = await harness()
    let promptRejected = false
    ctx.authorization.registerFlow(flow(ctx, undefined, async (session) => {
      await session.prompt({ kind: 'text', message: 'Paste the code' })
        .catch(() => { promptRejected = true })
      throw new Error('the flow stopped')
    }))

    controller.begin(KEY, undefined)
    await questioned(controller)
    controller.cancel(KEY)
    expect((await settled(controller)).phase).toBe('cancelled')
    expect(promptRejected).toBe(true)
  })

  it('releases a running attempt when the controller is disposed', async () => {
    const ctx = new Context()
    await ctx.plugin(MemoryCredentials)
    await ctx.plugin(AuthorizationService)
    const fiber = await ctx.plugin(AuthorizationController)
    const controller = ctx.authorizationController
    let aborted = false
    ctx.authorization.registerFlow(flow(ctx, undefined, async (session) => {
      session.signal.addEventListener('abort', () => { aborted = true }, { once: true })
      await new Promise((resolve) => { session.signal.addEventListener('abort', () => { resolve(undefined) }, { once: true }) })
    }))

    controller.begin(KEY, undefined)
    await fiber.dispose()
    expect(aborted).toBe(true)
  })

  it('reports the missing service while the composition mounts no authorization seam', async () => {
    const ctx = new Context()
    await ctx.plugin(MemoryCredentials)
    await ctx.plugin(AuthorizationController)
    const controller = ctx.authorizationController

    expect(remoteErrorOf(await refusal(() => controller.begin(KEY, undefined))))
      .toMatchObject({ code: 'gateway/internal' })
    expect(remoteErrorOf(await refusal(() => controller.list())))
      .toMatchObject({ code: 'gateway/internal' })
  })

  it('refuses a malformed request before touching the seam', async () => {
    const { ctx, controller } = await harness()
    ctx.authorization.registerFlow(flow(ctx))

    expect(remoteErrorOf(await refusal(() => controller.begin('', undefined))))
      .toMatchObject({ code: 'gateway/bad-request' })
    expect(remoteErrorOf(await refusal(() => controller.answer(KEY, -1, 'value'))))
      .toMatchObject({ code: 'gateway/bad-request' })
    expect(remoteErrorOf(await refusal(() => controller.state(''))))
      .toMatchObject({ code: 'gateway/bad-request' })
  })
})

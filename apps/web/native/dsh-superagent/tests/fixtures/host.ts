/**
 * In-memory DSH host fixture (as in tests/runtime.spec.ts) running a configured Runtime with a Feishu source,
 * allowed member Alice (`ou_alice`), listened chat `oc_team`, and `auto` eval mode. Stop it with `stop()`.
 */
import { Context } from '@deepseek-ai/cordis'
import { brandString } from '@deepseek-ai/dsh-brand'
import type { Workspace, WorkspaceId, WorkspaceWorktree } from '@deepseek-ai/dsh-workspace'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { stat } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { vi, type Mock } from 'vitest'
import type { UserMessage } from '@deepseek-ai/dsh-llm'
import { Runtime } from '../../src/app/runtime.ts'
import { Store } from '../../src/app/store.ts'
import { SettingsSchema } from '../../src/policy.ts'
import { feishuImKind, imTrigger } from '../../src/sources/feishu-im.ts'
import { setDshIm, type DshImEvent, type DshImService } from '../../src/sources/dsh-im.ts'
import { localKind } from '../../src/sources/local.ts'
import { Commands } from '../../src/infra/commands.ts'
import type { Trigger } from '../../src/core/index.ts'
import { testMachine } from './deployment.ts'

export const MEMBER = 'ou_alice'
export const CHAT = 'oc_team'

/**
 * The Workspace registry as the Executor and Runtime use it, backed by one
 * in-memory record per path. It records titles, parents, and worktree
 * presentation, so a test can inspect what the sidebar would receive.
 */
export function workspaceRegistryStub() {
  const records = new Map<string, Workspace>()
  const create = async (cwd: string): Promise<Workspace> => {
    const existing = records.get(cwd)
    if (existing !== undefined) return existing
    let title = cwd.split(/[\\/]/).filter(Boolean).at(-1) ?? cwd
    let parentId: WorkspaceId | undefined
    let worktree: WorkspaceWorktree | undefined
    const entity: Workspace = {
      id: brandString<WorkspaceId>(cwd),
      path: cwd,
      get title() { return title },
      get parentId() { return parentId },
      get worktree() { return worktree },
      createdAt: '2026-01-01T00:00:00.000Z',
      updatedAt: '2026-01-01T00:00:00.000Z',
      sessionIds: [],
      getSessionHeaders: () => [],
      setTitle: async (next) => { title = next },
      setPresentation: async (presentation) => { parentId = presentation.parentId; worktree = presentation.worktree },
      attachSession: async () => {},
      insertSessionBefore: async () => {},
      detachSession: async () => {},
      status: async () => await stat(cwd).then(entry => entry.isDirectory() ? 'ok' as const : 'missing-dir' as const, () => 'missing-dir' as const),
    }
    records.set(cwd, entity)
    return entity
  }
  return {
    create,
    list: (): Workspace[] => [...records.values()],
    get: (id: WorkspaceId): Workspace | undefined => [...records.values()].find(workspace => workspace.id === id),
    delete: async (id: WorkspaceId): Promise<boolean> => {
      for (const [path, workspace] of records) {
        if (workspace.id !== id) continue
        records.delete(path)
        return true
      }
      return false
    },
  }
}

/** The dsh-im bot the fixture's Source is bound to, and the account fingerprint dsh-im would report for it. */
export const BOT_ID = 'bot_1'
export const FINGERPRINT = 'f'.repeat(64)
/**
 * One authenticated dsh-im event as the handoff delivers it; `overrides` take the Feishu field names a test sets
 * (`root_id`, `chat_id`, `sender_id`, `sender_type`, `chat_type`, `create_time`, `mentions`).
 */
export function feishuEvent(id: string, text: string, overrides: Record<string, unknown> = {}): DshImEvent {
  const at = overrides.create_time === undefined
    ? new Date(Date.parse('2026-09-09T00:00:00Z') + Number(id.replace(/\D/g, '') || 0) * 1000).toISOString()
    : new Date(Number(overrides.create_time)).toISOString()
  const chatId = String(overrides.chat_id ?? CHAT)
  const sender = String(overrides.sender_id ?? MEMBER)
  const mentions = (overrides.mentions as { id: string; key?: string; name?: string }[] | undefined) ?? [{ id: 'ou_bot' }]
  return { version: 1, channel: 'feishu', botId: BOT_ID, fingerprint: FINGERPRINT, eventId: `event_${id}`, messageId: `om_${id}`,
    actor: { kind: overrides.sender_type === 'bot' ? 'bot' : 'user', id: sender, name: 'Alice' },
    conversation: { kind: overrides.chat_type === 'p2p' ? 'dm' : 'group', id: chatId },
    mentions: mentions.map((mention, index) => ({ id: mention.id, key: (mention as { key?: string }).key ?? `@_user_${index + 1}`,
      ...mention.name ? { name: mention.name } : {} })),
    mentionedAccount: mentions.length > 0, messageType: 'text', at, text,
    reply: { messageId: `om_${id}`, conversationId: chatId, actorId: sender,
      ...overrides.thread_id ? { threadId: String(overrides.thread_id) } : {},
      ...overrides.root_id ? { rootId: String(overrides.root_id) } : {},
      ...overrides.parent_id ? { parentId: String(overrides.parent_id) } : {} } }
}
/** Build a Feishu mention trigger from Alice in `oc_team`, using the digits in `id` as the timestamp offset in seconds. */
export function feishu(id: string, text: string, overrides: Record<string, unknown> = {}): Trigger {
  return imTrigger(feishuEvent(id, text, overrides))
}

type RegisteredTool = { name: string; execute: (args: unknown, execution: { signal: AbortSignal }) => Promise<unknown> }
type FakeAgent = {
  id: string; status: string; inbox: { nextTurn: UserMessage[]; nextStep: UserMessage[] }
  cancel: Mock; inject: Mock<(message: UserMessage) => void>; steer: Mock<(message: UserMessage) => void>; followup: Mock<(message: UserMessage) => void>
}
/** A text message, as DSH's job tools and subagents send their notices. */
const notice = (id: string, text: string) => ({ id, role: 'user', content: [{ type: 'text', text }], source: { kind: 'tool-jobs', form: 'notice' } }) as unknown as UserMessage
type HttpHandler = (req: unknown, res: unknown) => Promise<void>
type Delivery = { sessionId: string; mode: 'queue' | 'steer'; id: string; text: string }

/**
 * DSH's job registry as the Executor uses it, with DSH's job tools: the jobs a Session can see, their events, a wait and a
 * kill. `run` starts a job a Session owns, `output` has one print, and `end` ends one, as its process does; an end no wait
 * collected reaches its owner, which it wakes when idle, as the job tools deliver it. `released` counts the waits ends
 * released.
 * @param owner - the agent of a Session, which an end reaches.
 */
export function jobRegistry(owner: (sessionId: string) => FakeAgent | undefined = () => undefined) {
  type Job = { id: string; owner: string; status: string; label?: string }
  const jobs = new Map<string, Job>()
  const listeners = new Set<{ owner: string; listener: (event: { type: string }) => void }>()
  const waits = new Map<string, ((job: Job) => void)[]>()
  let released = 0
  const emit = (job: Job, type: string) => { for (const listener of [...listeners]) if (listener.owner === job.owner) listener.listener({ type }) }
  const end = (id: string, status = 'completed') => {
    const job = jobs.get(id)!
    job.status = status
    const waiting = waits.get(id) ?? []
    waits.delete(id); released += waiting.length
    for (const wake of waiting) wake({ ...job })
    emit(job, 'settled')
    const agent = waiting.length ? undefined : owner(job.owner)
    const ended = notice(`notice-${id}`, `Job ${id} ended: ${status}`)
    if (agent?.status === 'idle') agent.followup(ended)
    else agent?.inject(ended)
  }
  const service = {
    list: vi.fn((caller: string) => [...jobs.values()].filter(job => job.owner === caller)),
    // A wait ends with the job, or fails once its signal aborts, which leaves the job's end uncollected.
    wait: vi.fn((id: string, _timeoutMs: number, _caller: string, signal?: AbortSignal) => new Promise<Job>((resolve, reject) => {
      const wake = (job: Job) => { signal?.removeEventListener('abort', abort); resolve(job) }
      const abort = () => { waits.set(id, (waits.get(id) ?? []).filter(item => item !== wake)); reject(new Error('wait aborted')) }
      waits.set(id, [...waits.get(id) ?? [], wake])
      signal?.addEventListener('abort', abort, { once: true })
    })),
    // A killed job ends a moment later, as its process does, also under a test's fake timers.
    kill: vi.fn((id: string) => { queueMicrotask(() => { end(id, 'killed') }); return 'requested' }) as Mock<(id: string) => string>,
    events: { subscribe: (filter: { owner: string }, listener: (event: { type: string }) => void) => {
      const entry = { owner: filter.owner, listener }
      listeners.add(entry)
      return () => { listeners.delete(entry) }
    } },
  }
  return { service, end, output: (id: string) => { emit(jobs.get(id)!, 'output') }, get released() { return released },
    run: (owner: string, label?: string) => { const id = `bash-${jobs.size + 1}`; jobs.set(id, { id, owner, status: 'running', label }); return id } }
}

/**
 * DSH's subagents as the Executor uses them. `spawn` starts a child of a Session, an agent while it lives, which a drain of
 * the Session's descendants refuses from then on; `settle` ends one, whose end wakes its idle parent or steers a running one
 * before a wait for the parent's children returns. Interrupting a child stops its turn and keeps its Activation, as DSH
 * 0.2.2 does; releasing it settles it, and a drain of descendants injects their ends. The catalog lists every child a
 * Session had.
 */
export function subagentRuntime(agents: Map<string, FakeAgent>) {
  const children = new Map<string, { parent: string; live: boolean; settled: PromiseWithResolvers<void> }>()
  const closed = new Set<string>()
  const settle = (id: string, wake = true) => {
    const child = children.get(id)
    if (!child?.live) return
    child.live = false
    agents.delete(id)
    const parent = agents.get(child.parent), ended = notice(`settled-${id}`, `Subagent ${id} settled`)
    if (!wake) parent?.inject(ended)
    else if (parent?.status === 'idle') parent.followup(ended)
    else parent?.steer(ended)
    child.settled.resolve()
  }
  const service = {
    waitForTreeWork: vi.fn(async (root: { id: string }) => {
      const live = [...children.values()].filter(child => child.parent === root.id && child.live)
      await Promise.all(live.map(child => child.settled.promise))
      return live.length > 0
    }),
    waitForChildren: vi.fn(async (parent: { id: string }) => {
      const live = [...children.values()].filter(child => child.parent === parent.id && child.live)
      await Promise.all(live.map(child => child.settled.promise))
      return live.length > 0
    }),
    listDescendants: vi.fn(async (id: string) => {
      const descendants = (parent: string): string[] => [...children].filter(([, child]) => child.parent === parent)
        .flatMap(([childId]) => [childId, ...descendants(childId)])
      return descendants(id).map(childId => ({ id: childId }))
    }),
    // DSH 0.2.2 interrupts a child without releasing its Activation, so it stays resident and wakes no parent.
    interrupt: vi.fn((id: string) => { agents.get(id)?.cancel({ kind: 'parent' }, { keepInbox: true }) }),
    listChildren: vi.fn(async (id: string) => [...children].filter(([, child]) => child.parent === id).map(([childId]) => ({ id: childId }))),
    drainChildren: vi.fn(async (_parent: { id: string }, ids: readonly string[]) => { for (const id of ids) settle(id) }),
    drainDescendants: vi.fn(async (parents: readonly { id: string }[]) => {
      for (const parent of parents) {
        closed.add(parent.id)
        for (const [id, child] of children) if (child.parent === parent.id) settle(id, false)
      }
    }),
  }
  const spawn = (parent: string) => {
    if (closed.has(parent)) throw new Error('DRAINING: the Session admits no new subagents')
    const id = `child-${children.size + 1}`
    children.set(id, { parent, live: true, settled: Promise.withResolvers<void>() })
    agents.set(id, { id, status: 'running', session: { header: { parentSession: parent } } } as never)
    return id
  }
  return { service, settle, spawn, live: (id: string) => !!children.get(id)?.live }
}

/** The dsh-im service the fixture's Host offers, recording what the Source asks it to send. */
function dshImService(replies: { botId: string; route: DshImEvent['reply']; text: string; options: unknown }[],
  bindings: { sessionId: string; description: { channel: string; botId: string; conversationKey: string; chatId?: string } }[],
  suppressions: string[]): DshImService {
  return { contractVersion: 1, inboundVersion: 1,
    bindExternalSessionConversation: (sessionId, description) => { bindings.push({ sessionId, description }) },
    suppressNextSessionSyncAnswer: (sessionId) => { suppressions.push(sessionId) },
    listBots: async () => [{ botId: BOT_ID, channel: 'feishu' }],
    listTargets: async () => [{ targetId: CHAT, kind: 'group', name: 'Team', route: { chatId: CHAT } }],
    chatMembersChecked: async () => ({ version: 1, chatId: CHAT, users: [{ id: MEMBER, name: 'Alice' }] }),
    chatListChecked: async () => ({ version: 1, chats: [{ chatId: CHAT, name: 'Team' }] }),
    describeBot: async () => ({ version: 1, botId: BOT_ID, channel: 'feishu', domain: 'feishu',
      account: { fingerprint: FINGERPRINT, name: 'Widgets Bot' }, connected: true, superagent: true, superagentActive: true,
      capabilities: ['exclusive-text-consumer'] }),
    inboundStatus: async () => ({ version: 1, botId: BOT_ID, superagent: true, superagentActive: true }),
    consumeInbound: async () => () => {},
    historyChecked: async () => ({ version: 1, scope: 'thread', events: [], omitted: 0, hasMore: false,
      coverage: 'provider-visible-human-text' }),
    replyChecked: async (botId, route, text, options) => {
      replies.push({ botId, route, text, options })
      return { sent: true, messageId: `om_reply_${replies.length}` }
    },
    messageChecked: async () => ({ version: 1, messageId: 'om_1', chatId: CHAT, at: '2026-09-09T00:00:00.000Z', text: '',
      messageType: 'text', sender: { kind: 'user', id: MEMBER, name: 'Alice' }, appLink: 'https://applink.feishu.cn/client/message/open?message_id=om_1',
      deleted: false }),
  }
}
/** @param seed - writes to the database before the Runtime opens it, such as records an earlier version stored. */
export async function startRuntime(seed?: (store: Store) => void, options: { local?: boolean } = {}) {
  const directory = mkdtempSync(join(tmpdir(), 'orchestrator-host-'))
  // What the Source sent through dsh-im, in order: the assertions on a reply read it instead of a CLI call.
  const replies: { botId: string; route: DshImEvent['reply']; text: string; options: unknown }[] = []
  const bindings: { sessionId: string; description: { channel: string; botId: string; conversationKey: string; chatId?: string } }[] = []
  const suppressions: string[] = []
  setDshIm(dshImService(replies, bindings, suppressions))
  vi.spyOn(Commands.prototype, 'git').mockResolvedValue('')
  vi.spyOn(Commands.prototype, 'run').mockResolvedValue('{"ok":true,"data":{}}')
  /** A prepare command (`sh -c`) waits for this while it is set. */
  const hooks: { prepare?: Promise<void> } = {}
  const subprocess = { spawn: vi.fn((spec: { argv: readonly string[]; env?: Record<string, string> }) => {
    const argv = [...spec.argv]
    while (argv[0] === 'env' || /^[A-Z_]+=/.test(argv[0] ?? '')) argv.shift()
    const reader = (text: string) => ({ readFrom: () => ({ text, nextOffset: text.length, lossy: false }) })
    const exited = { exitCode: 0, signal: null }
    // A prepare command that succeeds creates the checkout it was asked for, as a real one does.
    const done = (argv[0] === 'sh' && hooks.prepare ? hooks.prepare : Promise.resolve()).then(() => {
      if (argv[0] === 'sh' && spec.env?.DSH_SUPERAGENT_DIR) {
        mkdirSync(spec.env.DSH_SUPERAGENT_DIR, { recursive: true })
        writeFileSync(join(spec.env.DSH_SUPERAGENT_DIR, '.prepared'), '')
      }
      return exited
    })
    return { done, collected: { stdout: reader(''), stderr: reader('') } }
  }) }
  const agents = new Map<string, FakeAgent>()
  /** Each Session's next step, which takes the messages that wait for one. */
  const steps = new Map<string, () => void>(), claims = new Map<string, () => void>()
  const tools = new Map<string, Map<string, RegisteredTool>>()
  const deliveries = vi.fn<(request: Delivery) => void>()
  let http!: HttpHandler
  const resolveDefault = async (id: string) => agents.has(id) ? { agent: agents.get(id) } : { error: { code: 'session/not-found' } }
  const controller = {
    create: vi.fn(async ({ sessionId }: { sessionId: string }) => {
      if (agents.has(sessionId)) return
      const agentCtx = new Context()
      const sessionTools = new Map<string, RegisteredTool>()
      tools.set(sessionId, sessionTools)
      const register = (tool: RegisteredTool) => { sessionTools.set(tool.name, tool); return () => sessionTools.delete(tool.name) }
      agentCtx.reflect.provide('tools', { register, get: () => undefined, restrict: vi.fn(() => () => {}), guard: vi.fn(() => () => {}) })
      agentCtx.reflect.provide('skills', { register: () => () => {} })
      agentCtx.reflect.provide('systemPrompt', { section: () => () => {} })
      const inbox = { nextTurn: [] as UserMessage[], nextStep: [] as UserMessage[],
        // A pending message taken out, as DSH's inbox does by its id.
        remove: (id: string) => [inbox.nextTurn, inbox.nextStep].some(list => {
          const at = list.findIndex(message => message.id === id)
          if (at >= 0) list.splice(at, 1)
          return at >= 0
        }) }
      // A step claims each message once, all that wait for a step and, as a turn starts, the next queued one, and takes them into
      // the Session's history, unless it is `cut` off before, as a cancel in its pre-step does; DSH says each.
      const taken = new Set<string>()
      let turn = 0
      const take = (starts: boolean, cut = false) => {
        const agent = agents.get(sessionId)!
        const queued = starts ? inbox.nextTurn.filter(message => !taken.has(message.id)).slice(0, 1) : []
        for (const message of [...inbox.nextStep.filter(item => !taken.has(item.id)), ...queued]) {
          taken.add(message.id)
          ctx.emit('agent/inbox/claimed', { agent, message, turn } as never)
          if (!cut) ctx.emit('session/event', session as never, { type: 'user/message', data: message } as never)
        }
      }
      steps.set(sessionId, () => { take(false) })
      claims.set(sessionId, () => { take(false, true) })
      const deliver = (mode: 'queue' | 'steer') => vi.fn((message: UserMessage) => {
        inbox[mode === 'queue' ? 'nextTurn' : 'nextStep'].push(message)
        deliveries({ sessionId, mode, id: message.id, text: message.content.map(block => block.type === 'text' ? block.text : '').join('') })
        // A message wakes an idle agent: its turn starts at once, and DSH says so, and its first step takes what waits.
        const agent = agents.get(sessionId)!
        if (agent.status !== 'idle') return
        agent.status = 'running'
        ctx.emit('agent/status', { agent, status: 'running' } as never)
        turn++
        take(true)
      })
      const session = { id: sessionId, snapshotEvents: () => [], requestHeader: () => undefined, append: vi.fn() }
      let status = 'idle'
      const idle: (() => void)[] = []
      // A cancel drops the inbox unless kept, and the turn it stops ends a moment later, which DSH says.
      const cancel = vi.fn((_cause: unknown, options?: { keepInbox?: boolean }) => {
        if (!options?.keepInbox) { inbox.nextTurn.length = 0; inbox.nextStep.length = 0 }
        if (status === 'idle') return
        queueMicrotask(() => {
          const agent = agents.get(sessionId)!
          if (agent.status === 'idle') return
          agent.status = 'idle'
          ctx.emit('agent/status', { agent, status: 'idle' } as never)
        })
      })
      agents.set(sessionId, { id: sessionId, ctx: agentCtx, inbox, followup: deliver('queue'), steer: deliver('steer'), inject: vi.fn(), session, cancel,
        get status() { return status }, set status(next: string) { status = next; if (next === 'idle') for (const wake of idle.splice(0)) wake() },
        // Once its turn ended, as DSH's does.
        whenIdle: () => status === 'idle' ? Promise.resolve() : new Promise<void>(resolve => { idle.push(resolve) }) } as never)
    }),
    selectModel: vi.fn(async () => {}), resolveAgent: vi.fn(resolveDefault), rename: vi.fn(async () => {}),
    // The Runtime backfills Ticket worker Session sidebar groups from the same listing the panel reads.
    list: vi.fn(async () => ({ items: [] })),
    prompt: vi.fn(() => { throw new Error('Plugin delivery must not use user RPC') }),
    inspect: vi.fn(async () => ({ events: [] })),
  }
  const reader = { read: async () => ({ events: [] }), close: async () => {}, [Symbol.asyncDispose]: async () => {} }
  const persistence = { flush: vi.fn(async () => {}), open: vi.fn(async () => reader) }
  const webServer = { host: '127.0.0.1', port: 3082, register: (route: { handler: HttpHandler }) => { http = route.handler; return () => {} } }
  const jobs = jobRegistry(id => agents.get(id))
  const subagents = subagentRuntime(agents)
  const services = { agents, llm: { resolveCallConfig: vi.fn(async (value: unknown) => value) }, sessionController: controller,
    sessionPersistence: persistence, workspaceRegistry: workspaceRegistryStub(), permissionPresets: { set: () => {} }, subprocess, webServer,
    jobs: jobs.service, subagents: subagents.service,
    // Ticket worker Sessions of chosen sessions start with no sidebar group override; appends are recorded on their fake Sessions.
    sessionProjections: { summaryKeys: () => new Set(), snapshot: () => ({ asOfSeq: -1, values: { workspaceGroup: null } }) } }
  const ctx = new Context()
  for (const [name, service] of Object.entries(services)) ctx.reflect.provide(name, service)
  const seeded = new Store(join(directory, 'orchestrator.sqlite'))
  const feishuConfig = { botId: BOT_ID, chats: [{ chatId: CHAT, listen: 'all' }], directMessages: true }
  const sources = { feishu: { enabled: true, config: feishuConfig },
    ...options.local ? { local: { enabled: true, config: {} } } : {} }
  seeded.saveSettings(SettingsSchema.parse({ ...seeded.settings(), sources }))
  seed?.(seeded)
  seeded.close()
  const runtime = new Runtime(ctx, testMachine({ stateDir: directory }), vi.fn())
  await runtime.ready
  runtime.sources.register(feishuImKind)
  if (options.local) runtime.sources.register(localKind)
  const settings = runtime.store.settings()
  runtime.store.saveSettings({ ...settings, eval: { ...settings.eval, mode: 'auto' },
    policy: { ...settings.policy, sources: { feishu: { allowed: [{ id: MEMBER, name: 'Alice' }], syncChat: CHAT } } } })
  deliveries.mockClear()

  /** Parsed JSON payloads delivered to `sessionId`. */
  const relays = (sessionId: string) => deliveries.mock.calls.map(([call]) => call).filter(call => call.sessionId === sessionId)
    .map(call => JSON.parse(call.text) as Record<string, unknown>)
  const candidates = () => relays(runtime.masterId).filter(item => item.kind === 'candidate').map(item => item.trigger_id)
  const runTool = async (sessionId: string, name: string, args: unknown) => {
    const tool = tools.get(sessionId)?.get(name)
    if (!tool) throw new Error(`Tool ${name} is not registered for ${sessionId}`)
    return await tool.execute(args, { signal: new AbortController().signal }) as Record<string, unknown>
  }
  /** Route one decision with the main Session's orchestrator_route, answering its result; a refused one throws its reason. */
  async function route(item: Record<string, unknown>): Promise<Record<string, unknown>> {
    const { results } = await runTool(runtime.masterId, 'orchestrator_route', { decisions: [item] }) as { results: Record<string, unknown>[] }
    if (results[0]!.refused) throw new Error(String(results[0]!.refused))
    return results[0]!
  }
  async function acceptedTicket(trigger: Trigger) {
    runtime.receive(trigger)
    await runtime.idle()
    const result = await route({ kind: 'accept', trigger_id: trigger.id, title: `Fix ${trigger.id}`, worktree_name: 'fix-regression', acknowledgement: '收到，修复回归。' })
    await runtime.idle()
    const ticket = runtime.store.tickets.get((result.ticket as { ticket_id: string }).ticket_id)!
    return { trigger, ticket, sessionId: `worker-${ticket.id}` }
  }
  async function api(method: string, path: string, json?: unknown) {
    let status = 0, text = ''
    const chunks = json === undefined ? [] : [Buffer.from(JSON.stringify(json))]
    const req = { method, url: `/orchestrator/api/${path}`, headers: { host: '127.0.0.1:3082', 'x-orchestrator-request': '1' }, async *[Symbol.asyncIterator]() { yield* chunks } }
    await http(req, { writeHead: (code: number) => { status = code }, end: (data: string) => { text = data } })
    return { status, body: JSON.parse(text) as Record<string, unknown> }
  }
  async function stop() { await runtime.stop(); setDshIm(undefined); vi.restoreAllMocks(); rmSync(directory, { recursive: true, force: true }) }
  /** The Session's model takes its next step, which takes the messages steered in since its last. */
  const step = (sessionId: string) => { steps.get(sessionId)!() }
  /** A step of the worker claims what waits for it, and is cut off before the Session took it. */
  const claim = (sessionId: string) => { claims.get(sessionId)!() }
  return { runtime, ctx, directory, agents, tools, deliveries, relays, candidates, runTool, route, acceptedTicket, api, controller, persistence, subprocess, hooks, resolveDefault, stop,
    jobs, subagents, step, claim, replies, bindings, suppressions, workspaces: ctx.get('workspaceRegistry')! }
}
export type Host = Awaited<ReturnType<typeof startRuntime>>

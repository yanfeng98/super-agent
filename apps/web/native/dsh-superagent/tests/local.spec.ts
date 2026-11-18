import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { Engine, type Assessor, type Candidate, type Executor, type Trigger } from '../src/core/index.ts'
import { Store } from '../src/app/store.ts'
import { Commands } from '../src/infra/commands.ts'
import { HealthBoard } from '../src/infra/health.ts'
import { SettingsSchema, settingsPolicy } from '../src/policy.ts'
import { sourceContextOf, type SourceContextDeps } from '../src/app/source-context.ts'
import { SourceRegistry } from '../src/sources/registry.ts'
import { archiveDir, createLocalFollowup, createLocalRequest, localKind, OPERATOR, seedPolicy } from '../src/sources/local.ts'
import { testDeployment } from './fixtures/deployment.ts'

let dir: string
beforeEach(() => { dir = mkdtempSync(join(tmpdir(), 'local-')) })
afterEach(() => { rmSync(dir, { recursive: true, force: true }) })

function harness({ allow = true, enabled = true } = {}) {
  const store = new Store(join(dir, 'orchestrator.sqlite'))
  store.saveSettings(SettingsSchema.parse({
    policy: allow ? { sources: { local: { allowed: [{ id: OPERATOR, name: 'Operator' }] } } } : {},
    sources: { local: { enabled, config: {} } },
  }))
  const sources = new SourceRegistry(() => store.settings(), new HealthBoard())
  sources.register(localKind)
  const deployment = testDeployment({ stateDir: dir, repository: '', github: { cli: '', login: '', noreply: '' } })
  const candidates: Candidate[] = []
  const assessor: Assessor = { request: vi.fn(candidate => { candidates.push(candidate) }) }
  const executor: Executor = { start: vi.fn(async () => {}), deliver: vi.fn(async () => {}), stop: vi.fn(async () => {}) }
  const outbox = { send: (message: Parameters<Store['receipts']['add']>[0]) => { store.receipts.add(message) }, status: () => undefined,
    withdraw: () => {} }
  const engine = new Engine({ repo: store, scope: sources, policy: () => settingsPolicy(store.settings(), sources), assessor, executor,
    outbox })
  const commands = new Commands(() => deployment, {}, new AbortController().signal)
  const receive = (trigger: Trigger) => { const result = engine.receive(trigger); engine.evaluateDue(); return result }
  const parts: SourceContextDeps = { store, engine, outbox, commands, sources, deployment, ready: Promise.resolve(), configured: () => true,
    receive }
  const host = { ...parts, context: (kind: { kind: string; health: readonly string[] }) => sourceContextOf(parts, kind) }
  return { host, store, engine, sources, assessor, candidates, deployment, commands }
}

describe('local tickets', () => {
  it('archives the request, ingests its Trigger, and queues its Ticket', async () => {
    const h = harness()
    const created = await createLocalRequest(h.host, { title: '修复登录页', text: '登录页在窄屏下溢出，请修好。' })
    expect(created.request).toMatch(/^local-\d{8}T\d{6}-[0-9a-f]{8}$/)
    expect(created.thread).toBe(`local:${created.request}`)
    const file = readFileSync(created.file, 'utf8')
    expect(file).toContain('title: "修复登录页"')
    expect(file).toContain('# 修复登录页')
    expect(file).toContain('登录页在窄屏下溢出，请修好。')
    const trigger = h.store.triggers.get(created.trigger)
    expect(trigger).toMatchObject({ source: 'local', actor: { source: 'local', id: OPERATOR }, human: true, addressed: 'direct',
      text: '修复登录页\n\n登录页在窄屏下溢出，请修好。',
      channel: { source: 'local', thread: created.thread, address: { request: created.request, file: created.file } } })
    expect(h.store.threads.get(created.thread)?.ref).toEqual({ source: 'local', id: created.request })
    expect(h.store.threads.get(created.thread)?.meta.label).toBe('修复登录页')
    // The Eval that decides the request was queued for the main Session.
    expect(h.assessor.request).toHaveBeenCalled()
  })

  it('appends each outbound message to the same Markdown file', async () => {
    const h = harness()
    const created = await createLocalRequest(h.host, { title: '更新文档', text: '补一段安装说明。' })
    const channel = { source: 'local', thread: created.thread, address: { request: created.request, file: created.file } }
    const send = localKind.channel!.send
    expect(await send({ commands: h.commands }, { id: 'r1', channel, text: '已补好安装说明。', kind: 'reply' })).toBe('sent')
    const file = readFileSync(created.file, 'utf8')
    expect(file).toContain('结果 / Result')
    expect(file).toContain('已补好安装说明。')
    // Without an address there is nowhere to write, which is a permanent failure rather than a retry.
    expect(await send({ commands: h.commands }, { id: 'r2', channel: { source: 'local', thread: created.thread, address: {} },
      text: 'x', kind: 'reply' })).toBe('gone')
  })

  it('turns the request into a Ticket whose acknowledgement and result reach the archive', async () => {
    const h = harness()
    const created = await createLocalRequest(h.host, { title: '修复登录页', text: '窄屏下按钮溢出。' })
    const candidate = h.candidates.at(-1)!
    const decision = h.engine.decide(created.trigger, { kind: 'accept', title: '修复登录页', scope: '窄屏按钮',
      reason: 'requested', acknowledgement: '好的，开始处理。', evalId: candidate.evalId })
    const ticket = h.store.tickets.get(decision.ticketId!)!
    expect(ticket.channels).toContainEqual(expect.objectContaining({ source: 'local', thread: created.thread }))
    // Deliver the acknowledgement the accept queued, as the Outbox would.
    const ack = h.store.receipts.ofTicket(ticket.id).find(receipt => receipt.kind === 'acknowledgement')!
    expect(ack.channel).toMatchObject({ source: 'local', address: { request: created.request, file: created.file } })
    await localKind.channel!.send({ commands: h.commands }, ack)
    expect(readFileSync(created.file, 'utf8')).toContain('好的，开始处理。')
    // The worker reports ready and the main Session sends the result the same way.
    h.engine.assignmentStarted(ticket.id, ticket.assignments[0]!.id)
    h.engine.report(ticket.id, { state: 'ready', summary: 'done' })
    const reply = h.engine.reply({ purpose: 'result', ticketId: ticket.id, text: '已修好窄屏。' })
    await localKind.channel!.send({ commands: h.commands }, h.store.receipts.get(reply.id)!)
    expect(readFileSync(created.file, 'utf8')).toContain('已修好窄屏。')
  })

  it('reads back the replies a ticket sent, with their timestamps', async () => {
    const h = harness()
    const message = { id: 'r1', channel: { source: 'local', thread: 'local:r', address: { request: 'r' } }, text: '已修好',
      kind: 'reply' as const, ticketId: 't1' }
    h.store.receipts.add(message)
    const queued = h.store.receipts.ofTicket('t1')[0]!
    expect(queued).toMatchObject({ id: 'r1', status: 'pending', at: expect.any(String) })
    // Settling updates the receipt's time, so the panel shows the send time rather than the queue time.
    await new Promise(resolve => setTimeout(resolve, 5))
    h.store.receipts.settle(message, 'sent')
    const sent = h.store.receipts.ofTicket('t1')[0]!
    expect(sent.status).toBe('sent')
    expect(Date.parse(sent.at!)).toBeGreaterThanOrEqual(Date.parse(queued.at!))
    expect(h.store.receipts.ofTicket('t2')).toEqual([])
  })

  it('names each Trigger of a Thread with its own message id', async () => {
    const h = harness()
    const created = await createLocalRequest(h.host, { title: '线程', text: '第一条。' })
    const followup = await createLocalFollowup(h.host, { thread: created.thread, text: '第二条。' })
    const ids = [created.trigger, followup.trigger].map(id => localKind.threadContext!.messageId(h.store.triggers.get(id)!))
    expect(new Set(ids).size).toBe(2)
    expect(ids).toEqual([h.store.triggers.get(created.trigger)?.meta.messageId, h.store.triggers.get(followup.trigger)?.meta.messageId])
  })

  it('removes a follow-up from the archive when the intake refuses or fails it', async () => {
    const h = harness()
    const created = await createLocalRequest(h.host, { title: '回滚', text: '原始请求。' })
    const before = readFileSync(created.file, 'utf8')
    const context = h.host.context({ kind: 'local', health: [] })
    const failing = { ...h.host, context: () => ({ ...context, receive: async () => { throw new Error('intake failed') } }) }
    await expect(createLocalFollowup(failing, { thread: created.thread, text: '不会留下。' })).rejects.toThrow('intake failed')
    expect(readFileSync(created.file, 'utf8')).toBe(before)
    const refusing = { ...h.host, context: () => ({ ...context, receive: async () => 'filtered' as const }) }
    await expect(createLocalFollowup(refusing, { thread: created.thread, text: '也不会留下。' })).rejects.toThrow(/filtered/)
    expect(readFileSync(created.file, 'utf8')).toBe(before)
  })

  it('adds a follow-up to the same Thread and archive, and resumes the Ticket that reads it', async () => {
    const h = harness()
    const created = await createLocalRequest(h.host, { title: '修复登录页', text: '先看窄屏。' })
    const followup = await createLocalFollowup(h.host, { thread: created.thread, text: '补充：只改移动端。' })
    expect(followup.thread).toBe(created.thread)
    const file = readFileSync(created.file, 'utf8')
    expect(file).toContain('追加 / Follow-up')
    expect(file).toContain('补充：只改移动端。')
    expect(h.store.triggers.get(followup.trigger)).toMatchObject({ source: 'local', human: true,
      threads: [{ source: 'local', id: created.request }], channel: { address: { request: created.request, file: created.file } } })
    // An unknown Thread and a Thread of another Source are refused before anything is written.
    await expect(createLocalFollowup(h.host, { thread: 'local:missing', text: 'x' })).rejects.toThrow(/unknown/)
    await expect(createLocalFollowup(h.host, { thread: 'feishu:oc_1', text: 'x' })).rejects.toThrow(/unknown/)
  })

  it('appends a message once even when its delivery is retried', async () => {
    const h = harness()
    const created = await createLocalRequest(h.host, { title: '幂等', text: '只发一次。' })
    const channel = { source: 'local', thread: created.thread, address: { request: created.request, file: created.file } }
    const message = { id: 'r1', channel, text: '已修好', kind: 'reply' as const }
    expect(await localKind.channel!.send({ commands: h.commands }, message)).toBe('sent')
    expect(await localKind.channel!.send({ commands: h.commands }, message)).toBe('sent')
    expect(readFileSync(created.file, 'utf8').match(/dsh-superagent:r1/g)).toHaveLength(1)
  })

  it('keeps a pasted line break in the title out of the front matter and heading', async () => {
    const h = harness()
    const created = await createLocalRequest(h.host, { title: '修复\n登录页', text: '换行不要破坏结构。' })
    const file = readFileSync(created.file, 'utf8')
    expect(file).toContain('title: "修复 登录页"')
    expect(file).toContain('# 修复 登录页')
    expect(file.split('\n').filter(line => line === 'title: "修复 登录页"')).toHaveLength(1)
  })

  it('refuses to create a ticket while the Source is off', async () => {
    const h = harness({ enabled: false })
    await expect(createLocalRequest(h.host, { title: '不要接', text: '来源关着。' })).rejects.toThrow(/is off/)
    expect(existsSync(join(dir, 'tickets'))).toBe(false)
  })

  it('saves the operator into the Policy when the Source has no part yet', async () => {
    const h = harness({ allow: false })
    const save = vi.fn(async (_block: 'policy', value: unknown) => { h.store.saveSettings(SettingsSchema.parse({ ...h.store.settings(), policy: value })) })
    await seedPolicy({ store: h.store, settings: { save }, sources: h.sources })
    expect(save).toHaveBeenCalledTimes(1)
    expect(h.store.settings().policy.sources.local?.allowed).toEqual([{ id: OPERATOR, name: 'Operator' }])
    // A part that already allows someone is left alone.
    await seedPolicy({ store: h.store, settings: { save }, sources: h.sources })
    expect(save).toHaveBeenCalledTimes(1)
  })

  it('leaves a saved Policy part alone, even when it allows nobody', async () => {
    const h = harness()
    const store = h.store
    store.saveSettings(SettingsSchema.parse({ policy: { sources: { local: { allowed: [] } } },
      sources: { local: { enabled: true, config: {} } } }))
    const save = vi.fn(async () => ({}))
    await seedPolicy({ store, settings: { save }, sources: h.sources })
    expect(save).not.toHaveBeenCalled()
    expect(store.settings().policy.sources.local?.allowed).toEqual([])
  })

  it('defaults the archive to the state directory and rejects a relative path', () => {
    expect(archiveDir({ stateDir: '/srv/state' }, { archive: '' })).toBe('/srv/state/tickets')
    expect(archiveDir({ stateDir: '/srv/state' }, { archive: '/srv/inbox' })).toBe('/srv/inbox')
    expect(() => localKind.schema.parse({ archive: 'inbox' })).toThrow()
    expect(localKind.schema.parse({})).toEqual({ archive: '' })
  })
})

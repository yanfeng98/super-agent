/** Main-session assessor per-eval state and context loading. */
import { expect, it } from 'vitest'
import { z } from 'zod'
import type { Agent } from '@deepseek-ai/dsh-agent'
import { AssessmentSchema, EvalReadSchema, MasterAssessor, type AssessorHost } from '../src/app/assessment.ts'
import { mainAdmissionPolicy } from '../src/app/context.ts'
import { Engine, MemoryRepositories, type Executor, type Policy, type Trigger } from '../src/core/index.ts'
import type { Commands } from '../src/infra/commands.ts'
import { SettingsSchema } from '../src/policy.ts'
import type { ContextMessage, SourceKind } from '../src/sources/kind.ts'
import { testDeployment } from './fixtures/deployment.ts'
import { deferred } from './fixtures/async.ts'
import { SourceRegistry } from '../src/sources/registry.ts'

/** The fixture Source's Thread: the request itself and the comments that follow it, named `m<index>`. */
const fixtureMessages = (bodies: string[]): ContextMessage[] => [
  { id: 'root', from: 'bob', at: '2026-09-09T00:00:00.000Z', text: 'Issue body' },
  ...bodies.map((text, index) => ({ id: `m${index}`, from: 'bob', at: new Date(Date.UTC(2026, 8, 9, 0, 1, index)).toISOString(), text })),
]
/** The fixture Source: one Thread of prepared messages, read without a CLI. */
const fixtureKind = (messages: ContextMessage[], before: () => Promise<void> | undefined): SourceKind => ({
  kind: 'fixture', title: { zh: '题', en: 'Fixture' }, description: { zh: '', en: '' }, health: [],
  schema: z.object({}).strict(), fields: [],
  threadContext: {
    messageId: trigger => String(trigger.meta.messageId ?? ''),
    // The tests read only the Thread; a hold lets a read finish after its Eval was decided.
    read: async () => { await before(); return { messages, body: { messages } } },
  },
})
/** Assessor fixture backed by in-memory repositories and a fixture Source that serves the Thread's messages. */
function setup(strength: 'explicit' | 'inferred' = 'inferred', bodies = Array.from({ length: 5 }, (_, i) => `comment ${i}`),
  threadContext?: AssessorHost['threadContext']) {
  const gate = { hold: undefined as Promise<void> | undefined }
  const kind = fixtureKind(fixtureMessages(bodies), () => gate.hold)
  const commands = { withSignal() { return commands } } as unknown as Commands
  const repo = new MemoryRepositories()
  const sources = new SourceRegistry(() => SettingsSchema.parse({}))
  sources.register(kind)
  let engine!: Engine
  const host: AssessorHost = {
    get engine() { return engine }, commands, deployment: testDeployment(), settings: () => SettingsSchema.parse({}),
    threadContext: threadContext ?? (trigger => sources.threadContext(trigger)),
    effects: { enqueue: () => {}, handle: () => {} }, paused: () => false, deliver: async () => {}, candidateCard: async candidate => ({ eval_id: candidate.evalId }),
  }
  const assessor = new MasterAssessor(host)
  const policy: Policy = { version: 'v1', paused: false, evalTimeoutMs: 600_000, gate: () => ({ admissible: true, strength, basis: { via: 'followup' } }),
    revalidate: () => true, admitsInferred: rules => rules.length > 0 }
  const executor: Executor = { start: async () => {}, deliver: async () => {}, stop: async () => {} }
  const scope = { scope: () => ({ listened: true as const, observed: true }) }
  const outbox = { send: () => {}, status: () => undefined, withdraw: () => {} }
  engine = new Engine({ repo, policy: () => policy, scope, assessor, executor, outbox })
  const tools: Record<string, (args: unknown, signal: AbortSignal) => Promise<unknown>> = {}
  tools.orchestrator_eval_read = (args, signal) => assessor.read({} as Agent, EvalReadSchema.parse(args), signal)
  tools.orchestrator_eval_decide = (args, signal) => {
    const { trigger_id, ...result } = args as { trigger_id: string }
    return assessor.decide(trigger_id, AssessmentSchema.parse(result), signal)
  }
  // Per-Eval state retained by the assessor and released when the Eval ends.
  const kept = assessor as unknown as { evals: Map<string, unknown> }
  const size = () => kept.evals.size
  return { repo, engine, assessor, tools, size, gate }
}
const issue = (n: number): Trigger => ({ id: `fixture-${n}`, source: 'fixture', threads: [{ source: 'fixture', id: String(n) }], actor: { source: 'fixture', id: 'bob' }, human: true,
  at: new Date(Date.UTC(2026, 8, 9, 0, 0, n)).toISOString(), text: `issue ${n}`, url: `https://example.test/issues/${n}`, meta: { messageId: 'm0' } })
const signal = new AbortController().signal

it('keeps nothing for an Eval once it is decided, as for one that timed out', async () => {
  const s = setup()
  for (let n = 1; n <= 3; n++) {
    s.engine.receive(issue(n)); s.engine.evaluateDue()
    await s.assessor.context(`fixture-${n}`, signal)
    await s.tools.orchestrator_eval_read!({ trigger_id: `fixture-${n}`, resource: 'thread', offset: 0 }, signal)
    expect(s.size()).toBeGreaterThan(0)
    s.engine.decide(`fixture-${n}`, { kind: 'ignore', reason: 'not a request' })
    expect(s.size()).toBe(0)
  }
  s.engine.receive(issue(9)); s.engine.evaluateDue()
  await s.assessor.context('fixture-9', signal)
  const thread = s.repo.threads.get('fixture:9')!
  s.repo.threads.save({ ...thread, expired: 2, evaluating: { ...thread.evaluating!, delivered: '2000-01-01T00:00:00.000Z', deadline: '2000-01-01T00:00:00.000Z' } })
  s.engine.evaluateDue()
  expect(s.size()).toBe(0)
})

it('keeps nothing from a context read that finishes after its Eval was decided', async () => {
  const s = setup()
  const hold = deferred()
  s.gate.hold = hold.promise
  s.engine.receive(issue(1)); s.engine.evaluateDue()
  const details = s.assessor.context('fixture-1', signal)
  s.engine.decide('fixture-1', { kind: 'ignore', reason: 'not a request' })
  hold.resolve()
  await expect(details).rejects.toThrow('This assessment has finished')
  expect(s.size()).toBe(0)
})

it('reads the context of a Trigger through its Source, and none for a Source that gives none', async () => {
  const s = setup()
  s.engine.receive(issue(1)); s.engine.evaluateDue()
  const context = await s.assessor.context('fixture-1', signal)
  expect(context.thread).toMatchObject({ complete: true, loaded_messages: 6, source_found: true })
  const page = await s.tools.orchestrator_eval_read!({ trigger_id: 'fixture-1', resource: 'thread', offset: 0 }, signal) as { text: string }
  expect(JSON.parse(page.text)).toMatchObject({ next_read: null,
    messages: expect.arrayContaining([expect.objectContaining({ text: 'comment 0' })]) })
  const other: Trigger = { ...issue(2), id: 'other-2', source: 'probe', threads: [{ source: 'probe', id: '2' }], actor: { source: 'probe',
    id: 'bob' } }
  s.engine.receive(other); s.engine.evaluateDue()
  expect((await s.assessor.context('other-2', signal)).thread).toMatchObject({ error: expect.stringContaining('gives no Thread context') })
  await expect(s.tools.orchestrator_eval_read!({ trigger_id: 'other-2', resource: 'thread', offset: 0 },
    signal)).rejects.toThrow('gives no Thread context')
  // The Trigger's own text is still readable when its Source provides no Thread context.
  expect(await s.tools.orchestrator_eval_read!({ trigger_id: 'other-2', resource: 'message', offset: 0 },
    signal)).toMatchObject({ text: 'issue 2' })
})
it('routes an explicit candidate instead of assessing it: each eval tool names the routing tools', async () => {
  const s = setup('explicit')
  s.engine.receive(issue(1)); s.engine.evaluateDue()
  const routed = 'orchestrator_route with its trigger_id and kind accept, attach, stop or ignore'
  await expect(s.assessor.context('fixture-1', signal)).rejects.toThrow(routed)
  await expect(s.tools.orchestrator_eval_read!({ trigger_id: 'fixture-1', resource: 'message', offset: 0 }, signal)).rejects.toThrow(routed)
  const direction = { implementation_requested: false, target_basis: 'undetermined', requires_design_decision: false, evidence: 'chatter' }
  const decision = { trigger_id: 'fixture-1', decision: 'ignore', rule_assessments: [], direction, title: 't', scope: 's', reason: 'r' }
  await expect(s.tools.orchestrator_eval_decide!(decision, signal)).rejects.toThrow(routed)
})

it('names a fallback window in the assessment payload', async () => {
  // An empty window is not an empty discussion: the payload says why it holds nothing, so an assessment of a recalled
  // source or of a chat with no Thread replies is not read as a message with no context.
  const threadContext: AssessorHost['threadContext'] = () => ({
    messageId: 'm0',
    read: async () => ({ messages: [], status: 'unavailable', body: { status: 'unavailable' } }),
    images: () => [],
    files: () => [],
    image: async () => { throw new Error('no image') },
    file: async () => { throw new Error('no file') },
  })
  const s = setup('inferred', [], threadContext)
  s.engine.receive(issue(1)); s.engine.evaluateDue()
  expect((await s.assessor.context('fixture-1', signal)).thread)
    .toMatchObject({ complete: true, loaded_messages: 0, source_found: false, status: 'unavailable' })
})

it('asks the assessment only for decisions its tool accepts, which no longer include reopen or stop', () => {
  expect(mainAdmissionPolicy).not.toMatch(/reopen|decision=stop/)
  const direction = { implementation_requested: true, target_basis: 'specified', requires_design_decision: false, evidence: 'e' }
  const result = (decision: string, extra = {}) =>
    AssessmentSchema.safeParse({ decision, rule_assessments: [], direction, title: 't', scope: 's', reason: 'r', ...extra }).success
  expect([result('ignore'), result('unavailable'), result('reopen'), result('stop')]).toEqual([true, true, false, false])
  expect(result('ignore', { ticket_id: 't' })).toBe(false)
})

it('loads a window without the assessment instructions: the trigger once, cut messages with their length, and stubs for the rest',
  async () => {
  const bodies = Array.from({ length: 30 }, (_, i) => `comment ${i}: ${'the label reads oddly on the settings page. '.repeat(i % 4 === 0
    ? 40 : 3)}`)
  const s = setup('inferred', bodies)
  const trigger = { ...issue(1), id: 'c15', text: bodies[15]!, url: 'https://example.test/issues/1#c15', meta: { messageId: 'm15',
    kind: 'comment' } }
  s.engine.receive(trigger); s.engine.evaluateDue()
  const result = await s.assessor.context('c15', signal) as Record<string, unknown> & { thread: { messages: Record<string, unknown>[] } }
  expect(result).not.toHaveProperty('instructions')
  expect(result.text).toBe(bodies[15])
  const messages = new Map(result.thread.messages.map(message => [message.id as string, message]))
  const at = (i: number) => messages.get(`m${i}`)!
  // The trigger's own text is given once at top level; its two nearest neighbours keep more text than the rest of the window.
  expect(at(15)).toEqual({ id: expect.any(String), from: 'bob', at: expect.any(String), trigger: true })
  expect(at(16)).toMatchObject({ chars: bodies[16]!.length, text: `${bodies[16]!.slice(0, 1200)}…` })
  expect(at(12)).toMatchObject({ chars: bodies[12]!.length, text: `${bodies[12]!.slice(0, 400)}…` })
  expect(at(14)).toMatchObject({ text: bodies[14] })
  expect(at(14)).not.toHaveProperty('chars')
  // Messages outside the window are stubs that orchestrator_eval_read can read in full.
  expect(at(8)).toEqual({ id: expect.any(String), from: 'bob', preview: bodies[8]!.slice(0, 50) })
  const whole = await s.tools.orchestrator_eval_read!({ trigger_id: 'c15', resource: 'message', message_id: at(8).id }, signal)
  expect(JSON.stringify(whole)).toContain(bodies[8]!.slice(-40))
  expect(JSON.stringify(result).length).toBeLessThan(9000)
})

it('lists a window message\'s files and hands the model the saved file read', async () => {
  const opened: unknown[] = []
  const bodies = ['ok', 'see [文件] notes.txt file_x']
  const context = {
    messageId: 'm0',
    read: async () => ({ messages: fixtureMessages(bodies), body: { messages: fixtureMessages(bodies) } }),
    images: () => [],
    image: async () => { throw new Error('No images') },
    files: (text: string) => [...new Set(text.match(/\bfile_[a-zA-Z0-9_-]+\b/g) ?? [])],
    file: async (_agent: unknown, _commands: unknown, message: { id: string; text: string }, key: string) => {
      opened.push([message, key])
      return { source_message_id: message.id, file_key: key, file: { attachmentId: 'sha256:abc' } }
    },
  }
  const s = setup('inferred', bodies, () => context)
  s.engine.receive(issue(1)); s.engine.evaluateDue()
  const result = await s.assessor.context('fixture-1', signal) as { thread: { messages: { id: string; files?: string[] }[] } }
  expect(result.thread.messages.find(message => message.id === 'm1')?.files).toEqual(['m1/file_x'])
  const read = await s.tools.orchestrator_eval_read!({ trigger_id: 'fixture-1', resource: 'file', file_id: 'm1/file_x' }, signal)
  expect(read).toMatchObject({ file_key: 'file_x', file: { attachmentId: 'sha256:abc' } })
  expect(opened).toEqual([[{ id: 'm1', text: bodies[1] }, 'file_x']])
  // A file id the window never listed is refused instead of being turned into a network read.
  await expect(s.tools.orchestrator_eval_read!({ trigger_id: 'fixture-1', resource: 'file', file_id: 'm9/file_y' }, signal))
    .rejects.toThrow('Unknown file reference')
  expect(EvalReadSchema.safeParse({ trigger_id: 'fixture-1', resource: 'file' }).success).toBe(false)
})

it('lets the main Session read on from the cursor its loaded window ends with, and points a cut batch message at its whole text', async () => {
  // Every page returns a next cursor so the bounded window ends with a continuation cursor.
  const page = (cursor?: string) => {
    const n = Number(cursor ?? 0)
    const message = { id: `m${n}`, from: 'bob', at: new Date(Date.UTC(2026, 8, 9, 0, 0, n)).toISOString(), text: `message ${n}` }
    return { messages: [message], next: String(n + 1), body: { messages: [message] } }
  }
  const context = { messageId: 'm0', read: async (_commands: Commands, _part: string, cursor?: string) => page(cursor), images: () => [],
    image: async () => { throw new Error('No images') }, files: () => [], file: async () => { throw new Error('No files') } }
  const s = setup('inferred', undefined, () => context)
  s.engine.receive(issue(1))
  const long = { ...issue(2), id: 'long', threads: issue(1).threads, text: 'a long follow-up. '.repeat(40) }
  s.engine.receive(long); s.engine.evaluateDue()
  const result = await s.assessor.context('long', signal) as { thread: { next_cursor?: string }, batch: Record<string, unknown>[] }
  expect(result.thread.next_cursor).toBe('10')
  const more = await s.tools.orchestrator_eval_read!({ trigger_id: 'long', resource: 'thread', cursor: '10' }, signal)
  expect(JSON.stringify(more)).toContain('message 10')
  const shorter = { trigger_id: 'fixture-1' }
  expect(result.batch).toEqual([expect.objectContaining(shorter)])
  const s2 = setup('inferred', undefined, () => context)
  s2.engine.receive(long); s2.engine.receive({ ...issue(3), threads: issue(1).threads }); s2.engine.evaluateDue()
  const other = await s2.assessor.context('fixture-3', signal) as { batch: Record<string, unknown>[] }
  expect(other.batch).toEqual([expect.objectContaining({ trigger_id: 'long', chars: long.text.length, read: 'trigger/long' })])
})

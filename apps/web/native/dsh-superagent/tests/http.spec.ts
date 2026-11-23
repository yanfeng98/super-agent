import { expect, it } from 'vitest'
import { z } from 'zod'
import { registerHttp, sameOrigin } from '../src/app/http.ts'
import { PromptConflict } from '../src/app/prompts.ts'
import { SettingsConflict } from '../src/app/settings.ts'
import type { HttpHost } from '../src/app/http.ts'
import { SettingsSchema } from '../src/policy.ts'
import type { LocalRequest } from '../src/sources/local.ts'
import { SetupError } from '../src/setup.ts'
import { UserError, type Ticket } from '../src/core/index.ts'

type Handler = (req: unknown, res: unknown) => Promise<void>
type Headers = Record<string, string | undefined>
/** Mount management HTTP routes on a fake web server and return a request helper that includes default panel headers. */
function serve(runtime: Partial<Record<keyof HttpHost, unknown>>, host = '127.0.0.1') {
  let handler: Handler = async () => {}
  const ctx = { webServer: { host, port: 3082, register: (route: { handler: Handler }) => { handler = route.handler; return () => {} } } }
  registerHttp(ctx as never, { ready: Promise.resolve(), ...runtime } as unknown as HttpHost)
  return async (method: string, path: string, body?: unknown, headers: Headers = {}) => {
    let status = 0, text = ''
    const chunks = body === undefined ? [] : [Buffer.from(typeof body === 'string' ? body : JSON.stringify(body))]
    const sent = Object.fromEntries(Object.entries({ host: '127.0.0.1:3082', 'x-orchestrator-request': '1', ...headers }).filter(([,
      value]) => value !== undefined))
    const req = { method, url: `/orchestrator/api/${path}`, headers: sent, async *[Symbol.asyncIterator]() { yield* chunks } }
    await handler(req, { writeHead: (code: number) => { status = code }, end: (data: string) => { text = data } })
    return { status, body: JSON.parse(text) as Record<string, unknown> }
  }
}

it('answers each kind of failure with its own status', async () => {
  const saves: unknown[] = []
  const call = serve({
    store: { tickets: { get: () => undefined }, blockRevision: () => 0 },
    sources: { instances: () => [{ id: 'feishu' }] },
    settings: { save: async (...args: unknown[]) => {
      saves.push(args)
      if (args[1] === 'stale') throw new SettingsConflict('Settings changed. Reload before saving.')
      if (args[1] === 'invalid') z.object({ maxWorkers: z.number() }).parse({})
      if (args[1] === 'refused') throw new SetupError('environmentFailed')
      if (args[1] === 'broken') throw new Error('disk full')
      return { saved: true, revision: 1, restarting: false }
    } },
  })
  expect(await call('PUT', 'settings/executor', { value: 'stale', revision: 0 })).toMatchObject({ status: 409 })
  expect(await call('PUT', 'settings/executor', { value: 'invalid', revision: 0 })).toMatchObject({ status: 400, body: { issues: [{ path: 'maxWorkers' }] } })
  expect(await call('PUT', 'settings/source:feishu', { value: 'refused', revision: 0 })).toMatchObject({ status: 400, body: { setup: { code: 'environmentFailed', place: 'deployment' } } })
  expect(await call('PUT', 'settings/executor', { value: 'broken', revision: 0 })).toEqual({ status: 500, body: { error: 'disk full' } })
  // Block names are validated before calling settings.save: reject unknown blocks or non-existent Sources.
  expect(await call('PUT', 'settings/pause', { value: {}, revision: 0 })).toMatchObject({ status: 400 })
  expect(await call('PUT', 'settings/source:missing', { value: {}, revision: 0 })).toEqual({ status: 404, body: { error: 'Source not found' } })
  expect(saves).toHaveLength(4)
  expect(await call('POST', 'tickets/t9/retry')).toEqual({ status: 404, body: { error: 'Ticket not found' } })
  expect(await call('GET', 'nothing/here')).toEqual({ status: 404, body: { error: 'Unknown operation' } })
  expect(await call('POST', 'executor/pause', 'not json')).toEqual({ status: 400, body: { error: 'The request body is not JSON' } })
})

it('decodes path parameters and parses each route\'s query', async () => {
  const seen: unknown[] = []
  const call = serve({
    store: { settings: () => SettingsSchema.parse({}),
      read: { decisions: (filter: unknown, offset: number, limit: number) => { seen.push({ filter, offset, limit }); return { items: [], more: false } } } },
    links: () => () => ({ url: '', label: { zh: '', en: '' } }),
    engine: { unsubscribe: (key: string) => { seen.push(key); return true } },
    sessions: { delivery: async () => 'injected' },
  })
  expect(await call('POST', `threads/${encodeURIComponent('feishu:oc_1:om/root')}/unsubscribe`)).toMatchObject({ status: 200, body: { context_delivery: 'injected' } })
  // Invalid offsets default to 0, and limits above the route maximum are clamped.
  expect(await call('GET', 'decisions?kind=accept%2Creopen&limit=500&offset=x&window=24h')).toEqual({ status: 200, body: { items: [], next_offset: null } })
  expect(seen).toEqual(['feishu:oc_1:om/root', { filter: { kinds: ['accept', 'reopen'], since: expect.any(String) }, offset: 0, limit: 200 }])
})

it('answers only a same-origin request from the panel on the loopback address', async () => {
  const call = serve({ sessions: { delivery: async () => 'injected' }, engine: { unsubscribe: () => true } })
  const post = (headers: Headers) => call('POST', 'threads/feishu%3A1/unsubscribe', undefined, headers)
  for (const headers of [{ 'x-orchestrator-request': undefined }, { host: 'evil.example:3082' }, { host: '127.0.0.1:9999' },
    { origin: 'http://evil.example' }, { host: 'localhost:3082', origin: 'http://127.0.0.1:3082' }]) {
    expect(await post(headers), JSON.stringify(headers)).toEqual({ status: 403,
      body: { error: 'Same-origin management request required' } })
  }
  expect(await post({ host: 'localhost:3082', origin: 'http://localhost:3082' })).toMatchObject({ status: 200 })
  // Management routes follow the bound Web host, so a container or LAN bind keeps its panel.
  expect(() => serve({}, '0.0.0.0')).not.toThrow()
  expect(() => serve({}, '127.0.0.2')).not.toThrow()
  expect(() => serve({}, '::1')).not.toThrow()
  const request = (host: string) => ({ headers: { host, 'x-orchestrator-request': '1' } })
  expect(sameOrigin(request('127.0.0.2:3082'), '127.0.0.2', 3082)).toBe(true)
  expect(sameOrigin(request('[::1]:3082'), '::1', 3082)).toBe(true)
  expect(sameOrigin(request('192.168.1.5:3082'), '192.168.1.5', 3082)).toBe(true)
  expect(sameOrigin(request('evil.example:3082'), '127.0.0.1', 3082)).toBe(false)
})

it('answers a prompt save that lost a race with 409, and a settings body over 128 KiB with 400', async () => {
  const saves: unknown[] = []
  const call = serve({
    machine: { promptMaxChars: { get: () => 100_000 } },
    store: { atomic: (work: () => unknown) => work(), blockRevision: () => 0 },
    prompts: { save: () => { throw new PromptConflict('Prompt version changed to 3; compare the latest version before saving') } },
    sessions: { notify: () => {}, delivery: async () => 'injected' },
    settings: { save: async (...args: unknown[]) => { saves.push(args); return { saved: true, revision: 1, restarting: false } } },
  })
  expect(await call('PUT', 'prompts', { config: {}, baseRevision: 2 }))
    .toEqual({ status: 409, body: { error: 'Prompt version changed to 3; compare the latest version before saving' } })
  const value = { prepare: 'x'.repeat(128 * 1024) }
  expect(await call('PUT', 'settings/executor', { value, revision: 0 })).toEqual({ status: 400,
    body: { error: `Request exceeds ${128 * 1024} bytes` } })
  expect(saves).toEqual([])
})

it('creates a local ticket only when setup resolved, the Source is on, and the operator is allowed', async () => {
  const created: unknown[] = []
  const call = serve({
    configured: () => true,
    store: { settings: () => SettingsSchema.parse({ policy: { sources: { local: { allowed: [{ id: 'operator' }] } } },
      sources: { local: { enabled: true, config: {} } } }), blockRevision: () => 1,
      threads: { get: (key: string) => key === 'local:r' ? { ref: { source: 'local', id: 'r' } } : undefined } },
    sources: { instance: () => ({ enabled: true, config: {} }), policyPart: () => ({ allowed: [{ id: 'operator', name: 'Operator' }] }) },
    settings: { save: async () => ({}) },
    localRequest: async (input: LocalRequest) => { created.push(input); return { request: 'r', thread: 'local:r', trigger: 'local:r', file: '/tmp/r.md' } },
    localFollowup: async (input: { thread: string; text: string }) => ({ trigger: 'f1', thread: input.thread }),
  })
  expect(await call('POST', 'local/requests', { title: '修复登录页', text: '窄屏溢出', key: 'request-key-0001' })).toEqual({ status: 200,
    body: { request: 'r', thread: 'local:r', trigger: 'local:r', file: '/tmp/r.md' } })
  expect(created).toEqual([{ title: '修复登录页', text: '窄屏溢出', key: 'request-key-0001' }])
  // A key shorter than 8 characters is refused before the Source writes anything.
  expect(await call('POST', 'local/requests', { title: 'x', text: 'y', key: 'short' })).toMatchObject({ status: 400 })
  expect(created).toHaveLength(1)
  // Attachments travel as base64 with the request; a file name with a path is refused.
  expect(await call('POST', 'local/requests', { title: '文件', text: '看附件', files: [{ name: 'a.txt', data: 'aGk=' }] }))
    .toMatchObject({ status: 200 })
  expect(created.at(-1)).toMatchObject({ files: [{ name: 'a.txt', data: 'aGk=' }] })
  expect(await call('POST', 'local/requests', { title: 'x', text: 'y', files: [{ name: 'a/b.txt', data: 'aGk=' }] }))
    .toMatchObject({ status: 400 })
  expect(await call('POST', 'local/requests', { title: 'x', text: 'y', files: [{ name: 'a\nb.txt', data: 'aGk=' }] }))
    .toMatchObject({ status: 400 })
  expect(created).toHaveLength(2)
  expect(await call('POST', 'local/followups', { thread: 'local:r', text: '补充' })).toEqual({ status: 200,
    body: { trigger: 'f1', thread: 'local:r' } })
  expect(await call('POST', 'local/followups', { thread: '', text: 'x' })).toMatchObject({ status: 400 })
  expect(await call('POST', 'local/followups', { thread: 'local:missing', text: 'x' })).toEqual({ status: 400,
    body: { error: 'The local ticket Thread is unknown' } })
  // Unknown fields and empty fields are refused before anything is written.
  expect(await call('POST', 'local/requests', { title: 'x', text: 'y', extra: 1 })).toMatchObject({ status: 400 })
  expect(await call('POST', 'local/requests', { title: ' ', text: 'y' })).toMatchObject({ status: 400 })
  const off = serve({ configured: () => true, store: { settings: () => SettingsSchema.parse({}) }, sources: { instance: () => ({ enabled: false }) } })
  expect(await off('POST', 'local/requests', { title: 'x', text: 'y' })).toMatchObject({ status: 400, body: { error: expect.stringContaining('off') } })
  const unset = serve({ configured: () => false })
  expect(await unset('POST', 'local/requests', { title: 'x', text: 'y' })).toMatchObject({ status: 400, body: { error: expect.stringContaining('setup') } })
  // A Source whose saved Policy part allows nobody refuses instead of silently queueing a Trigger no Ticket can take.
  const nobody = serve({ configured: () => true,
    store: { settings: () => SettingsSchema.parse({ policy: { sources: { local: { allowed: [] } } },
      sources: { local: { enabled: true, config: {} } } }), blockRevision: () => 1 },
    sources: { instance: () => ({ enabled: true, config: {} }) }, settings: { save: async () => ({}) } })
  expect(await nobody('POST', 'local/requests', { title: 'x', text: 'y' })).toEqual({ status: 400,
    body: { error: 'The local Source allows nobody; add the operator under Policy → On each Source' } })
})

it('reopens an ended ticket through the panel, and only a known ticket', async () => {
  const reopened: string[] = []
  const ended = { id: 't1', ended: { at: '2026-11-18T00:00:00Z', by: 'human', outcome: 'stopped' } }
  const call = serve({ store: { tickets: { get: (id: string) => id === 't1' ? ended : undefined } },
    engine: { reopenTicket: (id: string) => { reopened.push(id); return ended } } })
  expect(await call('POST', 'tickets/t1/reopen')).toEqual({ status: 200, body: { reopened: true } })
  expect(reopened).toEqual(['t1'])
  expect(await call('POST', 'tickets/t9/reopen')).toMatchObject({ status: 404, body: { error: 'Ticket not found' } })
})

it('serves a ticket workspace diff on demand, and refuses an unknown ticket', async () => {
  const call = serve({ store: { tickets: { get: (id: string) => id === 't1' ? { id } : undefined } },
    ticketDiff: async (id: string) => ({ diff: `diff for ${id}`, truncated: false }) })
  expect(await call('GET', 'tickets/t1/diff')).toEqual({ status: 200, body: { diff: 'diff for t1', truncated: false } })
  expect(await call('GET', 'tickets/t2/diff')).toMatchObject({ status: 404 })
})

it('merges or discards a ticket workspace, and reports a git refusal as a bad request', async () => {
  const calls: string[] = []
  const call = serve({ store: { tickets: { get: (id: string) => id === 't1' ? { id } : undefined } },
    mergeTicket: async (id: string) => { calls.push(`merge:${id}`); return { branch: 'agent/t', target: 'main' } },
    discardTicket: async (id: string) => { calls.push(`discard:${id}`) },
    rebaseTicket: async (id: string) => { calls.push(`rebase:${id}`); return { branch: 'agent/t', target: 'main' } },
    checkTicket: async (id: string) => { calls.push(`check:${id}`); return { command: 'pnpm test', ok: true, exitCode: 0, at: '',
      output: 'passed' } },
    cleanupTicket: async (id: string) => { calls.push(`cleanup:${id}`) },
    linkTicket: (id: string, reference: string, linked: boolean) => { calls.push(`link:${id}:${reference}:${linked}`)
      return { related: [] } },
    blockTicket: (id: string, reference: string, blocked: boolean) => { calls.push(`block:${id}:${reference}:${blocked}`)
      return { ticket_id: id, blocked_by: [] } },
    addNote: (id: string, input: { file: string; line?: number; endLine?: number; text: string }) => {
      calls.push(`note:${id}:${input.file}:${input.endLine ?? ''}`); return [] },
    removeNote: (id: string, noteId: string) => { calls.push(`unnote:${id}:${noteId}`); return [] },
    resolveNote: (id: string, noteId: string, resolved: boolean) => { calls.push(`resolve:${id}:${noteId}:${resolved}`); return [] },
    replyToNote: (id: string, noteId: string, text: string) => { calls.push(`reply:${id}:${noteId}:${text}`); return [] },
    reviewTicket: (id: string, decision: string) => { calls.push(`review:${id}:${decision}`); return { id,
      reviewed: { at: '2026-11-18T00:00:00Z', decision } } as Ticket },
    editRequest: (id: string, text: string) => { calls.push(`request:${id}:${text}`); return { id,
      request: { text, at: '2026-11-18T00:00:00Z' } } as Ticket },
    setTags: (id: string, tags: string[]) => { calls.push(`tags:${id}:${tags.join(',')}`); return { id, tags } as Ticket } })
  expect(await call('POST', 'tickets/t1/merge')).toEqual({ status: 200, body: { merged: true, branch: 'agent/t', target: 'main' } })
  expect(await call('POST', 'tickets/t1/discard')).toEqual({ status: 200, body: { discarded: true } })
  expect(await call('POST', 'tickets/t1/rebase')).toEqual({ status: 200, body: { rebased: true, branch: 'agent/t', target: 'main' } })
  expect(await call('POST', 'tickets/t1/check')).toEqual({ status: 200, body: { checks: { command: 'pnpm test', ok: true, exitCode: 0,
    at: '', output: 'passed' } } })
  expect(await call('POST', 'tickets/t1/cleanup')).toEqual({ status: 200, body: { cleaned: true } })
  expect(await call('POST', 'tickets/t1/related', { ticket: 'T-0002', linked: true })).toEqual({ status: 200, body: { related: [] } })
  expect(await call('POST', 'tickets/t1/blockers', { ticket: 'T-0002', blocked: true })).toEqual({ status: 200, body: { ticket_id: 't1', blocked_by: [] } })
  expect(await call('POST', 'tickets/t1/notes', { file: 'src/a.ts', line: 3, endLine: 9, text: '改这里' })).toEqual({ status: 200,
    body: { notes: [] } })
  expect(await call('POST', 'tickets/t1/notes/n1/remove')).toEqual({ status: 200, body: { notes: [] } })
  expect(await call('POST', 'tickets/t1/notes/n1/resolve', { resolved: true })).toEqual({ status: 200, body: { notes: [] } })
  expect(await call('POST', 'tickets/t1/notes/n1/replies', { text: '已处理' })).toEqual({ status: 200, body: { notes: [] } })
  expect(await call('POST', 'tickets/t1/tags', { tags: ['前端', 'p1'] })).toEqual({ status: 200, body: { tags: ['前端', 'p1'] } })
  expect(await call('POST', 'tickets/t1/review', { decision: 'approved' })).toEqual({ status: 200,
    body: { reviewed: true, decision: 'approved', at: '2026-11-18T00:00:00Z' } })
  expect(await call('POST', 'tickets/t1/request', { text: '新的正文' })).toEqual({ status: 200,
    body: { request: { text: '新的正文', at: '2026-11-18T00:00:00Z' } } })
  expect(calls).toEqual(['merge:t1', 'discard:t1', 'rebase:t1', 'check:t1', 'cleanup:t1', 'link:t1:T-0002:true',
    'block:t1:T-0002:true', 'note:t1:src/a.ts:9', 'unnote:t1:n1', 'resolve:t1:n1:true', 'reply:t1:n1:已处理', 'tags:t1:前端,p1',
    'review:t1:approved', 'request:t1:新的正文'])
  expect(await call('POST', 'tickets/t9/merge')).toMatchObject({ status: 404, body: { error: 'Ticket not found' } })
  const refused = serve({ store: { tickets: { get: () => ({ id: 't1' }) } },
    mergeTicket: async () => { throw new Error('The repository has uncommitted changes') } })
  expect(await refused('POST', 'tickets/t1/merge')).toEqual({ status: 400,
    body: { error: 'The repository has uncommitted changes' } })
})

it('renames a ticket, refusing a blank title or an unknown ticket', async () => {
  const call = serve({ store: { tickets: { get: (id: string) => id === 't1' ? { id } : undefined } },
    renameTicket: (id: string, title: string) => ({ id, title }) })
  expect(await call('POST', 'tickets/t1/title', { title: '新的标题' })).toEqual({ status: 200, body: { title: '新的标题' } })
  expect(await call('POST', 'tickets/t1/title', { title: '   ' })).toMatchObject({ status: 400 })
  expect(await call('POST', 'tickets/t9/title', { title: 'x' })).toMatchObject({ status: 404 })
})

it('approves a held ticket through the panel', async () => {
  const approved: string[] = []
  const call = serve({ store: { tickets: { get: (id: string) => id === 't1' ? { id } : undefined } },
    approveTicket: (id: string) => { approved.push(id); return { id } } })
  expect(await call('POST', 'tickets/t1/approve')).toEqual({ status: 200, body: { approved: true } })
  expect(approved).toEqual(['t1'])
  expect(await call('POST', 'tickets/t9/approve')).toMatchObject({ status: 404 })
})

it('passes the requested merge strategy, defaulting to fast-forward', async () => {
  const seen: (string | undefined)[] = []
  const call = serve({ store: { tickets: { get: () => ({ id: 't1' }) } },
    mergeTicket: async (_id: string, strategy?: string) => { seen.push(strategy); return { branch: 'agent/t', target: 'main' } } })
  expect(await call('POST', 'tickets/t1/merge?strategy=squash')).toMatchObject({ status: 200, body: { merged: true } })
  expect(await call('POST', 'tickets/t1/merge?strategy=merge')).toMatchObject({ status: 200, body: { merged: true } })
  expect(await call('POST', 'tickets/t1/merge')).toMatchObject({ status: 200, body: { merged: true } })
  expect(await call('POST', 'tickets/t1/merge?strategy=rebase')).toMatchObject({ status: 400 })
  expect(seen).toEqual(['squash', 'merge', 'ff'])
})

it('lists notifications and marks one or all read', async () => {
  const item = { id: 'n1', at: '2026-11-18T00:00:00Z', kind: 'attention' as const, ticketId: 't1', title: '修复回归', text: '哪种颜色？' }
  const call = serve({ notifications: () => ({ items: [item], unread: 1 }), readNotification: (id: string) => id === 'n1',
    readAllNotifications: () => 1 })
  expect(await call('GET', 'notifications')).toEqual({ status: 200, body: { items: [item], unread: 1 } })
  expect(await call('POST', 'notifications/n1/read')).toEqual({ status: 200, body: { read: true } })
  expect(await call('POST', 'notifications/n9/read')).toMatchObject({ status: 404 })
  expect(await call('POST', 'notifications/read-all')).toEqual({ status: 200, body: { read: 1 } })
})

it('keeps a refused note range a structured error the panel can localize', async () => {
  const call = serve({ store: { tickets: { get: () => ({ id: 't1' }) } },
    addNote: () => { throw new UserError('invalidNoteRange') } })
  expect(await call('POST', 'tickets/t1/notes', { file: 'src/a.ts', endLine: 9, text: '缺起点' })).toMatchObject({ status: 400,
    body: { code: 'invalidNoteRange' } })
})

it('deletes a Thread through its panel route and reports a refusal with its code', async () => {
  const deleted: string[] = []
  const call = serve({ deleteThread: (key: string) => {
    if (key === 'feishu:busy') throw new UserError('threadBusy', { thread: key })
    deleted.push(key)
  } })
  expect(await call('POST', 'threads/feishu%3A1/delete')).toEqual({ status: 200, body: { deleted: true } })
  expect(deleted).toEqual(['feishu:1'])
  expect(await call('POST', 'threads/feishu%3Abusy/delete')).toMatchObject({ status: 400,
    body: { code: 'threadBusy', values: { thread: 'feishu:busy' } } })
})

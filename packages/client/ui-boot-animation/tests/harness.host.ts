/**
 * Shared fixtures for the Host-side unit tests: a writable fake HTTP response,
 * request builders, a fake Cordis context that records routes, effects, and
 * injected child callbacks, and an in-memory storage domain.
 *
 * The HTTP doubles are built from real stream classes and carry every member
 * `IncomingMessage`/`ServerResponse` declares, so the route handlers accept
 * them without an assertion.
 */
import { Socket } from 'node:net'
import { PassThrough, Writable } from 'node:stream'
import type {
  IncomingHttpHeaders, IncomingMessage, OutgoingHttpHeader, OutgoingHttpHeaders, ServerResponse,
} from 'node:http'
import type { Mock } from 'vitest'
import { vi } from 'vitest'
import { Context } from '@deepseek-ai/cordis'
import type { WebRoute } from '@deepseek-ai/dsh-host-webserver'
import type { Domain, KvTable, TableKeyOf, TableValueOf } from '@deepseek-ai/dsh-storage-domain'
import type { PeerScope } from '@deepseek-ai/dsh-typert-protocol'
import { bootAnimationDomain, SessionOverrides } from '../src/host/overrides.ts'
import type { SessionIntro } from '../src/types.ts'

declare module '@deepseek-ai/dsh-typert-protocol' {
  interface PrincipalMap {
    /** Test-only authenticated caller that is not the owner. */
    'boot-animation-test-stranger': { readonly kind: 'boot-animation-test-stranger' }
  }
}

/** Request fields a test drives; a body is written after the handler subscribes. */
export interface FakeRequestInit {
  method?: string
  /** Request target; `null` leaves it unset like a malformed server request. */
  url?: string | null
  headers?: Record<string, string | undefined>
  body?: string | Uint8Array
  /** Body split into several chunks, written in order. */
  chunks?: readonly (string | Uint8Array)[]
  error?: Error
}

/**
 * Build a server request from a real readable stream.
 *
 * The body is written on the next microtask so a handler that subscribes while
 * still running synchronously sees every chunk.
 * @param init - method, target, headers, and the body or error to write later.
 * @returns the request carrying the HTTP members `IncomingMessage` declares.
 */
export function fakeRequest(init: FakeRequestInit = {}): IncomingMessage {
  const socket = new Socket()
  const headers: IncomingHttpHeaders = { ...init.headers }
  const rawHeaders: string[] = []
  const rawTrailers: string[] = []
  const req = Object.assign(new PassThrough(), {
    method: init.method ?? 'GET',
    url: init.url === undefined ? '/' : init.url === null ? undefined : init.url,
    headers,
    aborted: false,
    complete: true,
    httpVersion: '1.1',
    httpVersionMajor: 1,
    httpVersionMinor: 1,
    headersDistinct: {},
    trailersDistinct: {},
    rawHeaders,
    rawTrailers,
    trailers: {},
    socket,
    connection: socket,
    setTimeout(this: IncomingMessage, _msecs: number, _callback?: () => void): IncomingMessage {
      return this
    },
  })
  queueMicrotask(() => {
    if (init.error !== undefined) {
      req.destroy(init.error)
      return
    }
    for (const chunk of init.chunks ?? (init.body === undefined ? [] : [init.body])) {
      req.write(Buffer.from(chunk))
    }
    req.end()
  })
  return req
}

/** Header values as the write path stores them: one string per name. */
function flattenHeaders(headers: OutgoingHttpHeaders | OutgoingHttpHeader[] | undefined): Record<string, string> {
  if (headers === undefined) return {}
  if (Array.isArray(headers)) {
    const flat: Record<string, string> = {}
    for (let index = 0; index + 1 < headers.length; index += 2) {
      flat[String(headers[index])] = String(headers[index + 1])
    }
    return flat
  }
  const flat: Record<string, string> = {}
  for (const [name, value] of Object.entries(headers)) {
    if (value === undefined) continue
    flat[name] = Array.isArray(value) ? value.join(', ') : String(value)
  }
  return flat
}

/**
 * A `ServerResponse` stand-in that collects the body and emits `finish`/`close`.
 *
 * It extends a real `Writable` — the class Node's response writes through — and
 * implements every remaining HTTP member the interface declares.
 */
export class FakeResponse extends Writable implements ServerResponse {
  /** Status line written by the handler; 0 until `writeHead` runs. */
  status = 0
  /** Headers of the written status line, keyed as the handler passed them. */
  headers: Record<string, string> = {}
  statusCode = 0
  statusMessage = ''
  strictContentLength = false
  chunkedEncoding = false
  shouldKeepAlive = false
  useChunkedEncodingByDefault = false
  sendDate = false
  /** No transport backs this response. */
  readonly socket: Socket | null = null
  /** No transport backs this response. */
  readonly connection: Socket | null = null
  /** The request this response answers. */
  readonly req: IncomingMessage
  private readonly chunks: Buffer[] = []

  /**
   * @param req - the request this response answers.
   */
  constructor(req: IncomingMessage) {
    super({
      write: (chunk: Buffer, _encoding, callback) => {
        this.chunks.push(Buffer.from(chunk))
        callback()
      },
    })
    this.req = req
  }

  /** Whether a status line was written; `serveClip` destroys instead of answering 500 after it is. */
  get headersSent(): boolean {
    return this.status !== 0
  }

  /** Whether the response already ended (`OutgoingMessage` bookkeeping). */
  get finished(): boolean {
    return this.writableFinished
  }

  /**
   * Record the status line.
   * @param status - status code.
   * @param statusMessageOrHeaders - optional status message, or the headers.
   * @param headers - headers when a status message was given.
   * @returns this response.
   */
  writeHead(
    statusCode: number,
    statusMessageOrHeaders?: string | OutgoingHttpHeaders | OutgoingHttpHeader[],
    headers?: OutgoingHttpHeaders | OutgoingHttpHeader[],
  ): this {
    this.status = statusCode
    this.statusCode = statusCode
    if (typeof statusMessageOrHeaders === 'string') {
      this.statusMessage = statusMessageOrHeaders
      this.headers = flattenHeaders(headers)
    } else {
      this.headers = flattenHeaders(statusMessageOrHeaders)
    }
    return this
  }

  /**
   * Record one header on the implicit header set.
   * @param name - header name.
   * @param value - header value.
   * @returns this response.
   */
  setHeader(name: string, value: number | string | readonly string[]): this {
    this.headers[name.toLowerCase()] = Array.isArray(value) ? value.join(', ') : String(value)
    return this
  }

  /**
   * Record several headers on the implicit header set.
   * @param headers - headers to merge.
   * @returns this response.
   */
  setHeaders(headers: Headers | Map<string, number | string | readonly string[]>): this {
    const entries: Iterable<readonly [string, number | string | readonly string[]]> = headers
    for (const [name, value] of entries) this.setHeader(name, value)
    return this
  }

  /**
   * Append one header value.
   * @param name - header name.
   * @param value - header value or values.
   * @returns this response.
   */
  appendHeader(name: string, value: string | readonly string[]): this {
    const previous = this.headers[name.toLowerCase()]
    const added = typeof value === 'string' ? value : value.join(', ')
    this.headers[name.toLowerCase()] = previous === undefined ? added : `${previous}, ${added}`
    return this
  }

  /**
   * Read one recorded header.
   * @param name - header name.
   * @returns the header value, or undefined when unset.
   */
  getHeader(name: string): number | string | string[] | undefined {
    return this.headers[name.toLowerCase()]
  }

  /** Recorded headers, as an outgoing header table. */
  getHeaders(): OutgoingHttpHeaders {
    return { ...this.headers }
  }

  /** Header names recorded so far. */
  getHeaderNames(): string[] {
    return Object.keys(this.headers)
  }

  /**
   * Whether one header was recorded.
   * @param name - header name.
   * @returns true when the header is present.
   */
  hasHeader(name: string): boolean {
    return name.toLowerCase() in this.headers
  }

  /**
   * Drop one recorded header.
   * @param name - header name.
   */
  removeHeader(name: string): void {
    Reflect.deleteProperty(this.headers, name.toLowerCase())
  }

  /** Trailers are not part of a status line written by this double. */
  addTrailers(_headers: OutgoingHttpHeaders | ReadonlyArray<[string, string]>): void {}

  /** No transport buffers implicit headers here. */
  flushHeaders(): void {}

  /** No transport applies a timeout here. */
  setTimeout(_msecs: number, _callback?: () => void): this {
    return this
  }

  /** No transport to assign. */
  assignSocket(_socket: Socket): void {}

  /** No transport to detach. */
  detachSocket(_socket: Socket): void {}

  /** Interim responses are not part of this double. */
  writeContinue(_callback?: () => void): void {}

  /** Early hints are not part of this double. */
  writeEarlyHints(_hints: Record<string, string | string[]>, _callback?: () => void): void {}

  /** Interim processing responses are not part of this double. */
  writeProcessing(_callback?: () => void): void {}

  /** Bytes written to the response. */
  get body(): Buffer {
    return Buffer.concat(this.chunks)
  }

  /** Body decoded as UTF-8. */
  get text(): string {
    return this.body.toString('utf8')
  }
}

/**
 * Wait until the response finished or was destroyed.
 * @param res - the fake response to observe.
 * @returns resolution after `finish` or `close`.
 */
export function responseFinished(res: FakeResponse): Promise<void> {
  if (res.destroyed || res.writableFinished) return Promise.resolve()
  return new Promise((resolve) => {
    const done = (): void => {
      res.off('finish', done)
      res.off('close', done)
      resolve()
    }
    res.once('finish', done)
    res.once('close', done)
  })
}

/** The scope a Peer hangs under; nothing this plugin reads depends on its contents. */
const PEER_CONTEXT = new Context()

/**
 * Build one admitted Peer.
 * @param id - diagnostic identity of the Peer.
 * @param kind - caller identity kind.
 * @returns the complete Peer scope.
 */
function fakePeer(id: string, kind: 'owner' | 'boot-animation-test-stranger'): PeerScope {
  return {
    // Opaque connection-layer identity: the brand has no runtime constructor,
    // and this package declares no dependency on the brand package.
    id: id as PeerScope['id'],
    ctx: PEER_CONTEXT,
    principal: { kind },
    signal: new AbortController().signal,
    interactions: 'none',
    dispose: async () => {},
  }
}

/** An authenticated owner. */
export const OWNER: PeerScope = fakePeer('boot-animation-test-owner', 'owner')

/** An authenticated identity that is not the owner. */
export const STRANGER: PeerScope = fakePeer('boot-animation-test-stranger', 'boot-animation-test-stranger')

/** A fake Cordis context plus the records its tests inspect. */
export interface FakeContextHarness {
  readonly ctx: Context
  readonly routes: WebRoute[]
  readonly routeDisposers: Mock<() => void>[]
  /** Effects registered on the plugin context. */
  readonly effects: FakeEffect[]
  /** Effects registered on injected child contexts. */
  readonly childEffects: FakeEffect[]
  /** Dependency lists passed to `ctx.inject`, in call order. */
  readonly injections: string[][]
  readonly logger: { warn: Mock<(message: string) => void> }
  readonly fiber: { readonly marker: 'boot-animation-root-fiber' }
  readonly settingsConfigure: Mock<(config: { auto: boolean }, fiber: unknown) => void>
  readonly storageOpen: Mock<(spec: typeof bootAnimationDomain) => Promise<Domain<typeof bootAnimationDomain>>>
  /** Dispose every effect registered on the plugin context. */
  disposeEffects(): void
  /** Dispose every effect registered on injected child contexts. */
  disposeChildEffects(): void
  /** Find a registered route by its path. */
  route(path: string): WebRoute
}

/** One recorded effect body and its disposer. */
export interface FakeEffect {
  readonly label: string | undefined
  readonly dispose: () => void
}

/** Options for {@link createFakeContext}. */
export interface FakeContextOptions {
  /** Overrides the storage facility's `open`. */
  open?: (spec: typeof bootAnimationDomain) => Promise<Domain<typeof bootAnimationDomain>>
  /** Replaces the settings service's `configure`. */
  configure?: (config: { auto: boolean }, fiber: unknown) => void
}

/** The injected child context the plugin's optional services hand back. */
interface FakeChildContext {
  readonly logger: { warn: Mock<(message: string) => void> }
  readonly fiber: { readonly marker: 'boot-animation-root-fiber' }
  readonly effect: (fn: () => unknown, label?: string) => () => void
  readonly storageDomain: { open: FakeContextHarness['storageOpen'] }
  readonly settings: { configure: FakeContextHarness['settingsConfigure'] }
}

/**
 * The Context members the Host plugin body calls, implemented by the harness.
 *
 * The fake is installed as own properties over a real Cordis context: the
 * context's members are service and fiber classes with private state, so a
 * hand-built object cannot satisfy `Context` structurally. Extending a real
 * context replaces only the members this plugin reads.
 */
interface FakeContext {
  readonly logger: { warn: Mock<(message: string) => void> }
  readonly fiber: { readonly marker: 'boot-animation-root-fiber' }
  effect(fn: () => unknown, label?: string): () => void
  inject(names: string[], callback: (injected: FakeChildContext) => unknown): () => void
  readonly webServer: { register(route: WebRoute): () => void }
}

/**
 * Build a fake plugin context.
 * @param options - storage/settings overrides.
 * @returns the harness exposing the context and its records.
 */
export function createFakeContext(options: FakeContextOptions = {}): FakeContextHarness {
  const effects: FakeEffect[] = []
  const childEffects: FakeEffect[] = []
  const routes: WebRoute[] = []
  const routeDisposers: Mock<() => void>[] = []
  const injections: string[][] = []
  const logger = { warn: vi.fn<(message: string) => void>() }
  const fiber = { marker: 'boot-animation-root-fiber' as const }
  const settingsConfigure = vi.fn<(config: { auto: boolean }, fiber: unknown) => void>(
    options.configure ?? (() => {}),
  )
  const storageOpen = vi.fn<(spec: typeof bootAnimationDomain) => Promise<Domain<typeof bootAnimationDomain>>>(
    options.open ?? (async () => { throw new Error('storage unavailable') }),
  )

  const record = (target: FakeEffect[]) => (fn: () => unknown, label?: string): (() => void) => {
    const result = fn()
    const effect: FakeEffect = { label, dispose: typeof result === 'function' ? () => { Reflect.apply(result, undefined, []) } : () => {} }
    target.push(effect)
    return () => { effect.dispose() }
  }

  const child: FakeChildContext = {
    logger,
    fiber,
    effect: record(childEffects),
    storageDomain: { open: storageOpen },
    settings: { configure: settingsConfigure },
  }

  const services: FakeContext = {
    logger,
    fiber,
    effect: record(effects),
    inject(names: string[], callback: (injected: FakeChildContext) => unknown): () => void {
      injections.push([...names])
      callback(child)
      return () => {}
    },
    webServer: {
      register(route: WebRoute): () => void {
        routes.push(route)
        const dispose = vi.fn((): void => {})
        routeDisposers.push(dispose)
        return dispose
      },
    },
  }

  const ctx: Context = new Context().extend(services)

  return {
    ctx,
    routes,
    routeDisposers,
    effects,
    childEffects,
    injections,
    logger,
    fiber,
    settingsConfigure,
    storageOpen,
    disposeEffects: () => { for (const effect of effects) effect.dispose() },
    disposeChildEffects: () => { for (const effect of childEffects) effect.dispose() },
    route: (path: string) => {
      const route = routes.find(candidate => candidate.path === path)
      if (route === undefined) throw new Error(`route not registered: ${path}`)
      return route
    },
  }
}

/**
 * Invoke a registered route handler and wait for the response to settle.
 * @param route - the captured route.
 * @param init - request fields plus the admitted peer (`null` omits it).
 * @returns the fake response after `finish` or `close`.
 */
export async function callRoute(
  route: WebRoute,
  init: FakeRequestInit & { peer?: PeerScope | null } = {},
): Promise<FakeResponse> {
  const req = fakeRequest(init)
  const res = new FakeResponse(req)
  const peer = init.peer === undefined ? OWNER : init.peer
  await route.handler(req, res, peer === null ? undefined : peer)
  await responseFinished(res)
  return res
}

/** An in-memory stand-in for the `sessions` table of this package's domain. */
export interface FakeDomain {
  readonly domain: Domain<typeof bootAnimationDomain>
  readonly records: Map<string, SessionIntro>
  readonly get: Mock<(key: string) => SessionIntro | undefined>
  readonly put: Mock<(key: string, value: SessionIntro) => Promise<void>>
  readonly remove: Mock<(key: string) => Promise<boolean>>
  readonly close: Mock<() => Promise<void>>
}

/**
 * Build the in-memory domain.
 * @param records - initial stored overrides, keyed by session id.
 * @param failPut - when true, every write rejects like an unavailable backend.
 * @returns the domain handle plus the table spies.
 */
export function createFakeDomain(records = new Map<string, SessionIntro>(), failPut = false): FakeDomain {
  const get = vi.fn((key: string): SessionIntro | undefined => records.get(key))
  const put = vi.fn(async (key: string, value: SessionIntro): Promise<void> => {
    if (failPut) throw new Error('storage offline')
    records.set(key, value)
  })
  const remove = vi.fn(async (key: string): Promise<boolean> => records.delete(key))
  const close = vi.fn(async (): Promise<void> => {})
  const table: KvTable<string, SessionIntro> = {
    get,
    put,
    delete: remove,
    entries: () => records.entries(),
    keys: () => records.keys(),
    get size(): number {
      return records.size
    },
    deleteMany: async (keys: readonly string[]): Promise<number> => {
      let removed = 0
      for (const key of keys) if (records.delete(key)) removed += 1
      return removed
    },
    update: async (key: string, fn: (current: SessionIntro) => SessionIntro): Promise<SessionIntro> => {
      const current = records.get(key)
      if (current === undefined) throw new Error(`missing key ${key}`)
      const next = fn(current)
      records.set(key, next)
      return next
    },
  }
  const domain = {
    name: bootAnimationDomain.name,
    /** The spec declares no global, so its handle type is `never`: reading it is a defect. */
    get global(): never {
      throw new Error('the boot-animation domain declares no global')
    },
    table<N extends keyof typeof bootAnimationDomain.tables>(
      name: N,
    ): KvTable<TableKeyOf<typeof bootAnimationDomain, N>, TableValueOf<typeof bootAnimationDomain, N>> {
      if (name !== 'sessions') throw new Error(`unexpected table ${name}`)
      // The declared domain has one table, so both key and value mappings are
      // this handle's own types; TypeScript cannot reduce them for a generic N.
      return table as KvTable<
        TableKeyOf<typeof bootAnimationDomain, N>,
        TableValueOf<typeof bootAnimationDomain, N>
      >
    },
    close,
  } satisfies Domain<typeof bootAnimationDomain>
  return { domain, records, get, put, remove, close }
}

/**
 * A `SessionOverrides` backed by {@link createFakeDomain}.
 * @param records - initial stored overrides.
 * @param failPut - when true, writes reject.
 * @returns the overrides store plus its in-memory domain.
 */
export function createOverrides(records = new Map<string, SessionIntro>(), failPut = false): {
  overrides: SessionOverrides
  domain: FakeDomain
} {
  const domain = createFakeDomain(records, failPut)
  return { overrides: new SessionOverrides(domain.domain), domain }
}

/**
 * Let queued microtasks and the storage-open promise settle.
 * @returns resolution on the next macrotask.
 */
export function flush(): Promise<void> {
  return new Promise(resolve => setImmediate(resolve))
}

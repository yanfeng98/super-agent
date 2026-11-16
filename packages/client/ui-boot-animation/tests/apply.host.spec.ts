import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Domain } from '@deepseek-ai/dsh-storage-domain'
import { apply, Config } from '../src/index.ts'
import { bootAnimationDomain } from '../src/host/overrides.ts'
import type { ActiveResponse, CatalogResponse, SessionIntro } from '../src/types.ts'
import {
  callRoute, createFakeContext, createFakeDomain, flush, OWNER, STRANGER,
} from './harness.host.ts'
import type { FakeContextHarness, FakeDomain, FakeRequestInit } from './harness.host.ts'

const ROUTE = '/dsh-boot-animation'
const USER_CLIP_BYTES = 'local-video-bytes'

let roots: string[] = []

afterEach(() => {
  for (const root of roots) rmSync(root, { recursive: true, force: true })
  roots = []
  vi.unstubAllEnvs()
})

/** A temporary directory owned by the current test. */
function tempDir(): string {
  const root = mkdtempSync(join(tmpdir(), 'dsh-boot-animation-apply-'))
  roots.push(root)
  return root
}

/** Options for {@link boot}. */
interface BootOptions {
  /** `DSH_HOME` value; `null` unsets it and `undefined` uses the temporary root. */
  dshHome?: string | null
  /** Storage facility's `open`; defaults to an in-memory domain. */
  open?: (spec: typeof bootAnimationDomain) => Promise<Domain<typeof bootAnimationDomain>>
  selectedClip?: string
  random?: boolean
  /** Makes every durable write reject. */
  failPut?: boolean
  /** Initial stored conversation overrides. */
  records?: Map<string, SessionIntro>
}

/** A booted plugin plus the fixtures its tests inspect. */
interface Booted {
  readonly root: string
  readonly userDir: string
  readonly userClip: string
  readonly harness: FakeContextHarness
  readonly domain: FakeDomain
}

/**
 * Apply the plugin against a fake context over a temporary home.
 * @param options - home, settings, and storage behavior.
 * @returns the booted plugin and its fixtures.
 */
function boot(options: BootOptions = {}): Booted {
  const root = tempDir()
  vi.stubEnv('HOME', root)
  vi.stubEnv('USERPROFILE', root)
  const envHome = options.dshHome === undefined ? root : options.dshHome
  vi.stubEnv('DSH_HOME', envHome === null ? undefined : envHome)
  const storageRoot = envHome === null || envHome === undefined || envHome.trim() === ''
    ? join(root, '.dsh')
    : envHome.trim()
  const userDir = join(storageRoot, 'boot-animation', 'videos')
  mkdirSync(userDir, { recursive: true })
  const userClip = join(userDir, 'local.mp4')
  writeFileSync(userClip, USER_CLIP_BYTES)

  const domain = createFakeDomain(options.records ?? new Map<string, SessionIntro>(), options.failPut ?? false)
  const harness = createFakeContext({ open: options.open ?? (async () => domain.domain) })
  apply(
    harness.ctx,
    Config({ random: options.random ?? false, selectedClip: options.selectedClip ?? 'builtin:brand' }),
  )
  return { root, userDir, userClip, harness, domain }
}

/**
 * Read `catalog.json`.
 * @param harness - the booted plugin.
 * @param init - request target and peer.
 * @returns the response and decoded body.
 */
async function catalog(
  harness: FakeContextHarness,
  init: FakeRequestInit & { peer?: typeof OWNER | null } = {},
): Promise<{ res: Awaited<ReturnType<typeof callRoute>>; body: CatalogResponse }> {
  const res = await callRoute(harness.route(`${ROUTE}/catalog.json`), {
    url: `${ROUTE}/catalog.json`,
    ...init,
  })
  return { res, body: JSON.parse(res.text) as CatalogResponse }
}

/**
 * Read `active.json`.
 * @param harness - the booted plugin.
 * @param query - query string including its leading `?`.
 * @param init - request overrides.
 * @returns the response and decoded body.
 */
async function active(
  harness: FakeContextHarness,
  query = '',
  init: FakeRequestInit & { peer?: typeof OWNER | null } = {},
): Promise<{ res: Awaited<ReturnType<typeof callRoute>>; body: ActiveResponse }> {
  const res = await callRoute(harness.route(`${ROUTE}/active.json`), {
    url: `${ROUTE}/active.json${query}`,
    ...init,
  })
  return { res, body: JSON.parse(res.text) as ActiveResponse }
}

/**
 * Read one clip through the media route.
 * @param harness - the booted plugin.
 * @param target - request path, already encoded.
 * @param init - request overrides.
 * @returns the fake response.
 */
async function media(
  harness: FakeContextHarness,
  target: string,
  init: FakeRequestInit & { peer?: typeof OWNER | null } = {},
): Promise<Awaited<ReturnType<typeof callRoute>>> {
  return callRoute(harness.route(`${ROUTE}/media`), { url: `${ROUTE}/media${target}`, ...init })
}

/**
 * Post a conversation override.
 * @param harness - the booted plugin.
 * @param init - method, body, and request overrides.
 * @returns the fake response.
 */
async function postSession(
  harness: FakeContextHarness,
  init: FakeRequestInit = {},
): Promise<Awaited<ReturnType<typeof callRoute>>> {
  return callRoute(harness.route(`${ROUTE}/session`), {
    method: 'POST',
    url: `${ROUTE}/session`,
    ...init,
  })
}

/** The catalog's user clip. */
function userClipOf(body: CatalogResponse) {
  return body.clips.find(clip => clip.kind === 'file')
}

/**
 * Read one shipped sequence file through the audio route.
 * @param harness - the booted plugin.
 * @param target - request path after the prefix.
 * @param init - request overrides.
 * @returns the fake response.
 */
async function audio(
  harness: FakeContextHarness,
  target: string,
  init: FakeRequestInit & { peer?: typeof OWNER | null } = {},
): Promise<Awaited<ReturnType<typeof callRoute>>> {
  return callRoute(harness.route(`${ROUTE}/audio`), { url: `${ROUTE}/audio${target}`, ...init })
}

describe('audio route', () => {
  it('streams a shipped file with its media type', async () => {
    const { harness } = boot()
    const res = await audio(harness, '/line-1.mp3')
    expect(res.status).toBe(200)
    expect(res.headers['content-type']).toBe('audio/mpeg')
    expect(Number(res.headers['content-length'])).toBeGreaterThan(0)
  })

  it('answers ranges so a player can seek', async () => {
    const { harness } = boot()
    const res = await audio(harness, '/line-1.mp3', { headers: { range: 'bytes=0-9' } })
    expect(res.status).toBe(206)
    expect(res.headers['content-range']).toMatch(/^bytes 0-9\/\d+$/)
  })

  it('refuses anything outside the shipped name set', async () => {
    const { harness } = boot()
    expect((await audio(harness, '')).status).toBe(404)
    expect((await audio(harness, '/../../package.json')).status).toBe(404)
    expect((await audio(harness, '/line-9.mp3')).status).toBe(404)
  })

  it('refuses a caller that is not the owner', async () => {
    const { harness } = boot()
    expect((await audio(harness, '/line-1.mp3', { peer: STRANGER })).status).toBe(403)
  })
})

describe('apply route registration', () => {
  it('registers every owner-only route and disposes them with the effect', () => {
    const { harness } = boot()

    expect(harness.routes.map(route => ({ kind: route.kind, path: route.path, auth: route.auth }))).toEqual([
      { kind: 'prefix', path: `${ROUTE}/media`, auth: 'required' },
      { kind: 'prefix', path: `${ROUTE}/audio`, auth: 'required' },
      { kind: 'exact', path: `${ROUTE}/catalog.json`, auth: 'required' },
      { kind: 'exact', path: `${ROUTE}/active.json`, auth: 'required' },
      { kind: 'exact', path: `${ROUTE}/session`, auth: 'required' },
    ])

    harness.disposeEffects()
    expect(harness.routeDisposers).toHaveLength(5)
    for (const dispose of harness.routeDisposers) expect(dispose).toHaveBeenCalledTimes(1)
  })

  it('injects the storage and settings services', () => {
    const { harness } = boot()

    expect(harness.injections).toEqual([['storageDomain'], ['settings']])
    expect(harness.settingsConfigure).toHaveBeenCalledWith({ auto: false }, harness.fiber)
  })
})

describe('storage bootstrap', () => {
  it('opens the package\'s domain and reports overrides as available', async () => {
    const { harness } = boot()
    await flush()

    expect(harness.storageOpen).toHaveBeenCalledOnce()
    expect(harness.storageOpen.mock.calls[0]?.[0]).toBe(bootAnimationDomain)
    expect((await catalog(harness)).body.overridesAvailable).toBe(true)
  })

  it('keeps the intro working when the domain cannot be opened', async () => {
    const { harness } = boot({ open: async () => { throw new Error('no backend') } })
    await flush()

    expect(harness.logger.warn).toHaveBeenCalledWith(
      expect.stringContaining('per-conversation overrides are unavailable'),
    )
    expect((await catalog(harness)).body.overridesAvailable).toBe(false)
  })

  it('drops the overrides when its effect is disposed after opening', async () => {
    const { harness, domain } = boot()
    await flush()
    expect((await catalog(harness)).body.overridesAvailable).toBe(true)

    harness.disposeChildEffects()

    expect(domain.close).not.toHaveBeenCalled()
    expect((await catalog(harness)).body.overridesAvailable).toBe(false)
  })

  it('closes a domain that finishes opening after disposal', async () => {
    let resolveOpen: ((domain: Domain<typeof bootAnimationDomain>) => void) | undefined
    const open = (): Promise<Domain<typeof bootAnimationDomain>> => new Promise((resolve) => {
      resolveOpen = resolve
    })
    const { harness, domain } = boot({ open })
    harness.disposeChildEffects()

    resolveOpen?.(domain.domain)
    await flush()

    expect(domain.close).toHaveBeenCalledOnce()
    expect((await catalog(harness)).body.overridesAvailable).toBe(false)
  })
})

describe('catalog.json', () => {
  it('refuses a caller that is not the owner', async () => {
    const { harness } = boot()
    for (const peer of [STRANGER, null]) {
      const res = await callRoute(harness.route(`${ROUTE}/catalog.json`), {
        url: `${ROUTE}/catalog.json`,
        peer,
      })
      expect(res.status).toBe(403)
      expect(res.text).toBe('forbidden')
      expect(res.headers['cache-control']).toBe('no-store')
    }
  })

  it('lists the shipped clips before the user\'s own files', async () => {
    const { harness } = boot()
    await flush()

    const { res, body } = await catalog(harness)
    expect(res.status).toBe(200)
    expect(res.headers).toMatchObject({
      'content-type': 'application/json; charset=utf-8',
      'cache-control': 'no-store',
    })
    expect(body.clips.map(clip => clip.id)).toEqual([
      'builtin:brand',
      'builtin:awakening',
      'builtin:cyberpunk',
      'builtin:startup',
      expect.stringMatching(/^file:local-[0-9a-f]{8}$/),
    ])
    expect(body.clips[0]).toMatchObject({ name: null, kind: 'builtin' })
    expect(userClipOf(body)).toMatchObject({ name: 'local', kind: 'file', bytes: USER_CLIP_BYTES.length })
    for (const clip of body.clips) expect(clip.version).not.toBe('')
    expect(body.session).toEqual({ replay: false, clipId: null })
    expect(body.overridesAvailable).toBe(true)
  })

  it('answers the asking conversation\'s stored override', async () => {
    const records = new Map<string, SessionIntro>([
      ['session-1', { replay: true, clipId: 'builtin:startup' }],
    ])
    const { harness } = boot({ records })
    await flush()

    expect((await catalog(harness, { url: `${ROUTE}/catalog.json?session=session-1` })).body.session)
      .toEqual({ replay: true, clipId: 'builtin:startup' })
  })

  it('treats an unusable session id as the global question', async () => {
    const { harness } = boot()
    await flush()

    expect((await catalog(harness, { url: `${ROUTE}/catalog.json?session=${'x'.repeat(300)}` })).body.session)
      .toEqual({ replay: false, clipId: null })
  })

  it('answers the global question when the request target is absent', async () => {
    const { harness } = boot()
    await flush()

    const { body } = await catalog(harness, { url: null })
    expect(body.session).toEqual({ replay: false, clipId: null })
  })
})

describe('active.json', () => {
  it('refuses a caller that is not the owner', async () => {
    const { harness } = boot()
    const res = await callRoute(harness.route(`${ROUTE}/active.json`), {
      url: `${ROUTE}/active.json`,
      peer: STRANGER,
    })
    expect(res.status).toBe(403)
    expect(res.text).toBe('forbidden')
  })

  it('answers the selected clip and its version', async () => {
    const { harness } = boot({ selectedClip: 'builtin:awakening' })
    const { body: clips } = await catalog(harness)
    const expected = clips.clips.find(clip => clip.id === 'builtin:awakening')

    const { res, body } = await active(harness)
    expect(res.status).toBe(200)
    expect(body).toEqual({
      stages: ['archive', 'video'],
      clipId: 'builtin:awakening',
      version: expected?.version,
      how: 'selected',
    })
  })

  it('falls back to the first clip when nothing is selected', async () => {
    const { harness } = boot({ selectedClip: 'builtin:gone' })
    const { body } = await active(harness)

    expect(body).toMatchObject({ clipId: 'builtin:brand', how: 'none' })
  })

  it('picks a random clip in random mode', async () => {
    const { harness } = boot()
    const { body } = await active(harness, '?mode=random')

    expect(body.how).toBe('random')
    expect(body.clipId).not.toBeNull()
    expect(body.version).not.toBeNull()
  })

  it('picks a random clip when random playback is enabled', async () => {
    const { harness } = boot({ random: true })
    const { body } = await active(harness)

    expect(body.how).toBe('random')
  })

  it('answers the conversation override before random playback', async () => {
    const records = new Map<string, SessionIntro>([
      ['session-1', { replay: false, clipId: 'builtin:startup' }],
    ])
    const { harness } = boot({ records, random: true })
    await flush()

    const { body } = await active(harness, '?session=session-1')
    expect(body).toMatchObject({ clipId: 'builtin:startup', how: 'session' })
  })
})

describe('media route', () => {
  it('refuses a caller that is not the owner', async () => {
    const { harness } = boot()
    const res = await media(harness, '/anything.mp4', { peer: STRANGER })
    expect(res.status).toBe(403)
    expect(res.text).toBe('forbidden')
  })

  it('answers 404 for a target outside the media prefix and for unknown or malformed ids', async () => {
    const { harness } = boot()
    const outside = await callRoute(harness.route(`${ROUTE}/media`), { url: `${ROUTE}/media` })
    const unknown = await media(harness, '/file:missing-00000000')
    const malformed = await callRoute(harness.route(`${ROUTE}/media`), {
      url: `${ROUTE}/media/%E0%A4%A`,
    })
    const absent = await callRoute(harness.route(`${ROUTE}/media`), { url: null })

    for (const res of [outside, unknown, malformed, absent]) {
      expect(res.status).toBe(404)
      expect(res.text).toBe('dsh-boot-animation: no such clip')
      expect(res.headers['cache-control']).toBe('no-store')
    }
  })

  it('streams the user\'s clip with an immutable cache when the version is pinned', async () => {
    const { harness } = boot()
    await flush()
    const clip = userClipOf((await catalog(harness)).body)
    if (clip === undefined) throw new Error('fixture must list the user clip')

    const res = await media(harness, `/${encodeURIComponent(clip.id)}?v=${encodeURIComponent(clip.version)}`)

    expect(res.status).toBe(200)
    expect(res.headers).toMatchObject({
      'content-type': 'video/mp4',
      'content-length': String(USER_CLIP_BYTES.length),
      'cache-control': 'public, max-age=31536000, immutable',
    })
    expect(res.text).toBe(USER_CLIP_BYTES)
  })
})

describe('session route', () => {
  it('refuses a caller that is not the owner before checking the method', async () => {
    const { harness } = boot()
    const res = await callRoute(harness.route(`${ROUTE}/session`), {
      url: `${ROUTE}/session`,
      peer: null,
    })
    expect(res.status).toBe(403)
  })

  it('requires POST', async () => {
    const { harness } = boot()
    const res = await callRoute(harness.route(`${ROUTE}/session`), { url: `${ROUTE}/session` })

    expect(res.status).toBe(405)
    expect(res.text).toBe('POST required')
    expect(res.headers['cache-control']).toBe('no-store')
  })

  it('rejects bodies that are not a JSON object', async () => {
    const { harness } = boot()
    await flush()

    for (const body of ['not json', '', '[]', 'null', '42']) {
      const res = await postSession(harness, { body })
      expect(res.status).toBe(400)
      expect(res.text).toBe('expected a JSON object body')
    }
  })

  it('rejects oversize bodies and request failures without waiting for an end', async () => {
    const { harness } = boot()
    await flush()

    const oversized = await postSession(harness, { body: 'x'.repeat(4_097) })
    expect(oversized.status).toBe(400)

    const chunked = await postSession(harness, { chunks: ['x'.repeat(4_097), 'late'] })
    expect(chunked.status).toBe(400)

    const failed = await postSession(harness, { error: new Error('connection reset') })
    expect(failed.status).toBe(400)
  })

  it('rejects a body without a usable session id', async () => {
    const { harness } = boot()
    await flush()

    for (const body of ['{}', '{"sessionId":""}', '{"sessionId":"bad\\u0000id"}', '{"sessionId":42}']) {
      const res = await postSession(harness, { body })
      expect(res.status).toBe(400)
      expect(res.text).toBe('expected a sessionId')
    }
  })

  it('reports unavailable storage instead of accepting the write', async () => {
    const { harness } = boot({ open: async () => { throw new Error('no backend') } })
    await flush()

    const res = await postSession(harness, { body: '{"sessionId":"session-1","replay":true}' })
    expect(res.status).toBe(503)
    expect(res.text).toBe('conversation overrides are unavailable')
  })

  it('rejects an unknown or non-string clip id', async () => {
    const { harness } = boot()
    await flush()

    const unknown = await postSession(harness, {
      body: '{"sessionId":"session-1","clipId":"file:missing-00000000"}',
    })
    expect(unknown.status).toBe(404)
    expect(unknown.text).toBe('no such clip')

    const notString = await postSession(harness, {
      body: '{"sessionId":"session-1","clipId":42}',
    })
    expect(notString.status).toBe(404)
  })

  it('rejects a non-boolean replay flag', async () => {
    const { harness } = boot()
    await flush()

    const res = await postSession(harness, {
      body: '{"sessionId":"session-1","replay":"yes"}',
    })
    expect(res.status).toBe(400)
    expect(res.text).toBe('replay must be a boolean')
  })

  it('stores, merges, and clears the conversation override', async () => {
    const { harness } = boot()
    await flush()
    const clip = userClipOf((await catalog(harness)).body)
    if (clip === undefined) throw new Error('fixture must list the user clip')

    const stored = await postSession(harness, {
      body: JSON.stringify({ sessionId: 'session-1', replay: true, clipId: clip.id }),
    })
    expect(stored.status).toBe(200)
    expect(JSON.parse(stored.text)).toEqual({ replay: true, clipId: clip.id })
    expect((await catalog(harness, { url: `${ROUTE}/catalog.json?session=session-1` })).body.session)
      .toEqual({ replay: true, clipId: clip.id })

    const merged = await postSession(harness, {
      body: JSON.stringify({ sessionId: 'session-1', clipId: null }),
    })
    expect(JSON.parse(merged.text)).toEqual({ replay: true, clipId: null })

    const cleared = await postSession(harness, {
      body: JSON.stringify({ sessionId: 'session-1', replay: false }),
    })
    expect(cleared.status).toBe(200)
    expect(JSON.parse(cleared.text)).toEqual({ replay: false, clipId: null })
    expect((await catalog(harness, { url: `${ROUTE}/catalog.json?session=session-1` })).body.session)
      .toEqual({ replay: false, clipId: null })
  })

  it('accepts a body that sets nothing and leaves the conversation untouched', async () => {
    const { harness } = boot()
    await flush()

    const res = await postSession(harness, { body: '{"sessionId":"session-2"}' })
    expect(res.status).toBe(200)
    expect(JSON.parse(res.text)).toEqual({ replay: false, clipId: null })
  })

  it('reports a failed durable write', async () => {
    const { harness } = boot({ failPut: true })
    await flush()

    const res = await postSession(harness, {
      body: '{"sessionId":"session-1","replay":true}',
    })
    expect(res.status).toBe(500)
    expect(res.text).toBe('could not store the conversation override')
    expect(harness.logger.warn).toHaveBeenCalledWith(
      expect.stringContaining('could not store the conversation override'),
    )
  })
})

describe('user clip directory', () => {
  it('falls back to ~/.dsh when DSH_HOME is empty', async () => {
    const { harness, root } = boot({ dshHome: '   ' })
    await flush()

    const { body } = await catalog(harness)
    expect(userClipOf(body)?.name).toBe('local')
    expect(root).toBeTruthy()
  })

  it('falls back to ~/.dsh when DSH_HOME is unset', async () => {
    const { harness } = boot({ dshHome: null })
    await flush()

    const { body } = await catalog(harness)
    expect(userClipOf(body)?.name).toBe('local')
  })

  it('keeps serving the shipped clips when the user directory cannot be created', async () => {
    const root = tempDir()
    writeFileSync(join(root, 'blocked'), 'a file, not a directory')
    vi.stubEnv('HOME', root)
    vi.stubEnv('USERPROFILE', root)
    vi.stubEnv('DSH_HOME', join(root, 'blocked', 'nested'))
    const harness = createFakeContext()
    apply(harness.ctx, Config({}))

    expect(harness.logger.warn).toHaveBeenCalledWith(
      expect.stringContaining('user clip directory is unavailable'),
    )
    const { body } = await catalog(harness)
    expect(body.clips.map(clip => clip.id)).toEqual([
      'builtin:brand', 'builtin:awakening', 'builtin:cyberpunk', 'builtin:startup',
    ])
  })
})

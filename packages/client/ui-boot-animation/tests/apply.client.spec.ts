// @vitest-environment jsdom
/**
 * The plugin body: what it registers into both seats, when it decides to play,
 * how it reports a failed catalog, how the settings and conversation faces
 * reach the Host, and that teardown aborts requests and removes every effect.
 *
 * The slot, locale, settings-projection, and conversation services are recorders
 * because what matters here is what was handed to them; `decidePlay`, the
 * sources, and the HTTP narrowing have their own suites.
 */
import { Context } from '@deepseek-ai/cordis'
import { SlotRegistry } from '@deepseek-ai/dsh-client-ui-renderer/client'
import { resolveSlotLabel } from '@deepseek-ai/dsh-client-ui-slots'
import type { ComposedProps, SlotComponent } from '@deepseek-ai/dsh-client-ui-slots'
import type { ConfigForm } from '@deepseek-ai/dsh-client-ui-settings/client'
import { stubConfigForm } from '@deepseek-ai/dsh-client-test-runtime'
import { afterEach, beforeEach, describe, expect, it, onTestFinished, vi } from 'vitest'
import { apply, inject } from '../src/client/index.ts'
import { DEFAULT_SETTINGS, NS } from '../src/client/contract.ts'
import type {
  IntroSettings, OverlayInjected, PlaybackState, SettingsInjected,
} from '../src/client/contract.ts'
import { de, en, es, fr, ja, pt, ru, zh } from '../src/client/locales.ts'
import type { ClipInfo } from '../src/types.ts'

/**
 * The root seat's component. These tests never mount the shell; the seat exists
 * so the declarations below have a parent.
 */
const RootSeat: SlotComponent<
  ComposedProps<'root', string, 'shell.overlay' | 'settings.section', undefined, object>
> = () => null

const IDLE: PlaybackState = {
  phase: 'idle', stage: 'none', clipId: null, url: null, reason: 'none', nonce: 0, preview: false,
}

const BRAND: ClipInfo = { id: 'builtin:brand', name: null, kind: 'builtin', bytes: 10, version: 'v1' }
const MINE: ClipInfo = { id: 'file:mine', name: 'My clip', kind: 'file', bytes: 20, version: 'v2' }

const LAUNCH_KEY = 'dsh-boot-animation:launch'
const SESSION_PREFIX = 'dsh-boot-animation:session:'

/** The conversation binding shape this plugin reads from `uiSession.adapter.current`. */
interface Binding {
  key?: unknown
  hooks?: { session?: SessionFace }
}

interface SessionFace {
  getSnapshot?: () => unknown
  subscribe?: (listener: () => void) => unknown
}

/** The observable `uiSession.adapter.current` publishes. */
interface BindingSource {
  getSnapshot: () => Binding | null
  subscribe: (listener: () => void) => () => void
}

/** Locale service members the plugin body calls. */
interface FakeLocale {
  register(ns: string, dicts: Record<string, Record<string, string>>): () => void
  bind(ns: string): (key: string) => string
}

/** Config-form service members the plugin body calls; it reads only its own namespace. */
interface FakeConfigForms {
  get(entryId: string): ConfigForm<IntroSettings>
}

/** Session service members the plugin body calls. */
interface FakeUiSession {
  readonly adapter: { readonly current: BindingSource }
}

type Seat = 'shell.overlay' | 'settings.section'

type Handler = (url: string, init: RequestInit) => unknown

interface BenchOptions {
  /** Publish the settings projection during apply; false models a late mirror. */
  readonly projectSettings?: boolean
  readonly binding?: Binding | null
  readonly settings?: IntroSettings
  readonly catalog?: Handler
  readonly active?: Handler
  readonly write?: Handler
}

/** A minimal snapshot observable, the shape `uiSession.adapter.current` publishes. */
function observable<T>(initial: T) {
  let value = initial
  const listeners = new Set<() => void>()
  return {
    getSnapshot: () => value,
    subscribe: (listener: () => void) => {
      listeners.add(listener)
      return () => { listeners.delete(listener) }
    },
    set: (next: T) => {
      value = next
      for (const listener of [...listeners]) listener()
    },
    listenerCount: () => listeners.size,
  }
}

/** One mocked HTTP answer, as far as the API reads it. */
function answer(body: unknown, init: { ok?: boolean; status?: number } = {}) {
  return { ok: init.ok ?? true, status: init.status ?? 200, json: async () => body }
}

/** A catalog answer with the given clips and conversation override. */
function catalogBody(
  clips: readonly ClipInfo[] = [BRAND, MINE],
  session: { replay: boolean; clipId: string | null } = { replay: false, clipId: null },
  overridesAvailable = true,
) {
  return answer({ clips, session, overridesAvailable })
}

/** A conversation face whose snapshot and listeners the test can drive. */
function sessionFace(initial: unknown) {
  let snapshot = initial
  const listeners = new Set<() => void>()
  return {
    getSnapshot: () => snapshot,
    subscribe: (listener: () => void) => {
      listeners.add(listener)
      return () => { listeners.delete(listener) }
    },
    publish: (next: unknown) => {
      snapshot = next
      for (const listener of [...listeners]) listener()
    },
    listenerCount: () => listeners.size,
  }
}

/** Let every pending fetch answer and its continuations run. */
async function flush(): Promise<void> {
  await new Promise<void>((resolve) => { setTimeout(resolve, 0) })
}

/**
 * Boot the plugin body over recorder services and a routed fetch double.
 * @param options - scripted binding, settings, and HTTP answers.
 * @returns the context, the recorded services, the registrations, and the faces.
 */
async function bench(options: BenchOptions = {}) {
  const ctx = new Context()
  await ctx.plugin(SlotRegistry).await()
  const slots = ctx.slots
  slots.register({
    name: 'root',
    children: {
      'shell.overlay': { kind: 'list', scope: 'root' },
      'settings.section': { kind: 'list', scope: 'root' },
    },
  }, RootSeat)
  const dictionaries: Record<string, Record<string, Record<string, string>>> = {}
  const locale = {
    register: vi.fn((ns: string, dicts: Record<string, Record<string, string>>) => {
      dictionaries[ns] = dicts
      return () => { Reflect.deleteProperty(dictionaries, ns) }
    }),
    bind: vi.fn(() => (key: string) => key),
  } satisfies FakeLocale
  const form = stubConfigForm<IntroSettings>()
  if (options.projectSettings !== false) form.publish({ value: options.settings ?? DEFAULT_SETTINGS })
  const configForms = { get: vi.fn((_entryId: string) => form.scope) } satisfies FakeConfigForms
  const current = observable<Binding | null>(options.binding ?? null)
  const uiSession = { adapter: { current } } satisfies FakeUiSession

  const calls: { url: string; init: RequestInit }[] = []
  const catalogHandler: Handler = options.catalog ?? (() => catalogBody())
  const activeHandler: Handler = options.active ?? (() => answer({ clipId: BRAND.id, version: BRAND.version, how: 'none' }))
  const writeHandler: Handler = options.write ?? (() => answer({ replay: true, clipId: null }))
  const fetchMock = vi.fn(async (input: unknown, init?: RequestInit) => {
    const url = String(input)
    calls.push({ url, init: init ?? {} })
    if (url.includes('/catalog.json')) return await catalogHandler(url, init ?? {})
    if (url.includes('/active.json')) return await activeHandler(url, init ?? {})
    return await writeHandler(url, init ?? {})
  })
  vi.stubGlobal('fetch', fetchMock)

  // These services are classes with private state, so a recorder cannot satisfy
  // the typed arm; the untyped arm installs the same scope entry the plugin
  // resolves through `ctx.locale`/`ctx.configForms`/`ctx.uiSession`.
  ctx.provide('locale', locale)
  ctx.provide('configForms', configForms)
  ctx.provide('uiSession', uiSession)

  const fiber = ctx.plugin({ inject: [...inject], apply })
  await fiber.await()

  let disposed = false
  const dispose = async (): Promise<void> => {
    if (disposed) return
    disposed = true
    await fiber.dispose()
  }
  onTestFinished(dispose)

  const entriesOf = () => [
    ...slots.entries('shell.overlay'),
    ...slots.entries('settings.section'),
  ]
  const entry = (name: Seat) => {
    const found = slots.entries(name)[0]
    if (found === undefined) throw new Error(`expected a registration into ${name}`)
    return found
  }
  const face = (name: Seat): unknown => {
    const injected = entry(name).inject
    if (injected === undefined) throw new Error(`expected ${name} to inject a face`)
    return injected()
  }
  const overlay = (): OverlayInjected => face('shell.overlay') as OverlayInjected
  const settings = (): SettingsInjected => face('settings.section') as SettingsInjected
  const lastBody = (): unknown => {
    const body = calls.at(-1)?.init.body
    if (typeof body !== 'string') throw new Error('expected the request to carry a JSON string body')
    return JSON.parse(body)
  }
  const activeCalls = (): number => calls.filter(call => call.url.includes('/active.json')).length
  const catalogCalls = (): number => calls.filter(call => call.url.includes('/catalog.json')).length
  const writeCalls = (): number => calls.filter(call => call.url.endsWith('/session')).length

  return {
    ctx, slots, locale, dictionaries, form, configForms, current, calls,
    entriesOf, fiber, dispose, entry, settings, overlay, lastBody,
    activeCalls, catalogCalls, writeCalls,
  }
}

/** The playback snapshot the overlay binds. */
function playbackOf(h: Awaited<ReturnType<typeof bench>>): PlaybackState {
  return h.overlay().hooks.playback.getSnapshot()
}

/** The catalog snapshot the settings face binds. */
function catalogOf(h: Awaited<ReturnType<typeof bench>>) {
  return h.settings().hooks.catalog.getSnapshot()
}

beforeEach(() => { sessionStorage.clear() })
afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
  sessionStorage.clear()
})

describe('boot-animation apply', () => {
  it('declares the services it binds and registers them', () => {
    expect(inject).toEqual(['slots', 'locale', 'uiSession', 'configForms'])
  })

  it('registers all eight dictionaries and both seats, and releases everything on teardown', async () => {
    const h = await bench({ binding: { key: 's1' } })
    await vi.waitFor(() => { expect(catalogOf(h).loading).toBe(false) })

    expect(h.locale.register).toHaveBeenCalledExactlyOnceWith(NS, { zh, en, ja, es, fr, de, ru, pt })
    expect(h.entriesOf().map(entry => entry.options.id)).toEqual(['boot-animation', 'boot-animation'])

    const overlay = h.entry('shell.overlay')
    expect(overlay.options).toMatchObject({ id: 'boot-animation', order: 100 })
    expect(overlay.locale).toBe(NS)
    expect(h.overlay().stop).toEqual(expect.any(Function))
    expect(h.overlay().hooks.settings.getSnapshot()).toEqual(DEFAULT_SETTINGS)

    const settingsEntry = h.entry('settings.section')
    expect(settingsEntry.options).toMatchObject({ id: 'boot-animation', order: 40 })
    expect(settingsEntry.locale).toBe(NS)
    expect(resolveSlotLabel(settingsEntry.options.label)).toBe('section.label')
    expect(h.settings().sessionStorageAvailable).toBe(true)

    // A play through the Host, then the stop control returns to idle.
    await vi.waitFor(() => { expect(playbackOf(h).phase).toBe('loading') })
    h.overlay().stop()
    expect(playbackOf(h)).toEqual(IDLE)

    const signal = h.calls[0]?.init.signal
    expect(signal?.aborted).toBe(false)
    await h.dispose()

    expect(signal?.aborted).toBe(true)
    expect(h.entriesOf()).toEqual([])
    expect(h.dictionaries[NS]).toBeUndefined()
    expect(h.form.listenerCount()).toBe(0)
    expect(h.current.listenerCount()).toBe(0)
  })

  it('keeps the normalized settings snapshot stable between changes', async () => {
    const h = await bench()
    const face = h.overlay().hooks.settings
    const first = face.getSnapshot()
    expect(face.getSnapshot()).toBe(first)
    h.form.publish({ value: { ...DEFAULT_SETTINGS, sequence: 'video-only' } })
    const second = face.getSnapshot()
    expect(second.sequence).toBe('video-only')
    expect(face.getSnapshot()).toBe(second)
    expect(second).not.toBe(first)
  })

  it('uses the defaults until the settings projection answers, then mirrors it', async () => {
    const h = await bench()
    expect(h.overlay().hooks.settings.getSnapshot()).toEqual(DEFAULT_SETTINGS)

    const projected: IntroSettings = {
      ...DEFAULT_SETTINGS, enabled: false, timing: 'entry', fit: 'contain', random: true, selectedClip: 'file:mine',
    }
    h.form.publish({ value: projected })
    expect(h.overlay().hooks.settings.getSnapshot()).toEqual(projected)

    h.form.publish({ value: undefined })
    expect(h.overlay().hooks.settings.getSnapshot()).toEqual(DEFAULT_SETTINGS)
  })

  it('plays the launch intro when a conversation is already current', async () => {
    const h = await bench({
      binding: { key: 's1', hooks: { session: { getSnapshot: () => ({ blank: true }) } } },
      catalog: () => catalogBody(),
      active: () => answer({ clipId: BRAND.id, version: BRAND.version, how: 'none' }),
    })
    await vi.waitFor(() => { expect(playbackOf(h).phase).toBe('loading') })

    expect(playbackOf(h)).toMatchObject({
      phase: 'loading',
      clipId: 'builtin:brand',
      url: '/dsh-boot-animation/media/builtin%3Abrand?v=v1',
      reason: 'app-start',
      preview: false,
    })
    expect(h.calls.map(call => call.url)).toEqual([
      '/dsh-boot-animation/catalog.json?session=s1',
      '/dsh-boot-animation/active.json?mode=active&session=s1',
    ])
    expect(sessionStorage.getItem(LAUNCH_KEY)).toBe('1')
    expect(sessionStorage.getItem(`${SESSION_PREFIX}s1`)).toBe('1')
  })

  it('runs the configured plan stage by stage and then goes idle', async () => {
    const h = await bench({
      active: () => answer({ stages: ['archive', 'video'], clipId: BRAND.id, version: BRAND.version, how: 'selected' }),
    })
    await vi.waitFor(() => { expect(playbackOf(h).stage).toBe('archive') })
    expect(playbackOf(h)).toMatchObject({ phase: 'playing', stage: 'archive', clipId: null, url: null })

    h.overlay().advance()
    expect(playbackOf(h)).toMatchObject({
      phase: 'loading',
      stage: 'video',
      clipId: 'builtin:brand',
      url: '/dsh-boot-animation/media/builtin%3Abrand?v=v1',
      reason: 'selected',
    })

    h.overlay().advance()
    expect(playbackOf(h)).toEqual(IDLE)
  })

  it('starts with the video when the plan says so', async () => {
    const h = await bench({
      active: () => answer({ stages: ['video', 'archive'], clipId: BRAND.id, version: BRAND.version, how: 'none' }),
    })
    await vi.waitFor(() => { expect(playbackOf(h).stage).toBe('video') })
    expect(playbackOf(h).phase).toBe('loading')
  })

  it('drops the video stage when the Host names no clip', async () => {
    const h = await bench({
      active: () => answer({ stages: ['video', 'archive'], clipId: null, version: null, how: 'none' }),
    })
    await vi.waitFor(() => { expect(playbackOf(h).stage).toBe('archive') })
    h.overlay().advance()
    expect(playbackOf(h)).toEqual(IDLE)
  })

  it('plays nothing when the Host names no stage at all', async () => {
    const h = await bench({ active: () => answer({ stages: [], clipId: null, version: null, how: 'none' }) })
    await flush()
    expect(playbackOf(h)).toEqual(IDLE)
    expect(sessionStorage.getItem(LAUNCH_KEY)).toBeNull()
  })

  it('previews the archive sequence without recording a launch', async () => {
    const h = await bench({ active: () => answer({ stages: [], clipId: null, version: null, how: 'none' }) })
    await flush()
    h.settings().previewArchive()
    expect(playbackOf(h)).toMatchObject({ stage: 'archive', preview: true, reason: 'preview' })
    expect(sessionStorage.getItem(LAUNCH_KEY)).toBeNull()
  })

  it('writes the sequence and each screen field through the settings form', async () => {
    const h = await bench()
    await flush()
    h.settings().setSequence('archive-only')
    h.settings().setScreen('screenTheme', 'dark')
    h.settings().setScreen('screenSpeed', 2)
    expect(h.form.set.mock.calls).toEqual([
      ['sequence', 'archive-only'],
      ['screenTheme', 'dark'],
      ['screenSpeed', 2],
    ])
    expect(h.overlay().hooks.settings.getSnapshot()).toMatchObject({
      sequence: 'archive-only', screenTheme: 'dark', screenSpeed: 2,
    })
  })

  it('moves playback to playing only from the loading phase', async () => {
    const h = await bench({
      projectSettings: false,
      catalog: () => catalogBody(),
      active: () => answer({ clipId: BRAND.id, version: BRAND.version, how: 'none' }),
    })
    h.overlay().markPlaying()
    expect(playbackOf(h).phase).toBe('idle')
    h.form.publish({ value: DEFAULT_SETTINGS })
    await vi.waitFor(() => { expect(playbackOf(h).phase).toBe('loading') })
    h.overlay().markPlaying()
    expect(playbackOf(h).phase).toBe('playing')
    h.overlay().markPlaying()
    expect(playbackOf(h).phase).toBe('playing')
    const listener = vi.fn()
    const stopListening = h.overlay().hooks.settings.subscribe(listener)
    h.form.publish({ value: { ...DEFAULT_SETTINGS, enabled: false } })
    expect(listener).toHaveBeenCalled()
    stopListening()
  })

  it('writes a preference before the projection has ever arrived', async () => {
    const h = await bench({ projectSettings: false })
    h.settings().setEnabled(true)
    await vi.waitFor(() => { expect(h.overlay().hooks.settings.getSnapshot().enabled).toBe(true) })
  })

  it('decides once the projection arrives after apply', async () => {
    const h = await bench({
      projectSettings: false,
      catalog: () => catalogBody(),
      active: () => answer({ clipId: BRAND.id, version: BRAND.version, how: 'none' }),
    })
    expect(playbackOf(h).phase).toBe('idle')
    h.form.publish({ value: DEFAULT_SETTINGS })
    await vi.waitFor(() => { expect(playbackOf(h).phase).toBe('loading') })
    expect(playbackOf(h)).toMatchObject({ clipId: 'builtin:brand', reason: 'app-start' })
  })

  it('keeps quiet while the master switch is off', async () => {
    const h = await bench({
      settings: { ...DEFAULT_SETTINGS, enabled: false },
      binding: { key: 's1' },
    })
    await vi.waitFor(() => { expect(catalogOf(h).loading).toBe(false) })
    expect(h.activeCalls()).toBe(0)
    expect(playbackOf(h)).toEqual(IDLE)
  })

  it('keeps quiet when this launch already played', async () => {
    sessionStorage.setItem(LAUNCH_KEY, '1')
    const h = await bench({ binding: { key: 's1' } })
    await vi.waitFor(() => { expect(catalogOf(h).loading).toBe(false) })
    expect(h.activeCalls()).toBe(0)
  })

  it('waits for a blank conversation the user has not entered yet on entry timing', async () => {
    const h = await bench({
      settings: { ...DEFAULT_SETTINGS, timing: 'entry' },
      binding: { key: 's1', hooks: { session: { getSnapshot: () => ({ blank: false }) } } },
    })
    await vi.waitFor(() => { expect(catalogOf(h).loading).toBe(false) })
    expect(h.activeCalls()).toBe(0)

    // Leaving to no conversation is an entry, but there is no conversation to play for.
    h.current.set(null)
    await flush()
    expect(h.activeCalls()).toBe(0)
    expect(h.catalogCalls()).toBe(2)
  })

  it('plays on each new blank conversation on entry timing, once per conversation', async () => {
    const h = await bench({ settings: { ...DEFAULT_SETTINGS, timing: 'entry' } })
    const blank = (): Binding => ({ key: 's1', hooks: { session: { getSnapshot: () => ({ blank: true }) } } })

    h.current.set(blank())
    await vi.waitFor(() => { expect(playbackOf(h).reason).toBe('entry') })
    expect(h.activeCalls()).toBe(1)

    h.current.set({ key: 's2', hooks: { session: { getSnapshot: () => ({ blank: true }) } } })
    await vi.waitFor(() => { expect(h.activeCalls()).toBe(2) })
    await flush()
    expect(playbackOf(h).phase).toBe('loading')

    // Back to the first conversation: it already played during this launch.
    h.current.set(blank())
    await flush()
    expect(h.activeCalls()).toBe(2)
  })

  it('replays on every entry for a conversation that asked for it', async () => {
    const h = await bench({
      binding: { key: 's1' },
      catalog: () => catalogBody([BRAND], { replay: true, clipId: 'file:mine' }),
      active: () => answer({ clipId: BRAND.id, version: BRAND.version, how: 'selected' }),
    })
    await vi.waitFor(() => { expect(playbackOf(h).reason).toBe('selected') })
    expect(playbackOf(h).url).toBe('/dsh-boot-animation/media/builtin%3Abrand?v=v1')

    // The launch record exists, yet the conversation's request outranks it.
    h.current.set({ key: 's2' })
    await vi.waitFor(() => { expect(h.activeCalls()).toBe(2) })
    await flush()
    expect(playbackOf(h).phase).toBe('loading')
  })

  it('does not record a launch until a clip actually resolves, then retries', async () => {
    const h = await bench({
      binding: { key: 's1' },
      active: () => { throw new Error('offline') },
    })
    await vi.waitFor(() => { expect(h.activeCalls()).toBe(1) })
    await flush()
    expect(playbackOf(h)).toEqual(IDLE)
    expect(sessionStorage.getItem(LAUNCH_KEY)).toBeNull()

    // Four Host answers that name no usable clip, then a resolve for no conversation.
    const base = h.activeCalls()
    const answers: (() => unknown)[] = [
      () => answer(null),
      () => answer({ clipId: null, version: 'v1', how: 'none' }),
      () => answer({ clipId: BRAND.id, version: null, how: 'none' }),
      () => answer({ clipId: BRAND.id, version: BRAND.version, how: 'none' }),
    ]
    for (const [index, next] of answers.entries()) {
      vi.stubGlobal('fetch', vi.fn(async (input: unknown, init?: RequestInit) => {
        const url = String(input)
        h.calls.push({ url, init: init ?? {} })
        if (url.includes('/catalog.json')) return catalogBody()
        if (url.includes('/active.json')) return next()
        return answer({ replay: false, clipId: null })
      }))
      h.current.set({ key: `s${index + 2}` })
      await flush()
      await vi.waitFor(() => { expect(h.activeCalls()).toBe(base + index + 1) })
    }

    // The last answer named no conversation, so only the launch marker is recorded.
    expect(playbackOf(h).url).toBe('/dsh-boot-animation/media/builtin%3Abrand?v=v1')
    expect(sessionStorage.getItem(LAUNCH_KEY)).toBe('1')
    expect(sessionStorage.getItem(`${SESSION_PREFIX}s2`)).toBeNull()

    // The launch marker now suppresses the next entry.
    h.current.set({ key: 's6' })
    await flush()
    expect(h.activeCalls()).toBe(base + 4)
  })

  it('ignores a play already in flight and previews clips only after a catalog arrives', async () => {
    let resolveActive!: (value: unknown) => void
    const h = await bench({
      binding: { key: 's1' },
      catalog: () => catalogBody(),
      active: () => new Promise((resolve) => { resolveActive = resolve }),
    })
    await vi.waitFor(() => { expect(h.activeCalls()).toBe(1) })
    await vi.waitFor(() => { expect(catalogOf(h).clips).toHaveLength(2) })

    // A second entry while the first resolve is unanswered does not stack a request.
    h.current.set({ key: 's2' })
    await flush()
    expect(h.activeCalls()).toBe(1)

    // Nor does a preview: the in-flight play keeps the element to itself.
    h.settings().preview(MINE.id)
    await flush()
    expect(playbackOf(h)).toEqual(IDLE)

    resolveActive(answer({ clipId: BRAND.id, version: BRAND.version, how: 'none' }))
    await vi.waitFor(() => { expect(playbackOf(h).phase).toBe('loading') })
    expect(playbackOf(h)).toMatchObject({ clipId: 'builtin:brand', preview: false })

    // A clip the catalog does not know previews nothing.
    h.settings().preview('file:missing')
    await flush()
    expect(playbackOf(h).preview).toBe(false)

    h.settings().preview(MINE.id)
    expect(playbackOf(h)).toMatchObject({
      phase: 'loading',
      clipId: 'file:mine',
      url: '/dsh-boot-animation/media/file%3Amine?v=v2',
      reason: 'preview',
      preview: true,
      nonce: 2,
    })
    expect(h.activeCalls()).toBe(1)
  })

  it('writes the catalog failure into the catalog and recovers on the next refresh', async () => {
    let failing = true
    const h = await bench({
      catalog: () => {
        if (failing) throw new Error('offline')
        return catalogBody()
      },
    })
    await vi.waitFor(() => { expect(catalogOf(h).loading).toBe(false) })
    expect(catalogOf(h).error).toBe('Error: offline')
    expect(catalogOf(h).clips).toEqual([])

    failing = false
    h.settings().refresh()
    await vi.waitFor(() => { expect(catalogOf(h).clips).toHaveLength(2) })
    expect(catalogOf(h)).toMatchObject({ loading: false, error: null, overridesAvailable: true })
  })

  it('writes each preference through the settings form and keeps the optimistic value', async () => {
    const h = await bench({ settings: { ...DEFAULT_SETTINGS, enabled: false }, binding: { key: 's1' } })
    await flush()

    h.settings().setEnabled(true)
    h.settings().setTiming('entry')
    h.settings().setFit('contain')
    h.settings().setRandom(true)
    h.settings().setSelectedClip('file:mine')

    expect(h.form.set.mock.calls).toEqual([
      ['enabled', true],
      ['timing', 'entry'],
      ['fit', 'contain'],
      ['random', true],
      ['selectedClip', 'file:mine'],
    ])
    expect(h.overlay().hooks.settings.getSnapshot()).toEqual({
      ...DEFAULT_SETTINGS,
      enabled: true, timing: 'entry', fit: 'contain', random: true, selectedClip: 'file:mine',
    })

    // A refused write does not roll the optimistically applied value back.
    h.form.set.mockRejectedValueOnce(new Error('read-only'))
    h.settings().setEnabled(false)
    await flush()
    expect(h.overlay().hooks.settings.getSnapshot().enabled).toBe(false)
  })

  it('writes the conversation override, mirrors the Host answer, and ignores it without a conversation', async () => {
    let stored: unknown = { replay: true, clipId: 'file:mine' }
    const h = await bench({
      settings: { ...DEFAULT_SETTINGS, enabled: false },
      write: () => answer(stored),
    })
    await flush()

    // No conversation: neither write reaches the Host.
    h.settings().setSessionReplay(true)
    h.settings().setSessionClip('file:mine')
    await flush()
    expect(h.writeCalls()).toBe(0)

    h.current.set({ key: 's1' })
    await flush()
    h.settings().setSessionReplay(true)
    expect(h.calls.at(-1)?.url).toBe('/dsh-boot-animation/session')
    expect(h.lastBody()).toEqual({ sessionId: 's1', replay: true })
    await flush()
    await vi.waitFor(() => { expect(catalogOf(h).session).toEqual({ replay: true, clipId: 'file:mine' }) })

    // Clearing the conversation clip keeps the replay flag the Host stored.
    stored = { replay: true, clipId: null }
    h.settings().setSessionClip(null)
    expect(h.lastBody()).toEqual({ sessionId: 's1', clipId: null })
    await flush()
    await vi.waitFor(() => { expect(catalogOf(h).session).toEqual({ replay: true, clipId: null }) })

    // Choosing one keeps the replay flag too.
    stored = { replay: true, clipId: 'builtin:brand' }
    h.settings().setSessionClip('builtin:brand')
    expect(h.lastBody()).toEqual({ sessionId: 's1', clipId: 'builtin:brand' })
    await flush()

    // A failed write leaves the optimistic value in place.
    vi.stubGlobal('fetch', vi.fn(async (input: unknown, _init?: RequestInit) => {
      const url = String(input)
      if (url.includes('/catalog.json')) return catalogBody()
      if (url.includes('/active.json')) return answer({ clipId: BRAND.id, version: BRAND.version, how: 'none' })
      throw new Error('offline')
    }))
    h.settings().setSessionReplay(false)
    await flush()
    expect(catalogOf(h).session.replay).toBe(false)
  })

  it('publishes the conversation identity from the binding and tracks the session face', async () => {
    const faceA = sessionFace({ blank: true })
    const faceB = sessionFace({ blank: false })
    const h = await bench({ settings: { ...DEFAULT_SETTINGS, enabled: false } })

    h.current.set({ key: 's1', hooks: { session: faceA } })
    expect(h.settings().hooks.session.getSnapshot()).toEqual({ sessionId: 's1', blank: true })

    faceA.publish({ blank: false })
    expect(h.settings().hooks.session.getSnapshot()).toEqual({ sessionId: 's1', blank: false })

    // An empty or non-string key names no conversation.
    h.current.set({ key: '', hooks: { session: faceA } })
    expect(h.settings().hooks.session.getSnapshot()).toEqual({ sessionId: null, blank: false })
    h.current.set({ key: 42, hooks: { session: faceA } })
    expect(h.settings().hooks.session.getSnapshot()).toEqual({ sessionId: null, blank: false })

    // A different face replaces the subscription; a snapshot that is not a record reads as not blank.
    h.current.set({ key: 's2', hooks: { session: faceB } })
    expect(faceA.listenerCount()).toBe(0)
    expect(faceB.listenerCount()).toBe(1)
    faceB.publish(null)
    expect(h.settings().hooks.session.getSnapshot()).toEqual({ sessionId: 's2', blank: false })

    // A face without a subscription leaves nothing to release.
    h.current.set({ key: 's3', hooks: { session: { getSnapshot: () => ({ blank: true }) } } })
    expect(faceB.listenerCount()).toBe(0)
    expect(h.settings().hooks.session.getSnapshot()).toEqual({ sessionId: 's3', blank: true })

    // A scalar snapshot and a non-function subscription are both tolerated.
    h.current.set({ key: 's4', hooks: { session: { getSnapshot: () => 'scalar', subscribe: () => 'not a function' } } })
    expect(h.settings().hooks.session.getSnapshot()).toEqual({ sessionId: 's4', blank: false })

    // No binding at all clears the identity.
    h.current.set(null)
    expect(h.settings().hooks.session.getSnapshot()).toEqual({ sessionId: null, blank: false })

    // Publishing the same binding again is not a new entry.
    h.current.set(null)
    expect(h.settings().hooks.session.getSnapshot()).toEqual({ sessionId: null, blank: false })
  })
})

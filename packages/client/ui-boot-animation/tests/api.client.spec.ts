/**
 * The browser face's Host requests: URL composition, response narrowing, and
 * the failures that must degrade to "no clip" or reject rather than leak
 * undefined into a component.
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  audioUrlFor, fetchActive, fetchCatalog, mediaUrlFor, writeSessionOverride,
} from '../src/client/api.ts'

const BASE = '/dsh-boot-animation'
const signal = new AbortController().signal

/** One mocked HTTP answer, as far as the API reads it. */
function answer(body: unknown, init: { ok?: boolean; status?: number } = {}) {
  return { ok: init.ok ?? true, status: init.status ?? 200, json: async () => body }
}

/** Install a fetch double returning one scripted answer. */
function fetchAnswering(body: unknown, init?: { ok?: boolean; status?: number }) {
  const fetchMock = vi.fn(async (_input: string, _init?: RequestInit) => answer(body, init))
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

/** A clip record that passes validation. */
function clip(overrides: Record<string, unknown> = {}) {
  return { id: 'builtin:brand', kind: 'builtin', bytes: 10, version: 'v1', name: null, ...overrides }
}

afterEach(() => { vi.unstubAllGlobals() })

describe('mediaUrlFor', () => {
  it('addresses one clip at one content version, escaping both', () => {
    expect(mediaUrlFor({ id: 'builtin:brand', version: '1' }))
      .toBe(`${BASE}/media/builtin%3Abrand?v=1`)
    expect(mediaUrlFor({ id: 'file:a b/c', version: 'v 1' }))
      .toBe(`${BASE}/media/file%3Aa%20b%2Fc?v=v%201`)
  })
})

describe('fetchCatalog', () => {
  it('asks without a session parameter and narrows the answer', async () => {
    const fetchMock = fetchAnswering({
      clips: [clip(), clip({ id: 'file:one', kind: 'file', name: 'Mine' })],
      session: { replay: true, clipId: 'file:one' },
      overridesAvailable: true,
    })
    const catalog = await fetchCatalog(null, signal)
    expect(catalog).toEqual({
      clips: [
        { id: 'builtin:brand', kind: 'builtin', bytes: 10, version: 'v1', name: null },
        { id: 'file:one', kind: 'file', bytes: 10, version: 'v1', name: 'Mine' },
      ],
      session: { replay: true, clipId: 'file:one' },
      overridesAvailable: true,
    })
    expect(fetchMock).toHaveBeenCalledExactlyOnceWith(`${BASE}/catalog.json`, { cache: 'no-store', signal })
  })

  it('appends the session parameter with an escaped identity', async () => {
    const fetchMock = fetchAnswering({ clips: [] })
    const catalog = await fetchCatalog('s/1', signal)
    expect(fetchMock.mock.calls[0]?.[0]).toBe(`${BASE}/catalog.json?session=s%2F1`)
    // Missing optional fields collapse to the empty catalog defaults.
    expect(catalog).toEqual({
      clips: [], session: { replay: false, clipId: null }, overridesAvailable: false,
    })
  })

  it('rejects a non-2xx answer with its status', async () => {
    fetchAnswering({}, { ok: false, status: 503 })
    await expect(fetchCatalog(null, signal)).rejects.toThrow('catalog request failed with 503')
  })

  it('rejects a body that is not an object', async () => {
    fetchAnswering('not an object')
    await expect(fetchCatalog(null, signal)).rejects.toThrow('catalog response is not an object')
    fetchAnswering(null)
    await expect(fetchCatalog(null, signal)).rejects.toThrow('catalog response is not an object')
    fetchAnswering(42)
    await expect(fetchCatalog(null, signal)).rejects.toThrow('catalog response is not an object')
  })

  it('treats a non-array clip list as empty and filters invalid clip records', async () => {
    const fetchMock = fetchAnswering({
      clips: [
        clip(),
        null,
        'clip',
        { id: 7, kind: 'file', bytes: 1, version: 'v1' },
        clip({ id: '' }),
        clip({ kind: 'remote' }),
        clip({ kind: 'file', id: 'file:no-bytes', bytes: '1' }),
        clip({ bytes: Number.NaN }),
        clip({ bytes: Number.POSITIVE_INFINITY }),
        clip({ version: 1 }),
        clip({ version: '' }),
        clip({ id: 'file:good', kind: 'file', bytes: 0, version: 'v9', name: 'Good' }),
        clip({ id: 'file:anon', kind: 'file', name: 9 }),
        clip({ name: 'Label' }),
      ],
      session: { replay: false, clipId: null },
    })
    const catalog = await fetchCatalog(null, signal)
    expect(fetchMock.mock.calls[0]?.[0]).toBe(`${BASE}/catalog.json`)
    expect(catalog.clips.map(candidate => [candidate.id, candidate.name])).toEqual([
      ['builtin:brand', null],
      ['file:good', 'Good'],
      ['file:anon', null],
      ['builtin:brand', 'Label'],
    ])
  })

  it('treats a non-array clip list as empty when the field is absent', async () => {
    fetchAnswering({ clips: 'nope' })
    await expect(fetchCatalog(null, signal)).resolves.toEqual({
      clips: [], session: { replay: false, clipId: null }, overridesAvailable: false,
    })
  })

  it('narrows the conversation override, including malformed forms', async () => {
    fetchAnswering({ session: 'not a record' })
    await expect(fetchCatalog(null, signal)).resolves.toMatchObject({
      session: { replay: false, clipId: null },
    })

    fetchAnswering({ session: { replay: 'yes', clipId: '' } })
    await expect(fetchCatalog(null, signal)).resolves.toMatchObject({
      session: { replay: false, clipId: null },
    })

    fetchAnswering({ session: { replay: true, clipId: 9 } })
    await expect(fetchCatalog(null, signal)).resolves.toMatchObject({
      session: { replay: true, clipId: null },
    })
  })
})

describe('fetchActive', () => {
  it('asks in the requested mode and keeps the Host reason', async () => {
    const fetchMock = fetchAnswering({ stages: ['video'], clipId: 'builtin:brand', version: 'v2', how: 'session' })
    await expect(fetchActive(null, 'active', signal)).resolves.toEqual({
      stages: ['video'], clipId: 'builtin:brand', version: 'v2', how: 'session',
    })
    expect(fetchMock.mock.calls[0]).toEqual([
      `${BASE}/active.json?mode=active`,
      { cache: 'no-store', signal },
    ])

    fetchAnswering({ clipId: 'file:x', version: 'v1', how: 'random' })
    await expect(fetchActive(null, 'random', signal)).resolves.toMatchObject({ how: 'random' })

    fetchAnswering({ clipId: 'file:x', version: 'v1', how: 'selected' })
    await expect(fetchActive(null, 'active', signal)).resolves.toMatchObject({ how: 'selected' })

    fetchAnswering({ clipId: 'file:x', version: 'v1', how: 'other' })
    await expect(fetchActive(null, 'active', signal)).resolves.toMatchObject({ how: 'none' })
  })

  it('reads the stage plan, ignoring stages it does not know', async () => {
    fetchAnswering({ stages: ['archive', 'nope', 'video'], clipId: 'builtin:brand', version: 'v2', how: 'none' })
    await expect(fetchActive(null, 'active', signal)).resolves.toMatchObject({ stages: ['archive', 'video'] })
    // An older Host sends no plan: the clip stage is what it could play.
    fetchAnswering({ clipId: 'builtin:brand', version: 'v2', how: 'none' })
    await expect(fetchActive(null, 'active', signal)).resolves.toMatchObject({ stages: ['video'] })
    fetchAnswering({ stages: [], clipId: 'builtin:brand', version: 'v2', how: 'none' })
    await expect(fetchActive(null, 'active', signal)).resolves.toBeNull()
  })

  it('keeps an incomplete clip out of the plan instead of failing the request', async () => {
    fetchAnswering({ stages: ['archive'], clipId: null, version: null, how: 'none' })
    await expect(fetchActive(null, 'active', signal)).resolves.toEqual({
      stages: ['archive'], clipId: null, version: null, how: 'none',
    })
    fetchAnswering({ stages: ['video'], clipId: '', version: '', how: 'none' })
    await expect(fetchActive(null, 'active', signal)).resolves.toMatchObject({ clipId: null, version: null })
    fetchAnswering({ stages: ['video'], clipId: 1, version: 2, how: 'none' })
    await expect(fetchActive(null, 'active', signal)).resolves.toMatchObject({ clipId: null, version: null })
  })

  it('answers null when the answer is not an object at all', async () => {
    fetchAnswering(null)
    await expect(fetchActive(null, 'active', signal)).resolves.toBeNull()
    fetchAnswering('nope')
    await expect(fetchActive(null, 'active', signal)).resolves.toBeNull()
  })

  it('appends the session parameter with the right separator', async () => {
    const fetchMock = fetchAnswering({ clipId: 'file:x', version: 'v1', how: 'none' })
    await fetchActive('s 1', 'random', signal)
    expect(fetchMock.mock.calls[0]?.[0]).toBe(`${BASE}/active.json?mode=random&session=s%201`)
  })

  it('rejects a non-2xx answer with its status', async () => {
    fetchAnswering({}, { ok: false, status: 404 })
    await expect(fetchActive(null, 'active', signal)).rejects.toThrow('active request failed with 404')
  })
})

describe('audioUrlFor', () => {
  it('addresses one shipped file and escapes its name', () => {
    expect(audioUrlFor('line-1')).toBe(`${BASE}/audio/line-1.mp3`)
    expect(audioUrlFor('a b/c')).toBe(`${BASE}/audio/a%20b%2Fc.mp3`)
  })
})

describe('writeSessionOverride', () => {
  it('posts the conversation and patch, and narrows the stored override', async () => {
    const fetchMock = fetchAnswering({ replay: true, clipId: 'builtin:brand' })
    await expect(writeSessionOverride('s 1', { replay: true, clipId: 'builtin:brand' }, signal))
      .resolves.toEqual({ replay: true, clipId: 'builtin:brand' })
    expect(fetchMock).toHaveBeenCalledExactlyOnceWith(`${BASE}/session`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ sessionId: 's 1', replay: true, clipId: 'builtin:brand' }),
      cache: 'no-store',
      signal,
    })
  })

  it('treats a malformed stored answer as no override', async () => {
    fetchAnswering(null)
    await expect(writeSessionOverride('s1', { replay: true }, signal))
      .resolves.toEqual({ replay: false, clipId: null })
    fetchAnswering({ replay: false, clipId: 7 })
    await expect(writeSessionOverride('s1', { clipId: null }, signal))
      .resolves.toEqual({ replay: false, clipId: null })
  })

  it('rejects a non-2xx answer with its status', async () => {
    fetchAnswering({}, { ok: false, status: 500 })
    await expect(writeSessionOverride('s1', { replay: true }, signal))
      .rejects.toThrow('session write failed with 500')
  })
})

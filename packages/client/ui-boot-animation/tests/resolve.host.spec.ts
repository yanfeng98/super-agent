import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ClipCatalog } from '../src/host/catalog.ts'
import type { DiscoveredClip } from '../src/host/catalog.ts'
import { IntroResolver, pickRandom } from '../src/host/resolve.ts'
import type { SessionOverrides } from '../src/host/overrides.ts'
import type { SessionIntro } from '../src/types.ts'
import { createOverrides } from './harness.host.ts'

let roots: string[] = []

afterEach(() => {
  for (const root of roots) rmSync(root, { recursive: true, force: true })
  roots = []
  vi.restoreAllMocks()
})

/** A temporary directory owned by the current test. */
function tempDir(): string {
  const root = mkdtempSync(join(tmpdir(), 'dsh-boot-animation-resolve-'))
  roots.push(root)
  return root
}

/**
 * Describe a clip without touching the filesystem.
 * @param id - clip id.
 * @returns the clip record.
 */
function clip(id: string): DiscoveredClip {
  return { info: { id, name: null, kind: 'builtin', bytes: 1, version: `v-${id}` }, path: `/clips/${id}.mp4` }
}

/** A user directory holding three playable clips. */
function catalogWithThreeClips(): ClipCatalog {
  const root = tempDir()
  mkdirSync(root, { recursive: true })
  writeFileSync(join(root, 'a.mp4'), 'a')
  writeFileSync(join(root, 'b.mp4'), 'b')
  writeFileSync(join(root, 'c.mp4'), 'c')
  return new ClipCatalog([], [root])
}

/** A catalog with no playable clip. */
function emptyCatalog(): ClipCatalog {
  return new ClipCatalog([], [])
}

/**
 * Build a resolver over fixed settings.
 * @param catalog - clip discovery.
 * @param options - overrides store and the settings snapshot.
 * @returns the resolver.
 */
function resolverFor(
  catalog: ClipCatalog,
  options: { overrides?: SessionOverrides | null; random?: boolean; selectedClip?: string } = {},
): IntroResolver {
  return new IntroResolver(
    catalog,
    () => options.overrides ?? null,
    () => ({ random: options.random ?? false, selectedClip: options.selectedClip ?? 'builtin:brand' }),
  )
}

/**
 * Store one override for a conversation.
 * @param record - the override to store.
 * @param sessionId - the conversation id.
 * @returns the overrides store.
 */
function overridesFor(record: SessionIntro, sessionId = 's1'): SessionOverrides {
  const { overrides } = createOverrides(new Map([[sessionId, record]]))
  return overrides
}

describe('pickRandom', () => {
  it('returns null for an empty pool', () => {
    expect(pickRandom([], null, () => 0)).toBeNull()
  })

  it('returns the only clip regardless of the previous choice and the source', () => {
    const only = clip('builtin:only')
    expect(pickRandom([only], 'builtin:only', () => 0)).toBe(only)
    expect(pickRandom([only], 'builtin:only', () => 0.99)).toBe(only)
  })

  it('excludes the previous choice when another candidate exists', () => {
    const pool = [clip('a'), clip('b'), clip('c')]
    vi.spyOn(Math, 'random').mockReturnValue(0)
    expect(pickRandom(pool, 'a', Math.random)?.info.id).toBe('b')
    expect(pickRandom(pool, null, Math.random)?.info.id).toBe('a')
  })

  it('clamps a source returning 1 to the last candidate', () => {
    const pool = [clip('a'), clip('b'), clip('c')]
    expect(pickRandom(pool, null, () => 1)?.info.id).toBe('c')
    expect(pickRandom(pool, null, () => 0.999999)?.info.id).toBe('c')
    expect(pickRandom(pool, null, () => -3)?.info.id).toBe('a')
  })

  it('falls back to the whole pool when every candidate matches the previous id', () => {
    const duplicated = [clip('a'), clip('a')]
    expect(pickRandom(duplicated, 'a', () => 0)?.info.id).toBe('a')
  })
})

describe('IntroResolver.active', () => {
  it('reports none when no clip is discoverable', () => {
    expect(resolverFor(emptyCatalog()).active('s1')).toEqual({ clipId: null, version: null, how: 'none' })
  })

  it('answers with the conversation override before any settings', () => {
    const catalog = catalogWithThreeClips()
    const clips = catalog.list()
    const target = clips[1]
    if (target === undefined) throw new Error('fixture must list two clips')
    const resolver = resolverFor(catalog, {
      overrides: overridesFor({ replay: false, clipId: target.info.id }),
      random: true,
    })

    expect(resolver.active('s1')).toEqual({
      clipId: target.info.id,
      version: target.info.version,
      how: 'session',
    })
  })

  it('ignores an override naming a clip that is no longer listed', () => {
    const catalog = catalogWithThreeClips()
    const selected = catalog.list()[2]
    if (selected === undefined) throw new Error('fixture must list three clips')
    const resolver = resolverFor(catalog, {
      overrides: overridesFor({ replay: false, clipId: 'file:gone-00000000' }),
      selectedClip: selected.info.id,
    })

    expect(resolver.active('s1')).toEqual({
      clipId: selected.info.id,
      version: selected.info.version,
      how: 'selected',
    })
  })

  it('answers with a random clip when random playback is enabled', () => {
    const catalog = catalogWithThreeClips()
    vi.spyOn(Math, 'random').mockReturnValue(0)
    const clips = catalog.list()
    const resolver = resolverFor(catalog, { random: true })

    expect(resolver.active('s1')).toEqual({
      clipId: clips[0]?.info.id,
      version: clips[0]?.info.version,
      how: 'random',
    })
  })

  it('forces a random clip when the caller asks for random mode', () => {
    const catalog = catalogWithThreeClips()
    vi.spyOn(Math, 'random').mockReturnValue(0.99)
    const clips = catalog.list()
    const resolver = resolverFor(catalog, { selectedClip: clips[0]?.info.id ?? 'builtin:brand' })

    expect(resolver.active(null, 'random')).toMatchObject({
      clipId: clips[2]?.info.id,
      how: 'random',
    })
  })

  it('never repeats the previous random answer', () => {
    const catalog = catalogWithThreeClips()
    vi.spyOn(Math, 'random').mockReturnValue(0.99)
    const resolver = resolverFor(catalog)

    const first = resolver.active(null, 'random')
    const second = resolver.active(null, 'random')
    const third = resolver.active(null, 'random')

    expect(first.how).toBe('random')
    expect(second.how).toBe('random')
    expect(third.how).toBe('random')
    expect(second.clipId).not.toBe(first.clipId)
    expect(third.clipId).not.toBe(second.clipId)
  })

  it('answers with the selected clip', () => {
    const catalog = catalogWithThreeClips()
    const selected = catalog.list()[1]
    if (selected === undefined) throw new Error('fixture must list two clips')
    const resolver = resolverFor(catalog, { selectedClip: selected.info.id })

    expect(resolver.active(null)).toEqual({
      clipId: selected.info.id,
      version: selected.info.version,
      how: 'selected',
    })
  })

  it('falls back to the first clip when the selection is not listed', () => {
    const catalog = catalogWithThreeClips()
    const first = catalog.list()[0]
    const resolver = resolverFor(catalog, { selectedClip: 'builtin:brand' })

    expect(resolver.active(null)).toEqual({
      clipId: first?.info.id,
      version: first?.info.version,
      how: 'none',
    })
  })
})

describe('IntroResolver.catalogFor', () => {
  it('returns every clip and the conversation override', () => {
    const catalog = catalogWithThreeClips()
    const selected = catalog.list()[1]
    if (selected === undefined) throw new Error('fixture must list two clips')
    const resolver = resolverFor(catalog, {
      overrides: overridesFor({ replay: true, clipId: selected.info.id }),
    })

    const answer = resolver.catalogFor('s1')
    expect(answer.clips).toEqual(catalog.list().map(clip => clip.info))
    expect(answer.session).toEqual({ replay: true, clipId: selected.info.id })
  })

  it('has no override when storage is unavailable', () => {
    const catalog = catalogWithThreeClips()
    const answer = resolverFor(catalog, { overrides: null }).catalogFor('s1')

    expect(answer.clips).toHaveLength(3)
    expect(answer.session).toEqual({ replay: false, clipId: null })
  })
})

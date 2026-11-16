import {
  chmodSync, mkdirSync, mkdtempSync, rmSync, statSync, utimesSync, writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { builtinId, ClipCatalog } from '../src/host/catalog.ts'

let roots: string[] = []

afterEach(() => {
  for (const root of roots) rmSync(root, { recursive: true, force: true })
  roots = []
})

/** A temporary directory owned by the current test. */
function tempDir(): string {
  const root = mkdtempSync(join(tmpdir(), 'dsh-boot-animation-catalog-'))
  roots.push(root)
  return root
}

/**
 * Write a file of the given content.
 * @param path - target path.
 * @param content - file bytes as text.
 */
function write(path: string, content: string): void {
  mkdirSync(dirname(path), { recursive: true })
  writeFileSync(path, content)
}

describe('builtinId', () => {
  it('names a shipped clip stably', () => {
    expect(builtinId('brand')).toBe('builtin:brand')
  })
})

describe('ClipCatalog', () => {
  it('lists playable built-ins first and only readable video files from the user directories', () => {
    const root = tempDir()
    const assets = join(root, 'assets')
    const videos = join(root, 'videos')
    mkdirSync(assets, { recursive: true })
    mkdirSync(videos, { recursive: true })
    write(join(assets, 'brand.mp4'), 'brand')
    write(join(assets, 'empty.mp4'), '')
    // A shipped clip whose file is gone, and one shadowed by a directory.
    mkdirSync(join(assets, 'awakening.mp4'))
    write(join(videos, 'Holiday Cut.MP4'), 'holiday')
    write(join(videos, 'second.m4v'), 'second')
    write(join(videos, 'third.mov'), 'third')
    write(join(videos, 'fourth.webm'), 'fourth')
    write(join(videos, 'readme.txt'), 'not a video')
    write(join(videos, '.hidden.mp4'), 'hidden')
    write(join(videos, 'empty.mp4'), '')
    // A directory named like a video is not a playable clip.
    mkdirSync(join(videos, 'folder.mp4'))

    const catalog = new ClipCatalog(
      [
        { key: 'brand', path: join(assets, 'brand.mp4') },
        { key: 'empty', path: join(assets, 'empty.mp4') },
        { key: 'missing', path: join(assets, 'missing.mp4') },
        { key: 'directory', path: join(assets, 'awakening.mp4') },
      ],
      [videos],
    )

    const clips = catalog.list()
    const infos = clips.map(clip => clip.info)
    expect(infos[0]).toMatchObject({ id: 'builtin:brand', name: null, kind: 'builtin', bytes: 5 })
    expect(infos.map(info => info.kind)).toEqual(['builtin', 'file', 'file', 'file', 'file'])
    expect(infos.map(info => info.name)).toEqual([null, 'Holiday Cut', 'fourth', 'second', 'third'])
    expect(infos.map(info => info.bytes)).toEqual([5, 7, 6, 6, 5])
    for (const info of infos.slice(1)) expect(info.id).toMatch(/^file:-?[A-Za-z ]+-[0-9a-f]{8}$/)
    expect(clips.map(clip => clip.path)).toEqual([
      join(assets, 'brand.mp4'),
      join(videos, 'Holiday Cut.MP4'),
      join(videos, 'fourth.webm'),
      join(videos, 'second.m4v'),
      join(videos, 'third.mov'),
    ])
    for (const clip of clips) expect(clip.info.version).not.toBe('')
  })

  it('keeps the shipped clips when a user directory cannot be read', () => {
    const root = tempDir()
    const assets = join(root, 'assets')
    const unreadable = join(root, 'unreadable')
    const readable = join(root, 'readable')
    mkdirSync(assets, { recursive: true })
    mkdirSync(unreadable, { recursive: true })
    mkdirSync(readable, { recursive: true })
    write(join(assets, 'brand.mp4'), 'brand')
    write(join(readable, 'visible.mp4'), 'visible')
    chmodSync(unreadable, 0o000)
    try {
      const catalog = new ClipCatalog(
        [{ key: 'brand', path: join(assets, 'brand.mp4') }],
        [unreadable, readable],
      )
      expect(catalog.list().map(clip => clip.info.name)).toEqual([null, 'visible'])
    } finally {
      chmodSync(unreadable, 0o700)
    }
  })

  it('does not list the same directory twice when it is configured twice', () => {
    const root = tempDir()
    write(join(root, 'clip.mp4'), 'clip')
    const catalog = new ClipCatalog([], [root, root])
    expect(catalog.list()).toHaveLength(1)
  })

  it('caches the listing by a per-entry signature, not the directory mtime', () => {
    const root = tempDir()
    const videos = join(root, 'videos')
    const file = join(videos, 'clip.mp4')
    write(file, 'aaaa')
    const catalog = new ClipCatalog([], [videos])

    const first = catalog.list()
    expect(catalog.list()).toBe(first)

    const directoryMtime = statSync(videos).mtimeMs
    // Rewrite the file in place with the same length and a bumped file mtime;
    // the directory itself is untouched, which is the stale-cache regression.
    const fileMtime = statSync(file).mtimeMs
    writeFileSync(file, 'bbbb')
    utimesSync(file, new Date(fileMtime), new Date(fileMtime + 5_000))
    expect(statSync(videos).mtimeMs).toBe(directoryMtime)

    const second = catalog.list()
    expect(second).not.toBe(first)
    expect(second[0]?.info.version).not.toBe(first[0]?.info.version)

    // A different length is also a new version, without any directory change.
    writeFileSync(file, 'cccccc')
    utimesSync(file, new Date(fileMtime + 10_000), new Date(fileMtime + 10_000))
    expect(statSync(videos).mtimeMs).toBe(directoryMtime)
    const third = catalog.list()
    expect(third[0]?.info.version).not.toBe(second[0]?.info.version)
    expect(third[0]?.info.bytes).toBe(6)
  })

  it('resolves a listed clip by id and reports unknown ids', () => {
    const root = tempDir()
    write(join(root, 'clip.mp4'), 'clip')
    const catalog = new ClipCatalog([], [root])
    const [clip] = catalog.list()
    expect(clip).toBeDefined()
    expect(catalog.get(String(clip?.info.id))).toBe(clip)
    expect(catalog.get('file:missing-00000000')).toBeNull()
    expect(catalog.get('builtin:brand')).toBeNull()
  })
})

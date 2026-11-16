import { mkdirSync, mkdtempSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import type { DiscoveredClip } from '../src/host/catalog.ts'
import { parseRange, pinnedVersion, serveClip } from '../src/host/media.ts'
import { FakeResponse, fakeRequest, responseFinished } from './harness.host.ts'
import type { FakeRequestInit } from './harness.host.ts'

let roots: string[] = []

afterEach(() => {
  for (const root of roots) rmSync(root, { recursive: true, force: true })
  roots = []
})

/** A temporary directory owned by the current test. */
function tempDir(): string {
  const root = mkdtempSync(join(tmpdir(), 'dsh-boot-animation-media-'))
  roots.push(root)
  return root
}

/**
 * Write a file below a temporary root.
 * @param root - owning temporary directory.
 * @param name - file name.
 * @param content - file bytes as text.
 * @returns the absolute file path.
 */
function videoFile(root: string, name: string, content = '0123456789'): string {
  const path = join(root, name)
  mkdirSync(dirname(path), { recursive: true })
  writeFileSync(path, content)
  return path
}

/**
 * Describe a file as a discovered clip.
 * @param path - absolute path of the bytes.
 * @param version - content version to publish.
 * @param bytes - byte length; defaults to the real file size.
 * @returns the clip record.
 */
function clipFor(path: string, version = 'v1', bytes?: number): DiscoveredClip {
  return {
    info: {
      id: 'file:clip-00000000',
      name: 'clip',
      kind: 'file',
      bytes: bytes ?? statSync(path).size,
      version,
    },
    path,
  }
}

/**
 * Answer one request for a clip and wait for the response to settle.
 * @param clip - the clip to serve.
 * @param init - request method, target, and headers.
 * @returns the fake response after `finish` or `close`.
 */
async function serve(clip: DiscoveredClip, init: FakeRequestInit = {}): Promise<FakeResponse> {
  const req = fakeRequest(init)
  const res = new FakeResponse(req)
  serveClip(req, res, clip)
  await responseFinished(res)
  return res
}

describe('parseRange', () => {
  it('resolves closed, open-ended, and suffix ranges', () => {
    expect(parseRange('bytes=0-3', 10)).toEqual({ start: 0, end: 3 })
    expect(parseRange('bytes=5-', 10)).toEqual({ start: 5, end: 9 })
    expect(parseRange('bytes=9-9', 10)).toEqual({ start: 9, end: 9 })
    expect(parseRange('bytes=-3', 10)).toEqual({ start: 7, end: 9 })
    expect(parseRange('bytes=-20', 10)).toEqual({ start: 0, end: 9 })
    expect(parseRange('  bytes=0-3  ', 10)).toEqual({ start: 0, end: 3 })
  })

  it('clamps an end past the resource length', () => {
    expect(parseRange('bytes=0-100', 10)).toEqual({ start: 0, end: 9 })
  })

  it('rejects unsatisfiable ranges', () => {
    expect(parseRange('bytes=10-', 10)).toBe('unsatisfiable')
    expect(parseRange('bytes=10-20', 10)).toBe('unsatisfiable')
    expect(parseRange('bytes=5-4', 10)).toBe('unsatisfiable')
    expect(parseRange('bytes=-0', 10)).toBe('unsatisfiable')
    expect(parseRange(`bytes=${'9'.repeat(400)}-`, 10)).toBe('unsatisfiable')
    expect(parseRange(`bytes=0-${'9'.repeat(400)}`, 10)).toBe('unsatisfiable')
    expect(parseRange(`bytes=-${'9'.repeat(400)}`, 10)).toBe('unsatisfiable')
  })

  it('ignores a header that is not a single byte range', () => {
    expect(parseRange('bytes=-', 10)).toBeNull()
    expect(parseRange('bytes=0-1,3-4', 10)).toBeNull()
    expect(parseRange('items=0-3', 10)).toBeNull()
    expect(parseRange('bytes=abc-def', 10)).toBeNull()
    expect(parseRange('0-3', 10)).toBeNull()
  })
})

describe('pinnedVersion', () => {
  it('matches only the query version equal to this clip version', () => {
    expect(pinnedVersion('/dsh-boot-animation/media/file:clip-00000000', 'v1')).toBe(false)
    expect(pinnedVersion('/dsh-boot-animation/media/file:clip-00000000?v=v1', 'v1')).toBe(true)
    expect(pinnedVersion('/dsh-boot-animation/media/file:clip-00000000?v=v1&x=1', 'v1')).toBe(true)
    expect(pinnedVersion('/dsh-boot-animation/media/file:clip-00000000?v=v2', 'v1')).toBe(false)
  })
})

describe('serveClip', () => {
  it('answers a full request with validators and the file bytes', async () => {
    const root = tempDir()
    const clip = clipFor(videoFile(root, 'clip.mp4'))
    const res = await serve(clip, { url: '/dsh-boot-animation/media/file:clip-00000000' })

    expect(res.status).toBe(200)
    expect(res.headers).toMatchObject({
      'content-type': 'video/mp4',
      'accept-ranges': 'bytes',
      etag: '"v1"',
      'cache-control': 'no-cache',
      'content-length': '10',
    })
    expect(res.text).toBe('0123456789')
    expect(res.destroyed).toBe(true)
  })

  it('serves the full clip when the request target is absent', async () => {
    const root = tempDir()
    const clip = clipFor(videoFile(root, 'clip.mp4'))
    const res = await serve(clip, { url: null })

    expect(res.status).toBe(200)
    expect(res.text).toBe('0123456789')
  })

  it('allows immutable caching only for the exact pinned version', async () => {
    const root = tempDir()
    const clip = clipFor(videoFile(root, 'clip.mp4'))
    const pinned = await serve(clip, { url: '/dsh-boot-animation/media/file:clip-00000000?v=v1' })
    const unpinned = await serve(clip, { url: '/dsh-boot-animation/media/file:clip-00000000?v=old' })

    expect(pinned.headers['cache-control']).toBe('public, max-age=31536000, immutable')
    expect(unpinned.headers['cache-control']).toBe('no-cache')
  })

  it('answers a byte range with 206 and the requested slice', async () => {
    const root = tempDir()
    const clip = clipFor(videoFile(root, 'clip.mp4'))
    const res = await serve(clip, {
      url: '/dsh-boot-animation/media/file:clip-00000000',
      headers: { range: 'bytes=2-5' },
    })

    expect(res.status).toBe(206)
    expect(res.headers).toMatchObject({
      'content-type': 'video/mp4',
      'content-range': 'bytes 2-5/10',
      'content-length': '4',
    })
    expect(res.text).toBe('2345')
  })

  it('answers HEAD with the metadata and no body', async () => {
    const root = tempDir()
    const clip = clipFor(videoFile(root, 'clip.mp4'))
    const res = await serve(clip, {
      method: 'HEAD',
      url: '/dsh-boot-animation/media/file:clip-00000000',
    })

    expect(res.status).toBe(200)
    expect(res.headers['content-length']).toBe('10')
    expect(res.body).toHaveLength(0)
  })

  it('revalidates with 304 when the request carries this clip\'s etag', async () => {
    const root = tempDir()
    const clip = clipFor(videoFile(root, 'clip.mp4'))
    const res = await serve(clip, {
      url: '/dsh-boot-animation/media/file:clip-00000000',
      headers: { 'if-none-match': '"other", "v1"' },
    })

    expect(res.status).toBe(304)
    expect(res.headers).toMatchObject({ etag: '"v1"', 'cache-control': 'no-cache' })
    expect(res.body).toHaveLength(0)
  })

  it('serves the body again when the request names another etag', async () => {
    const root = tempDir()
    const clip = clipFor(videoFile(root, 'clip.mp4'))
    const res = await serve(clip, {
      url: '/dsh-boot-animation/media/file:clip-00000000',
      headers: { 'if-none-match': '"other"' },
    })

    expect(res.status).toBe(200)
    expect(res.text).toBe('0123456789')
  })

  it('rejects an unsatisfiable range with 416 and an uncacheable answer', async () => {
    const root = tempDir()
    const clip = clipFor(videoFile(root, 'clip.mp4'))
    const res = await serve(clip, {
      url: '/dsh-boot-animation/media/file:clip-00000000',
      headers: { range: 'bytes=999-' },
    })

    expect(res.status).toBe(416)
    expect(res.headers).toMatchObject({ 'content-range': 'bytes */10', 'cache-control': 'no-store' })
    expect(res.body).toHaveLength(0)
  })

  it('answers 500 with no-store when the file cannot be opened', async () => {
    const root = tempDir()
    const clip = clipFor(join(root, 'missing.mp4'), 'v1', 10)
    const res = await serve(clip, { url: '/dsh-boot-animation/media/file:clip-00000000' })

    expect(res.status).toBe(500)
    expect(res.headers).toMatchObject({
      'content-type': 'text/plain; charset=utf-8',
      'cache-control': 'no-store',
    })
    expect(res.text).toContain('dsh-boot-animation: could not read this clip')
  })

  // POSIX opens a directory read-only and fails on the first read; Windows
  // rejects the open itself, which the 500 case above already covers.
  it.runIf(process.platform !== 'win32')('destroys the response when a read fails after the status line is written', async () => {
    const root = tempDir()
    const directory = join(root, 'not-a-file.mp4')
    mkdirSync(directory, { recursive: true })
    const clip = clipFor(directory, 'v1', 10)
    const res = await serve(clip, { url: '/dsh-boot-animation/media/file:clip-00000000' })

    expect(res.status).toBe(200)
    expect(res.headersSent).toBe(true)
    expect(res.destroyed).toBe(true)
    expect(res.writableFinished).toBe(false)
  })

  it('serves an unknown extension as an opaque download', async () => {
    const root = tempDir()
    const clip = clipFor(videoFile(root, 'clip.mkv'))
    const res = await serve(clip, { url: '/dsh-boot-animation/media/file:clip-00000000' })

    expect(res.status).toBe(200)
    expect(res.headers['content-type']).toBe('application/octet-stream')
  })
})

/**
 * One clip's bytes, addressed by that clip's own id.
 *
 * The response carries validators for exactly this clip, so a switched clip can
 * never be served under a URL that named another one. Only `?v=` equal to this
 * clip's version may be cached forever; every other request revalidates, and
 * every failure is uncacheable so a transient error cannot stick in the browser.
 *
 * A read stream is opened before the status line is written: a missing or
 * permission-denied file answers 500 instead of dying inside a 200 response, and
 * the stream's `error` listener keeps a filesystem failure from becoming an
 * uncaught exception that would take the whole Host down.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/host/media
 */
import { createReadStream } from 'node:fs'
import type { IncomingMessage, ServerResponse } from 'node:http'
import { extname } from 'node:path'
import type { DiscoveredClip } from './catalog.ts'

/** Media types for the extensions the catalog accepts. */
const CONTENT_TYPES: Record<string, string> = {
  '.mp4': 'video/mp4',
  '.m4v': 'video/mp4',
  '.mov': 'video/quicktime',
  '.webm': 'video/webm',
  '.mp3': 'audio/mpeg',
}

/** A byte range resolved against the clip's current length. */
interface ByteRange {
  readonly start: number
  readonly end: number
}

/** Uncached plain-text failure. */
function fail(res: ServerResponse, status: number, message: string): void {
  res.writeHead(status, { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' })
  res.end(message)
}

/**
 * Parse one Range header against a known length.
 * @param header - the raw `Range` value.
 * @param size - current byte length of the resource.
 * @returns the range, `'unsatisfiable'`, or null when the header is not a single byte range.
 */
export function parseRange(header: string, size: number): ByteRange | 'unsatisfiable' | null {
  const match = /^bytes=(\d*)-(\d*)$/.exec(header.trim())
  if (match === null) return null
  /* v8 ignore next -- this anchored pattern always captures group 1. */
  const rawStart = match[1] ?? ''
  /* v8 ignore next -- this anchored pattern always captures group 2. */
  const rawEnd = match[2] ?? ''
  if (rawStart === '' && rawEnd === '') return null
  let start: number
  let end: number
  if (rawStart === '') {
    // Suffix form: the last N bytes.
    const length = Number(rawEnd)
    if (!Number.isFinite(length) || length <= 0) return 'unsatisfiable'
    start = Math.max(0, size - length)
    end = size - 1
  } else {
    start = Number(rawStart)
    end = rawEnd === '' ? size - 1 : Number(rawEnd)
    if (!Number.isFinite(start) || !Number.isFinite(end)) return 'unsatisfiable'
    if (start > end) return 'unsatisfiable'
    if (start >= size) return 'unsatisfiable'
    end = Math.min(end, size - 1)
  }
  return { start, end }
}

/**
 * Whether this request's `?v=` pins the exact bytes being served.
 * @param url - the raw request target.
 * @param version - this clip's version.
 * @returns true when the response may be cached immutably.
 */
export function pinnedVersion(url: string, version: string): boolean {
  const query = url.indexOf('?')
  if (query === -1) return false
  return new URLSearchParams(url.slice(query + 1)).get('v') === version
}

/**
 * Answer one request for this clip.
 * @param req - the incoming request.
 * @param res - the response to own completely.
 * @param clip - the clip whose bytes are wanted.
 */
export function serveClip(req: IncomingMessage, res: ServerResponse, clip: DiscoveredClip): void {
  const { bytes: size, version } = clip.info
  const contentType = CONTENT_TYPES[extname(clip.path).toLowerCase()] ?? 'application/octet-stream'
  const etag = `"${version}"`
  const cacheControl = pinnedVersion(req.url ?? '', version) ? 'public, max-age=31536000, immutable' : 'no-cache'
  const common = {
    'content-type': contentType,
    'accept-ranges': 'bytes',
    etag,
    'cache-control': cacheControl,
  }

  const ifNoneMatch = req.headers['if-none-match']
  if (typeof ifNoneMatch === 'string'
    && ifNoneMatch.split(',').some(candidate => candidate.trim() === etag)) {
    res.writeHead(304, { etag, 'cache-control': cacheControl })
    res.end()
    return
  }

  const rangeHeader = req.headers.range
  const parsed = typeof rangeHeader === 'string' ? parseRange(rangeHeader, size) : null
  if (parsed === 'unsatisfiable') {
    res.writeHead(416, { 'content-range': `bytes */${String(size)}`, 'cache-control': 'no-store' })
    res.end()
    return
  }

  const start = parsed === null ? undefined : parsed.start
  const end = parsed === null ? undefined : parsed.end
  const stream = start === undefined ? createReadStream(clip.path) : createReadStream(clip.path, { start, end })
  let settled = false
  const onError = (error: unknown): void => {
    /* v8 ignore next -- a read stream reports its failure once. */
    if (settled) return
    settled = true
    if (res.headersSent) {
      // The status line is already on the wire; end the connection rather than
      // pretend the truncated body was complete.
      res.destroy()
      return
    }
    /* v8 ignore next -- node:fs stream failures are always Error instances. */
    fail(res, 500, `dsh-boot-animation: could not read this clip (${error instanceof Error ? error.message : String(error)})`)
  }
  stream.on('error', onError)
  // A client that navigates away must not leave the read running.
  res.on('close', () => { stream.destroy() })
  stream.once('open', () => {
    /* v8 ignore next -- no step can settle the request before its open event. */
    if (settled) { stream.destroy(); return }
    settled = false
    if (parsed === null) {
      res.writeHead(200, { ...common, 'content-length': String(size) })
    } else {
      res.writeHead(206, {
        ...common,
        'content-range': `bytes ${String(parsed.start)}-${String(parsed.end)}/${String(size)}`,
        'content-length': String(parsed.end - parsed.start + 1),
      })
    }
    if (req.method === 'HEAD') { stream.destroy(); res.end(); return }
    stream.pipe(res)
  })
}

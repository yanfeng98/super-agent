/**
 * Clip discovery: shipped built-ins plus the user's own video files.
 *
 * Identity is content-addressed rather than positional: a built-in is
 * `builtin:<name>`, a user file is `file:<stem>-<hash>`. The hash covers the
 * absolute path, so renaming or moving a file yields a new id and the caller
 * falls back instead of playing different bytes under an old name.
 *
 * The scan is cached under a signature built from EVERY entry's size and mtime.
 * A directory-level signature is not enough: replacing a file in place leaves
 * the directory's mtime untouched, which would keep publishing a stale version
 * and let `?v=` serve permanently cached old bytes.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/host/catalog
 */
import { readdirSync, statSync } from 'node:fs'
import { basename, extname, join } from 'node:path'
import type { ClipInfo, ClipKind } from '../types.ts'

/** Extensions the browser can decode; anything else is not listed. */
const VIDEO_EXTENSIONS = new Set(['.mp4', '.m4v', '.mov', '.webm'])

/** One shipped intro, resolved beside the package root. */
export interface BuiltinClip {
  /** Suffix after `builtin:`. */
  readonly key: string
  /** File name under `assets/`. */
  readonly fileName: string
}

/** A listed clip plus where its bytes live. */
export interface DiscoveredClip {
  readonly info: ClipInfo
  /** Absolute path of the file holding the bytes. */
  readonly path: string
}

/** Stable 32-bit FNV-1a, rendered as hex; identity for paths, not security. */
function pathHash(value: string): string {
  let hash = 0x811c9dc5
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index)
    hash = Math.imul(hash, 0x01000193) >>> 0
  }
  return hash.toString(16).padStart(8, '0')
}

/** `<size>-<mtime>` in base 36: the version this clip's bytes carry. */
function versionOf(size: number, mtimeMs: number): string {
  return `${size.toString(36)}-${Math.round(mtimeMs).toString(36)}`
}

/** A stat result, or null when the path is unreadable. */
function statOrNull(path: string): { size: number; mtimeMs: number } | null {
  try {
    const stats = statSync(path)
    return stats.isFile() ? { size: stats.size, mtimeMs: stats.mtimeMs } : null
  } catch {
    return null
  }
}

/**
 * The clip id of a shipped intro.
 * @param key - the built-in's id suffix.
 * @returns the stable `builtin:` id.
 */
export function builtinId(key: string): string {
  return `builtin:${key}`
}

/**
 * The clip list served to the browser, cached by content signature.
 *
 * Reads never throw: an unreadable user directory yields the built-ins, which
 * ship inside the package and therefore cannot be missing.
 */
export class ClipCatalog {
  private signature: string | null = null
  private cached: DiscoveredClip[] = []

  /**
   * @param builtinFiles - absolute paths of the shipped clips, keyed by id suffix.
   * @param userDirs - directories the user drops files into.
   */
  constructor(
    private readonly builtins: readonly { key: string; path: string }[],
    private readonly userDirs: readonly string[],
  ) {}

  /**
   * Every playable clip, built-ins first.
   * @returns the clips currently discoverable, from the cache when nothing changed.
   */
  list(): DiscoveredClip[] {
    const signature = this.currentSignature()
    if (this.signature === signature) return this.cached
    const clips = [...this.builtinClips(), ...this.fileClips()]
    this.signature = signature
    this.cached = clips
    return clips
  }

  /**
   * Resolve one clip id.
   * @param id - a `builtin:` or `file:` id from the wire.
   * @returns the clip, or null when no such clip exists now.
   */
  get(id: string): DiscoveredClip | null {
    return this.list().find(clip => clip.info.id === id) ?? null
  }

  private currentSignature(): string {
    const parts: string[] = []
    for (const builtin of this.builtins) {
      const stats = statOrNull(builtin.path)
      parts.push(stats === null ? `${builtin.path}:-` : `${builtin.path}:${String(stats.size)}:${String(stats.mtimeMs)}`)
    }
    for (const dir of this.userDirs) {
      parts.push(this.directorySignature(dir))
    }
    return parts.join('|')
  }

  private directorySignature(dir: string): string {
    let names: string[]
    try {
      names = readdirSync(dir).sort()
    } catch {
      return `${dir}:-`
    }
    const entries = names.map((name) => {
      const path = join(dir, name)
      const stats = statOrNull(path)
      return stats === null ? null : `${name}:${String(stats.size)}:${String(stats.mtimeMs)}`
    })
    return `${dir}:${entries.filter(entry => entry !== null).join(',')}`
  }

  private builtinClips(): DiscoveredClip[] {
    const clips: DiscoveredClip[] = []
    for (const builtin of this.builtins) {
      const stats = statOrNull(builtin.path)
      if (stats === null || stats.size === 0) continue
      clips.push({
        info: {
          id: builtinId(builtin.key),
          name: null,
          kind: 'builtin' satisfies ClipKind,
          bytes: stats.size,
          version: versionOf(stats.size, stats.mtimeMs),
        },
        path: builtin.path,
      })
    }
    return clips
  }

  private fileClips(): DiscoveredClip[] {
    const clips: DiscoveredClip[] = []
    const seen = new Set<string>()
    for (const dir of this.userDirs) {
      let names: string[]
      try {
        names = readdirSync(dir).sort()
      } catch {
        continue
      }
      for (const name of names) {
        if (name.startsWith('.')) continue
        const extension = extname(name).toLowerCase()
        if (!VIDEO_EXTENSIONS.has(extension)) continue
        const path = join(dir, name)
        const stats = statOrNull(path)
        if (stats === null || stats.size === 0) continue
        const id = `file:${basename(name, extname(name))}-${pathHash(path)}`
        if (seen.has(id)) continue
        seen.add(id)
        clips.push({
          info: {
            id,
            name: basename(name, extname(name)),
            kind: 'file' satisfies ClipKind,
            bytes: stats.size,
            version: versionOf(stats.size, stats.mtimeMs),
          },
          path,
        })
      }
    }
    return clips
  }
}

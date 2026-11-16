/**
 * The archive sequence's shipped audio: six recorded lines and two interface
 * cues, addressed by name.
 *
 * Names are a closed set, so a request can never name a path: the route validates
 * against {@link AUDIO_NAMES} and the file name is built here. A missing file is
 * not an error the browser must handle; the sequence falls back to synthesized
 * audio and speech.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/host/audio
 */

import { statSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import type { AudioName } from '../audio-names.ts'
import type { DiscoveredClip } from './catalog.ts'

/**
 * Resolve the shipped audio directory of this package.
 * @param moduleUrl - the executing module's URL, so the answer follows the built entry.
 * @returns the absolute directory holding the audio files.
 */
export function audioDir(moduleUrl: string): string {
  return fileURLToPath(new URL('../assets/audio/', moduleUrl)).replace(/[\\/]$/u, '')
}

/**
 * One named audio file, stat'ed on demand.
 *
 * The version is the file's size and mtime, so a replaced file is served under a
 * new `?v=` and the browser cannot keep stale bytes.
 */
export class AudioCatalog {
  /** @param dir - absolute directory holding the shipped files. */
  constructor(private readonly dir: string) {}

  /**
   * Resolve one shipped file.
   * @param name - a name from {@link AUDIO_NAMES}.
   * @returns the file's clip record, or null when it is absent or empty.
   */
  get(name: AudioName): DiscoveredClip | null {
    const path = join(this.dir, `${name}.mp3`)
    let stats
    try {
      stats = statSync(path)
    } catch {
      return null
    }
    if (!stats.isFile() || stats.size === 0) return null
    return {
      info: {
        id: name,
        name,
        kind: 'builtin',
        bytes: stats.size,
        version: `${stats.size.toString(36)}-${Math.round(stats.mtimeMs).toString(36)}`,
      },
      path,
    }
  }
}

/**
 * Names of the archive sequence's shipped audio, shared by both faces.
 *
 * The Host serves exactly these files and the browser asks for exactly these
 * names, so the closed set lives in one browser-safe module rather than in the
 * Host's filesystem code.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/audio-names
 */

/** Every shipped audio file, without its extension. */
export const AUDIO_NAMES = [
  'line-1',
  'line-2',
  'line-3',
  'line-4',
  'line-5',
  'line-6',
  'sfx-tick',
  'sfx-ticks-run',
] as const

/** One shipped audio file. */
export type AudioName = (typeof AUDIO_NAMES)[number]

/**
 * Whether a request named one of the shipped files.
 * @param value - the decoded path segment.
 * @returns true when the name belongs to {@link AUDIO_NAMES}.
 */
export function isAudioName(value: string): value is AudioName {
  return (AUDIO_NAMES as readonly string[]).includes(value)
}

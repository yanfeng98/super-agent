/**
 * Which startup stages run, in order.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/host/plan
 */

import type { SequenceMode, StartupStage } from '../types.ts'

/** The stage order each mode produces. */
const STAGES: Record<SequenceMode, readonly StartupStage[]> = {
  'archive-then-video': ['archive', 'video'],
  'video-then-archive': ['video', 'archive'],
  'archive-only': ['archive'],
  'video-only': ['video'],
}

/**
 * Resolve the configured sequence into the stages the browser plays.
 * @param mode - the user's sequence preference.
 * @returns the stages, in playback order.
 */
export function stagesOf(mode: SequenceMode): readonly StartupStage[] {
  return STAGES[mode]
}

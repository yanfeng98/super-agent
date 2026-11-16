/**
 * The single answer to "which clip plays now".
 *
 * Every caller asks the same question, so the priority chain exists once:
 * a conversation override, then random playback, then the user's selection, then
 * whatever is left. The override outranks random on purpose - a conversation
 * that names a clip and is handed a random one is not what naming it meant.
 *
 * Random memory is per process and shared by every caller. It is advanced only
 * when a request actually asks for a random answer, so a catalog refresh or a
 * status read never consumes the "do not repeat" state.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/host/resolve
 */
import type { ClipCatalog, DiscoveredClip } from './catalog.ts'
import type { SessionOverrides } from './overrides.ts'
import type { ActiveClip, ClipInfo, SessionIntro } from '../types.ts'

/** The settings the chain reads, resolved per request. */
export interface ResolverConfig {
  readonly random: boolean
  readonly selectedClip: string
}

/** Why a clip was chosen, for diagnostics. */
type Reason = ActiveClip['how']

/**
 * Choose one clip from a pool, avoiding the previous choice.
 * @param pool - candidates in catalog order.
 * @param last - the previously chosen id, excluded when another candidate exists.
 * @param random - uniform source in `[0, 1)`; clamped so `1` still selects a candidate.
 * @returns the chosen clip, or null for an empty pool.
 */
export function pickRandom(
  pool: readonly DiscoveredClip[],
  last: string | null,
  random: () => number,
): DiscoveredClip | null {
  if (pool.length === 0) return null
  const candidates = pool.length > 1 ? pool.filter(clip => clip.info.id !== last) : pool
  const usable = candidates.length === 0 ? pool : candidates
  const raw = Math.floor(Math.min(Math.max(random(), 0), 0.999999) * usable.length)
  /* v8 ignore next -- the clamp keeps raw inside the usable index range. */
  return usable[raw] ?? usable[usable.length - 1] ?? null
}

/** Answers clip questions for one plugin instance. */
export class IntroResolver {
  private lastPicked: string | null = null

  /**
   * @param catalog - clip discovery.
   * @param overrides - per-conversation store, or null when storage is unavailable.
   * @param config - current settings, read once per question.
   */
  constructor(
    private readonly catalog: ClipCatalog,
    private readonly overrides: () => SessionOverrides | null,
    private readonly config: () => ResolverConfig,
  ) {}

  /**
   * Which clip this conversation plays now.
   * @param sessionId - the asking conversation, or null for the global question.
   * @param mode - `random` forces a random pick; `active` follows the settings.
   * @returns the answer plus why it was chosen.
   */
  active(sessionId: string | null, mode: 'active' | 'random' = 'active'): ActiveClip {
    const clips = this.catalog.list()
    if (clips.length === 0) return { clipId: null, version: null, how: 'none' }
    const override = this.#override(sessionId)
    if (override.clipId !== null) {
      const named = clips.find(clip => clip.info.id === override.clipId)
      if (named !== undefined) return this.#answer(named, 'session')
    }
    const settings = this.config()
    if (mode === 'random' || settings.random) {
      const chosen = pickRandom(clips, this.lastPicked, Math.random)
      /* v8 ignore next 4 -- the non-empty list checked above always yields a clip. */
      if (chosen !== null) {
        this.lastPicked = chosen.info.id
        return this.#answer(chosen, 'random')
      }
    }
    const selected = clips.find(clip => clip.info.id === settings.selectedClip)
    const fallback = selected ?? clips[0]
    /* v8 ignore next -- the non-empty list checked above always has a first clip. */
    if (fallback === undefined) return { clipId: null, version: null, how: 'none' }
    return this.#answer(fallback, selected === undefined ? 'none' : 'selected')
  }

  /**
   * The catalog page: every clip plus this conversation's override.
   * @param sessionId - the asking conversation, or null.
   * @returns clips and the stored override.
   */
  catalogFor(sessionId: string | null): { clips: readonly ClipInfo[]; session: SessionIntro } {
    return {
      clips: this.catalog.list().map(clip => clip.info satisfies ClipInfo),
      session: this.#override(sessionId),
    }
  }

  #override(sessionId: string | null): SessionIntro {
    const store = this.overrides()
    return store === null ? { replay: false, clipId: null } : store.read(sessionId)
  }

  #answer(clip: DiscoveredClip, how: Reason): ActiveClip {
    return { clipId: clip.info.id, version: clip.info.version, how }
  }
}

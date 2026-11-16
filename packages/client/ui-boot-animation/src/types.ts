/**
 * Wire types shared by the Host video route and the browser face.
 *
 * The browser learns clip identity, byte size and content version from
 * `catalog.json`, then addresses one clip through `media/<id>?v=<version>`.
 * Names arrive for user files only: built-in clips are localized by the browser
 * from their stable id, so product copy never travels as Host data.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/types
 */

/** How the intro pixel-fits the window. */
export type FitMode = 'cover' | 'contain'

/** Which moment auto-plays the intro. */
export type PlayTiming = 'app-start' | 'entry'

/** Where one clip's bytes come from. */
export type ClipKind = 'builtin' | 'file'

/** One playable intro, as published to the browser. */
export interface ClipInfo {
  /** Stable identity: `builtin:<name>` or `file:<stem>-<hash>`. */
  readonly id: string
  /** User-file display name; null for built-ins, which the browser localizes. */
  readonly name: string | null
  readonly kind: ClipKind
  /** Byte length, used to show sizes and to reject empty files. */
  readonly bytes: number
  /** Content identity; changes whenever the bytes do. */
  readonly version: string
}

/** One conversation's intro override. */
export interface SessionIntro {
  /** Play the intro on every entry into this conversation, not just its first. */
  readonly replay: boolean
  /** The clip this conversation plays; null uses the global selection. */
  readonly clipId: string | null
}

/** `GET catalog.json` answer: every clip and the asking conversation's override. */
export interface CatalogResponse {
  readonly clips: readonly ClipInfo[]
  readonly session: SessionIntro
  /** False when the Host has no durable storage, so per-conversation state is unavailable. */
  readonly overridesAvailable: boolean
}

/** Why the Host chose the clip it answered with. */
export type ActiveReason = 'session' | 'random' | 'selected' | 'none'

/** `GET active.json` answer: the one clip this conversation plays now. */
export interface ActiveResponse {
  readonly clipId: string | null
  readonly version: string | null
  readonly how: ActiveReason
}

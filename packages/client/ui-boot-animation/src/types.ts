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

/** One stage of the startup experience. */
export type StartupStage = 'archive' | 'video'

/** Which stages run, and in which order. */
export type SequenceMode = 'archive-then-video' | 'video-then-archive' | 'archive-only' | 'video-only'

/** The archive sequence's own settings, as the Host projects them. */
export interface ScreenSettings {
  /** Color scheme of the sequence: 'light' is the warm paper plinth, 'dark' the dark one. */
  readonly screenTheme: ('light' | 'dark')
  /** Multiplier for every fixed duration in the sequence; 0.35 is slow, 3 is fast. */
  readonly screenSpeed: number
  /** Whether the first two lines wait for a click, Enter, or Space. */
  readonly screenRequireInteraction: boolean
  /** Whether the skip control is shown and Escape dismisses the sequence. */
  readonly screenAllowSkip: boolean
  /** Whether interface cues play at all. */
  readonly screenSound: boolean
  /** Volume of the interface cues, 0 to 1. */
  readonly screenSoundVolume: number
  /** Whether the five lines are spoken. */
  readonly screenVoice: boolean
  /** Where the voice comes from: 'asset' prefers the recorded takes, 'tts' always synthesizes. */
  readonly screenVoiceSource: ('asset' | 'tts')
  /** Language spoken: 'en', 'zh', or 'both'. */
  readonly screenVoiceLang: ('en' | 'zh' | 'both')
  /** Exact installed voice name, or an empty string to pick one automatically. */
  readonly screenVoiceName: string
  /** Speaking rate for synthesized lines. */
  readonly screenVoiceRate: number
  /** Pitch for synthesized lines; below 1 is deeper. */
  readonly screenVoicePitch: number
  /** Volume of the spoken lines, 0 to 1. */
  readonly screenVoiceVolume: number
  /** Whether a short cue plays before a spoken line. */
  readonly screenVoiceCue: boolean
  /** Silence in milliseconds between the cue and the spoken line. */
  readonly screenVoiceLead: number
  /** Source of the interface cues: 'audio' plays the shipped files, 'synth' synthesizes them. */
  readonly screenSfx: ('audio' | 'synth')
  /** Name in the closing welcome line and on the identity card. */
  readonly screenIdentity: string
  /** Identity number on the identity card and the confirmation line. */
  readonly screenIdentityId: string
  /** Access level shown on the identity card. */
  readonly screenAccessLevel: string
  /** First line of the brand block in the top-left corner. */
  readonly screenBrand1: string
  /** Second line of the brand block. */
  readonly screenBrand2: string
  /** Letter-spaced mark in the third brand line. */
  readonly screenBrand3a: string
  /** Suffix after the brand mark. */
  readonly screenBrand3b: string
  /** First line of the black prologue wordmark. */
  readonly screenOrgZh: string
  /** Second line of the black prologue wordmark. */
  readonly screenOrgEn: string
  /** Subtitle under the prologue wordmark. */
  readonly screenOrgSub: string
  /** Text in the black box of the closing act. */
  readonly screenWelcomeBox: string
  /** Text in the bottom-right footer. */
  readonly screenFooter: string
  /** Accent color for the status bar, progress bar, and reticle. */
  readonly screenAccent: string
  /** First color of the chromatic split during the reveal. */
  readonly screenGlitchA: string
  /** Second color of the chromatic split during the reveal. */
  readonly screenGlitchB: string
  /** Whether the plinth and prologue become translucent frosted glass. */
  readonly screenGlass: boolean
  /** Opacity of the glass layers, 0.15 to 0.95. */
  readonly screenGlassOpacity: number
  /** Frosted blur radius in pixels. */
  readonly screenGlassBlur: number
  /** Saturation percentage behind the glass. */
  readonly screenGlassSaturate: number
  /** Blur radius in pixels applied behind the overlay. */
  readonly screenGlassBackdropBlur: number
}


/** The clip the video stage should play, and why it was chosen. */
export interface ActiveClip {
  readonly clipId: string | null
  readonly version: string | null
  readonly how: ActiveReason
}

/** `GET active.json` answer: the stages this conversation plays now, and the clip the video stage uses. */
export interface ActiveResponse extends ActiveClip {
  readonly stages: readonly StartupStage[]
}

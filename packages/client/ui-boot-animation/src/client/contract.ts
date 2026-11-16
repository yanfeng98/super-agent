/**
 * The browser face's contract: namespace, shared data, injected callbacks.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/contract
 */
import type { HostObservable, InjectFace, PropsLocale, PropsRuntime } from '@deepseek-ai/dsh-client-ui-slots'
import type {} from '@deepseek-ai/dsh-client-ui-layout/client'
import type { ClipInfo, FitMode, PlayTiming, SessionIntro } from '../types.ts'
import type { BootAnimationLocaleKey } from './locales.ts'

/** Locale namespace owned by this package. */
export const NS = 'bootAnimation'

declare module '@deepseek-ai/dsh-client-ui-slots' {
  interface LocaleNamespaceMap {
    /** Startup intro settings and overlay copy. */
    bootAnimation: BootAnimationLocaleKey
  }
}

/** Playback phases the overlay renders. */
export type PlaybackPhase = 'idle' | 'loading' | 'playing'

/** What the overlay is playing right now. */
export interface PlaybackState {
  readonly phase: PlaybackPhase
  readonly clipId: string | null
  readonly url: string | null
  readonly reason: IntroReason
  /** Bumped on every new clip so re-playing the same clip restarts the element. */
  readonly nonce: number
  /** True while the user is auditioning a clip from the settings page. */
  readonly preview: boolean
}

/** Why the intro is playing, for the overlay's corner label. */
export type IntroReason = 'app-start' | 'entry' | 'session' | 'random' | 'selected' | 'preview' | 'none'

/** The user's preferences, as the Host projects them. */
export interface IntroSettings {
  readonly enabled: boolean
  readonly timing: PlayTiming
  readonly fit: FitMode
  readonly random: boolean
  readonly selectedClip: string
}

/** Preferences used before the Host has projected a value; they mirror the Host defaults. */
export const DEFAULT_SETTINGS: IntroSettings = {
  enabled: true,
  timing: 'app-start',
  fit: 'cover',
  random: false,
  selectedClip: 'builtin:brand',
}

/** The clip list and the asking conversation's override. */
export interface CatalogState {
  readonly clips: readonly ClipInfo[]
  readonly session: SessionIntro
  readonly loading: boolean
  readonly error: string | null
  /** False when the Host has no durable storage for per-conversation state. */
  readonly overridesAvailable: boolean
}

/** The current conversation, as far as the intro needs it. */
export interface SessionBinding {
  readonly sessionId: string | null
  readonly blank: boolean
}

/** What the overlay registration injects. */
export interface OverlayInjected {
  readonly hooks: {
    readonly playback: HostObservable<PlaybackState>
    readonly settings: HostObservable<IntroSettings>
  }
  /** Report that the first frame arrived, so the overlay stops claiming to load. */
  readonly markPlaying: () => void
  /** Stop the current playback (skip, end of clip, or Escape). */
  readonly stop: () => void
}

/** What the settings registration injects. */
export interface SettingsInjected {
  readonly hooks: {
    readonly catalog: HostObservable<CatalogState>
    readonly settings: HostObservable<IntroSettings>
    readonly session: HostObservable<SessionBinding>
  }
  /** Reload the clip list from the Host. */
  readonly refresh: () => void
  /** Play one clip now without changing the selection. */
  readonly preview: (clipId: string) => void
  readonly setEnabled: (enabled: boolean) => void
  readonly setTiming: (timing: PlayTiming) => void
  readonly setFit: (fit: FitMode) => void
  readonly setRandom: (random: boolean) => void
  readonly setSelectedClip: (clipId: string) => void
  readonly setSessionReplay: (replay: boolean) => void
  readonly setSessionClip: (clipId: string | null) => void
  /** False when the Host offers no durable storage for per-conversation state. */
  readonly sessionStorageAvailable: boolean
}

/** Overlay props: runtime seat, localized copy, injected face. */
export type IntroOverlayProps =
  PropsRuntime<'shell.overlay'> & PropsLocale<typeof NS> & InjectFace<OverlayInjected>

/** Settings section props: runtime seat, localized copy, injected face. */
export type SettingsSectionProps =
  PropsRuntime<'settings.section'> & PropsLocale<typeof NS> & InjectFace<SettingsInjected>

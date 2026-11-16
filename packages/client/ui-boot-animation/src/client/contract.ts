/**
 * The browser face's contract: namespace, shared data, injected callbacks.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/contract
 */
import type { HostObservable, InjectFace, PropsLocale, PropsRuntime } from '@deepseek-ai/dsh-client-ui-slots'
import type {} from '@deepseek-ai/dsh-client-ui-layout/client'
import type { ClipInfo, FitMode, PlayTiming, ScreenSettings, SequenceMode, SessionIntro, StartupStage } from '../types.ts'
import { SCREEN_DEFAULTS, SEQUENCE_MODES, screenSettingsOf } from '../screen.ts'
import type { ScreenProjection } from '../screen.ts'
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

/** The stage playing, or `none` while nothing runs. */
export type StageName = StartupStage | 'none'

/** What the overlay is playing right now. */
export interface PlaybackState {
  readonly phase: PlaybackPhase
  /** Which stage is on screen. */
  readonly stage: StageName
  readonly clipId: string | null
  readonly url: string | null
  readonly reason: IntroReason
  /** Bumped on every new stage so re-playing the same clip restarts the element. */
  readonly nonce: number
  /** True while the user is auditioning from the settings page. */
  readonly preview: boolean
}

/** Why the intro is playing, for the overlay's corner label. */
export type IntroReason = 'app-start' | 'entry' | 'session' | 'random' | 'selected' | 'preview' | 'none'

/** The user's preferences, as the Host projects them. */
export interface IntroSettings extends ScreenSettings {
  readonly enabled: boolean
  readonly timing: PlayTiming
  readonly fit: FitMode
  readonly random: boolean
  readonly selectedClip: string
  /** Which stages run, and in which order. */
  readonly sequence: SequenceMode
}

/** Preferences used before the Host has projected a value; they mirror the Host defaults. */
export const DEFAULT_SETTINGS: IntroSettings = {
  ...SCREEN_DEFAULTS,
  enabled: true,
  timing: 'app-start',
  fit: 'cover',
  random: false,
  selectedClip: 'builtin:brand',
  sequence: 'archive-then-video',
}

/** The two moments the experience can start on its own. */
const TIMINGS: readonly PlayTiming[] = ['app-start', 'entry']

/** The two ways a clip fits the window. */
const FITS: readonly FitMode[] = ['cover', 'contain']

/** A preference projection: any field may be absent or carry an unusable value. */
export type SettingsProjection = ScreenProjection & {
  readonly enabled?: unknown
  readonly timing?: unknown
  readonly fit?: unknown
  readonly random?: unknown
  readonly selectedClip?: unknown
  readonly sequence?: unknown
}

/**
 * Read a full set of preferences from a possibly stale projection.
 * @param raw - the Host projection, or null before one arrives.
 * @returns every preference, with absent or unusable values replaced by defaults.
 */
export function normalizeIntroSettings(raw: SettingsProjection | null): IntroSettings {
  const value = raw ?? DEFAULT_SETTINGS
  const timing = TIMINGS.find(candidate => candidate === value.timing)
  const fit = FITS.find(candidate => candidate === value.fit)
  const sequence = SEQUENCE_MODES.find(candidate => candidate === value.sequence)
  return {
    ...screenSettingsOf(value),
    enabled: typeof value.enabled === 'boolean' ? value.enabled : DEFAULT_SETTINGS.enabled,
    timing: timing ?? DEFAULT_SETTINGS.timing,
    fit: fit ?? DEFAULT_SETTINGS.fit,
    random: typeof value.random === 'boolean' ? value.random : DEFAULT_SETTINGS.random,
    selectedClip: typeof value.selectedClip === 'string' && value.selectedClip !== ''
      ? value.selectedClip
      : DEFAULT_SETTINGS.selectedClip,
    sequence: sequence ?? DEFAULT_SETTINGS.sequence,
  }
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
export interface StartupSessionBinding {
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
  /** Finish the current stage and continue with the next one, if any. */
  readonly advance: () => void
  /** Stop the whole experience (skip, Escape, or an unplayable stage). */
  readonly stop: () => void
}

/** What the settings registration injects. */
export interface SettingsInjected {
  readonly hooks: {
    readonly catalog: HostObservable<CatalogState>
    readonly settings: HostObservable<IntroSettings>
    readonly session: HostObservable<StartupSessionBinding>
  }
  /** Reload the clip list from the Host. */
  readonly refresh: () => void
  /** Play one clip now without changing the selection. */
  readonly preview: (clipId: string) => void
  /** Play the archive sequence now without changing anything. */
  readonly previewArchive: () => void
  readonly setEnabled: (enabled: boolean) => void
  readonly setTiming: (timing: PlayTiming) => void
  readonly setFit: (fit: FitMode) => void
  readonly setRandom: (random: boolean) => void
  readonly setSelectedClip: (clipId: string) => void
  readonly setSequence: (sequence: SequenceMode) => void
  /** Write one screen field through the Host-backed settings form. */
  readonly setScreen: (key: keyof ScreenSettings, value: string | number | boolean) => void
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

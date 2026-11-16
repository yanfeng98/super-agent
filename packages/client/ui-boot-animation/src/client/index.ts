/**
 * Startup intro - browser face.
 *
 * This module owns every decision and every Host request; the two components
 * render what it publishes and call back into it. Keeping it here (not in a
 * component effect) means the rule runs once per relevant change instead of once
 * per render, and the overlay has no lifecycle of its own to get wrong.
 *
 * The intro plays when {@link decidePlay} says so. A play records itself before
 * the element is pointed at a clip, and only a play that produced a clip is
 * recorded, so a failed resolve is retried on the next evaluation instead of
 * consuming the launch.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/client
 */
import type { Context } from '@deepseek-ai/cordis'
import type {} from '@deepseek-ai/dsh-client-locale/client'
import type {} from '@deepseek-ai/dsh-client-ui-renderer/client'
import type {} from '@deepseek-ai/dsh-client-ui-slots'
import type {} from '@deepseek-ai/dsh-client-ui-layout/client'
import type {} from '@deepseek-ai/dsh-client-ui-session/client'
import type {} from '@deepseek-ai/dsh-client-ui-settings/client'
import { fetchActive, fetchCatalog, mediaUrlFor, writeSessionOverride } from './api.ts'
import { IntroOverlay } from './IntroOverlay.tsx'
import { IntroRecords, decidePlay, sessionStorageOrNull } from './decide.ts'
import { SettingsSection } from './SettingsSection.tsx'
import { NS } from './contract.ts'
import { DEFAULT_SETTINGS } from './contract.ts'
import type { CatalogState, IntroReason, IntroSettings, PlaybackState, SessionBinding } from './contract.ts'
import { createSource } from './source.ts'
import { de, en, es, fr, ja, pt, ru, zh } from './locales.ts'
import type { FitMode, PlayTiming } from '../types.ts'

/** Stable Cordis plugin name. */
export const name = 'dsh-client-ui-boot-animation'

/** Services this plugin reads; without them it has nothing to attach to. */
export const inject = ['slots', 'locale', 'uiSession', 'configForms']

/** Profile entry id owning the preferences. */
const SETTINGS_ID = 'ui-boot-animation'

/** Nothing is playing. */
const IDLE: PlaybackState = { phase: 'idle', clipId: null, url: null, reason: 'none', nonce: 0, preview: false }

/** The conversation binding shape this plugin reads. */
interface SessionFace {
  getSnapshot?: () => { blank?: unknown } | null
  subscribe?: (listener: () => void) => unknown
}

/** The resolved session binding, as far as the intro needs it. */
interface Binding {
  key?: unknown
  hooks?: { session?: SessionFace }
}

/** The `uiSession.adapter.current` observable. */
interface BindingSource {
  getSnapshot: () => Binding | null
  subscribe: (listener: () => void) => () => void
}

/** Read the settings projection's current value. */
function settingsOf(form: { getSnapshot: () => { value: IntroSettings | undefined } }): IntroSettings | null {
  return form.getSnapshot().value ?? null
}

/**
 * Wire the intro into the client.
 * @param ctx - client root context.
 */
export function apply(ctx: Context): void {
  ctx.effect(() => ctx.locale.register(NS, { zh, en, ja, es, fr, de, ru, pt }), 'boot-animation: dictionaries')
  const t = ctx.locale.bind(NS)

  const form = ctx.configForms.get<IntroSettings>(SETTINGS_ID)
  const settings = createSource<IntroSettings | null>(settingsOf(form))
  const catalog = createSource<CatalogState>({
    clips: [], session: { replay: false, clipId: null }, loading: true, error: null, overridesAvailable: false,
  })
  const playback = createSource<PlaybackState>(IDLE)
  const bindingSource = ctx.uiSession.adapter.current as BindingSource
  const session = createSource<SessionBinding>({ sessionId: null, blank: false })
  const records = new IntroRecords(sessionStorageOrNull())
  const lifetime = new AbortController()
  let lastSessionId: string | null = null
  let mirroredFace: SessionFace | undefined
  let nestedStop: (() => void) | null = null
  let inFlight = false

  const stop = (): void => { playback.set(IDLE) }
  /** The first frame arrived: only `loading` may become `playing`. */
  const markPlaying = (): void => {
    const current = playback.getSnapshot()
    if (current.phase !== 'loading') return
    playback.set({ ...current, phase: 'playing' })
  }

  /** Publish the conversation identity and its blank flag. */
  const publishSession = (): void => {
    const binding = bindingSource.getSnapshot()
    const face = binding?.hooks?.session
    const snapshot = face?.getSnapshot?.()
    const key = typeof binding?.key === 'string' && binding.key !== '' ? binding.key : null
    const blank = snapshot !== null && typeof snapshot === 'object' && snapshot.blank === true
    session.set({ sessionId: key, blank })
  }

  /**
   * Follow the current conversation. The session face is itself observable - a
   * conversation is created blank and settles later - so it is subscribed to,
   * but only when the face changes, which is what keeps the resubscription from
   * feeding itself.
   */
  const syncSession = (): void => {
    const face = bindingSource.getSnapshot()?.hooks?.session
    if (face !== mirroredFace) {
      mirroredFace = face
      nestedStop?.()
      nestedStop = null
      if (typeof face?.subscribe === 'function') {
        const subscription = face.subscribe(() => { publishSession() })
        nestedStop = typeof subscription === 'function' ? subscription as () => void : null
      }
    }
    publishSession()
  }

  /** Ask the Host for the clip that should play and point the overlay at it. */
  const play = async (reason: IntroReason, sessionId: string | null, previewClipId?: string): Promise<void> => {
    if (inFlight) return
    if (previewClipId !== undefined) {
      const clip = catalog.getSnapshot().clips.find(candidate => candidate.id === previewClipId)
      if (clip === undefined) return
      playback.set({ phase: 'loading', clipId: clip.id, url: mediaUrlFor(clip), reason: 'preview', nonce: playback.getSnapshot().nonce + 1, preview: true })
      return
    }
    inFlight = true
    try {
      /* v8 ignore next -- decidePlay never yields "random" and preview is handled above, so this always asks for "active" */
      const answer = await fetchActive(sessionId, reason === 'random' ? 'random' : 'active', lifetime.signal)
      if (answer === null || answer.clipId === null || answer.version === null) return
      playback.set({
        phase: 'loading',
        clipId: answer.clipId,
        url: mediaUrlFor({ id: answer.clipId, version: answer.version }),
        reason: answer.how === 'none' ? reason : answer.how,
        nonce: playback.getSnapshot().nonce + 1,
        preview: false,
      })
      records.markLaunch()
      if (sessionId !== null) records.markSession(sessionId)
    } catch (error) {
      // A failed resolve leaves the launch record untouched, so a later
      // evaluation retries instead of losing the intro; the failure is already
      // visible as "no clip played".
      void error
    } finally {
      inFlight = false
    }
  }

  /** Apply the when-to-play rule to the current facts. */
  const evaluate = (): void => {
    // No decision before the Host has projected the user's preferences: guessing
    // either way would play against an explicit "off".
    const current = settings.getSnapshot()
    if (current === null) return
    const now = session.getSnapshot()
    const entered = now.sessionId !== lastSessionId
    lastSessionId = now.sessionId
    const decision = decidePlay({
      enabled: current.enabled,
      timing: current.timing,
      sessionId: now.sessionId,
      blank: now.blank,
      entered,
      playedThisLaunch: records.playedLaunch(),
      replay: catalog.getSnapshot().session.replay,
      playedThisSession: now.sessionId === null ? false : records.playedSession(now.sessionId),
    })
    if (decision.action === 'play') void play(decision.reason, now.sessionId)
  }

  /** Reload the clip list and this conversation's override. */
  const refresh = (): void => {
    void (async () => {
      try {
        const answer = await fetchCatalog(session.getSnapshot().sessionId, lifetime.signal)
        catalog.set({
          clips: answer.clips,
          session: answer.session,
          loading: false,
          error: null,
          overridesAvailable: answer.overridesAvailable,
        })
      } catch (error: unknown) {
        catalog.set({ ...catalog.getSnapshot(), loading: false, error: String(error) })
      }
    })()
  }

  /** Write one preference through the Host-backed settings form. */
  const patchSetting = (key: keyof IntroSettings, value: boolean | string): void => {
    settings.set({ ...(settings.getSnapshot() ?? DEFAULT_SETTINGS), [key]: value })
    void form.set(key, value).catch(() => undefined)
  }

  const writeSession = (patch: { replay?: boolean; clipId?: string | null }): void => {
    const sessionId = session.getSnapshot().sessionId
    if (sessionId === null) return
    const before = catalog.getSnapshot()
    catalog.set({
      ...before,
      session: {
        replay: patch.replay ?? before.session.replay,
        clipId: patch.clipId === undefined ? before.session.clipId : patch.clipId,
      },
    })
    void writeSessionOverride(sessionId, patch, lifetime.signal)
      .then((stored) => { catalog.set({ ...catalog.getSnapshot(), session: stored }) })
      .catch(() => undefined)
  }

  // Settings arrive from the Host and change under us when another window
  // writes them, so the projection is mirrored rather than sampled once.
  ctx.effect(() => {
    const mirror = (): void => {
      settings.set(settingsOf(form))
      // The projection can arrive before OR after apply; deciding on both edges
      // is what keeps the launch rule from silently never running.
      evaluate()
    }
    return form.subscribe(mirror)
  }, 'boot-animation: settings mirror')
  ctx.effect(() => bindingSource.subscribe(() => { syncSession(); refresh(); evaluate() }), 'boot-animation: conversation')
  ctx.effect(() => () => { lifetime.abort() }, 'boot-animation: requests')
  syncSession()
  refresh()
  // The projection may have become ready between construction and here, and the
  // subscription only covers later arrivals.
  settings.set(settingsOf(form))
  evaluate()

  /** Render code never sees the "not projected yet" state; it renders the defaults. */
  const deriveSettings = (source: typeof settings) => ({
    getSnapshot: () => source.getSnapshot() ?? DEFAULT_SETTINGS,
    subscribe: (listener: () => void) => source.subscribe(listener),
  })

  ctx.slots.inject('shell.overlay', () => ctx.slots.register({
    name: 'shell.overlay',
    id: 'boot-animation',
    order: 100,
    locale: NS,
    inject: () => ({
      hooks: {
        playback,
        settings: deriveSettings(settings),
      },
      markPlaying,
      stop,
    }),
  }, IntroOverlay))

  ctx.slots.inject('settings.section', () => ctx.slots.register({
    name: 'settings.section',
    id: 'boot-animation',
    order: 40,
    label: () => t('section.label'),
    locale: NS,
    inject: () => ({
      hooks: { catalog, settings: deriveSettings(settings), session },
      refresh,
      preview: (clipId: string) => { void play('preview', session.getSnapshot().sessionId, clipId) },
      setEnabled: (enabled: boolean) => { patchSetting('enabled', enabled) },
      setTiming: (timing: PlayTiming) => { patchSetting('timing', timing) },
      setFit: (fit: FitMode) => { patchSetting('fit', fit) },
      setRandom: (random: boolean) => { patchSetting('random', random) },
      setSelectedClip: (clipId: string) => { patchSetting('selectedClip', clipId) },
      setSessionReplay: (replay: boolean) => { writeSession({ replay }) },
      setSessionClip: (clipId: string | null) => { writeSession({ clipId }) },
      sessionStorageAvailable: catalog.getSnapshot().overridesAvailable,
    }),
  }, SettingsSection))
}

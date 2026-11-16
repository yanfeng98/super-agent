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
import { DEFAULT_SETTINGS, normalizeIntroSettings } from './contract.ts'
import type { CatalogState, IntroReason, IntroSettings, PlaybackState, StartupSessionBinding } from './contract.ts'
import { createSource } from './source.ts'
import { de, en, es, fr, ja, pt, ru, zh } from './locales.ts'
import type { FitMode, PlayTiming, ScreenSettings, SequenceMode } from '../types.ts'

/** One stage of the running plan, carrying everything that stage needs. */
type PlanStage =
  | { readonly kind: 'archive' }
  | { readonly kind: 'video'; readonly clipId: string; readonly url: string; readonly how: IntroReason }

/** Stable Cordis plugin name. */
export const name = 'dsh-client-ui-boot-animation'

/** Services this plugin reads; without them it has nothing to attach to. */
export const inject = ['slots', 'locale', 'uiSession', 'configForms']

/** Profile entry id owning the preferences. */
const SETTINGS_ID = 'ui-boot-animation'

/** Nothing is playing. */
const IDLE: PlaybackState = {
  phase: 'idle', stage: 'none', clipId: null, url: null, reason: 'none', nonce: 0, preview: false,
}

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

/** Read the settings projection's current value, normalized against the shared table. */
function settingsOf(form: { getSnapshot: () => { value: IntroSettings | undefined } }): IntroSettings | null {
  const value = form.getSnapshot().value
  return value === undefined ? null : normalizeIntroSettings(value)
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
  const session = createSource<StartupSessionBinding>({ sessionId: null, blank: false })
  const records = new IntroRecords(sessionStorageOrNull())
  const lifetime = new AbortController()
  let lastSessionId: string | null = null
  let mirroredFace: SessionFace | undefined
  let nestedStop: (() => void) | null = null
  let inFlight = false
  /** The stages the current answer asked for, in order, with everything each one needs. */
  let plan: readonly PlanStage[] = []
  /** Index into {@link plan} of the stage on screen. */
  let planIndex = 0
  /** The reason this plan started with, kept for every stage it runs through. */
  let planReason: IntroReason = 'none'
  /** Whether the running plan is the settings page's preview. */
  let planPreview = false

  const stop = (): void => {
    plan = []
    planIndex = 0
    planReason = 'none'
    planPreview = false
    playback.set(IDLE)
  }

  /** Put one stage of the plan on screen. */
  const startStage = (index: number): void => {
    const stage = plan[index]
    if (stage === undefined) { stop(); return }
    planIndex = index
    const nonce = playback.getSnapshot().nonce + 1
    if (stage.kind === 'archive') {
      playback.set({ phase: 'playing', stage: 'archive', clipId: null, url: null, reason: planReason, nonce, preview: planPreview })
      return
    }
    playback.set({
      phase: 'loading',
      stage: 'video',
      clipId: stage.clipId,
      url: stage.url,
      reason: planPreview ? 'preview' : stage.how === 'none' ? planReason : stage.how,
      nonce,
      preview: planPreview,
    })
  }

  /** Finish the stage on screen and start the next one. */
  const advance = (): void => { startStage(planIndex + 1) }
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

  /** Ask the Host which stages should play now, then start the first one. */
  const play = async (reason: IntroReason, sessionId: string | null, previewClipId?: string): Promise<void> => {
    if (inFlight) return
    if (previewClipId !== undefined) {
      const clip = catalog.getSnapshot().clips.find(candidate => candidate.id === previewClipId)
      if (clip === undefined) return
      plan = [{ kind: 'video', clipId: clip.id, url: mediaUrlFor(clip), how: 'selected' }]
      planIndex = 0
      planReason = 'preview'
      planPreview = true
      startStage(0)
      return
    }
    inFlight = true
    try {
      /* v8 ignore next -- decidePlay never yields "random" and preview is handled above, so this always asks for "active" */
      const answer = await fetchActive(sessionId, reason === 'random' ? 'random' : 'active', lifetime.signal)
      if (answer === null) return
      const clip = answer.clipId === null || answer.version === null
        ? null
        : { clipId: answer.clipId, url: mediaUrlFor({ id: answer.clipId, version: answer.version }), how: answer.how }
      const stages: PlanStage[] = []
      for (const stage of answer.stages) {
        if (stage === 'archive') { stages.push({ kind: 'archive' }); continue }
        if (clip !== null) stages.push({ kind: 'video', ...clip })
      }
      if (stages.length === 0) return
      plan = stages
      planIndex = 0
      planReason = reason
      planPreview = false
      startStage(0)
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

  /** Play the archive sequence alone, for the settings page's preview button. */
  const previewArchive = (): void => {
    plan = [{ kind: 'archive' }]
    planIndex = 0
    planReason = 'preview'
    planPreview = true
    startStage(0)
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
  const patchSetting = (key: keyof IntroSettings, value: boolean | string | number): void => {
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

  /**
   * Render code never sees the "not projected yet" state; it renders the defaults.
   *
   * The normalized value is cached per projection snapshot: React subscribes with
   * `useSyncExternalStore`, which requires the same reference until the fact
   * changes, and normalizing on every read would hand it a new object forever.
   */
  const deriveSettings = (source: typeof settings) => {
    let mirrored: IntroSettings | null = null
    let normalized: IntroSettings = DEFAULT_SETTINGS
    return {
      getSnapshot: (): IntroSettings => {
        const current = source.getSnapshot()
        if (current !== mirrored) {
          mirrored = current
          normalized = normalizeIntroSettings(current)
        }
        return normalized
      },
      subscribe: (listener: () => void) => source.subscribe(listener),
    }
  }

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
      advance,
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
      previewArchive,
      setEnabled: (enabled: boolean) => { patchSetting('enabled', enabled) },
      setTiming: (timing: PlayTiming) => { patchSetting('timing', timing) },
      setFit: (fit: FitMode) => { patchSetting('fit', fit) },
      setRandom: (random: boolean) => { patchSetting('random', random) },
      setSelectedClip: (clipId: string) => { patchSetting('selectedClip', clipId) },
      setSequence: (sequence: SequenceMode) => { patchSetting('sequence', sequence) },
      setScreen: (key: keyof ScreenSettings, value: string | number | boolean) => { patchSetting(key, value) },
      setSessionReplay: (replay: boolean) => { writeSession({ replay }) },
      setSessionClip: (clipId: string | null) => { writeSession({ clipId }) },
      sessionStorageAvailable: catalog.getSnapshot().overridesAvailable,
    }),
  }, SettingsSection))
}

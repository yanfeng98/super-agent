/**
 * The archive sequence's sound: recorded lines, interface cues, and the speech
 * fallback.
 *
 * Every playback is owned here so the sequence can stop all of it in one call -
 * skipping the screen must not leave a recorded line talking over the app. The
 * browser primitives arrive through {@link AudioFactory} rather than being
 * reached directly: the same engine runs against real media elements in the
 * window and against plain fakes in tests.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/archive/audio
 */
import type { ScreenSettings } from '../../types.ts'

/** The slice of a media element this engine drives. */
export interface MediaElementPort {
  play(): Promise<void> | void
  pause(): void
  volume: number
  currentTime: number
  readonly duration: number
  addEventListener(type: 'ended' | 'error', listener: () => void): void
  removeEventListener(type: 'ended' | 'error', listener: () => void): void
}

/** The slice of the speech synthesizer this engine drives. */
export interface SpeechPort {
  /** Speak one line, reporting through the request's own callbacks. */
  speak(request: SpeechRequest): void
  cancel(): void
  resume(): void
}

/** One line handed to the synthesizer. */
export interface SpeechRequest {
  readonly text: string
  readonly lang: string
  readonly rate: number
  readonly pitch: number
  readonly volume: number
  /** Exact installed voice the user picked, or an empty string to let the browser decide. */
  readonly voiceName: string
  /** The line finished normally. */
  onEnd(): void
  /** The line could not be spoken. */
  onError(): void
}

/** One installed voice. */
export interface SpeechVoicePort {
  readonly name: string
  readonly lang: string
}

/** Browser primitives, absent when this environment has no audio at all. */
export interface AudioFactory {
  /** Create one media element pointed at `src`. */
  element(src: string): MediaElementPort
  /** The speech synthesizer, when the browser has one. */
  speech(): SpeechPort | null
  /** Installed voices, best first is the caller's business. */
  voices(): readonly SpeechVoicePort[]
}

/** One interface cue. */
export type CueName = 'tick' | 'ticks' | 'confirm' | 'chime'

/**
 * Bind the browser's audio primitives.
 * @returns the factory, or null in an environment without media elements.
 */
export function browserAudioFactory(): AudioFactory | null {
  if (typeof Audio === 'undefined') return null
  const synth = typeof speechSynthesis === 'undefined' ? null : speechSynthesis
  const port: SpeechPort | null = synth === null ? null : {
    speak: (request) => {
      const utterance = new SpeechSynthesisUtterance(request.text)
      utterance.lang = request.lang
      utterance.rate = request.rate
      utterance.pitch = request.pitch
      utterance.volume = request.volume
      const picked = synth.getVoices().find(voice => voice.name === request.voiceName)
      if (picked !== undefined) utterance.voice = picked
      utterance.onend = () => { request.onEnd() }
      utterance.onerror = () => { request.onError() }
      synth.speak(utterance)
    },
    cancel: () => { synth.cancel() },
    resume: () => { synth.resume() },
  }
  return {
    element: src => new Audio(src),
    speech: () => port,
    voices: () => (synth === null ? [] : synth.getVoices()),
  }
}

/** Milliseconds one recorded line occupied, and whether it actually played. */
export interface Spoken {
  readonly milliseconds: number
  readonly played: boolean
}

/** The engine the sequence talks to. */
export interface ArchiveAudio {
  /**
   * Play one recorded line and report how long it took.
   * @param name - the shipped file name, without extension.
   * @param fallback - spoken text for the speech path.
   * @returns the occupied time.
   */
  line(name: string | null, fallback: string): Promise<Spoken>
  /** Play one interface cue, when interface sound is on. */
  cue(name: CueName): void
  /** Release every element and stop the synthesizer. */
  stop(): void
}

/** No sound at all: every environment without audio, and every test that does not want it. */
export const SILENT_AUDIO: ArchiveAudio = {
  line: () => Promise.resolve({ milliseconds: 0, played: false }),
  cue: () => undefined,
  stop: () => undefined,
}

/** Longest a recorded line may block the sequence before it is abandoned. */
const LINE_TIMEOUT_MS = 12_000

/**
 * Build the engine for one playback.
 * @param settings - the user's sound preferences.
 * @param factory - browser primitives, or null for silence.
 * @param assetUrl - maps a shipped file name to its URL.
 * @returns the engine; playback is a no-op when audio is unavailable.
 */
export function createArchiveAudio(
  settings: ScreenSettings,
  factory: AudioFactory | null,
  assetUrl: (name: string) => string,
): ArchiveAudio {
  if (factory === null) return SILENT_AUDIO
  /** Every element this playback created, so one call can stop them all. */
  const elements: MediaElementPort[] = []
  let stopped = false

  const playCue = (name: CueName): void => {
    if (!settings.screenSound) return
    const file = name === 'ticks' ? 'sfx-ticks-run' : 'sfx-tick'
    let element: MediaElementPort
    try {
      element = factory.element(assetUrl(file))
    } catch (error) {
      // An environment that cannot hand out an element simply has no cue.
      void error
      return
    }
    element.volume = settings.screenSoundVolume
    elements.push(element)
    try {
      void Promise.resolve(element.play()).catch(() => undefined)
    } catch (error) {
      // A blocked autoplay costs one cue, never the sequence.
      void error
    }
  }

  const spokenBySpeech = (text: string, lang: string): Promise<Spoken> => {
    const speech = factory.speech()
    if (speech === null) return Promise.resolve({ milliseconds: 0, played: false })
    return new Promise<Spoken>((resolve) => {
      let settled = false
      const finish = (): void => {
        if (settled) return
        settled = true
        resolve({ milliseconds: estimateSpeech(text, settings.screenVoiceRate), played: true })
      }
      try {
        speech.speak({
          text,
          lang,
          rate: settings.screenVoiceRate,
          pitch: settings.screenVoicePitch,
          volume: settings.screenVoiceVolume,
          voiceName: pickVoice(factory.voices(), settings.screenVoiceName, lang)?.name ?? '',
          onEnd: finish,
          onError: () => {
            if (settled) return
            settled = true
            resolve({ milliseconds: 0, played: false })
          },
        })
      } catch {
        finish()
      }
    })
  }

  return {
    async line(name, fallback) {
      if (!settings.screenVoice) return { milliseconds: 0, played: false }
      // The hint belongs to the line, not to the caller: an audible line always
      // gets it first when the user asked for one.
      if (settings.screenVoiceCue) playCue('tick')
      if (settings.screenVoiceSource === 'asset' && name !== null) {
        const element = factory.element(assetUrl(name))
        element.volume = settings.screenVoiceVolume
        elements.push(element)
        const played = await playToEnd(element, LINE_TIMEOUT_MS)
        if (played !== null) return played
      }
      // The lead is a pause of the synthesized voice only; a recorded take carries
      // its own room before the first word.
      if (settings.screenVoiceLead > 0) await new Promise<void>((resolve) => { setTimeout(resolve, settings.screenVoiceLead) })
      const lang = settings.screenVoiceLang === 'zh' ? 'zh-CN' : 'en-US'
      return spokenBySpeech(fallback, lang)
    },
    cue: (name) => {
      if (stopped) return
      playCue(name)
    },
    stop: () => {
      stopped = true
      for (const element of elements) {
        try {
          element.pause()
        } catch (error) {
          // A detached element can refuse the call; the element is released anyway.
          void error
        }
        try {
          element.currentTime = 0
        } catch (error) {
          // Rewinding a detached element is equally pointless; the reference is dropped next.
          void error
        }
      }
      elements.length = 0
      try {
        factory.speech()?.cancel()
      } catch (error) {
        // Cancelling an idle synthesizer is allowed to fail silently.
        void error
      }
    },
  }
}

/**
 * Play one element and report when it finished.
 * @param element - the element to play.
 * @param timeoutMs - how long to wait before giving up on it.
 * @returns the occupied time, or null when the element could not play.
 */
function playToEnd(element: MediaElementPort, timeoutMs: number): Promise<Spoken | null> {
  return new Promise<Spoken | null>((resolve) => {
    let settled = false
    let timer: ReturnType<typeof setTimeout> | null = null
    const finish = (value: Spoken | null): void => {
      if (settled) return
      settled = true
      if (timer !== null) clearTimeout(timer)
      element.removeEventListener('ended', onEnded)
      element.removeEventListener('error', onError)
      resolve(value)
    }
    const onEnded = (): void => {
      const seconds = Number.isFinite(element.duration) ? element.duration : 0
      finish({ milliseconds: Math.round(seconds * 1000), played: seconds > 0 })
    }
    const onError = (): void => { finish(null) }
    element.addEventListener('ended', onEnded)
    element.addEventListener('error', onError)
    try {
      void Promise.resolve(element.play()).catch(() => { finish(null) })
    } catch {
      finish(null)
      return
    }
    timer = setTimeout(() => {
      const seconds = Number.isFinite(element.duration) ? element.duration : 0
      finish(seconds > 0 ? { milliseconds: Math.round(seconds * 1000), played: true } : null)
    }, timeoutMs)
  })
}

/**
 * Estimate how long a synthesized line takes, so pacing stays close to the
 * spoken length when the browser does not report it.
 * @param text - the line.
 * @param rate - the configured speech rate.
 * @returns milliseconds, clamped to a plausible range.
 */
export function estimateSpeech(text: string, rate: number): number {
  const characters = Math.max(1, text.length)
  const seconds = characters / (14 * Math.max(0.5, rate))
  return Math.round(Math.min(12_000, Math.max(400, seconds * 1000)))
}

/**
 * Choose the installed voice best matching the preference.
 * @param voices - installed voices.
 * @param preferred - the exact voice name the user picked, or an empty string.
 * @param lang - the language being spoken.
 * @returns the voice to use, or null to let the browser decide.
 */
export function pickVoice(
  voices: readonly SpeechVoicePort[],
  preferred: string,
  lang: string,
): SpeechVoicePort | null {
  if (preferred !== '') {
    const exact = voices.find(voice => voice.name === preferred)
    if (exact !== undefined) return exact
  }
  const prefix = lang.slice(0, 2).toLowerCase()
  return voices.find(voice => voice.lang.toLowerCase().startsWith(prefix)) ?? null
}

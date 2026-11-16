// @vitest-environment jsdom
/**
 * The archive sequence's sound engine: recorded takes, the speech fallback, the
 * cues, and the one call that stops all of it.
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  browserAudioFactory, createArchiveAudio, estimateSpeech, pickVoice,
} from '../src/client/archive/audio.ts'
import type { AudioFactory, MediaElementPort, SpeechPort, SpeechRequest } from '../src/client/archive/audio.ts'
import { SCREEN_DEFAULTS } from '../src/screen.ts'
import type { ScreenSettings } from '../src/types.ts'

/** One media element the engine drove, with what happened to it. */
interface FakeElement extends MediaElementPort {
  played: number
  paused: number
  ended: boolean
}

/** A factory whose elements end on their own or fail, as scripted. */
function factory(options: { readonly fail?: boolean; readonly duration?: number; readonly voice?: SpeechPort | null } = {}) {
  const elements: FakeElement[] = []
  const created = {
    element: (src: string): MediaElementPort => {
      const listeners = new Set<() => void>()
      const element: FakeElement = {
        played: 0,
        paused: 0,
        ended: false,
        volume: 1,
        currentTime: 0,
        duration: options.duration ?? 2,
        play: () => {
          element.played += 1
          if (options.fail === true) return Promise.reject(new Error('blocked'))
          element.ended = true
          for (const listener of [...listeners]) listener()
        },
        pause: () => { element.paused += 1 },
        addEventListener: (_type, listener) => { listeners.add(listener) },
        removeEventListener: (_type, listener) => { listeners.delete(listener) },
      }
      elements.push(element)
      void src
      return element
    },
    speech: () => options.voice ?? null,
    voices: () => [{ name: 'Aria', lang: 'en-US' }, { name: 'Xiaoxiao', lang: 'zh-CN' }],
  }
  const audioFactory: AudioFactory = created
  return { audioFactory, elements }
}

/** A speech port recording what was asked of it. */
function speechPort(behaviour: 'end' | 'error' | 'throw' = 'end') {
  const requests: SpeechRequest[] = []
  let cancelled = 0
  const port: SpeechPort = {
    speak: (request) => {
      requests.push(request)
      if (behaviour === 'throw') throw new Error('no voice')
      if (behaviour === 'end') request.onEnd()
      else request.onError()
    },
    cancel: () => { cancelled += 1 },
    resume: () => undefined,
  }
  return { port, requests, cancelled: () => cancelled }
}

/** Settings with one field overridden. */
function settings(overrides: Partial<ScreenSettings> = {}): ScreenSettings {
  return { ...SCREEN_DEFAULTS, ...overrides }
}

const ASSET = (name: string): string => `/audio/${name}.mp3`

afterEach(() => { vi.useRealTimers() })

describe('createArchiveAudio', () => {
  it('is silent when the environment has no audio', async () => {
    const audio = createArchiveAudio(settings(), null, ASSET)
    await expect(audio.line('line-1', 'hello')).resolves.toEqual({ milliseconds: 0, played: false })
    audio.cue('tick')
    audio.prelude()
    audio.stop()
  })

  it('plays one recorded take and reports its length', async () => {
    const { audioFactory } = factory({ duration: 2 })
    const audio = createArchiveAudio(settings(), audioFactory, ASSET)
    await expect(audio.line('line-1', 'hello')).resolves.toEqual({ milliseconds: 2_000, played: true })
  })

  it('reports nothing spoken when the recorded take cannot play and speech is absent', async () => {
    const { audioFactory } = factory({ fail: true })
    const audio = createArchiveAudio(settings(), audioFactory, ASSET)
    await expect(audio.line('line-1', 'hello')).resolves.toEqual({ milliseconds: 0, played: false })
  })

  it('falls back to speech when the recorded take is missing', async () => {
    const speech = speechPort('end')
    const { audioFactory } = factory({ fail: true })
    const audio = createArchiveAudio(settings(), { ...audioFactory, speech: () => speech.port }, ASSET)
    await expect(audio.line('line-1', 'Access permission required.'))
      .resolves.toMatchObject({ played: true })
    expect(speech.requests[0]).toMatchObject({ lang: 'en-US', text: 'Access permission required.' })
  })

  it('speaks Chinese when that language is chosen', async () => {
    const speech = speechPort('end')
    const { audioFactory } = factory()
    const audio = createArchiveAudio(settings({ screenVoiceSource: 'tts', screenVoiceLang: 'zh' }), { ...audioFactory, speech: () => speech.port }, ASSET)
    await audio.line('line-1', '需要访问许可')
    expect(speech.requests[0]).toMatchObject({ lang: 'zh-CN', text: '需要访问许可' })
  })

  it('reports an unsuccessful speech attempt as nothing spoken', async () => {
    const speech = speechPort('error')
    const { audioFactory } = factory()
    const audio = createArchiveAudio(settings({ screenVoiceSource: 'tts' }), { ...audioFactory, speech: () => speech.port }, ASSET)
    await expect(audio.line(null, 'hello')).resolves.toEqual({ milliseconds: 0, played: false })
  })

  it('survives a synthesizer that throws', async () => {
    const speech = speechPort('throw')
    const { audioFactory } = factory()
    const audio = createArchiveAudio(settings({ screenVoiceSource: 'tts' }), { ...audioFactory, speech: () => speech.port }, ASSET)
    await expect(audio.line(null, 'hello')).resolves.toMatchObject({ played: true })
  })

  it('stays quiet when the user turned spoken lines off', async () => {
    const { audioFactory } = factory()
    const audio = createArchiveAudio(settings({ screenVoice: false }), audioFactory, ASSET)
    await expect(audio.line('line-1', 'hello')).resolves.toEqual({ milliseconds: 0, played: false })
  })

  it('plays interface cues only while interface sound is on', () => {
    const on = factory()
    const audioOn = createArchiveAudio(settings(), on.audioFactory, ASSET)
    audioOn.cue('tick')
    audioOn.cue('ticks')
    expect(on.elements).toHaveLength(2)

    const off = factory()
    const audioOff = createArchiveAudio(settings({ screenSound: false }), off.audioFactory, ASSET)
    audioOff.cue('tick')
    expect(off.elements).toHaveLength(0)
  })

  it('plays the prelude only when asked for, and nothing before a cue', () => {
    const on = factory()
    const audio = createArchiveAudio(settings(), on.audioFactory, ASSET)
    audio.prelude()
    expect(on.elements).toHaveLength(1)
    audio.stop()
    audio.prelude()
    expect(on.elements).toHaveLength(1)

    const off = factory()
    createArchiveAudio(settings({ screenVoiceCue: false }), off.audioFactory, ASSET).prelude()
    expect(off.elements).toHaveLength(0)
  })

  it('stops every element and the synthesizer in one call, and again on a second call', () => {
    const speech = speechPort('end')
    const { audioFactory, elements } = factory()
    const audio = createArchiveAudio(settings({ screenVoiceSource: 'tts' }), { ...audioFactory, speech: () => speech.port }, ASSET)
    audio.cue('tick')
    audio.stop()
    expect(speech.cancelled()).toBe(1)
    expect(elements.some(element => element.paused > 0)).toBe(true)
    audio.stop()
    expect(speech.cancelled()).toBe(2)
  })

  it('gives up on a take that never ends', async () => {
    vi.useFakeTimers()
    const listeners = new Set<() => void>()
    const stuck: MediaElementPort = {
      play: () => Promise.resolve(),
      pause: () => undefined,
      volume: 1,
      currentTime: 0,
      duration: 1,
      addEventListener: (_type, listener) => { listeners.add(listener) },
      removeEventListener: (_type, listener) => { listeners.delete(listener) },
    }
    const audio = createArchiveAudio(settings(), { element: () => stuck, speech: () => null, voices: () => [] }, ASSET)
    const pending = audio.line('line-1', 'hello')
    await vi.advanceTimersByTimeAsync(12_000)
    await expect(pending).resolves.toEqual({ milliseconds: 1_000, played: true })
  })

  it('treats a take whose play throws synchronously as unavailable', async () => {
    const element: MediaElementPort = {
      play: () => { throw new Error('nope') },
      pause: () => undefined,
      volume: 1,
      currentTime: 0,
      duration: 1,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }
    const audio = createArchiveAudio(settings(), { element: () => element, speech: () => null, voices: () => [] }, ASSET)
    await expect(audio.line('line-1', 'hello')).resolves.toEqual({ milliseconds: 0, played: false })
  })

  it('reports a take that ends with no known duration as not played', async () => {
    const listeners = new Set<() => void>()
    const element: MediaElementPort = {
      play: () => { for (const listener of [...listeners]) listener(); return Promise.resolve() },
      pause: () => undefined,
      volume: 1,
      currentTime: 0,
      duration: Number.NaN,
      addEventListener: (_type, listener) => { listeners.add(listener) },
      removeEventListener: (_type, listener) => { listeners.delete(listener) },
    }
    const audio = createArchiveAudio(settings(), { element: () => element, speech: () => null, voices: () => [] }, ASSET)
    await expect(audio.line('line-1', 'hello')).resolves.toEqual({ milliseconds: 0, played: false })
  })

  it('ignores a stop that arrives while a take is being prepared', async () => {
    const { audioFactory } = factory({ fail: true })
    const audio = createArchiveAudio(settings(), audioFactory, ASSET)
    const pending = audio.line('line-1', 'hello')
    audio.stop()
    await expect(pending).resolves.toEqual({ milliseconds: 0, played: false })
  })
})

describe('cues and failure paths', () => {
  it('ignores a cue whose playback is refused asynchronously', async () => {
    const element = {
      play: () => Promise.reject(new Error('blocked')),
      pause: () => undefined,
      volume: 1,
      currentTime: 0,
      duration: 1,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }
    const audio = createArchiveAudio(settings(), { element: () => element, speech: () => null, voices: () => [] }, ASSET)
    audio.cue('tick')
    await Promise.resolve()
    audio.stop()
  })

  it('ignores a cue whose playback throws, and any cue after a stop', () => {
    const element = {
      play: () => { throw new Error('blocked') },
      pause: () => undefined,
      volume: 1,
      currentTime: 0,
      duration: 1,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }
    const audio = createArchiveAudio(settings(), { element: () => element, speech: () => null, voices: () => [] }, ASSET)
    audio.cue('tick')
    audio.stop()
    audio.cue('ticks')
  })

  it('survives an element that refuses to pause or rewind', () => {
    const element = {
      play: () => Promise.resolve(),
      pause: () => { throw new Error('detached') },
      volume: 1,
      get currentTime(): number { return 1 },
      set currentTime(_value: number) { throw new Error('detached') },
      duration: 1,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }
    const audio = createArchiveAudio(settings(), { element: () => element, speech: () => null, voices: () => [] }, ASSET)
    audio.cue('tick')
    audio.stop()
  })

  it('survives a synthesizer that refuses to cancel', () => {
    const speech: SpeechPort = {
      speak: () => undefined,
      cancel: () => { throw new Error('idle') },
      resume: () => undefined,
    }
    const audio = createArchiveAudio(settings(), { element: () => { throw new Error('unused') }, speech: () => speech, voices: () => [] }, ASSET)
    audio.stop()
  })

  it('answers nothing when speech is chosen but the browser has no synthesizer', async () => {
    const { audioFactory } = factory()
    const audio = createArchiveAudio(settings({ screenVoiceSource: 'tts' }), { ...audioFactory, speech: () => null }, ASSET)
    await expect(audio.line('line-1', 'hello')).resolves.toEqual({ milliseconds: 0, played: false })
  })

  it('answers nothing when a take is refused asynchronously and speech is absent', async () => {
    const element = {
      play: () => Promise.reject(new Error('blocked')),
      pause: () => undefined,
      volume: 1,
      currentTime: 0,
      duration: 1,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }
    const audio = createArchiveAudio(settings(), { element: () => element, speech: () => null, voices: () => [] }, ASSET)
    await expect(audio.line('line-1', 'hello')).resolves.toEqual({ milliseconds: 0, played: false })
  })
})

describe('synth voice selection and utterance settling', () => {
  it('passes an empty voice name when no installed voice matches', async () => {
    const requests: SpeechRequest[] = []
    const speech: SpeechPort = {
      speak: (request) => { requests.push(request); request.onEnd() },
      cancel: () => undefined,
      resume: () => undefined,
    }
    const audio = createArchiveAudio(
      settings({ screenVoiceSource: 'tts' }),
      { element: () => { throw new Error('unused') }, speech: () => speech, voices: () => [] },
      ASSET,
    )
    await audio.line(null, 'hello')
    expect(requests[0]?.voiceName).toBe('')
  })

  it('ignores a second end report from the same line', async () => {
    const speech: SpeechPort = {
      speak: (request) => { request.onEnd(); request.onEnd() },
      cancel: () => undefined,
      resume: () => undefined,
    }
    const audio = createArchiveAudio(
      settings({ screenVoiceSource: 'tts' }),
      { element: () => { throw new Error('unused') }, speech: () => speech, voices: () => [] },
      ASSET,
    )
    await expect(audio.line(null, 'hello')).resolves.toMatchObject({ played: true })
  })

  it('ignores a failure reported after the line already ended', async () => {
    const speech: SpeechPort = {
      speak: (request) => { request.onEnd(); request.onError() },
      cancel: () => undefined,
      resume: () => undefined,
    }
    const audio = createArchiveAudio(
      settings({ screenVoiceSource: 'tts' }),
      { element: () => { throw new Error('unused') }, speech: () => speech, voices: () => [] },
      ASSET,
    )
    await expect(audio.line(null, 'hello')).resolves.toMatchObject({ played: true })
  })

  it('gives up on a take whose duration is unknown when the timeout fires', async () => {
    vi.useFakeTimers()
    const element: MediaElementPort = {
      play: () => Promise.resolve(),
      pause: () => undefined,
      volume: 1,
      currentTime: 0,
      duration: Number.NaN,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }
    const audio = createArchiveAudio(settings(), { element: () => element, speech: () => null, voices: () => [] }, ASSET)
    const pending = audio.line('line-1', 'hello')
    await vi.advanceTimersByTimeAsync(12_000)
    await expect(pending).resolves.toEqual({ milliseconds: 0, played: false })
  })
})

describe('browserAudioFactory', () => {
  class FakeElement {
    volume = 1
    currentTime = 0
    readonly duration = 2
    readonly src: string
    constructor(src: string) { this.src = src }
    play = () => Promise.resolve()
    pause = (): void => undefined
    addEventListener = (): void => undefined
    removeEventListener = (): void => undefined
  }

  class FakeUtterance {
    lang = ''
    rate = 1
    pitch = 1
    volume = 1
    voice: unknown = null
    onend: (() => void) | null = null
    onerror: (() => void) | null = null
    constructor(readonly text: string) {}
  }

  function stubBrowser(options: { readonly speech?: boolean } = {}) {
    const spoken: FakeUtterance[] = []
    const synth = {
      speak: (utterance: FakeUtterance) => { spoken.push(utterance); utterance.onend?.() },
      cancel: vi.fn(),
      resume: vi.fn(),
      getVoices: () => [{ name: 'Aria', lang: 'en-US' }],
    }
    vi.stubGlobal('Audio', FakeElement)
    vi.stubGlobal('SpeechSynthesisUtterance', FakeUtterance)
    if (options.speech === false) vi.stubGlobal('speechSynthesis', undefined)
    else vi.stubGlobal('speechSynthesis', synth)
    return { synth, spoken }
  }

  it('is absent where media elements do not exist', () => {
    vi.stubGlobal('Audio', undefined)
    expect(browserAudioFactory()).toBeNull()
  })

  it('builds a media element for a source', () => {
    stubBrowser()
    const factory = browserAudioFactory()
    expect(factory?.element('line-1.mp3')).toBeInstanceOf(FakeElement)
  })

  it('speaks through the synthesizer, picking the named voice when it is installed', () => {
    const { synth, spoken } = stubBrowser()
    const factory = browserAudioFactory()
    const ended = vi.fn()
    const request = {
      text: 'hello', lang: 'en-US', rate: 1, pitch: 1, volume: 1, voiceName: 'Aria', onEnd: ended, onError: vi.fn(),
    }
    factory?.speech()?.speak(request)
    expect(spoken[0]).toMatchObject({ text: 'hello', lang: 'en-US' })
    expect(spoken[0]?.voice).toEqual({ name: 'Aria', lang: 'en-US' })
    expect(ended).toHaveBeenCalledOnce()
    factory?.speech()?.cancel()
    factory?.speech()?.resume()
    expect(synth.cancel).toHaveBeenCalledOnce()
    expect(synth.resume).toHaveBeenCalledOnce()
    expect(factory?.voices()).toEqual([{ name: 'Aria', lang: 'en-US' }])
  })

  it('reports a failing utterance through its own callback', () => {
    const { synth, spoken } = stubBrowser()
    synth.speak = (utterance: FakeUtterance) => { spoken.push(utterance); utterance.onerror?.() }
    const failed = vi.fn()
    browserAudioFactory()?.speech()?.speak({
      text: 'hello', lang: 'en-US', rate: 1, pitch: 1, volume: 1, voiceName: '', onEnd: vi.fn(), onError: failed,
    })
    expect(failed).toHaveBeenCalledOnce()
  })

  it('offers no speech and no voices when the browser has no synthesizer', () => {
    stubBrowser({ speech: false })
    const factory = browserAudioFactory()
    expect(factory?.speech()).toBeNull()
    expect(factory?.voices()).toEqual([])
  })
})

describe('pickVoice', () => {
  it('prefers the exact installed voice the user picked', () => {
    expect(pickVoice([{ name: 'Aria', lang: 'en-US' }], 'Aria', 'en-US')?.name).toBe('Aria')
  })

  it('falls back to a voice in the spoken language', () => {
    expect(pickVoice([{ name: 'Xiaoxiao', lang: 'zh-CN' }], 'Missing', 'zh-CN')?.name).toBe('Xiaoxiao')
  })

  it('answers null when nothing matches', () => {
    expect(pickVoice([{ name: 'Aria', lang: 'en-US' }], '', 'zh-CN')).toBeNull()
    expect(pickVoice([], '', 'en-US')).toBeNull()
  })
})

describe('estimateSpeech', () => {
  it('grows with the text and shrinks with the rate', () => {
    expect(estimateSpeech('a'.repeat(28), 1)).toBe(2_000)
    expect(estimateSpeech('a'.repeat(28), 2)).toBe(1_000)
  })

  it('stays inside a plausible window', () => {
    expect(estimateSpeech('', 1)).toBe(400)
    expect(estimateSpeech('a'.repeat(10_000), 0.5)).toBe(12_000)
  })
})

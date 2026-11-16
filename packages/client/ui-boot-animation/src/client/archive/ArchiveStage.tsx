/**
 * The archive sequence: six acts of full-window authorization theatre.
 *
 * The visual state lives in one {@link Frame} object so every act is a state
 * patch rather than a chain of direct DOM writes; the timeline is one async
 * function guarded by an abort flag, so skipping, unmounting, and the end of the
 * sequence all tear down through the same path.
 *
 * The stylesheet is a plain global sheet scoped under `.dsh-boot-archive-root`:
 * the sequence paints roughly ninety elements and eleven keyframes, and CSS
 * Modules references would add a name table without enabling any reuse.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/archive/ArchiveStage
 */
import { useEffect, useRef, useState } from 'react'
import type { CSSProperties, ReactNode } from 'react'
import type { IntroSettings } from '../contract.ts'
import { ARCHIVE_STEPS, ARCHIVE_TOTAL, dwellFor, scaleMs } from './script.ts'
import type { ArchiveStep } from './script.ts'
import { WHALE_PATH } from './whale.ts'
import { createArchiveAudio, SILENT_AUDIO, browserAudioFactory } from './audio.ts'
import { audioUrlFor } from '../api.ts'
import './archive.css'

/** Copy this stage needs beyond the fixed script lines. */
export interface ArchiveLabels {
  /** Skip control. */
  readonly skip: string
  /** Keyboard hint under the confirm button. */
  readonly hint: string
  /** Identity card row labels. */
  readonly cardName: string
  readonly cardId: string
  readonly cardLevel: string
  /** `欢迎 {name} 访问`. */
  readonly welcome: (name: string) => string
  /** `· ID CONFIRMED : {id}`. */
  readonly idLine: (id: string) => string
  /** Bottom log while the plinth settles. */
  readonly selfCheck: { readonly zh: string; readonly en: string }
  /** Bottom log before the reveal. */
  readonly finale: { readonly zh: string; readonly en: string }
  /** Small revision line beside the clock. */
  readonly revision: string
  /** Letter-spaced mark under the whale, in the central column. */
  readonly logoMark: string
  /** Welcome act's fixed English lead-in. */
  readonly welcomeTo: string
  /** Letter-spaced mark under the welcome whale. */
  readonly welcomeMark: string
}

/** Everything visible or animation-relevant at one instant. */
export interface Frame {
  /** Black prologue has begun. */
  readonly blackIn: boolean
  /** The black prologue's wordmark tightens. */
  readonly blackTight: boolean
  /** The sweeping light crosses the wordmark. */
  readonly sweepGo: boolean
  /** The wordmark leaves. */
  readonly blackOut: boolean
  /** The overexposed wash covers the window. */
  readonly washOn: boolean
  /** The warm plinth is settling. */
  readonly plateOn: boolean
  /** The watermark is faintly visible. */
  readonly watermarkOn: boolean
  /** The chrome (brand, footer, frame, slats, meta, voice) is in. */
  readonly chromeOn: boolean
  /** Corner counter value, `[ 00 / 06 ] CODE`. */
  readonly stepIndex: number
  readonly stepCode: string
  /** Headline and its mono subtitle, typed in. */
  readonly statusZh: string
  readonly statusEn: string
  /** The caret blinks while a step waits for confirmation. */
  readonly caretOn: boolean
  /** The status block has faded in. */
  readonly statusOn: boolean
  /** The rule under the subtitle is in. */
  readonly ruleOn: boolean
  /** Bottom log line. */
  readonly logZh: string
  readonly logEn: string
  readonly logOn: boolean
  /** Progress bar fill, 0..1. */
  readonly progress: number
  /** The active slat, 0..6. */
  readonly slat: number
  /** The whale mark is drawn. */
  readonly logoOn: boolean
  /** The whale's dotted orbit is visible. */
  readonly dotsOn: boolean
  /** The reticle is visible. */
  readonly reticleOn: boolean
  /** The identity readout is visible. */
  readonly idOn: boolean
  /** The hexadecimal readout is running. */
  readonly hexOn: boolean
  /** Reserved for the scanline's replay key. */
  readonly sweepKey: number
  /** The executable confirm block is visible. */
  readonly actionOn: boolean
  /** The identity card is shown beside the confirm button. */
  readonly cardOn: boolean
  /** The welcome act is visible. */
  readonly welcomeOn: boolean
  /** The welcome words carry the chromatic split. */
  readonly glitchOn: boolean
  /** The central column left the screen. */
  readonly centerOff: boolean
  /** The bloom and streak are running. */
  readonly bloomOn: boolean
  /** The short highlight. */
  readonly flashOn: boolean
  /** The camera pulls back before handing off. */
  readonly handoffOn: boolean
  /** The skip control is visible. */
  readonly skipOn: boolean
}

/** Nothing on screen yet. */
export const INITIAL_FRAME: Frame = {
  blackIn: false,
  blackTight: false,
  sweepGo: false,
  blackOut: false,
  washOn: false,
  plateOn: false,
  watermarkOn: false,
  chromeOn: false,
  stepIndex: 0,
  stepCode: 'BOOT',
  statusZh: '',
  statusEn: '',
  caretOn: false,
  statusOn: false,
  ruleOn: false,
  logZh: '',
  logEn: '',
  logOn: false,
  progress: 0.05,
  slat: 0,
  logoOn: false,
  dotsOn: false,
  reticleOn: false,
  idOn: false,
  hexOn: false,
  sweepKey: 0,
  actionOn: false,
  cardOn: false,
  welcomeOn: false,
  glitchOn: false,
  centerOff: false,
  bloomOn: false,
  flashOn: false,
  handoffOn: false,
  skipOn: false,
}

/** Raised through pending waits when the sequence is skipped or unmounted. */
class SequenceAborted extends Error {}

/** Run the sequence for one mount; `onFinished` fires exactly once. */
export interface ArchiveStageProps {
  readonly settings: IntroSettings
  readonly labels: ArchiveLabels
  readonly onFinished: () => void
  /** Test seam: audio primitives, or null for a silent run. */
  readonly audioFactory?: ReturnType<typeof browserAudioFactory>
}

/** The custom properties the stylesheet reads, set once per mount. */
function stageVars(settings: IntroSettings): CSSProperties {
  return {
    background: 'transparent',
    '--dba-accent': settings.screenAccent,
    '--dba-glitch-a': settings.screenGlitchA,
    '--dba-glitch-b': settings.screenGlitchB,
    '--dba-glass-alpha': String(settings.screenGlassOpacity),
    '--dba-glass-blur': `${String(settings.screenGlassBlur)}px`,
    '--dba-glass-sat': `${String(settings.screenGlassSaturate)}%`,
    '--dba-backdrop-blur': `${String(settings.screenGlassBackdropBlur)}px`,
    '--dba-speed': String(settings.screenSpeed),
  } as CSSProperties
}

/** One character per span, so the stylesheet's staggered fade can reach it. */
function typedChars(text: string, stagger: number): ReactNode[] {
  return Array.from(text).map((character, index) => (
    <span key={index} className="dsh-boot-archive-ch" style={{ transitionDelay: `${String(index * stagger)}ms` }}>{character}</span>
  ))
}

/** Clock text for one instant. */
function clockText(now: Date): string {
  const pad = (value: number): string => String(value).padStart(2, '0')
  return `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`
}

/** One random hexadecimal row. */
function hexRow(): string {
  let row = ''
  for (let index = 0; index < 26; index += 1) {
    row += '0123456789ABCDEF'.charAt(Math.floor(Math.random() * 16))
    if (index % 2 === 1) row += ' '
  }
  return row
}

/**
 * Render and run the archive sequence.
 * @param props - settings, localized chrome, and the finish callback.
 * @returns the overlay tree.
 */
export function ArchiveStage({ settings, labels, onFinished, audioFactory }: ArchiveStageProps): ReactNode {
  const [frame, setFrame] = useState<Frame>(INITIAL_FRAME)
  const [clock, setClock] = useState(() => clockText(new Date()))
  const [hex, setHex] = useState<readonly string[]>([])
  const confirmRef = useRef<(() => void) | null>(null)
  /** The sequence's own skip control; the confirm button uses {@link confirmRef}. */
  const skipRef = useRef<(() => void) | null>(null)
  const finishRef = useRef(onFinished)
  finishRef.current = onFinished

  useEffect(() => {
    const timer = window.setInterval(() => { setClock(clockText(new Date())) }, 1000)
    return () => { window.clearInterval(timer) }
  }, [])

  // The sequence owns the window while it runs: the page behind it must not scroll
  // under the overlay. The previous value is restored so an app that manages its
  // own overflow is left exactly as it was.
  useEffect(() => {
    const root = document.documentElement
    const previous = root.style.overflow
    root.style.overflow = 'hidden'
    return () => { root.style.overflow = previous }
  }, [])

  useEffect(() => {
    const speed = settings.screenSpeed
    const factory = audioFactory === undefined ? browserAudioFactory() : audioFactory
    const audio = factory === null ? SILENT_AUDIO : createArchiveAudio(settings, factory, audioUrlFor)
    let aborted = false
    let finished = false
    const timers = new Set<number>()
    /** Rejections for the waits a skip must cancel. */
    const waiting = new Set<(reason: unknown) => void>()
    const patch = (next: Partial<Frame>): void => {
      setFrame(previous => ({ ...previous, ...next }))
    }
    const sleep = (milliseconds: number): Promise<void> => new Promise((resolve, reject) => {
      const handle = window.setTimeout(() => {
        timers.delete(handle)
        waiting.delete(reject)
        resolve()
      }, Math.max(0, scaleMs(milliseconds, speed)))
      timers.add(handle)
      waiting.add(reject)
    })
    const type = async (text: string, perCharacter: number, apply: (value: string) => void): Promise<void> => {
      apply('')
      for (let index = 1; index <= text.length; index += 1) {
        await sleep(perCharacter)
        apply(text.slice(0, index))
      }
    }
    const finish = (): void => {
      /* v8 ignore next 2 -- skip and the end of the sequence both guard on `finished`; this keeps a future third caller safe. */
      if (finished) return
      finished = true
      audio.stop()
      finishRef.current()
    }
    /** Drop every pending wait; the one catch at the bottom turns that into a quiet end. */
    const releaseWaits = (): void => {
      for (const reject of [...waiting]) reject(new SequenceAborted())
      waiting.clear()
      for (const handle of timers) window.clearTimeout(handle)
      timers.clear()
      confirmRef.current = null
    }
    const stopSequence = (): void => {
      if (aborted || finished) return
      aborted = true
      releaseWaits()
      audio.stop()
      finish()
    }
    skipRef.current = stopSequence
    const waitConfirm = (): Promise<void> => new Promise((resolve, reject) => {
      if (!settings.screenRequireInteraction) { resolve(); return }
      waiting.add(reject)
      confirmRef.current = () => {
        waiting.delete(reject)
        confirmRef.current = null
        resolve()
      }
    })
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === 'Escape' && settings.screenAllowSkip) { stopSequence(); return }
      if (event.key === 'Enter' || event.key === ' ' || event.key === 'Spacebar') {
        if (confirmRef.current !== null) {
          event.preventDefault()
          confirmRef.current()
        }
      }
    }
    window.addEventListener('keydown', onKey)

    const runStep = async (step: ArchiveStep, index: number): Promise<void> => {
      const number = index + 1
      patch({
        stepIndex: number,
        stepCode: step.badge,
        slat: number,
        progress: step.progress,
        logZh: step.log,
        logEn: step.logEn,
        logOn: true,
        idOn: index < 2,
        caretOn: step.action !== 'auto',
        statusOn: false,
        logoOn: step.assembleLogo === true ? true : step.reticle === true ? false : frame.logoOn,
        dotsOn: step.assembleLogo === true,
        reticleOn: step.reticle === true,
        hexOn: step.hex === true,
      })
      audio.cue('ticks')
      const spokenPromise = audio.line(step.audio, step.say)
      await type(step.zh, 46, (value) => { patch({ statusZh: value }) })
      patch({ statusOn: true, ruleOn: true })
      await type(step.en, 13, (value) => { patch({ statusEn: value }) })
      if (step.action !== 'auto') {
        patch({ actionOn: true, cardOn: step.action === 'confirm' })
        await sleep(520)
        await waitConfirm()
        audio.cue('confirm')
        patch({ actionOn: false, cardOn: false, caretOn: false })
      }
      const spoken = await spokenPromise
      const dwell = dwellFor(step.hold, spoken.milliseconds, speed)
      await sleep(Math.max(0, dwell - spoken.milliseconds))
      if (step.hex) patch({ hexOn: false })
    }

    const run = async (): Promise<void> => {
      // Act 1: black, then the wordmark, then the sweep.
      await sleep(820)
      patch({ blackIn: true })
      audio.cue('tick')
      await sleep(240)
      patch({ blackTight: true })
      await sleep(880)
      patch({ sweepGo: true })
      await sleep(760)
      // Act 2: overexposure, then the warm plinth.
      patch({ blackOut: true, washOn: true })
      audio.cue('ticks')
      await sleep(260)
      patch({ plateOn: true, watermarkOn: true, washOn: false })
      await sleep(760)
      // Act 3: chrome, counter, first log line.
      patch({
        chromeOn: true,
        stepIndex: 0,
        stepCode: 'BOOT',
        progress: 0.12,
        logZh: labels.selfCheck.zh,
        logEn: labels.selfCheck.en,
        logOn: true,
        skipOn: settings.screenAllowSkip,
        sweepKey: 1,
      })
      await sleep(700)
      // Act 4: the five authorization lines.
      for (const [index, step] of ARCHIVE_STEPS.entries()) await runStep(step, index)
      // Act 5: welcome and the chromatic reveal.
      patch({
        stepIndex: ARCHIVE_TOTAL,
        stepCode: 'WELCOME',
        slat: ARCHIVE_TOTAL,
        progress: 1,
        logZh: labels.finale.zh,
        logEn: labels.finale.en,
        logOn: true,
        statusOn: false,
        ruleOn: false,
        reticleOn: false,
        centerOff: true,
        hexOn: false,
        idOn: false,
      })
      audio.cue('chime')
      await sleep(360)
      patch({ welcomeOn: true })
      void audio.line('line-6', '')
      await sleep(420)
      patch({ glitchOn: true })
      await sleep(640)
      patch({ bloomOn: true })
      await sleep(430)
      patch({ flashOn: true })
      await sleep(90)
      patch({ flashOn: false })
      await sleep(320)
      // Act 6: hand off.
      patch({ handoffOn: true })
      await sleep(780)
      finish()
    }

    void (async () => {
      try {
        await run()
      } catch (error) {
        /* v8 ignore next -- the only rejection is the skip sentinel; a real defect must stay loud. */
        if (!(error instanceof SequenceAborted)) throw error
      }
    })()
    return () => {
      aborted = true
      window.removeEventListener('keydown', onKey)
      releaseWaits()
      audio.stop()
    }
  }, [])

  useEffect(() => {
    if (!frame.hexOn) { setHex([]); return undefined }
    const paint = (): void => { setHex(Array.from({ length: 12 }, () => hexRow())) }
    paint()
    const timer = window.setInterval(paint, 110)
    return () => { window.clearInterval(timer) }
  }, [frame.hexOn])

  const base = (name: string): string => `dsh-boot-archive-${name}`
  const on = (name: string, active: boolean): string => (active ? `${base(name)} dsh-boot-archive-in` : base(name))
  const go = (name: string, active: boolean): string => (active ? `${base(name)} dsh-boot-archive-go` : base(name))
  const fade = (ms: number, easing = 'linear'): string => `opacity ${String(Math.round(ms / settings.screenSpeed))}ms ${easing}`
  const rootClass = [
    'dsh-boot-archive-root',
    settings.screenTheme === 'dark' ? 'dsh-boot-archive-dark' : '',
    settings.screenGlass ? 'dsh-boot-archive-glass' : '',
    frame.bloomOn ? 'dsh-boot-archive-peak' : '',
    frame.handoffOn ? 'dsh-boot-archive-handoff' : '',
  ].filter(Boolean).join(' ')

  return (
    <div
      className={rootClass}
      style={stageVars(settings)}
    >
      <div className={on('vignette', frame.plateOn)} />
      <div className={base('watermark')} style={{ opacity: frame.watermarkOn ? 1 : 0, transition: fade(1100, 'ease') }} />
      <div className="dsh-boot-archive-grain" />
      <div className={base('black')} style={{ opacity: frame.blackOut ? 0 : 1, transition: fade(720, 'ease') }}>
        <div className={`dsh-boot-archive-black-inner${frame.blackIn ? ' dsh-boot-archive-in' : ''}${frame.blackOut ? ' dsh-boot-archive-out' : ''}`}>
          <svg className="dsh-boot-archive-whale" viewBox="0 0 49 44" aria-hidden><path d={WHALE_PATH} /></svg>
          <div className="dsh-boot-archive-black-zh">{settings.screenOrgZh}</div>
          <div className={`dsh-boot-archive-black-en${frame.blackTight ? ' dsh-boot-archive-tighten' : ''}`}>{settings.screenOrgEn}</div>
          <div className="dsh-boot-archive-black-sub">{settings.screenOrgSub}</div>
        </div>
        <div className={`dsh-boot-archive-sweep${frame.sweepGo ? ' dsh-boot-archive-go' : ''}`} />
      </div>
      <div className={base('wash')} style={{ opacity: frame.washOn ? 1 : 0, transition: fade(260) }} />
      <div className={base('plate')} style={{ opacity: frame.plateOn ? 1 : 0, transition: fade(900, 'ease') }} />
      <header className={on('brand', frame.chromeOn)}>
        <div className="dsh-boot-archive-brand-1">{settings.screenBrand1}</div>
        <div className="dsh-boot-archive-brand-2">{settings.screenBrand2}</div>
        <div className="dsh-boot-archive-brand-3"><b>{settings.screenBrand3a}</b><span>{settings.screenBrand3b}</span></div>
      </header>
      <div className={on('stepcount', frame.chromeOn)}>
        {`[ ${String(frame.stepIndex).padStart(2, '0')} / ${String(ARCHIVE_TOTAL).padStart(2, '0')} ] ${frame.stepCode}`}
      </div>
      <div className={`dsh-boot-archive-center${frame.centerOff ? ' dsh-boot-archive-off' : ''}`}>
        <div className={`dsh-boot-archive-status${frame.statusOn ? ' dsh-boot-archive-in' : ''}${frame.caretOn ? ' dsh-boot-archive-zhline' : ''}`}>
          <span className="dsh-boot-archive-status-zh">{typedChars(frame.statusZh, 46)}</span>
          <i className={`dsh-boot-archive-caret${frame.caretOn ? ' dsh-boot-archive-on' : ''}`} />
        </div>
        <div className={`dsh-boot-archive-sub-en${frame.statusOn ? ' dsh-boot-archive-in' : ''}`}>{typedChars(frame.statusEn, 13)}</div>
        <div className={on('rule', frame.ruleOn)} />
        <div className={on('logo', frame.logoOn)}>
          <svg className="dsh-boot-archive-whale" viewBox="0 0 49 44" aria-hidden><path d={WHALE_PATH} /></svg>
          <div className="dsh-boot-archive-logolabel">{labels.logoMark}</div>
        </div>
        <div className="dsh-boot-archive-dots" hidden={!frame.dotsOn} />
      </div>
      <div className={on('idline', frame.idOn)}>{labels.idLine(settings.screenIdentityId)}</div>
      <div className={on('reticle', frame.reticleOn)}>
        <i className="dsh-boot-archive-ring-a" />
        <i className="dsh-boot-archive-ring-b" />
      </div>
      <div className={on('hex', frame.hexOn)}>
        {hex.map((row, index) => <span key={index}>{row}</span>)}
      </div>
      <div className={on('welcome', frame.welcomeOn)}>
        <div className="dsh-boot-archive-w1">{labels.welcomeTo}</div>
        <div className={`dsh-boot-archive-w2${frame.glitchOn ? ' dsh-boot-archive-glitch' : ''}`} data-text={settings.screenWelcomeBox}>
          {settings.screenWelcomeBox}
        </div>
        <div className="dsh-boot-archive-w3">
          <svg className="dsh-boot-archive-whale" viewBox="0 0 49 44" aria-hidden><path d={WHALE_PATH} /></svg>
          <span className="dsh-boot-archive-w3label">{labels.welcomeMark}</span>
        </div>
        <div className={`dsh-boot-archive-w4${frame.glitchOn ? ' dsh-boot-archive-glitch' : ''}`} data-text={labels.welcome(settings.screenIdentity)}>
          {labels.welcome(settings.screenIdentity)}
        </div>
      </div>
      <div className={on('action', frame.actionOn)}>
        <div className="dsh-boot-archive-card" hidden={!frame.cardOn}>
          <div className="dsh-boot-archive-card-row"><span>{labels.cardName}</span><b className="dsh-boot-archive-zhname">{settings.screenIdentity}</b></div>
          <div className="dsh-boot-archive-card-row"><span>{labels.cardId}</span><b>{settings.screenIdentityId}</b></div>
          <div className="dsh-boot-archive-card-row"><span>{labels.cardLevel}</span><b>{settings.screenAccessLevel}</b></div>
        </div>
        <button
          className={`dsh-boot-archive-btn${frame.actionOn ? ' dsh-boot-archive-pulse' : ''}`}
          type="button"
          onClick={() => { confirmRef.current?.() }}
        >
          <span className="dsh-boot-archive-btn-zh">{ARCHIVE_STEPS[frame.stepIndex - 1]?.btn ?? ''}</span>
          <span className="dsh-boot-archive-btn-en">{ARCHIVE_STEPS[frame.stepIndex - 1]?.btnSub ?? ''}</span>
        </button>
        <div className="dsh-boot-archive-hint">{labels.hint}</div>
      </div>
      <div className={on('frame', frame.chromeOn)}>
        <i className="dsh-boot-archive-corner dsh-boot-archive-c-tl" />
        <i className="dsh-boot-archive-corner dsh-boot-archive-c-tr" />
        <i className="dsh-boot-archive-corner dsh-boot-archive-c-bl" />
        <i className="dsh-boot-archive-corner dsh-boot-archive-c-br" />
        <div key={frame.sweepKey} className={`dsh-boot-archive-scanline${frame.chromeOn ? ' dsh-boot-archive-go' : ''}`} />
      </div>
      <div className={on('slats', frame.chromeOn)}>
        {Array.from({ length: ARCHIVE_TOTAL }, (_, index) => (
          <i
            key={index}
            className={`dsh-boot-archive-slat${index + 1 === frame.slat ? ' dsh-boot-archive-on' : ''}${index + 1 < frame.slat ? ' dsh-boot-archive-past' : ''}`}
          />
        ))}
      </div>
      <div className={on('voice', frame.chromeOn && settings.screenSound)}>
        <i /><i /><i /><i /><i />
      </div>
      <div className={on('meta', frame.chromeOn)}>
        <span className="dsh-boot-archive-meta-1">{labels.revision}</span>
        <span className="dsh-boot-archive-meta-2">{clock}</span>
      </div>
      {frame.skipOn
        ? <div className="dsh-boot-archive-skip dsh-boot-archive-in" onClick={() => { skipRef.current?.() }}>{labels.skip}</div>
        : null}
      <footer className={on('foot', frame.chromeOn)}><span>{settings.screenFooter}</span></footer>
      <div className="dsh-boot-archive-subtitle">
        <span className={`dsh-boot-archive-sub-zh${frame.logOn ? ' dsh-boot-archive-in' : ''}`}>{frame.logZh}</span>
        <span className={`dsh-boot-archive-sub-en2${frame.logOn ? ' dsh-boot-archive-in' : ''}`}>{frame.logEn}</span>
      </div>
      <div className="dsh-boot-archive-progress"><i style={{ width: `${String(Math.round(frame.progress * 100))}%` }} /></div>
      <div className={go('bloom', frame.bloomOn)} />
      <div className={go('streak', frame.bloomOn)} />
      <div className={base('flash')} style={{ opacity: frame.flashOn ? 0.72 : 0, transition: fade(frame.flashOn ? 70 : 340, 'ease') }} />
    </div>
  )
}

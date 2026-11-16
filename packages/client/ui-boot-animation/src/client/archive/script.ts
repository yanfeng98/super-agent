/**
 * The archive sequence's script and its timing model.
 *
 * The five authorization lines are fixed bilingual art: a Chinese headline with an
 * English monospace subtitle, the same in every interface language, exactly like
 * the burned-in text of the clips this screen follows. Settings own the brand
 * lines, the identity, and the pacing; they never own these five lines.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/archive/script
 */
import type { AudioName } from '../../audio-names.ts'

/** What a step asks the user to do. */
export type StepAction = 'grant' | 'confirm' | 'auto'

/** One line of the authorization sequence. */
export interface ArchiveStep {
  /** Chinese headline. */
  readonly zh: string
  /** English monospace subtitle, typed character by character. */
  readonly en: string
  /** Spoken line, used when the recorded take is unavailable or TTS is chosen. */
  readonly say: string
  /** Recorded take, or null for a line that only exists as speech. */
  readonly audio: AudioName | null
  /** Bottom log line, Chinese. */
  readonly log: string
  /** Bottom log line, English. */
  readonly logEn: string
  /** Stage code in the corner counter. */
  readonly badge: string
  /** Button label, Chinese. */
  readonly btn: string
  /** Button label, English. */
  readonly btnSub: string
  readonly action: StepAction
  /** Visual dwell in milliseconds before the next line, at 1x. */
  readonly hold: number
  /** Progress bar position after this line. */
  readonly progress: number
  /** Reveal the whale mark during this line. */
  readonly assembleLogo?: boolean
  /** Run the hexadecimal readout during this line. */
  readonly hex?: boolean
  /** Show the reticle during this line. */
  readonly reticle?: boolean
}

/** The five authorization lines, in order. */
export const ARCHIVE_STEPS: readonly ArchiveStep[] = [
  {
    zh: '需要访问许可',
    en: 'ACCESS PERMISSION REQUIRED',
    say: 'Access permission required.',
    audio: 'line-1',
    log: '等待用户授权…',
    logEn: 'AWAITING OPERATOR CONSENT',
    badge: 'AUTHORIZATION',
    btn: '授予访问权限',
    btnSub: 'GRANT ACCESS',
    action: 'grant',
    hold: 900,
    progress: 0.34,
  },
  {
    zh: '身份资料确认',
    en: 'IDENTITY VERIFICATION',
    say: 'Identity verification.',
    audio: 'line-2',
    log: '核对身份资料…',
    logEn: 'MATCHING OPERATOR RECORD',
    badge: 'IDENTITY',
    btn: '确认身份',
    btnSub: 'CONFIRM IDENTITY',
    action: 'confirm',
    hold: 700,
    progress: 0.5,
  },
  {
    zh: '接收访问需求',
    en: 'ACCESS REQUEST RECEIVED',
    say: 'Access request received.',
    audio: 'line-3',
    log: '接收访问需求，建立会话通道…',
    logEn: 'HANDSHAKE ESTABLISHED',
    badge: 'REQUEST',
    action: 'auto',
    btn: '',
    btnSub: '',
    hold: 1200,
    progress: 0.66,
    assembleLogo: true,
  },
  {
    zh: '开始读取权限',
    en: 'PERMISSION READ STARTED',
    say: 'Beginning permission read.',
    audio: 'line-4',
    log: '开始读取权限，索引工作区与会话…',
    logEn: 'READING PERMISSION SET',
    badge: 'READ',
    action: 'auto',
    btn: '',
    btnSub: '',
    hold: 1200,
    progress: 0.8,
    hex: true,
  },
  {
    zh: '读取完毕 · 核验通过',
    en: 'READ COMPLETE · VERIFIED',
    say: 'Read complete. Verification passed.',
    audio: 'line-5',
    log: '读取完毕，核验通过，权限已授予',
    logEn: 'PERMISSION AUTHORIZED',
    badge: 'VERIFY',
    action: 'auto',
    btn: '',
    btnSub: '',
    hold: 1600,
    progress: 0.93,
    reticle: true,
  },
]

/** Lines plus the welcome act, for the corner counter. */
export const ARCHIVE_TOTAL = ARCHIVE_STEPS.length + 1

/**
 * Scale one fixed duration by the user's speed.
 * @param milliseconds - the duration at 1x.
 * @param speed - the configured multiplier.
 * @returns the rounded duration.
 */
export function scaleMs(milliseconds: number, speed: number): number {
  return Math.round(milliseconds / speed)
}

/**
 * How long a line dwells in total: its visual base, or the spoken line plus a
 * tail, whichever is longer.
 * @param hold - the line's visual base at 1x.
 * @param spoken - milliseconds the recorded or spoken line took.
 * @param speed - the configured multiplier.
 * @returns the total dwell in milliseconds.
 */
export function dwellFor(hold: number, spoken: number, speed: number): number {
  return Math.max(scaleMs(hold, speed), spoken + scaleMs(240, speed))
}

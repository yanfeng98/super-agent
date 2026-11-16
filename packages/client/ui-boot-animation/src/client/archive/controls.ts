/**
 * The archive stage's settings controls, as data.
 *
 * One table drives the page: which field each control writes, how it is drawn, and
 * which dictionary keys name it. Adding a setting means adding a row here plus its
 * dictionary entry, not another hand-written block of markup.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/archive/controls
 */
import { SCREEN_RANGES } from '../../screen.ts'
import type { ScreenKey } from '../../screen.ts'
import type { BootAnimationLocaleKey } from '../locales.ts'

/** How one field is edited. */
export type ControlKind = 'toggle' | 'select' | 'range' | 'text' | 'color'

/** One editable row. */
export interface ControlSpec {
  /** The settings field this row writes. */
  readonly key: ScreenKey
  readonly kind: ControlKind
  /** Row label. */
  readonly label: BootAnimationLocaleKey
  /** Optional explanation under the row. */
  readonly hint?: BootAnimationLocaleKey
  /** Closed choices, for `select`. */
  readonly options?: readonly { readonly value: string; readonly label: BootAnimationLocaleKey }[]
  /** Percentage-style display for `range`. */
  readonly unit?: '%'
}

/** One page section. */
export interface ControlGroup {
  /** Stable anchor id, also used by the page's jump list. */
  readonly id: string
  readonly title: BootAnimationLocaleKey
  readonly controls: readonly ControlSpec[]
}

/** Every section, in the order a user meets them. */
export const CONTROL_GROUPS: readonly ControlGroup[] = [
  {
    id: 'screen-play',
    title: 'screen.play',
    controls: [
      {
        key: 'screenTheme',
        kind: 'select',
        label: 'screen.theme.label',
        options: [
          { value: 'light', label: 'screen.theme.light' },
          { value: 'dark', label: 'screen.theme.dark' },
        ],
      },
      { key: 'screenSpeed', kind: 'range', label: 'screen.speed.label', hint: 'screen.speed.hint' },
      {
        key: 'screenRequireInteraction',
        kind: 'toggle',
        label: 'screen.requireInteraction.label',
        hint: 'screen.requireInteraction.hint',
      },
      { key: 'screenAllowSkip', kind: 'toggle', label: 'screen.allowSkip.label', hint: 'screen.allowSkip.hint' },
    ],
  },
  {
    id: 'screen-identity',
    title: 'screen.identity',
    controls: [
      { key: 'screenIdentity', kind: 'text', label: 'screen.identity.label', hint: 'screen.identity.hint' },
      { key: 'screenIdentityId', kind: 'text', label: 'screen.identityId.label' },
      { key: 'screenAccessLevel', kind: 'text', label: 'screen.accessLevel.label' },
    ],
  },
  {
    id: 'screen-voice',
    title: 'screen.voice',
    controls: [
      { key: 'screenVoice', kind: 'toggle', label: 'screen.voice.label' },
      {
        key: 'screenVoiceSource',
        kind: 'select',
        label: 'screen.voiceSource.label',
        options: [
          { value: 'asset', label: 'screen.voiceSource.asset' },
          { value: 'tts', label: 'screen.voiceSource.tts' },
        ],
      },
      {
        key: 'screenVoiceLang',
        kind: 'select',
        label: 'screen.voiceLang.label',
        options: [
          { value: 'en', label: 'screen.voiceLang.en' },
          { value: 'zh', label: 'screen.voiceLang.zh' },
          { value: 'both', label: 'screen.voiceLang.both' },
        ],
      },
      { key: 'screenVoiceName', kind: 'text', label: 'screen.voiceName.label', hint: 'screen.voiceName.hint' },
      { key: 'screenVoiceRate', kind: 'range', label: 'screen.voiceRate.label' },
      { key: 'screenVoicePitch', kind: 'range', label: 'screen.voicePitch.label' },
      { key: 'screenVoiceVolume', kind: 'range', label: 'screen.voiceVolume.label' },
      { key: 'screenVoiceCue', kind: 'toggle', label: 'screen.voiceCue.label' },
      { key: 'screenVoiceLead', kind: 'range', label: 'screen.voiceLead.label', hint: 'screen.voiceLead.hint' },
      { key: 'screenSound', kind: 'toggle', label: 'screen.sound.label' },
      { key: 'screenSoundVolume', kind: 'range', label: 'screen.soundVolume.label' },
      {
        key: 'screenSfx',
        kind: 'select',
        label: 'screen.sfx.label',
        options: [
          { value: 'audio', label: 'screen.sfx.audio' },
          { value: 'synth', label: 'screen.sfx.synth' },
        ],
      },
    ],
  },
  {
    id: 'screen-visual',
    title: 'screen.visual',
    controls: [
      { key: 'screenGlass', kind: 'toggle', label: 'screen.glass.label', hint: 'screen.glass.hint' },
      { key: 'screenGlassOpacity', kind: 'range', label: 'screen.glassOpacity.label' },
      { key: 'screenGlassBlur', kind: 'range', label: 'screen.glassBlur.label' },
      { key: 'screenGlassSaturate', kind: 'range', label: 'screen.glassSaturate.label' },
      { key: 'screenGlassBackdropBlur', kind: 'range', label: 'screen.glassBackdropBlur.label' },
      { key: 'screenAccent', kind: 'color', label: 'screen.accent.label' },
      { key: 'screenGlitchA', kind: 'color', label: 'screen.glitchA.label' },
      { key: 'screenGlitchB', kind: 'color', label: 'screen.glitchB.label' },
    ],
  },
  {
    id: 'screen-text',
    title: 'screen.text',
    controls: [
      { key: 'screenBrand1', kind: 'text', label: 'screen.brand1.label' },
      { key: 'screenBrand2', kind: 'text', label: 'screen.brand2.label' },
      { key: 'screenBrand3a', kind: 'text', label: 'screen.brand3a.label' },
      { key: 'screenBrand3b', kind: 'text', label: 'screen.brand3b.label' },
      { key: 'screenOrgZh', kind: 'text', label: 'screen.orgZh.label' },
      { key: 'screenOrgEn', kind: 'text', label: 'screen.orgEn.label' },
      { key: 'screenOrgSub', kind: 'text', label: 'screen.orgSub.label' },
      { key: 'screenWelcomeBox', kind: 'text', label: 'screen.welcomeBox.label' },
      { key: 'screenFooter', kind: 'text', label: 'screen.footer.label' },
    ],
  },
]

/**
 * The bounds of one numeric control.
 * @param key - the field being edited.
 * @returns the inclusive bounds; an unbounded field falls back to a unit range.
 */
export function boundsOf(key: ScreenKey): readonly [number, number] {
  return SCREEN_RANGES[key] ?? [0, 1]
}

/**
 * The choices of one select control.
 * @param spec - the control being rendered.
 * @returns the options, empty when the table declared none.
 */
export function optionsOf(spec: ControlSpec): NonNullable<ControlSpec['options']> {
  return spec.options ?? []
}

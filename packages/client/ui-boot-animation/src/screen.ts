/**
 * The startup screen's own settings, shared by both faces.
 *
 * One table carries every field's name, default, and bounds so the Host schema,
 * the settings page, and the sequence runtime cannot drift apart. Values arrive
 * from the Host already clamped; this table is what the runtime normalizes
 * against when it renders a projection written by an older or newer Host.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/screen
 */

import type { ScreenSettings } from './types.ts'

/** Every screen field, in settings-page order. */
export const SCREEN_KEYS = [
  'screenTheme',
  'screenSpeed',
  'screenRequireInteraction',
  'screenAllowSkip',
  'screenSound',
  'screenSoundVolume',
  'screenVoice',
  'screenVoiceSource',
  'screenVoiceLang',
  'screenVoiceName',
  'screenVoiceRate',
  'screenVoicePitch',
  'screenVoiceVolume',
  'screenVoiceCue',
  'screenVoiceLead',
  'screenSfx',
  'screenIdentity',
  'screenIdentityId',
  'screenAccessLevel',
  'screenBrand1',
  'screenBrand2',
  'screenBrand3a',
  'screenBrand3b',
  'screenOrgZh',
  'screenOrgEn',
  'screenOrgSub',
  'screenWelcomeBox',
  'screenFooter',
  'screenAccent',
  'screenGlitchA',
  'screenGlitchB',
  'screenGlass',
  'screenGlassOpacity',
  'screenGlassBlur',
  'screenGlassSaturate',
  'screenGlassBackdropBlur',
] as const

/** One field name. */
export type ScreenKey = (typeof SCREEN_KEYS)[number]

/** Values the sequence starts from, and what "reset to default" restores. */
export const SCREEN_DEFAULTS = {
  screenTheme: 'light',
  screenSpeed: 1,
  screenRequireInteraction: true,
  screenAllowSkip: true,
  screenSound: true,
  screenSoundVolume: 0.5,
  screenVoice: true,
  screenVoiceSource: 'asset',
  screenVoiceLang: 'en',
  screenVoiceName: '',
  screenVoiceRate: 0.88,
  screenVoicePitch: 0.85,
  screenVoiceVolume: 0.9,
  screenVoiceCue: true,
  screenVoiceLead: 240,
  screenSfx: 'audio',
  screenIdentity: 'JOYCE MOORE',
  screenIdentityId: '0087',
  screenAccessLevel: '3',
  screenBrand1: 'DEEPSEEK',
  screenBrand2: 'SYNTHESIZE INTELLIGENCE',
  screenBrand3a: 'HARNESS',
  screenBrand3b: 'OS',
  screenOrgZh: '深度求索',
  screenOrgEn: 'DEEPSEEK',
  screenOrgSub: 'DEEPSEEK HARNESS',
  screenWelcomeBox: 'DEEPSEEK HARNESS',
  screenFooter: 'POWERED BY DEEPSEEK',
  screenAccent: '#ff7500',
  screenGlitchA: '#ff3b30',
  screenGlitchB: '#2971b8',
  screenGlass: true,
  screenGlassOpacity: 0.75,
  screenGlassBlur: 22,
  screenGlassSaturate: 160,
  screenGlassBackdropBlur: 28,
} satisfies ScreenSettings

/** Closed value sets, with the first entry as the fallback. */
export const SCREEN_ENUMS: Partial<Record<ScreenKey, readonly string[]>> = {
  screenTheme: ['light', 'dark'],
  screenVoiceSource: ['asset', 'tts'],
  screenVoiceLang: ['en', 'zh', 'both'],
  screenSfx: ['audio', 'synth'],
}

/** Numeric bounds, inclusive. */
export const SCREEN_RANGES: Partial<Record<ScreenKey, readonly [number, number]>> = {
  screenSpeed: [0.35, 3],
  screenSoundVolume: [0, 1],
  screenVoiceRate: [0.5, 2],
  screenVoicePitch: [0, 2],
  screenVoiceVolume: [0, 1],
  screenVoiceLead: [0, 1200],
  screenGlassOpacity: [0.15, 0.95],
  screenGlassBlur: [0, 60],
  screenGlassSaturate: [100, 200],
  screenGlassBackdropBlur: [0, 80],
}

/** A screen-settings projection: any field may be absent or carry an unusable value. */
export type ScreenProjection = { readonly [K in keyof ScreenSettings]?: unknown }

/** The startup sequence's stage orders, shared by the Host schema and the browser. */
export const SEQUENCE_MODES = ['archive-then-video', 'video-then-archive', 'archive-only', 'video-only'] as const

/** Text fields, all clipped to this many characters. */
export const SCREEN_TEXT_LIMIT = 64

/** `#rgb`, `#rrggbb`, or the same lengths with an alpha byte. */
const COLOR_PATTERN = /^#[0-9a-fA-F]{3,8}$/u

/**
 * A string field's text.
 * @param value - the raw value from the projection or a stored record.
 * @param fallback - the value to use when nothing usable arrived.
 * @returns the text to validate.
 */
export function textOf(value: unknown, fallback: string): string {
  if (value === undefined) return fallback
  if (value === null) return fallback
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return fallback
}

/**
 * Whether a value is one of the field's closed choices.
 * @param key - the field.
 * @param value - the candidate.
 * @returns true when the value belongs to the field.
 */
export function isScreenChoice(key: ScreenKey, value: unknown): boolean {
  const choices = SCREEN_ENUMS[key]
  return choices === undefined || (typeof value === 'string' && choices.includes(value))
}

/**
 * Clip and validate one raw value against its field.
 * @param key - the field being normalized.
 * @param value - a value from the Host projection or a stored record.
 * @returns the normalized value, or the default when the input is unusable.
 */
export function normalizeScreenValue(key: ScreenKey, value: unknown): string | number | boolean {
  const fallback = SCREEN_DEFAULTS[key]
  if (typeof fallback === 'boolean') {
    if (value === undefined || value === null) return fallback
    return value === true || value === 'true' || value === 1 || value === '1'
  }
  if (typeof fallback === 'number') {
    const numeric = Number(value)
    if (!Number.isFinite(numeric)) return fallback
    const range = SCREEN_RANGES[key]
    /* v8 ignore next -- every numeric field declares a range; the settings test pins that invariant. */
    if (range === undefined) return numeric
    return Math.min(range[1], Math.max(range[0], numeric))
  }
  const text = textOf(value, fallback)
  const choices = SCREEN_ENUMS[key]
  if (choices !== undefined) return choices.includes(text) ? text : fallback
  if (key === 'screenAccent' || key === 'screenGlitchA' || key === 'screenGlitchB') {
    return COLOR_PATTERN.test(text.trim()) ? text.trim() : fallback
  }
  return text.slice(0, SCREEN_TEXT_LIMIT)
}

/**
 * Read a full set of screen settings from a possibly stale projection.
 * @param raw - the Host projection, or null before one arrives.
 * @returns every screen field, with absent or unusable values replaced by defaults.
 */
export function screenSettingsOf(raw: ScreenProjection | null | undefined): ScreenSettings {
  if (raw === null || raw === undefined) return { ...SCREEN_DEFAULTS }
  const fields = Object.fromEntries(
    SCREEN_KEYS.map(key => [key, normalizeScreenValue(key, raw[key])]),
  )
  return { ...SCREEN_DEFAULTS, ...fields }
}

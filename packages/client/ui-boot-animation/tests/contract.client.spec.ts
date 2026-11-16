/**
 * Preference normalization: a stale projection must never reach the runtime with
 * a missing or unusable field.
 */
import { describe, expect, it } from 'vitest'
import { DEFAULT_SETTINGS, normalizeIntroSettings } from '../src/client/contract.ts'
import type { IntroSettings } from '../src/client/contract.ts'
import { SCREEN_DEFAULTS, screenSettingsOf } from '../src/screen.ts'
import type { ScreenSettings } from '../src/types.ts'

describe('DEFAULT_SETTINGS', () => {
  it('starts from the screen table so the two cannot drift', () => {
    for (const [key, value] of Object.entries(SCREEN_DEFAULTS)) {
      expect(DEFAULT_SETTINGS[key as keyof ScreenSettings]).toEqual(value)
    }
    expect(DEFAULT_SETTINGS.enabled).toBe(true)
    expect(DEFAULT_SETTINGS.sequence).toBe('archive-then-video')
  })
})

describe('screenSettingsOf', () => {
  it('answers every default for no projection at all', () => {
    expect(screenSettingsOf(null)).toEqual(SCREEN_DEFAULTS)
    expect(screenSettingsOf(undefined)).toEqual(SCREEN_DEFAULTS)
    expect(screenSettingsOf({})).toEqual(SCREEN_DEFAULTS)
  })

  it('normalizes the values that did arrive and keeps the rest at their defaults', () => {
    const settings = screenSettingsOf({
      screenTheme: 'dark',
      screenSpeed: 99,
      screenIdentity: 'ADA',
      screenAccent: 'orange',
      screenGlass: false,
    })
    expect(settings.screenTheme).toBe('dark')
    expect(settings.screenSpeed).toBe(3)
    expect(settings.screenIdentity).toBe('ADA')
    expect(settings.screenAccent).toBe(SCREEN_DEFAULTS.screenAccent)
    expect(settings.screenGlass).toBe(false)
    expect(settings.screenFooter).toBe(SCREEN_DEFAULTS.screenFooter)
  })
})

describe('normalizeIntroSettings', () => {
  it('answers the defaults for a missing projection', () => {
    expect(normalizeIntroSettings(null)).toEqual(DEFAULT_SETTINGS)
  })

  it('keeps values that are already usable', () => {
    const settings: IntroSettings = {
      ...DEFAULT_SETTINGS,
      enabled: false,
      timing: 'entry',
      fit: 'contain',
      random: true,
      selectedClip: 'file:mine',
      sequence: 'video-only',
    }
    expect(normalizeIntroSettings(settings)).toEqual(settings)
  })

  it('replaces unusable values with defaults', () => {
    const broken = {
      ...DEFAULT_SETTINGS,
      enabled: 'yes',
      timing: 'later',
      fit: 'stretch',
      random: 1,
      selectedClip: '',
      sequence: 'archive-forever',
    }
    expect(normalizeIntroSettings(broken)).toEqual({
      ...DEFAULT_SETTINGS,
      selectedClip: DEFAULT_SETTINGS.selectedClip,
    })
  })

  it('normalizes the screen fields through the shared table', () => {
    const settings = normalizeIntroSettings({
      ...DEFAULT_SETTINGS,
      screenTheme: 'dark',
      screenSpeed: 0.1,
    })
    expect(settings.screenTheme).toBe('dark')
    expect(settings.screenSpeed).toBe(0.35)
  })
})

/**
 * Field normalization: the browser's defensive pass over a Host projection that
 * may come from an older or newer build.
 */
import { describe, expect, it } from 'vitest'
import {
  SCREEN_DEFAULTS, SCREEN_ENUMS, SCREEN_KEYS, SCREEN_RANGES, SCREEN_TEXT_LIMIT, isScreenChoice, normalizeScreenValue, textOf,
} from '../src/screen.ts'
import type { ScreenKey } from '../src/screen.ts'

describe('screen defaults', () => {
  it('covers every declared key once', () => {
    expect(new Set(SCREEN_KEYS).size).toBe(SCREEN_KEYS.length)
    expect(Object.keys(SCREEN_DEFAULTS).sort()).toEqual([...SCREEN_KEYS].sort())
  })

  it('states a bound for every numeric field', () => {
    const numeric = SCREEN_KEYS.filter(key => typeof SCREEN_DEFAULTS[key] === 'number')
    for (const key of numeric) expect(SCREEN_RANGES[key], key).toBeDefined()
  })

  it('keeps every default inside its own bound', () => {
    for (const [key, range] of Object.entries(SCREEN_RANGES)) {
      const value = SCREEN_DEFAULTS[key as ScreenKey] as number
      expect(value, key).toBeGreaterThanOrEqual(range[0])
      expect(value, key).toBeLessThanOrEqual(range[1])
    }
  })
})

describe('isScreenChoice', () => {
  it('accepts only the closed set of an enumerated field', () => {
    expect(isScreenChoice('screenTheme', 'dark')).toBe(true)
    expect(isScreenChoice('screenTheme', 'sepia')).toBe(false)
    expect(isScreenChoice('screenTheme', 3)).toBe(false)
  })

  it('accepts anything for a field without choices', () => {
    expect(isScreenChoice('screenIdentity', 'anything')).toBe(true)
  })
})

describe('textOf', () => {
  it('answers the fallback for an absent value and the text otherwise', () => {
    expect(textOf(undefined, 'x')).toBe('x')
    expect(textOf(null, 'x')).toBe('x')
    expect(textOf(7, 'x')).toBe('7')
    expect(textOf('  a  ', 'x')).toBe('  a  ')
    expect(textOf(true, 'x')).toBe('true')
    expect(textOf({ nested: 1 }, 'x')).toBe('x')
  })
})

describe('normalizeScreenValue', () => {
  it('coerces booleans from the forms a stored value can take', () => {
    expect(normalizeScreenValue('screenGlass', 'true')).toBe(true)
    expect(normalizeScreenValue('screenGlass', 1)).toBe(true)
    expect(normalizeScreenValue('screenGlass', 'false')).toBe(false)
    expect(normalizeScreenValue('screenGlass', 0)).toBe(false)
    // An absent value keeps the field's default instead of flipping it off.
    expect(normalizeScreenValue('screenGlass', null)).toBe(true)
    expect(normalizeScreenValue('screenGlass', undefined)).toBe(true)
  })

  it('clamps numbers and falls back when they are not finite', () => {
    expect(normalizeScreenValue('screenSpeed', 9)).toBe(3)
    expect(normalizeScreenValue('screenSpeed', 0.1)).toBe(0.35)
    expect(normalizeScreenValue('screenSpeed', 'fast')).toBe(1)
    expect(normalizeScreenValue('screenGlassBlur', '12')).toBe(12)
  })

  it('falls back for an unknown choice and passes a known one through', () => {
    expect(normalizeScreenValue('screenTheme', 'sepia')).toBe('light')
    expect(normalizeScreenValue('screenTheme', 'dark')).toBe('dark')
    expect(normalizeScreenValue('screenVoiceLang', 7)).toBe('en')
  })

  it('keeps a valid colour and falls back for anything else', () => {
    expect(normalizeScreenValue('screenAccent', '#ff7500')).toBe('#ff7500')
    expect(normalizeScreenValue('screenAccent', '#abc')).toBe('#abc')
    expect(normalizeScreenValue('screenAccent', 'orange')).toBe('#ff7500')
  })

  it('clips free text and strips nothing else', () => {
    const long = 'x'.repeat(200)
    expect(normalizeScreenValue('screenFooter', long)).toHaveLength(SCREEN_TEXT_LIMIT)
    expect(normalizeScreenValue('screenFooter', '  padded  ')).toBe('  padded  ')
    expect(normalizeScreenValue('screenFooter', 42)).toBe('42')
    expect(normalizeScreenValue('screenFooter', null)).toBe(SCREEN_DEFAULTS.screenFooter)
    expect(normalizeScreenValue('screenFooter', undefined)).toBe(SCREEN_DEFAULTS.screenFooter)
    expect(normalizeScreenValue('screenTheme', null)).toBe('light')
  })

  it('names exactly four closed sets', () => {
    expect(Object.keys(SCREEN_ENUMS).sort()).toEqual([
      'screenSfx', 'screenTheme', 'screenVoiceLang', 'screenVoiceSource',
    ])
  })
})

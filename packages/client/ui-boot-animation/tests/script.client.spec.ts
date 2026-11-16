// @vitest-environment jsdom
/**
 * The sequence's fixed script and its pacing arithmetic.
 */
import { describe, expect, it } from 'vitest'
import { ARCHIVE_STEPS, ARCHIVE_TOTAL, dwellFor, scaleMs } from '../src/client/archive/script.ts'
import { archiveLabels } from '../src/client/archive/labels.ts'
import { CONTROL_GROUPS, boundsOf, optionsOf } from '../src/client/archive/controls.ts'
import type { Translate } from '../src/client/archive/labels.ts'

describe('archive script', () => {
  it('has five lines, each fully specified', () => {
    expect(ARCHIVE_STEPS).toHaveLength(5)
    expect(ARCHIVE_TOTAL).toBe(6)
    for (const step of ARCHIVE_STEPS) {
      expect(step.zh.length).toBeGreaterThan(0)
      expect(step.en).toMatch(/^[A-Z0-9 ·]+$/)
      expect(step.hold).toBeGreaterThan(0)
      expect(step.progress).toBeGreaterThan(0)
      expect(step.progress).toBeLessThanOrEqual(1)
    }
  })

  it('waits for the user on exactly the first two lines', () => {
    expect(ARCHIVE_STEPS.map(step => step.action)).toEqual(['grant', 'confirm', 'auto', 'auto', 'auto'])
  })

  it('reveals the whale, the readout, and the reticle on their own lines', () => {
    expect(ARCHIVE_STEPS.filter(step => step.assembleLogo === true)).toHaveLength(1)
    expect(ARCHIVE_STEPS.filter(step => step.hex === true)).toHaveLength(1)
    expect(ARCHIVE_STEPS.filter(step => step.reticle === true)).toHaveLength(1)
  })

  it('raises the progress bar monotonically', () => {
    const positions = ARCHIVE_STEPS.map(step => step.progress)
    expect([...positions].sort((left, right) => left - right)).toEqual(positions)
  })
})

describe('scaleMs', () => {
  it('divides by the speed and rounds', () => {
    expect(scaleMs(900, 1)).toBe(900)
    expect(scaleMs(900, 3)).toBe(300)
    expect(scaleMs(1000, 3)).toBe(333)
  })
})

describe('dwellFor', () => {
  it('keeps the visual base when the line was short', () => {
    expect(dwellFor(900, 200, 1)).toBe(900)
  })

  it('waits out a long spoken line plus the tail', () => {
    expect(dwellFor(700, 3_744, 1)).toBe(3_744 + 240)
  })

  it('scales both the base and the tail', () => {
    expect(dwellFor(900, 200, 3)).toBe(300)
    expect(dwellFor(100, 1_000, 2)).toBe(1_120)
  })
})

describe('archiveLabels', () => {
  it('reads every label through the dictionary', () => {
    const seen: string[] = []
    const t: Translate = (key) => { seen.push(key); return key }
    const labels = archiveLabels(t)
    expect(labels.skip).toBe('archive.skip')
    expect(labels.welcome('Ada')).toBe('archive.welcome')
    expect(labels.idLine('42')).toBe('archive.idLine')
    expect(labels.selfCheck).toEqual({ zh: 'archive.selfCheck.zh', en: 'archive.selfCheck.en' })
    expect(labels.finale).toEqual({ zh: 'archive.finale.zh', en: 'archive.finale.en' })
    expect(labels.logoMark).toBe('archive.logo')
    expect(seen.length).toBeGreaterThanOrEqual(12)
  })
})

describe('control table', () => {
  it('gives every control a label and covers each section once', () => {
    const ids = CONTROL_GROUPS.map(group => group.id)
    expect(new Set(ids).size).toBe(ids.length)
    for (const group of CONTROL_GROUPS) {
      expect(group.controls.length).toBeGreaterThan(0)
      for (const control of group.controls) expect(control.label.length).toBeGreaterThan(0)
    }
  })

  it('declares choices for every select and bounds for every range', () => {
    for (const group of CONTROL_GROUPS) {
      for (const control of group.controls) {
        if (control.kind === 'select') expect(optionsOf(control).length).toBeGreaterThan(1)
        if (control.kind === 'range') expect(boundsOf(control.key)).toHaveLength(2)
      }
    }
  })

  it('falls back to a unit range and no options', () => {
    expect(boundsOf('screenIdentity')).toEqual([0, 1])
    expect(optionsOf({ key: 'screenIdentity', kind: 'text', label: 'screen.identity.label' })).toEqual([])
  })
})

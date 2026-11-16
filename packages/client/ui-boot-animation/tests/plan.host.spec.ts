/**
 * The configured sequence, resolved into the stages the browser plays.
 */
import { describe, expect, it } from 'vitest'
import { stagesOf } from '../src/host/plan.ts'

describe('stagesOf', () => {
  it('keeps the configured order', () => {
    expect(stagesOf('archive-then-video')).toEqual(['archive', 'video'])
    expect(stagesOf('video-then-archive')).toEqual(['video', 'archive'])
  })

  it('runs one stage alone', () => {
    expect(stagesOf('archive-only')).toEqual(['archive'])
    expect(stagesOf('video-only')).toEqual(['video'])
  })
})

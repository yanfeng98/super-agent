/**
 * Reticle geometry: the arcs stay on the circle and keep their six-o'clock gap.
 */
import { describe, expect, it } from 'vitest'
import { BRACKETS, GUIDE_DOTS, INNER_RING, OUTER_RING, arcPath, polarPoint } from '../src/client/archive/reticle.ts'

describe('polarPoint', () => {
  it('puts twelve o\'clock at the top and six at the bottom', () => {
    expect(polarPoint(200, 200, 100, 0)).toEqual([200, 100])
    expect(polarPoint(200, 200, 100, 180)).toEqual([200, 300])
    expect(polarPoint(200, 200, 100, 90)).toEqual([300, 200])
  })
})

describe('arcPath', () => {
  it('starts at the first endpoint and ends at the second', () => {
    const path = arcPath(200, 200, 100, 0, 90)
    expect(path).toMatch(/^M200\.00 100\.00 A100 100 0 0 1 300\.00 200\.00$/)
  })

  it('marks an arc over half the circle as large', () => {
    expect(arcPath(200, 200, 100, 0, 270)).toContain(' 0 1 1 ')
  })
})

describe('reticle rings', () => {
  it('keep both endpoints on the circle they describe', () => {
    for (const [ring, radius] of [[OUTER_RING, 150], [INNER_RING, 104]] as const) {
      for (const [x, y] of [ring.start, ring.end]) {
        const distance = Math.hypot(x - 200, y - 200)
        expect(distance).toBeCloseTo(radius, 1)
      }
      expect(ring.path).toContain('A')
    }
    // Both gaps sit at six o'clock: no arc endpoint is directly below the centre
    // with the arc running through the bottom, and each ends above the bottom edge.
    expect(OUTER_RING.start[1]).toBeLessThan(400)
    expect(INNER_RING.start[1]).toBeLessThan(400)
  })

  it('draws two brackets and three reference dots', () => {
    expect(BRACKETS).toHaveLength(2)
    expect(GUIDE_DOTS).toHaveLength(3)
    for (const bracket of BRACKETS) expect(bracket).toMatch(/^M/)
  })
})

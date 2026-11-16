/**
 * The reticle's arc geometry, in the 400x400 viewBox the stylesheet sizes.
 *
 * Both rings leave their gap at six o'clock, so the bottom subtitle is never
 * covered and the arc ends land where the reference does. The angles are degrees
 * clockwise from twelve o'clock, matching the SVG arc's own direction.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/archive/reticle
 */

/** One arc: its path data and both endpoints. */
export interface ReticleArc {
  /** SVG path data for the arc. */
  readonly path: string
  /** Where the arc starts. */
  readonly start: readonly [number, number]
  /** Where the arc ends. */
  readonly end: readonly [number, number]
}

/**
 * One point on a circle.
 * @param centerX - circle centre, x.
 * @param centerY - circle centre, y.
 * @param radius - circle radius.
 * @param degrees - clockwise degrees from twelve o'clock.
 * @returns the point, rounded to two decimals.
 */
export function polarPoint(centerX: number, centerY: number, radius: number, degrees: number): readonly [number, number] {
  const radians = ((degrees - 90) * Math.PI) / 180
  return [
    Number((centerX + radius * Math.cos(radians)).toFixed(2)),
    Number((centerY + radius * Math.sin(radians)).toFixed(2)),
  ]
}

/**
 * Path data for one clockwise arc.
 * @param centerX - circle centre, x.
 * @param centerY - circle centre, y.
 * @param radius - circle radius.
 * @param fromDegrees - start angle, clockwise from twelve o'clock.
 * @param toDegrees - end angle, clockwise from twelve o'clock.
 * @returns the `M … A …` path data.
 */
export function arcPath(
  centerX: number,
  centerY: number,
  radius: number,
  fromDegrees: number,
  toDegrees: number,
): string {
  const start = polarPoint(centerX, centerY, radius, fromDegrees)
  const end = polarPoint(centerX, centerY, radius, toDegrees)
  const largeArc = toDegrees - fromDegrees > 180 ? 1 : 0
  return `M${start[0].toFixed(2)} ${start[1].toFixed(2)}`
    + ` A${String(radius)} ${String(radius)} 0 ${String(largeArc)} 1 ${end[0].toFixed(2)} ${end[1].toFixed(2)}`
}

/** The outer ring: a 290-degree arc, dot at its end. */
export const OUTER_RING: ReticleArc = {
  path: arcPath(200, 200, 150, 215, 505),
  start: polarPoint(200, 200, 150, 215),
  end: polarPoint(200, 200, 150, 505),
}

/** The inner ring: a 290-degree arc, dot at its start. */
export const INNER_RING: ReticleArc = {
  path: arcPath(200, 200, 104, 200, 490),
  start: polarPoint(200, 200, 104, 200),
  end: polarPoint(200, 200, 104, 490),
}

/** The two fixed brackets at three and nine o'clock. */
export const BRACKETS: readonly string[] = [
  arcPath(200, 200, 62, 118, 242),
  arcPath(200, 200, 62, 298, 62),
]

/** Three reference dots inside the reticle. */
export const GUIDE_DOTS: readonly (readonly [number, number])[] = [[152, 132], [252, 268], [278, 150]]

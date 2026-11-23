export function clamp(n: number, low: number, high: number): number {
  return Math.min(Math.max(n, low), high);
}

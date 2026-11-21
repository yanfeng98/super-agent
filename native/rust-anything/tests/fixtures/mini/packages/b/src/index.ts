import { count } from 'a'

export function spaces(lines: string[]): number[] {
  return lines.map(line => count(line))
}

import { slug } from '@m/a'

export function anchor(title: string, index: number): string {
  return `${index}-${slug(title)}`
}

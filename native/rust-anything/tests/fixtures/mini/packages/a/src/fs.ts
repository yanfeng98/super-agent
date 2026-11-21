import { readFileSync } from 'node:fs'
import { words } from './util.js'

export function wordsIn(path: string): string[] {
  return words(readFileSync(path, 'utf8'))
}

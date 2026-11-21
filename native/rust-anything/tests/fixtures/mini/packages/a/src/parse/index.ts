import { parse } from 'yaml'
import { flatten } from './helpers'

export function load(text: string): Map<string, string> {
  return flatten(parse(text))
}

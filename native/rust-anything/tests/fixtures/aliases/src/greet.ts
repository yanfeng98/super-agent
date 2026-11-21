import { shout } from '~/util/strings'
import { SEP } from '@lib/join'

export function greet(name: string, place: string): string {
  return shout(`hello ${name}`) + SEP + place
}

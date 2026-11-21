import { compile } from 'svelte/compiler'

export function compileComponent(source: string) {
  return compile(source).js.code
}

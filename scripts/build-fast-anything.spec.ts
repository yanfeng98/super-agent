import { describe, expect, it } from 'vitest'

import { cargoOutput, executableName, stagedBinary } from './build-fast-anything.ts'

describe('fast-anything executable staging', () => {
  it('names the executable the crate emits on each platform', () => {
    expect(executableName('linux')).toBe('fast-anything')
    expect(executableName('darwin')).toBe('fast-anything')
    expect(executableName('win32')).toBe('fast-anything.exe')
  })

  it('reads cargo output and writes the staged copy under the vendored crate', () => {
    expect(cargoOutput('linux')).toMatch(/[/\\]native[/\\]fast-anything[/\\]target[/\\]release[/\\]fast-anything$/)
    expect(cargoOutput('win32')).toMatch(/[/\\]target[/\\]release[/\\]fast-anything\.exe$/)
    expect(stagedBinary('linux')).toMatch(/[/\\]native[/\\]fast-anything[/\\]bin[/\\]fast-anything$/)
    expect(stagedBinary('win32')).toMatch(/[/\\]native[/\\]fast-anything[/\\]bin[/\\]fast-anything\.exe$/)
    expect(stagedBinary('linux')).not.toBe(cargoOutput('linux'))
  })
})

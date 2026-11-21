import { describe, expect, it } from 'vitest'

import { cargoOutput, executableName, stagedBinary } from './build-rust-anything.ts'

describe('rust-anything executable staging', () => {
  it('names the executable the crate emits on each platform', () => {
    expect(executableName('linux')).toBe('rust-anything')
    expect(executableName('darwin')).toBe('rust-anything')
    expect(executableName('win32')).toBe('rust-anything.exe')
  })

  it('reads cargo output and writes the staged copy under the vendored crate', () => {
    expect(cargoOutput('linux')).toMatch(/[/\\]native[/\\]rust-anything[/\\]target[/\\]release[/\\]rust-anything$/)
    expect(cargoOutput('win32')).toMatch(/[/\\]target[/\\]release[/\\]rust-anything\.exe$/)
    expect(stagedBinary('linux')).toMatch(/[/\\]native[/\\]rust-anything[/\\]bin[/\\]rust-anything$/)
    expect(stagedBinary('win32')).toMatch(/[/\\]native[/\\]rust-anything[/\\]bin[/\\]rust-anything\.exe$/)
    expect(stagedBinary('linux')).not.toBe(cargoOutput('linux'))
  })
})

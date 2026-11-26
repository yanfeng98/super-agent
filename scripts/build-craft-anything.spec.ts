import { describe, expect, it } from 'vitest'

import { cargoOutput, executableName, stagedBinary } from './build-craft-anything.ts'

describe('craft-anything executable staging', () => {
  it('names the executable the workspace emits on each platform', () => {
    expect(executableName('linux')).toBe('craft-anything-cli')
    expect(executableName('darwin')).toBe('craft-anything-cli')
    expect(executableName('win32')).toBe('craft-anything-cli.exe')
  })

  it('reads cargo output and writes the staged copy under the vendored workspace', () => {
    expect(cargoOutput('linux')).toMatch(/[/\\]native[/\\]craft-anything[/\\]target[/\\]release[/\\]craft-anything-cli$/)
    expect(cargoOutput('win32')).toMatch(/[/\\]target[/\\]release[/\\]craft-anything-cli\.exe$/)
    expect(stagedBinary('linux')).toMatch(/[/\\]native[/\\]craft-anything[/\\]bin[/\\]craft-anything-cli$/)
    expect(stagedBinary('win32')).toMatch(/[/\\]native[/\\]craft-anything[/\\]bin[/\\]craft-anything-cli\.exe$/)
    expect(stagedBinary('linux')).not.toBe(cargoOutput('linux'))
  })
})

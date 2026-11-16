/**
 * The shipped audio catalog: the closed name set, the directory the files live
 * in, and what an absent or empty file answers.
 */
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { AudioCatalog, audioDir } from '../src/host/audio.ts'
import { AUDIO_NAMES, isAudioName } from '../src/audio-names.ts'

const roots: string[] = []

/** A fresh directory holding one named audio file. */
function library(files: Record<string, string>): string {
  const root = mkdtempSync(join(tmpdir(), 'dsh-boot-audio-'))
  roots.push(root)
  for (const [name, content] of Object.entries(files)) writeFileSync(join(root, name), content)
  return root
}

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true })
})

describe('audio names', () => {
  it('lists the six lines and two cues', () => {
    expect(AUDIO_NAMES).toHaveLength(8)
    expect(isAudioName('line-1')).toBe(true)
    expect(isAudioName('sfx-tick')).toBe(true)
    expect(isAudioName('line-7')).toBe(false)
    expect(isAudioName('../line-1')).toBe(false)
  })
})

describe('audioDir', () => {
  it('resolves beside the built entry', () => {
    const dir = audioDir('file:///opt/dsh/packages/client/ui-boot-animation/lib/index.js')
    expect(dir.replaceAll('\\', '/')).toMatch(/ui-boot-animation\/assets\/audio$/)
  })
})

describe('AudioCatalog', () => {
  it('answers the file, its size, and a version derived from size and mtime', () => {
    const dir = library({ 'line-1.mp3': 'abcd' })
    const clip = new AudioCatalog(dir).get('line-1')
    expect(clip?.info).toMatchObject({ id: 'line-1', name: 'line-1', kind: 'builtin', bytes: 4 })
    expect(clip?.info.version).toMatch(/^[0-9a-z]+-[0-9a-z]+$/)
    expect(clip?.path).toBe(join(dir, 'line-1.mp3'))
  })

  it('answers null for a file that is missing or empty', () => {
    const dir = library({ 'line-2.mp3': '' })
    const catalog = new AudioCatalog(dir)
    expect(catalog.get('line-1')).toBeNull()
    expect(catalog.get('line-2')).toBeNull()
  })

  it('answers null when the directory itself is gone', () => {
    expect(new AudioCatalog(join(tmpdir(), 'dsh-boot-audio-missing-dir')).get('line-3')).toBeNull()
  })
})

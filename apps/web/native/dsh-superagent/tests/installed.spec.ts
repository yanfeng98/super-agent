import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { installedRepository } from '../src/installed.ts'

let dir: string
beforeEach(() => { dir = mkdtempSync(join(tmpdir(), 'installed-')) })
afterEach(() => { rmSync(dir, { recursive: true, force: true }) })

describe('installedRepository', () => {
  it('finds the repository that contains a nested installation', () => {
    const repository = join(dir, 'widgets')
    const packageDir = join(repository, 'apps', 'web')
    mkdirSync(join(repository, '.git'), { recursive: true })
    mkdirSync(packageDir, { recursive: true })
    expect(installedRepository(packageDir)).toBe(repository)
  })
  it('counts a linked worktree, whose .git is a file, and the repository root itself', () => {
    const repository = join(dir, 'widgets')
    mkdirSync(repository, { recursive: true })
    writeFileSync(join(repository, '.git'), `gitdir: ${join(dir, 'main', '.git', 'worktrees', 'widgets')}\n`)
    expect(installedRepository(join(repository, 'src'))).toBe(repository)
    expect(installedRepository(repository)).toBe(repository)
  })
  it('answers undefined for an installation outside every repository', () => {
    const outside = join(dir, 'plain')
    mkdirSync(outside, { recursive: true })
    expect(installedRepository(outside)).toBeUndefined()
  })
})

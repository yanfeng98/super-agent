import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

import {
  buildScript,
  compiledEntry,
  dependenciesCurrent,
  installStampPath,
  launcherPath,
  lockfileDigest,
  npmCommand,
} from './build-rea.ts'

describe('rea runtime paths', () => {
  it('names the npm executable on each platform', () => {
    expect(npmCommand('linux')).toBe('npm')
    expect(npmCommand('darwin')).toBe('npm')
    expect(npmCommand('win32')).toBe('npm.cmd')
  })

  it('resolves the launcher the plugin spawns and the entrypoint the build produces', () => {
    expect(launcherPath('/repo/native/rea')).toBe(join('/repo/native/rea', 'scripts', 'rea.mjs'))
    expect(compiledEntry('/repo/native/rea')).toBe(join('/repo/native/rea', 'dist', 'mcp', 'main.js'))
    expect(installStampPath('/repo/native/rea')).toBe(join('/repo/native/rea', 'node_modules', '.dsh-rea-install'))
  })

  it('uses the full upstream build below Node 26 and the compile chain above it', () => {
    expect(buildScript(24)).toBe('build')
    expect(buildScript(25)).toBe('build')
    expect(buildScript(26)).toBe('compile')
  })

  it('digests the vendored lockfile deterministically', () => {
    const first = lockfileDigest()
    expect(first).toMatch(/^[0-9a-f]{64}$/)
    expect(lockfileDigest()).toBe(first)
  })

  it('treats dependencies as current only when the stamp records the lockfile digest', async () => {
    const tree = await mkdtemp(join(tmpdir(), 'dsh-rea-install-'))
    try {
      expect(dependenciesCurrent(tree)).toBe(false)
      await writeFile(join(tree, 'package-lock.json'), '{"lockfileVersion":3}\n')
      expect(dependenciesCurrent(tree)).toBe(false)
      await mkdir(join(tree, 'node_modules'))
      await writeFile(installStampPath(tree), `${lockfileDigest(tree)}\n`)
      expect(dependenciesCurrent(tree)).toBe(true)
      await writeFile(join(tree, 'package-lock.json'), '{"lockfileVersion":3,"changed":true}\n')
      expect(dependenciesCurrent(tree)).toBe(false)
    } finally {
      await rm(tree, { recursive: true, force: true })
    }
  })
})

/** The built-in proactive coding agent (vendored dsh-superagent) ships in the Web bundle, not as an optional install. */

import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const bundleRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const repositoryRoot = resolve(bundleRoot, '../../..')
const manifest = (): {
  dependencies?: Record<string, string>
  files?: string[]
  dsh?: { bundle?: { patch?: string[] } }
} => JSON.parse(readFileSync(resolve(bundleRoot, 'package.json'), 'utf8')) as never

describe('built-in Superagent', () => {
  it('declares the vendored plugin as a Web bundle dependency', () => {
    expect(manifest().dependencies?.['dsh-superagent']).toBe('workspace:*')
  })

  it('mounts the Host plugin, the GitHub Source, the disabled Feishu Source and the two agent presets', () => {
    const patch = readFileSync(resolve(bundleRoot, 'cordis.patch.yml'), 'utf8')
    expect(patch).toMatch(/- id: dsh-superagent\n\s+name: dsh-superagent\n/)
    expect(patch).toMatch(/- id: dsh-superagent-github\n\s+name: dsh-superagent\/github-watch\n/)
    expect(patch).toMatch(/- id: dsh-superagent-feishu\n\s+name: dsh-superagent\/feishu-watch\n\s+disabled: true\n/)
    const patches = manifest().dsh?.bundle?.patch ?? []
    expect(patches).toContain('./presets/superagent-worker.patch.yml')
    expect(patches).toContain('./presets/superagent-router.patch.yml')
    expect(manifest().files).toContain('presets/superagent-worker.patch.yml')
    expect(manifest().files).toContain('presets/superagent-router.patch.yml')
  })

  it('defines the worker preset with 5-second shell and job waits, and the router preset after it', () => {
    const worker = readFileSync(resolve(bundleRoot, 'presets/superagent-worker.patch.yml'), 'utf8')
    expect(worker).toMatch(/id: dsh-superagent-worker\n/)
    expect(worker).toMatch(/id: superagent-bash-sandbox[\s\S]*?timeoutMs: 5000\n\s+maxTimeoutMs: 5000/)
    expect(worker).toMatch(/id: superagent-pwsh-sandbox[\s\S]*?timeoutMs: 5000\n\s+maxTimeoutMs: 5000/)
    expect(worker).toMatch(/waitTimeoutMs: 5000\n\s+maxWaitTimeoutMs: 5000/)
    const router = readFileSync(resolve(bundleRoot, 'presets/superagent-router.patch.yml'), 'utf8')
    expect(router).toMatch(/id: dsh-superagent-router\n/)
    expect(router).toMatch(/order: 91\n/)
  })

  it('ships a native package with Host and browser entries', () => {
    const native = JSON.parse(readFileSync(resolve(repositoryRoot, 'apps/web/native/dsh-superagent/package.json'), 'utf8')) as {
      private?: boolean
      exports?: Record<string, unknown>
      dsh?: { client?: { platform?: string } }
    }
    expect(native.private).toBe(true)
    expect(native.exports?.['.']).toEqual({ types: './lib/index.d.ts', default: './lib/index.js' })
    expect(native.exports?.['./github-watch']).toBe('./lib/github-watch.js')
    expect(native.exports?.['./feishu-watch']).toBe('./lib/feishu-watch.js')
    expect(native.exports?.['./client']).toBe('./lib/client.js')
    expect(native.dsh?.client?.platform).toBe('web')
  })
})

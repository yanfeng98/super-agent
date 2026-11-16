/** The built-in IM bot plugin is part of the shipped Web bundle, not an optional install. */

import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const bundleRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const repositoryRoot = resolve(bundleRoot, '../../..')

describe('built-in IM bots', () => {
  it('declares the vendored plugin as a Web bundle dependency', () => {
    const manifest = JSON.parse(readFileSync(resolve(bundleRoot, 'package.json'), 'utf8')) as {
      dependencies?: Record<string, string>
    }
    expect(manifest.dependencies?.['@xmanrui/dsh-im']).toBe('workspace:*')
  })

  it('mounts the Host and client faces from the Web patch layer', () => {
    const patch = readFileSync(resolve(bundleRoot, 'cordis.patch.yml'), 'utf8')
    expect(patch).toMatch(/- id: dsh-im\n\s+name: '@xmanrui\/dsh-im'/)
  })

  it('ships a native client bundle entry beside the Host entry', () => {
    const manifest = JSON.parse(readFileSync(
      resolve(repositoryRoot, 'apps/web/native/dsh-im/package.json'), 'utf8',
    )) as {
      private?: boolean
      exports?: Record<string, unknown>
      dsh?: { client?: { platform?: string } }
    }
    expect(manifest.private).toBe(true)
    expect(manifest.exports?.['./client']).toBe('./lib/client.js')
    expect(manifest.dsh?.client?.platform).toBe('web')
  })
})

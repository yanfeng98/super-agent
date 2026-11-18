import { spawn } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, expect, it } from 'vitest'
import type { SubprocessRuntime } from '@deepseek-ai/dsh-subprocess'
import { mergeBlockedByCheck, workspaceCheck } from '../src/app/check.ts'

let directory: string
let signal: AbortSignal
beforeEach(() => {
  directory = mkdtempSync(join(tmpdir(), 'check-'))
  signal = new AbortController().signal
})
afterEach(() => { rmSync(directory, { recursive: true, force: true }) })

/** Minimal SubprocessRuntime backed by child_process.spawn, used by `runAsWorker`. */
const subprocess = (): SubprocessRuntime => ({
  spawn: ({ argv, cwd, env }: { argv: readonly string[]; cwd: string; env?: Record<string, string> }) => {
    const child = spawn(argv[0]!, argv.slice(1), { cwd, env: { ...process.env, ...env } })
    const output = { stdout: '', stderr: '' }
    child.stdout.on('data', chunk => { output.stdout += String(chunk) })
    child.stderr.on('data', chunk => { output.stderr += String(chunk) })
    const reader = (name: 'stdout' | 'stderr') => ({ readFrom: () => ({ text: output[name], nextOffset: 0, lossy: false }) })
    const done = new Promise(resolve => child.on('close', code => resolve({ exitCode: code, signal: null })))
    return { done, collected: { stdout: reader('stdout'), stderr: reader('stderr') } }
  },
}) as unknown as SubprocessRuntime

it('reports a passing check with its output and working directory', async () => {
  const checks = await workspaceCheck(subprocess(), directory, 'pwd && printf ok', signal)
  expect(checks).toMatchObject({ command: 'pwd && printf ok', ok: true, exitCode: 0, output: expect.stringContaining('ok') })
  expect(checks.output).toContain(directory)
  expect(checks.at).toMatch(/^\d{4}-\d{2}-\d{2}T/)
})

it('reports a failing check with its exit code and stderr, and passes the given environment', async () => {
  const checks = await workspaceCheck(subprocess(), directory, 'echo "$CHECK_VALUE" >&2; exit 7', signal, { CHECK_VALUE: 'boom' })
  expect(checks).toMatchObject({ ok: false, exitCode: 7 })
  expect(checks.output).toContain('boom')
  const passing = await workspaceCheck(subprocess(), directory, 'test "$CHECK_VALUE" = yes', signal, { CHECK_VALUE: 'yes' })
  expect(passing.ok).toBe(true)
})

it('keeps the end of a long output and says it was truncated', async () => {
  const checks = await workspaceCheck(subprocess(), directory, 'printf "%05000d" 0; printf tail', signal)
  expect(checks.truncated).toBe(true)
  expect(checks.output.length).toBe(4_000)
  expect(checks.output.endsWith('tail')).toBe(true)
})

it('refuses a merge while a configured check has no passing run, and allows it otherwise', () => {
  expect(mergeBlockedByCheck('', undefined)).toBeUndefined()
  expect(mergeBlockedByCheck('pnpm test', undefined)).toMatch(/has not passed/)
  expect(mergeBlockedByCheck('pnpm test', { command: 'pnpm test', ok: false, exitCode: 1, at: '', output: '' }))
    .toMatch(/has not passed/)
  expect(mergeBlockedByCheck('pnpm test', { command: 'pnpm test', ok: true, exitCode: 0, at: '', output: '' })).toBeUndefined()
  // Only a run after this ending counts; a pass from before the ticket was resumed does not gate the new work.
  const endedAt = '2026-11-18T00:00:00.000Z'
  expect(mergeBlockedByCheck('pnpm test', { command: 'pnpm test', ok: true, exitCode: 0, at: '2026-11-17T23:00:00.000Z',
    output: '' }, endedAt)).toMatch(/has not passed/)
  expect(mergeBlockedByCheck('pnpm test', { command: 'pnpm test', ok: true, exitCode: 0, at: endedAt, output: '' }, endedAt))
    .toBeUndefined()
})

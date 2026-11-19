/** Runs the configured workspace check command in a ticket's worktree and records its outcome. */
import type { SubprocessRuntime } from '@deepseek-ai/dsh-subprocess'
import type { NamedCheck, TicketCheck, TicketCheckRun } from '../core/index.ts'
import { CliError } from '../infra/commands.ts'
import { runAsWorker } from '../infra/environment.ts'

/** Longest a check command may run before it counts as failed. */
export const CHECK_TIMEOUT_MS = 10 * 60_000
/** Captured output limit; a failing test run usually states its reason at the end. */
const OUTPUT_LIMIT = 4_000

/** Most checks one deployment may configure, so a merge does not run an unbounded suite. */
export const MAX_CHECKS = 8
/** `name: command` on one line names a check; a line without it is the command and its own name. */
const NAMED = /^([A-Za-z0-9._-]{1,40}):\s+(.+)$/
/**
 * Reads the configured checks, one per line, each optionally `name: command`.
 * @param text - The executor's check setting; empty means no gate.
 * @returns The checks to run, in order, at most `MAX_CHECKS` of them.
 */
export function parseChecks(text: string): NamedCheck[] {
  return text.split('\n').map(line => line.trim()).filter(Boolean).slice(0, MAX_CHECKS).map(line => {
    const named = NAMED.exec(line)
    if (named) return { name: named[1]!, command: named[2]!.trim() }
    // A command without a name is its own label, shortened for the page.
    return { name: line.length > 40 ? `${line.slice(0, 39)}…` : line, command: line }
  })
}
/**
 * Whether a configured check keeps a ticket's branch from merging.
 * @param command - The executor's configured check command; empty means no gate.
 * @param check - The check run recorded on the ticket, if any.
 * @param endedAt - When the ticket ended; a check from before that ending belongs to earlier work and does not count.
 * @returns The reason to refuse the merge, or undefined when the merge may proceed.
 */
export const mergeBlockedByCheck = (command: string, check: TicketCheck | undefined, endedAt?: string): string | undefined => {
  if (!command.trim()) return undefined
  if (check?.ok && (!endedAt || check.at >= endedAt)) return undefined
  return 'The configured workspace check has not passed; run it from the ticket page and fix any failure before merging'
}
/**
 * Runs every configured check in order and combines them: the aggregate passes only when all of them pass, and its output
 * keeps the end of each check's output under its name.
 * @param env - Environment variables the checks read, such as the ticket's execution variables.
 * @returns Each check's own outcome and the aggregate the merge gate and the archive read.
 */
export async function workspaceChecks(subprocess: SubprocessRuntime, directory: string, checks: readonly NamedCheck[],
  signal: AbortSignal, env: Record<string, string> = {}): Promise<{ runs: TicketCheckRun[]; aggregate: TicketCheck }> {
  const runs: TicketCheckRun[] = []
  for (const check of checks) runs.push({ name: check.name, ...await workspaceCheck(subprocess, directory, check.command, signal, env) })
  return { runs, aggregate: aggregateCheck(runs) }
}
/** Combines named runs into the single record the merge gate and the archive read. */
export function aggregateCheck(runs: readonly TicketCheckRun[]): TicketCheck {
  const failed = runs.find(run => !run.ok)
  const joined = runs.map(run => `## ${run.name}\n${run.output}`).join('\n')
  const truncated = joined.length > OUTPUT_LIMIT
  return { command: runs.map(run => run.name).join(', '), ok: !failed, exitCode: failed ? failed.exitCode : 0,
    at: runs.at(-1)?.at ?? new Date().toISOString(), output: truncated ? joined.slice(-OUTPUT_LIMIT) : joined,
    ...truncated ? { truncated } : {} }
}
/**
 * Runs `command` with `sh -c` in `directory` and captures whether it passed, its exit code, and the end of its output.
 * @param env - Environment variables the check command reads, such as the ticket's execution variables.
 * @returns The command's outcome; a timeout or a missing shell is a failed check, not a thrown error.
 */
export async function workspaceCheck(subprocess: SubprocessRuntime, directory: string, command: string, signal: AbortSignal,
  env: Record<string, string> = {}): Promise<TicketCheck> {
  const at = new Date().toISOString()
  const run = await runAsWorker(subprocess, ['sh', '-c', command], directory,
    AbortSignal.any([signal, AbortSignal.timeout(CHECK_TIMEOUT_MS)]), env)
    .then(output => ({ ok: true, exitCode: 0, output }))
    .catch((error: unknown) => ({ ok: false, exitCode: error instanceof CliError ? error.exitCode ?? null : null,
      output: error instanceof Error ? error.message : String(error) }))
  const truncated = run.output.length > OUTPUT_LIMIT
  return { command, at, ...run, output: truncated ? run.output.slice(-OUTPUT_LIMIT) : run.output, ...truncated ? { truncated } : {} }
}

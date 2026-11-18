/** Runs the configured workspace check command in a ticket's worktree and records its outcome. */
import type { SubprocessRuntime } from '@deepseek-ai/dsh-subprocess'
import type { TicketCheck } from '../core/index.ts'
import { CliError } from '../infra/commands.ts'
import { runAsWorker } from '../infra/environment.ts'

/** Longest a check command may run before it counts as failed. */
export const CHECK_TIMEOUT_MS = 10 * 60_000
/** Captured output limit; a failing test run usually states its reason at the end. */
const OUTPUT_LIMIT = 4_000

/**
 * Whether a configured check keeps a ticket's branch from merging.
 * @param command - The executor's configured check command; empty means no gate.
 * @param check - The latest check run on the ticket, if any.
 * @returns The reason to refuse the merge, or undefined when the merge may proceed.
 */
export const mergeBlockedByCheck = (command: string, check: TicketCheck | undefined): string | undefined =>
  !command.trim() || check?.ok ? undefined
    : 'The configured workspace check has not passed; run it from the ticket page and fix any failure before merging'
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

/** Captures the git facts of a ticket's workspace when it ends, so the panel shows what changed without opening the worktree. */
import type { SubprocessRuntime } from '@deepseek-ai/dsh-subprocess'
import type { Ticket, TicketReview } from '../core/index.ts'
import { runAsWorker } from '../infra/environment.ts'

const TIMEOUT_MS = 30_000
/** Longest diff the panel receives; a larger change set is truncated with a marker. */
const DIFF_LIMIT = 200_000
/** Cap each list so a large change set stays readable and bounded in the ticket record. */
const LIMIT = 50
const run = async (subprocess: SubprocessRuntime, directory: string, args: string[], signal: AbortSignal): Promise<string | undefined> =>
  runAsWorker(subprocess, ['git', ...args], directory, AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
    .then(output => output.trim()).catch(() => undefined)

/** Reads the current commit of a workspace; undefined when it is not a git repository or git is unavailable. */
export const workspaceHead = (subprocess: SubprocessRuntime, directory: string, signal: AbortSignal): Promise<string | undefined> =>
  run(subprocess, directory, ['rev-parse', 'HEAD'], signal)

/**
 * Reads the committed diff of a ticket workspace since `base`, for the panel's on-demand view.
 * @returns The diff, truncated when it exceeds the panel limit.
 * @throws when git fails, such as in a workspace that is not a repository.
 */
export async function workspaceDiff(subprocess: SubprocessRuntime, directory: string, base: string, signal: AbortSignal): Promise<{ diff: string; truncated: boolean }> {
  const output = await runAsWorker(subprocess, ['git', 'diff', '--no-color', '--unified=3', `${base}..HEAD`], directory,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
  return output.length > DIFF_LIMIT ? { diff: output.slice(0, DIFF_LIMIT), truncated: true } : { diff: output, truncated: false }
}
/**
 * Collects a ticket workspace's branch, head, commits since the recorded base, changed files, and uncommitted files.
 * @returns undefined when the ticket has no workspace; a non-git workspace keeps `capturedAt` with the reason in `error`.
 * @param ticket - The ticket whose `execution.directory` and `execution.base` are read.
 * @param signal - Aborted when the host stops, cancelling the git subprocesses.
 */
export async function captureReview(subprocess: SubprocessRuntime, ticket: Ticket, signal: AbortSignal): Promise<TicketReview | undefined> {
  const directory = ticket.execution?.directory
  if (!directory) return undefined
  const capturedAt = new Date().toISOString()
  const [branch, head] = await Promise.all([
    run(subprocess, directory, ['rev-parse', '--abbrev-ref', 'HEAD'], signal),
    workspaceHead(subprocess, directory, signal),
  ])
  if (!head) return { capturedAt, commits: [], files: [], dirty: [], error: 'The workspace is not a git repository, or git could not read it' }
  const base = ticket.execution?.base
  const range = base && base !== head ? `${base}..HEAD` : ''
  const [log, files, stat, status] = await Promise.all([
    range ? run(subprocess, directory, ['log', '--format=%h%x09%s', `--max-count=${LIMIT}`, range], signal) : Promise.resolve(undefined),
    range ? run(subprocess, directory, ['diff', '--name-status', range], signal) : Promise.resolve(undefined),
    range ? run(subprocess, directory, ['diff', '--shortstat', range], signal) : Promise.resolve(undefined),
    run(subprocess, directory, ['status', '--porcelain'], signal),
  ])
  const commits = (log ?? '').split('\n').filter(Boolean).slice(0, LIMIT).map(line => {
    const [sha, subject = ''] = line.split('\t')
    return { sha: sha ?? '', subject: subject.slice(0, 200) }
  })
  const changed = (files ?? '').split('\n').filter(Boolean).slice(0, LIMIT).map(line => {
    const [status, ...path] = line.split('\t')
    return { status: status ?? '', path: path.join('\t') }
  })
  // `git status --porcelain` writes a two-column status, a space, then the path.
  const dirty = (status ?? '').split('\n').filter(Boolean).slice(0, LIMIT).map(line => ({ status: line.slice(0, 2).trim(), path: line.slice(3) }))
  return { capturedAt, ...(branch && branch !== 'HEAD' ? { branch } : {}), ...(base ? { base } : {}), head, commits, files: changed, dirty,
    ...(stat ? { stat } : {}) }
}

/** Captures the git facts of a ticket's workspace when it ends, so the panel shows what changed without opening the worktree. */
import { basename, dirname } from 'node:path'
import type { SubprocessRuntime } from '@deepseek-ai/dsh-subprocess'
import type { Ticket, TicketReview } from '../core/index.ts'
import { CliError } from '../infra/commands.ts'
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

/** Where a ticket workspace sits in git, and the repository branch a merge would move. */
export interface WorkspaceRepo {
  /** Main repository root, shared by its worktrees. */
  root: string
  branch: string
  head: string
  /** Branch checked out in the main working tree, which a merge would fast-forward. */
  target: string
  /** Whether the main working tree has uncommitted changes. */
  dirty: boolean
  /** False when the ticket works directly in the repository instead of a linked worktree. */
  linked: boolean
}
/**
 * Reads a ticket workspace's branch, repository root, and the branch checked out there.
 * @returns undefined when the workspace is not a git repository on a branch.
 */
export async function workspaceRepo(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal): Promise<WorkspaceRepo | undefined> {
  const git = (cwd: string, args: string[]) => runAsWorker(subprocess, ['git', ...args], cwd,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)])).then(output => output.trim())
  const branch = await git(directory, ['rev-parse', '--abbrev-ref', 'HEAD']).catch(() => undefined)
  if (!branch || branch === 'HEAD') return undefined
  const head = await git(directory, ['rev-parse', 'HEAD']).catch(() => undefined)
  const common = await git(directory, ['rev-parse', '--path-format=absolute', '--git-common-dir']).catch(() => undefined)
  if (!head || !common) return undefined
  const root = basename(common) === '.git' ? dirname(common) : common
  const target = await git(root, ['rev-parse', '--abbrev-ref', 'HEAD']).catch(() => undefined)
  if (!target || target === 'HEAD') return undefined
  // Untracked files, such as a ticket archive kept in the checkout, do not block a fast-forward; tracked changes do.
  const dirty = (await git(root, ['status', '--porcelain', '--untracked-files=no']).catch(() => '')).length > 0
  return { root, branch, head, target, dirty, linked: root !== directory }
}
/**
 * Fast-forwards the repository's checked-out branch to the ticket branch, so no merge commit or conflict state is created.
 * @throws when the workspace is not a linked worktree, the repository is dirty, the branch is checked out there, or the
 * repository cannot fast-forward.
 * @returns The branch merged and the branch it moved.
 */
export async function mergeWorkspace(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal): Promise<{ branch: string; target: string }> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  if (!repo) throw new Error('The ticket workspace is not a git repository on a branch')
  if (!repo.linked) throw new Error('This ticket works in the repository itself; merge its branch manually')
  if (repo.dirty) throw new Error('The repository has uncommitted changes; commit or stash them before merging')
  if (repo.target === repo.branch) throw new Error('The repository already has the ticket branch checked out')
  // A branch that lacks the repository's newer commits cannot fast-forward; name the fix instead of git's bare refusal.
  const ahead = await runAsWorker(subprocess, ['git', 'merge-base', '--is-ancestor', repo.target, repo.branch], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
    .then(() => true).catch((error: unknown) => {
      if (error instanceof CliError && error.exitCode === 1) return false
      throw error
    })
  if (!ahead) throw new Error(`${repo.target} has commits that ${repo.branch} does not; rebase the ticket branch onto ${repo.target} and merge again`)
  await runAsWorker(subprocess, ['git', 'merge', '--ff-only', repo.branch], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
  return { branch: repo.branch, target: repo.target }
}
/**
 * Rebases the ticket branch onto the repository's checked-out branch, so a merge can fast-forward after the repository moved.
 * A conflicted rebase is aborted, leaving the workspace on its previous commits.
 * @returns The branch rebased and the branch it moved onto.
 * @throws when the workspace is not a linked worktree, the repository already has the ticket branch checked out, or the
 * rebase cannot finish cleanly.
 */
export async function rebaseWorkspace(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal): Promise<{ branch: string; target: string }> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  if (!repo) throw new Error('The ticket workspace is not a git repository on a branch')
  if (!repo.linked) throw new Error('This ticket works in the repository itself; rebase its branch manually')
  if (repo.target === repo.branch) throw new Error('The ticket branch is the repository branch')
  try {
    await runAsWorker(subprocess, ['git', 'rebase', repo.target], directory,
      AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
  } catch (error) {
    await runAsWorker(subprocess, ['git', 'rebase', '--abort'], directory,
      AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)])).catch(() => {
      // A rebase that never started has nothing to abort; the original failure is the report.
    })
    throw error
  }
  return { branch: repo.branch, target: repo.target }
}
/**
 * Removes the ticket's linked worktree and deletes its branch; the repository itself and other worktrees stay.
 * @param options - `merged` deletes the branch with git's safe check, for a branch whose commits the target already has.
 * @throws when the workspace is not a linked worktree or either removal fails.
 */
export async function discardWorkspace(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal,
  options: { merged?: boolean } = {}): Promise<void> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  if (!repo) throw new Error('The ticket workspace is not a git repository on a branch')
  if (!repo.linked) throw new Error('This ticket works in the repository itself; remove its changes yourself')
  await runAsWorker(subprocess, ['git', 'worktree', 'remove', '--force', directory], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
  await runAsWorker(subprocess, ['git', 'branch', options.merged ? '-d' : '-D', repo.branch], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
}

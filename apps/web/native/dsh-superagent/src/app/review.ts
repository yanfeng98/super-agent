/** Captures the git facts of a ticket's workspace when it ends, so the panel shows what changed without opening the worktree. */
import { createHash } from 'node:crypto'
import { readdir } from 'node:fs/promises'
import { basename, dirname } from 'node:path'
import type { SubprocessRuntime } from '@deepseek-ai/dsh-subprocess'
import type { MergeStrategy, Ticket, TicketNote, TicketReview } from '../core/index.ts'
import { CliError } from '../infra/commands.ts'
import { runAsWorker } from '../infra/environment.ts'

const TIMEOUT_MS = 30_000
/** One file of a diff, as the panel lists it: its paths, what happened to it, and its hunks. */
export interface DiffFile {
  path: string
  /** The path before a rename. */
  oldPath?: string
  status: 'added' | 'deleted' | 'renamed' | 'modified'
  additions: number
  deletions: number
  /** The file's part of the diff, headers and hunks included, so the panel can show it verbatim. */
  patch: string
}
/**
 * A stable name for the changes a Ticket adds on top of `base`, which a rebase leaves alone when it only moves them onto a
 * newer commit: the same changes keep the same fingerprint, so an approval of them still holds.
 * @returns The fingerprint, or undefined when git cannot read the diff.
 */
export async function patchFingerprint(subprocess: SubprocessRuntime, directory: string, base: string,
  signal: AbortSignal): Promise<string | undefined> {
  const diff = await run(subprocess, directory, ['diff', '--no-color', `${base}..HEAD`], signal)
  return diff === undefined ? undefined : createHash('sha256').update(diff).digest('hex')
}
/** Longest diff the panel receives; a larger change set is truncated with a marker. */
const DIFF_LIMIT = 200_000
/** Cap each list so a large change set stays readable and bounded in the ticket record. */
const LIMIT = 50
const run = async (subprocess: SubprocessRuntime, directory: string, args: string[], signal: AbortSignal): Promise<string | undefined> =>
  runAsWorker(subprocess, ['git', '-c', 'core.quotePath=false', ...args], directory,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
    .then(output => output.trim()).catch(() => undefined)

/** Reads the current commit of a workspace; undefined when it is not a git repository or git is unavailable. */
export const workspaceHead = (subprocess: SubprocessRuntime, directory: string, signal: AbortSignal): Promise<string | undefined> =>
  run(subprocess, directory, ['rev-parse', 'HEAD'], signal)

/**
 * The commit the ticket branch forked from the repository's branch, which is where its review starts even when the branch
 * merged or rebased the repository's newer commits in after it launched.
 * @returns `undefined` when the ticket records no workspace or target branch, or git cannot answer.
 */
export async function workingBase(subprocess: SubprocessRuntime, ticket: Ticket, signal: AbortSignal): Promise<string | undefined> {
  const directory = ticket.execution?.directory
  const target = ticket.execution?.target
  if (!directory || !target) return undefined
  const base = await run(subprocess, directory, ['merge-base', 'HEAD', target], signal)
  return base?.split('\n')[0] || undefined
}

/**
 * Reads the committed diff of a ticket workspace since `base`, for the panel's on-demand view.
 * @returns The diff, truncated when it exceeds the panel limit.
 * @throws when git fails, such as in a workspace that is not a repository.
 */
export async function workspaceDiff(subprocess: SubprocessRuntime, directory: string, base: string, signal: AbortSignal):
  Promise<{ diff: string; truncated: boolean; files: DiffFile[] }> {
  const output = await runAsWorker(subprocess, ['git', '-c', 'core.quotePath=false', 'diff', '--no-color', '--unified=3', `${base}..HEAD`], directory,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
  const truncated = output.length > DIFF_LIMIT
  const diff = truncated ? output.slice(0, DIFF_LIMIT) : output
  return { diff, truncated, files: parseDiff(diff) }
}
/**
 * Splits a unified diff into one entry per file, which the panel lists and expands one at a time.
 * @param diff - The output of `git diff`, such as the committed diff of a Ticket's branch.
 * @returns One entry per `diff --git` header, in the order git printed them.
 */
export function parseDiff(diff: string): DiffFile[] {
  const files: DiffFile[] = []
  for (const block of diff.split(/^diff --git /m).slice(1)) {
    const lines = `diff --git ${block}`.split('\n')
    // A trailing newline ends the file rather than starting a line.
    if (lines.at(-1) === '') lines.pop()
    const header = lines.filter(line => !line.startsWith('@@') && !line.startsWith('+') && !line.startsWith('-'))
    const plus = /^\+\+\+ b\/(.+)$/m.exec(block)?.[1]
    const minus = /^--- a\/(.+)$/m.exec(block)?.[1]
    const from = /^rename from (.+)$/m.exec(block)?.[1]
    const to = /^rename to (.+)$/m.exec(block)?.[1]
    // A binary or mode-only file has no `+++` line, so the header's own paths are the fallback.
    const pair = /^diff --git a\/(.+) b\/(.+)$/.exec(lines[0] ?? '')
    const status = header.some(line => line.startsWith('new file mode')) ? 'added'
      : header.some(line => line.startsWith('deleted file mode')) ? 'deleted'
        : from ? 'renamed' : 'modified'
    const path = plus ?? to ?? minus ?? from ?? pair?.[2] ?? ''
    const oldPath = from && from !== path ? from : undefined
    let additions = 0, deletions = 0, hunks = false
    for (const line of lines) {
      if (line.startsWith('@@')) { hunks = true; continue }
      if (!hunks) continue
      if (line.startsWith('+')) additions++
      else if (line.startsWith('-')) deletions++
    }
    files.push({ path, ...oldPath ? { oldPath } : {}, status, additions, deletions, patch: lines.join('\n') })
  }
  return files
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
  // A missing or empty directory has no git to read; naming that state avoids running git in a directory that is not there.
  if (!await readdir(directory).then(files => files.length > 0, () => false)) {
    return { capturedAt, commits: [], files: [], dirty: [], error: await reviewProblem(directory) }
  }
  const [branch, head] = await Promise.all([
    run(subprocess, directory, ['rev-parse', '--abbrev-ref', 'HEAD'], signal),
    workspaceHead(subprocess, directory, signal),
  ])
  if (!head) return { capturedAt, commits: [], files: [], dirty: [], error: await reviewProblem(directory) }
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

/** One hunk of a unified diff, in old and new line numbers, for moving a note's anchor with the code. */
interface DiffHunk { oldStart: number; oldCount: number; newStart: number; newCount: number }
/** One file a diff changes: its hunks, or `all` when the file's lines no longer anchor. */
interface ChangedFile { path: string; oldPath?: string; hunks: DiffHunk[] | 'all' }
/** Reads the old and new line ranges a unified diff changes, for moving a review note's anchor. */
function changedFiles(diff: string): ChangedFile[] {
  const files: ChangedFile[] = []
  for (const block of diff.split(/^diff --git /m).slice(1)) {
    const text = `diff --git ${block}`
    const plus = /^\+\+\+ b\/(.+)$/m.exec(text)?.[1]
    const minus = /^--- a\/(.+)$/m.exec(text)?.[1]
    const from = /^rename from (.+)$/m.exec(text)?.[1]
    const to = /^rename to (.+)$/m.exec(text)?.[1]
    const pair = /^diff --git a\/(.+) b\/(.+)$/m.exec(text)
    const path = plus ?? to ?? minus ?? from ?? pair?.[2] ?? ''
    if (!path) continue
    const oldPath = from && from !== path ? from : undefined
    // A file that was added, deleted, renamed, or is binary has no stable old line for a note to keep.
    if (/^new file mode /m.test(text) || /^deleted file mode /m.test(text) || from || /^Binary files /m.test(text)) {
      files.push({ path, oldPath, hunks: 'all' })
      continue
    }
    const hunks: DiffHunk[] = []
    for (const match of text.matchAll(/^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/gm)) {
      hunks.push({ oldStart: Number(match[1]), oldCount: match[2] === undefined ? 1 : Number(match[2]),
        newStart: Number(match[3]), newCount: match[4] === undefined ? 1 : Number(match[4]) })
    }
    files.push({ path, oldPath, hunks })
  }
  return files
}
/** Where a note's line now sits after the hunks before it, or that the line itself changed. */
function remapLine(hunks: readonly DiffHunk[], line: number): { line: number; stale: boolean } {
  let shift = 0
  for (const hunk of hunks) {
    const oldEnd = hunk.oldCount === 0 ? hunk.oldStart : hunk.oldStart + hunk.oldCount - 1
    // A hunk that covers the line replaced or deleted it; the note no longer describes code.
    if (hunk.oldCount > 0 && line >= hunk.oldStart && line <= oldEnd) return { line, stale: true }
    if (line > oldEnd) shift += hunk.newCount - hunk.oldCount
  }
  return { line: line + shift, stale: false }
}
/** Where a note's anchor now sits: the moved line and range, or that the code it covered changed. */
export interface NoteAnchor { stale: boolean; line?: number; endLine?: number }
/**
 * Moves each review note's line and range through the commits since it was written, so the page anchors it on the line
 * the code now occupies; a note whose own line was replaced or deleted is stale instead.
 * @param ticket - The ticket whose notes are checked.
 * @param review - The current review, whose head the notes are compared against.
 * @returns A map from note id to its anchor; a note absent from the map could not be checked.
 */
export async function remapNoteAnchors(subprocess: SubprocessRuntime, ticket: Ticket, review: TicketReview | undefined,
  signal: AbortSignal): Promise<Map<string, NoteAnchor>> {
  const directory = ticket.execution?.directory
  const head = review?.head
  if (!directory || !head) return new Map()
  const byCommit = new Map<string, TicketNote[]>()
  for (const note of ticket.notes ?? []) {
    if (!note.commit || note.commit === head) continue
    const group = byCommit.get(note.commit) ?? []
    group.push(note)
    byCommit.set(note.commit, group)
  }
  const result = new Map<string, NoteAnchor>()
  for (const [commit, group] of byCommit) {
    // The whole diff, not only the notes' paths: a pathspec would hide a rename that moved a note's file.
    const diff = await run(subprocess, directory, ['diff', '--no-color', '--unified=0', '-M', `${commit}..${head}`], signal)
    // A failed diff leaves these notes unchecked; the page then falls back to the whole-head comparison.
    if (diff === undefined) continue
    const changed = changedFiles(diff)
    for (const note of group) {
      const keep = { ...note.line === undefined ? {} : { line: note.line }, ...note.endLine === undefined ? {} : { endLine: note.endLine } }
      const file = changed.find(item => item.path === note.file || item.oldPath === note.file)
      if (!file) {
        result.set(note.id, { stale: false, ...keep })
        continue
      }
      if (file.hunks === 'all') {
        result.set(note.id, { stale: true, ...keep })
        continue
      }
      // A file-level note is stale once any content line changed, not when only its mode did.
      if (note.line === undefined) {
        result.set(note.id, { stale: file.hunks.length > 0 })
        continue
      }
      const end = note.endLine ?? note.line
      const start = remapLine(file.hunks, note.line)
      const stop = remapLine(file.hunks, end)
      // A hunk inside the range replaced or deleted lines the note covers even when neither endpoint moved.
      const inside = file.hunks.some(hunk => hunk.oldCount > 0
        && hunk.oldStart <= end && hunk.oldStart + hunk.oldCount - 1 >= note.line!)
      if (start.stale || stop.stale || inside) {
        result.set(note.id, { stale: true, ...keep })
      } else {
        result.set(note.id, { stale: false, line: start.line, ...note.endLine === undefined ? {} : { endLine: stop.line } })
      }
    }
  }
  return result
}
/** How a merge moves the target branch: fast-forward only, a squashed commit, or a merge commit. */
export type { MergeStrategy }
/**
 * A rebase that stopped on conflicts, which the workspace then abandoned, leaving the branch on its commits.
 * `files` are the paths git left unmerged, which a worker resolves in its own worktree.
 */
export class RebaseConflictError extends Error {
  constructor(readonly target: string, readonly files: readonly string[]) {
    super(`Rebasing onto ${target} conflicts in ${files.join(', ')}; the Ticket branch keeps its commits`)
    this.name = 'RebaseConflictError'
  }
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
  // A missing or empty directory starts no git: git could not run there, and the merge gates describe that state instead.
  if (!await readdir(directory).then(files => files.length > 0, () => false)) return undefined
  const git = (cwd: string, args: string[]) => runAsWorker(subprocess, ['git', ...args], cwd,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)])).then(output => output.trim())
  const branch = await git(directory, ['rev-parse', '--abbrev-ref', 'HEAD']).catch(() => undefined)
  if (!branch || branch === 'HEAD') return undefined
  const head = await git(directory, ['rev-parse', 'HEAD']).catch(() => undefined)
  const common = await git(directory, ['rev-parse', '--path-format=absolute', '--git-common-dir']).catch(() => undefined)
  const gitDir = await git(directory, ['rev-parse', '--path-format=absolute', '--git-dir']).catch(() => undefined)
  if (!head || !common || !gitDir) return undefined
  const root = basename(common) === '.git' ? dirname(common) : common
  const target = await git(root, ['rev-parse', '--abbrev-ref', 'HEAD']).catch(() => undefined)
  if (!target || target === 'HEAD') return undefined
  // Untracked files, such as a ticket archive kept in the checkout, do not block a fast-forward; tracked changes do.
  const dirty = (await git(root, ['status', '--porcelain', '--untracked-files=no']).catch(() => '')).length > 0
  // A linked worktree keeps its own git directory under the repository's; the repository's own tree and a plain
  // subdirectory of it share theirs.
  return { root, branch, head, target, dirty, linked: gitDir !== common }
}
/** Why a workspace has no reviewable head, in the words that fit its state; a plain directory is a supported workspace. */
async function reviewProblem(directory: string): Promise<string> {
  const entries = await readdir(directory).then(files => files.length, () => undefined)
  if (entries === undefined) return `The ticket workspace ${directory} does not exist`
  if (entries === 0) return `The ticket workspace ${directory} is empty`
  return `The ticket workspace ${directory} is not a git repository, or git could not read it`
}
/**
 * Explains why a ticket workspace is not a checkout a Ticket can work in, naming the state a person must repair rather
 * than git's bare refusal.
 * @param directory - The ticket's workspace directory.
 * @returns A sentence naming the real state, with the step that prepares a checkout; a plain directory within a
 * repository is described, not corrected.
 */
export async function workspaceProblem(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal): Promise<string> {
  const entries = await readdir(directory).then(files => files.length, () => undefined)
  if (entries === undefined) {
    return `The ticket workspace ${directory} does not exist, so it is not a git repository: no checkout was created. `
      + 'Set Execution → Repository to a git checkout, or a prepare command that creates one, then retry the Ticket.'
  }
  if (entries === 0) {
    return `The ticket workspace ${directory} is empty, so it is not a git repository: no checkout was created. `
      + 'Set Execution → Repository to a git checkout, or a prepare command that creates one, then retry the Ticket.'
  }
  const git = (cwd: string, args: string[]) => runAsWorker(subprocess, ['git', ...args], cwd,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)])).then(output => output.trim())
  const branch = await git(directory, ['rev-parse', '--abbrev-ref', 'HEAD']).catch(() => undefined)
  if (!branch) {
    return `The ticket workspace ${directory} is not a git repository, or git could not read it. `
      + 'A Ticket needs its own checkout of the repository it changes; set Execution → Repository or a prepare command that creates one.'
  }
  if (branch === 'HEAD') {
    return `The ticket workspace ${directory} is on a detached HEAD, not on its Ticket branch. `
      + 'Check out the Ticket branch there before merging or reviewing it.'
  }
  const common = await git(directory, ['rev-parse', '--path-format=absolute', '--git-common-dir']).catch(() => undefined)
  const gitDir = await git(directory, ['rev-parse', '--path-format=absolute', '--git-dir']).catch(() => undefined)
  if (!common || !gitDir) return `The ticket workspace ${directory} is not a git repository, or git could not read it.`
  const root = basename(common) === '.git' ? dirname(common) : common
  const target = await git(root, ['rev-parse', '--abbrev-ref', 'HEAD']).catch(() => undefined)
  if (!target || target === 'HEAD') {
    return `The repository at ${root} is on a detached HEAD, so there is no branch to merge into. Check out its branch.`
  }
  if (gitDir === common) {
    return `The ticket workspace ${directory} is the repository's main working tree, not a linked worktree. `
      + 'A Ticket works in its own git worktree so the main checkout keeps its branch; set a prepare command in Execution settings that creates one.'
  }
  return `The ticket workspace ${directory} is not a git repository on a branch.`
}
/** The git facts a Ticket records when it starts: its branch, head, repository root, and the target branch it forked from. */
export interface WorkspaceFacts {
  branch: string
  head: string
  root: string
  target: string
  /** The commit the Ticket branch forked from the target, which an ending's review starts from; absent for unrelated histories. */
  base?: string
}
/**
 * Reads the facts a Ticket records for its workspace: its branch, head, the repository's checked-out branch, and the commit
 * its branch forked from that branch.
 * @returns `undefined` when the workspace is not a linked worktree on a branch with a target branch.
 */
export async function workspaceFacts(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal): Promise<WorkspaceFacts | undefined> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  if (!repo) return undefined
  const base = await runAsWorker(subprocess, ['git', 'merge-base', 'HEAD', repo.target], directory,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)])).then(output => output.trim()).catch(() => undefined)
  return { branch: repo.branch, head: repo.head, root: repo.root, target: repo.target, ...base ? { base } : {} }
}
/**
 * Whether the repository's checked-out commit is already in the ticket branch, which is what lets a fast-forward, a squash
 * or a merge commit finish without a conflict.
 */
const contains = (subprocess: SubprocessRuntime, repo: WorkspaceRepo, signal: AbortSignal): Promise<boolean> =>
  runAsWorker(subprocess, ['git', 'merge-base', '--is-ancestor', repo.target, repo.branch], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
    .then(() => true).catch((error: unknown) => {
      // Exit code 1 answers that the commit is not an ancestor; any other failure is a broken repository, not an answer.
      if (error instanceof CliError && error.exitCode === 1) return false
      throw error
    })

/**
 * Whether the repository's checked-out commit is already in the ticket branch, so a merge would not conflict.
 * @returns `undefined` when the workspace is not a git repository on a branch.
 */
export async function targetContained(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal): Promise<boolean | undefined> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  return repo ? contains(subprocess, repo, signal) : undefined
}

/** How long back the remote log is searched for a commit rewritten from a merge's head. */
const REWRITTEN_WINDOW_MS = 30 * 24 * 60 * 60 * 1000
/** A rewritten commit keeps its message and author date, which identify it across a history rewrite. */
const COMMIT_IDENTITY = '%s%x09%aI'
/**
 * Whether a merge's commit already reached its target's upstream, so a recorded push failure is stale.
 *
 * A local push normally fast-forwards the upstream through the recorded head, which `merge-base --is-ancestor` answers
 * directly. A later history rewrite can replace that commit with an identical one under another SHA; the remote log is then
 * searched for a commit with the same subject and author date, which a rewrite preserves. The remote-tracking ref is read
 * as git last saw it: no fetch runs here, so a stale ref simply leaves the recorded failure in place.
 *
 * @param directory - The repository root recorded on the merge; a worktree removed by cleanup is not needed.
 * @param head - The commit the merge left on its target branch.
 * @param target - The branch the merge moved, whose upstream is asked for the commit.
 * @param mergedAt - When the merge landed; rewritten commits before this time are within the search window.
 * @returns `true` when the commit is in the upstream, `false` when it is not or no upstream is configured, and `undefined`
 * when git cannot read the repository; an unknown answer never changes a recorded push result.
 */
export async function pushLanded(subprocess: SubprocessRuntime, directory: string, head: string, target: string,
  mergedAt: string | undefined, signal: AbortSignal): Promise<boolean | undefined> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  if (!repo) return undefined
  const git = (args: string[]) => runAsWorker(subprocess, ['git', ...args], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)])).then(output => output.trim())
  const upstream = await git(['rev-parse', '--abbrev-ref', '--symbolic-full-name', `${target}@{u}`]).catch(() => undefined)
  if (!upstream) return false
  const ancestor = await git(['merge-base', '--is-ancestor', head, upstream]).then(() => true)
    .catch((error: unknown) => {
      if (error instanceof CliError && error.exitCode === 1) return false
      return undefined
    })
  if (ancestor === true) return true
  const identity = await git(['show', '-s', `--format=${COMMIT_IDENTITY}`, head]).catch(() => undefined)
  if (!identity) return ancestor === false ? false : undefined
  const since = mergedAt && Date.parse(mergedAt) > REWRITTEN_WINDOW_MS
    ? new Date(Date.parse(mergedAt) - REWRITTEN_WINDOW_MS).toISOString() : undefined
  const log = await git(['log', `--format=${COMMIT_IDENTITY}`, ...since ? [`--since=${since}`] : [], upstream]).catch(() => undefined)
  return log?.split('\n').includes(identity) ?? false
}
/**
 * Merges the ticket branch into the repository's checked-out branch by `strategy`: a fast-forward, one squashed commit, or a
 * merge commit. The ticket branch must contain the repository's commit, so the merge cannot conflict.
 * @param strategy - The merge form to use; `ff` refuses when the repository moved.
 * @param expectedTarget - The branch recorded when the Ticket started; the merge refuses when the repository is on another.
 * @param message - The commit message for a squash or merge commit; omitted, the default is used.
 * @throws when the workspace is not a linked worktree, the repository is dirty, on another branch than `expectedTarget`,
 * the ticket branch is checked out there, the ticket branch does not contain the repository's commit, or git cannot complete
 * the merge.
 * @returns The branch merged, the branch it moved, the commit that branch holds after the merge, the commit it moved from,
 * and the repository root.
 */
export async function mergeWorkspace(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal,
  strategy: MergeStrategy = 'ff', expectedTarget?: string, message?: string):
  Promise<{ branch: string; target: string; head?: string; base?: string; root: string }> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  if (!repo) throw new Error(await workspaceProblem(subprocess, directory, signal))
  if (!repo.linked) throw new Error('This ticket works in the repository itself; merge its branch manually')
  if (expectedTarget && repo.target !== expectedTarget) {
    throw new Error(`The repository is on ${repo.target}, not the Ticket's target ${expectedTarget}; check it out and merge again`)
  }
  if (repo.dirty) throw new Error('The repository has uncommitted changes; commit or stash them before merging')
  if (repo.target === repo.branch) throw new Error('The repository already has the ticket branch checked out')
  // A branch that lacks the repository's newer commits cannot fast-forward; name the fix instead of git's bare refusal.
  if (!await contains(subprocess, repo, signal)) {
    throw new Error(`${repo.target} has commits that ${repo.branch} does not; rebase the ticket branch onto ${repo.target} and merge again`)
  }
  const run = (args: string[]) => runAsWorker(subprocess, ['git', ...args], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
  // The target's own commit, read before the merge moves it, so a fast-forward merge can be undone commit by commit.
  const base = await run(['rev-parse', 'HEAD']).then(output => output.trim()).catch(() => undefined)
  if (strategy === 'ff') {
    await run(['merge', '--ff-only', repo.branch])
  } else if (strategy === 'merge') {
    try { await run(['merge', '--no-ff', ...message ? ['-m', message] : ['--no-edit'], repo.branch]) }
    catch (error) {
      // A merge commit that cannot be made (such as a missing identity) must not leave the repository mid-merge.
      await run(['merge', '--abort']).catch(() => undefined)
      throw error
    }
  } else {
    try {
      await run(['merge', '--squash', repo.branch])
      await run(['commit', '-m', message ?? `Squash merge ${repo.branch}`])
    } catch (error) {
      // Leave no staged squash or conflict state behind when the squash cannot be completed.
      await run(['reset', '--hard', 'HEAD']).catch(() => undefined)
      throw error
    }
  }
  // The merge already moved the branch, so a head that git cannot report must not turn it into a failure.
  const head = await run(['rev-parse', 'HEAD']).then(output => output.trim()).catch(() => undefined)
  return { branch: repo.branch, target: repo.target, root: repo.root, ...base ? { base } : {}, ...head ? { head } : {} }
}
/** A revert that stopped on conflicts, which the repository then abandoned, leaving the target branch as it was. */
export class RevertConflictError extends Error {
  constructor(readonly commit: string, readonly files: readonly string[]) {
    super(`Reverting ${commit} conflicts in ${files.join(', ')}; the branch keeps its commits`)
    this.name = 'RevertConflictError'
  }
}
/**
 * Reverts a merge on the repository's checked-out branch, which is how a person undoes one from any interface. A merge
 * commit is reverted against its first parent; a fast-forward merge's commits are reverted one after another, since undoing
 * only the last would leave the rest of its work on the branch. No branch or worktree is rewritten.
 * @param directory - Where to run git: the repository root a merge recorded, or the Ticket's worktree while it exists.
 * @param commit - The commit to undo, such as the one a merge recorded as `head`.
 * @param expectedTarget - The branch a merge recorded; a revert refuses when the repository is on another one.
 * @param base - The commit the merge moved the target from; with it, the commits in `base..commit` are reverted in order.
 * @throws when the directory is not a git repository on a branch, the repository is on another branch or has tracked
 * changes, the commit is unknown, the revert conflicts (nothing is left half-applied), or git cannot commit the revert.
 * @returns The branch reverted and the commit it holds afterwards.
 */
export async function revertWorkspace(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal,
  commit: string, expectedTarget?: string, base?: string): Promise<{ target: string; head: string }> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  if (!repo) throw new Error(await workspaceProblem(subprocess, directory, signal))
  if (expectedTarget && repo.target !== expectedTarget) {
    throw new Error(`The repository is on ${repo.target}, not the branch that merge moved (${expectedTarget}); check it out and revert again`)
  }
  const run = (args: string[]) => runAsWorker(subprocess, ['git', ...args], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)])).then(output => output.trim())
  // A commit git cannot name is a merge that was rewritten away, not a conflict or a dirty tree the caller can resolve.
  for (const object of base && base !== commit ? [base, commit] : [commit]) {
    await run(['cat-file', '-e', `${object}^{commit}`]).catch(() => {
      throw new Error(`The merge commit ${object} is not in ${repo.root}; its branch may have been rewritten`)
    })
  }
  if (repo.dirty) throw new Error('The repository has uncommitted changes; commit or stash them before reverting')
  // A fast-forward merge's commits are reverted one after another; a merge commit needs its first parent named.
  const range = base && base !== commit ? `${base}..${commit}` : undefined
  const parents = range ? [] : await run(['rev-list', '--parents', '-n', '1', commit]).then(line => line.split(' ').slice(1))
  const args = range ? ['revert', '--no-edit', range] : parents.length > 1 ? ['revert', '--no-edit', '-m', '1', commit] : ['revert', '--no-edit', commit]
  try {
    await run(args)
  } catch (error) {
    const unmerged = await run(['diff', '--name-only', '--diff-filter=U']).then(output => output.split('\n').filter(Boolean)).catch(() => [])
    await run(['revert', '--abort']).catch(() => {
      // A revert that never started has nothing to abort; the original failure is the report.
    })
    if (unmerged.length) throw new RevertConflictError(range ?? commit, unmerged)
    throw error
  }
  return { target: repo.target, head: await run(['rev-parse', 'HEAD']) }
}
/** Longest a push may take; a remote that stalls ends the wait with a failure rather than blocking the merge. */const PUSH_TIMEOUT_MS = 120_000
/**
 * Pushes the repository's checked-out branch, which a merge just moved, to the remote branch its upstream names. The remote,
 * branch and credentials are the repository's own, so a deployment needs no push setting.
 * @param expectedTarget - The branch a merge recorded; a push refuses when the repository is on another one, so a retry
 * after someone switched branches pushes the right branch or nothing.
 * @throws when the workspace is not a git repository on a branch, is on another branch than `expectedTarget`, has no
 * upstream, or git cannot push.
 */
export async function pushWorkspace(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal,
  expectedTarget?: string): Promise<void> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  if (!repo) throw new Error(await workspaceProblem(subprocess, directory, signal))
  if (expectedTarget && repo.target !== expectedTarget) {
    throw new Error(`The repository is on ${repo.target}, not the branch that merge moved (${expectedTarget}); check it out and push again`)
  }
  await runAsWorker(subprocess, ['git', 'push'], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(PUSH_TIMEOUT_MS)]))
}
/**
 * Rebases the ticket branch onto the repository's checked-out branch, so a merge can fast-forward after the repository moved.
 * A conflicted rebase is aborted, leaving the workspace on its previous commits.
 * @param expectedTarget - The branch recorded when the Ticket started; the rebase refuses when the repository is on another.
 * @returns The branch rebased, the branch it moved onto, and the target's commit that is now the workspace's base.
 * @throws when the workspace is not a linked worktree, the repository is on another branch than `expectedTarget`, already
 * has the ticket branch checked out, or the rebase cannot finish cleanly.
 */
export async function rebaseWorkspace(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal,
  expectedTarget?: string): Promise<{ branch: string; target: string; base: string }> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  if (!repo) throw new Error(await workspaceProblem(subprocess, directory, signal))
  if (!repo.linked) throw new Error('This ticket works in the repository itself; rebase its branch manually')
  if (expectedTarget && repo.target !== expectedTarget) {
    throw new Error(`The repository is on ${repo.target}, not the Ticket's target ${expectedTarget}; check it out and rebase again`)
  }
  if (repo.target === repo.branch) throw new Error('The ticket branch is the repository branch')
  try {
    await runAsWorker(subprocess, ['git', 'rebase', repo.target], directory,
      AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
  } catch (error) {
    // Name the files still in conflict before the abort clears them, so the report says what stopped the rebase.
    const unmerged = await runAsWorker(subprocess, ['git', 'diff', '--name-only', '--diff-filter=U'], directory,
      AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)])).then(output => output.split('\n').filter(Boolean)).catch(() => [])
    await runAsWorker(subprocess, ['git', 'rebase', '--abort'], directory,
      AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)])).catch(() => {
      // A rebase that never started has nothing to abort; the original failure is the report.
    })
    if (unmerged.length) throw new RebaseConflictError(repo.target, unmerged)
    throw error
  }
  const base = (await runAsWorker(subprocess, ['git', 'rev-parse', repo.target], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))).trim()
  return { branch: repo.branch, target: repo.target, base }
}
/**
 * Removes the ticket's linked worktree and deletes its branch; the repository itself and other worktrees stay.
 * @param options - `merged` deletes the branch with git's safe check, for a branch whose commits the target already has;
 * `squash` skips that check and forces deletion, for a recorded squash merge whose commit is not an ancestor.
 * @throws when the workspace is not a linked worktree or either removal fails.
 */
export async function discardWorkspace(subprocess: SubprocessRuntime, directory: string, signal: AbortSignal,
  options: { merged?: boolean; squash?: boolean } = {}): Promise<void> {
  const repo = await workspaceRepo(subprocess, directory, signal)
  if (!repo) throw new Error(await workspaceProblem(subprocess, directory, signal))
  if (!repo.linked) throw new Error('This ticket works in the repository itself; remove its changes yourself')
  if (options.merged && !options.squash) {
    // Refuse before removing the worktree when the target does not contain the branch, so new commits cannot be lost.
    const contained = await runAsWorker(subprocess, ['git', 'merge-base', '--is-ancestor', repo.branch, repo.target], repo.root,
      AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)])).then(() => true).catch((error: unknown) => {
      if (error instanceof CliError && error.exitCode === 1) return false
      throw error
    })
    if (!contained) throw new Error(`${repo.target} does not contain ${repo.branch}; merge it before cleaning up`)
  }
  await runAsWorker(subprocess, ['git', 'worktree', 'remove', '--force', directory], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
  // A squash merge records a new commit, so the branch is not an ancestor of the target; git's safe -d cannot apply.
  await runAsWorker(subprocess, ['git', 'branch', options.merged && !options.squash ? '-d' : '-D', repo.branch], repo.root,
    AbortSignal.any([signal, AbortSignal.timeout(TIMEOUT_MS)]))
}

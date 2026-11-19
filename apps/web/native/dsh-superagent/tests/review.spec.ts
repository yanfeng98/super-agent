import { execFileSync, spawn } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import type { SubprocessRuntime } from '@deepseek-ai/dsh-subprocess'
import type { Ticket } from '../src/core/index.ts'
import { captureReview, discardWorkspace, mergeWorkspace, parseDiff, pushWorkspace, RebaseConflictError, rebaseWorkspace, RevertConflictError, revertWorkspace, targetContained, workingBase, workspaceDiff, workspaceHead } from '../src/app/review.ts'

let dir: string
let repo: string
let config: string
let signal: AbortSignal
beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), 'review-'))
  repo = join(dir, 'repo')
  mkdirSync(repo)
  config = join(dir, 'gitconfig')
  writeFileSync(config, '[user]\n\tname = Test\n\temail = test@example.com\n')
  signal = new AbortController().signal
})
afterEach(() => { rmSync(dir, { recursive: true, force: true }) })

/** Minimal SubprocessRuntime backed by child_process.spawn, used by `runAsWorker`. */
const subprocess = (): SubprocessRuntime => ({
  spawn: ({ argv, cwd }: { argv: readonly string[]; cwd: string }) => {
    const child = spawn(argv[0]!, argv.slice(1), { cwd,
      env: { ...process.env, GIT_CONFIG_GLOBAL: config, GIT_CONFIG_NOSYSTEM: '1' } })
    const output = { stdout: '', stderr: '' }
    child.stdout.on('data', chunk => { output.stdout += String(chunk) })
    child.stderr.on('data', chunk => { output.stderr += String(chunk) })
    const reader = (name: 'stdout' | 'stderr') => ({ readFrom: () => ({ text: output[name], nextOffset: 0, lossy: false }) })
    const done = new Promise(resolve => child.on('close', code => resolve({ exitCode: code, signal: null })))
    return { done, collected: { stdout: reader('stdout'), stderr: reader('stderr') } }
  },
}) as unknown as SubprocessRuntime

const gitAt = (cwd: string, ...args: string[]): string => execFileSync('git', args, { cwd, encoding: 'utf8',
  env: { ...process.env, GIT_CONFIG_GLOBAL: config, GIT_CONFIG_NOSYSTEM: '1' } }).trim()
const git = (...args: string[]): string => gitAt(repo, ...args)
const ticket = (base?: string): Ticket => ({ execution: { name: 't1', directory: repo, assignment: '', ...base ? { base } : {} } }) as Ticket

describe('captureReview', () => {
  it('lists the branch, commits since the launch base, changed files, and uncommitted work', async () => {
    git('init', '-q')
    writeFileSync(join(repo, 'a.txt'), 'one\n')
    git('add', 'a.txt')
    git('commit', '-q', '-m', 'base')
    const base = git('rev-parse', 'HEAD')
    writeFileSync(join(repo, 'a.txt'), 'two\n')
    writeFileSync(join(repo, 'b.txt'), 'new\n')
    git('add', 'a.txt', 'b.txt')
    git('commit', '-q', '-m', '修复窄屏按钮')
    const head = git('rev-parse', 'HEAD')
    writeFileSync(join(repo, 'notes.txt'), 'draft\n')
    const review = (await captureReview(subprocess(), ticket(base), signal))!
    expect(review).toMatchObject({ branch: git('branch', '--show-current'), base, head,
      commits: [{ sha: head.slice(0, 7), subject: '修复窄屏按钮' }],
      files: [{ status: 'M', path: 'a.txt' }, { status: 'A', path: 'b.txt' }],
      dirty: [{ status: '??', path: 'notes.txt' }] })
    expect(review.stat).toMatch(/2 files changed/)
    expect(review.error).toBeUndefined()
    const diff = await workspaceDiff(subprocess(), repo, base, signal)
    expect(diff.truncated).toBe(false)
    expect(diff.diff).toContain('+two')
  })

  it('lists no commits when the workspace did not move, and reports a non-git directory instead of throwing', async () => {
    git('init', '-q')
    writeFileSync(join(repo, 'a.txt'), 'one\n')
    git('add', 'a.txt')
    git('commit', '-q', '-m', 'base')
    const head = git('rev-parse', 'HEAD')
    const unchanged = (await captureReview(subprocess(), ticket(head), signal))!
    expect(unchanged).toMatchObject({ head, commits: [], files: [], dirty: [] })
    expect(await workspaceHead(subprocess(), repo, signal)).toBe(head)
    // A directory outside a repository still yields a review, with the reason in `error`.
    const plain = join(dir, 'plain')
    mkdirSync(plain)
    const outside = await captureReview(subprocess(), { execution: { name: 't2', directory: plain, assignment: '' } } as Ticket, signal)
    expect(outside).toMatchObject({ commits: [], files: [], dirty: [], error: expect.stringMatching(/not a git repository/) })
  })
})

describe('parseDiff', () => {
  it('splits a diff per file with its status, counts, paths and patch', () => {
    const diff = ['diff --git a/src/a.ts b/src/a.ts', 'index 1..2 100644', '--- a/src/a.ts', '+++ b/src/a.ts', '@@ -1,2 +1,3 @@',
      '-old', '+new', '+more', 'diff --git a/img.png b/img.png', 'new file mode 100644', 'index 0..3', '--- /dev/null',
      '+++ b/img.png', '@@ -0,0 +1 @@', '+binary', 'diff --git a/old.ts b/renamed.ts', 'similarity index 90%',
      'rename from old.ts', 'rename to renamed.ts', 'diff --git a/gone.ts b/gone.ts', 'deleted file mode 100644',
      'index 4..0', '--- a/gone.ts', '+++ /dev/null', '@@ -1 +0,0 @@', '-bye'].join('\n')
    expect(parseDiff(diff)).toEqual([
      expect.objectContaining({ path: 'src/a.ts', status: 'modified', additions: 2, deletions: 1,
        patch: expect.stringContaining('-old') }),
      expect.objectContaining({ path: 'img.png', status: 'added', additions: 1, deletions: 0 }),
      expect.objectContaining({ path: 'renamed.ts', oldPath: 'old.ts', status: 'renamed', additions: 0, deletions: 0 }),
      expect.objectContaining({ path: 'gone.ts', status: 'deleted', additions: 0, deletions: 1 }),
    ])
    // An empty diff has no files, and a header git could not split still names its file.
    expect(parseDiff('')).toEqual([])
    expect(parseDiff('diff --git a/x.bin b/x.bin\nBinary files a/x.bin and b/x.bin differ')).toEqual([
      expect.objectContaining({ path: 'x.bin', status: 'modified', additions: 0, deletions: 0 })])
  })
})
describe('merge and discard', () => {
  const base = (): string => {
    git('init', '-q')
    writeFileSync(join(repo, 'a.txt'), 'one\n')
    git('add', 'a.txt'); git('commit', '-q', '-m', 'base')
    return git('rev-parse', 'HEAD')
  }
  it('fast-forwards the checked-out branch from a linked worktree, and refuses a dirty repository', async () => {
    base()
    const worktree = join(dir, 'worktree')
    git('worktree', 'add', '-q', '-b', 'agent/t', worktree)
    writeFileSync(join(worktree, 'a.txt'), 'two\n')
    gitAt(worktree, 'add', 'a.txt'); gitAt(worktree, 'commit', '-q', '-m', 'change')
    const head = gitAt(worktree, 'rev-parse', 'HEAD')
    expect(await mergeWorkspace(subprocess(), worktree, signal)).toMatchObject({ branch: 'agent/t', target: git('branch', '--show-current') })
    expect(git('rev-parse', 'HEAD')).toBe(head)
    // The main working tree must be clean of tracked changes; an untracked file alone does not block the merge.
    writeFileSync(join(repo, 'dirty.txt'), 'x\n')
    expect(await mergeWorkspace(subprocess(), worktree, signal)).toMatchObject({ branch: 'agent/t', target: git('branch', '--show-current') })
    writeFileSync(join(repo, 'a.txt'), 'dirty\n')
    await expect(mergeWorkspace(subprocess(), worktree, signal)).rejects.toThrow(/uncommitted/)
  })
  it('squashes a ticket branch into one commit when asked', async () => {
    base()
    const worktree = join(dir, 'worktree-squash')
    git('worktree', 'add', '-q', '-b', 'agent/q', worktree)
    writeFileSync(join(worktree, 'q.txt'), 'squashed\n')
    gitAt(worktree, 'add', 'q.txt'); gitAt(worktree, 'commit', '-q', '-m', 'ticket work')
    await mergeWorkspace(subprocess(), worktree, signal, 'squash')
    expect(git('log', '--oneline', '-1')).toMatch(/Squash merge agent\/q/)
    // One parent: the squash commit, not the ticket branch's commit.
    expect(git('rev-list', '--parents', '-n1', 'HEAD').split(' ')).toHaveLength(2)
    expect(readFileSync(join(repo, 'q.txt'), 'utf8')).toBe('squashed\n')
    // The branch is not an ancestor after a squash, so the safe check refuses; the recorded squash merge allows force removal.
    await expect(discardWorkspace(subprocess(), worktree, signal, { merged: true })).rejects.toThrow(/does not contain/)
    await discardWorkspace(subprocess(), worktree, signal, { merged: true, squash: true })
    expect(git('branch', '--list', 'agent/q')).toBe('')
    expect(existsSync(worktree)).toBe(false)
  })
  it('reverts the merge it recorded, on the fast-forward commit and on a merge commit', async () => {
    base()
    const worktree = join(dir, 'worktree-revert')
    git('worktree', 'add', '-q', '-b', 'agent/rv', worktree)
    writeFileSync(join(worktree, 'rv.txt'), 'rv\n')
    gitAt(worktree, 'add', 'rv.txt'); gitAt(worktree, 'commit', '-q', '-m', 'revert me')
    const merged = await mergeWorkspace(subprocess(), worktree, signal)
    expect(existsSync(join(repo, 'rv.txt'))).toBe(true)
    // The record names the repository the merge moved, so a revert needs no worktree.
    const undone = await revertWorkspace(subprocess(), merged.root, signal, merged.head!, merged.target)
    expect(undone).toMatchObject({ target: git('branch', '--show-current') })
    expect(undone.head).toBe(git('rev-parse', 'HEAD'))
    expect(existsSync(join(repo, 'rv.txt'))).toBe(false)
    // A merge commit is undone against its first parent, which keeps the repository's own commits.
    const other = join(dir, 'worktree-revert-merge')
    git('worktree', 'add', '-q', '-b', 'agent/rm', other)
    writeFileSync(join(other, 'rm.txt'), 'rm\n')
    gitAt(other, 'add', 'rm.txt'); gitAt(other, 'commit', '-q', '-m', 'merge me')
    const commit = await mergeWorkspace(subprocess(), other, signal, 'merge')
    expect(git('rev-list', '--parents', '-n1', commit.head!).split(' ')).toHaveLength(3)
    await revertWorkspace(subprocess(), commit.root, signal, commit.head!, commit.target)
    expect(existsSync(join(repo, 'rm.txt'))).toBe(false)
  })
  it('leaves the branch where it was when the revert conflicts, and refuses one git cannot start', async () => {
    base()
    const worktree = join(dir, 'worktree-revert-conflict')
    git('worktree', 'add', '-q', '-b', 'agent/rc', worktree)
    writeFileSync(join(worktree, 'a.txt'), 'ticket\n')
    gitAt(worktree, 'add', 'a.txt'); gitAt(worktree, 'commit', '-q', '-m', 'ticket change')
    const merged = await mergeWorkspace(subprocess(), worktree, signal)
    // A later change to the file the merge moved makes the reverting commit conflict.
    writeFileSync(join(repo, 'a.txt'), 'moved on\n')
    git('add', 'a.txt'); git('commit', '-q', '-m', 'move past the merge')
    const head = git('rev-parse', 'HEAD')
    await expect(revertWorkspace(subprocess(), merged.root, signal, merged.head!, merged.target))
      .rejects.toBeInstanceOf(RevertConflictError)
    // The abort leaves the branch on its commit, with nothing half-applied.
    expect(git('rev-parse', 'HEAD')).toBe(head)
    expect(git('status', '--porcelain')).toBe('')
    // A dirty repository and an unknown commit are named refusals, not half-finished reverts.
    writeFileSync(join(repo, 'a.txt'), 'dirty\n')
    await expect(revertWorkspace(subprocess(), merged.root, signal, merged.head!, merged.target)).rejects.toThrow(/uncommitted/)
    await expect(revertWorkspace(subprocess(), merged.root, signal, 'f'.repeat(40), merged.target)).rejects.toThrow(/is not in/)
    await expect(revertWorkspace(subprocess(), merged.root, signal, merged.head!, 'other')).rejects.toThrow(/not the branch that merge moved/)
  })
  it('makes a merge commit when asked, keeping both parents', async () => {
    base()
    const worktree = join(dir, 'worktree-merge')
    git('worktree', 'add', '-q', '-b', 'agent/m', worktree)
    writeFileSync(join(worktree, 'm.txt'), 'merged\n')
    gitAt(worktree, 'add', 'm.txt'); gitAt(worktree, 'commit', '-q', '-m', 'ticket work')
    await mergeWorkspace(subprocess(), worktree, signal, 'merge')
    expect(git('rev-list', '--parents', '-n1', 'HEAD').split(' ')).toHaveLength(3)
    expect(git('log', '--oneline', '-1')).toMatch(/Merge branch 'agent\/m'/)
  })
  it('names the rebase when the repository moved past the ticket branch, then rebases so the merge can fast-forward', async () => {
    base()
    const worktree = join(dir, 'worktree-rebase')
    git('worktree', 'add', '-q', '-b', 'agent/r', worktree)
    writeFileSync(join(worktree, 'b.txt'), 'ticket\n')
    gitAt(worktree, 'add', 'b.txt'); gitAt(worktree, 'commit', '-q', '-m', 'ticket work')
    // The repository takes a commit of its own, so the ticket branch is no longer a fast-forward.
    writeFileSync(join(repo, 'a.txt'), 'main moved\n'); git('commit', '-qam', 'main work')
    await expect(mergeWorkspace(subprocess(), worktree, signal)).rejects.toThrow(/rebase the ticket branch onto/)
    expect(await targetContained(subprocess(), worktree, signal)).toBe(false)
    const rebased = await rebaseWorkspace(subprocess(), worktree, signal)
    expect(await targetContained(subprocess(), worktree, signal)).toBe(true)
    // A branch that rebased the repository in forked from its new commit, which is where its review starts.
    expect(await workingBase(subprocess(), { execution: { name: 't1', directory: worktree, assignment: '',
      base: rebased.base, target: git('branch', '--show-current') } } as Ticket, signal)).toBe(git('rev-parse', 'HEAD'))
    expect(rebased).toEqual({ branch: 'agent/r', target: git('branch', '--show-current'), base: git('rev-parse', 'HEAD') })
    // With the recorded base moved to the target's commit, the review lists only the ticket's own commit and file.
    const review = (await captureReview(subprocess(), { ...ticket(rebased.base),
      execution: { name: 't1', directory: worktree, assignment: '', base: rebased.base } }, signal))!
    expect(review.commits.map(commit => commit.subject)).toEqual(['ticket work'])
    expect(review.files).toEqual([{ status: 'A', path: 'b.txt' }])
    expect(await mergeWorkspace(subprocess(), worktree, signal)).toMatchObject({ branch: 'agent/r', target: git('branch', '--show-current') })
    expect(git('log', '--oneline', '-1')).toMatch(/ticket work/)
    // The rebased history sits on top of the repository commit.
    expect(git('log', '--oneline', '-2').split('\n').map(line => line.replace(/^[0-9a-f]+ /, '')))
      .toEqual(['ticket work', 'main work'])
  })
  it('aborts a conflicted rebase, keeps the ticket commits, and names the files that conflict', async () => {
    base()
    const worktree = join(dir, 'worktree-conflict')
    git('worktree', 'add', '-q', '-b', 'agent/c', worktree)
    writeFileSync(join(worktree, 'a.txt'), 'ticket\n')
    gitAt(worktree, 'commit', '-qam', 'ticket edit')
    const ticketHead = gitAt(worktree, 'rev-parse', 'HEAD')
    writeFileSync(join(repo, 'a.txt'), 'main\n')
    git('commit', '-qam', 'main edit')
    // The typed conflict names the target and the files, which is what hands the work back to the worker.
    const conflict = await rebaseWorkspace(subprocess(), worktree, signal).then(() => undefined, (error: unknown) => error)
    expect(conflict).toBeInstanceOf(RebaseConflictError)
    expect(conflict).toMatchObject({ target: git('branch', '--show-current'), files: ['a.txt'] })
    // The abort leaves the ticket branch on its own commit, with no rebase in progress.
    expect(gitAt(worktree, 'rev-parse', 'HEAD')).toBe(ticketHead)
    expect(gitAt(worktree, 'status', '--porcelain')).toBe('')
  })
  it('starts the review at the fork point when the branch merged the repository in', async () => {
    const launched = base()
    const worktree = join(dir, 'worktree-fork')
    git('worktree', 'add', '-q', '-b', 'agent/f', worktree)
    writeFileSync(join(worktree, 'b.txt'), 'ticket\n')
    gitAt(worktree, 'add', 'b.txt'); gitAt(worktree, 'commit', '-q', '-m', 'ticket work')
    writeFileSync(join(repo, 'a.txt'), 'main moved\n'); git('commit', '-qam', 'main work')
    const target = git('branch', '--show-current')
    gitAt(worktree, 'merge', '-q', '--no-edit', target)
    const execution = { name: 't1', directory: worktree, assignment: '', base: launched, target }
    const anchored = (await workingBase(subprocess(), { execution } as Ticket, signal))!
    expect(anchored).toBe(git('rev-parse', 'HEAD'))
    // The repository's own change is not the ticket's, so the review lists only the file the ticket added.
    const review = (await captureReview(subprocess(), { execution: { ...execution, base: anchored } } as Ticket, signal))!
    expect(review.files).toEqual([{ status: 'A', path: 'b.txt' }])
    // The repository's own commit is not the ticket's, so it is not listed.
    expect(review.commits.map(commit => commit.subject)).toContain('ticket work')
    expect(review.commits.map(commit => commit.subject)).not.toContain('main work')
  })
  it('removes a merged worktree and branch with git\'s safe deletion check', async () => {
    base()
    const worktree = join(dir, 'worktree-clean')
    git('worktree', 'add', '-q', '-b', 'agent/clean', worktree)
    writeFileSync(join(worktree, 'b.txt'), 'merged\n')
    gitAt(worktree, 'add', 'b.txt'); gitAt(worktree, 'commit', '-q', '-m', 'merged work')
    await mergeWorkspace(subprocess(), worktree, signal)
    await discardWorkspace(subprocess(), worktree, signal, { merged: true })
    expect(existsSync(worktree)).toBe(false)
    expect(() => git('show-ref', '--verify', '--quiet', 'refs/heads/agent/clean')).toThrow()
  })
  it('refuses to merge or rebase when the repository is on another branch than the recorded target', async () => {
    base()
    const worktree = join(dir, 'worktree-target')
    git('worktree', 'add', '-q', '-b', 'agent/t', worktree)
    writeFileSync(join(worktree, 'x.txt'), 'x\n')
    gitAt(worktree, 'add', 'x.txt'); gitAt(worktree, 'commit', '-q', '-m', 'work')
    await expect(mergeWorkspace(subprocess(), worktree, signal, 'ff', 'other')).rejects.toThrow(/not the Ticket's target/)
    await expect(rebaseWorkspace(subprocess(), worktree, signal, 'other')).rejects.toThrow(/not the Ticket's target/)
    // The recorded target still merges.
    expect(await mergeWorkspace(subprocess(), worktree, signal, 'ff', git('branch', '--show-current')))
      .toMatchObject({ branch: 'agent/t', target: git('branch', '--show-current') })
  })
  it('refuses a merged cleanup when the target does not contain the branch, keeping its worktree', async () => {
    base()
    const worktree = join(dir, 'worktree-unmerged-clean')
    git('worktree', 'add', '-q', '-b', 'agent/unmerged', worktree)
    writeFileSync(join(worktree, 'b.txt'), 'not merged\n')
    gitAt(worktree, 'add', 'b.txt'); gitAt(worktree, 'commit', '-q', '-m', 'not merged')
    await expect(discardWorkspace(subprocess(), worktree, signal, { merged: true })).rejects.toThrow(/does not contain/)
    expect(existsSync(join(worktree, 'b.txt'))).toBe(true)
  })
  it('pushes the repository branch to its upstream, and says when it has none', async () => {
    const remote = join(dir, 'origin.git')
    execFileSync('git', ['init', '-q', '--bare', remote])
    git('init', '-q')
    writeFileSync(join(repo, 'a.txt'), 'one\n')
    git('add', 'a.txt'); git('commit', '-q', '-m', 'base')
    const branch = git('branch', '--show-current')
    git('remote', 'add', 'origin', remote)
    git('push', '-q', '-u', 'origin', branch)
    writeFileSync(join(repo, 'a.txt'), 'two\n')
    git('add', 'a.txt'); git('commit', '-q', '-m', 'work')
    await pushWorkspace(subprocess(), repo, signal)
    expect(gitAt(remote, 'rev-parse', branch)).toBe(git('rev-parse', 'HEAD'))
    // A repository that names no upstream fails without touching the branch.
    const other = join(dir, 'no-upstream')
    mkdirSync(other)
    execFileSync('git', ['init', '-q'], { cwd: other })
    writeFileSync(join(other, 'b.txt'), 'x\n')
    gitAt(other, 'add', 'b.txt'); gitAt(other, 'commit', '-q', '-m', 'base')
    await expect(pushWorkspace(subprocess(), other, signal)).rejects.toThrow(/no upstream|destination|upstream/i)
  })
  it('removes a linked worktree and its branch, and refuses the repository itself', async () => {
    base()
    const worktree = join(dir, 'worktree-discard')
    git('worktree', 'add', '-q', '-b', 'agent/discard', worktree)
    await discardWorkspace(subprocess(), worktree, signal)
    expect(existsSync(worktree)).toBe(false)
    expect(() => git('show-ref', '--verify', '--quiet', 'refs/heads/agent/discard')).toThrow()
    await expect(discardWorkspace(subprocess(), repo, signal)).rejects.toThrow(/repository itself/)
  })
})

import { execFileSync, spawn } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import type { SubprocessRuntime } from '@deepseek-ai/dsh-subprocess'
import type { Ticket } from '../src/core/index.ts'
import { captureReview, discardWorkspace, mergeWorkspace, rebaseWorkspace, targetContained, workspaceDiff, workspaceHead } from '../src/app/review.ts'

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
    expect(await mergeWorkspace(subprocess(), worktree, signal)).toEqual({ branch: 'agent/t', target: git('branch', '--show-current') })
    expect(git('rev-parse', 'HEAD')).toBe(head)
    // The main working tree must be clean of tracked changes; an untracked file alone does not block the merge.
    writeFileSync(join(repo, 'dirty.txt'), 'x\n')
    expect(await mergeWorkspace(subprocess(), worktree, signal)).toEqual({ branch: 'agent/t', target: git('branch', '--show-current') })
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
    expect(rebased).toEqual({ branch: 'agent/r', target: git('branch', '--show-current'), base: git('rev-parse', 'HEAD') })
    // With the recorded base moved to the target's commit, the review lists only the ticket's own commit and file.
    const review = (await captureReview(subprocess(), { ...ticket(rebased.base),
      execution: { name: 't1', directory: worktree, assignment: '', base: rebased.base } }, signal))!
    expect(review.commits.map(commit => commit.subject)).toEqual(['ticket work'])
    expect(review.files).toEqual([{ status: 'A', path: 'b.txt' }])
    expect(await mergeWorkspace(subprocess(), worktree, signal)).toEqual({ branch: 'agent/r', target: git('branch', '--show-current') })
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
    await expect(rebaseWorkspace(subprocess(), worktree, signal)).rejects.toThrow(/conflicts in a\.txt/)
    // The abort leaves the ticket branch on its own commit, with no rebase in progress.
    expect(gitAt(worktree, 'rev-parse', 'HEAD')).toBe(ticketHead)
    expect(gitAt(worktree, 'status', '--porcelain')).toBe('')
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
      .toEqual({ branch: 'agent/t', target: git('branch', '--show-current') })
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

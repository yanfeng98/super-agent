/** Reading a review patch into rows, so a note can anchor on a line the person clicked. */
import { expect, it } from 'vitest'
import { patchLines } from '../src/client/diff.ts'

it('reads a patch into rows that carry the file line each one occupies', () => {
  const patch = ['diff --git a/src/a.ts b/src/a.ts', 'index 1..2 100644', '--- a/src/a.ts', '+++ b/src/a.ts',
    '@@ -1,3 +1,4 @@', ' one', '-two', '+2', '+2b', ' three'].join('\n')
  expect(patchLines(patch)).toEqual([
    { kind: 'meta', text: 'diff --git a/src/a.ts b/src/a.ts' },
    { kind: 'meta', text: 'index 1..2 100644' },
    { kind: 'meta', text: '--- a/src/a.ts' },
    { kind: 'meta', text: '+++ b/src/a.ts' },
    { kind: 'hunk', text: '@@ -1,3 +1,4 @@' },
    { kind: 'context', text: ' one', oldLine: 1, newLine: 1 },
    // A removed line has no line in the file after the change, so it anchors no note.
    { kind: 'del', text: '-two', oldLine: 2 },
    { kind: 'add', text: '+2', newLine: 2 },
    { kind: 'add', text: '+2b', newLine: 3 },
    { kind: 'context', text: ' three', oldLine: 3, newLine: 4 },
  ])
})

it('keeps a hunk header and a no-newline marker out of the line numbers', () => {
  const patch = ['--- a/b.ts', '+++ b/b.ts', '@@ -1 +1 @@', '-old', '+new', '\\ No newline at end of file'].join('\n')
  expect(patchLines(patch)).toEqual([
    { kind: 'meta', text: '--- a/b.ts' },
    { kind: 'meta', text: '+++ b/b.ts' },
    { kind: 'hunk', text: '@@ -1 +1 @@' },
    { kind: 'del', text: '-old', oldLine: 1 },
    { kind: 'add', text: '+new', newLine: 1 },
    { kind: 'meta', text: '\\ No newline at end of file' },
  ])
})

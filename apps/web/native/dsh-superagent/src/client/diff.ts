/** Reading a review patch into rows, so the panel can anchor a note on a line of it. */

/** One rendered row of a patch: what it is, its text, and the file line it occupies. */
export interface PatchLine {
  kind: 'add' | 'del' | 'context' | 'meta' | 'hunk'
  text: string
  oldLine?: number
  /** The line in the file after the change, which a note anchors on; a removed line has none. */
  newLine?: number
}

/** File metadata a `diff --git` block may carry, which occupies no line of either side. */
const META = /^(diff --git|index |new file|deleted file|old mode|new mode|similarity|rename |copy |Binary files)/

/**
 * Reads one `diff --git` block into rows that carry the file lines they occupy, so a person can comment on a line
 * instead of typing its number.
 * @param patch - The block `parseDiff` returned for one file.
 * @returns Its rows in patch order; a hunk header and file metadata carry no line.
 */
export function patchLines(patch: string): PatchLine[] {
  const rows: PatchLine[] = []
  let oldNo = 0
  let newNo = 0
  for (const text of patch.split('\n')) {
    const hunk = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(text)
    if (hunk) {
      oldNo = Number(hunk[1]) - 1
      newNo = Number(hunk[2]) - 1
      rows.push({ kind: 'hunk', text })
      continue
    }
    if (text.startsWith('---') || text.startsWith('+++') || META.test(text)) {
      rows.push({ kind: 'meta', text })
      continue
    }
    if (text.startsWith('+')) {
      newNo++
      rows.push({ kind: 'add', text, newLine: newNo })
      continue
    }
    if (text.startsWith('-')) {
      oldNo++
      // A removed line has no line in the file after the change, so a note anchors on a surrounding line instead.
      rows.push({ kind: 'del', text, oldLine: oldNo })
      continue
    }
    if (text.startsWith('\\')) {
      rows.push({ kind: 'meta', text })
      continue
    }
    // The empty string after the final newline is not a context line.
    if (!text) continue
    oldNo++
    newNo++
    rows.push({ kind: 'context', text, oldLine: oldNo, newLine: newNo })
  }
  return rows
}

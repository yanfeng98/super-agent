/** Where this installation's checkout sits, and the git repository Tickets work in by default. */
import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

/**
 * This package's directory: `src/`'s parent in a source launch and `lib/`'s parent in a build, so the same module
 * answers the installed checkout in both.
 */
export const installationRoot = (): string => fileURLToPath(new URL('..', import.meta.url))

/**
 * The git working tree that contains `start`, found by walking up to the first `.git` entry. A checkout inside a
 * repository therefore answers that repository's working tree; an installation outside every repository answers
 * `undefined`, and Tickets keep their empty directory then.
 * @param start - Absolute directory to search from; defaults to this installation's package directory.
 * @returns The enclosing working tree directory, or `undefined` when no parent holds a `.git` entry.
 */
export function installedRepository(start: string = installationRoot()): string | undefined {
  for (let directory = start; ; directory = dirname(directory)) {
    if (existsSync(join(directory, '.git'))) return directory
    if (dirname(directory) === directory) return undefined
  }
}

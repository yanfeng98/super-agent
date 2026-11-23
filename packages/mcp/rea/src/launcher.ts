/**
 * Resolution of the rea launcher that this repository builds from the source
 * vendored at `native/rea`.
 *
 * Kept out of the plugin entry because the Cordis Loader reads only a function
 * plugin's `name`, `inject`, `Config` and `apply` from an entry module.
 */

import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

/** Vendored rea tree root, resolved once from this package. */
const VENDORED_ROOT = fileURLToPath(new URL('../../../../native/rea/', import.meta.url))

/**
 * The launcher the vendored build produces.
 * @returns Absolute path to `native/rea/scripts/rea.mjs`, resolved from this
 *   package's own `import.meta.url`.
 */
export function vendoredLauncher(): string {
  return join(VENDORED_ROOT, 'scripts', 'rea.mjs')
}

/**
 * The launcher to spawn: the configured one, else the vendored build.
 * @param launcher - the plugin's explicit `launcher` override.
 * @returns Absolute path or command name passed to the server process.
 */
export function resolveLauncher(launcher: string | undefined): string {
  return launcher ?? vendoredLauncher()
}

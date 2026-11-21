/**
 * Resolution of the rust-anything executable that `pnpm run build` stages from
 * the crate vendored at `native/rust-anything`.
 *
 * Kept out of the plugin entry because the Cordis Loader reads only a function
 * plugin's `name`, `inject`, `Config` and `apply` from an entry module.
 */

import { fileURLToPath } from 'node:url'

/**
 * The executable name one platform uses.
 * @param platform - `process.platform`.
 * @returns `rust-anything.exe` on Windows, `rust-anything` elsewhere.
 */
export function executableName(platform: string): string {
  return platform === 'win32' ? 'rust-anything.exe' : 'rust-anything'
}

/**
 * The executable this repository's build stages from the vendored crate.
 * @param platform - `process.platform`.
 * @returns Absolute path inside `native/rust-anything/bin`; the caller does not
 *   check that it exists, because `command` and `RUST_ANYTHING_BIN` may take
 *   precedence over a checkout that never ran the build.
 */
export function vendoredBinary(platform: string): string {
  return fileURLToPath(new URL(`../../../../native/rust-anything/bin/${executableName(platform)}`, import.meta.url))
}

/**
 * The command to start: the configured executable, `RUST_ANYTHING_BIN`, else
 * the executable this checkout built from source.
 * @param command - the plugin's explicit `command` override.
 * @param env - environment to read `RUST_ANYTHING_BIN` from.
 * @returns Executable path or command name.
 */
export function resolveCommand(command: string | undefined, env: NodeJS.ProcessEnv = process.env): string {
  return command ?? env.RUST_ANYTHING_BIN ?? vendoredBinary(process.platform)
}

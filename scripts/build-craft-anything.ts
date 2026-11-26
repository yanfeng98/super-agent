/**
 * Build the vendored craft-anything CLI from source and stage its executable
 * for the MCP plugin.
 *
 * `native/craft-anything` is the upstream Rust workspace at its pinned tag;
 * `pnpm run build` compiles its `craft-anything-cli` package here so a checkout
 * needs no published binary. The staged executable lives under
 * `native/craft-anything/bin/`, next to the source that produces it;
 * `packages/mcp/craft-anything` resolves that path.
 *
 * The workspace declares `rust-version = "1.97"` and the vendored copy adds a
 * `rust-toolchain.toml` pin, so rustup selects the required toolchain instead
 * of failing with an older default. A missing toolchain fails here with the
 * path to the vendored source's requirements.
 */

import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, renameSync, rmSync, statSync } from 'node:fs'
import { join, resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')

/** Directory holding the vendored upstream Rust workspace. */
export const CRAFT_ANYTHING_ROOT = join(root, 'native', 'craft-anything')

/**
 * Executable name the workspace emits on one platform.
 * @param platform - `process.platform`.
 * @returns `craft-anything-cli.exe` on Windows, `craft-anything-cli` elsewhere.
 */
export function executableName(platform: string): string {
  return platform === 'win32' ? 'craft-anything-cli.exe' : 'craft-anything-cli'
}

/**
 * Where `cargo build --release -p craft-anything-cli` writes the executable.
 * @param platform - `process.platform`.
 * @returns Absolute path under the vendored workspace's `target/release`.
 */
export function cargoOutput(platform: string): string {
  return join(CRAFT_ANYTHING_ROOT, 'target', 'release', executableName(platform))
}

/**
 * Where the build stages the executable for the plugin to spawn.
 * @param platform - `process.platform`.
 * @returns Absolute path under the vendored workspace's `bin`.
 */
export function stagedBinary(platform: string): string {
  return join(CRAFT_ANYTHING_ROOT, 'bin', executableName(platform))
}

/**
 * Compile the vendored workspace and stage its CLI executable.
 * @returns The staged executable's absolute path.
 * @throws If cargo is missing, the compile fails, or no executable appears.
 */
export function buildCraftAnything(): string {
  const platform = process.platform
  // The toolchain file applies to cargo's working directory, not its manifest,
  // so the compile must run from the vendored workspace root.
  const compiled = spawnSync('cargo', ['build', '--release', '--locked', '-p', 'craft-anything-cli'], {
    cwd: CRAFT_ANYTHING_ROOT,
    stdio: 'inherit',
  })
  if (compiled.error !== undefined) {
    throw new Error(
      `build:craft-anything: cannot run cargo (${compiled.error.message}). Install Rust 1.97 through rustup; see native/craft-anything/NATIVE.md.`,
    )
  }
  if (compiled.status !== 0) {
    throw new Error(
      `build:craft-anything: cargo build --release --locked -p craft-anything-cli exited with ${String(compiled.status)}; the vendored workspace needs Rust 1.97 or newer. See native/craft-anything/NATIVE.md.`,
    )
  }

  const produced = cargoOutput(platform)
  if (!existsSync(produced)) {
    throw new Error(
      `build:craft-anything: cargo finished without ${produced}; unset CARGO_TARGET_DIR, which moves the workspace's output outside its own target directory.`,
    )
  }
  const staged = stagedBinary(platform)
  // Stage beside the destination and rename: overwriting a binary an MCP server
  // is currently executing fails with ETXTBSY on POSIX, while a rename swaps the
  // directory entry and leaves the running process on its old inode.
  const temporary = `${staged}.tmp`
  mkdirSync(join(CRAFT_ANYTHING_ROOT, 'bin'), { recursive: true })
  copyFileSync(produced, temporary)
  try {
    renameSync(temporary, staged)
  } catch (error) {
    rmSync(temporary, { force: true })
    throw new Error(
      `build:craft-anything: cannot replace ${staged} (${error instanceof Error ? error.message : String(error)}). Stop the running DSH session that spawns it, then build again.`,
    )
  }
  console.log(`build:craft-anything: staged ${staged} (${String(Math.round(statSync(staged).size / 1024))} KiB)`)
  return staged
}

if (import.meta.main) buildCraftAnything()

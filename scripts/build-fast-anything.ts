/**
 * Build the vendored fast-anything CLI from source and stage its executable for
 * the MCP plugin.
 *
 * `native/fast-anything` is the upstream crate at its pinned tag; `pnpm run
 * build` compiles it here so a checkout needs no published npm binary. The
 * staged executable lives under `native/fast-anything/bin/`, next to the source
 * that produces it; `packages/mcp/fast-anything` resolves that path.
 *
 * The crate declares `rust-version = "1.88"` and ships no toolchain file, so the
 * caller's default rustup toolchain compiles it. A missing or too-old toolchain
 * fails here with the path to the vendored source's requirements.
 */

import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, renameSync, rmSync, statSync } from 'node:fs'
import { join, resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')

/** Directory holding the vendored upstream crate. */
export const FAST_ANYTHING_ROOT = join(root, 'native', 'fast-anything')

/**
 * Executable name the crate emits on one platform.
 * @param platform - `process.platform`.
 * @returns `fast-anything.exe` on Windows, `fast-anything` elsewhere.
 */
export function executableName(platform: string): string {
  return platform === 'win32' ? 'fast-anything.exe' : 'fast-anything'
}

/**
 * Where `cargo build --release` writes the executable.
 * @param platform - `process.platform`.
 * @returns Absolute path under the vendored crate's `target/release`.
 */
export function cargoOutput(platform: string): string {
  return join(FAST_ANYTHING_ROOT, 'target', 'release', executableName(platform))
}

/**
 * Where the build stages the executable for the plugin to spawn.
 * @param platform - `process.platform`.
 * @returns Absolute path under the vendored crate's `bin`.
 */
export function stagedBinary(platform: string): string {
  return join(FAST_ANYTHING_ROOT, 'bin', executableName(platform))
}

/**
 * Compile the vendored crate and stage its executable.
 * @returns The staged executable's absolute path.
 * @throws If cargo is missing, the compile fails, or no executable appears.
 */
export function buildFastAnything(): string {
  const platform = process.platform
  const compiled = spawnSync('cargo', ['build', '--release', '--locked'], {
    cwd: FAST_ANYTHING_ROOT,
    stdio: 'inherit',
  })
  if (compiled.error !== undefined) {
    throw new Error(
      `build:fast-anything: cannot run cargo (${compiled.error.message}). Install Rust 1.88 or newer through rustup; see native/fast-anything/NATIVE.md.`,
    )
  }
  if (compiled.status !== 0) {
    throw new Error(
      `build:fast-anything: cargo build --release --locked exited with ${String(compiled.status)}; the vendored crate needs Rust 1.88 or newer. See native/fast-anything/NATIVE.md.`,
    )
  }

  const produced = cargoOutput(platform)
  if (!existsSync(produced)) {
    throw new Error(
      `build:fast-anything: cargo finished without ${produced}; unset CARGO_TARGET_DIR, which moves the crate's output outside its own target directory.`,
    )
  }
  const staged = stagedBinary(platform)
  // Stage beside the destination and rename: overwriting a binary an MCP server
  // is currently executing fails with ETXTBSY on POSIX, while a rename swaps the
  // directory entry and leaves the running process on its old inode.
  const temporary = `${staged}.tmp`
  mkdirSync(join(FAST_ANYTHING_ROOT, 'bin'), { recursive: true })
  copyFileSync(produced, temporary)
  try {
    renameSync(temporary, staged)
  } catch (error) {
    rmSync(temporary, { force: true })
    throw new Error(
      `build:fast-anything: cannot replace ${staged} (${error instanceof Error ? error.message : String(error)}). Stop the running DSH session that spawns it, then build again.`,
    )
  }
  console.log(`build:fast-anything: staged ${staged} (${String(Math.round(statSync(staged).size / 1024))} KiB)`)
  return staged
}

if (import.meta.main) buildFastAnything()

/**
 * Build the vendored rust-anything CLI from source and stage its executable for
 * the MCP plugin.
 *
 * `native/rust-anything` is the upstream crate at its pinned tag; `pnpm run
 * build` compiles it here so a checkout needs no published npm binary. The
 * staged executable lives under `native/rust-anything/bin/`, next to the source
 * that produces it; `packages/mcp/rust-anything` resolves that path.
 *
 * The crate declares `rust-version = "1.97"`, and its `rust-toolchain.toml`
 * pins that channel, so rustup selects (and installs) the required toolchain
 * instead of failing the build on an older default.
 */

import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, renameSync, rmSync, statSync } from 'node:fs'
import { join, resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')

/** Directory holding the vendored upstream crate. */
export const RUST_ANYTHING_ROOT = join(root, 'native', 'rust-anything')

/**
 * Executable name the crate emits on one platform.
 * @param platform - `process.platform`.
 * @returns `rust-anything.exe` on Windows, `rust-anything` elsewhere.
 */
export function executableName(platform: string): string {
  return platform === 'win32' ? 'rust-anything.exe' : 'rust-anything'
}

/** Where `cargo build --release` writes the executable. */
export function cargoOutput(platform: string): string {
  return join(RUST_ANYTHING_ROOT, 'target', 'release', executableName(platform))
}

/** Where the build stages the executable for the plugin to spawn. */
export function stagedBinary(platform: string): string {
  return join(RUST_ANYTHING_ROOT, 'bin', executableName(platform))
}

/**
 * Compile the vendored crate and stage its executable.
 * @returns The staged executable's absolute path.
 * @throws If cargo is missing, the compile fails, or no executable appears.
 */
export function buildRustAnything(): string {
  const platform = process.platform
  // The toolchain file applies to cargo's working directory, not its manifest,
  // so the compile must run from the crate root.
  const compiled = spawnSync('cargo', ['build', '--release', '--locked'], {
    cwd: RUST_ANYTHING_ROOT,
    stdio: 'inherit',
  })
  if (compiled.error !== undefined) {
    throw new Error(
      `build:rust-anything: cannot run cargo (${compiled.error.message}). Install Rust 1.97 through rustup; see native/rust-anything/NATIVE.md.`,
    )
  }
  if (compiled.status !== 0) {
    throw new Error(`build:rust-anything: cargo build --release --locked exited with ${String(compiled.status)}`)
  }

  const produced = cargoOutput(platform)
  if (!existsSync(produced)) {
    throw new Error(
      `build:rust-anything: cargo finished without ${produced}; unset CARGO_TARGET_DIR, which moves the crate's output outside its own target directory.`,
    )
  }
  const staged = stagedBinary(platform)
  // Stage beside the destination and rename: overwriting a binary an MCP server
  // is currently executing fails with ETXTBSY on POSIX, while a rename swaps the
  // directory entry and leaves the running process on its old inode.
  const temporary = `${staged}.tmp`
  mkdirSync(join(RUST_ANYTHING_ROOT, 'bin'), { recursive: true })
  copyFileSync(produced, temporary)
  try {
    renameSync(temporary, staged)
  } catch (error) {
    rmSync(temporary, { force: true })
    throw new Error(
      `build:rust-anything: cannot replace ${staged} (${error instanceof Error ? error.message : String(error)}). Stop the running DSH session that spawns it, then build again.`,
    )
  }
  console.log(`build:rust-anything: staged ${staged} (${String(Math.round(statSync(staged).size / 1024))} KiB)`)
  return staged
}

if (import.meta.main) buildRustAnything()

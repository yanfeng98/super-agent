/**
 * Build the vendored rea MCP server from source for the Web `standard` preset.
 *
 * `native/rea` is the upstream repository at its pinned release; `pnpm run
 * build` installs its npm dependencies and runs its own build, so a checkout
 * needs no published tarball and no global install. The generated `dist/` and
 * `node_modules/` stay beside the vendored source but never enter git;
 * `packages/mcp/rea` spawns `native/rea/scripts/rea.mjs` from the same tree.
 *
 * The upstream package declares Node.js `^22.23.4 || ^24.22.1 || >=26.12.1`
 * and enables `engine-strict`, while this repository requires Node.js
 * `^24.22.3 || >=26.13.1`, so a supported harness runtime already satisfies it.
 */

import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { delimiter, dirname, join, resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')

/** Directory holding the vendored upstream repository. */
export const REA_ROOT = join(root, 'native', 'rea')

/**
 * Package-manager executable to install and build with.
 * @param platform - `process.platform`.
 * @returns `npm.cmd` on Windows, `npm` elsewhere.
 */
export function npmCommand(platform: string): string {
  return platform === 'win32' ? 'npm.cmd' : 'npm'
}

/**
 * Entry script the plugin spawns as `<node> <launcher> mcp`.
 * @param reaRoot - vendored source root; defaults to {@link REA_ROOT}.
 * @returns Absolute path to `scripts/rea.mjs` inside the vendored tree.
 */
export function launcherPath(reaRoot: string = REA_ROOT): string {
  return join(reaRoot, 'scripts', 'rea.mjs')
}

/**
 * Compiled MCP entrypoint the launcher requires; its absence means the build
 * did not finish.
 * @param reaRoot - vendored source root; defaults to {@link REA_ROOT}.
 * @returns Absolute path to `dist/mcp/main.js`.
 */
export function compiledEntry(reaRoot: string = REA_ROOT): string {
  return join(reaRoot, 'dist', 'mcp', 'main.js')
}

/**
 * Stamp recording which `package-lock.json` a successful install produced.
 * @param reaRoot - vendored source root; defaults to {@link REA_ROOT}.
 * @returns Absolute path to the stamp file under `node_modules`.
 */
export function installStampPath(reaRoot: string = REA_ROOT): string {
  return join(reaRoot, 'node_modules', '.dsh-rea-install')
}

/**
 * Digest of the lockfile that decides whether the installed dependencies are
 * current.
 * @param reaRoot - vendored source root; defaults to {@link REA_ROOT}.
 * @returns Lowercase hex SHA-256 of `package-lock.json`.
 */
export function lockfileDigest(reaRoot: string = REA_ROOT): string {
  return createHash('sha256').update(readFileSync(join(reaRoot, 'package-lock.json'))).digest('hex')
}

/**
 * Whether the installed dependencies came from the current lockfile. A
 * checkout that never ran the build, or one whose lockfile changed, installs
 * them again.
 * @param reaRoot - vendored source root; defaults to {@link REA_ROOT}.
 * @returns `true` when the stamp records the current lockfile digest.
 */
export function dependenciesCurrent(reaRoot: string = REA_ROOT): boolean {
  try {
    return readFileSync(installStampPath(reaRoot), 'utf8').trim() === lockfileDigest(reaRoot)
  } catch (_missingInstallStamp) {
    // A missing stamp or lockfile means the install is stale or never ran; the
    // following `npm ci` reports anything worse with npm's own diagnostics.
    return false
  }
}

/**
 * Run one npm command in the vendored tree with the running Node.js first on
 * `PATH`, so npm's shebang resolves the same runtime that enforces the engine
 * floor here.
 * @param args - npm arguments, such as `['ci', '--no-audit', '--no-fund']`.
 * @param purpose - human-readable phase used in failure messages.
 * @throws If npm cannot start or exits nonzero.
 */
function runNpm(args: readonly string[], purpose: string): void {
  const result = spawnSync(npmCommand(process.platform), [...args], {
    cwd: REA_ROOT,
    stdio: 'inherit',
    env: {
      ...process.env,
      HUSKY: '0',
      PATH: `${dirname(process.execPath)}${delimiter}${process.env.PATH ?? ''}`,
    },
    // Windows resolves `npm.cmd` only through a shell; every argument is fixed here.
    shell: process.platform === 'win32',
  })
  if (result.error !== undefined) {
    throw new Error(
      `build:rea: cannot run npm (${result.error.message}). Install Node.js 24.22.3 or newer and try again; see native/rea/NATIVE.md.`,
    )
  }
  if (result.status !== 0) {
    throw new Error(
      `build:rea: ${purpose} exited with ${String(result.status)}. The vendored tree needs Node.js 24.22.3 or newer and network access for a first install; see native/rea/NATIVE.md.`,
    )
  }
}

/**
 * Upstream build command for one Node.js major version.
 *
 * The upstream metadata and skill generators import TypeScript through
 * `module.stripTypeScriptTypes` in transform mode, which Node.js 26 no longer
 * supports. `compile` still runs the mandatory tsc, catalog-record and bundle
 * chain, and this repository's skill package ships the skill text independently,
 * so Node.js 26 builds only that chain and skips the metadata check and
 * `skills/` output.
 * @param nodeMajor - major version of the running Node.js.
 * @returns The npm script to run.
 */
export function buildScript(nodeMajor: number): 'compile' | 'build' {
  return nodeMajor >= 26 ? 'compile' : 'build'
}

/**
 * Install the vendored dependencies when the lockfile changed, then run the
 * upstream build and verify the MCP entrypoint exists.
 * @returns The launcher the MCP plugin spawns.
 * @throws If the install, the build, or the entrypoint check fails.
 */
export function buildRea(): string {
  const launcher = launcherPath()
  if (!existsSync(launcher)) {
    throw new Error(`build:rea: ${launcher} is missing; native/rea does not contain the vendored source.`)
  }

  if (!dependenciesCurrent()) {
    runNpm(['ci', '--no-audit', '--no-fund'], 'npm ci')
    writeFileSync(installStampPath(), `${lockfileDigest()}\n`)
  }
  const nodeMajor = Number.parseInt(process.versions.node.split('.')[0] ?? '', 10)
  const script = buildScript(nodeMajor)
  if (script === 'compile') {
    console.warn('build:rea: Node.js 26 removed the TypeScript transform mode upstream metadata scripts use; running the compile chain only (skills/ is not generated).')
  }
  runNpm(['run', script], `npm run ${script}`)

  if (!existsSync(compiledEntry())) {
    throw new Error(
      `build:rea: npm run ${script} finished without ${compiledEntry()}; the vendored tree is incomplete. See native/rea/NATIVE.md.`,
    )
  }
  console.log(`build:rea: staged ${launcher}`)
  return launcher
}

if (import.meta.main) buildRea()

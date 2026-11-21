/**
 * Mounts the rust-anything MCP server: verified, unit-by-unit ports of
 * TypeScript/JavaScript, Python, C, Go, Ruby, Java and Kotlin code to Rust.
 *
 * The CLI is resolved from this package's own `rust-anything` dependency
 * (`binaries/<platform>-<arch>/rust-anything`), so a checkout that ran
 * `pnpm install` needs no global install. `command` or `RUST_ANYTHING_BIN`
 * overrides the resolved binary.
 *
 * @module @deepseek-ai/dsh-rust-anything
 */

import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'

import type { Context } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import * as McpClient from '@deepseek-ai/dsh-mcp-client'

/** Cordis plugin name used by loader diagnostics. */
export const name = 'rust-anything'

/** The MCP client publishes its tools through the tool registry. */
export const inject = ['tools']

/**
 * Per-`tools/call` budget. `gate`, `build`, `bench` and `app record` compile
 * crates and replay whole corpora, so the MCP default of 60 seconds is short.
 */
const DEFAULT_TOOL_CALL_TIMEOUT_MS = 15 * 60 * 1000

/** Budget for the stdio probe, connection and initial tool sync. */
const DEFAULT_STARTUP_TIMEOUT_MS = 20 * 1000

/** Plugin config: an explicit executable plus the transport budgets. */
export interface Config {
  /** Executable to run instead of the bundled platform binary. */
  command?: string
  /**
   * `eager` puts all 16 tool schemas in every request; `on-demand` lists them
   * in the system prompt and loads the ones an agent asks for.
   */
  toolLoading: 'eager' | 'on-demand'
  /** Per-`tools/call` budget in milliseconds. */
  toolCallTimeoutMs: number
  /** Budget for the stdio probe, connection and initial tool sync. */
  startupTimeoutMs: number
  /** Extra environment for the server process. */
  env: Record<string, string>
}

/** {@link Config} with defaults. */
export const Config: z<Partial<Config>, Config> = z.object({
  command: z.string(),
  toolLoading: z.union(['eager', 'on-demand']).default('eager'),
  toolCallTimeoutMs: z.number().default(DEFAULT_TOOL_CALL_TIMEOUT_MS),
  startupTimeoutMs: z.number().default(DEFAULT_STARTUP_TIMEOUT_MS),
  env: z.dict(String).default({}),
})

/**
 * The platform binary shipped inside the `rust-anything` npm package.
 * @param platform - `process.platform`.
 * @param arch - `process.arch`.
 * @returns Absolute path; the caller does not check that it exists.
 */
export function bundledBinary(platform: string, arch: string): string {
  const executable = platform === 'win32' ? 'rust-anything.exe' : 'rust-anything'
  const packageDirectory = dirname(createRequire(import.meta.url).resolve('rust-anything/package.json'))
  return join(packageDirectory, 'binaries', `${platform}-${arch}`, executable)
}

/**
 * The command to start: the configured executable, `RUST_ANYTHING_BIN`, else
 * the binary this package installed.
 * @param config - resolved plugin config.
 * @param env - environment to read `RUST_ANYTHING_BIN` from.
 * @returns Executable path or command name.
 */
export function resolveCommand(config: Config, env: NodeJS.ProcessEnv = process.env): string {
  return config.command ?? env.RUST_ANYTHING_BIN ?? bundledBinary(process.platform, process.arch)
}

/**
 * Connect the rust-anything MCP server and publish its tools.
 * @param ctx - plugin context carrying the tool registry.
 * @param config - resolved plugin config.
 */
export async function apply(ctx: Context, config: Config): Promise<void> {
  await ctx.plugin(McpClient, {
    serverName: 'rust-anything',
    transport: 'stdio',
    command: resolveCommand(config),
    args: ['mcp'],
    toolLoading: config.toolLoading,
    toolCallTimeoutMs: config.toolCallTimeoutMs,
    startupTimeoutMs: config.startupTimeoutMs,
    env: config.env,
  })
}

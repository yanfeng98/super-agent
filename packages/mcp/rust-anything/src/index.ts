/**
 * Mounts the rust-anything MCP server: verified, unit-by-unit ports of
 * TypeScript/JavaScript, Python, C, Go, Ruby, Java and Kotlin code to Rust.
 *
 * The CLI is compiled from the crate vendored at `native/rust-anything` by
 * `pnpm run build`, which stages `bin/rust-anything` for the checkout. A
 * checkout that ran that build needs no global install and no published
 * binary; `command` or `RUST_ANYTHING_BIN` overrides the staged executable.
 *
 * @module @deepseek-ai/dsh-rust-anything
 */

import type { Context } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import * as McpClient from '@deepseek-ai/dsh-mcp-client'

import { resolveCommand } from './binary.ts'

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
  /** Executable to run instead of the staged vendored build. */
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
 * Connect the rust-anything MCP server and publish its tools.
 * @param ctx - plugin context carrying the tool registry.
 * @param config - resolved plugin config.
 */
export async function apply(ctx: Context, config: Config): Promise<void> {
  await ctx.plugin(McpClient, {
    serverName: 'rust-anything',
    transport: 'stdio',
    command: resolveCommand(config.command),
    args: ['mcp'],
    toolLoading: config.toolLoading,
    toolCallTimeoutMs: config.toolCallTimeoutMs,
    startupTimeoutMs: config.startupTimeoutMs,
    env: config.env,
  })
}

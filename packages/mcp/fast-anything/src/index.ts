/**
 * Mounts the fast-anything MCP server: port the hottest pure functions of an
 * app to Rust, proving each port against the calls the original really made.
 *
 * The CLI is compiled from the crate vendored at `native/fast-anything` by
 * `pnpm run build`, which stages `bin/fast-anything` for the checkout. A
 * checkout that ran that build needs no global install and no published
 * binary; `command` or `FAST_ANYTHING_BIN` overrides the staged executable.
 *
 * @module @deepseek-ai/dsh-fast-anything
 */

import type { Context } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import { mountStdioServer, STDIO_STARTUP_TIMEOUT_MS, STDIO_TOOL_CALL_TIMEOUT_MS } from '@deepseek-ai/dsh-mcp-client/stdio-plugin'

import { resolveCommand } from './binary.ts'

/** Cordis plugin name used by loader diagnostics. */
export const name = 'fast-anything'

/** The MCP client publishes its tools through the tool registry. */
export const inject = ['tools']

// The config catalog pastes each plugin config declaration verbatim, so it stays
// local; the transport, budgets and mounting are shared through stdio-plugin.ts.
/* jscpd:ignore-start */
/** Plugin config: an explicit executable plus the transport budgets. */
export interface Config {
  /** Executable to run instead of the staged vendored build. */
  command?: string
  /**
   * `eager` puts all 8 tool schemas in every request; `on-demand` lists them
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
  toolCallTimeoutMs: z.number().default(STDIO_TOOL_CALL_TIMEOUT_MS),
  startupTimeoutMs: z.number().default(STDIO_STARTUP_TIMEOUT_MS),
  env: z.dict(String).default({}),
})
/* jscpd:ignore-end */

/**
 * Connect the fast-anything MCP server and publish its tools.
 * @param ctx - plugin context carrying the tool registry.
 * @param config - resolved plugin config.
 */
/**
 * Connect the fast-anything MCP server and publish its tools.
 * @param ctx - plugin context carrying the tool registry.
 * @param config - resolved plugin config.
 */
export async function apply(ctx: Context, config: Config): Promise<void> {
  await mountStdioServer(ctx, 'fast-anything', resolveCommand(config.command), ['mcp'], config)
}

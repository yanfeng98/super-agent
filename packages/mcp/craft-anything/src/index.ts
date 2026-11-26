/**
 * Mounts the craft-anything MCP server: evidence-backed investigation of
 * shipped programs, bundles, firmware and captures through toolsets that an
 * agent enables as needed.
 *
 * The CLI is compiled from the Rust workspace vendored at
 * `native/craft-anything` by `pnpm run build`, which stages
 * `bin/craft-anything-cli[.exe]` for the checkout. A checkout that ran that
 * build needs no global install; `command` or `CRAFT_ANYTHING_BIN` overrides
 * the staged executable.
 *
 * @module @deepseek-ai/dsh-craft-anything
 */

import type { Context } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import { mountStdioServer, STDIO_STARTUP_TIMEOUT_MS, STDIO_TOOL_CALL_TIMEOUT_MS } from '@deepseek-ai/dsh-mcp-client/stdio-plugin'

import { resolveCommand } from './binary.ts'

/** Cordis plugin name used by loader diagnostics. */
export const name = 'craft-anything'

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
   * `on-demand` keeps the initial Core toolset in the request and loads an
   * enabled toolset only when the agent asks for it; `eager` publishes every
   * tool from the start.
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
  toolLoading: z.union(['eager', 'on-demand']).default('on-demand'),
  toolCallTimeoutMs: z.number().default(STDIO_TOOL_CALL_TIMEOUT_MS),
  startupTimeoutMs: z.number().default(STDIO_STARTUP_TIMEOUT_MS),
  env: z.dict(String).default({}),
})
/* jscpd:ignore-end */

/**
 * Connect the craft-anything MCP server and publish its tools.
 * @param ctx - plugin context carrying the tool registry.
 * @param config - resolved plugin config.
 */
export async function apply(ctx: Context, config: Config): Promise<void> {
  await mountStdioServer(ctx, 'craft-anything', resolveCommand(config.command), ['mcp'], config)
}

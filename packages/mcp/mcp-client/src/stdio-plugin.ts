/**
 * Mounting shared by plugins that connect one stdio MCP server: the transport,
 * per-call budget and extra environment a vendored or host-provided server
 * needs. The plugin keeps its own executable resolution and config schema.
 *
 * @module @deepseek-ai/dsh-mcp-client/src/stdio-plugin
 */

import type { Context } from '@deepseek-ai/cordis'
import * as McpClient from './index.ts'

/** Transport and budget fields every stdio plugin config carries. */
export interface StdioServerSettings {
  /** `eager` publishes every tool schema; `on-demand` lists them for loading. */
  readonly toolLoading: 'eager' | 'on-demand'
  /** Per-`tools/call` budget in milliseconds. */
  readonly toolCallTimeoutMs: number
  /** Budget for the stdio probe, connection and initial tool sync. */
  readonly startupTimeoutMs: number
  /** Extra environment for the server process. */
  readonly env: Record<string, string>
}

/** Default per-call budget for servers that compile crates or replay corpora. */
export const STDIO_TOOL_CALL_TIMEOUT_MS = 15 * 60 * 1000

/** Default budget for the stdio probe, connection and initial tool sync. */
export const STDIO_STARTUP_TIMEOUT_MS = 20 * 1000

/**
 * Connect one stdio MCP server and publish its tools through the tool registry.
 * @param ctx - plugin context carrying the tool registry and the MCP client.
 * @param serverName - MCP server name used in tool names and the instruction section.
 * @param command - resolved executable or command name.
 * @param args - server arguments.
 * @param settings - transport budgets and extra environment.
 * @param env - environment for the server process; defaults to `settings.env`.
 */
export async function mountStdioServer(
  ctx: Context,
  serverName: string,
  command: string,
  args: string[],
  settings: StdioServerSettings,
  env: Record<string, string> = settings.env,
): Promise<void> {
  await ctx.plugin(McpClient, {
    serverName,
    transport: 'stdio',
    command,
    args,
    toolLoading: settings.toolLoading,
    toolCallTimeoutMs: settings.toolCallTimeoutMs,
    startupTimeoutMs: settings.startupTimeoutMs,
    env,
  })
}

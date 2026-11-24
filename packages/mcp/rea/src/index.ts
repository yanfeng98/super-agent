/**
 * Mounts the rea MCP server: evidence-based reverse engineering of native
 * binaries, JavaScript/Electron apps, .NET assemblies, websites, mobile apps,
 * firmware and recorded runtime behavior.
 *
 * `pnpm run build` installs and builds the upstream source vendored at
 * `native/rea`, so a checkout needs no published tarball and no global
 * install. The server starts on the same Node.js runtime as the Host; `command`
 * overrides that runtime and `launcher` overrides the entry script.
 *
 * The default catalog is upstream's `hot-cold` profile: the server advertises
 * five tools, and `discover_tools` loads the rest per connection, which keeps
 * the 263-tool surface out of every request. With `toolLoading: on-demand` the
 * five appear in the system-prompt catalog and an agent loads the ones it
 * needs.
 *
 * @module @deepseek-ai/dsh-rea
 */

import type { Context } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import { mountStdioServer, STDIO_STARTUP_TIMEOUT_MS, STDIO_TOOL_CALL_TIMEOUT_MS } from '@deepseek-ai/dsh-mcp-client/stdio-plugin'

import { resolveLauncher } from './launcher.ts'

/** Cordis plugin name used by loader diagnostics. */
export const name = 'rea'

/** The MCP client publishes its tools through the tool registry. */
export const inject = ['tools']


// The config catalog pastes each plugin config declaration verbatim, so it stays
// local; the transport, budgets and mounting are shared through stdio-plugin.ts.
/* jscpd:ignore-start */
/** Plugin config: the runtime, the advertised catalog, and the transport budgets. */
export interface Config {
  /** Node.js executable to run the launcher; defaults to the Host's own runtime. */
  command?: string
  /** Entry script to run instead of the vendored `native/rea/scripts/rea.mjs`. */
  launcher?: string
  /**
   * `full` advertises all 263 tools on connect (about 353 KB of schemas);
   * `hot-cold` advertises five and loads the rest through `discover_tools`.
   */
  catalogProfile: 'full' | 'hot-cold'
  /**
   * `on-demand` lists the advertised tools in the system prompt and loads the
   * ones an agent asks for; `eager` puts every schema in every request.
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
  launcher: z.string(),
  catalogProfile: z.union(['full', 'hot-cold']).default('hot-cold'),
  toolLoading: z.union(['eager', 'on-demand']).default('on-demand'),
  toolCallTimeoutMs: z.number().default(STDIO_TOOL_CALL_TIMEOUT_MS),
  startupTimeoutMs: z.number().default(STDIO_STARTUP_TIMEOUT_MS),
  env: z.dict(String).default({}),
})
/* jscpd:ignore-end */

/**
 * Connect the rea MCP server and publish its tools.
 * @param ctx - plugin context carrying the tool registry.
 * @param config - resolved plugin config.
 */
export async function apply(ctx: Context, config: Config): Promise<void> {
  await mountStdioServer(ctx, 'rea', config.command ?? process.execPath, [resolveLauncher(config.launcher), 'mcp'], config, {
    // The vendored launcher refuses runtimes below its engine floor; keep
    // packaged Electron hosts from passing their own binary through.
    ELECTRON_RUN_AS_NODE: '1',
    REA_MCP_TOOL_CATALOG_PROFILE: config.catalogProfile,
    ...config.env,
  })
}

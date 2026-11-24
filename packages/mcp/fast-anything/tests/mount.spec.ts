import { describe, expect, it } from 'vitest'

import { Context } from '@deepseek-ai/cordis'
import SessionProjectionRegistry from '@deepseek-ai/dsh-session-projection'
import SystemPrompt from '@deepseek-ai/dsh-system-prompt'
import ToolRuntime from '@deepseek-ai/dsh-tools'

import { resolveCommand, vendoredBinary } from '../src/binary.ts'
import { apply, Config } from '../src/index.ts'

describe('fast-anything MCP server mount', () => {
  it('resolves the executable the repository build stages from the vendored crate', () => {
    expect(vendoredBinary('linux')).toMatch(/[/\\]native[/\\]fast-anything[/\\]bin[/\\]fast-anything$/)
    expect(vendoredBinary('darwin')).toMatch(/[/\\]native[/\\]fast-anything[/\\]bin[/\\]fast-anything$/)
    expect(vendoredBinary('win32')).toMatch(/[/\\]native[/\\]fast-anything[/\\]bin[/\\]fast-anything\.exe$/)
  })

  it('prefers a configured command, then FAST_ANYTHING_BIN, then the vendored build', () => {
    const config = Config({ command: '/opt/fast-anything' })
    expect(resolveCommand(config.command, { FAST_ANYTHING_BIN: '/env/fast-anything' })).toBe('/opt/fast-anything')
    expect(resolveCommand(Config({}).command, { FAST_ANYTHING_BIN: '/env/fast-anything' })).toBe('/env/fast-anything')
    expect(resolveCommand(Config({}).command, {})).toBe(vendoredBinary(process.platform))
  })

  it('registers every tool eagerly through the shared MCP client', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ toolLoading: 'eager' }))
    const names = ctx.tools.schemas().map(schema => schema.name)
    // The vendored crate's surface at `v0.8.6`; a new upstream tool fails here so
    // the package README's tool count and token estimate move with it.
    const tools = [
      'fast_scan', 'fast_port', 'fast_verify', 'fast_enable',
      'fast_disable', 'fast_bench', 'fast_report', 'fast_status',
    ]
    expect([...names].sort()).toEqual(tools.map(tool => `mcp__fast-anything__${tool}`).sort())
    await ctx.fiber.dispose()
  })

  it('leaves agent creation working when the executable is missing', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ command: '/nonexistent/fast-anything' }))
    expect(ctx.tools.schemas()).toEqual([])
    await ctx.fiber.dispose()
  })

  it('publishes only the loader when tools load on demand', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ toolLoading: 'on-demand' }))
    expect(ctx.tools.schemas().map(schema => schema.name)).toEqual(['mcp__fast-anything__load_tools'])
    await ctx.fiber.dispose()
  })
})

import { describe, expect, it } from 'vitest'

import { Context } from '@deepseek-ai/cordis'
import SessionProjectionRegistry from '@deepseek-ai/dsh-session-projection'
import SystemPrompt from '@deepseek-ai/dsh-system-prompt'
import ToolRuntime from '@deepseek-ai/dsh-tools'

import { resolveCommand, vendoredBinary } from '../src/binary.ts'
import { apply, Config } from '../src/index.ts'

describe('rust-anything MCP server mount', () => {
  it('resolves the executable the repository build stages from the vendored crate', () => {
    expect(vendoredBinary('linux')).toMatch(/[/\\]native[/\\]rust-anything[/\\]bin[/\\]rust-anything$/)
    expect(vendoredBinary('darwin')).toMatch(/[/\\]native[/\\]rust-anything[/\\]bin[/\\]rust-anything$/)
    expect(vendoredBinary('win32')).toMatch(/[/\\]native[/\\]rust-anything[/\\]bin[/\\]rust-anything\.exe$/)
  })

  it('prefers a configured command, then RUST_ANYTHING_BIN, then the vendored build', () => {
    const config = Config({ command: '/opt/rust-anything' })
    expect(resolveCommand(config.command, { RUST_ANYTHING_BIN: '/env/rust-anything' })).toBe('/opt/rust-anything')
    expect(resolveCommand(Config({}).command, { RUST_ANYTHING_BIN: '/env/rust-anything' })).toBe('/env/rust-anything')
    expect(resolveCommand(Config({}).command, {})).toBe(vendoredBinary(process.platform))
  })

  it('registers every tool eagerly through the shared MCP client', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ toolLoading: 'eager' }))
    const names = ctx.tools.schemas().map(schema => schema.name)
    // The vendored crate's surface at `v3.3.0`; a new upstream tool fails here so
    // the package README's schema count and token estimate move with it.
    const tools = [
      'analyze', 'bridge', 'refresh', 'record', 'build', 'diff', 'profile', 'go', 'enable', 'disable',
      'report', 'bench', 'fuzz', 'shadow', 'map', 'package', 'gate',
      'app_record', 'app_replay', 'app_front_door', 'app_wire', 'app_lift',
    ]
    expect([...names].sort()).toEqual(tools.map(tool => `mcp__rust-anything__${tool}`).sort())
    await ctx.fiber.dispose()
  })

  it('leaves agent creation working when the executable is missing', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ command: '/nonexistent/rust-anything' }))
    expect(ctx.tools.schemas()).toEqual([])
    await ctx.fiber.dispose()
  })

  it('publishes only the loader when tools load on demand', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ toolLoading: 'on-demand' }))
    expect(ctx.tools.schemas().map(schema => schema.name)).toEqual(['mcp__rust-anything__load_tools'])
    await ctx.fiber.dispose()
  })
})

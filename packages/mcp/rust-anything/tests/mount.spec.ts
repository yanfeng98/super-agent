import { describe, expect, it } from 'vitest'

import { Context } from '@deepseek-ai/cordis'
import SystemPrompt from '@deepseek-ai/dsh-system-prompt'
import ToolRuntime from '@deepseek-ai/dsh-tools'

import { apply, bundledBinary, Config, resolveCommand } from '../src/index.ts'

describe('rust-anything MCP server mount', () => {
  it('resolves the bundled platform binary from the package dependency', () => {
    expect(bundledBinary('linux', 'x64')).toMatch(/[/\\]rust-anything[/\\]binaries[/\\]linux-x64[/\\]rust-anything$/)
    expect(bundledBinary('darwin', 'arm64')).toMatch(/[/\\]binaries[/\\]darwin-arm64[/\\]rust-anything$/)
    expect(bundledBinary('win32', 'x64')).toMatch(/[/\\]binaries[/\\]win32-x64[/\\]rust-anything\.exe$/)
  })

  it('prefers a configured command, then RUST_ANYTHING_BIN, then the dependency binary', () => {
    const config = Config({ command: '/opt/rust-anything' })
    expect(resolveCommand(config, { RUST_ANYTHING_BIN: '/env/rust-anything' })).toBe('/opt/rust-anything')
    expect(resolveCommand(Config({}), { RUST_ANYTHING_BIN: '/env/rust-anything' })).toBe('/env/rust-anything')
    expect(resolveCommand(Config({}), {})).toBe(bundledBinary(process.platform, process.arch))
  })

  it('registers every tool eagerly through the shared MCP client', async () => {
    const ctx = new Context()
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ toolLoading: 'eager' }))
    const names = ctx.tools.schemas().map(schema => schema.name)
    expect(names).toContain('mcp__rust-anything__analyze')
    expect(names).toContain('mcp__rust-anything__gate')
    expect(names).toContain('mcp__rust-anything__app_lift')
    await ctx.fiber.dispose()
  })

  it('publishes only the loader when tools load on demand', async () => {
    const ctx = new Context()
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ toolLoading: 'on-demand' }))
    expect(ctx.tools.schemas().map(schema => schema.name)).toEqual(['mcp__rust-anything__load_tools'])
    await ctx.fiber.dispose()
  })
})

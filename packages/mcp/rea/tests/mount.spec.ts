import { describe, expect, it } from 'vitest'

import { Context } from '@deepseek-ai/cordis'
import SessionProjectionRegistry from '@deepseek-ai/dsh-session-projection'
import SystemPrompt from '@deepseek-ai/dsh-system-prompt'
import ToolRuntime from '@deepseek-ai/dsh-tools'

import { apply, Config } from '../src/index.ts'
import { resolveLauncher, vendoredLauncher } from '../src/launcher.ts'

describe('rea MCP server mount', () => {
  it('resolves the launcher the repository builds from the vendored source', () => {
    expect(vendoredLauncher()).toMatch(/[/\\]native[/\\]rea[/\\]scripts[/\\]rea\.mjs$/)
  })

  it('prefers a configured launcher over the vendored build', () => {
    expect(resolveLauncher('/opt/rea/scripts/rea.mjs')).toBe('/opt/rea/scripts/rea.mjs')
    expect(resolveLauncher(undefined)).toBe(vendoredLauncher())
  })

  it('registers the hot-cold catalog eagerly when asked', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ toolLoading: 'eager' }))
    const names = ctx.tools.schemas().map(schema => schema.name)
    // The upstream hot-cold advertisement at `6.10.1`; a new initial tool fails
    // here so the package README's tool count moves with the vendored release.
    expect([...names].sort()).toEqual([
      'mcp__rea__binary_session',
      'mcp__rea__close_binary',
      'mcp__rea__discover_tools',
      'mcp__rea__get_evidence_bundle',
      'mcp__rea__open_binary',
    ])
    await ctx.fiber.dispose()
  })

  it('publishes only the loader when tools load on demand', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({}))
    expect(ctx.tools.schemas().map(schema => schema.name)).toEqual(['mcp__rea__load_tools'])
    await ctx.fiber.dispose()
  })

  it('leaves agent creation working when the runtime is missing', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ command: '/nonexistent/node' }))
    expect(ctx.tools.schemas()).toEqual([])
    await ctx.fiber.dispose()
  })
})

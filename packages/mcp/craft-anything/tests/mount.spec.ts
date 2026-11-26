import { describe, expect, it } from 'vitest'

import { Context } from '@deepseek-ai/cordis'
import SessionProjectionRegistry from '@deepseek-ai/dsh-session-projection'
import SystemPrompt from '@deepseek-ai/dsh-system-prompt'
import ToolRuntime from '@deepseek-ai/dsh-tools'

import { resolveCommand, vendoredBinary } from '../src/binary.ts'
import { apply, Config } from '../src/index.ts'

/** The Core toolset plus two toolset selectors the vendored v0.6.1 server advertises first. */
const INITIAL_TOOLS = [
  'core_target_close', 'core_session_show', 'core_targets_list', 'core_target_select',
  'core_evidence_show', 'core_evidence_export', 'core_evidence_import',
  'core_unknowns_show', 'core_unknown_add', 'core_unknown_edit', 'core_unknown_verify',
  'core_person_ask', 'core_action_approve', 'assets_image_generate', 'assets_model_generate',
  'core_investigation_show', 'core_brief_write', 'core_plan_update',
  'core_hypothesis_record', 'core_hypothesis_update', 'core_finding_record',
  'core_finding_edit', 'core_handoff_write', 'core_journal_read',
  'core_report_save', 'core_report_write', 'core_evidence_browse', 'rebuild_check',
  'list_toolsets', 'enable_toolset',
]

describe('craft-anything MCP server mount', () => {
  it('resolves the executable the repository build stages from the vendored workspace', () => {
    expect(vendoredBinary('linux')).toMatch(/[/\\]native[/\\]craft-anything[/\\]bin[/\\]craft-anything-cli$/)
    expect(vendoredBinary('darwin')).toMatch(/[/\\]native[/\\]craft-anything[/\\]bin[/\\]craft-anything-cli$/)
    expect(vendoredBinary('win32')).toMatch(/[/\\]native[/\\]craft-anything[/\\]bin[/\\]craft-anything-cli\.exe$/)
  })

  it('prefers a configured command, then CRAFT_ANYTHING_BIN, then the vendored build', () => {
    const config = Config({ command: '/opt/craft-anything-cli' })
    expect(resolveCommand(config.command, { CRAFT_ANYTHING_BIN: '/env/craft-anything-cli' })).toBe('/opt/craft-anything-cli')
    expect(resolveCommand(Config({}).command, { CRAFT_ANYTHING_BIN: '/env/craft-anything-cli' })).toBe('/env/craft-anything-cli')
    expect(resolveCommand(Config({}).command, {})).toBe(vendoredBinary(process.platform))
  })

  it('registers the initial Core toolset eagerly through the shared MCP client', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ toolLoading: 'eager' }))
    const names = ctx.tools.schemas().map(schema => schema.name)
    // The vendored workspace's surface at `v0.6.1`; a new upstream initial tool
    // fails here so the package README's tool count and token estimate move with it.
    expect([...names].sort()).toEqual(INITIAL_TOOLS.map(tool => `mcp__craft-anything__${tool}`).sort())
    await ctx.fiber.dispose()
  })

  it('leaves agent creation working when the executable is missing', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ command: '/nonexistent/craft-anything-cli' }))
    expect(ctx.tools.schemas()).toEqual([])
    await ctx.fiber.dispose()
  })

  it('publishes only the loader when tools load on demand', async () => {
    const ctx = new Context()
    await ctx.plugin(SessionProjectionRegistry)
    await ctx.plugin(SystemPrompt)
    await ctx.plugin(ToolRuntime)
    await apply(ctx, Config({ toolLoading: 'on-demand' }))
    expect(ctx.tools.schemas().map(schema => schema.name)).toEqual(['mcp__craft-anything__load_tools'])
    await ctx.fiber.dispose()
  })
})

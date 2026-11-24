/** Built-in porting and reverse-engineering skills ship with every Web profile, not only the standard preset. */

import { fileURLToPath } from 'node:url'
import { Context } from '@deepseek-ai/cordis'
import { loadOverlayPatches } from '@deepseek-ai/dsh-app-boot'
import { createScope } from '@deepseek-ai/dsh-scope'
import SkillRegistry from '@deepseek-ai/dsh-skill'
import * as SkillFastAnything from '@deepseek-ai/dsh-skill-fast-anything'
import * as SkillRea from '@deepseek-ai/dsh-skill-rea'
import * as SkillRustAnything from '@deepseek-ai/dsh-skill-rust-anything'
import { expect, it, onTestFinished } from 'vitest'

/** Plugin rows of the Web bundle host patch. */
function hostRows(): { id: string; name: string }[] {
  return loadOverlayPatches('bundled-skills', fileURLToPath(new URL('../cordis.patch.yml', import.meta.url)))
    .flatMap(patch => patch.insert ?? [])
}

/** Plugin rows of one shipped agent preset. */
function presetRows(preset: string): { id: string; name: string }[] {
  const [row] = loadOverlayPatches(preset, fileURLToPath(new URL(`../presets/${preset}.patch.yml`, import.meta.url)))
    .flatMap(patch => patch.insert ?? [])
  return (row!.config as { plugins: { id: string; name: string }[] }).plugins
}

const BUNDLED_SKILLS = [
  { id: 'skill-rust-anything', name: '@deepseek-ai/dsh-skill-rust-anything' },
  { id: 'skill-rea', name: '@deepseek-ai/dsh-skill-rea' },
  { id: 'skill-fast-anything', name: '@deepseek-ai/dsh-skill-fast-anything' },
]

it('mounts the built-in porting and reverse-engineering skills in the host patch', () => {
  const host = hostRows().filter(row => BUNDLED_SKILLS.some(skill => skill.id === row.id))
  expect(host).toEqual(BUNDLED_SKILLS)
})

it('keeps the heavy MCP tool catalogs in the standard preset', () => {
  const standard = presetRows('standard').filter(row => ['mcp-rust-anything', 'mcp-rea', 'mcp-fast-anything'].includes(row.id))
  expect(standard.map(row => row.id)).toEqual(['mcp-rust-anything', 'mcp-rea', 'mcp-fast-anything'])
  for (const preset of ['superagent-worker', 'superagent-router', 'ptc', 'cordis', 'minimal']) {
    expect(presetRows(preset).some(row => BUNDLED_SKILLS.some(skill => skill.id === row.id)),
      `${preset} must not duplicate the host skill rows`).toBe(false)
  }
})

it('lists the host-mounted skills from an agent scope without preset rows', async () => {
  const ctx = new Context()
  onTestFinished(() => ctx.fiber.dispose())
  await ctx.plugin(SkillRegistry)
  for (const plugin of [SkillRustAnything, SkillRea, SkillFastAnything]) {
    await ctx.plugin(plugin, {})
  }
  // An orchestrator/agent scope can be nested without any preset mounting the skill rows.
  const agentKey = {}
  createScope(ctx, agentKey)
  const names = (await ctx.skills.list({ scope: agentKey })).map(skill => skill.name)
  expect(names).toEqual(expect.arrayContaining(['dsh-rust-anything', 'dsh-rea', 'dsh-fast-anything']))
})

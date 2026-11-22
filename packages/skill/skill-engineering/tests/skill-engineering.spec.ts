import { cp, mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, sep } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { Context } from '@deepseek-ai/cordis'
import Loader from '@deepseek-ai/cordis-plugin-loader'
import Include from '@deepseek-ai/cordis-plugin-include'
import SkillRegistry, { type SkillCandidate, type SkillProvider, type SkillProviderObservation } from '@deepseek-ai/dsh-skill'
import * as SkillEngineering from '@deepseek-ai/dsh-skill-engineering'
import { describe, expect, it, vi } from 'vitest'

vi.mock('node:fs/promises', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:fs/promises')>()
  return { ...actual, readFile: vi.fn(actual.readFile) }
})

const assets = fileURLToPath(new URL('../assets/', import.meta.url))
const names = [
  'eng-ask-matt',
  'eng-code-review',
  'eng-codebase-design',
  'eng-diagnosing-bugs',
  'eng-domain-modeling',
  'eng-grill-me',
  'eng-grill-with-docs',
  'eng-grilling',
  'eng-handoff',
  'eng-implement',
  'eng-implement-spec',
  'eng-improve-codebase-architecture',
  'eng-loop-me',
  'eng-pr',
  'eng-prototype',
  'eng-research',
  'eng-retro',
  'eng-setup-matt-pocock-skills',
  'eng-setup-ts-deep-modules',
  'eng-tdd',
  'eng-teach',
  'eng-to-questionnaire',
  'eng-to-spec',
  'eng-to-tickets',
  'eng-triage',
  'eng-wait-what',
  'eng-wayfinder',
  'eng-wizard',
  'eng-writing-beats',
  'eng-writing-for-agents',
  'eng-writing-fragments',
  'eng-writing-shape',
] as const

/** Instruction body with the frontmatter block removed, as the provider reports it. */
async function bodyOf(path: string): Promise<string> {
  const raw = await readFile(path, 'utf8')
  return raw.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n/u, '').trim()
}

/** Copy the packaged assets into a temporary root so one skill can be edited. */
async function relocatedAssets(prefix: string, name: string): Promise<{ root: string; skill: string }> {
  const root = await mkdtemp(join(tmpdir(), prefix))
  await cp(assets, root, { recursive: true })
  return { root, skill: join(root, name, 'SKILL.md') }
}

/** Distinguish an explicit observation from the complete-discovery array shorthand. */
function isObservation(value: readonly SkillCandidate[] | SkillProviderObservation): value is SkillProviderObservation {
  return !Array.isArray(value)
}

/** Discovery result with the array shorthand expanded, so a test never reads the union directly. */
async function observe(provider: SkillProvider): Promise<SkillProviderObservation> {
  const result = await provider.list({})
  return isObservation(result) ? result : { candidates: result, complete: true }
}

function captureWarnings(ctx: Context): string[] {
  const warnings: string[] = []
  ctx.logger.exporter({
    levels: { default: 2 },
    export: ({ type, args }) => {
      if (type === 'warn') warnings.push(String(args[0]))
    },
  })
  return warnings
}

describe('bundled engineering workflow skills', () => {
  it('lists every packaged skill with its invocation policy', async () => {
    const ctx = new Context()
    try {
      await ctx.plugin(SkillRegistry)
      await ctx.plugin(SkillEngineering)
      const catalog = await ctx.skills.list()
      expect(catalog.map(skill => skill.name)).toEqual([...names])
      for (const skill of catalog) {
        expect(skill).toMatchObject({
          source: 'bundled',
          provider: 'dsh-engineering',
          upstream: 'mattpocock/skills',
        })
        expect(skill.description.length).toBeGreaterThan(0)
        expect(skill.description.length).toBeLessThanOrEqual(500)
        expect(skill.resourceBase).toEqual({ kind: 'directory', path: join(assets, skill.name).split(sep).join('/') })
      }
      const policy = (name: string) => catalog.find(skill => skill.name === name)?.invocation
      expect(policy('eng-ask-matt')).toEqual({ modelInvocable: false, userInvocable: true })
      expect(policy('eng-implement')).toEqual({ modelInvocable: false, userInvocable: true })
      expect(policy('eng-grill-me')).toEqual({ modelInvocable: false, userInvocable: true })
      expect(policy('eng-tdd')).toEqual({ modelInvocable: true, userInvocable: true })
      expect(policy('eng-research')).toEqual({ modelInvocable: true, userInvocable: true })
    } finally { await ctx.fiber.dispose() }
  })

  it('keeps the packaged asset directories and the provider list in step', async () => {
    const directories = (await readdir(assets, { withFileTypes: true })).filter(entry => entry.isDirectory()).map(entry => entry.name)
    expect(directories.sort()).toEqual([...names])
    for (const name of names) {
      const raw = await readFile(join(assets, name, 'SKILL.md'), 'utf8')
      expect(raw.startsWith('---\n')).toBe(true)
      expect(raw).toContain(`name: ${name}\n`)
    }
  })

  it('loads each packaged body and removes every candidate on disposal', async () => {
    const ctx = new Context()
    try {
      await ctx.plugin(SkillRegistry)
      const fiber = await ctx.plugin(SkillEngineering)
      for (const name of names) {
        const loaded = await ctx.skills.get(name)
        expect(loaded?.content).toBe(await bodyOf(join(assets, name, 'SKILL.md')))
        expect(loaded?.resourceBase).toEqual({ kind: 'directory', path: join(assets, name).split(sep).join('/') })
      }
      expect((await ctx.skills.get('eng-tdd'))?.content).toContain('[tests.md](tests.md)')
      await fiber.dispose()
      expect(await ctx.skills.list()).toEqual([])
    } finally { await ctx.fiber.dispose() }
  })

  it('loads relocated resources through a real cordis.yml composition', async () => {
    const root = await mkdtemp(join(tmpdir(), 'dsh-engineering-skills-'))
    const ctx = new Context()
    try {
      const external = join(root, '中文 assets')
      await cp(assets, external, { recursive: true })
      const configPath = join(root, 'cordis.yml')
      await writeFile(configPath, [
        "- name: '@deepseek-ai/dsh-skill'",
        "- name: '@deepseek-ai/dsh-skill-engineering'",
        '  config:',
        `    assetRoot: ${JSON.stringify(external)}`,
        '',
      ].join('\n'))
      ctx.baseUrl = pathToFileURL(root).href + '/'
      await ctx.plugin(Loader)
      ctx.loader.builtins.include = Include
      const modules = new Map<string, unknown>([
        ['@deepseek-ai/dsh-skill', SkillRegistry],
        ['@deepseek-ai/dsh-skill-engineering', SkillEngineering],
      ])
      ctx.loader.internal = {
        version: 'v2',
        loadCache: new Map(),
        register() { throw new Error('unexpected register') },
        getOrCreateModuleJob() { throw new Error('unexpected module job') },
        resolveSync() { throw new Error('unexpected resolution') },
        load() { throw new Error('unexpected load') },
        async import(specifier: string) {
          if (!modules.has(specifier)) throw new Error(`Unexpected Loader import: ${specifier}`)
          return modules.get(specifier)
        },
      }
      await ctx.loader.create({ name: 'cordis:include', config: { path: pathToFileURL(configPath).href } })
      await ctx.loader.await()
      expect((await ctx.skills.list()).map(skill => skill.name)).toEqual([...names])
      expect((await ctx.skills.get('eng-tdd'))?.resourceBase)
        .toEqual({ kind: 'directory', path: join(external, 'eng-tdd').split(sep).join('/') })
      expect((await ctx.skills.get('eng-tdd'))?.content).toBe(await bodyOf(join(external, 'eng-tdd', 'SKILL.md')))
    } finally {
      await ctx.fiber.dispose()
      await rm(root, { recursive: true, force: true })
    }
  })

  it('rejects a relative asset root before registration', async () => {
    const ctx = new Context()
    try {
      await ctx.plugin(SkillRegistry)
      expect(() => { SkillEngineering.apply(ctx, { assetRoot: 'assets' }) }).toThrow('absolute directory')
      expect(await ctx.skills.list()).toEqual([])
    } finally { await ctx.fiber.dispose() }
  })

  it('omits unreadable or invalid skills from a complete catalog and reports each failure once', async () => {
    const { root, skill } = await relocatedAssets('dsh-engineering-invalid-', 'eng-tdd')
    const ctx = new Context()
    try {
      const warnings = captureWarnings(ctx)
      await ctx.plugin(SkillRegistry)
      const registerProvider = vi.spyOn(ctx.skills, 'registerProvider')
      const fiber = await ctx.plugin(SkillEngineering, { assetRoot: root })
      const provider: SkillProvider = registerProvider.mock.calls[0]![0]({ signal: new AbortController().signal, invalidate: () => {} })
      const original = await readFile(skill, 'utf8')
      const remaining = names.filter(name => name !== 'eng-tdd')
      const listed = async () => (await observe(provider)).candidates.map(entry => entry.name)
      const missing = async () => {
        const observed = await ctx.skills.snapshot()
        return { complete: observed.complete, names: observed.skills.map(entry => entry.name) }
      }

      await ctx.skills.list()
      await rm(skill)
      expect(await ctx.skills.get('eng-tdd')).toBeUndefined()
      expect(await missing()).toEqual({ complete: true, names: remaining })
      expect(warnings).toEqual([expect.stringMatching(/^skill-engineering: skill "eng-tdd" is unavailable: Error: ENOENT.*SKILL\.md/u)])
      await observe(provider)
      expect(warnings).toHaveLength(1)

      await writeFile(skill, original)
      expect(await listed()).toEqual([...names])

      const invalid: [content: string, failure: string][] = [
        ['# Missing frontmatter\nname: eng-tdd\n', 'has no YAML frontmatter'],
        ['---\nnull\n---\n# Instructions\n', 'has no metadata'],
        ['---\nname: other\ndescription: 描述\n---\n# Instructions\n', 'declares name "other", expected "eng-tdd"'],
        ['---\nname: eng-tdd\n---\n# Instructions\n', 'has no description'],
        ['---\nname: eng-tdd\ndescription: 描述\ndisable-model-invocation: yes\n---\n# Instructions\n', 'must be a boolean'],
        ['---\nname: eng-tdd\ndescription: [unclosed\n---\n# Instructions\n', 'YAMLParseError'],
      ]
      for (const [content, failure] of invalid) {
        await writeFile(skill, content)
        const before = warnings.length
        expect(await listed()).toEqual(remaining)
        expect(warnings).toHaveLength(before + 1)
        expect(warnings.at(-1)).toContain(failure)
        expect(await listed()).toEqual(remaining)
        expect(warnings).toHaveLength(before + 1)
      }

      await writeFile(skill, original)
      expect(await listed()).toEqual([...names])
      await fiber.dispose()
    } finally {
      await ctx.fiber.dispose()
      await rm(root, { recursive: true, force: true })
    }
  })

  it('reads extra invocation fields and rejects a load whose file became unreadable', async () => {
    const { root, skill } = await relocatedAssets('dsh-engineering-policy-', 'eng-implement')
    const ctx = new Context()
    try {
      const warnings = captureWarnings(ctx)
      await ctx.plugin(SkillRegistry)
      const registerProvider = vi.spyOn(ctx.skills, 'registerProvider')
      const raw = await readFile(skill, 'utf8')
      await writeFile(skill, raw.replace('disable-model-invocation: true\n', 'user-invocable: false\n'))
      const fiber = await ctx.plugin(SkillEngineering, { assetRoot: root })
      const provider: SkillProvider = registerProvider.mock.calls[0]![0]({ signal: new AbortController().signal, invalidate: () => {} })
      const listed = await observe(provider)
      expect(listed.candidates.find(entry => entry.name === 'eng-implement')?.invocation)
        .toEqual({ modelInvocable: true, userInvocable: false })

      await ctx.skills.list()
      await rm(skill)
      await mkdir(skill)
      expect((await observe(provider)).complete).toBe(false)
      expect(warnings.at(-1)).toContain('skill "eng-implement" is unavailable')
      await expect(ctx.skills.get('eng-implement')).rejects.toThrow()
      await fiber.dispose()
    } finally {
      await ctx.fiber.dispose()
      await rm(root, { recursive: true, force: true })
    }
  })

  it('treats a read failure without an error code as transient', async () => {
    const { root } = await relocatedAssets('dsh-engineering-transient-', 'eng-tdd')
    const ctx = new Context()
    try {
      const warnings = captureWarnings(ctx)
      await ctx.plugin(SkillRegistry)
      await ctx.plugin(SkillEngineering, { assetRoot: root })
      vi.mocked(readFile).mockRejectedValueOnce(new Error('locked by another process'))
      const observed = await ctx.skills.snapshot()
      expect(observed.complete).toBe(false)
      expect(observed.skills.map(entry => entry.name)).toEqual(names.filter(entry => entry !== 'eng-ask-matt'))
      expect(warnings.at(-1)).toContain('locked by another process')
      expect((await ctx.skills.snapshot()).complete).toBe(true)
    } finally {
      await ctx.fiber.dispose()
      await rm(root, { recursive: true, force: true })
    }
  })

  it('stops discovery when its signal aborts', async () => {
    const { root, skill } = await relocatedAssets('dsh-engineering-abort-', 'eng-tdd')
    const ctx = new Context()
    try {
      await ctx.plugin(SkillRegistry)
      const registerProvider = vi.spyOn(ctx.skills, 'registerProvider')
      await rm(skill)
      await ctx.plugin(SkillEngineering, { assetRoot: root })
      const provider: SkillProvider = registerProvider.mock.calls[0]![0]({ signal: new AbortController().signal, invalidate: () => {} })
      await expect(provider.list({ signal: AbortSignal.abort() })).rejects.toMatchObject({ name: 'AbortError' })
    } finally {
      await ctx.fiber.dispose()
      await rm(root, { recursive: true, force: true })
    }
  })
})

/**
 * Registration shared by bundled-skill providers: read packaged `SKILL.md`
 * files, validate their frontmatter, and publish one registry provider that
 * reports missing or invalid skills without taking the catalog down.
 *
 * A provider supplies its identity, skill directory names, and a parser; this
 * module owns the discovery/load lifecycle, failure reporting and candidate
 * projection so every bundled provider behaves the same way.
 *
 * @module @deepseek-ai/dsh-skill/src/bundled-provider
 */

import { readFile } from 'node:fs/promises'
import { isAbsolute, join, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import type { Context } from '@deepseek-ai/cordis'
import { parse as parseYaml } from 'yaml'

import { BUNDLED_SKILL_RANK, type SkillCandidate, type SkillInvocationPolicy } from './index.ts'

/** One parsed bundled skill: catalog metadata plus the instruction body. */
export interface BundledSkill {
  /** Routing description rendered in the session catalog. */
  readonly description: string
  /** Normalized model and user invocation controls. */
  readonly invocation: SkillInvocationPolicy
  /** Instruction body with the frontmatter removed. */
  readonly content: string
}

/**
 * Parse one packaged `SKILL.md`.
 * @param skillName - directory name the file must match, when the parser checks it.
 * @param raw - complete file text.
 * @param path - file path used in failure messages.
 * @returns the catalog metadata and trimmed body.
 * @throws When the frontmatter or a required field is invalid.
 */
export type BundledSkillParser = (skillName: string, raw: string, path: string) => BundledSkill

/** One bundled provider's identity and asset rules. */
export interface BundledSkillSource {
  /** Provider name reported by the registry and the `/` menu grouping. */
  readonly providerName: string
  /** Prefix every warning carries. */
  readonly logPrefix: string
  /** Upstream project the packaged text comes from; omitted for the product's own text. */
  readonly upstream?: string
  /** Skill directory names, in catalog order. */
  readonly skillNames: readonly string[]
  /** Parser for this provider's frontmatter contract. */
  readonly parse: BundledSkillParser
  /** Rewrite a loaded body, for example to append installation guidance. */
  readonly render?: (skillName: string, skill: BundledSkill) => string
}

/** Outcome of reading one skill file: parsed metadata, or a failure that may clear on a later read. */
type SkillRead =
  | { readonly kind: 'loaded'; readonly skill: BundledSkill }
  | { readonly kind: 'failed'; readonly error: unknown; readonly persistent: boolean }

/** Missing paths persist until the file appears; other read errors, such as locks, may clear. */
function persistentReadFailure(error: unknown): boolean {
  const code = error instanceof Error && 'code' in error ? error.code : undefined
  return code === 'ENOENT' || code === 'ENOTDIR'
}

/** Read one non-empty string field, treating any other value as absent. */
function stringField(data: object, key: string): string | undefined {
  const value = (data as Record<string, unknown>)[key]
  return typeof value === 'string' && value.length > 0 ? value : undefined
}

/** Read one optional boolean field, rejecting values the frontmatter contract does not allow. */
function booleanField(data: object, key: string, logPrefix: string): boolean | undefined {
  const value = (data as Record<string, unknown>)[key]
  if (value === undefined) return undefined
  if (typeof value !== 'boolean') throw new Error(`${logPrefix}: frontmatter field "${key}" must be a boolean`)
  return value
}

/**
 * Build the parser for the strict frontmatter contract: the directory name,
 * description, and boolean invocation fields are required or defaulted.
 * @param logPrefix - prefix every failure message carries.
 * @returns the parser passed to {@link registerBundledSkillProvider}.
 */
export function strictSkillParser(logPrefix: string): BundledSkillParser {
  return (skillName, raw, path) => {
    const frontmatter = /^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/u.exec(raw)
    if (frontmatter?.[1] === undefined) throw new Error(`${logPrefix}: ${path} has no YAML frontmatter`)
    const metadata: unknown = parseYaml(frontmatter[1])
    if (typeof metadata !== 'object' || metadata === null) throw new Error(`${logPrefix}: ${path} has no metadata`)
    const declared = stringField(metadata, 'name')
    if (declared !== skillName) {
      throw new Error(`${logPrefix}: ${path} declares name ${JSON.stringify(declared)}, expected ${JSON.stringify(skillName)}`)
    }
    const description = stringField(metadata, 'description')
    if (description === undefined) throw new Error(`${logPrefix}: ${path} has no description`)
    return {
      description,
      invocation: {
        modelInvocable: booleanField(metadata, 'disable-model-invocation', logPrefix) !== true,
        userInvocable: booleanField(metadata, 'user-invocable', logPrefix) !== false,
      },
      content: raw.slice(frontmatter[0].length).trim(),
    }
  }
}

/**
 * Build the parser for providers whose frontmatter carries only a description;
 * the skill name is the directory and both invocation surfaces stay enabled.
 * @param logPrefix - prefix every failure message carries.
 * @returns the parser passed to {@link registerBundledSkillProvider}.
 */
export function descriptionSkillParser(logPrefix: string): BundledSkillParser {
  return (_skillName, raw, path) => {
    const frontmatter = /^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/u.exec(raw)
    if (frontmatter?.[1] === undefined) throw new Error(`${logPrefix}: ${path} has no YAML frontmatter`)
    const metadata: unknown = parseYaml(frontmatter[1])
    const description = typeof metadata === 'object' && metadata !== null && 'description' in metadata
      ? metadata.description : undefined
    if (typeof description !== 'string' || description.length === 0) {
      throw new Error(`${logPrefix}: ${path} has no description`)
    }
    return {
      description,
      invocation: { modelInvocable: true, userInvocable: true },
      content: raw.slice(frontmatter[0].length).trim(),
    }
  }
}

/**
 * Resolve a provider's assets directory, defaulting to the packaged location.
 * @param assetRoot - configured absolute directory, when set.
 * @param packaged - the provider's packaged assets URL.
 * @param logPrefix - prefix the failure message carries.
 * @returns the absolute assets directory.
 * @throws When a configured asset root is relative.
 */
export function bundledAssetRoot(assetRoot: string | undefined, packaged: URL, logPrefix: string): string {
  const root = assetRoot ?? fileURLToPath(packaged)
  if (!isAbsolute(root)) throw new Error(`${logPrefix}: assetRoot must be an absolute directory`)
  return root
}

/**
 * Register one bundled provider for its skill directories.
 *
 * Skill files are read during catalog discovery and loading, and each distinct
 * failure is reported once. A missing or invalid skill is omitted from a
 * complete catalog, and its failed load returns `undefined` and invalidates the
 * catalog. Any other read error leaves discovery incomplete, so the registry
 * retries it, and rejects a load.
 * @param ctx - Context whose skill registry receives the provider once the service is available.
 * @param source - provider identity, skill names, parser and optional body rewrite.
 * @param assetRoot - absolute directory holding one directory per skill.
 */
export function registerBundledSkillProvider(ctx: Context, source: BundledSkillSource, assetRoot: string): void {
  const reportedFailures = new Map<string, string>()

  async function readSkill(skillName: string, signal: AbortSignal | undefined): Promise<SkillRead> {
    const path = join(assetRoot, skillName, 'SKILL.md')
    let raw: string
    try {
      raw = await readFile(path, { encoding: 'utf8', signal })
    } catch (error) {
      if (signal?.aborted === true) throw error
      return failed(skillName, error, persistentReadFailure(error))
    }
    try {
      const skill = source.parse(skillName, raw, path)
      reportedFailures.delete(skillName)
      return { kind: 'loaded', skill }
    } catch (error) {
      // Invalid metadata, including YAML errors that carry their own codes, persists until the file changes.
      return failed(skillName, error, true)
    }
  }

  function failed(skillName: string, error: unknown, persistent: boolean): SkillRead {
    const failure = String(error)
    if (reportedFailures.get(skillName) !== failure) {
      reportedFailures.set(skillName, failure)
      ctx.logger.warn(`${source.logPrefix}: skill "${skillName}" is unavailable: ${failure}`)
    }
    return { kind: 'failed', error, persistent }
  }

  // A composition that removes the skill registry keeps this row idle instead of reporting a pending injection.
  ctx.inject(['skills'], (scope) => {
    scope.skills.registerProvider(control => ({
      name: source.providerName,
      shipped: true,
      async list(options) {
        const reads = await Promise.all(source.skillNames.map(async skillName => ({
          skillName, read: await readSkill(skillName, options.signal),
        })))
        const candidates: SkillCandidate[] = []
        for (const { skillName, read } of reads) {
          if (read.kind === 'failed') continue
          const directory = join(assetRoot, skillName)
          candidates.push({
            name: skillName,
            description: read.skill.description,
            invocation: read.skill.invocation,
            provider: source.providerName,
            source: 'bundled',
            ...(source.upstream === undefined ? {} : { upstream: source.upstream }),
            rank: BUNDLED_SKILL_RANK,
            resourceBase: { kind: 'directory', path: directory.split(sep).join('/') },
            locator: join(directory, 'SKILL.md'),
          })
        }
        return { candidates, complete: reads.every(({ read }) => read.kind === 'loaded' || read.persistent) }
      },
      async get(candidate, options) {
        const { rank: _rank, locator: _locator, ...summary } = candidate
        const read = await readSkill(candidate.name, options.signal)
        if (read.kind === 'failed') {
          if (!read.persistent) throw read.error
          control.invalidate()
          return undefined
        }
        return { ...summary, content: source.render?.(candidate.name, read.skill) ?? read.skill.content }
      },
    }))
  })
}

/** Built-in rea skill: evidence-based reverse engineering of binaries, applications and runtime behavior. */

import { readFile } from 'node:fs/promises'
import { isAbsolute, join, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import type { Context } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import {
  BUNDLED_SKILL_RANK,
  type SkillCandidate,
  type SkillInvocationPolicy,
} from '@deepseek-ai/dsh-skill'
import { parse as parseYaml } from 'yaml'

/** Log prefix and provider identity shared by every failure report. */
const PROVIDER_NAME = 'dsh-rea'
const LOG_PREFIX = 'skill-rea'

/** Upstream project the packaged skill text is taken from; labels the skill in the `/` menu. */
const UPSTREAM = 'morluto/rea'

/** Packaged skill directory; its `references/` guides ship beside it. */
const SKILL_NAME = 'dsh-rea'

/** Bundled skill asset location. */
export interface Config {
  /** Absolute assets directory containing the skill directory; defaults to the packaged assets. */
  assetRoot?: string
}

/** Validated resource configuration. */
export const Config: z<Config> = z.object({
  assetRoot: z.string().min(1),
})

/** Cordis plugin identity. */
export const name = 'skill-rea'

/** One parsed skill: catalog metadata plus the instruction body. */
interface ParsedSkill {
  /** Routing description rendered in the session catalog. */
  readonly description: string
  /** Normalized model and user invocation controls. */
  readonly invocation: SkillInvocationPolicy
  /** Instruction body with the frontmatter removed. */
  readonly content: string
}

/** Outcome of reading the skill file: parsed metadata, or a failure that may clear on a later read. */
type SkillRead =
  | { readonly kind: 'loaded'; readonly skill: ParsedSkill }
  | { readonly kind: 'failed'; readonly error: unknown; readonly persistent: boolean }

/** Missing paths persist until the file appears; other read errors, such as locks, may clear. */
function persistentReadFailure(error: unknown): boolean {
  const code = error instanceof Error && 'code' in error ? error.code : undefined
  return code === 'ENOENT' || code === 'ENOTDIR'
}

/**
 * Parse the packaged `SKILL.md`, requiring the directory name, description, and boolean invocation fields.
 * @param skillName - directory name the skill must declare.
 * @param raw - complete file text.
 * @param path - file path used in failure messages.
 * @returns the routing description, invocation controls, and trimmed body.
 * @throws When the frontmatter, name, description, or invocation fields are invalid.
 */
function parseSkill(skillName: string, raw: string, path: string): ParsedSkill {
  const frontmatter = /^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/u.exec(raw)
  if (frontmatter?.[1] === undefined) throw new Error(`${LOG_PREFIX}: ${path} has no YAML frontmatter`)
  const metadata: unknown = parseYaml(frontmatter[1])
  if (typeof metadata !== 'object' || metadata === null) throw new Error(`${LOG_PREFIX}: ${path} has no metadata`)
  const declared = stringField(metadata, 'name')
  if (declared !== skillName) {
    throw new Error(`${LOG_PREFIX}: ${path} declares name ${JSON.stringify(declared)}, expected ${JSON.stringify(skillName)}`)
  }
  const description = stringField(metadata, 'description')
  if (description === undefined) throw new Error(`${LOG_PREFIX}: ${path} has no description`)
  return {
    description,
    invocation: {
      modelInvocable: booleanField(metadata, 'disable-model-invocation') !== true,
      userInvocable: booleanField(metadata, 'user-invocable') !== false,
    },
    content: raw.slice(frontmatter[0].length).trim(),
  }
}

/** Read one non-empty string field, treating any other value as absent. */
function stringField(data: object, key: string): string | undefined {
  const value = (data as Record<string, unknown>)[key]
  return typeof value === 'string' && value.length > 0 ? value : undefined
}

/** Read one optional boolean field, rejecting values the frontmatter contract does not allow. */
function booleanField(data: object, key: string): boolean | undefined {
  const value = (data as Record<string, unknown>)[key]
  if (value === undefined) return undefined
  if (typeof value !== 'boolean') throw new Error(`${LOG_PREFIX}: frontmatter field "${key}" must be a boolean`)
  return value
}

/**
 * Register the packaged rea skill as one bundled provider.
 * The skill file is read during catalog discovery and loading, and each distinct failure is reported once. A missing
 * or invalid skill is omitted from a complete catalog, and its failed load returns `undefined` and invalidates the
 * catalog. Any other read error leaves discovery incomplete, so the registry retries it, and rejects a load.
 * @param ctx - Context whose skill registry receives the provider once the service is available.
 * @param config - Resolved configuration; `assetRoot` falls back to the packaged assets.
 * @throws When `assetRoot` is not absolute.
 */
export function apply(ctx: Context, config: Config): void {
  const assetRoot = config.assetRoot ?? fileURLToPath(new URL('../assets/', import.meta.url))
  if (!isAbsolute(assetRoot)) throw new Error(`${LOG_PREFIX}: assetRoot must be an absolute directory`)
  const directory = join(assetRoot, SKILL_NAME)
  const reportedFailures = new Map<string, string>()

  async function readSkill(signal: AbortSignal | undefined): Promise<SkillRead> {
    const path = join(directory, 'SKILL.md')
    let raw: string
    try {
      raw = await readFile(path, { encoding: 'utf8', signal })
    } catch (error) {
      if (signal?.aborted === true) throw error
      return failed(error, persistentReadFailure(error))
    }
    try {
      const skill = parseSkill(SKILL_NAME, raw, path)
      reportedFailures.delete('read')
      return { kind: 'loaded', skill }
    } catch (error) {
      // Invalid metadata, including YAML errors that carry their own codes, persists until the file changes.
      return failed(error, true)
    }
  }

  function failed(error: unknown, persistent: boolean): SkillRead {
    const failure = String(error)
    if (reportedFailures.get('read') !== failure) {
      reportedFailures.set('read', failure)
      ctx.logger.warn(`${LOG_PREFIX}: skill "${SKILL_NAME}" is unavailable: ${failure}`)
    }
    return { kind: 'failed', error, persistent }
  }

  // A composition that removes the skill registry keeps this row idle instead of reporting a pending injection.
  ctx.inject(['skills'], (scope) => {
    scope.skills.registerProvider(control => ({
      name: PROVIDER_NAME,
      shipped: true,
      async list(options) {
        const read = await readSkill(options.signal)
        const candidates: SkillCandidate[] = read.kind === 'failed' ? [] : [{
          name: SKILL_NAME,
          description: read.skill.description,
          invocation: read.skill.invocation,
          provider: PROVIDER_NAME,
          source: 'bundled',
          upstream: UPSTREAM,
          rank: BUNDLED_SKILL_RANK,
          resourceBase: { kind: 'directory', path: directory.split(sep).join('/') },
          locator: join(directory, 'SKILL.md'),
        }]
        return { candidates, complete: read.kind === 'loaded' || read.persistent }
      },
      async get(candidate, options) {
        const { rank: _rank, locator: _locator, ...summary } = candidate
        const read = await readSkill(options.signal)
        if (read.kind === 'failed') {
          if (!read.persistent) throw read.error
          control.invalidate()
          return undefined
        }
        return { ...summary, content: read.skill.content }
      },
    }))
  })
}

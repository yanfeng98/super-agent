/** Built-in engineering workflow skills: grilling, specs, tickets, TDD, code review, domain modeling, and writing. */

import type { Context } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import { bundledAssetRoot, registerBundledSkillProvider, strictSkillParser } from '@deepseek-ai/dsh-skill/bundled-provider'

/** Log prefix and provider identity shared by every failure report. */
const PROVIDER_NAME = 'dsh-engineering'
const LOG_PREFIX = 'skill-engineering'

/** Upstream project the packaged skill text is taken from; labels the skills in the `/` menu. */
const UPSTREAM = 'mattpocock/skills'

/** Packaged skill directories, in catalog order. */
const SKILL_NAMES = [
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

/** Bundled skill asset location. */
export interface Config {
  /** Absolute assets directory containing one directory per skill; defaults to the packaged assets. */
  assetRoot?: string
}

/** Validated resource configuration. */
export const Config: z<Config> = z.object({ assetRoot: z.string().min(1) })

/** Cordis plugin identity. */
export const name = 'skill-engineering'

/**
 * Register the packaged engineering workflow skills as one bundled provider.
 * Skill files are read during catalog discovery and loading, and each distinct failure is reported once. A missing
 * or invalid skill is omitted from a complete catalog, and its failed load returns `undefined` and invalidates the
 * catalog. Any other read error leaves discovery incomplete, so the registry retries it, and rejects a load.
 * @param ctx - Context whose skill registry receives the provider once the service is available.
 * @param config - Resolved configuration; `assetRoot` falls back to the packaged assets.
 */
export function apply(ctx: Context, config: Config): void {
  registerBundledSkillProvider(ctx, {
    providerName: PROVIDER_NAME,
    logPrefix: LOG_PREFIX,
    upstream: UPSTREAM,
    skillNames: SKILL_NAMES,
    parse: strictSkillParser(LOG_PREFIX),
  }, bundledAssetRoot(config.assetRoot, new URL('../assets/', import.meta.url), LOG_PREFIX))
}

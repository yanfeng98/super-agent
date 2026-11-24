/** Built-in rea skill: evidence-based reverse engineering of binaries, applications and runtime behavior */

import type { Context } from '@deepseek-ai/cordis'
import type { BundledSkillSource } from '@deepseek-ai/dsh-skill/bundled-provider'
import z from '@deepseek-ai/schemastery'
import { bundledAssetRoot, registerBundledSkillProvider, strictSkillParser } from '@deepseek-ai/dsh-skill/bundled-provider'

/** Log prefix and provider identity shared by every failure report. */
const PROVIDER_NAME = 'dsh-rea'
const LOG_PREFIX = 'skill-rea'

/** Upstream project the packaged skill text is taken from; labels the skill in the `/` menu. */
const UPSTREAM = 'morluto/rea'

/** Packaged skill directory; its `references/` guides ship beside it. */
const SKILL_NAME = 'dsh-rea'

// Every bundled provider declares its own config type because the config catalog
// pastes that declaration verbatim and rejects an imported one; the transport and
// registration bodies are shared through bundled-provider.ts.
/* jscpd:ignore-start */
/** Bundled skill asset location. */
export interface Config {
  /** Absolute assets directory containing the skill directory; defaults to the packaged assets. */
  assetRoot?: string
}

/** Validated resource configuration. */
export const Config: z<Config> = z.object({ assetRoot: z.string().min(1) })
/* jscpd:ignore-end */

/** Provider identity and asset rules shared by registration and failure reporting. */
const SOURCE = {
  providerName: PROVIDER_NAME,
  logPrefix: LOG_PREFIX,
  upstream: UPSTREAM,
  skillNames: [SKILL_NAME],
  parse: strictSkillParser(LOG_PREFIX),
} satisfies BundledSkillSource

/** Cordis plugin identity. */
export const name = 'skill-rea'

/**
 * Register the packaged rea skill as one bundled provider.
 * The skill file is read during catalog discovery and loading, and each distinct failure is reported once. A missing
 * or invalid skill is omitted from a complete catalog, and its failed load returns `undefined` and invalidates the
 * catalog. Any other read error leaves discovery incomplete, so the registry retries it, and rejects a load.
 * @param ctx - Context whose skill registry receives the provider once the service is available.
 * @param config - Resolved configuration; `assetRoot` falls back to the packaged assets.
 */
export function apply(ctx: Context, config: Config): void {
  registerBundledSkillProvider(ctx, SOURCE, bundledAssetRoot(config.assetRoot, new URL('../assets/', import.meta.url), LOG_PREFIX))
}

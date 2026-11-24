/** Bundled Office workflows and filesystem resources for document authoring and operations. */

import type { Context } from '@deepseek-ai/cordis'
import z from '@deepseek-ai/schemastery'
import { bundledAssetRoot, descriptionSkillParser, registerBundledSkillProvider } from '@deepseek-ai/dsh-skill/bundled-provider'

const SKILL_NAMES = ['office'] as const

/**
 * Python source for Office scripts: `'workspace-dependencies'` directs the model to the bundled interpreter and libraries
 * returned by `load_workspace_dependencies`; `'user'` directs it to an existing project or user Python.
 */
export type PythonEnvironment = 'workspace-dependencies' | 'user'

const PYTHON_ENVIRONMENT_SECTIONS: Readonly<Record<PythonEnvironment, string>> = {
  'workspace-dependencies': 'Follow an explicit user or applicable AGENTS.md requirement for a Python environment. Otherwise call `load_workspace_dependencies` and run scripts with its Python executable and bundled libraries. Do not install packages or search for a system Python for the default workflow. If the tool is unavailable, use an already configured environment and report a missing dependency only when it prevents the requested operation.',
  user: 'Follow an explicit user or applicable AGENTS.md requirement for a Python environment. Otherwise use the project\'s or user\'s existing Python, such as `python3` or `python` on PATH. Before the first script, check that this Python can import the libraries the workflow needs. If Python or a library is missing, tell the user which packages to install; do not install packages into a global environment without the user\'s agreement.',
}

/** Office skill resources and Python environment. */
export interface Config {
  /** Absolute assets directory containing the `office` skill folder; defaults to packaged assets. */
  assetRoot?: string
  /** Python environment named at the end of the `office` skill. @default 'workspace-dependencies' */
  pythonEnvironment?: PythonEnvironment
}

/** Configuration after schema defaults apply. */
export interface ResolvedConfig extends Config {
  /** Python environment named at the end of the `office` skill. */
  pythonEnvironment: PythonEnvironment
}

/** Validated resource configuration. */
export const Config: z<Config, ResolvedConfig> = z.object({
  assetRoot: z.string().min(1),
  pythonEnvironment: z.union([z.const('workspace-dependencies'), z.const('user')]).default('workspace-dependencies'),
})

/** Cordis plugin identity. */
export const name = 'skill-office'

/** The MCP client publishes skills through the registry. */
export const inject = ['skills']

/**
 * Register the `office` skill whose `office/SKILL.md` the provider reads from `assetRoot` during catalog discovery and
 * loading; each distinct failure is reported once as a warning. A missing or invalid skill file omits the skill from a
 * complete catalog, and its failed load returns `undefined` and invalidates the catalog. Any other read error leaves
 * discovery incomplete, so the registry retries it, and rejects a load.
 * @param ctx - Context carrying the skill registry.
 * @param config - Resolved configuration; `assetRoot` falls back to the packaged assets.
 */
export function apply(ctx: Context, config: ResolvedConfig): void {
  registerBundledSkillProvider(ctx, {
    providerName: 'dsh-office',
    logPrefix: 'skill-office',
    skillNames: SKILL_NAMES,
    parse: descriptionSkillParser('skill-office'),
    render: (_skillName, skill) => `${skill.content}\n\n## Python environment\n\n${PYTHON_ENVIRONMENT_SECTIONS[config.pythonEnvironment]}`,
  }, bundledAssetRoot(config.assetRoot, new URL('../assets/', import.meta.url), 'skill-office'))
}

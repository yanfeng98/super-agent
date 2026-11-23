/** Plugin configuration and machine settings for dsh-superagent. */
import { LIMITS } from './limits.ts'
import { homedir } from 'node:os'
import { isAbsolute, join } from 'node:path'
import type { Volatile } from '@deepseek-ai/cordis'
import s from '@deepseek-ai/schemastery'

// Accept POSIX roots, Windows drive roots and UNC paths; `machineOf` checks the result with `isAbsolute`.
const absolute = (description: string) => s.string().pattern(/^(?:\/|[A-Za-z]:[\\/]|\\\\)/)
  .description(`${description} Absolute path.`)
/**
 * The state directory a fresh installation uses: `<DSH_HOME>/state/superagent`, where `DSH_HOME` defaults to
 * `~/.dsh`. The built-in copy starts without machine configuration, so the first Web profile that mounts it needs no
 * profile edit.
 */
export const defaultStateDir = (): string => join(process.env.DSH_HOME ?? join(homedir(), '.dsh'), 'state', 'superagent')
/** Plugin configuration of the dsh-superagent entry. Volatile fields apply without a restart because dsh-superagent reads them on each use; changing `stateDir` remounts the plugin. */
export const Config = s.object({
  stateDir: absolute('Directory for the database, locks, worker shims, the agent\'s git configuration and, by default, the ticket directories.')
    .default(defaultStateDir()),
  promptMaxChars: s.number().min(LIMITS.promptMaxChars.min).max(LIMITS.promptMaxChars.max).step(1).default(100000)
    .description('Largest saved prompt configuration, in characters.').volatile(),
  admissionContextMaxPages: s.number().min(LIMITS.admissionContextMaxPages.min).max(LIMITS.admissionContextMaxPages.max).step(1).default(10)
    .description('Pages of thread context one assessment may read.').volatile(),
  admissionContextTimeoutMs: s.number().min(LIMITS.admissionContextTimeoutMs.min).max(LIMITS.admissionContextTimeoutMs.max).step(1)
    .default(15000).description('Time limit of one context read during an assessment, in milliseconds.').volatile(),
})
export type DeploymentConfig = ReturnType<typeof Config>
export interface Machine {
  stateDir: string
  /**
   * Working tree of the git repository containing this installation, from which a Ticket gets a linked worktree when
   * neither its project nor the Execution settings configure a prepare command. Absent when the installation is
   * outside every git repository, which leaves a Ticket's directory empty.
   */
  repository?: string
  promptMaxChars: Volatile<number>
  admissionContextMaxPages: Volatile<number>
  admissionContextTimeoutMs: Volatile<number>
}
const MOVED: Record<string, string> = {
  repository: 'derived from the checkout this installation runs from', branchPrefix: 'in the prepare command of the Execution settings',
  feishu: 'on the Feishu Source', author: 'prompt text',
  repositoryPath: 'in the prepare command of the Execution settings',
  worktreeRoot: 'the ticket directory of the Execution settings', blockedCommands: 'in the Execution settings',
}

/**
 * Read machine settings and reject unknown configuration keys (naming where deprecated keys moved).
 * @param config - The plugin configuration.
 * @param repository - The working tree containing this installation, from which Tickets get a worktree by default.
 */
export function machineOf(config: DeploymentConfig, repository?: string): Machine {
  const unknown = Object.keys(config).filter(key => !(key in (Config.dict ?? {})))
  if (unknown.length) throw new Error(`dsh-superagent configuration has keys it does not use: ${unknown.map(key => MOVED[key] ? `${key} (now ${MOVED[key]})` : key).join(', ')}; docs/006-deployment.md lists the keys`)
  const { stateDir, promptMaxChars, admissionContextMaxPages, admissionContextTimeoutMs } = config
  if (!isAbsolute(stateDir)) throw new Error(`stateDir must be an absolute path: ${stateDir}`)
  return { stateDir, ...repository ? { repository } : {}, promptMaxChars, admissionContextMaxPages, admissionContextTimeoutMs }
}

/** What startup resolved: the machine settings the Sources and Sessions run with. */
export type Deployment = Machine

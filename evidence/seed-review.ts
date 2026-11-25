// Seed one fresh Superagent state directory with local tickets that cover the review worklist.
// Usage: pnpm exec tsx evidence/seed-review.ts <stateDir>
import { Store } from '../apps/web/native/dsh-superagent/src/app/store.ts'
import { SettingsSchema } from '../apps/web/native/dsh-superagent/src/policy.ts'

const dir = process.argv[2]
if (!dir) throw new Error('usage: tsx evidence/seed-review.ts <stateDir>')
const store = new Store(`${dir}/orchestrator.sqlite`)
const at = '2026-11-25T04:00:00.000Z'
const settings = SettingsSchema.parse({
  policy: { sources: { local: { allowed: [{ id: 'operator', name: 'Operator' }] } },
    rules: [{ id: 'local', name: 'Local work', enabled: true, instruction: 'Do local work' }] },
})
store.saveSettings(settings)
const base = {
  channels: [] as never[], undelivered: [] as string[], createdAt: at, updatedAt: at,
  mandate: { strength: 'explicit' as const, policyVersion: 'v', basis: { via: 'listed' as const },
    actor: { source: 'local', id: 'operator' }, scope: '' },
  origin: { triggerId: 'seed', triggerIds: ['seed'] }, assignments: [], inputs: [] as never[],
}
const review = { capturedAt: at, branch: 'agent/seed', base: 'a'.repeat(40), head: 'b'.repeat(40),
  commits: [{ sha: 'b123456', subject: 'Change' }], files: [{ status: 'M', path: 'src/example.ts' }], dirty: [] }
const save = (id: string, title: string, patch: Record<string, unknown>) => store.tickets.save({ ...base, id, title, decisionId: 'seed', ...patch } as never)
save('t1', '已完成但未请求合并', { ended: { at, by: 'executor', outcome: 'completed' }, review })
save('t2', '已放弃', { ended: { at, by: 'human', outcome: 'stopped', reason: 'No longer needed' }, review })
save('t3', '有改动，等待合并批准', { ended: { at, by: 'executor', outcome: 'completed' }, review,
  autoMerge: { at, attempts: 0 } })
save('t4', '已合并', { ended: { at, by: 'executor', outcome: 'completed' }, review,
  merged: { at, branch: 'agent/seed', target: 'main', head: 'b'.repeat(40) } })
save('t5', '已回退', { ended: { at, by: 'executor', outcome: 'completed' }, review,
  merged: { at, branch: 'agent/seed', target: 'main', head: 'b'.repeat(40) },
  reverted: { at, commit: 'c'.repeat(40), target: 'main', merge: 'b'.repeat(40) } })
store.close()
console.log(`seeded ${dir}`)

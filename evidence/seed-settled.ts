// Seed a fresh Superagent state directory for the Execution settled-filter before/after captures.
// Usage: pnpm exec tsx evidence/seed-settled.ts <stateDir>
import { Store } from '../apps/web/native/dsh-superagent/src/app/store.ts'
import { SettingsSchema } from '../apps/web/native/dsh-superagent/src/policy.ts'

const dir = process.argv[2]
if (!dir) throw new Error('usage: tsx evidence/seed-settled.ts <stateDir>')
const store = new Store(`${dir}/state/superagent/orchestrator.sqlite`)
const at = '2026-11-25T04:00:00.000Z'
store.saveSettings(SettingsSchema.parse({
  policy: { sources: { local: { allowed: [{ id: 'operator', name: 'Operator' }] } },
    rules: [{ id: 'local', name: 'Local work', enabled: true, instruction: 'Do local work' }] },
}))
const base = {
  channels: [] as never[], undelivered: [] as string[], createdAt: at, updatedAt: at,
  mandate: { strength: 'explicit' as const, policyVersion: 'v', basis: { via: 'listed' as const },
    actor: { source: 'local', id: 'operator' }, scope: '' },
  origin: { triggerId: 'seed', triggerIds: ['seed'] }, assignments: [], inputs: [] as never[],
}
const save = (id: string, title: string, patch: Record<string, unknown>) =>
  store.tickets.save({ ...base, id, title, decisionId: 'seed', ...patch } as never)
// Two completed tickets nobody asked to merge: an investigation and a temporary cleanup.
save('t1', 'T-0031 本地输入的主 Session 内容同步到飞书会话', { ended: { at, by: 'executor', outcome: 'completed' } })
save('t2', 'T-0029 处理主检出未提交改动以解除合并/推送阻塞', { ended: { at, by: 'executor', outcome: 'completed' } })
// A Ticket with a standing merge request really is waiting to land.
save('t3', 'T-0030 修复筛选条件的分支待合并', { ended: { at, by: 'executor', outcome: 'completed' },
  autoMerge: { at, strategy: 'ff', attempts: 0 } })
save('t4', 'T-0028 已合并的修复', { ended: { at, by: 'executor', outcome: 'completed' },
  merged: { at, branch: 'agent/t4', target: 'main', head: 'b'.repeat(40), push: { at, ok: true } } })
store.close()
console.log(`seeded ${dir}`)

// Seed a fresh Superagent state directory with an old unavailable assessment on a settled Thread
// and one on an open Thread, for the before/after capture of the Trigger-page status fix.
// Usage: pnpm exec tsx evidence/seed-merge-trigger.ts <stateDir>
import { Store } from '../apps/web/native/dsh-superagent/src/app/store.ts'
import { SettingsSchema } from '../apps/web/native/dsh-superagent/src/policy.ts'

const dir = process.argv[2]
if (!dir) throw new Error('usage: tsx evidence/seed-merge-trigger.ts <stateDir>')
const store = new Store(`${dir}/orchestrator.sqlite`)
const at = new Date().toISOString()
store.saveSettings(SettingsSchema.parse({
  policy: { sources: { local: { allowed: [{ id: 'operator', name: 'Operator' }] } },
    rules: [{ id: 'local', name: 'Local work', enabled: true, instruction: 'Do local work' }] },
}))
const base = {
  channels: [] as never[], createdAt: at, updatedAt: at,
  mandate: { strength: 'explicit' as const, policyVersion: 'v1', basis: { via: 'listed' as const },
    actor: { source: 'local', id: 'operator' }, scope: '' },
  origin: { triggerId: 'seed', triggerIds: ['seed'] }, assignments: [], inputs: [] as never[],
}
const ticket = (id: string, title: string, ended: boolean) => store.tickets.save({ ...base, id, title,
  decisionId: `d-${id}`, undelivered: [], ...ended ? { ended: { at, by: 'executor', outcome: 'completed' } } : {},
} as never)
ticket('t-settled', '合并推送已完成，评估问题应消失', true)
ticket('t-open', '仍在进行的工作，评估问题保留', false)
const thread = (key: string, ticketId: string) => store.threads.save({ key, ref: { source: 'local', id: key },
  createdAt: at, lastAt: at, subscriptions: [ticketId], correlated: [], pending: [], evaluating: undefined, meta: {} } as never)
thread('local:settled-merge', 't-settled')
thread('local:open-work', 't-open')
for (const [id, key, ticketId] of [['trigger-settled', 'local:settled-merge', 't-settled'],
  ['trigger-open', 'local:open-work', 't-open']] as const) {
  store.triggers.save({ id, source: 'local', threads: [{ source: 'local', id: key }],
    actor: { source: 'local', id: 'operator' }, human: true, addressed: 'direct', at, text: '合并并推送', meta: {} })
  store.triggers.saveDisposition(id, { fanout: [ticketId], at })
  store.decisions.save({ id: `decision-${id}`, evalId: `eval-${id}`, threadKey: key, triggerIds: [id], policyVersion: 'v1',
    at, kind: 'unavailable', reason: [{ code: 'timeout' }], strength: 'explicit' })
  store.triggers.saveDisposition(id, { fanout: [ticketId], decisionId: `decision-${id}`, at })
}
store.close()
console.log(`seeded ${dir}`)

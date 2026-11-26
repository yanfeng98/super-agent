// Browser fixture for the Execution page's push mark. The page mounts the real
// panel with a fake orchestrator API: one merged Ticket whose recorded push
// failed, and the same Ticket after reconciliation found its commit upstream.
// Set `?landed=1` for the converged state.
import * as React from 'react'
import { createRoot } from 'react-dom/client'
import { App } from '../apps/web/native/dsh-superagent/src/client/app.tsx'
import { zh } from '../apps/web/native/dsh-superagent/src/client/locales.ts'

const t = key => zh[key] ?? key
const landed = new URLSearchParams(location.search).has('landed')
const push = landed ? { ok: true, via: 'auto' }
  : { ok: false, error: 'fatal: no upstream branch' }
const merged = { at: '2026-11-26T11:29:37.406Z', branch: 'agent/fix-feishu-badge', target: 'main',
  head: 'merged-head', push }
const ticket = {
  id: 'ticket-0040', number: 40, title: '修复截图中飞书来源标志不好看的问题',
  tags: [], state: 'ended', stage: 'ended', ended: { at: '2026-11-26T11:29:31.955Z', by: 'executor', outcome: 'dropped' },
  mergeRequested: false, settlement: landed ? 'settled' : 'unfinished', merged, actions: {}, strength: 'explicit',
  assignments: 1, inputs: 1, undelivered: 0, directory: '/repo', summary: '', createdAt: '2026-11-26T11:00:00.000Z',
  updatedAt: '2026-11-26T11:29:37.406Z', threads: [],
}
const routes = {
  'GET state': { paused: false, load: { running: 0, queued: 0, maxWorkers: 2 }, setup: {}, notifications: 0 },
  'GET tags': { tags: [] },
  'GET tickets': { items: [ticket], next_offset: null },
  'GET merge-queue': { items: [], paused: false },
  'GET notifications': { items: [], unread: 0 },
  'GET search': { items: [], threads: [], tickets: [] },
  'GET prompts': { current: { config: {} }, revisions: [] },
  'GET settings': {},
  'GET sources': [],
}
const realFetch = window.fetch.bind(window)
window.fetch = (input, init = {}) => {
  const url = String(input)
  if (!url.includes('/orchestrator/api/')) return realFetch(input, init)
  const path = url.slice(url.indexOf('/orchestrator/api/') + '/orchestrator/api/'.length).split('?')[0]
  const method = init.method || 'GET'
  const key = `${method} ${path}`
  const value = routes[key] ?? {}
  return Promise.resolve(new Response(JSON.stringify(value), { status: 200, headers: { 'content-type': 'application/json' } }))
}
sessionStorage.setItem('dsh-superagent/route', JSON.stringify({ route: { page: 'execution', query: { filter: 'ended' } }, stack: [] }))
createRoot(document.getElementById('app')).render(React.createElement(App, { t, openSession: async () => {}, deploymentForm: undefined }))

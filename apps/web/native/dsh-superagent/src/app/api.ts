/** Type-only management API contract between the web panel and host routes, imported by the panel without bundling host code. */
import type { ModelCatalog } from '@deepseek-ai/dsh-api-session-controller/types'
import type { TicketCheck, TicketNote } from '../core/index.ts'
import type { Notification } from './notifications.ts'
import type { PromptConfig, Prompts } from './prompts.ts'
import type { PromptsView, StateView, TicketPageView } from './http.ts'
import type { HistoryView } from './settings-history.ts'
import type { OverviewView } from './views/overview.ts'
import type { SettingsView } from './views/settings.ts'
import type { SourceStatusView } from './views/sources.ts'
import type { DecisionListView, SearchView, ThreadDetailView, ThreadListView } from './views/threads.ts'
import type { TicketListView } from './views/tickets.ts'

export const TICKET_FILTERS = ['open', 'person', 'working', 'idle', 'ended'] as const
export const THREAD_FILTERS = ['subscribed', 'evaluating', 'recent', 'all'] as const
export type TicketFilter = typeof TICKET_FILTERS[number]
export type ThreadFilter = typeof THREAD_FILTERS[number]
interface Paged { offset?: number; limit?: number }
/** Whether a context update was already injected into the main Session or is still pending delivery. */
type Delivery = 'injected' | 'pending'

export interface Api {
  'GET state': { response: StateView }
  'GET overview': { response: OverviewView }
  'GET search': { query: { q: string; offset?: number }; response: SearchView }
  'GET notifications': { response: { items: Notification[]; unread: number } }
  'POST notifications/:id/read': { response: { read: true } }
  'POST notifications/read-all': { response: { read: number } }
  'GET sources': { response: SourceStatusView[] }
  'GET settings': { response: SettingsView }
  'POST sources/:id/labels': { response: { added: string[] } }
  'GET threads': { query?: Paged & { filter?: ThreadFilter }; response: ThreadListView }
  'GET threads/:key': { query?: { limit?: number; before?: string; around?: string }; response: ThreadDetailView }
  'POST threads/:key/unsubscribe': { response: { removed: boolean; thread: string; context_delivery: Delivery } }
  'POST triggers/:id/requeue': { response: { queued: boolean } }
  'GET decisions': { query?: Paged & { kind?: string; window?: '24h' }; response: DecisionListView }
  'POST decisions/:id/dismiss': { response: { dismissed: true } }
  'GET tickets': { query?: Paged & { state?: TicketFilter; tag?: string }; response: TicketListView }
  'GET tags': { response: { tags: string[] } }
  'GET tickets/:id': { response: TicketPageView }
  'POST tickets/:id/retry': { response: { retried: true } }
  'POST tickets/:id/stop': { response: { stopped: true } }
  'POST tickets/:id/reopen': { response: { reopened: true } }
  'GET tickets/:id/diff': { response: { diff: string; truncated: boolean; error?: string } }
  'POST tickets/:id/merge': { query?: { strategy?: 'ff' | 'squash' | 'merge' };
    response: { merged: true; branch: string; target: string; rebased?: true; recaptured?: true } }
  'POST tickets/:id/discard': { response: { discarded: true } }
  'POST tickets/:id/rebase': { response: { rebased: true; branch: string; target: string } }
  'POST tickets/:id/check': { response: { checks: TicketCheck } }
  'POST tickets/:id/blockers': { body: { ticket: string; blocked: boolean };
    response: { blocked_by: { id: string; number?: number; title: string; merged: boolean }[] } }
  'POST tickets/:id/related': { body: { ticket: string; linked: boolean };
    response: { related: { id: string; number?: number; title: string }[] } }
  'POST tickets/:id/notes': { body: { file: string; line?: number; text: string }; response: { notes: TicketNote[] } }
  'POST tickets/:id/notes/:noteId/remove': { response: { notes: TicketNote[] } }
  'POST tickets/:id/notes/:noteId/resolve': { body: { resolved: boolean }; response: { notes: TicketNote[] } }
  'POST tickets/:id/notes/:noteId/replies': { body: { text: string }; response: { notes: TicketNote[] } }
  'POST tickets/:id/cleanup': { response: { cleaned: true } }
  'POST tickets/:id/title': { body: { title: string }; response: { title: string } }
  'POST tickets/:id/tags': { body: { tags: string[] }; response: { tags: string[] } }
  'POST tickets/:id/review': { body: { decision: 'approved' | 'changes-requested'; note?: string };
    response: { reviewed: true; decision: 'approved' | 'changes-requested'; at: string } }
  'POST tickets/:id/request': { body: { text: string }; response: { request: { text: string; at: string } } }
  'POST tickets/:id/approve': { response: { approved: true } }
  'POST local/requests': { body: { title: string; text: string; key?: string; project?: string; tags?: string[];
    files?: { name: string; data: string }[]; hold?: boolean };
    response: { request: string; thread: string; trigger: string; file: string } }
  'POST local/followups': { body: { thread: string; text: string }; response: { trigger: string; thread: string } }
  'POST executor/pause': { body: { paused: boolean }; response: { paused: boolean } }
  'PUT settings/:block': { body: { value: unknown; revision: number }; response: { revision: number; restarting: boolean } }
  'GET policy/history': { response: HistoryView }
  'POST policy-sync/:id': { response: { policy: unknown } }
  'POST setup/check': { response: { restarting: true } }
  'GET models': { response: ModelCatalog }
  'GET prompts': { query?: { revision?: number; offset?: number }; response: PromptsView }
  'PUT prompts': { body: { config: PromptConfig; baseRevision: number }; response: { revision: number; context_delivery: Delivery } }
  'POST prompts/preview': { body: PromptConfig; response: ReturnType<Prompts['preview']> }
}
type Path<K> = K extends `${string} ${infer P}` ? P : never
type Params<P> = P extends `${string}:${infer Name}/${infer Rest}` ? Record<Name, string> & Params<Rest>
  : P extends `${string}:${infer Name}` ? Record<Name, string> : unknown
export type RouteParams<K extends keyof Api> = Params<Path<K>>
export type ApiRequest<K extends keyof Api> = RouteParams<K> & Omit<Api[K], 'response'>
export type ApiResponse<K extends keyof Api> = Api[K]['response']

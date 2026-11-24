/**
 * Derives the workspace browser tree from caller-projected Workspace and
 * Session order. Unassigned Sessions trail under Other sessions; only the selected
 * blank Session remains visible.
 */
import {
  type SessionListState, type SessionSearchResultItem, type SessionSummary,
} from '@deepseek-ai/dsh-api-session-controller/client'
import { selectSessionDirectory } from '@deepseek-ai/dsh-api-session-controller/client/directories'
import type { WorkspaceId, WorkspaceView, WorkspaceWorktree } from '@deepseek-ai/dsh-api-workspace-controller/client'
import type {
  SessionStatusSnapshot,
} from '@deepseek-ai/dsh-client-ui-session-status/client'
import type { SessionId } from '@deepseek-ai/dsh-session/types'
import { TextSearchIndex, type SearchKeys } from '@deepseek-ai/dsh-util-text-search'
import { assertNever } from '@deepseek-ai/dsh-util-values'
import { workspaceTitleOf } from '@deepseek-ai/dsh-util-workspace-path'

/** Group key for Sessions outside every Workspace. */
export const UNGROUPED_KEY = ''

/**
 * Resolve the Workspace browser group that owns one Session.
 * @param workspaces - authoritative Workspace membership.
 * @param sessionId - Session whose browser group is required.
 * @param summaries - current Session list entries, which carry any durable grouping hint.
 * @returns owning Workspace id (membership or a durable group hint), or {@link UNGROUPED_KEY} when no Workspace accounts for it.
 */
export function owningGroupKey(
  workspaces: readonly WorkspaceView[],
  sessionId: SessionId,
  summaries?: SessionListState['byId'],
): string {
  const member = workspaces.find(workspace => workspace.sessionIds.includes(sessionId))
  if (member !== undefined) return member.workspaceId
  const hinted = workspaceGroupPath(summaries?.[sessionId])
  const owner = hinted === undefined ? undefined : workspaces.find(workspace => workspace.path === hinted)
  return owner?.workspaceId ?? UNGROUPED_KEY
}

/**
 * Durable sidebar group hint carried by one Session summary: the Workspace
 * directory it should be browsed under when its header cwd names another
 * project. `undefined` means the Session has no hint.
 * @param summary - Session list entry to read.
 * @returns the hinted Workspace directory, or undefined.
 */
export function workspaceGroupPath(summary: SessionSummary | undefined): string | undefined {
  const hint = summary?.projectionValues?.workspaceGroup
  return hint === null || hint === undefined ? undefined : hint.path
}

/** Pending interaction kinds with dedicated Workspace-row presentation. */
export type SessionPendingInteractionStatus = 'approval' | 'plan-review' | 'question'
type SessionStatuses = SessionStatusSnapshot

function mainSessionId(list: SessionListState): SessionId | undefined {
  return Object.values(list.byId)
    .find(session => (session.retainedBy.mainView ?? 0) > 0)?.id
}

/** One top-level session row in a group or the flat list. */
export interface SessionNode {
  id: SessionId
  /** Stored title, or empty; the renderer localizes blank and unnamed row labels. */
  title: string
  /** The provisional blank session (renderer shows the localized New Session title). */
  blank: boolean
  /** A Session-scoped UI consumer is awaiting this user. */
  pendingInteraction?: SessionPendingInteractionStatus
  running: boolean
  /** Running direct children in the loaded subagent catalog. */
  runningSubagentCount: number
  /** Host-confirmed unread completion or explicit unread mark. */
  completed: boolean
  /** Latest unread completion ended with an execution error. */
  failed: boolean
  /** In the registry-global pin set: leads its section, reorderable only among pinned rows. */
  pinned: boolean
  /** In the registry-global archive set: shown grayed in place and not openable. */
  archived: boolean
  updatedAt: number
  /** Workspace root the Session works in; absent when the Host reports none or an empty path. */
  cwd?: string
  /** Listed under a Workspace by a durable hint rather than workspace membership; not reorderable there. */
  groupedByHint?: boolean
  /** Durable role marker shown beside the row (the IM bot's orchestrator). */
  orchestrator?: boolean
  /** A Ticket's worker Session browsed under its repository once the Ticket's checkout is gone. */
  ticket?: boolean
  /** That Ticket's branch landed on its target. */
  merged?: boolean
  /** That Ticket's terminal outcome when it did not merge. */
  outcome?: 'completed' | 'closed' | 'dropped' | 'stopped'
}

/** Session order selected by the Workspace browser. */
export type SessionOrderBy = 'manual' | 'updated'

/** One workspace group section: header row facts + visible top-level session rows. */
export interface GroupNode {
  /** Group key: the workspace id or {@link UNGROUPED_KEY}. */
  key: string
  /** Backing Workspace id; absent only for the ungrouped bucket. */
  workspaceId: WorkspaceId | undefined
  cwd: string | undefined
  /** Workspace creation time (epoch ms); absent only for the ungrouped bucket. */
  createdAt: number | undefined
  label: string
  /** Display parent while that registration exists; absent for a top-level group or the ungrouped bucket. */
  parentId?: WorkspaceId
  /** Worktree presentation, whose name is the row's subtitle. */
  worktree?: WorkspaceWorktree
  /** Total visible sessions in the group. */
  sessionCount: number
  expanded: boolean
  /** The group contains the selected session (active folder tint; supplied here so the renderer never scans). */
  containsCurrent: boolean
  /** Visible session rows (empty while the group is folded). */
  sessions: readonly SessionNode[]
}

/** One flat search row combining list metadata with an optional content match. */
export interface SearchResultNode {
  id: SessionId
  title: string
  workspace: string
  /** A Session-scoped UI consumer is awaiting this user. */
  pendingInteraction?: SessionPendingInteractionStatus
  running: boolean
  /** Running direct children in the loaded subagent catalog. */
  runningSubagentCount: number
  /** Host-confirmed unread completion or explicit unread mark. */
  completed: boolean
  /** Latest unread completion ended with an execution error. */
  failed: boolean
  /** In the registry-global archive set: shown grayed and not openable. */
  archived: boolean
  snippet?: string
}

/** Bounded merged search projection plus the refine-query hint bit. */
export interface SearchResultSet {
  items: readonly SearchResultNode[]
  hasMore: boolean
}

/** Viewing state consumed by the derivation. */
export interface TreeView {
  expandedGroups: readonly string[]
  /** Browser-local order for Sessions without a backing Workspace account. */
  ungroupedOrder?: readonly string[]
}

interface Group {
  key: string
  workspaceId: WorkspaceId | undefined
  cwd: string | undefined
  createdAt: number | undefined
  label: string
  parentId?: WorkspaceId
  worktree?: WorkspaceWorktree
  sessions: SessionSummary[]
}

/**
 * Directory display label: basename of the path (both separators accepted).
 * Ungrouped-bucket fallback for surfaces without a workspace title.
 * @param cwd - directory path, or undefined for the ungrouped bucket.
 * @returns basename, the raw cwd when it has no basename, or an empty ungrouped marker.
 */
export function workspaceLabel(cwd: string | undefined): string {
  if (cwd === undefined || cwd === '') return ''
  const base = workspaceTitleOf(cwd)
  return base !== '' ? base : cwd
}

/**
 * Project known account members by current Session recency.
 * @param sessionIds - authoritative account membership.
 * @param summaries - current Session summaries; members without a summary are omitted until it arrives.
 * @returns known members newest first, with Session identity as the deterministic tie-break.
 */
export function orderByRecency(
  sessionIds: readonly SessionId[],
  summaries: SessionListState['byId'],
): SessionId[] {
  return sessionIds.flatMap((id) => {
    const summary = summaries[id]
    if (summary === undefined) return []
    return [{ id, rank: summary.updatedAt }]
  })
    .sort((a, b) => {
      if (a.rank !== b.rank) return b.rank - a.rank
      return a.id < b.id ? -1 : 1
    })
    .map(member => member.id)
}

/**
 * Reconcile a browser-local manual order with current account membership.
 * Saved entries retain their relative order; new ordinary members follow them by recency.
 * @param memberIds - authoritative account membership.
 * @param savedOrder - previously saved browser-local order.
 * @param summaries - current Session metadata; unknown new members wait for their summaries.
 * @param rowState - global pin and archive membership; only account members can supplement the order.
 * @returns saved relative positions plus missing members ordered by pin, recency, and archive status.
 */
export function reconcileManualOrder(
  memberIds: readonly SessionId[],
  savedOrder: readonly string[] | undefined,
  summaries: SessionListState['byId'],
  rowState?: Pick<SessionRowState, 'pinnedSessionIds' | 'archivedSessionIds'>,
): SessionId[] {
  const members = new Map(memberIds.map(id => [id as string, id]))
  const included = new Set<string>()
  const ordered: SessionId[] = []
  for (const key of savedOrder ?? []) {
    const id = members.get(key)
    if (id === undefined || included.has(key)) continue
    ordered.push(id)
    included.add(key)
  }
  const archived = new Set(rowState?.archivedSessionIds)
  const pins: SessionId[] = []
  for (const sessionId of rowState?.pinnedSessionIds ?? []) {
    const id = members.get(sessionId)
    if (id === undefined || included.has(id) || archived.has(id) || summaries[id] === undefined) continue
    pins.push(id)
    included.add(id)
  }
  const ordinary: SessionId[] = []
  const archives: SessionId[] = []
  for (const id of orderByRecency([...members.values()].filter(id => !included.has(id)), summaries)) {
    if (archived.has(id)) archives.push(id)
    else ordinary.push(id)
  }
  return [...pins, ...ordered, ...ordinary, ...archives]
}

/**
 * Keep the selected provisional New Session ahead of either base order.
 * @param order - recency or reconciled manual order.
 * @param currentBlank - selected blank Session in this account, when present.
 * @returns a copy with the selected blank first and no duplicate slot.
 */
export function pinCurrentBlank(
  order: readonly SessionId[],
  currentBlank: SessionId | undefined,
): SessionId[] {
  if (currentBlank === undefined) return [...order]
  return [currentBlank, ...order.filter(id => id !== currentBlank)]
}

/**
 * Archived-row visibility choice: the default hides archived rows, `show`
 * mixes them into their kept slots, and `only` restricts the view (and
 * search) to archived rows.
 */
export type ArchivedFilter = 'default' | 'show' | 'only'

/**
 * Ordinary sessions are visible; among blank sessions, only the current one
 * is visible. Subagent children use their parent header catalog; archived
 * sessions follow the archived filter, while their accounting slots remain
 * either way so unarchiving restores position.
 */
function sessionVisible(
  session: SessionSummary,
  current: SessionId | undefined,
  archived: ReadonlySet<SessionId>,
  archivedFilter: ArchivedFilter,
): boolean {
  if (session.origin === 'subagent') return false
  if (session.blank && session.id !== current) return false
  switch (archivedFilter) {
    case 'default':
      return !archived.has(session.id)
    case 'show':
      return true
    case 'only':
      return archived.has(session.id)
    /* v8 ignore next 2 -- closed-union backstop; only reached if the filter is forged */
    default:
      return assertNever(archivedFilter)
  }
}

/** Registry-global row state consumed by every tree derivation. */
export interface SessionRowState {
  /** Registry-global pin ids; pinned rows lead their section in the local order. */
  pinnedSessionIds: readonly SessionId[]
  /** Archive set; members keep their slots and show grayed while visible. */
  archivedSessionIds: readonly SessionId[]
  /** Archived-row visibility choice applied to lists and search alike. */
  archivedFilter: ArchivedFilter
}

/**
 * Keep the visible New Session placeholder first, then partition pinned and
 * ordinary rows without changing either partition's caller order.
 */
function sectionMembers(
  members: readonly SessionSummary[],
  pinned: ReadonlySet<SessionId>,
  archived: ReadonlySet<SessionId>,
): SessionSummary[] {
  const placeholders: SessionSummary[] = []
  const leading: SessionSummary[] = []
  const rest: SessionSummary[] = []
  for (const member of members) {
    if (member.blank) placeholders.push(member)
    else if (!archived.has(member.id) && pinned.has(member.id)) leading.push(member)
    else rest.push(member)
  }
  return [...placeholders, ...leading, ...rest]
}

/**
 * A blank session is the selected Workspace's provisional New Session row;
 * its canonical title never enters search (blank rows are query-excluded)
 * and the renderer localizes its display label. Unnamed history also yields an
 * empty title for localization and does not match a directory-name title search.
 */
function sessionTitle(session: SessionSummary): string {
  return session.blank ? '' : (session.title?.trim() ?? '')
}

/** Build one group without projecting session lineage into presentation. */
function buildGroup(
  key: string,
  workspaceId: WorkspaceId | undefined,
  cwd: string | undefined,
  createdAt: number | undefined,
  label: string,
  members: readonly SessionSummary[],
  presentation: Pick<WorkspaceView, 'parentId' | 'worktree'> = {},
): Group {
  return {
    key, workspaceId, cwd, createdAt, label, sessions: [...members],
    ...presentation.parentId === undefined ? {} : { parentId: presentation.parentId },
    ...presentation.worktree === undefined ? {} : { worktree: presentation.worktree },
  }
}

/**
 * Worktree children that display their Sessions under a registered parent
 * instead of as their own entry: a worktree whose work finished. The child's
 * registration and sessions remain; only the entry moves into the parent.
 * @param workspaces - Workspaces in Host order.
 * @returns child Workspace id to its registered display parent.
 */
export function foldedWorkspaces(workspaces: readonly WorkspaceView[]): ReadonlyMap<WorkspaceId, WorkspaceId> {
  const registered = new Set(workspaces.map(workspace => workspace.workspaceId))
  const folded = new Map<WorkspaceId, WorkspaceId>()
  for (const workspace of workspaces) {
    if (workspace.worktree === undefined || workspace.worktree.active) continue
    const parentId = workspace.parentId
    if (parentId === undefined || parentId === workspace.workspaceId || !registered.has(parentId)) continue
    folded.set(workspace.workspaceId, parentId)
  }
  return folded
}

/**
 * Resolve the group that displays a Workspace's Sessions after finished-worktree folding.
 * @param folded - child Workspace id to registered display parent.
 * @param workspaceId - Workspace whose displayed group is required.
 * @returns the Workspace itself, or the ancestor its worktree folding ends at.
 */
export function displayedWorkspaceId(
  folded: ReadonlyMap<WorkspaceId, WorkspaceId>,
  workspaceId: WorkspaceId,
): WorkspaceId {
  const seen = new Set<WorkspaceId>([workspaceId])
  let current = workspaceId
  for (let parent = folded.get(current); parent !== undefined; parent = folded.get(current)) {
    if (seen.has(parent)) return current
    seen.add(parent)
    current = parent
  }
  return current
}

/**
 * Resolve the Workspace browser group that displays one Session: its owning
 * Workspace or durable group hint, or the parent group a finished worktree
 * folds into.
 * @param workspaces - authoritative Workspace registration and membership.
 * @param sessionId - Session whose displayed group is required.
 * @param summaries - current Session list entries, which carry any durable grouping hint.
 * @returns the displayed group key, or {@link UNGROUPED_KEY} when no Workspace accounts for it.
 */
export function displayGroupKey(
  workspaces: readonly WorkspaceView[],
  sessionId: SessionId,
  summaries?: SessionListState['byId'],
): string {
  const owning = owningGroupKey(workspaces, sessionId, summaries)
  if (owning === UNGROUPED_KEY) return owning
  return displayedWorkspaceId(foldedWorkspaces(workspaces), owning as WorkspaceId)
}

/** Apply a stored Ungrouped order and append newly loose Sessions by recency. */
function orderedUngrouped(
  members: readonly SessionSummary[],
  stored: readonly string[] | undefined,
  summaries: SessionListState['byId'],
): SessionSummary[] {
  const byId = new Map(members.map(session => [session.id as string, session]))
  const ids = stored === undefined
    ? orderByRecency(members.map(session => session.id), summaries)
    : reconcileManualOrder(members.map(session => session.id), stored, summaries)
  return ids.flatMap((id) => {
    const session = byId.get(id)
    /* v8 ignore next -- ids are projected exclusively from the members used to build byId. */
    return session === undefined ? [] : [session]
  })
}

/**
 * Group Sessions by Workspace: one group per caller-ordered entity, with
 * members resolved from caller-ordered sessionIds (including finished
 * worktrees folded into it), then members whose durable group hint names this
 * Workspace or one of those worktrees. Sessions outside every Workspace trail in
 * the browser-local Other sessions order, which falls back to recency before
 * that order is initialized.
 */
function groupByWorkspace(
  list: SessionListState,
  workspaces: readonly WorkspaceView[],
  archived: ReadonlySet<SessionId>,
  archivedFilter: ArchivedFilter,
  ungroupedOrder: readonly string[] | undefined,
): Group[] {
  const current = mainSessionId(list)
  const folded = foldedWorkspaces(workspaces)
  // A finished worktree keeps its Sessions accounted to itself; they render inside the parent's group.
  const foldedMembers = new Map<WorkspaceId, SessionSummary[]>()
  for (const workspace of workspaces) {
    const displayedId = displayedWorkspaceId(folded, workspace.workspaceId)
    if (displayedId === workspace.workspaceId) continue
    const members = foldedMembers.get(displayedId) ?? []
    for (const id of workspace.sessionIds) {
      const summary = list.byId[id]
      if (summary === undefined) continue // account may lead the list pull; the row appears when the summary lands
      if (!sessionVisible(summary, current, archived, archivedFilter)) continue
      members.push(summary)
    }
    foldedMembers.set(displayedId, members)
  }
  const groups: Group[] = []
  const accounted = new Set<SessionId>()
  // A grouped Session's own cwd keeps its Workspace membership; the hint only
  // decides which Workspace section lists it. A hint naming a Workspace that
  // is not registered falls back to the same bucket as any other stray.
  const memberIds = new Set<SessionId>(workspaces.flatMap(workspace => workspace.sessionIds))
  const workspaceByPath = new Map(workspaces.map(workspace => [workspace.path, workspace.workspaceId]))
  const hinted = new Map<WorkspaceId, SessionSummary[]>()
  for (const id of list.ids) {
    const summary = list.byId[id]
    if (summary === undefined || memberIds.has(id)) continue
    const path = workspaceGroupPath(summary)
    const owner = path === undefined ? undefined : workspaceByPath.get(path)
    if (owner === undefined) continue
    accounted.add(id)
    if (!sessionVisible(summary, current, archived, archivedFilter)) continue
    const display = displayedWorkspaceId(folded, owner)
    const members = hinted.get(display)
    if (members === undefined) hinted.set(display, [summary])
    else members.push(summary)
  }
  for (const workspace of workspaces) {
    if (displayedWorkspaceId(folded, workspace.workspaceId) !== workspace.workspaceId) {
      for (const id of workspace.sessionIds) accounted.add(id)
      continue
    }
    const members: SessionSummary[] = []
    for (const id of workspace.sessionIds) {
      const summary = list.byId[id]
      if (summary === undefined) continue // account may lead the list pull; the row appears when the summary lands
      accounted.add(id)
      if (!sessionVisible(summary, current, archived, archivedFilter)) continue
      members.push(summary)
    }
    members.push(...foldedMembers.get(workspace.workspaceId) ?? [])
    // Hinted members trail the Workspace's own manual order in list recency order.
    const extra = hinted.get(workspace.workspaceId)
    if (extra !== undefined) members.push(...extra)
    // The archived-only view lists archives, not the Workspace inventory, so
    // a Workspace without archived Sessions contributes no group.
    if (archivedFilter === 'only' && members.length === 0) continue
    groups.push(buildGroup(
      workspace.workspaceId, workspace.workspaceId, workspace.path,
      Date.parse(workspace.createdAt), workspace.title, members, workspace,
    ))
  }
  const stray = list.ids
    .map(id => list.byId[id])
    .filter((s): s is SessionSummary =>
      s !== undefined && !accounted.has(s.id) && sessionVisible(s, current, archived, archivedFilter))
  if (stray.length > 0) {
    groups.push(buildGroup(
      UNGROUPED_KEY,
      undefined,
      undefined,
      undefined,
      '',
      orderedUngrouped(stray, ungroupedOrder, list.byId),
    ))
  }
  return groups
}

/** Keep navigation presentation independent from domain-owned interaction objects. */
function visiblePendingKind(kind: string | undefined): SessionPendingInteractionStatus | undefined {
  switch (kind) {
    case 'approval':
    case 'plan-review':
    case 'question':
      return kind
    default:
      return undefined
  }
}

function runningChildCount(list: SessionListState, parentId: SessionId, statuses: SessionStatuses): number {
  return list.projectionsBySession[parentId]?.values.subagentCatalog?.reduce(
    (count, child) => count + ((statuses.get(child.id)?.running ?? list.byId[child.id]?.running) === true ? 1 : 0),
    0,
  ) ?? 0
}

function sessionNode(
  s: SessionSummary,
  list: SessionListState,
  statuses: SessionStatuses,
  pinned: ReadonlySet<SessionId>,
  archived: ReadonlySet<SessionId>,
): SessionNode {
  const status = statuses.get(s.id)
  const pendingInteraction = visiblePendingKind(status?.pendingInteraction?.kind)
  // Rows show the checkout the Session works in; grouping stays with the header's original project.
  const root = selectSessionDirectory(list, s.id)
  return {
    id: s.id,
    title: sessionTitle(s),
    blank: s.blank,
    running: status?.running ?? s.running,
    runningSubagentCount: runningChildCount(list, s.id, statuses),
    completed: status?.completionUnread === true,
    failed: status?.completionFailed === true,
    pinned: !archived.has(s.id) && pinned.has(s.id),
    archived: archived.has(s.id),
    updatedAt: s.updatedAt,
    ...(workspaceGroupPath(s) === undefined ? {} : { groupedByHint: true }),
    ...(s.projectionValues?.workspaceGroup?.role === 'orchestrator' ? { orchestrator: true } : {}),
    ...(s.projectionValues?.workspaceGroup?.role === 'ticket' ? { ticket: true } : {}),
    ...(s.projectionValues?.workspaceGroup?.merged === true ? { merged: true } : {}),
    ...(s.projectionValues?.workspaceGroup?.outcome === undefined ? {}
      : { outcome: s.projectionValues.workspaceGroup.outcome }),
    ...(root === undefined || root === '' ? {} : { cwd: root }),
    ...(pendingInteraction === undefined ? {} : { pendingInteraction }),
  }
}

/**
 * Derive the workspace browser groups with every session as a top-level row.
 *
 * Every group shows, except that the archived-only filter drops groups
 * without visible members; sessions populate under expanded groups with
 * pinned rows leading in the selected local order. Blank sessions are
 * excluded except for the selected provisional New Session row; archived
 * sessions keep their slots and appear per the archived filter. Content
 * search lives outside this derivation (see {@link deriveSearchResults}).
 * @param list - sessions list snapshot (`mainView` retention feeds containsCurrent).
 * @param workspaces - real Workspaces in Host group order with caller-projected Session order.
 * @param rowState - registry-global pin and archive sets plus the archived filter.
 * @param statuses - unified UI status by Session.
 * @param view - local expansion arrays.
 * @returns group sections in render order.
 */
export function deriveGroups(
  list: SessionListState,
  workspaces: readonly WorkspaceView[],
  rowState: SessionRowState,
  statuses: SessionStatuses,
  view: TreeView,
): GroupNode[] {
  const archived = new Set(rowState.archivedSessionIds)
  const pinned = new Set(rowState.pinnedSessionIds)
  const expandedGroups = new Set(view.expandedGroups)
  const current = mainSessionId(list)
  const currentGroup = current === undefined
    ? undefined
    : displayGroupKey(workspaces, current, list.byId)
  const groups: GroupNode[] = []
  for (const g of groupByWorkspace(list, workspaces, archived, rowState.archivedFilter, view.ungroupedOrder)) {
    const expanded = expandedGroups.has(g.key)
    groups.push({
      key: g.key,
      workspaceId: g.workspaceId,
      cwd: g.cwd,
      createdAt: g.createdAt,
      label: g.label,
      ...g.parentId === undefined ? {} : { parentId: g.parentId },
      ...g.worktree === undefined ? {} : { worktree: g.worktree },
      sessionCount: g.sessions.length,
      expanded,
      containsCurrent: g.key === currentGroup,
      sessions: expanded
        ? sectionMembers(g.sessions, pinned, archived)
          .map(session => sessionNode(session, list, statuses, pinned, archived))
        : [],
    })
  }
  return groups
}

/**
 * Select complete flat-list membership, independently of archive visibility.
 * @param list - sessions list snapshot.
 * @returns known ordinary Session ids, including archives and only the current blank.
 */
export function sessionMemberIds(list: SessionListState): SessionId[] {
  return visibleSessionIds(list, [], 'show')
}

/**
 * Select visible flat-list members without deriving row presentation or ordering.
 * @param list - sessions list snapshot.
 * @param archivedSessionIds - registry-global archive set.
 * @param archivedFilter - archived-row visibility choice.
 * @returns known visible Session ids in list order, including ordinary forks and only the current blank.
 */
export function visibleSessionIds(
  list: SessionListState,
  archivedSessionIds: readonly SessionId[],
  archivedFilter: ArchivedFilter,
): SessionId[] {
  const archived = new Set(archivedSessionIds)
  const current = mainSessionId(list)
  return list.ids.filter((id) => {
    const s = list.byId[id]
    return s !== undefined && sessionVisible(s, current, archived, archivedFilter)
  })
}

/**
 * Derive flat rows from the browser's complete ordered Session ids, with
 * pinned rows fronted ahead of the supplied order.
 * @param list - sessions list snapshot used to select the ids.
 * @param sessionIds - complete account members in the selected order, including hidden archives.
 * @param rowState - registry-global pin and archive sets plus the archived filter.
 * @param statuses - unified UI status by Session.
 * @returns flat rows in sectioned order with current status indicators.
 */
export function deriveFlat(
  list: SessionListState,
  sessionIds: readonly SessionId[],
  rowState: SessionRowState,
  statuses: SessionStatuses,
): SessionNode[] {
  const archived = new Set(rowState.archivedSessionIds)
  const pinned = new Set(rowState.pinnedSessionIds)
  const current = mainSessionId(list)
  const members = sessionIds.flatMap((id) => {
    const session = list.byId[id]
    return session !== undefined && sessionVisible(session, current, archived, rowState.archivedFilter)
      ? [session]
      : []
  })
  return sectionMembers(members, pinned, archived)
    .map(session => sessionNode(session, list, statuses, pinned, archived))
}

/** Display label of each Session's Workspace: its first Workspace's title, else its cwd label. */
function workspaceLabels(workspaces: readonly WorkspaceView[]): (summary: SessionSummary) => string {
  const workspaceBySession = new Map<SessionId, string>()
  const workspaceByPath = new Map(workspaces.map(workspace => [workspace.path, workspace.title]))
  for (const workspace of workspaces) {
    for (const sessionId of workspace.sessionIds) {
      if (!workspaceBySession.has(sessionId)) workspaceBySession.set(sessionId, workspace.title)
    }
  }
  return (summary) => {
    const member = workspaceBySession.get(summary.id)
    if (member !== undefined) return member
    const path = workspaceGroupPath(summary)
    const hinted = path === undefined ? undefined : workspaceByPath.get(path)
    return hinted ?? workspaceLabel(summary.cwd)
  }
}

/**
 * Search index over the Sessions sidebar search can show, newest first: title
 * and Workspace match fuzzily, the Session id as a substring. Blank placeholders
 * are left out because their canonical title displays localized.
 * @param list - session metadata authority.
 * @param workspaces - Workspace membership and display labels.
 * @param archivedSessionIds - registry-global archive set (members match per the archived filter).
 * @param archivedFilter - archived-row visibility choice; search follows it.
 * @param previous - the index of an earlier list; its prepared titles, labels, and ids are reused.
 * @returns the index {@link deriveSearchResults} searches.
 */
export function sessionSearchIndex(
  list: SessionListState,
  workspaces: readonly WorkspaceView[],
  archivedSessionIds: readonly SessionId[],
  archivedFilter: ArchivedFilter,
  previous?: TextSearchIndex<SessionSummary>,
): TextSearchIndex<SessionSummary> {
  const archived = new Set(archivedSessionIds)
  const current = mainSessionId(list)
  const labelOf = workspaceLabels(workspaces)
  const visible = list.ids.filter((id) => {
    const summary = list.byId[id]
    return summary !== undefined && !summary.blank && sessionVisible(summary, current, archived, archivedFilter)
  })
  const sessions = orderByRecency(visible, list.byId).map(id => list.byId[id] as SessionSummary)
  const keys = (summary: SessionSummary): SearchKeys => ({ fuzzy: [sessionTitle(summary), labelOf(summary)], substring: [summary.id] })
  return previous === undefined ? new TextSearchIndex(sessions, keys) : previous.update(sessions, keys)
}

/**
 * Merge immediate title/Workspace/Session id fuzzy matches with ranked Host content
 * matches. Local rows lead best match first, newest first among equal matches;
 * content-only rows retain backend order, and duplicate sessions receive the
 * backend snippet in place.
 * @param list - session metadata authority.
 * @param workspaces - Workspace membership and display labels.
 * @param query - caller text; surrounding whitespace is ignored.
 * @param archivedSessionIds - registry-global archive set (members match per the archived filter).
 * @param archivedFilter - archived-row visibility choice; search follows it.
 * @param statuses - unified UI status by Session.
 * @param content - ranked Host content-search page.
 * @param limit - protocol-owned maximum merged row count.
 * @param index - {@link sessionSearchIndex} of the same list, Workspaces, and archive choice.
 * @returns bounded deduplicated flat rows and a refine-query hint bit.
 */
export function deriveSearchResults(
  list: SessionListState,
  workspaces: readonly WorkspaceView[],
  query: string,
  archivedSessionIds: readonly SessionId[],
  archivedFilter: ArchivedFilter,
  statuses: SessionStatuses,
  content: { items: readonly SessionSearchResultItem[]; hasMore: boolean },
  limit: number,
  index: TextSearchIndex<SessionSummary>,
): SearchResultSet {
  if (query.trim() === '') return { items: [], hasMore: false }
  const archived = new Set(archivedSessionIds)
  const current = mainSessionId(list)
  const labelOf = workspaceLabels(workspaces)
  const contentBySession = new Map<SessionId, SessionSearchResultItem>()
  for (const item of content.items) {
    if (!contentBySession.has(item.sessionId)) contentBySession.set(item.sessionId, item)
  }

  const orderedLocal = index.search(query)

  const ordered: SessionSummary[] = []
  const included = new Set<SessionId>()
  const include = (summary: SessionSummary): void => {
    if (included.has(summary.id)) return
    included.add(summary.id)
    ordered.push(summary)
  }
  for (const summary of orderedLocal) include(summary)
  for (const item of content.items) {
    const summary = list.byId[item.sessionId]
    if (summary !== undefined && !summary.blank && sessionVisible(summary, current, archived, archivedFilter)) include(summary)
  }

  return {
    items: ordered.slice(0, limit).map((summary) => {
      const match = contentBySession.get(summary.id)
      const status = statuses.get(summary.id)
      const pendingInteraction = visiblePendingKind(status?.pendingInteraction?.kind)
      return {
        id: summary.id,
        title: sessionTitle(summary),
        workspace: labelOf(summary),
        running: status?.running ?? summary.running,
        runningSubagentCount: runningChildCount(list, summary.id, statuses),
        ...(pendingInteraction === undefined
          ? {}
          : { pendingInteraction }),
        completed: status?.completionUnread === true,
        failed: status?.completionFailed === true,
        archived: archived.has(summary.id),
        ...match === undefined ? {} : { snippet: match.snippet },
      }
    }),
    hasMore: content.hasMore || ordered.length > limit,
  }
}

/** Normalize separators for comparison without interpreting POSIX backslashes as separators. */
function folderPath(path: string): string {
  const windows = /^[A-Za-z]:[/\\]/.test(path) || path.startsWith('\\\\')
  return (windows ? path.replaceAll('\\', '/') : path).replace(/\/+$/, '')
}

/**
 * Find the nearest registered ancestor, excluding the Workspace directory itself.
 * Paths use Host spelling; matching is case-sensitive, like Workspace identity.
 * @param path - Workspace directory.
 * @param parents - registered Workspace directory paths.
 * @returns the owning parent path, or undefined when no parent contains the Workspace.
 */
export function owningParentFolder(path: string, parents: readonly string[]): string | undefined {
  const child = folderPath(path)
  let owner: string | undefined
  let length = -1
  for (const parent of parents) {
    const root = folderPath(parent)
    if (root.length > length && child !== root && child.startsWith(`${root}/`)) {
      owner = parent
      length = root.length
    }
  }
  return owner
}

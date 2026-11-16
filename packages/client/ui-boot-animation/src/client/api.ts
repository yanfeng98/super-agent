/**
 * Host requests for the browser face.
 *
 * Every answer crosses the HTTP boundary, so each one is validated before it
 * reaches a component: a stale Host or an unexpected route must degrade to "no
 * clip" rather than render undefined. Requests take the plugin's abort signal so
 * unloading the plugin cancels them.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/api
 */
import type { ActiveReason, CatalogResponse, ClipInfo, SessionIntro, StartupStage } from '../types.ts'

/** Route prefix owned by this plugin. */
const BASE = '/dsh-boot-animation'

/**
 * The resource URL for exactly one clip at exactly one content version.
 * @param clip - the clip identity and its content version.
 * @returns the absolute path the video element loads.
 */
export function mediaUrlFor(clip: { readonly id: string; readonly version: string }): string {
  return `${BASE}/media/${encodeURIComponent(clip.id)}?v=${encodeURIComponent(clip.version)}`
}

/** Append a session query parameter when the conversation is known. */
function withSession(path: string, sessionId: string | null): string {
  return sessionId === null ? path : `${path}${path.includes('?') ? '&' : '?'}session=${encodeURIComponent(sessionId)}`
}

/** Narrow an unknown wire value to a clip record. */
function asClip(value: unknown): ClipInfo | null {
  if (typeof value !== 'object' || value === null) return null
  const record = value as Record<string, unknown>
  const { id, kind, bytes, version } = record
  if (typeof id !== 'string' || id === '') return null
  if (kind !== 'builtin' && kind !== 'file') return null
  if (typeof bytes !== 'number' || !Number.isFinite(bytes)) return null
  if (typeof version !== 'string' || version === '') return null
  const name = record.name
  return {
    id,
    kind,
    bytes,
    version,
    name: typeof name === 'string' ? name : null,
  }
}

/** Narrow an unknown wire value to a conversation override. */
function asSessionIntro(value: unknown): SessionIntro {
  if (typeof value !== 'object' || value === null) return { replay: false, clipId: null }
  const record = value as Record<string, unknown>
  return {
    replay: record.replay === true,
    clipId: typeof record.clipId === 'string' && record.clipId !== '' ? record.clipId : null,
  }
}

/**
 * Read the clip library and the asking conversation's override.
 * @param sessionId - the conversation, or null.
 * @param signal - cancels the request when the plugin unloads.
 * @returns the catalog; a failed request rejects.
 */
export async function fetchCatalog(sessionId: string | null, signal: AbortSignal): Promise<CatalogResponse> {
  const response = await fetch(withSession(`${BASE}/catalog.json`, sessionId), { cache: 'no-store', signal })
  if (!response.ok) throw new Error(`catalog request failed with ${String(response.status)}`)
  const body: unknown = await response.json()
  if (typeof body !== 'object' || body === null) throw new Error('catalog response is not an object')
  const record = body as Record<string, unknown>
  const clips = Array.isArray(record.clips)
    ? record.clips.map(asClip).filter((clip): clip is ClipInfo => clip !== null)
    : []
  return { clips, session: asSessionIntro(record.session), overridesAvailable: record.overridesAvailable === true }
}

/** The stages this conversation plays, and the clip the video stage uses when it runs. */
export interface ActivePlan {
  readonly stages: readonly StartupStage[]
  readonly clipId: string | null
  readonly version: string | null
  readonly how: ActiveReason
}

/**
 * Ask the Host which stages play now, and which clip the video stage uses.
 * @param sessionId - the conversation, or null.
 * @param mode - `random` forces a random pick.
 * @param signal - cancels the request when the plugin unloads.
 * @returns the plan, or null when the Host named no stage at all.
 */
export async function fetchActive(
  sessionId: string | null,
  mode: 'active' | 'random',
  signal: AbortSignal,
): Promise<ActivePlan | null> {
  const response = await fetch(withSession(`${BASE}/active.json?mode=${mode}`, sessionId), { cache: 'no-store', signal })
  if (!response.ok) throw new Error(`active request failed with ${String(response.status)}`)
  const body: unknown = await response.json()
  if (typeof body !== 'object' || body === null) return null
  const record = body as Record<string, unknown>
  const stages = Array.isArray(record.stages)
    ? record.stages.filter((stage): stage is StartupStage => stage === 'archive' || stage === 'video')
    : ['video' as const]
  if (stages.length === 0) return null
  const clipId = typeof record.clipId === 'string' && record.clipId !== '' ? record.clipId : null
  const version = typeof record.version === 'string' && record.version !== '' ? record.version : null
  const how = record.how
  const reason: ActiveReason = how === 'session' || how === 'random' || how === 'selected' ? how : 'none'
  return { stages, clipId, version, how: reason }
}

/**
 * The URL of one shipped sequence audio file.
 * @param name - the file name, without its extension.
 * @returns the absolute path the audio element loads.
 */
export function audioUrlFor(name: string): string {
  return `${BASE}/audio/${encodeURIComponent(name)}.mp3`
}

/**
 * Store this conversation's override.
 * @param sessionId - the conversation.
 * @param patch - fields to replace.
 * @param signal - cancels the request when the plugin unloads.
 * @returns the stored override.
 */
export async function writeSessionOverride(
  sessionId: string,
  patch: { replay?: boolean; clipId?: string | null },
  signal: AbortSignal,
): Promise<SessionIntro> {
  const response = await fetch(`${BASE}/session`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ sessionId, ...patch }),
    cache: 'no-store',
    signal,
  })
  if (!response.ok) throw new Error(`session write failed with ${String(response.status)}`)
  const body: unknown = await response.json()
  return asSessionIntro(body)
}

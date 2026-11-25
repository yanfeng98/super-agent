/** Consecutive-image grouping for the Feishu Source: keeps a run of standalone images on one Thread. */
import { KV } from '../kv.ts'
import type { DshImEvent } from './dsh-im.ts'

/** One sender's open image run in a chat: the first image's id, the sender, and the last message's time. */
export interface ImageGroup { senderId: string; rootId: string; lastAt: string }

/** The store surface the grouping helper needs; the Source context and the backfill context both provide `state`. */
export interface ImageGroupState {
  get(key: ReturnType<typeof KV.feishu.imageGroups>): Record<string, ImageGroup> | undefined
  set(key: ReturnType<typeof KV.feishu.imageGroups>, value: Record<string, ImageGroup>): void
}

/**
 * Whether a message is a standalone image Feishu left in the chat container: dsh-im rendered an image body and Feishu
 * reported neither a Topic id nor a root, so `imTrigger` would otherwise key a Thread on the message itself.
 */
export const isStandaloneImage = (event: DshImEvent): boolean =>
  event.messageType === 'image' && !event.reply.threadId && !event.reply.rootId

/**
 * Returns the message id that should root an image's Thread, or `undefined` for a message that is not part of an image run.
 *
 * A run starts with a standalone image and continues while the same sender sends standalone images within `windowMs` of
 * the previous one. Any other message by that sender ends the run, so a later image does not join a discussion that came
 * between them. The returned id is only the Thread key: the reply route stays exactly as dsh-im reported it.
 *
 * @param state - The Source's key-value state, holding one run map per chat.
 * @param event - The authenticated message, in arrival order.
 * @param windowMs - Longest gap between two images of one run, in milliseconds.
 * @returns The first image's message id while the run continues; `undefined` when no grouping applies.
 */
export function imageGroupRoot(state: ImageGroupState, event: DshImEvent, windowMs: number): string | undefined {
  if (!isStandaloneImage(event)) return undefined
  const key = KV.feishu.imageGroups(event.conversation.id)
  const groups: Record<string, ImageGroup> = { ...state.get(key) }
  const existing = groups[event.actor.id]
  const within = existing && Date.parse(event.at) - Date.parse(existing.lastAt) <= windowMs
    && Date.parse(event.at) >= Date.parse(existing.lastAt)
  if (!within) {
    groups[event.actor.id] = { senderId: event.actor.id, rootId: event.messageId, lastAt: event.at }
    state.set(key, groups)
    return event.messageId
  }
  groups[event.actor.id] = { ...existing, lastAt: event.at }
  state.set(key, groups)
  return existing.rootId
}

/**
 * Ends the sender's image run when a message that is not a standalone image arrives, so a later image starts a new Thread.
 * A standalone image leaves its own run open; callers may pass every message in arrival order.
 *
 * @param state - The Source's key-value state.
 * @param event - The authenticated message that broke the run.
 */
export function endImageGroup(state: ImageGroupState, event: DshImEvent): void {
  if (isStandaloneImage(event)) return
  const key = KV.feishu.imageGroups(event.conversation.id)
  const groups = state.get(key)
  if (!groups?.[event.actor.id]) return
  const next = { ...groups }
  delete next[event.actor.id]
  state.set(key, next)
}

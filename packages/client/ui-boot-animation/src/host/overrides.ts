/**
 * Per-conversation intro overrides, stored in a durable Host domain.
 *
 * `$DSH_HOME` storage is the right home for this: the choice must survive a
 * restart, must be the same in every browser and on every port, and must not
 * become a Session event (it changes nothing the model sees). The domain form
 * gives that durability with schema validation and one owner, instead of a
 * hand-written JSON file this package would have to repair itself.
 *
 * A record is a conversation's answer to two independent questions: play the
 * intro on every entry (`replay`), and which clip to play (`clipId`). Both are
 * optional so one can be set without the other.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/host/overrides
 */
import { defineDomain, domainTable } from '@deepseek-ai/dsh-storage-domain'
import type { Domain } from '@deepseek-ai/dsh-storage-domain'
import { z } from 'zod'
import type { SessionIntro } from '../types.ts'

/** Stored record; absent fields keep their defaults. */
const sessionIntroSchema = z.object({
  replay: z.boolean().default(false),
  clipId: z.string().min(1).nullable().default(null),
}).strict()

/** This package's durable domain. */
export const bootAnimationDomain = defineDomain({
  name: 'boot_animation',
  version: 1,
  tables: { sessions: domainTable<string, z.infer<typeof sessionIntroSchema>>(sessionIntroSchema) },
})

/** The override with nothing set. */
export const NO_OVERRIDE: SessionIntro = { replay: false, clipId: null }

/** Longest session id accepted from the wire. */
const MAX_SESSION_ID_LENGTH = 200

/**
 * Validate a session id arriving as a query parameter or JSON field.
 * @param value - the raw value.
 * @returns the trimmed id, or null when it cannot name a conversation.
 */
export function normalizeSessionId(value: unknown): string | null {
  if (typeof value !== 'string') return null
  const trimmed = value.trim()
  if (trimmed === '' || trimmed.length > MAX_SESSION_ID_LENGTH) return null
  if (/[\u0000-\u001f\u007f]/.test(trimmed)) return null
  return trimmed
}

/** Reads and writes one conversation's override. */
export class SessionOverrides {
  /** @param domain - the open domain this plugin owns. */
  constructor(private readonly domain: Domain<typeof bootAnimationDomain>) {}

  /**
   * The override stored for one conversation.
   * @param sessionId - a validated session id, or null for the global question.
   * @returns the stored override, or {@link NO_OVERRIDE}.
   */
  read(sessionId: string | null): SessionIntro {
    if (sessionId === null) return NO_OVERRIDE
    const record = this.domain.table('sessions').get(sessionId)
    if (record === undefined) return NO_OVERRIDE
    return { replay: record.replay, clipId: record.clipId }
  }

  /**
   * Merge a patch into one conversation's override.
   * @param sessionId - the conversation to change.
   * @param patch - fields to replace; omitted fields keep their stored value.
   * @returns the stored override after the write.
   */
  async write(sessionId: string, patch: Partial<SessionIntro>): Promise<SessionIntro> {
    const current = this.read(sessionId)
    const next: SessionIntro = {
      replay: patch.replay ?? current.replay,
      clipId: patch.clipId === undefined ? current.clipId : patch.clipId,
    }
    if (!next.replay && next.clipId === null) {
      await this.domain.table('sessions').delete(sessionId)
      return NO_OVERRIDE
    }
    await this.domain.table('sessions').put(sessionId, { replay: next.replay, clipId: next.clipId })
    return next
  }
}

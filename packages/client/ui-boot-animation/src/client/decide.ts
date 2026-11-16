/**
 * When the intro plays, as one pure function, and the launch record it reads.
 *
 * The rule is data-in, answer-out so it can be exercised over its whole matrix
 * without rendering anything. Its inputs are the two settings, the conversation
 * identity, and two durable records: whether this launch already played, and
 * whether this conversation already did.
 *
 * A conversation override that says "play on every entry" outranks both records
 * - that is what the user asked for - and every other path records its play, so
 * one launch cannot produce two intros by accident.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/decide
 */
import type { PlayTiming } from '../types.ts'
import type { IntroReason } from './contract.ts'

/** Everything the rule reads. */
export interface DecideInput {
  /** The user's master switch. */
  readonly enabled: boolean
  readonly timing: PlayTiming
  /** The conversation the main view shows, or null before one is known. */
  readonly sessionId: string | null
  /** The conversation still has no turns. */
  readonly blank: boolean
  /** The view entered this conversation on this evaluation. */
  readonly entered: boolean
  /** This launch already played its intro. */
  readonly playedThisLaunch: boolean
  /** This conversation's override asks for a replay on every entry. */
  readonly replay: boolean
  /** This conversation already played during this launch. */
  readonly playedThisSession: boolean
}

/** The answer: play for a stated reason, or stay quiet for a stated reason. */
export type Decision =
  | { readonly action: 'play'; readonly reason: IntroReason }
  | { readonly action: 'none'; readonly because: 'disabled' | 'played' | 'not-now' }

/**
 * Decide whether to play now.
 * @param input - settings, conversation identity, and durable records.
 * @returns the decision and its reason.
 */
export function decidePlay(input: DecideInput): Decision {
  if (!input.enabled) return { action: 'none', because: 'disabled' }
  const hasSession = input.sessionId !== null
  // 1. The conversation asked for a replay on every entry.
  if (hasSession && input.replay && input.entered) return { action: 'play', reason: 'session' }
  // 2. Once per launch: the launch is the scope, so no conversation is required.
  if (input.timing === 'app-start') {
    return input.playedThisLaunch
      ? { action: 'none', because: 'played' }
      : { action: 'play', reason: 'app-start' }
  }
  // 3. The legacy rule: a new conversation's first entry.
  if (!hasSession || !input.blank || !input.entered) return { action: 'none', because: 'not-now' }
  return input.playedThisSession
    ? { action: 'none', because: 'played' }
    : { action: 'play', reason: 'entry' }
}

/** Session-scoped storage, or nothing when the browser refuses it. */
type MaybeStorage = Pick<Storage, 'getItem' | 'setItem'> | null

/** Key holding this launch's marker. */
const LAUNCH_KEY = 'dsh-boot-animation:launch'

/** Key prefix for the per-conversation markers. */
const SESSION_PREFIX = 'dsh-boot-animation:session:'

/**
 * Durable-enough records for one page lifetime.
 *
 * Storage can be missing or throw (private mode, a blocked third-party context,
 * quota). Both directions matter: a read that silently answers "played" costs
 * the whole launch its intro, and a write that silently fails would replay it on
 * every evaluation. The memory flags are the fallback for both.
 */
export class IntroRecords {
  private launchInMemory = false
  private readonly sessionsInMemory = new Set<string>()

  /** @param storage - session-scoped storage, or null. */
  constructor(private readonly storage: MaybeStorage) {}

  /**
   * Whether this launch already played.
   * @returns true when the launch record exists.
   */
  playedLaunch(): boolean {
    return this.read(LAUNCH_KEY) === '1' || this.launchInMemory
  }

  /** Record that this launch played. */
  markLaunch(): void {
    this.launchInMemory = true
    this.write(LAUNCH_KEY, '1')
  }

  /**
   * Whether this conversation already played during this launch.
   * @param sessionId - the conversation.
   * @returns true when this conversation's record exists.
   */
  playedSession(sessionId: string): boolean {
    return this.read(SESSION_PREFIX + sessionId) === '1' || this.sessionsInMemory.has(sessionId)
  }

  /**
   * Record that this conversation played.
   * @param sessionId - the conversation.
   */
  markSession(sessionId: string): void {
    this.sessionsInMemory.add(sessionId)
    this.write(SESSION_PREFIX + sessionId, '1')
  }

  private read(key: string): string | null {
    try {
      return this.storage?.getItem(key) ?? null
    } catch (error) {
      // A storage that refuses the read is the same as an absent value.
      void error
      return null
    }
  }

  private write(key: string, value: string): void {
    try {
      this.storage?.setItem(key, value)
    } catch (error) {
      // The memory flag already carries the decision for this launch.
      void error
      return
    }
  }
}

/**
 * Bind the browser's session storage, or null when the property itself throws.
 * @returns a usable storage handle, or null.
 */
export function sessionStorageOrNull(): MaybeStorage {
  try {
    return globalThis.sessionStorage
  } catch (error) {
    // A throwing storage getter means there is no usable storage here.
    void error
    return null
  }
}

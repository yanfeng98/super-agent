/**
 * Typed key-value store keys and key families grouped by the single component that owns and writes each family.
 * Readers and writers refer to a record by its entry here so each key is spelled once with its value type.
 */
import type { QueuedMerge } from './app/merge-queue.ts'
import type { PromptVersion } from './app/prompts.ts'
import type { SettingsVersion } from './app/store.ts'
import type { SettingsSave } from './policy.ts'

/** Key of a KV record whose value has type `V`; a plain string represents an untyped key. */
export type Key<V> = string & { readonly __value?: V }
/** Family of KV records sharing a prefix: callable to build a key from its name parts, and exposing `prefix` for listing. */
export type Family<V, A extends unknown[]> = ((...name: A) => Key<V>) & { readonly prefix: string }
/** Define a single typed KV key; external Source plugins use `record` and `family` to type their own keys. */
export const record = <V>(key: string) => key as Key<V>
/** Define a KV key family whose keys join `prefix` and name parts with `:` (or a custom `format`). */
export function family<V, A extends unknown[] = [string]>(prefix: string, format = (...name: A) => name.join(':')): Family<V, A> {
  return Object.assign((...name: A) => `${prefix}${format(...name)}` as Key<V>, { prefix })
}

export const KV = {
  /** Store: current database schema version. */
  store: { schema: record<number>('schema') },
  /** Settings: current revision, per-block revision, and saved settings snapshots (including the current one). */
  settings: { revision: record<number>('settings-revision'),
    block: family<number, [block: SettingsSave]>('settings-block-revision:'),
    version: family<SettingsVersion, [revision: number]>('settings-version:') },
  /** Prompts: current revision and saved prompt versions. */
  prompts: { revision: record<number>('prompt-revision'), version: family<PromptVersion, [revision: number]>('prompt-version:'),
    /** The built-in text of a prompt part an override was edited from, by its `base`, which shows what an upgrade changed. */
    builtIn: family<string, [base: string]>('prompt-built-in:') },
  /** Runtime: the main Session's ID, and the Tickets waiting for their turn in the merge queue, in order. */
  runtime: { masterId: record<string>('master-id'), mergeQueue: record<QueuedMerge[]>('merge-queue'),
    mergeQueuePaused: record<boolean>('merge-queue-paused') },
  /** Sessions: prompt revisions each Session has worked under, ordered oldest first; the last entry (`.at(-1)`) is its active revision. */
  sessions: { prompts: family<number[], [session: string]>('session-prompts:') },
  /** Per-Source resolved display names mapping a named ID (such as a Feishu chat ID) to its display name. */
  sources: { labels: family<Record<string, string>, [source: string]>('source-labels:') },
  /** Feishu Source: known chats (`true` for direct messages) and the per-chat history cursor. */
  feishu: { chats: record<Record<string, boolean>>('feishu-chats'), history: family<string, [chat: string]>('feishu-history:'),
    /**
     * Topics the dsh-im Source recovers: a topic's last activity, the message that roots it, and the time up to which it
     * has been read.
     */
    imTopics: family<Record<string, { at: string; rootId: string; readThrough?: string }>, [chat: string]>('feishu-im-topics:'),
    /** Per-polled-thread topic lookup: the Thread activity it was looked up at, and the topic when Feishu reports one. */
    topic: family<{ at: string; threadId?: string }, [thread: string]>('thread-topic:') },
}

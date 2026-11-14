/** Per-folder browser persistence for accepted composer input. */

/** Newest accepted composer submissions retained per folder. */
export const INPUT_HISTORY_LIMIT = 100

const STORAGE_PREFIX = 'dsh.conversation.input-history.v1.'

const EMPTY_HISTORY: readonly string[] = []

/** Composer history direction used by the keymap and the input shell. */
export type HistoryDirection = 'older' | 'newer'

/**
 * Browser storage key for one folder's composer input history.
 * @param folder - canonical working directory of the Session.
 * @returns the localStorage key for that folder.
 */
export function inputHistoryStorageKey(folder: string): string {
  return STORAGE_PREFIX + folder
}

/**
 * Decode one stored history value.
 * @param raw - localStorage value, or null when the folder has no stored history.
 * @returns the nonblank string entries, or an empty list for unusable data.
 */
function decodeStoredHistory(raw: string | null): readonly string[] {
  if (raw === null) return EMPTY_HISTORY
  const parsed: unknown = JSON.parse(raw)
  if (!Array.isArray(parsed)) return EMPTY_HISTORY
  return parsed
    .filter((value): value is string => typeof value === 'string' && value.trim() !== '')
    .slice(-INPUT_HISTORY_LIMIT)
}

/**
 * Read one folder's retained submissions, oldest first.
 * @param folder - canonical working directory of the Session.
 * @returns the folder's entries bounded to the newest limit, or an empty list.
 */
export function readInputHistory(folder: string): readonly string[] {
  if (typeof localStorage === 'undefined') return EMPTY_HISTORY
  try {
    return decodeStoredHistory(localStorage.getItem(inputHistoryStorageKey(folder)))
  } catch (_error: unknown) {
    // A disabled or full browser store, or malformed JSON, has no usable history.
    return EMPTY_HISTORY
  }
}

/**
 * Append one accepted submission to its folder's history, keeping the newest
 * bounded entries. Blank submissions and an exact repeat of the newest entry
 * are not stored.
 * @param folder - canonical working directory of the Session.
 * @param text - clipboard text the submit plane accepted.
 */
export function appendInputHistory(folder: string, text: string): void {
  if (text.trim() === '' || typeof localStorage === 'undefined') return
  const entries = readInputHistory(folder)
  if (entries.at(-1) === text) return
  const next = [...entries, text].slice(-INPUT_HISTORY_LIMIT)
  try {
    localStorage.setItem(inputHistoryStorageKey(folder), JSON.stringify(next))
  } catch (_error: unknown) {
    // A private or full browser store must not block the composer.
  }
}

/**
 * Whether a collapsed caret may leave the draft for history in one direction.
 * Up leaves from the first line and Down from the last, so arrows inside a
 * recalled multi-line entry still move the caret; a single-line entry always
 * navigates.
 * @param text - current draft text.
 * @param caret - collapsed caret offset in that text.
 * @param direction - history direction the key requests.
 * @returns whether the caret sits on the line that navigation leaves.
 */
export function atHistoryBoundary(text: string, caret: number, direction: HistoryDirection): boolean {
  if (direction === 'older') {
    const firstBreak = text.indexOf('\n')
    return caret <= (firstBreak === -1 ? text.length : firstBreak)
  }
  const lastBreak = text.lastIndexOf('\n')
  return caret >= (lastBreak === -1 ? 0 : lastBreak + 1)
}

/** Folder-scoped input history one Session shell reads and records. */
export interface ComposerHistorySource {
  /**
   * Resolve the Session's folder on demand.
   * @returns the catalogued working directory, or undefined while it is unknown.
   */
  folder(): string | undefined
  /**
   * @param folder - canonical working directory of the Session.
   * @returns that folder's retained entries, oldest first.
   */
  entries(folder: string): readonly string[]
  /**
   * @param folder - canonical working directory of the Session.
   * @param text - clipboard text the submit plane accepted.
   */
  record(folder: string, text: string): void
}

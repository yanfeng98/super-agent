export interface Entry {
  readonly key: string
  readonly value: string | null
}

/** Normalizes entries after an await, the way a loader would. */
export async function normalize(entries: readonly Entry[], prefix?: string): Promise<Entry[]> {
  await null
  return entries
    .filter(entry => entry.value !== null)
    .map(entry => ({ key: `${prefix ?? ''}${entry.key.trim()}`, value: entry.value!.trim() }))
}

export async function countWords(text: string): Promise<number> {
  await Promise.resolve()
  return text.split(/\s+/).filter(Boolean).length
}

export function upper(text: string): string {
  return text.toUpperCase()
}

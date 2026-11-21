export interface Stats {
  readonly words: number
  readonly lines: number
}

/** Counts words and lines of the text it is given, and remembers the longest word. */
export class TextStats {
  private words = 0
  private lines = 0
  private longest = ''

  constructor(private readonly separator: string = '\n') {}

  add(text: string): number {
    const parts = text.split(this.separator)
    this.lines += parts.length
    for (const part of parts) {
      const words = part.split(/\s+/).filter(Boolean)
      this.words += words.length
      for (const word of words) {
        if (word.length > this.longest.length)
          this.longest = word
      }
    }
    return this.words
  }

  stats(): Stats {
    return { words: this.words, lines: this.lines }
  }

  longestWord(): string {
    return this.longest
  }

  reset(): void {
    this.words = 0
    this.lines = 0
    this.longest = ''
  }
}

/** A tiny least-recently-used cache. */
export class LruCache {
  #map = new Map<string, string>()

  constructor(private readonly capacity: number) {
    if (!(capacity >= 1))
      throw new RangeError('capacity must be at least 1')
  }

  get(key: string): string | undefined {
    const value = this.#map.get(key)
    if (value !== undefined) {
      this.#map.delete(key)
      this.#map.set(key, value)
    }
    return value
  }

  set(key: string, value: string): string[] {
    this.#map.delete(key)
    this.#map.set(key, value)
    const evicted: string[] = []
    while (this.#map.size > this.capacity) {
      const oldest = this.#map.keys().next().value!
      this.#map.delete(oldest)
      evicted.push(oldest)
    }
    return evicted
  }

  keys(): string[] {
    return [...this.#map.keys()]
  }
}

export class Blocked extends TextStats {}

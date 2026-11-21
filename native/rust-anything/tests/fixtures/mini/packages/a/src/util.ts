const after = /(?<=\$)\w+/g

export function rangeOf(text: string, word: string) {
  const start = text.indexOf(word)
  return { start, end: start + word.length, rest: text.slice(start) }
}

export function words(text: string): string[] {
  const out: string[] = []
  for (const match of text.matchAll(after))
    out.push(match[0])
  return out
}

export function count(text: string): number {
  let n = 0
  for (let i = 0; i < text.length; i++) {
    if (text[i] === ' ')
      n++
  }
  return n
}

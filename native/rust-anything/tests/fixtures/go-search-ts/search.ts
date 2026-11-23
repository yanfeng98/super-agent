export function score(query: string, text: string): number {
  const q = query.toLowerCase()
  const t = text.toLowerCase()
  let at = 0
  let total = 0
  let streak = 0
  for (const ch of q) {
    const found = t.indexOf(ch, at)
    if (found < 0)
      return 0
    streak = found === at ? streak + 1 : 1
    total += streak
    at = found + 1
  }
  return total / (t.length + 1)
}

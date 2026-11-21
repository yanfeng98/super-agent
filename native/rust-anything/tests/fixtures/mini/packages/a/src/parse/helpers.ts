export function flatten(value: unknown, prefix = '', out = new Map<string, string>()): Map<string, string> {
  if (typeof value === 'object' && value !== null) {
    for (const [k, v] of Object.entries(value))
      flatten(v, prefix ? `${prefix}.${k}` : k, out)
  }
  else {
    out.set(prefix, String(value))
  }
  return out
}

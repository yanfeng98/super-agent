/**
 * Resolve the application root context without Cordis's service proxy.
 *
 * Cordis exposes the root through `Symbol.for('cordis.root')`; reading the
 * plain `ctx.root` property is rejected on a fiber context that does not
 * inject a service named "root". Plain test contexts expose `root` directly,
 * so that legacy field remains a fallback after the symbol lookup.
 */
const ROOT = Symbol.for('cordis.root');

export function hostScope(ctx) {
  if (!ctx) return ctx;
  try {
    const root = ctx[ROOT];
    if (root !== undefined) return root;
  } catch {
    // A context proxy may reject symbol access; fall through to the legacy field.
  }
  try {
    return ctx.root ?? ctx;
  } catch {
    return ctx;
  }
}

/**
 * The smallest observable the renderer can bind to a `use<Name>` hook.
 *
 * The hooks compartment takes bare observables; these are registrant-private
 * facts, so they are created in `apply` and never shared as module state. A
 * source keeps its snapshot identity until the value actually changes, which is
 * what the renderer's selector cache requires.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/source
 */
import type { HostObservable } from '@deepseek-ai/dsh-client-ui-slots'

/** A readable value plus its writer, owned by the plugin body. */
export interface Source<T> extends HostObservable<T> {
  /** Publish a new value; listeners run synchronously. */
  set: (value: T) => void
}

/**
 * Create a writable observable.
 * @param initial - the first snapshot.
 * @returns the source.
 */
export function createSource<T>(initial: T): Source<T> {
  let current = initial
  const listeners = new Set<() => void>()
  return {
    getSnapshot: () => current,
    subscribe: (listener) => {
      listeners.add(listener)
      return () => { listeners.delete(listener) }
    },
    set: (value) => {
      if (Object.is(value, current)) return
      current = value
      for (const listener of [...listeners]) {
        try {
          listener()
        } catch (error) {
          // One throwing consumer must not starve the others.
          void error
        }
      }
    },
  }
}

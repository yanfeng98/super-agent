/**
 * The registrant-private observable: snapshot identity, deduplication, one
 * throwing listener contained, and unsubscribe.
 */
import { describe, expect, it, vi } from 'vitest'
import { createSource } from '../src/client/source.ts'

describe('createSource', () => {
  it('hands out its initial snapshot until a new value is published', () => {
    const source = createSource({ count: 1 })
    const initial = source.getSnapshot()
    expect(initial).toEqual({ count: 1 })
    expect(source.getSnapshot()).toBe(initial)

    const next = { count: 2 }
    source.set(next)
    expect(source.getSnapshot()).toBe(next)
  })

  it('does not notify for a value the current snapshot is already identical to', () => {
    const source = createSource(1)
    const listener = vi.fn()
    source.subscribe(listener)
    source.set(1)
    expect(listener).not.toHaveBeenCalled()

    // Object.is identity, not equality: NaN is the same value as itself.
    source.set(Number.NaN)
    expect(listener).toHaveBeenCalledTimes(1)
    source.set(Number.NaN)
    expect(listener).toHaveBeenCalledTimes(1)

    // ...and -0 is a different value from 0.
    source.set(0)
    source.set(-0)
    expect(listener).toHaveBeenCalledTimes(3)
  })

  it('keeps notifying the remaining listeners when one throws', () => {
    const source = createSource('idle')
    const boom = vi.fn(() => { throw new Error('consumer failed') })
    const after = vi.fn()
    source.subscribe(boom)
    source.subscribe(after)

    expect(() => { source.set('playing') }).not.toThrow()
    expect(boom).toHaveBeenCalledOnce()
    expect(after).toHaveBeenCalledOnce()
    expect(source.getSnapshot()).toBe('playing')
  })

  it('stops calling an unsubscribed listener, including during a publish', () => {
    const source = createSource(0)
    const first = vi.fn()
    const second = vi.fn()
    const unsubscribeFirst = source.subscribe(first)
    source.subscribe(second)

    unsubscribeFirst()
    source.set(1)
    expect(first).not.toHaveBeenCalled()
    expect(second).toHaveBeenCalledOnce()

    // A listener that unsubscribes itself mid-publish is gone from the next value.
    const selfRemoving = vi.fn(() => { unsubscribeSecond() })
    const unsubscribeSecond = source.subscribe(selfRemoving)
    source.set(2)
    source.set(3)
    expect(selfRemoving).toHaveBeenCalledOnce()
  })
})

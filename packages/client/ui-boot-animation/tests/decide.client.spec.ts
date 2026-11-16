/**
 * The when-to-play rule over its whole input matrix, plus the launch records
 * that make a play one-shot and the storage fallbacks those records rely on.
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { IntroRecords, decidePlay, sessionStorageOrNull } from '../src/client/decide.ts'
import type { DecideInput } from '../src/client/decide.ts'

const BASE: DecideInput = {
  enabled: true,
  timing: 'app-start',
  sessionId: null,
  blank: true,
  entered: false,
  playedThisLaunch: false,
  replay: false,
  playedThisSession: false,
}

/** One row of the matrix, with every field the row does not care about defaulted. */
function input(patch: Partial<DecideInput>): DecideInput {
  return { ...BASE, ...patch }
}

/** Session-scoped storage double over a Map. */
function memoryStorage(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial))
  return {
    getItem: vi.fn((key: string) => values.get(key) ?? null),
    setItem: vi.fn((key: string, value: string) => { values.set(key, value) }),
    values,
  }
}

const LAUNCH_KEY = 'dsh-boot-animation:launch'
const SESSION_KEY = 'dsh-boot-animation:session:'

describe('decidePlay', () => {
  it('stays quiet while the master switch is off, whatever else is true', () => {
    expect(decidePlay(input({ enabled: false }))).toEqual({ action: 'none', because: 'disabled' })
    expect(decidePlay(input({
      enabled: false,
      sessionId: 's1',
      entered: true,
      replay: true,
      blank: true,
      timing: 'entry',
      playedThisLaunch: false,
      playedThisSession: false,
    }))).toEqual({ action: 'none', because: 'disabled' })
  })

  it('plays for a conversation that asks for a replay on every entry, ahead of both records', () => {
    expect(decidePlay(input({
      sessionId: 's1',
      entered: true,
      replay: true,
      blank: false,
      timing: 'app-start',
      playedThisLaunch: true,
      playedThisSession: true,
    }))).toEqual({ action: 'play', reason: 'session' })
  })

  it('does not treat a replay request as enough without a conversation or without an entry', () => {
    expect(decidePlay(input({
      sessionId: null,
      entered: true,
      replay: true,
      timing: 'app-start',
      playedThisLaunch: false,
    }))).toEqual({ action: 'play', reason: 'app-start' })
    expect(decidePlay(input({
      sessionId: 's1',
      entered: false,
      replay: true,
      timing: 'app-start',
      playedThisLaunch: false,
    }))).toEqual({ action: 'play', reason: 'app-start' })
    expect(decidePlay(input({
      sessionId: null,
      entered: true,
      replay: true,
      timing: 'entry',
    }))).toEqual({ action: 'none', because: 'not-now' })
  })

  it('plays once per launch at app start, conversation or not', () => {
    expect(decidePlay(input({ timing: 'app-start', playedThisLaunch: false })))
      .toEqual({ action: 'play', reason: 'app-start' })
    expect(decidePlay(input({ timing: 'app-start', playedThisLaunch: true })))
      .toEqual({ action: 'none', because: 'played' })
    expect(decidePlay(input({
      timing: 'app-start', sessionId: 's1', entered: true, playedThisLaunch: false,
    }))).toEqual({ action: 'play', reason: 'app-start' })
  })

  it('waits for a new conversation that is blank and entered on entry timing', () => {
    expect(decidePlay(input({ timing: 'entry', blank: false, entered: true })))
      .toEqual({ action: 'none', because: 'not-now' })
    expect(decidePlay(input({ timing: 'entry', blank: true, entered: false })))
      .toEqual({ action: 'none', because: 'not-now' })
    expect(decidePlay(input({ timing: 'entry', sessionId: 's1', blank: true, entered: false })))
      .toEqual({ action: 'none', because: 'not-now' })
    expect(decidePlay(input({ timing: 'entry', sessionId: null, blank: true, entered: true })))
      .toEqual({ action: 'none', because: 'not-now' })
    expect(decidePlay(input({ timing: 'entry', sessionId: 's1', blank: true, entered: true })))
      .toEqual({ action: 'play', reason: 'entry' })
    expect(decidePlay(input({
      timing: 'entry', sessionId: 's1', blank: true, entered: true, playedThisSession: true,
    }))).toEqual({ action: 'none', because: 'played' })
  })
})

describe('IntroRecords', () => {
  it('reads launch and conversation markers back from storage', () => {
    const records = new IntroRecords(memoryStorage({
      [LAUNCH_KEY]: '1',
      [`${SESSION_KEY}s1`]: '1',
    }))
    expect(records.playedLaunch()).toBe(true)
    expect(records.playedSession('s1')).toBe(true)
    expect(records.playedSession('s2')).toBe(false)
  })

  it('writes both markers and a fresh reader sees them', () => {
    const storage = memoryStorage()
    const records = new IntroRecords(storage)
    expect(records.playedLaunch()).toBe(false)
    records.markLaunch()
    records.markSession('s1')
    expect(storage.values.get(LAUNCH_KEY)).toBe('1')
    expect(storage.values.get(`${SESSION_KEY}s1`)).toBe('1')
    expect(records.playedLaunch()).toBe(true)
    expect(records.playedSession('s1')).toBe(true)

    const reread = new IntroRecords(storage)
    expect(reread.playedLaunch()).toBe(true)
    expect(reread.playedSession('s1')).toBe(true)
  })

  it('answers from memory when storage refuses a write, exactly once', () => {
    const storage = {
      getItem: vi.fn(() => { throw new Error('blocked') }),
      setItem: vi.fn(() => { throw new Error('quota') }),
    }
    const records = new IntroRecords(storage)
    expect(records.playedLaunch()).toBe(false)
    expect(records.playedSession('s1')).toBe(false)

    records.markLaunch()
    records.markSession('s1')

    expect(records.playedLaunch()).toBe(true)
    expect(records.playedSession('s1')).toBe(true)
    expect(records.playedSession('s2')).toBe(false)
  })

  it('works without any storage at all', () => {
    const records = new IntroRecords(null)
    expect(records.playedLaunch()).toBe(false)
    expect(records.playedSession('s1')).toBe(false)
    records.markLaunch()
    records.markSession('s1')
    expect(records.playedLaunch()).toBe(true)
    expect(records.playedSession('s1')).toBe(true)
  })

  it('does not trust a marker other than the exact recorded value', () => {
    const records = new IntroRecords(memoryStorage({
      [LAUNCH_KEY]: '0',
      [`${SESSION_KEY}s1`]: '',
    }))
    expect(records.playedLaunch()).toBe(false)
    expect(records.playedSession('s1')).toBe(false)
  })
})

describe('sessionStorageOrNull', () => {
  const original = Object.getOwnPropertyDescriptor(globalThis, 'sessionStorage')

  afterEach(() => {
    if (original === undefined) Reflect.deleteProperty(globalThis, 'sessionStorage')
    else Object.defineProperty(globalThis, 'sessionStorage', original)
  })

  it('binds the browser storage when it is readable', () => {
    const storage = memoryStorage()
    Object.defineProperty(globalThis, 'sessionStorage', { configurable: true, get: () => storage })
    expect(sessionStorageOrNull()).toBe(storage)
  })

  it('answers null when reading the property itself throws', () => {
    Object.defineProperty(globalThis, 'sessionStorage', {
      configurable: true,
      get: () => { throw new Error('storage denied') },
    })
    expect(sessionStorageOrNull()).toBeNull()
  })
})

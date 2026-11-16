import { describe, expect, it } from 'vitest'
import { NO_OVERRIDE, normalizeSessionId } from '../src/host/overrides.ts'
import { createOverrides } from './harness.host.ts'

describe('normalizeSessionId', () => {
  it('rejects values that cannot name a conversation', () => {
    expect(normalizeSessionId(null)).toBeNull()
    expect(normalizeSessionId(42)).toBeNull()
    expect(normalizeSessionId(undefined)).toBeNull()
    expect(normalizeSessionId('')).toBeNull()
    expect(normalizeSessionId('   ')).toBeNull()
    expect(normalizeSessionId(` ${'a'.repeat(201)} `)).toBeNull()
    expect(normalizeSessionId('s\u0000ession')).toBeNull()
    expect(normalizeSessionId('s\nession')).toBeNull()
    expect(normalizeSessionId('s\u007fession')).toBeNull()
  })

  it('returns the trimmed id for a valid conversation id', () => {
    expect(normalizeSessionId('  session-1  ')).toBe('session-1')
    expect(normalizeSessionId('a'.repeat(200))).toBe('a'.repeat(200))
  })
})

describe('SessionOverrides', () => {
  it('returns the empty override for the global question and unknown conversations', () => {
    const { overrides } = createOverrides()
    expect(overrides.read(null)).toBe(NO_OVERRIDE)
    expect(overrides.read('missing')).toBe(NO_OVERRIDE)
  })

  it('reads a stored conversation override', () => {
    const { overrides, domain } = createOverrides(new Map([
      ['s1', { replay: true, clipId: 'builtin:brand' }],
    ]))
    expect(overrides.read('s1')).toEqual({ replay: true, clipId: 'builtin:brand' })
    expect(domain.get).toHaveBeenCalledWith('s1')
  })

  it('merges a patch into the stored override', async () => {
    const { overrides, domain } = createOverrides(new Map([
      ['s1', { replay: true, clipId: 'builtin:brand' }],
    ]))

    expect(await overrides.write('s1', { clipId: 'file:own-00000000' })).toEqual({
      replay: true,
      clipId: 'file:own-00000000',
    })
    expect(domain.put).toHaveBeenCalledWith('s1', { replay: true, clipId: 'file:own-00000000' })

    expect(await overrides.write('s1', { replay: false })).toEqual({
      replay: false,
      clipId: 'file:own-00000000',
    })
    expect(domain.put).toHaveBeenLastCalledWith('s1', { replay: false, clipId: 'file:own-00000000' })
  })

  it('deletes the record once neither field is set', async () => {
    const { overrides, domain } = createOverrides(new Map([
      ['s1', { replay: false, clipId: 'builtin:brand' }],
    ]))

    expect(await overrides.write('s1', { clipId: null })).toBe(NO_OVERRIDE)
    expect(domain.remove).toHaveBeenCalledWith('s1')
    expect(domain.records.has('s1')).toBe(false)
    expect(domain.put).not.toHaveBeenCalled()
  })

  it('stores a record that only enables replay on every entry', async () => {
    const { overrides, domain } = createOverrides()

    expect(await overrides.write('s1', { replay: true })).toEqual({ replay: true, clipId: null })
    expect(domain.records.get('s1')).toEqual({ replay: true, clipId: null })
    expect(domain.put).toHaveBeenCalledTimes(1)
  })

  it('surfaces a failed durable write', async () => {
    const { overrides } = createOverrides(new Map(), true)
    await expect(overrides.write('s1', { replay: true })).rejects.toThrow('storage offline')
  })
})

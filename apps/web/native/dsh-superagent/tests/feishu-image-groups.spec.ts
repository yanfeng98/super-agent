import { describe, expect, it } from 'vitest'
import { KV } from '../src/kv.ts'
import type { DshImEvent } from '../src/sources/dsh-im.ts'
import { endImageGroup, imageGroupRoot, isStandaloneImage, type ImageGroup, type ImageGroupState } from '../src/sources/feishu-image-groups.ts'

const event = (overrides: Partial<DshImEvent> = {}): DshImEvent => ({
  version: 1, channel: 'feishu', botId: 'bot_1', fingerprint: 'f'.repeat(64), eventId: 'event_1',
  messageId: 'om_1', actor: { kind: 'user', id: 'ou_alice', name: 'Alice' },
  conversation: { kind: 'group', id: 'oc_team' }, mentions: [], mentionedAccount: false,
  at: '2026-11-25T10:00:00.000Z', text: '[图片] img_a', messageType: 'image', images: ['img_a'],
  reply: { messageId: 'om_1', conversationId: 'oc_team', actorId: 'ou_alice' },
  ...overrides,
})

const store = (): ImageGroupState & { rows: Map<string, Record<string, ImageGroup>> } => {
  const rows = new Map<string, Record<string, ImageGroup>>()
  return { get: key => rows.get(key), set: (key, value) => { rows.set(key, value) }, rows }
}

describe('standalone image grouping', () => {
  it('only treats a chat-container image as part of a run', () => {
    expect(isStandaloneImage(event())).toBe(true)
    expect(isStandaloneImage(event({ messageType: 'text', text: 'hello' }))).toBe(false)
    expect(isStandaloneImage(event({ reply: { ...event().reply, threadId: 'omt_1' } }))).toBe(false)
    expect(isStandaloneImage(event({ reply: { ...event().reply, rootId: 'om_root' } }))).toBe(false)
  })

  it('roots consecutive images from one sender on the first image', () => {
    const state = store()
    expect(imageGroupRoot(state, event({ messageId: 'om_a', at: '2026-11-25T10:00:00.000Z' }), 30_000)).toBe('om_a')
    expect(imageGroupRoot(state, event({ messageId: 'om_b', at: '2026-11-25T10:00:02.000Z' }), 30_000)).toBe('om_a')
    expect(imageGroupRoot(state, event({ messageId: 'om_c', at: '2026-11-25T10:00:10.000Z' }), 30_000)).toBe('om_a')
    expect(state.rows.get(KV.feishu.imageGroups('oc_team'))).toMatchObject({ ou_alice: { rootId: 'om_a', lastAt: '2026-11-25T10:00:10.000Z' } })
  })

  it('starts a new run when the gap exceeds the window or another sender sends an image', () => {
    const state = store()
    expect(imageGroupRoot(state, event({ messageId: 'om_a', at: '2026-11-25T10:00:00.000Z' }), 30_000)).toBe('om_a')
    expect(imageGroupRoot(state, event({ messageId: 'om_b', at: '2026-11-25T10:01:00.000Z' }), 30_000)).toBe('om_b')
    expect(imageGroupRoot(state, event({ messageId: 'om_c', actor: { kind: 'user', id: 'ou_bob' }, at: '2026-11-25T10:01:01.000Z' }), 30_000)).toBe('om_c')
  })

  it('ends a run when the same sender sends a non-image message', () => {
    const state = store()
    expect(imageGroupRoot(state, event({ messageId: 'om_a' }), 30_000)).toBe('om_a')
    const text = event({ messageId: 'om_text', messageType: 'text', text: 'look at these', at: '2026-11-25T10:00:05.000Z' })
    expect(imageGroupRoot(state, text, 30_000)).toBeUndefined()
    endImageGroup(state, text)
    expect(imageGroupRoot(state, event({ messageId: 'om_b', at: '2026-11-25T10:00:06.000Z' }), 30_000)).toBe('om_b')
  })

  it('keeps senders in separate runs', () => {
    const state = store()
    expect(imageGroupRoot(state, event({ messageId: 'om_a', at: '2026-11-25T10:00:00.000Z' }), 30_000)).toBe('om_a')
    expect(imageGroupRoot(state, event({ messageId: 'om_b', actor: { kind: 'user', id: 'ou_bob' }, at: '2026-11-25T10:00:01.000Z' }), 30_000)).toBe('om_b')
    expect(imageGroupRoot(state, event({ messageId: 'om_c', at: '2026-11-25T10:00:02.000Z' }), 30_000)).toBe('om_a')
  })
})

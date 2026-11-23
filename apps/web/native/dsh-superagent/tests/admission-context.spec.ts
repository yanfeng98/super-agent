import { afterEach, expect, it } from 'vitest'
import { admissionContext, threadWindow } from '../src/sources/admission-context.ts'
import { Commands } from '../src/infra/commands.ts'
import { feishuImKind, imTrigger } from '../src/sources/feishu-im.ts'
import { setDshIm, type DshImEvent, type DshImService } from '../src/sources/dsh-im.ts'
import type { ContextMessage } from '../src/sources/kind.ts'
import { SourceRegistry } from '../src/sources/registry.ts'
import { SettingsSchema } from '../src/policy.ts'
import { testDeployment } from './fixtures/deployment.ts'
const message = (index: number): ContextMessage => ({ id: `om_${index}`, from: 'ou_user', at: String(index), text: `Message ${index}` })
/** One authenticated message as dsh-im's history returns it. */
const event = (index: number): DshImEvent => ({ version: 1, channel: 'feishu', botId: 'bot_1', fingerprint: 'f'.repeat(64), eventId: `event_${index}`,
  messageId: `om_${index}`, actor: { kind: 'user', id: 'ou_user', name: 'ou_user' }, conversation: { kind: 'group', id: 'oc_chat' },
  mentions: [], mentionedAccount: true, at: new Date(Date.parse('2026-09-11T12:00:00Z') + index * 1000).toISOString(), text: `message ${index}`,
  reply: { messageId: `om_${index}`, conversationId: 'oc_chat', actorId: 'ou_user' } })
/** The dsh-im service the kind reads Feishu context through; `overrides` answer the reads a test inspects. */
const service = (overrides: Partial<DshImService> = {}): DshImService => ({
  contractVersion: 1, inboundVersion: 1,
  listBots: async () => [{ botId: 'bot_1', channel: 'feishu' }],
  listTargets: async () => [],
  describeBot: async () => ({ version: 1, botId: 'bot_1', channel: 'feishu', account: { fingerprint: 'f'.repeat(64), name: 'Bot' },
    connected: true, superagent: true, superagentActive: false, capabilities: ['exclusive-text-consumer'] }),
  consumeInbound: async () => () => {},
  historyChecked: async () => ({ version: 1, scope: 'chat', events: [], omitted: 0, hasMore: false, coverage: 'provider-visible-human-text' }),
  replyChecked: async () => ({ sent: true, messageId: 'om_reply' }),
  ...overrides,
})
afterEach(() => { setDshIm(undefined) })
it('returns the thread opening and trigger neighbors with exact internal and trailing gaps', () => {
  const result = threadWindow(Array.from({ length: 40 }, (_, i) => message(i)), 'om_25')
  expect(result.entries.map((item: { position: number }) => item.position)).toEqual([1, 2, 3, 4, 5, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31])
  expect(result.entries[5]?.omitted_before).toBe(15)
  expect(result.omitted_after).toBe(9)
  expect(result.source_found).toBe(true)
})
it('deduplicates overlapping windows and reports an absent source without inventing neighbors', () => {
  const messages = Array.from({ length: 12 }, (_, i) => message(i))
  expect(threadWindow(messages, 'om_3').entries.map((item: { position: number }) => item.position)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9])
  expect(threadWindow(messages, 'om_absent')).toMatchObject({ source_found: false, loaded_messages: 12, omitted_after: 7 })
})
it('retains pagination limits and exact loaded counts rather than claiming complete history', async () => {
  const asked: unknown[][] = []
  setDshIm(service({ threadHistoryChecked: async (...args) => { asked.push(args); return { version: 1, scope: 'thread',
    events: [event(0), event(1)], omitted: 0, hasMore: true, nextCursor: 'next', coverage: 'provider-visible-human-text' } } }))
  const trigger = imTrigger({ ...event(9), reply: { messageId: 'om_9', conversationId: 'oc_chat', actorId: 'ou_user', threadId: 'omt_thread' } })
  const sources = new SourceRegistry(() => SettingsSchema.parse({ sources: { feishu: { enabled: true, config: { readPageSize: 2 } } } }))
  sources.register(feishuImKind)
  const commands = new Commands(() => testDeployment(), {}, new AbortController().signal)
  const result = await admissionContext(sources.threadContext(trigger)!, commands, 1)
  expect(result).toMatchObject({ complete: false, next: 'next', window: { loaded_messages: 2, source_found: false } })
  expect(asked[0]?.[1]).toMatchObject({ chatId: 'oc_chat', threadId: 'omt_thread', limit: 2 })
})
it('gives a message outside a topic the five chat messages before it, and says it read the chat', async () => {
  const asked: unknown[][] = []
  setDshIm(service({ historyChecked: async (...args) => { asked.push(args); return { version: 1, scope: 'nearby',
    events: [-5, -4, -3, -2, -1, 0].map(event), omitted: 0, hasMore: false, coverage: 'provider-visible-human-text' } } }))
  const trigger = imTrigger(event(0))
  const sources = new SourceRegistry(() => SettingsSchema.parse({ sources: { feishu: { enabled: true, config: {} } } }))
  sources.register(feishuImKind)
  const commands = new Commands(() => testDeployment(), {}, new AbortController().signal)
  const result = await admissionContext(sources.threadContext(trigger)!, commands, 3)
  expect(result).toMatchObject({ status: 'chat_neighbours', complete: true, window: { source_found: true, loaded_messages: 6 } })
  expect(result.messages.map(message => message.id)).toEqual([-5, -4, -3, -2, -1, 0].map(n => `om_${n}`))
  expect(asked[0]?.[2]).toMatchObject({ scope: 'nearby', beforeCount: 5, afterCount: 5 })
})

import { describe, expect, it } from 'vitest'
import { feishu, startRuntime } from './fixtures/host.ts'
import { BOT_ID, CHAT, MEMBER } from './fixtures/host.ts'

describe('main Session Feishu mirror', () => {
  it('binds the orchestrator Session to the chat of the latest Feishu trigger', async () => {
    const h = await startRuntime()
    try {
      h.runtime.receive(feishu('1', '@bot 修一下回归'))
      await h.runtime.idle()
      expect(h.bindings.at(-1)).toMatchObject({
        sessionId: h.runtime.masterId,
        description: { channel: 'feishu', botId: BOT_ID, conversationKey: `group:${CHAT}`, chatId: CHAT },
      })
    } finally { await h.stop() }
  })

  it('binds a direct chat to the speaker, not the chat id', async () => {
    const h = await startRuntime()
    try {
      h.runtime.receive(feishu('2', '@bot 你好', { chat_type: 'p2p' }))
      await h.runtime.idle()
      expect(h.bindings.at(-1)?.description.conversationKey).toBe(`p2p:${MEMBER}`)
    } finally { await h.stop() }
  })

  it('suppresses the current turn when orchestrator_reply already answered the person', async () => {
    const h = await startRuntime()
    try {
      const { ticket, sessionId } = await h.acceptedTicket(feishu('3', '@bot 修一下回归'))
      await h.runTool(sessionId, 'orchestrator_report', { state: 'ready', summary: 'Done.' })
      h.replies.length = 0
      await h.runTool(h.runtime.masterId, 'orchestrator_reply', {
        purpose: 'result', ticket_id: ticket.id, text: '已处理。',
      })
      expect(h.suppressions.at(-1)).toBe(h.runtime.masterId)
    } finally { await h.stop() }
  })
})

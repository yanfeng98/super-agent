import { createServer, type Server } from 'node:http'
import type { AddressInfo } from 'node:net'
import { afterEach, describe, expect, it } from 'vitest'
import { sendWebhook, type WebhookPayload } from '../src/app/webhook.ts'

let server: Server | undefined
afterEach(() => { server?.close(); server = undefined })
const payload: WebhookPayload = { event: 'attention', at: '2026-11-18T00:00:00.000Z',
  ticket: { id: 't1', number: 3, title: '修复回归' }, question: '用哪个分支？' }
/** Start a local webhook receiver and return its URL and the bodies it received. */
async function receiver(status = 204): Promise<{ url: string; bodies: string[]; types: string[] }> {
  const bodies: string[] = [], types: string[] = []
  server = createServer((request, response) => {
    let body = ''
    request.on('data', chunk => { body += String(chunk) })
    request.on('end', () => {
      bodies.push(body); types.push(String(request.headers['content-type'] ?? ''))
      response.writeHead(status); response.end()
    })
  })
  await new Promise<void>(resolve => server!.listen(0, '127.0.0.1', resolve))
  return { url: `http://127.0.0.1:${(server!.address() as AddressInfo).port}/hook`, bodies, types }
}

describe('webhook notifications', () => {
  it('POSTs the question or ending as JSON', async () => {
    const { url, bodies, types } = await receiver()
    await sendWebhook(url, payload, new AbortController().signal)
    expect(JSON.parse(bodies[0]!)).toEqual(payload)
    expect(types[0]).toContain('application/json')
  })
  it('reports a non-2xx answer so the caller can log it', async () => {
    const { url } = await receiver(500)
    await expect(sendWebhook(url, payload, new AbortController().signal)).rejects.toThrow(/answered 500/)
  })
})

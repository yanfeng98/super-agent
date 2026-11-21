/** Best-effort HTTP notifications for local tickets, so a person hears about work while the Web app is closed. */
export interface WebhookTicket { id: string; number: number; title: string }
/** The body POSTed to the local Source's notification webhook. */
export type WebhookPayload =
  | { event: 'attention'; at: string; ticket: WebhookTicket; question: string }
  | { event: 'ended'; at: string; ticket: WebhookTicket; outcome: string; reason: string }
  /** A merge event, a check result or a push, stated in one sentence. */
  | { event: 'merged' | 'reverted' | 'review' | 'automerge' | 'check' | 'push'; at: string; ticket: WebhookTicket; text: string }
/**
 * POSTs a notification as JSON, with a short timeout.
 * @throws when the webhook answers outside 2xx or the request fails.
 * @param signal - Aborted when the host stops.
 */
export async function sendWebhook(url: string, payload: WebhookPayload, signal: AbortSignal, fetchImpl: typeof fetch = fetch): Promise<void> {
  const response = await fetchImpl(url, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(payload),
    signal: AbortSignal.any([signal, AbortSignal.timeout(5000)]) })
  if (!response.ok) throw new Error(`The webhook answered ${response.status}`)
}

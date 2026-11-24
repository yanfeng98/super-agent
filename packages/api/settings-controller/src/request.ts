/** Remote request validation shared by the settings Controller namespaces. */

import { RemoteError } from '@deepseek-ai/dsh-typert-protocol'
import type { z } from 'zod'

/**
 * Parse the domain constraints that are more specific than generated TypeScript codecs.
 * @param method - Remote method name named in the failure.
 * @param schema - request schema to enforce.
 * @param value - decoded request payload.
 * @returns the schema's parsed value.
 * @throws RemoteError with `gateway/bad-request` and the schema issues.
 */
export function parseRequest<T>(method: string, schema: z.ZodType<T>, value: unknown): T {
  const parsed = schema.safeParse(value)
  if (!parsed.success) {
    throw new RemoteError('gateway/bad-request', `invalid payload for ${method}`, { issues: parsed.error.issues })
  }
  return parsed.data
}

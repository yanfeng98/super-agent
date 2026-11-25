// @vitest-environment jsdom
/** A Feishu relay shows its plain text and source badge, with the model-facing payload collapsed. */
import { afterEach, describe, expect, it } from 'vitest'
import { cleanup, render } from '@testing-library/react'
import { makeTranslate } from '@deepseek-ai/dsh-client-test-runtime'
import { zh as commonZh } from '@deepseek-ai/dsh-client-locale/src/locales/zh.ts'
import type { ContextMessageNode } from '../src/client/contract/snapshot.ts'
import { ContextInjectionRow } from '../src/client/chat/ContextInjectionRow.tsx'
import { contextForm, contextProducer } from '../src/client/conversation-nodes/event-projection.ts'
import { zh } from '../src/client/locale.ts'

afterEach(cleanup)

const t = makeTranslate(zh, commonZh)
const raw = JSON.stringify({ kind: 'candidate', trigger_id: 'tr1', thread: 'feishu:oc_1:om_1', text: '都合并且push了吗',
  next_action: 'Route it with orchestrator_route.' })
const source = { kind: 'dsh-superagent', form: 'relay', origin: 'feishu', summary: '都合并且push了吗' }
const content = [{ type: 'text' as const, text: raw }] as ContextMessageNode['content']

function show() {
  const producer = contextProducer(source)
  return render(<ContextInjectionRow content={content} source={source} producer={producer} form={contextForm(source)} t={t} />)
}

describe('Feishu relay context row', () => {
  it('shows the plain message and source badge above the collapsed raw payload', () => {
    const view = show()
    const plain = view.container.querySelector('[data-context-relay-text]')!
    expect(plain.textContent).toBe('都合并且push了吗')
    expect(view.container.querySelector('[data-context-origin-badge]')!.textContent).toBe(zh['message.origin.feishu'])
    const details = view.container.querySelector('details')!
    expect(details.open).toBe(false)
    expect(details.textContent).toContain('trigger_id')
  })

  it('keeps the model-facing payload available behind a details toggle', () => {
    const view = show()
    const details = view.container.querySelector('details')!
    expect(details.open).toBe(false)
    details.querySelector('summary')!.click()
    expect(details.open).toBe(true)
    expect(details.textContent).toContain('trigger_id')
  })
})

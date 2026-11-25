// @vitest-environment jsdom
/** Writes production-component render captures for the Feishu relay before/after evidence. */
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { it } from 'vitest'
import { act, render } from '@testing-library/react'
import { makeTranslate } from '@deepseek-ai/dsh-client-test-runtime'
import { zh as commonZh } from '@deepseek-ai/dsh-client-locale/src/locales/zh.ts'
import type { ContextMessageNode } from '../src/client/contract/snapshot.ts'
import { ContextInjectionRow } from '../src/client/chat/ContextInjectionRow.tsx'
import { contextForm, contextProducer } from '../src/client/conversation-nodes/event-projection.ts'
import { zh } from '../src/client/locale.ts'

const t = makeTranslate(zh, commonZh)
const raw = JSON.stringify({ kind: 'candidate', trigger_id: 'tr1', from: 'ou_member', thread: 'feishu:oc_team:om_1',
  at: '2026-11-25T16:00:00.000Z', text: '都合并且push了吗', strength: 'inferred',
  next_action: 'Ignore it with orchestrator_route if the card shows it touches no enabled rule; else assess it with orchestrator_eval.' })
const content = [{ type: 'text' as const, text: raw }] as ContextMessageNode['content']

function cssFor(root: HTMLElement): string {
  const body = readFileSync(resolve(process.cwd(), 'packages/client/ui-chat/src/client/chat/ContextInjectionRow.module.css'), 'utf8')
  // CSS Modules in the test transform maps `.class` to `_class_hash`; map each
  // hashed class found in the rendered DOM back to its source rule.
  const used = new Set<string>()
  root.querySelectorAll('*').forEach((el) => { el.classList.forEach((name) => { used.add(name) }) })
  const rules = [...body.matchAll(/(^|\})\s*([^{}]+)\{([^}]*)\}/g)].map(match => ({ selector: match[2]!.trim(), body: match[3]!.trim() }))
  return rules.filter(rule => rule.selector.startsWith('.')).map((rule) => {
    const key = rule.selector.slice(1).split(/[\s,:]/)[0]!
    const hashed = [...used].find(name => name.startsWith(`_${key}_`))
    return hashed ? rule.selector.replace(`.${key}`, `.${hashed}`) + ` { ${rule.body} }` : ''
  }).filter(Boolean).join('\n')
}

it('writes the production capture files', () => {
  const beforeSource = { kind: 'dsh-superagent', form: 'relay' }
  const afterSource = { kind: 'dsh-superagent', form: 'relay', origin: 'feishu', summary: '都合并且push了吗' }
  for (const [name, source] of [['before', beforeSource], ['after', afterSource]] as const) {
    const view = render(<ContextInjectionRow content={content} source={source}
      producer={contextProducer(source)} form={contextForm(source)} t={t} />)
    if (name === 'before') {
      // The reported view had the disclosure expanded, exposing the raw JSON.
      act(() => { view.container.querySelector('[data-disclosure-row]')?.dispatchEvent(new MouseEvent('click', { bubbles: true })) })
    }
    const css = cssFor(view.container)
    const html = `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>Feishu relay ${name}</title>
<style>html,body{margin:0;background:#fff;color:#222;font-family:system-ui,"PingFang SC",sans-serif}
.chat{width:720px;padding:28px;box-sizing:border-box}
${css}
</style></head><body><div class="chat">${view.container.innerHTML}</div></body></html>`
    const out = resolve(process.cwd(), `evidence/feishu-relay/capture-${name}.html`)
    writeFileSync(out, html)
    view.unmount()
  }
})

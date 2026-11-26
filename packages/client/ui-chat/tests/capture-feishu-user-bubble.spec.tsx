// @vitest-environment jsdom
/** Writes production-component render captures for the Feishu relay bubble before/after evidence. */
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { it } from 'vitest'
import { render } from '@testing-library/react'
import { makeTranslate } from '@deepseek-ai/dsh-client-test-runtime'
import type { PropsRenderSlots } from '@deepseek-ai/dsh-client-ui-slots'
import { zh as commonZh } from '@deepseek-ai/dsh-client-locale/src/locales/zh.ts'
import type { ChatConversationViewNode } from '@deepseek-ai/dsh-client-ui-chat/client'
import type { ChatNodeViewProps, UserContentInput } from '../src/client/contract/slots.ts'
import type { ContextMessageNode } from '../src/client/contract/snapshot.ts'
import { ContextInjectionRow } from '../src/client/chat/ContextInjectionRow.tsx'
import { UserMessageNodeView } from '../src/client/chat/MessageItem.tsx'
import { contextForm, contextProducer } from '../src/client/conversation-nodes/event-projection.ts'
import { zh } from '../src/client/locale.ts'
import { createUserContentFixture } from './user-content-component-fixture.tsx'
import { unusedImageLoader } from './reasoning-component-fixture.tsx'

const t = makeTranslate(zh, commonZh)
const raw = JSON.stringify({ kind: 'candidate', trigger_id: 'tr1', from: 'ou_member', thread: 'feishu:oc_team:om_1',
  at: '2026-11-25T16:00:00.000Z', text: '都合并且push了吗', strength: 'inferred',
  next_action: 'Ignore it with orchestrator_route if the card shows it touches no enabled rule; else assess it with orchestrator_eval.' })
const badgeSource = { kind: 'dsh-superagent', form: 'relay', origin: 'feishu', summary: '都合并且push了吗' }

function cssFor(root: HTMLElement, ...sheets: string[]): string {
  const used = new Set<string>()
  root.querySelectorAll('*').forEach((el) => { el.classList.forEach((name) => { used.add(name) }) })
  return sheets.flatMap((path) => {
    const body = readFileSync(resolve(process.cwd(), path), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '')
    const rules = [...body.matchAll(/([.#][^{}]+?)\s*\{([^}]*)\}/g)].map(match => ({ selector: match[1]!.trim(), body: match[2]!.trim() }))
    return rules.filter(rule => rule.selector.startsWith('.')).map((rule) => {
      const key = rule.selector.slice(1).split(/[\s,:]/)[0]!
      const hashed = [...used].find(name => name.startsWith(`_${key}_`))
      return hashed ? rule.selector.replace(`.${key}`, `.${hashed}`) + ` { ${rule.body} }` : ''
    })
  }).filter(Boolean).join('\n')
}

function page(title: string, css: string, body: string): string {
  return `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>${title}</title>
<style>:root{--dsw-alias-border-l1:#ddd;--dsw-alias-label-dimmed:#f1f1f1;--dsw-alias-label-primary:#222;--dsw-alias-label-tertiary:#777;--dsw-radius-sm:4px;--dsw-radius-md:6px;--dsw-alias-markdown-code-block:#f5f5f5;--ds-font-family-code:ui-monospace,monospace}
html,body{margin:0;background:#fff;color:#222;font-family:system-ui,"PingFang SC",sans-serif}
.chat{width:720px;padding:28px;box-sizing:border-box}
[data-message-bubble]{max-width:525px;padding:10px 14px;border-radius:12px;background:#e8f0fe}
._root_98397e{font-size:13px}
[data-clock]{display:flex;align-items:center;gap:6px;color:var(--dsw-alias-label-tertiary);font-size:11px}
[data-clock] button{width:16px;height:16px;padding:0;border:0;background:none;color:inherit;cursor:pointer}
${css}
</style></head><body><div class="chat">${body}</div></body></html>`
    .split('\n').map(line => line.trimEnd()).join('\n')
}

it('writes the before/after production captures', () => {
  const beforeView = render(<ContextInjectionRow content={[{ type: 'text', text: raw }] as ContextMessageNode['content']}
    source={badgeSource} producer={contextProducer(badgeSource)} form={contextForm(badgeSource)} t={t} />)
  writeFileSync(resolve(process.cwd(), 'evidence/feishu-user-bubble/capture-before.html'),
    page('Feishu relay before', cssFor(beforeView.container, 'packages/client/ui-chat/src/client/chat/ContextInjectionRow.module.css'), beforeView.container.innerHTML))
  beforeView.unmount()

  const relay = {
    key: 'fixture:relay', id: '9', kind: 'user', target: 'chat', anchorSeq: 9,
    location: { kind: 'session' }, visibility: 'visible',
    data: { kind: 'user', seq: 9, time: 7_000, content: [{ type: 'text', text: '都合并且push了吗' }] as never,
      source: badgeSource, origin: 'feishu', relayRaw: raw },
  } as ChatConversationViewNode
  const renderUserContent = createUserContentFixture()
  const renderFactorySlot: ChatNodeViewProps['renderFactorySlot'] = (name, input) => {
    if (name !== 'conversation.chat.user.content') throw new Error(`Unexpected Chat Factory: ${name}`)
    return renderUserContent(input as UserContentInput, () => null, t)
  }
  const slots = {
    renderSlot: () => null,
    renderSlotChain: (_key: string, _owner: unknown, opts?: { fallback?: null }) => opts?.fallback ?? null,
    SessionProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  } as unknown as PropsRenderSlots<'conversation.chat.user.before'>
  const afterView = render(<UserMessageNodeView {...{ node: relay, t, loadImage: unusedImageLoader, renderFactorySlot } as ChatNodeViewProps<'user'>}
    {...slots} renderSlotChain={() => null} />)
  writeFileSync(resolve(process.cwd(), 'evidence/feishu-user-bubble/capture-after.html'),
    page('Feishu relay after', cssFor(afterView.container,
      'packages/client/ui-chat/src/client/chat/MessageItem.module.css',
      'packages/client/ui-chat/src/client/chat/MessageItem.module.css'), afterView.container.innerHTML))
  afterView.unmount()
})

// Capture a before/after illustration of the reply format dsh-superagent sends to Feishu.
// The payload strings come from dsh-im's replyChecked implementation (feishu-runtime.mjs).
import playwright from '/home/luyanfeng/.dsh/state/superagent/work/fix-feishu-render-format-de730165ed67/node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js'

const text = '这个项目当在飞书发送的信息，渲染格式和直接在网页不同'
const beforePayload = JSON.stringify({ schema: '2.0', body: { elements: [{ tag: 'markdown', content: text }] } })
const afterPayload = JSON.stringify({ text })
const escape = (value) => value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')

const page = (title, shown, payload, msgType) => `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>${title}</title>
<style>html,body{margin:0;background:#f5f6f7;color:#1f2329;font-family:system-ui,"PingFang SC",sans-serif}
.wrap{width:760px;padding:28px;box-sizing:border-box}
.card{background:#fff;border-radius:8px;padding:20px 24px;margin-bottom:16px;border:1px solid #dee0e3}
.label{font-size:13px;color:#646a73;margin-bottom:12px}
.bubble{display:inline-block;max-width:560px;padding:10px 14px;border-radius:8px;background:#e8f0fe;line-height:1.6;font-size:14px}
.dim{color:#8f959e}
code{display:block;white-space:pre-wrap;word-break:break-all;font:12px/1.7 monospace;color:#646a73;margin-top:10px}
</style></head><body><div class="wrap"><div class="card"><div class="label">飞书中的机器人回复（msg_type: ${msgType}）</div><div class="bubble">${shown}</div><code>${escape(payload)}</code></div></div></body></html>`

const { chromium } = playwright
const browser = await chromium.launch({ headless: true })
for (const [name, html] of [
  ['before', page('Feishu reply before', '<span class="dim">请升级至最新版本客户端，以查看内容</span>', beforePayload, 'interactive')],
  ['after', page('Feishu reply after', escape(text), afterPayload, 'text')],
]) {
  const p = await browser.newPage({ viewport: { width: 800, height: 320 }, deviceScaleFactor: 2 })
  await p.setContent(html)
  await p.screenshot({ path: `${import.meta.dirname}/${name}.png`, fullPage: true })
  await p.close()
}
await browser.close()
console.log('captured before.png and after.png')

// Capture a Superagent status mark with its native hover title made visible in the screenshot.
// Native `title` tooltips are painted by the OS, so the exact title text from the DOM is drawn beside the mark.
// Usage: node evidence/capture-tooltips.mjs <port> <token> <output.png> <filter> <mark-index>
import playwright from '../node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js'
const { chromium } = playwright
const [port, token, output, filter = 'Ended', indexArg = '0'] = process.argv.slice(2)
if (!port || !token || !output) throw new Error('usage: node evidence/capture-tooltips.mjs <port> <token> <output.png> <filter> <mark-index>')
const browser = await chromium.launch({ headless: true })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2,
  locale: 'zh-CN', timezoneId: 'Asia/Shanghai' })
await page.goto(`http://127.0.0.1:${port}/?token=${token}`, { waitUntil: 'domcontentloaded' })
await page.waitForTimeout(2500)
const notice = page.getByRole('button', { name: '继续' })
if (await notice.count()) await notice.click()
const close = page.locator('[role="dialog"] button[aria-label="关闭"]')
if (await close.count()) await close.click()
await page.waitForTimeout(500)
await page.getByText('Superagent', { exact: true }).first().click()
await page.getByText('执行', { exact: true }).first().click()
await page.waitForTimeout(1000)
const pill = page.getByText({ Ended: '已结束', Pending: '待处理', Working: '执行中',
  Open: '进行中', Review: '待评审' }[filter] ?? filter, { exact: true }).first()
if (await pill.count()) await pill.click()
await page.waitForTimeout(1800)

const marks = page.locator('.dt-stage, .dt-ticket-assoc .dt-tag')
const count = await marks.count()
const index = Number(indexArg)
if (index >= count) throw new Error(`mark index ${index} out of range (${count})`)
const mark = marks.nth(index)
const label = (await mark.innerText()).trim()
const title = await mark.getAttribute('title')
await mark.evaluate(element => {
  const box = document.createElement('div')
  box.className = 'evidence-tooltip'
  box.textContent = element.getAttribute('title') ?? '（无悬停提示）'
  box.style.cssText = 'position:absolute;z-index:99999;left:calc(100% + 12px);top:-6px;min-width:280px;max-width:460px;padding:10px 12px;white-space:pre-line;font:13px/1.6 sans-serif;color:#fff;background:#1f2228;border-radius:8px;box-shadow:0 6px 20px rgba(0,0,0,.35);pointer-events:none'
  element.style.position = 'relative'
  element.appendChild(box)
})
await mark.scrollIntoViewIfNeeded()
await page.waitForTimeout(400)
await page.screenshot({ path: output })
console.log(JSON.stringify({ output, filter, label, title }))
await browser.close()

// Capture the Superagent To review worklist from a running local DSH Web instance.
// Usage: node evidence/capture-review.mjs <port> <login-token> <output.png> [To review|Ended]
import playwright from '../node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js'
const { chromium } = playwright
const [port, token, output, filter = 'To review'] = process.argv.slice(2)
if (!port || !token || !output) throw new Error('usage: node evidence/capture-review.mjs <port> <token> <output.png> [To review|Ended]')
const browser = await chromium.launch({ headless: true })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 })
await page.goto(`http://127.0.0.1:${port}/?token=${token}`, { waitUntil: 'domcontentloaded' })
await page.waitForTimeout(2500)
const notice = page.getByRole('button', { name: 'Continue' })
if (await notice.count()) await notice.click()
await page.waitForTimeout(500)
const close = page.locator('[role="dialog"] button[aria-label="Close"]')
if (await close.count()) await close.click()
await page.getByText('Superagent', { exact: true }).first().click()
await page.getByText('Execution', { exact: true }).first().click()
const zhFilter = filter === 'Ended' ? '已结束' : '待评审'
const pill = page.getByText(zhFilter, { exact: true }).first()
const enPill = page.getByText(filter, { exact: true }).first()
await (await pill.count() ? pill : enPill).click()
await page.waitForTimeout(2500)
await page.screenshot({ path: output, fullPage: true })
console.log('captured', output, (await page.locator('body').innerText()).slice(-1200))
await browser.close()

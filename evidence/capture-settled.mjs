// Capture the Superagent Execution list from a running local DSH Web instance.
// Usage: node evidence/capture-settled.mjs <port> <token> <output.png> <filter label>
import playwright from '../node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js'
const { chromium } = playwright
const [port, token, output, filter] = process.argv.slice(2)
if (!port || !token || !output || !filter) throw new Error('usage: node evidence/capture-settled.mjs <port> <token> <output.png> <filter>')
const browser = await chromium.launch({ headless: true })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 })
await page.goto(`http://127.0.0.1:${port}/?token=${token}`, { waitUntil: 'domcontentloaded' })
await page.waitForTimeout(4000)
const skip = page.getByRole('button', { name: /Skip/ })
if (await skip.count()) await skip.click()
await page.waitForTimeout(600)
const continueButton = page.getByRole('button', { name: 'Continue' })
for (let index = 0; index < 20 && await continueButton.count(); index += 1) {
  await continueButton.click()
  await page.waitForTimeout(500)
}
await page.keyboard.press('Escape')
const close = page.locator('[role="dialog"] button[aria-label="Close"]')
if (await close.count()) await close.click()
await page.waitForTimeout(600)
await page.getByText('Superagent', { exact: true }).first().click()
await page.waitForTimeout(800)
await page.getByText('Execution', { exact: true }).first().click()
await page.waitForTimeout(1500)
const zh = { '已结束': 'Ended', '无需合并 / 已收尾': 'Settled / no merge needed' }
const label = zh[filter] ?? filter
const pill = page.getByText(filter, { exact: true }).first()
const english = page.getByText(label, { exact: true }).first()
await (await pill.count() ? pill : english).click()
await page.waitForTimeout(2500)
await page.screenshot({ path: output, fullPage: true })
console.log('captured', output)
console.log((await page.locator('body').innerText()).slice(-1500))
await browser.close()

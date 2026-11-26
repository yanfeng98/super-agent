// Capture a live Execution row after scrolling it into view.
// Usage: node evidence/capture-live-ticket-row.mjs <token> <output.png> <filter> <ticket-number> [port]
import playwright from '../node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js'
const { chromium } = playwright
const [token, output, filter, number, port = '3080'] = process.argv.slice(2)
if (!token || !output || !filter || !number) throw new Error('usage: node evidence/capture-live-ticket-row.mjs <token> <output.png> <filter> <number> [port]')
const browser = await chromium.launch({ headless: true })
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 })
  await page.goto(`http://127.0.0.1:${port}/?token=${token}`, { waitUntil: 'domcontentloaded' })
  await page.waitForTimeout(4000)
  const skip = page.getByRole('button', { name: /Skip/ })
  if (await skip.count()) await skip.click()
  await page.waitForTimeout(600)
  const cont = page.getByRole('button', { name: 'Continue' })
  for (let i = 0; i < 20 && await cont.count(); i += 1) { await cont.click(); await page.waitForTimeout(400) }
  await page.keyboard.press('Escape')
  const close = page.locator('[role="dialog"] button[aria-label="Close"]')
  if (await close.count()) await close.click()
  await page.waitForTimeout(600)
  await page.getByText('Superagent', { exact: true }).first().click()
  await page.waitForTimeout(800)
  await page.getByText('Execution', { exact: true }).first().click()
  await page.waitForTimeout(1200)
  const pill = page.getByText(filter, { exact: true }).first()
  if (!await pill.count()) throw new Error(`filter not found: ${filter}`)
  await pill.click()
  await page.waitForTimeout(2000)
  const row = page.locator('.dt-ticket', { hasText: number }).first()
  await row.scrollIntoViewIfNeeded()
  await page.waitForTimeout(600)
  await page.screenshot({ path: output })
  console.log('captured', output)
  console.log((await row.innerText()).replace(/\n/g, ' | '))
} finally { await browser.close() }

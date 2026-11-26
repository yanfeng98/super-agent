// Capture the real, running Superagent Execution page for the stale-push fix.
// Usage: node evidence/capture-live-push-marks.mjs <token> <output.png> [port]
import playwright from '../node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js'
const { chromium } = playwright
const [token, output, port = '3080'] = process.argv.slice(2)
if (!token || !output) throw new Error('usage: node evidence/capture-live-push-marks.mjs <token> <output.png> [port]')
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
  for (const label of ['已结束', 'Ended']) {
    const pill = page.getByText(label, { exact: true }).first()
    if (await pill.count()) { await pill.click(); break }
  }
  await page.waitForTimeout(2500)
  await page.screenshot({ path: output, fullPage: true })
  console.log('captured', output)
  console.log((await page.locator('body').innerText()).slice(-2200))
} finally { await browser.close() }

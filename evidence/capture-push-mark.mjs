// Captures the real panel's Execution row for the stale and reconciled push states.
// Usage: node evidence/capture-push-mark.mjs <output.png> [landed]
import playwright from '../node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js'
const { chromium } = playwright
const [output, landed] = process.argv.slice(2)
if (!output) throw new Error('usage: node evidence/capture-push-mark.mjs <output.png> [landed]')
const browser = await chromium.launch({ headless: true })
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 2 })
  await page.goto(`file://${process.cwd()}/evidence/push-mark.fixture.html${landed ? '?landed=1' : ''}`)
  await page.waitForSelector('.dt-ticket', { timeout: 15_000 })
  await page.waitForTimeout(600)
  await page.screenshot({ path: output, fullPage: true })
  console.log('captured', output)
  console.log((await page.locator('.dt-ticket').first().innerText()).replace(/\n/g, ' | '))
} finally { await browser.close() }

// Capture the committed baseline and current render of the Feishu relay bubble
// under identical viewport, theme and fixture conditions.
import playwright from '/home/luyanfeng/luyanfeng/super-agent/node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js'
const { chromium } = playwright
const browser = await chromium.launch({ headless: true })
for (const name of ['before', 'after']) {
  const page = await browser.newPage({ viewport: { width: 760, height: 320 }, deviceScaleFactor: 2 })
  await page.goto(`file://${process.cwd()}/evidence/feishu-badge-tidy/${name}.html`)
  await page.screenshot({ path: `${process.cwd()}/evidence/feishu-badge-tidy/${name}.png`, fullPage: true })
  await page.close()
}
await browser.close()
console.log('captured before.png and after.png')

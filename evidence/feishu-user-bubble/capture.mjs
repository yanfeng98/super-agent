// Capture the before/after production-component renders written by
// packages/client/ui-chat/tests/capture-feishu-user-bubble.spec.tsx.
import playwright from '/home/luyanfeng/luyanfeng/super-agent/node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js'
const { chromium } = playwright
const browser = await chromium.launch({ headless: true })
for (const name of ['before', 'after']) {
  const page = await browser.newPage({ viewport: { width: 760, height: 320 }, deviceScaleFactor: 2 })
  await page.goto(`file://${process.cwd()}/evidence/feishu-user-bubble/capture-${name}.html`)
  await page.screenshot({ path: `${process.cwd()}/evidence/feishu-user-bubble/${name}.png`, fullPage: true })
  await page.close()
}
await browser.close()
console.log('captured before.png and after.png')

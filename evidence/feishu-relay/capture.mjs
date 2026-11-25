// Capture the before/after production-component renders written by
// packages/client/ui-chat/tests/capture-feishu-relay.spec.tsx.
import playwright from '/home/luyanfeng/.dsh/state/superagent/work/render-feishu-input-clean-fc1ec52cf35f/node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js'
const { chromium } = playwright
const browser = await chromium.launch({ headless: true })
for (const name of ['before', 'after']) {
  const page = await browser.newPage({ viewport: { width: 760, height: 300 }, deviceScaleFactor: 2 })
  await page.goto(`file://${process.cwd()}/evidence/feishu-relay/capture-${name}.html`)
  await page.screenshot({ path: `${process.cwd()}/evidence/feishu-relay/${name}.png`, fullPage: true })
  await page.close()
}
await browser.close()
console.log('captured before.png and after.png')

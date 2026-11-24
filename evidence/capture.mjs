// Capture a screenshot of the Feishu repair waiting view from a real browser.
// Usage: node evidence/capture.mjs <output.png>
import playwright from '/home/luyanfeng/.dsh/state/superagent/work/fix-feishu-repair-spinner-e3e90872158f/node_modules/.pnpm/playwright@1.61.1/node_modules/playwright/index.js';
const { chromium } = playwright;

const output = process.argv[2];
if (!output) throw new Error('usage: node evidence/capture.mjs <output.png>');

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 900, height: 760 }, deviceScaleFactor: 2 });
await page.goto(`file://${process.cwd()}/evidence/fixture.html`);
await page.waitForSelector('.bxf-provisionCard', { state: 'attached', timeout: 15_000 });
await page.click('.dim-accountSettingsToggle');
await page.waitForTimeout(700);
await page.screenshot({ path: output, fullPage: true });
await browser.close();
console.log(`captured ${output}`);

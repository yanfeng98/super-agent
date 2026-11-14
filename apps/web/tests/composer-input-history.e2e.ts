/** Folder-scoped composer input history recalled through the real browser gesture. */
import { join } from 'node:path'
import { chromium, type Browser } from 'playwright'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { launchWebScaffold, watchConsole, type WebScaffold } from './scaffold.ts'
import { connectFreshWorkspace, newEnglishPage } from './support.ts'

/** Browser-local key format owned by the composer history module. */
const STORAGE_PREFIX = 'dsh.conversation.input-history.v1.'

describe('web e2e: folder-scoped composer input history', () => {
  let scaffold: WebScaffold
  let browser: Browser

  beforeAll(async () => {
    scaffold = await launchWebScaffold({})
    browser = await chromium.launch()
  }, 120_000)

  afterAll(async () => {
    await browser?.close()
    await scaffold?.close()
  })

  it('recalls only the connected folder through ArrowUp and ArrowDown', async () => {
    const page = await newEnglishPage(browser)
    const tripwire = watchConsole(page)
    await page.goto(scaffold.authenticatedUrl, { waitUntil: 'load' })
    await connectFreshWorkspace(page, scaffold.workspaceCwd)
    const folder = join(scaffold.workspaceCwd, 'workspace')
    const otherFolder = join(scaffold.workspaceCwd, 'other-workspace')
    const entries = ['first prompt', '/compact']
    await page.evaluate(([currentKey, otherKey, current, other]) => {
      localStorage.setItem(currentKey, JSON.stringify(current))
      localStorage.setItem(otherKey, JSON.stringify(other))
    }, [`${STORAGE_PREFIX}${folder}`, `${STORAGE_PREFIX}${otherFolder}`, entries, ['other folder prompt']] as const)
    const input = page.locator('[data-composer-input]').first()
    await input.click()
    const draft = async (): Promise<string> => (await input.textContent()) ?? ''

    await page.keyboard.press('ArrowUp')
    await expect.poll(draft).toBe('/compact')
    // A recalled command must not open the trigger menu and steal this key.
    await page.keyboard.press('ArrowUp')
    await expect.poll(draft).toBe('first prompt')
    await page.keyboard.press('ArrowUp')
    await expect.poll(draft).toBe('first prompt')
    await page.keyboard.press('ArrowDown')
    await expect.poll(draft).toBe('/compact')
    await page.keyboard.press('ArrowDown')
    await expect.poll(draft).toBe('')
    expect(tripwire.pageErrors).toEqual([])
  })
})

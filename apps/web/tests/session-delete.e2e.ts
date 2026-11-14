// Permanent Session deletion through the sidebar: the hover button and the
// row menu both raise the confirmation, a live conversation releases its Agent
// through the same action, the Host removes the log, and a reload proves the
// removal is durable. This flow makes no model calls.
import { readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import type { Browser, Locator, Page } from 'playwright'
import { chromium } from 'playwright'
import { afterAll, beforeAll, describe, expect, it, onTestFailed } from 'vitest'
import type { SessionId } from '@deepseek-ai/dsh-session'
import {
  acknowledgeReloadConnectionLoss, launchWebScaffold, seedSession, watchConsole, type WebScaffold,
} from './scaffold.ts'
import { newEnglishPage, saveFailureShot } from './support.ts'
import { brandString } from '@deepseek-ai/dsh-brand'

// The seed is another scenario's committed fixture, reused read-only: this
// spec needs cold Session rows, not new recorded content.
const SEED = fileURLToPath(new URL('../../../snapshots/web/seeded-history/session.v3.jsonl', import.meta.url))
const BUTTON_ID = 'session-delete-button-web-e2e'
const MENU_ID = 'session-delete-menu-web-e2e'
const LIVE_ID = 'session-delete-live-web-e2e'

describe('web e2e: permanent Session deletion from the sidebar', () => {
  let scaffold: WebScaffold
  let browser: Browser
  let page: Page
  let tripwire: ReturnType<typeof watchConsole>

  /**
   * Expand the Ungrouped group when it renders collapsed so its rows are
   * addressable, then return the section holding them.
   * @returns the expanded Ungrouped section locator.
   */
  async function ungroupedSection(): Promise<Locator> {
    const header = page.getByText('Ungrouped', { exact: true })
    const groupRow = header.locator('..').locator('..')
    await expect.poll(async () => {
      if (await groupRow.count() === 0) return 'absent'
      if (await groupRow.getAttribute('aria-expanded') !== 'true') {
        await header.click()
        return 'collapsed'
      }
      return 'expanded'
    }, { timeout: 15_000 }).toBe('expanded')
    return groupRow.locator('..')
  }

  /**
   * Hover a row until one of its hover action buttons is visible.
   * @param row - the row owning the action.
   * @param name - accessible name of the action button.
   */
  async function revealHoverAction(row: Locator, name: string): Promise<Locator> {
    const button = row.getByRole('button', { name })
    await expect.poll(async () => {
      await row.hover()
      return await button.isVisible()
    }, { timeout: 10_000 }).toBe(true)
    return button
  }

  /**
   * Open the row's action menu, choose the permanent-delete item, and confirm.
   * @param row - Session row to delete.
   * @param title - the title whose menu trigger belongs to the row.
   */
  async function deleteThroughMenu(row: Locator, title: string): Promise<void> {
    await (await revealHoverAction(row, `Session actions for ${title}`)).click()
    await page.getByRole('menuitem', { name: 'Delete session…' }).click()
    await page.getByRole('dialog', { name: 'Delete this conversation permanently?' }).waitFor({ state: 'visible' })
    await page.getByRole('button', { name: 'Delete', exact: true }).click()
  }

  beforeAll(async () => {
    scaffold = await launchWebScaffold({})
    // Two cold sessions under the same cwd: one leaves through the hover
    // button, the other through the menu.
    await seedSession(scaffold, await readFile(SEED, 'utf8'), BUTTON_ID, undefined, { createdAt: Date.now() - 180_000 })
    await seedSession(scaffold, await readFile(SEED, 'utf8'), MENU_ID, undefined, { createdAt: Date.now() - 120_000 })
    await seedSession(scaffold, await readFile(SEED, 'utf8'), LIVE_ID, undefined, { createdAt: Date.now() - 60_000 })
    browser = await chromium.launch()
    page = await newEnglishPage(browser)
    tripwire = watchConsole(page)
    await page.goto(scaffold.authenticatedUrl, { waitUntil: 'load' })
    await page.waitForSelector('[class*="frame"]', { timeout: 30_000 })
  }, 120_000)

  afterAll(async () => {
    await browser?.close()
    await scaffold?.close()
  })

  it('deletes from the hover button and the row menu, and both removals survive a reload', async () => {
    onTestFailed(() => saveFailureShot(page, 'web-e2e-session-delete'))
    await ungroupedSection()
    // Establish a user-owned title so each row is addressable on both sides.
    const buttonTitle = `Delete button ${BUTTON_ID}`
    await scaffold.ctx.sessionController.rename({ sessionId: brandString<SessionId>(BUTTON_ID), title: buttonTitle })
    const menuTitle = `Delete menu ${MENU_ID}`
    await scaffold.ctx.sessionController.rename({ sessionId: brandString<SessionId>(MENU_ID), title: menuTitle })
    const liveTitle = `Delete live ${LIVE_ID}`
    await scaffold.ctx.sessionController.rename({ sessionId: brandString<SessionId>(LIVE_ID), title: liveTitle })

    const buttonSessionRow = page.getByRole('treeitem').filter({ has: page.getByText(buttonTitle, { exact: true }) })
    const menuSessionRow = page.getByRole('treeitem').filter({ has: page.getByText(menuTitle, { exact: true }) })
    const liveSessionRow = page.getByRole('treeitem').filter({ has: page.getByText(liveTitle, { exact: true }) })
    await expect.poll(() => buttonSessionRow.count(), { timeout: 10_000 }).toBe(1)
    await expect.poll(() => menuSessionRow.count(), { timeout: 10_000 }).toBe(1)
    await expect.poll(() => liveSessionRow.count(), { timeout: 10_000 }).toBe(1)

    // Opening a conversation activates its Agent, so this row is live and holds
    // the persistence write handle the deletion must release first.
    await liveSessionRow.click()
    await expect.poll(
      () => scaffold.ctx.agents.get(brandString<SessionId>(LIVE_ID)) !== undefined,
      { timeout: 20_000 },
    ).toBe(true)
    await deleteThroughMenu(liveSessionRow, liveTitle)
    await expect.poll(() => liveSessionRow.count(), { timeout: 15_000 }).toBe(0)
    await expect.poll(
      async () => await scaffold.ctx.sessionPersistence.stat(brandString<SessionId>(LIVE_ID)),
      { timeout: 10_000 },
    ).toBeUndefined()
    expect(scaffold.ctx.agents.get(brandString<SessionId>(LIVE_ID))).toBeUndefined()

    // Hover button: the confirmation names the conversation and warns about permanence.
    await (await revealHoverAction(buttonSessionRow, 'Delete session…')).click()
    const dialog = page.getByRole('dialog', { name: 'Delete this conversation permanently?' })
    await dialog.waitFor({ state: 'visible' })
    await dialog.getByText(`“${buttonTitle}” and its entire conversation history will be permanently deleted.`).waitFor()
    await page.getByRole('button', { name: 'Delete', exact: true }).click()
    await expect.poll(() => buttonSessionRow.count(), { timeout: 15_000 }).toBe(0)
    // The Host removed the stored Session, not merely the row.
    await expect.poll(
      async () => await scaffold.ctx.sessionPersistence.stat(brandString<SessionId>(BUTTON_ID)),
      { timeout: 10_000 },
    ).toBeUndefined()

    // Row menu: the same confirmation and the same durable removal.
    await deleteThroughMenu(menuSessionRow, menuTitle)
    await expect.poll(() => menuSessionRow.count(), { timeout: 15_000 }).toBe(0)
    await expect.poll(
      async () => await scaffold.ctx.sessionPersistence.stat(brandString<SessionId>(MENU_ID)),
      { timeout: 10_000 },
    ).toBeUndefined()
    expect([...scaffold.ctx.workspaceRegistry.archivedSessionIds]).not.toContain(BUTTON_ID)
    expect(scaffold.ctx.workspaceRegistry.list().flatMap(workspace => [...workspace.sessionIds]))
      .not.toContain(BUTTON_ID)

    // Reload: neither row is rebuilt from the Host baseline.
    const warningStart = tripwire.warnings.length
    await page.reload({ waitUntil: 'load' })
    await page.waitForSelector('[class*="frame"]', { timeout: 30_000 })
    acknowledgeReloadConnectionLoss(tripwire, warningStart)
    await expect.poll(() => page.getByText(buttonTitle, { exact: true }).count(), { timeout: 15_000 }).toBe(0)
    await expect.poll(() => page.getByText(menuTitle, { exact: true }).count(), { timeout: 15_000 }).toBe(0)
    await expect.poll(() => page.getByText(liveTitle, { exact: true }).count(), { timeout: 15_000 }).toBe(0)
    expect(tripwire.pageErrors).toEqual([])
  }, 120_000)
})

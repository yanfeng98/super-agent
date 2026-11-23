/** A Session's durable Workspace group hint moves its sidebar row without changing its cwd. */
import { mkdir, mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import type { Browser, Page } from 'playwright'
import { chromium } from 'playwright'
import { afterAll, beforeAll, describe, expect, it, onTestFailed } from 'vitest'
import type { SessionId } from '@deepseek-ai/dsh-session/types'
import type {} from '@deepseek-ai/dsh-api-session-controller'
import type {} from '@deepseek-ai/dsh-workspace'
import { appendWorkspaceGroup } from '@deepseek-ai/dsh-api-session-controller'
import { launchWebScaffold, type WebScaffold } from './scaffold.ts'
import { saveFailureShot, startWithSessionPanelHidden } from './support.ts'

const ORCHESTRATOR = 'workspace-group-orchestrator'
const BUSINESS_SESSION = 'workspace-group-member'
const SHOTS = fileURLToPath(new URL('../../../.artifacts/screenshots/workspace-group', import.meta.url))
/** `after` records the grouped-state sidebar capture for review; unset only asserts the behavior. */
const CAPTURE = process.env.DSH_CAPTURE_WORKSPACE_GROUP

/** A `turn/start` clears the blank flag that hides an unprompted Session from the sidebar. */
async function completeEmptyTurn(host: WebScaffold, sessionId: SessionId): Promise<void> {
  const resolved = await host.ctx.sessionController.resolveAgent(sessionId)
  if ('error' in resolved) throw resolved.error
  resolved.agent.session.append('turn/start', { turn: 1 })
  resolved.agent.session.append('turn/end', { turn: 1, reason: { kind: 'completed' } })
}

describe('sidebar workspace group hint', () => {
  let scaffold: WebScaffold
  let browser: Browser
  let page: Page
  let base: string
  let stateDir: string
  beforeAll(async () => {
    scaffold = await launchWebScaffold({})
    // The scaffold's own Workspaces would add empty sidebar groups unrelated to this scenario.
    for (const workspace of scaffold.ctx.workspaceRegistry.list()) await scaffold.ctx.workspaceRegistry.delete(workspace.id)
    base = await mkdtemp(join(tmpdir(), 'dsh-workspace-group-'))
    const botPath = join(base, 'super-agent')
    await mkdir(botPath, { recursive: true })
    stateDir = join(base, 'state')
    await mkdir(stateDir, { recursive: true })
    const bot = await scaffold.ctx.workspaceRegistry.create(botPath)
    const member = await scaffold.ctx.sessionController.create({ workspaceId: bot.id, sessionId: BUSINESS_SESSION as SessionId })
    await completeEmptyTurn(scaffold, member.sessionId)
    await scaffold.ctx.sessionController.rename({ sessionId: member.sessionId, title: '发布前检查' })
    // The orchestrator works in the superagent state directory, which owns no Workspace row.
    const orchestrator = await scaffold.ctx.sessionController.create({ cwd: stateDir, sessionId: ORCHESTRATOR as SessionId })
    await completeEmptyTurn(scaffold, orchestrator.sessionId)
    await scaffold.ctx.sessionController.rename({ sessionId: orchestrator.sessionId, title: 'dsh-superagent orchestrator' })
    const resolved = await scaffold.ctx.sessionController.resolveAgent(ORCHESTRATOR as SessionId)
    if ('error' in resolved) throw resolved.error
    appendWorkspaceGroup(resolved.agent.session, { path: bot.path, role: 'orchestrator' })
    browser = await chromium.launch()
    page = await browser.newPage({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN', timezoneId: 'Asia/Shanghai' })
    await startWithSessionPanelHidden(page)
    await page.goto(scaffold.authenticatedUrl, { waitUntil: 'load' })
  })

  afterAll(async () => {
    await browser?.close()
    await scaffold?.close()
    await rm(base, { recursive: true, force: true })
  })

  it('lists the hinted Session inside its Workspace group, marked as the orchestrator', async () => {
    onTestFailed(() => saveFailureShot(page, 'workspace-group'))
    const sidebar = page.locator('[class*="sidebarCol"]').first()
    await sidebar.waitFor()
    const workspaceHeader = page.locator('[data-row-key^="workspace:"]').first()
    await workspaceHeader.waitFor()
    const orchestratorRow = page.locator(`[data-row-key="session:${ORCHESTRATOR}"]`)
    // The group starts folded, and a late Workspace baseline can fold it again;
    // keep opening it until the hinted row is rendered.
    await expect.poll(async () => {
      if (await orchestratorRow.isVisible()) return true
      await workspaceHeader.click()
      return orchestratorRow.isVisible()
    }, { timeout: 30_000, interval: 500 }).toBe(true)
    await page.locator(`[data-row-key="session:${BUSINESS_SESSION}"]`).waitFor()
    const section = orchestratorRow.locator('xpath=ancestor::*[contains(@class,"groupSection")]')
    expect(await section.locator('[data-row-key^="workspace:"]').first().getAttribute('data-row-key'))
      .toBe(await workspaceHeader.getAttribute('data-row-key'))
    expect(await section.getByText('orchestrator', { exact: true }).count()).toBe(1)
    // The empty Other sessions bucket stays hidden.
    expect(await page.getByText('其他会话').count()).toBe(0)
    if (CAPTURE !== undefined) {
      await mkdir(SHOTS, { recursive: true })
      await sidebar.screenshot({ path: join(SHOTS, `${CAPTURE}.png`), animations: 'disabled' })
    }
  })
})

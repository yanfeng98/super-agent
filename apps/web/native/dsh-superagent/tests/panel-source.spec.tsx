// @vitest-environment jsdom
// The Source status and settings.
import { act } from 'react'
import { expect, it, onTestFinished, vi } from 'vitest'
import { zh } from '../src/client/locales.ts'
import { usePanel, mount, remount, settle, all, find, tab, click, type, text, field, at } from './fixtures/panel.tsx'
import { server, problemOf, droppedReply } from './fixtures/panel-server.ts'

usePanel()
it('shows each Source\'s state on the Source status and opens the deployment settings from a setup problem', async () => {
  await mount()
  await click(zh['tab.source'])
  expect(document.querySelector('.dt-status-title')?.textContent).toBe(zh.statusTitle)
  expect(all('.dt-form > .dt-list > .dt-row .dt-row-name').map(title => title.textContent)).toEqual(['飞书', '本地工单'])
  expect(all('.dt-form > .dt-list > .dt-row')[0]!.querySelector('.dt-row-meta')?.textContent).toBe('24 小时到达 12被过滤 2最近 ' + document.querySelector('.dt-row-meta time, .dt-row-meta .dt-lv:last-child .dt-lv-value')?.textContent)
  expect(text()).toContain(zh['sourceState.error'])
  expect(text()).toContain('socket closed')
  expect(all('.dt-card')).toHaveLength(0)
  server.setup = { ...server.setup, problem: problemOf('environmentFailed'), accounts: null }
  await remount()
  await click(zh['tab.source'])
  expect(text()).toContain(`部署检查未通过：${zh['setup.environmentFailed']}`)
  await click(find(zh.setupFixLink, 'button, a, [role="button"]'))
  expect(tab(zh['tab.settings']).getAttribute('aria-selected')).toBe('true')
  expect(document.querySelector('.dt-settings-link[aria-current="page"]')?.textContent).toBe(zh['settingsGroup.deployment'])
})

it('folds more than six chats into a line of their names', async () => {
  const chats = Array.from({ length: 7 }, (_, n) => ({ chatId: n ? `oc_c${n}` : 'oc_team', listen: 'all' as const }))
  server.settings.sources.feishu = { ...server.settings.sources.feishu!, config: { ...server.settings.sources.feishu!.config, chats } }
  await mount()
  await click(zh['tab.source'])
  await click(find(zh['tab.settings'], '.dt-status-head button'))
  const summary = all('.dt-card')[0]!.querySelector('.dt-rows-summary')!
  expect(summary.textContent).toBe(['团队群', 'oc_c1', 'oc_c2', 'oc_c3', 'oc_c4', 'oc_c5', 'oc_c6'].join('、'))
})

it('rereads the chats the bot has joined on request and says which are new, keeping unsaved edits', async () => {
  await mount()
  await click(zh['tab.source'])
  await click(find(zh['tab.settings'], '.dt-status-head button'))
  const feishu = () => all('.dt-card')[0]!
  const button = () => [...feishu().querySelectorAll('button')].find(item => item.textContent === '刷新群列表')!
  await click([...feishu().querySelectorAll('button')].find(item => item.textContent === '添加')!)
  server.joining = { oc_new: '新群' }
  await click(button())
  expect(server.requests.find(request => request.path === 'sources/feishu/labels')).toMatchObject({ method: 'POST' })
  expect(document.querySelector('.dt-toast')?.textContent).toBe('列表已更新，新增：新群')
  // The row being added stays, and now offers the new chat.
  const pick = feishu().querySelectorAll('.dt-rows-item')[1]!.querySelector<HTMLSelectElement>('select[aria-label="选择机器人已加入的群"]')!
  expect([...pick.options].map(option => option.textContent)).toEqual(['选择机器人已加入的群', '另一个群', '新群'])
  await click(button())
  expect([...document.querySelectorAll('.dt-toast')].at(-1)?.textContent).toBe(zh.labelsCurrent)
  server.joinError = 'dsh-im: The bot is not handed over'
  await click(button())
  expect([...document.querySelectorAll('.dt-toast')].at(-1)?.textContent).toBe('读取失败，列表保持不变：dsh-im: The bot is not handed over')
  expect(button().disabled).toBe(false)
  // Only a field whose key column offers a refresh shows the button.
  expect([...all('.dt-card')[1]!.querySelectorAll('button')].some(item => item.textContent === '刷新群列表')).toBe(false)
})

it('renders each Source\'s settings from its kind\'s fields and saves one Source against its own revision', async () => {
  await mount()
  await click(zh['tab.source'])
  await click(find(zh['tab.settings'], '.dt-status-head button'))
  const cards = all('.dt-card')
  expect(cards.map(card => card.querySelector('.dt-row-title')?.firstChild?.textContent)).toEqual(['飞书', '本地工单'])
  const feishu = cards[0]!
  expect(feishu.textContent).toContain('接收的群')
  await click([...feishu.querySelectorAll('button')].find(button => button.textContent === '添加')!)
  const rows = feishu.querySelectorAll('.dt-rows-item')
  // A saved chat shows its name; a row being added picks from the joined chats not yet configured.
  expect(rows[0]!.querySelector('.dt-row-label')?.textContent).toBe('团队群')
  expect(rows[0]!.querySelector('select[aria-label="选择机器人已加入的群"]')).toBeNull()
  const pick = rows[1]!.querySelector<HTMLSelectElement>('select[aria-label="选择机器人已加入的群"]')!
  expect([...pick.options].map(option => option.textContent)).toEqual(['选择机器人已加入的群', '另一个群'])
  await type(pick, 'oc_other')
  await type(rows[1]!.querySelector('select[aria-label="接收"]')!, 'all')
  await click('接收私聊')
  await act(async () => { feishu.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })) }); await settle()
  expect(server.requests.find(request => request.path === 'settings/source:feishu')).toMatchObject({ method: 'PUT', body: { revision: 0,
    value: { enabled: true, config: { chats: [{ chatId: 'oc_team', listen: 'mentions' }, { chatId: 'oc_other', listen: 'all' }],
      directMessages: false } } } })
  expect(server.settings.sources.feishu!.config).toMatchObject({ directMessages: false })
  const local = all('.dt-card')[1]!
  await click('运行来源：本地工单')
  server.revisions['source:local'] = 5
  await act(async () => { local.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })) }); await settle()
  // Another save changed nothing this draft changed, so the draft moves onto the newer revision and saves from there.
  expect(local.querySelector('.dt-savebar')?.textContent).toContain('设置已在别处修改，你的修改已合并到最新版本')
  await act(async () => { local.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })) }); await settle()
  expect(server.settings.sources.local).toMatchObject({ enabled: false })
  expect(server.revisions['source:local']).toBe(6)
})

it('shows the bot each dsh-im account signs in as, hides its fact on an unsaved edit, and offers the advanced fields', async () => {
  server.joined = { bot_1: 'Widgets Bot', bot_2: 'Other Bot' }
  await mount()
  await click(tab(zh['tab.settings']))
  const feishu = all('.dt-card')[0]!
  expect(feishu.textContent).toContain('当前机器人 Widgets Bot')
  expect(field<HTMLSelectElement>('IM 飞书机器人').value).toBe('bot_1')
  // A fact describes the saved value, so an edited bot shows none until it is saved.
  await type(field<HTMLSelectElement>('IM 飞书机器人'), 'bot_2')
  expect(feishu.textContent).not.toContain('当前机器人 Widgets Bot')
  await click(zh.discard)
  expect(feishu.textContent).toContain('当前机器人 Widgets Bot')
  await click([...feishu.querySelectorAll<HTMLElement>('[data-disclosure-row]')].find(row => row.textContent?.includes('高级'))!)
  expect(field('每页消息数').closest('.dt-field')).not.toBeNull()
  expect(field('原消息链接的查询时限 / 毫秒').closest('.dt-field')).not.toBeNull()
  await type(field<HTMLSelectElement>('IM 飞书机器人'), 'bot_2')
  await act(async () => { feishu.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })) }); await settle()
  expect(server.settings.sources.feishu!.config).toMatchObject({ botId: 'bot_2' })
  expect(text()).toContain(zh.saved)
  expect(field<HTMLSelectElement>('IM 飞书机器人').value).toBe('bot_2')
  expect(field('IM 飞书机器人').matches(':disabled')).toBe(false)
  expect(sessionStorage.getItem('dsh-superagent/draft/master/source:feishu')).toBeNull()
})

it('points a setup problem at the settings that fix it', async () => {
  server.setup = { ...server.setup, problem: problemOf('environmentFailed'), accounts: null }
  await mount()
  expect(text()).toContain(zh.problemSetup)
  expect(text()).toContain(zh['setup.environmentFailed'])
  // Nothing runs while setup waits, so the Executor says so instead of counting.
  expect(text()).toContain(zh.executorWaitingSetup)
  expect(all('.dt-stat').map(stat => stat.textContent)).not.toContain('执行中2 / 3')
  expect(text()).toContain(zh['problemSetupFix.deployment'])
  await click(find(zh.problemSetup, 'button, a, [role="button"]'))
  expect(tab(zh['tab.settings']).getAttribute('aria-selected')).toBe('true')
  expect(document.querySelector('.dt-settings-link[aria-current="page"]')?.textContent).toBe(zh['settingsGroup.deployment'])
  expect(text()).toContain('部署检查未通过：准备智能体的运行环境失败。')
  // After a fix outside dsh-superagent, a check runs setup again.
  await click(zh.setupCheck)
  expect(server.requests.filter(request => request.path === 'setup/check')).toEqual([{ path: 'setup/check', method: 'POST' }])
  expect(text()).toContain(zh.setupChecking)
})

it('leaves the Feishu skill out when no Feishu bot is configured', async () => {
  server.setup = { ...server.setup, accounts: { feishu: null } }
  at({ page: 'settings', query: { group: 'prompts' } })
  await mount()
  await click(tab(zh.promptViewFull))
  expect([...field<HTMLSelectElement>(zh.promptPreviewTarget).options].map(option => option.textContent)).toEqual(['主 Session',
    '执行器 Session'])
})

it('shows a Source\'s health in the interface language, and a diagnostic as reported', async () => {
  server.health = { feishu: { state: 'error', detail: 'Another consumer already owns this Feishu bot', code: 'imConsumerTaken' },
    history: { state: 'error', detail: 'history read failed: EOF' } }
  await mount()
  await click(zh['tab.source'])
  const feishu = all('.dt-form > .dt-list > .dt-row').find(row => row.querySelector('.dt-row-name')?.textContent === '飞书')!
  expect(feishu.textContent).toContain('飞书来源：另一个消费者已经接管了这个飞书机器人')
  expect(feishu.textContent).not.toContain('Another consumer')
  // A health entry the panel has no words for shows the detail the host reported.
  expect(feishu.textContent).toContain('history read failed: EOF')
  const local = all('.dt-form > .dt-list > .dt-row').find(row => row.querySelector('.dt-row-name')?.textContent === '本地工单')!
  expect(local.querySelector('.dt-mono')).toBeNull()
})

it('lists the messages a Source did not send in the last day under it, newest first, with why and where they belong', async () => {
  // Include a drop older than 24h (m0) and one with a missing thread (m1).
  server.dropped = [droppedReply('m3', 1, 'channelGone', { ticket: { id: 't1', title: '修复回归' } }),
    droppedReply('m2', 3, 'channelIgnored'), droppedReply('m1', 4, 'channelGone', { thread: false }),
    droppedReply('m0', 30, 'channelGone')]
  await mount()
  await click(zh['tab.source'])
  const [feishu, local] = all('.dt-form > .dt-list > .dt-row')
  const disclosure = (scope: Element) =>
    [...scope.querySelectorAll<HTMLElement>('[data-disclosure-row]')].find(row => row.textContent?.includes(zh.sourceDroppedTitle))!
  await click(disclosure(feishu!))
  const rows = [...feishu!.querySelectorAll('.dt-disclosure-body .dt-row')].map(row => [row.querySelector('.dt-row-title')?.textContent,
    row.querySelector('.dt-tag')?.textContent, row.querySelector('button')?.textContent])
  expect(rows).toEqual([['Reply m3', zh['reason.channelGone'], '修复回归'],
    ['Reply m2', zh['reason.channelIgnored'], zh['term.thread']], ['Reply m1', zh['reason.channelGone'], undefined]])
  expect(local!.textContent).not.toContain(zh.sourceDroppedTitle)
  await click(find('修复回归', '.dt-disclosure-body button'))
  expect(tab(zh['tab.execution']).getAttribute('aria-selected')).toBe('true')
  await remount()
  await click(zh['tab.source'])
  await click(disclosure(document.body))
  // A Feishu Channel names its chat; the link opens the Thread of the message it answers, which the host found.
  await click(find(zh['term.thread'], '.dt-disclosure-body button'))
  expect(server.requests.some(request => request.path.startsWith('threads/feishu:oc_1:om_root'))).toBe(true)
  expect(text()).not.toContain(zh.threadNotFound)
})

it('takes no space for messages that were all sent', async () => {
  await mount()
  await click(zh['tab.source'])
  expect(text()).not.toContain(zh.sourceDroppedTitle)
})

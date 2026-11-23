import { describe, expect, it, vi } from 'vitest'
import { CliError, Commands, commandError } from '../src/infra/commands.ts'
import { feishuImKind } from '../src/sources/feishu-im.ts'
import { key } from '../src/core/index.ts'
import { testDeployment } from './fixtures/deployment.ts'
const deployment = testDeployment()
describe('account-isolated operations', () => {
  it('runs every command with its environment over the process environment, where undefined removes a variable', async () => {
    process.env.DSH_SUPERAGENT_PROBE_TOKEN = 'personal'
    try {
      const output = await new Commands({ DSH_SUPERAGENT_PROBE_TOKEN: undefined, DSH_SUPERAGENT_PROBE: 'agent' },
        new AbortController().signal).run('env', [])
      expect(output).toContain('DSH_SUPERAGENT_PROBE=agent')
      expect(output).not.toContain('DSH_SUPERAGENT_PROBE_TOKEN=')
    } finally { delete process.env.DSH_SUPERAGENT_PROBE_TOKEN }
  })
})

it('reports a CLI failure with its code and without command arguments', () => {
  const membership = commandError({stderr:'[page 1] fetching...\n'+JSON.stringify({error:{code:232011,message:'operator is not in the chat'}})})
  expect(membership).toMatchObject({ message: 'operator is not in the chat (232011)', code: 232011 })
  expect(commandError({stderr:JSON.stringify({error:{code:999,message:'scope missing'}})}).message).toBe('scope missing (999)')
})
describe('Channels', () => {
  it('link each address to its chat page with its label', () => {
    const message = { chatId: 'oc_1', messageId: 'om_1' }
    expect(feishuImKind.channel!.link(message)).toEqual({ url: 'https://applink.feishu.cn/client/chat/open?openChatId=oc_1',
      label: { zh: '飞书消息', en: 'Feishu message' }, triggerId: key('feishu', 'om_1') })
  })
})
it('names only the program and how it ended when a command prints no diagnostics, never its arguments', async () => {
  const commands = new Commands({}, new AbortController().signal)
  const secret = 'secret reply text'
  await expect(commands.run('sh', ['-c', 'exit 3', secret])).rejects.toMatchObject({ message: 'sh exited with code 3', exitCode: 3 })
  await expect(commands.run('sh', ['-c', 'kill -9 $$', secret])).rejects.toMatchObject({ message: 'sh was stopped by SIGKILL' })
  await expect(commands.run('/no/such/program', ['secret reply text'])).rejects.toMatchObject({ message: 'program could not run: ENOENT' })
  // Timeouts kill the child with SIGTERM; Node's default error message would echo every argument.
  const timedOut = { killed: true, signal: 'SIGTERM', code: null, stderr: '',
    message: 'Command failed: /path/lark-bot im +messages-reply --markdown withdrawn' }
  expect(commandError(timedOut, '/path/lark-bot')).toMatchObject({ message: 'lark-bot timed out after 90 s' })
  expect(String(commandError(timedOut, '/path/lark-bot'))).not.toContain('withdrawn')
})
it('reports failures as CliError with the code the CLI names and the exit status', async () => {
  const notFound = commandError({ stderr: 'Not Found (HTTP 404)\n', code: 1 })
  expect(notFound).toBeInstanceOf(CliError)
  expect(notFound).toMatchObject({ message: 'Not Found (HTTP 404)', code: 404, exitCode: 1 })
  expect(commandError({ stderr: JSON.stringify({ error: { code: 230011, message: 'The message is recalled' } }),
    code: 1 })).toMatchObject({ message: 'The message is recalled (230011)', code: 230011 })
  // A cancelled command stays an AbortError so callers can distinguish cancellation from failure.
  const aborted = Object.assign(new Error('The operation was aborted'), { name: 'AbortError', stderr: '' })
  expect(commandError(aborted)).toBe(aborted)
  const controller = new AbortController()
  const running = new Commands({}, controller.signal).run('sleep', ['5'])
  controller.abort()
  await expect(running).rejects.toMatchObject({ name: 'AbortError' })
})

it('stops a running command when the signal of its work aborts, or the lifetime, and leaves the other work running', async () => {
  const lifetime = new AbortController(), work = new AbortController()
  const commands = new Commands({}, lifetime.signal)
  const started = Date.now()
  const running = commands.withSignal(work.signal).run('sleep', ['5'])
  work.abort()
  await expect(running).rejects.toThrow()
  // Only commands bound to the aborted work signal stop.
  expect(await commands.run('echo', ['still'])).toBe('still\n')
  const other = commands.withSignal(new AbortController().signal).run('sleep', ['5'])
  lifetime.abort()
  await expect(other).rejects.toThrow()
  expect(Date.now() - started).toBeLessThan(4000)
})

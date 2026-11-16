// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { makeTranslate } from '@deepseek-ai/dsh-client-test-runtime'
import { zh as commonZh } from '@deepseek-ai/dsh-client-locale/src/locales/zh.ts'
import type { ChatViewSlotProps, EditUserMessageRequest } from '../src/client/contract/slots.ts'
import { zh } from '../src/client/locale.ts'
import { EditMessageDialog } from '../src/client/chat/EditMessageDialog.tsx'

afterEach(cleanup)

const t: ChatViewSlotProps['t'] = makeTranslate(zh, commonZh)

const request: EditUserMessageRequest = {
  text: 'original message',
  boundarySeq: 4,
  runningTurn: false,
}

describe('edit-and-resend dialog', () => {
  it('prefills the message and submits the edited text', async () => {
    const onSubmit = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
    const onClose = vi.fn()
    render(<EditMessageDialog request={request} onClose={onClose} onSubmit={onSubmit} t={t} />)

    expect(screen.getByRole('dialog', { name: '编辑并重新发送' })).toBeTruthy()
    const editor = screen.getByRole('textbox', { name: '消息内容' }) as HTMLTextAreaElement
    expect(editor.value).toBe('original message')
    fireEvent.change(editor, { target: { value: 'corrected message\nsecond line' } })
    fireEvent.click(screen.getByRole('button', { name: '重新发送' }))

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledExactlyOnceWith('corrected message\nsecond line')
    })
    expect(onClose).not.toHaveBeenCalled()
  })

  it('disables resend for blank text', () => {
    const onSubmit = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
    render(<EditMessageDialog request={request} onClose={vi.fn()} onSubmit={onSubmit} t={t} />)

    fireEvent.change(screen.getByRole('textbox', { name: '消息内容' }), { target: { value: '   ' } })
    expect(screen.getByRole('button', { name: '重新发送' })).toHaveProperty('disabled', true)
  })

  it('submits with Ctrl+Enter and ignores plain Enter', async () => {
    const onSubmit = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
    render(<EditMessageDialog request={request} onClose={vi.fn()} onSubmit={onSubmit} t={t} />)
    const editor = screen.getByRole('textbox', { name: '消息内容' })

    fireEvent.keyDown(editor, { key: 'Enter' })
    expect(onSubmit).not.toHaveBeenCalled()

    fireEvent.keyDown(editor, { key: 'Enter', ctrlKey: true })
    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledExactlyOnceWith('original message')
    })
  })

  it('keeps the dialog open and reports a failed resend', async () => {
    const onSubmit = vi.fn<(text: string) => Promise<void>>().mockRejectedValue(new Error('prompt refused'))
    render(<EditMessageDialog request={request} onClose={vi.fn()} onSubmit={onSubmit} t={t} />)

    fireEvent.click(screen.getByRole('button', { name: '重新发送' }))

    const alert = await screen.findByRole('alert')
    expect(alert.textContent).toBe('重新发送失败：prompt refused')
    expect(screen.getByRole('dialog', { name: '编辑并重新发送' })).toBeTruthy()
    expect(screen.getByRole('button', { name: '重新发送' })).toHaveProperty('disabled', false)
    fireEvent.change(screen.getByRole('textbox', { name: '消息内容' }), { target: { value: 'try again' } })
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('blocks closing and duplicate submission while the resend is in flight', async () => {
    const pending = Promise.withResolvers<undefined>()
    const onSubmit = vi.fn<(text: string) => Promise<void>>(() => pending.promise)
    const onClose = vi.fn()
    render(<EditMessageDialog request={request} onClose={onClose} onSubmit={onSubmit} t={t} />)

    fireEvent.click(screen.getByRole('button', { name: '重新发送' }))
    expect(screen.getByRole('button', { name: '重新发送' })).toHaveProperty('disabled', true)
    expect(screen.getByRole('button', { name: '取消' })).toHaveProperty('disabled', true)
    fireEvent.click(screen.getByRole('button', { name: '关闭' }))
    expect(onClose).not.toHaveBeenCalled()
    expect(onSubmit).toHaveBeenCalledOnce()

    pending.resolve(undefined)
    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledOnce()
    })
  })

  it('closes on Cancel', () => {
    const onClose = vi.fn()
    render(<EditMessageDialog request={request} onClose={onClose} onSubmit={vi.fn(async () => {})} t={t} />)
    fireEvent.click(screen.getByRole('button', { name: '取消' }))
    expect(onClose).toHaveBeenCalledOnce()
  })
})

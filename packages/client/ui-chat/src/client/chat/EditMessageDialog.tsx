/** Modal editor for one sent user message that will be resent in a new branch. */

import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from 'react'
import { Button, Modal } from '@deepseek-ai/dsh-client-ui-primitives'
import type { ChatViewSlotProps, EditUserMessageRequest } from '../contract/slots.ts'
import css from './EditMessageDialog.module.css'

/** One deliberately ignored modal close while the resend is in flight. */
function ignoreClose(): void {}

function failureText(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

/** Props of the Chat-owned edit-and-resend dialog. */
export interface EditMessageDialogProps {
  readonly request: EditUserMessageRequest
  readonly onClose: () => void
  readonly onSubmit: (text: string) => Promise<void>
  readonly t: ChatViewSlotProps['t']
}

/**
 * Edit one sent user message and submit its replacement text.
 * @param props.request - original text and branch facts.
 * @param props.onClose - close request; ignored while submitting.
 * @param props.onSubmit - resend transaction; rejection keeps the dialog open.
 * @param props.t - Chat locale.
 * @returns the modal dialog.
 */
export function EditMessageDialog({ request, onClose, onSubmit, t }: EditMessageDialogProps) {
  const [text, setText] = useState(request.text)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const editor = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    setText(request.text)
    setBusy(false)
    setError(null)
  }, [request])

  useLayoutEffect(() => {
    const node = editor.current
    if (node === null) return
    node.setSelectionRange(node.value.length, node.value.length)
  }, [request])

  const submit = async (): Promise<void> => {
    if (busy || text.trim() === '') return
    setBusy(true)
    setError(null)
    try {
      await onSubmit(text)
    } catch (failure: unknown) {
      setError(t('message.edit.failed', { message: failureText(failure) }))
      setBusy(false)
    }
  }

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>): void => {
    if (event.key !== 'Enter' || event.shiftKey || (!event.metaKey && !event.ctrlKey)) return
    const composing = event.nativeEvent.isComposing || Reflect.get(event.nativeEvent, 'keyCode') === 229
    if (composing) return
    event.preventDefault()
    void submit()
  }

  const close = busy ? ignoreClose : onClose
  return (
    <Modal
      open
      onClose={close}
      closeLabel={t('close')}
      title={t('message.edit')}
      description={t('message.edit.description')}
      className={css.dialog ?? ''}
      footer={(
        <>
          <Button variant="outline" disabled={busy} onClick={onClose}>{t('cancel')}</Button>
          <Button variant="primary" disabled={busy || text.trim() === ''} onClick={() => { void submit() }}>
            {t('message.edit.submit')}
          </Button>
        </>
      )}
    >
      <textarea
        ref={editor}
        data-modal-autofocus
        className={css.editor}
        aria-label={t('message.edit.label')}
        rows={5}
        value={text}
        onChange={(event) => {
          setText(event.currentTarget.value)
          setError(null)
        }}
        onKeyDown={onKeyDown}
      />
      {error !== null && <p className={css.error} role="alert">{error}</p>}
    </Modal>
  )
}

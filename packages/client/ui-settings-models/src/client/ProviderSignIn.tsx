/**
 * Provider sign-in dialog: renders the attempt one card started — its notices,
 * its pending question, and its outcome — and sends the page's answers back.
 *
 * The component owns no subscription: the attempt arrives as a value from the
 * page's sign-in store, so a card can close and reopen, or the page can reload,
 * without losing a flow the host is still running.
 */

import { useState } from 'react'
import { Button, Modal } from '@deepseek-ai/dsh-client-ui-primitives'
import type { AuthorizationFlowView, AuthorizationPromptView } from '@deepseek-ai/dsh-api-remotes/client'
import type { ProviderSignInState } from './sign-in.ts'
import type { en } from './locales.ts'
import css from './ProviderSignIn.module.css'

/** What one card passes down from the page's sign-in store. */
export interface ProviderSignInView {
  /** The attempt as the page store holds it. */
  readonly attempt: ProviderSignInState
  /** Start, or adopt, one method of the flow. */
  begin(key: string, method: string | undefined): void
  /** Answer the question the attempt is waiting on. */
  answer(promptId: number, value: string): void
  /** Withdraw the attempt this page drove. */
  cancel(): void
  /** Forget the attempt so the next opening starts clean. */
  reset(): void
}

/** Props of {@link ProviderSignIn}. */
export interface ProviderSignInProps {
  /** Whether the dialog is showing. */
  open: boolean
  /** Provider display name for the dialog title. */
  displayName: string
  /** The host flow this card signs into. */
  flow: AuthorizationFlowView
  /** The page's sign-in actions and the attempt they drive. */
  signIn: ProviderSignInView
  /** Close the dialog and forget the attempt. */
  onClose: () => void
  /** Section copy. */
  t: (key: keyof typeof en) => string
  /** Accessible label of the close button. */
  closeLabel: string
}

/** Fill one `{provider}` placeholder without touching other braces. */
function withProvider(copy: string, provider: string): string {
  return copy.replace('{provider}', () => provider)
}

/**
 * One question's answer control. Mounted with the question's id as its key, so
 * a replacement question starts from an empty field instead of carrying the
 * previous answer across.
 */
function PromptAnswer({ prompt, submit, submitLabel, disabled }: {
  prompt: AuthorizationPromptView
  submit: (value: string) => void
  submitLabel: string
  disabled: boolean
}) {
  const [value, setValue] = useState('')
  if (prompt.kind === 'select') {
    return (
      <div className={css['options']} role="group" aria-label={prompt.message}>
        {prompt.options.map(option => (
          <Button
            key={option.id}
            variant="outline"
            disabled={disabled}
            onClick={() => { submit(option.id) }}
          >
            {option.description === undefined ? option.label : `${option.label} — ${option.description}`}
          </Button>
        ))}
      </div>
    )
  }
  return (
    <form
      className={css['answer']}
      onSubmit={(event) => {
        event.preventDefault()
        if (value.length === 0 || disabled) return
        submit(value)
      }}
    >
      <input
        className={css['input']}
        type={prompt.kind === 'secret' ? 'password' : 'text'}
        value={value}
        placeholder={prompt.placeholder}
        aria-label={prompt.message}
        autoComplete="off"
        spellCheck={false}
        disabled={disabled}
        onChange={(event) => { setValue(event.target.value) }}
      />
      <Button type="submit" disabled={disabled || value.length === 0}>{submitLabel}</Button>
    </form>
  )
}

/**
 * Render the current attempt of one provider's sign-in flow.
 * @param props - the flow, the attempt, the page's sign-in actions, and section copy.
 * @returns the dialog, portaled by {@link Modal}.
 */
export function ProviderSignIn({ open, displayName, flow, signIn, onClose, t, closeLabel }: ProviderSignInProps) {
  const { attempt } = signIn
  const prompt = attempt.prompt
  const busy = attempt.phase === 'starting'
  const working = attempt.phase === 'starting' || attempt.phase === 'running'
  const title = withProvider(t('signInTitle'), displayName)
  const close = (): void => {
    if (working) signIn.cancel()
    onClose()
  }
  return (
    <Modal
      open={open}
      onClose={close}
      title={title}
      closeLabel={closeLabel}
      description={t('signInIntro')}
      footer={(
        <>
          <Button variant="outline" onClick={close}>{working ? t('cancel') : t('close')}</Button>
          {attempt.phase === 'failed'
            ? <Button onClick={() => { signIn.begin(flow.key, flow.methods[0]?.id) }}>{t('signInRetry')}</Button>
            : null}
          {attempt.phase === 'authorized' ? <Button onClick={onClose}>{t('close')}</Button> : null}
        </>
      )}
    >
      <div className={css['body']}>
        {attempt.notices.length === 0
          ? (busy ? <p className={css['hint']}>{t('signInWaiting')}</p> : null)
          : (
            <ul className={css['notices']}>
              {attempt.notices.map((notice, index) => (
                <li key={`${index}-${notice.message}`} className={css['notice']}>
                  <span>{notice.message}</span>
                  {notice.code === undefined ? null : <code className={css['code']}>{notice.code}</code>}
                  {notice.url === undefined
                    ? null
                    : <a className={css['link']} href={notice.url} target="_blank" rel="noreferrer">{t('signInOpen')}</a>}
                </li>
              ))}
            </ul>
          )}
        {prompt === null
          ? null
          : (
            <div className={css['question']}>
              <p className={css['questionText']}>{prompt.message}</p>
              <PromptAnswer
                key={prompt.promptId}
                prompt={prompt}
                disabled={attempt.phase !== 'running'}
                submitLabel={t('signInSubmit')}
                submit={(value) => { signIn.answer(prompt.promptId, value) }}
              />
            </div>
          )}
        {attempt.phase === 'authorized' ? <p className={css['done']} role="status">{t('signInDone')}</p> : null}
        {attempt.failure === null
          ? null
          : (
            <div className={css['error']} role="alert">
              <p className={css['errorText']}>{t('signInFailed')}</p>
              {/* The host's own words: a refused attempt explains itself better
                  than any copy this page could invent for it. */}
              <p className={css['errorDetail']}>{attempt.failure}</p>
            </div>
          )}
      </div>
    </Modal>
  )
}

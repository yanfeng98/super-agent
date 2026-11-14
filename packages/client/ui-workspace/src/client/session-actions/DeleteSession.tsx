/** Permanent Session deletion: the row button, its confirmation, and the refusal mapping. */
import { useState } from 'react'
import type { SessionDeleteError } from '@deepseek-ai/dsh-api-session-controller/client'
import type { SessionActivity } from '@deepseek-ai/dsh-api-workspace-controller/client'
// Type-only: the family keys each provider merges; a key this program did not compile takes the generic line.
import type {} from '@deepseek-ai/dsh-agent/types'
import type {} from '@deepseek-ai/dsh-jobs/view'
import type {} from '@deepseek-ai/dsh-schedule/client'
import type {} from '@deepseek-ai/dsh-subagent/client'
import {
  Button, IconTrashOutlineRegular, Modal, Tooltip,
} from '@deepseek-ai/dsh-client-ui-primitives'
import type {
  DeleteSessionInjected, SessionDeleteConfirmProps, SessionDeleteTarget, SessionRowActionProps,
} from '../contract/slots.ts'
import css from '../rows/Rows.module.css'
import browserCss from '../rows/WorkspaceBrowser.module.css'
import { activityLine } from './ArchiveSession.tsx'

/**
 * Hover button (order 300): ask for the permanent-delete confirmation.
 * Absent while the Session's access snapshot excludes `session/delete`.
 * @param props - owner share, the access Hook, and the delete share. The
 *   action only raises the request; the dialog owns the deletion.
 * @returns the button, or nothing when the caller may not delete this Session.
 */
export function DeleteSessionRowButton({
  sessionId, displayTitle, requestSessionDelete, useSessionAccess, t,
}: SessionRowActionProps<DeleteSessionInjected>) {
  const allowed = useSessionAccess(sessionId, 'session/delete')
  if (!allowed) return null
  return (
    <Tooltip label={t('actions.delete')} side="bottom" align="end" delayMs={500} portal>
      <button
        type="button"
        className={css.iconButton}
        aria-label={t('menu.deleteSession')}
        onClick={() => { requestSessionDelete(sessionId, displayTitle) }}
      >
        <IconTrashOutlineRegular size={14} />
      </button>
    </Tooltip>
  )
}

/** One Host refusal, mapped to the copy and detail the dialog renders. */
type DeleteRefusal =
  | { readonly kind: 'active'; readonly activity: readonly SessionActivity[] }
  | { readonly kind: 'inUse' }
  | { readonly kind: 'notFound' }
  | { readonly kind: 'subagent' }
  | { readonly kind: 'other'; readonly message: string }

/**
 * Map a rejection onto the dialog's refusal arm. Client plugin bundles do not
 * share error-class identity, so the name decides; an unknown code keeps the
 * Host message for diagnosis.
 * @param reason - the rejected deletion call's reason.
 * @returns the refusal to render.
 */
function deleteRefusal(reason: unknown): DeleteRefusal {
  if (!(reason instanceof Error) || reason.name !== 'SessionDeleteError') {
    return { kind: 'other', message: reason instanceof Error ? reason.message : String(reason) }
  }
  const failure = reason as SessionDeleteError
  switch (failure.rpcError.code) {
    case 'session/active':
      return { kind: 'active', activity: failure.rpcError.details.activity }
    case 'session/in-use': return { kind: 'inUse' }
    case 'session/not-found': return { kind: 'notFound' }
    case 'session/subagent': return { kind: 'subagent' }
    default: return { kind: 'other', message: failure.rpcError.message }
  }
}

/**
 * The `shell.overlay` entry: nothing while no confirmation is pending,
 * otherwise one dialog per request (keyed by the Session). Confirming asks
 * the Host to remove the conversation permanently; cancelling leaves it.
 * @param props - the request hook, its settlement, the delete hop, and the locale seat.
 * @returns the open dialog, or null.
 */
export function SessionDeleteConfirmDialog({
  useDeleteRequest, settleSessionDelete, deleteSession, t,
}: SessionDeleteConfirmProps) {
  const request = useDeleteRequest(pending => pending)
  if (request === null) return null
  return (
    <DeleteConfirmForm
      key={request.sessionId}
      request={request}
      deleteSession={deleteSession}
      onSettle={settleSessionDelete}
      t={t}
    />
  )
}

/** One request's dialog: in-flight and refusal state die with it. */
function DeleteConfirmForm({ request, deleteSession, onSettle, t }: {
  request: SessionDeleteTarget
  deleteSession: SessionDeleteConfirmProps['deleteSession']
  onSettle: () => void
  t: SessionDeleteConfirmProps['t']
}) {
  const [deleting, setDeleting] = useState(false)
  const [refusal, setRefusal] = useState<DeleteRefusal | null>(null)
  const close = () => {
    if (deleting) return
    onSettle()
  }
  const confirm = () => {
    setDeleting(true)
    setRefusal(null)
    deleteSession(request.sessionId).then(() => {
      setDeleting(false)
      onSettle()
    }).catch((reason: unknown) => {
      setDeleting(false)
      setRefusal(deleteRefusal(reason))
    })
  }
  return (
    <Modal
      open
      onClose={close}
      closeLabel={t('close')}
      title={t('delete.confirm.title')}
      description={t('delete.confirm.desc', { title: request.displayTitle })}
      footer={(
        <>
          <Button variant="outline" disabled={deleting} onClick={close}>{t('cancel')}</Button>
          <Button
            variant="outline"
            className={browserCss.deleteAction}
            disabled={deleting}
            onClick={confirm}
          >
            {t('delete.confirm.action')}
          </Button>
        </>
      )}
    >
      {refusal !== null && <div className={browserCss.renameError} role="alert">{refusalCopy(refusal, t)}</div>}
      {refusal?.kind === 'active' && (
        <ul className={browserCss.archiveActivity} aria-label={t('delete.confirm.activity')}>
          {refusal.activity.map((entry, index) => (
            <li key={`${entry.kind}-${String(index)}`}>{activityLine(entry, t)}</li>
          ))}
        </ul>
      )}
      {deleting && <div className={browserCss.deleteStatus} role="status">{t('delete.confirm.pending')}</div>}
    </Modal>
  )
}

/**
 * Localized copy for one refusal.
 * @param refusal - the mapped Host refusal.
 * @param t - workspace locale seat.
 * @returns the sentence rendered above the activity list, or the Host message.
 */
function refusalCopy(refusal: DeleteRefusal, t: SessionDeleteConfirmProps['t']): string {
  switch (refusal.kind) {
    case 'active': return t('delete.confirm.active')
    case 'inUse': return t('delete.confirm.inUse')
    case 'notFound': return t('delete.confirm.notFound')
    case 'subagent': return t('delete.confirm.subagent')
    case 'other': return refusal.message
  }
}

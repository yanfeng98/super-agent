/**
 * Start a new conversation branch for one edited user message and send its
 * replacement text there. The source Session remains an immutable record.
 * @module @deepseek-ai/dsh-client-ui-chat/chat/edit-resend
 */

import type {
  BeginSubmissionInput, ISessions, SessionFace, SubmissionHandle,
} from '@deepseek-ai/dsh-api-session-controller/client'
import type { IWorkspaces } from '@deepseek-ai/dsh-api-workspace-controller/client'
import type { IConversation } from '@deepseek-ai/dsh-client-ui-conversation/client'
import type { SessionId } from '@deepseek-ai/dsh-session/types'
import type { EditUserMessageRequest } from '../contract/slots.ts'

declare module '@deepseek-ai/dsh-api-session-controller/client' {
  interface SessionReferenceSourceMap {
    /** Chat's temporary ownership while it admits the edited message into its branch. */
    chatEditResend: unknown
  }
}

/** Narrow Session face used by the edit transaction. */
export interface EditResendSessions {
  readonly list: Pick<ISessions['list'], 'getSnapshot'>
  readonly create: ISessions['create']
  readonly fork: ISessions['fork']
  readonly retain: (
    target: Parameters<ISessions['retain']>[0],
    options: Parameters<ISessions['retain']>[1],
  ) => EditResendReference
  readonly binding: (sessionId: SessionId) => EditResendBinding | undefined
}

/** Narrow binding lookup result needed by the edit transaction. */
export interface EditResendBinding {
  readonly session: EditResendSession
}

/** Narrow Session face consumed by the edit transaction. */
export interface EditResendSession {
  getSnapshot(): { readonly running: boolean }
  beginSubmission(input: BeginSubmissionInput): SubmissionHandle
  prompt: SessionFace['prompt']
  cancel: SessionFace['cancel']
}

/** Temporary ownership of the branch while the edited prompt is admitted. */
export interface EditResendReference {
  readonly binding: { readonly session: EditResendSession }
  release(): void
}

/** Narrow draft-initialization face used only to recover a refused edit. */
export interface EditResendConversation {
  readonly input: {
    requestDraftInitialization(
      binding: EditResendBinding,
      options: Parameters<IConversation['input']['requestDraftInitialization']>[1],
    ): ReturnType<IConversation['input']['requestDraftInitialization']>
  }
}

/** Dependencies of the edit-and-resend transaction. */
export interface EditResendDeps {
  readonly sessions: EditResendSessions
  /** Workspace catalog when this composition mounts the Workspace service. */
  readonly workspaces?: Pick<IWorkspaces, 'list'> | undefined
  /** Input recovery when this composition mounts the Conversation service. */
  readonly conversation?: EditResendConversation | undefined
  /** Reveal the created or forked Session. */
  readonly openSession: (sessionId: SessionId) => void
}

function errorOf(value: unknown): Error {
  return value instanceof Error ? value : new Error(String(value))
}

async function cancelRunningSource(sessions: EditResendSessions, sessionId: SessionId): Promise<void> {
  const binding = sessions.binding(sessionId)
  if (binding === undefined || !binding.session.getSnapshot().running) return
  const result = await binding.session.cancel()
  if (!result.ok) throw new Error(`${result.error.code}: ${result.error.message}`)
}

/**
 * Fork or create the edited message's new conversation, reveal it, and send
 * the replacement text. A failed admission leaves the text in the new
 * composer so the user can retry without retyping it.
 * @param deps - Session, Workspace, input, and navigation capabilities.
 * @param request - source Session, edited text, and branch facts.
 * @returns the created or forked Session identity.
 */
export async function resendEditedMessage(
  deps: EditResendDeps,
  request: EditUserMessageRequest & { readonly sessionId: SessionId },
): Promise<SessionId> {
  if (request.text.trim() === '') throw new Error('resendEditedMessage requires non-blank text')
  if (request.runningTurn) await cancelRunningSource(deps.sessions, request.sessionId)
  const source = deps.sessions.list.getSnapshot().byId[request.sessionId]
  let childId: SessionId
  if (request.boundarySeq === undefined) {
    const workspace = deps.workspaces?.list.getSnapshot().items.find(item =>
      item.sessionIds.includes(request.sessionId))
    const preset = source?.projectionValues?.agentPreset
    childId = await deps.sessions.create({
      ...workspace === undefined ? {} : { workspaceId: workspace.workspaceId },
      ...source?.cwd === undefined ? {} : { cwd: source.cwd },
      ...typeof preset === 'string' ? { agentPreset: preset } : {},
    })
  } else {
    childId = await deps.sessions.fork({
      sessionId: request.sessionId,
      atSeq: request.boundarySeq,
      increaseTitle: true,
    })
  }
  const reference = deps.sessions.retain(childId, { source: 'chatEditResend' })
  try {
    let submission: ReturnType<EditResendReference['binding']['session']['beginSubmission']> | undefined
    let failure: Error | undefined
    try {
      submission = reference.binding.session.beginSubmission({
        mode: 'queue',
        text: request.text,
        attachments: [],
      })
      const result = await reference.binding.session.prompt(
        [{ type: 'text', text: request.text }],
        'queue',
        undefined,
        submission.requestId,
      )
      if (!result.ok) failure = new Error(`${result.error.code}: ${result.error.message}`)
    } catch (error: unknown) {
      failure = errorOf(error)
    }
    if (failure !== undefined) {
      submission?.abandon()
      try {
        deps.openSession(childId)
      } catch {
        // The prompt failure remains the report; an unavailable reveal leaves the source view in place.
      }
      const binding = deps.sessions.binding(childId)
      if (binding !== undefined && deps.conversation !== undefined) {
        try {
          deps.conversation.input.requestDraftInitialization(binding, {
            prompt: request.text,
            whenDraftExists: 'keep',
          })
        } catch {
          // The prompt failure remains the report; the user can still retype in the source view.
        }
      }
      throw failure
    }
    deps.openSession(childId)
    return childId
  } finally {
    reference.release()
  }
}

# Agent Note: Permanent Session deletion

Status: implemented

English | [中文](2026-11-14-session-deletion.zh.md)

## Problem

A conversation could be hidden but never removed: `WorkspaceRegistry.archiveSession` writes a durable archive set, the log, attachments, and the Workspace account stay, and the package README recorded session deletion as an absent capability. Users accumulate conversations forever, and an archived mistake is one filter switch away from returning.

The storage seam had no removal operation, and removing a log safely is not a filesystem call. Opening a conversation in the Web GUI activates its Agent (`activate: true` on the follow stream), and `ApiSessionAgentController` discarded the returned `AgentHandle`, so every opened Session held its persistence write handle — and its `session.lock` lease — until the Host exited. A deletion that ignored that holder would unlink a log a live writer still appends to; a deletion that refused every live Session would refuse most rows a user sees.

## Decision

**The persistence seam gains one exclusive, complete removal.** `SessionPersistence.remove(id)` deletes a Session's entire artifact set, not one generation. Every backend must refuse while an open handle or pending creation holds the id in this process (`SessionPersistenceBusyError`), refuse cross-process while another Host holds the write lease, and reject an absent id (`SessionPersistenceNotFoundError`). The shipped JSONL backend implements that with its in-process tracker, the same `SessionWriteLease` append publication takes — held across the directory removal, so the lock file disappears with the log — and drops its header and cold-log memo entries. A test double that only serves reads must implement the method.

**The controller can now stop the Agents it published.** `ApiSessionAgentController` retains the handle returned by every `agents.create`/`agents.resume` it performs. `disposeAgent(sessionId)` waits for an in-flight resume, then disposes the retained handle, which stops the loop, closes the write handle, and removes the Session. `beginDeletion`/`endDeletion` fence new resumes and named Session creation for that identity while a removal runs, so a reconnecting client, a reminder, or a New Session reuse cannot republish a Session whose log is about to disappear. A live Agent without a retained handle (only possible after a dev reload that replaced this plugin while the Host kept its Agents), or a Session still attached to the store without an Agent this controller published, is reported as `session/in-use` instead of being deleted underneath its holder.

**`session/delete` orders admission before any destructive step.** The Host resolves the identity from the live store or `stat` (`session/not-found`), refuses subagent conversations (`session/subagent`, because the parent's catalog projection owns their rows), asks the existing `workspace/session-activity` waterfall and refuses a non-empty answer with the same activity detail archive uses (`session/active`), and only then fences, disposes, removes storage, and calls `workspaceRegistry.forgetSession`. Deletion is refused while work runs rather than stopping it, because deleting is irreversible: the user stops the turn first, exactly as they would before archiving without `stopActivity`. `forgetSession` drops the account slot, archive and pin membership, and cached header and path facts in two writes; a crash between them leaves only references the membership projection and the next mutation already tolerate, so no pending-mutation marker was added. A disposed Agent already emits `api-session/removed`; a cold deletion emits that edge directly, so every peer drops the row. A live Session that never materialized has no artifact, and its missing log confirms the deletion instead of failing it.

**The Client owns only presentation.** `ISessions.delete` calls the Remote and refreshes the authoritative list, throwing `SessionDeleteError` with the Host failure. The sidebar's trash hover button and the menu's danger-styled Delete session… item raise one confirmation dialog; confirming clears the selection when it named the deleted Session and raises a success notice, and the broadcast removal resets any other Client's view that still addressed it. No undo is offered because no copy remains; the refusal copy names the running work or the holder. Subagent rows never render the action (the sidebar already filters `origin === 'subagent'`), and the Host refusal covers direct API callers.

## Alternatives considered

**Treat deletion as an archive alias.** Lost: a log the user explicitly asked to destroy stays on disk, and the archived filter keeps resurfacing it. The product already had archive; naming a second hide as delete would misdescribe the data guarantee.

**Allow only cold Sessions.** Lost: every conversation opened in this Host run would be undeletable, because the Web opening path activates the Agent and the write handle lives until process exit. The usable action would degrade to "restart the Host, then delete", which is not what a delete button promises.

**Add `AgentRegistry.dispose(id)` as a second teardown owner.** Lost: the handle contract deliberately makes the creating consumer the only owner of the disposer, and a core lifecycle change for a GUI BFF need would require loop documentation and broader review. Retaining the handle where it is created keeps the capability with its existing owner.

**Cascade deletion to subagent descendants.** Lost: children are independent persisted conversations, often with their own transcripts the user may still read; a parent's deletion must not silently destroy them. They become root rows when their parent is gone, which `flattenLineage` already handles.

**Collect projection checkpoints, attachments, and schedule targets.** Lost: attachments and checkpoints may be shared with resumed or forked conversations, so per-Session collection is unsafe; retention policy owns them. Ended or paused schedule targets keep their id and the Schedule UI already reports an unavailable conversation.

## Consequences

Deletion is permanent and irreversible; the confirmation states that, and no undo notice exists. The Host may refuse while its work or holders are transient, so the dialog maps `session/active`, `session/in-use`, `session/not-found`, and `session/subagent` to localized copy instead of hiding the action. `SessionPersistence` gained a required abstract method, recorded in the upgrade guide for custom backends. After a development reload that replaces the Session Controller while the Host keeps its Agents, a live Session cannot be deleted until the Host restarts; production builds do not reload plugins. The seam deletes one Session's own artifacts only: projection-cache rows, in-memory reading records, inactive schedule targets, and orphaned subagent children remain, none of which is read as authoritative for the deleted identity.

Coverage: the shared persistence contract gained removal cases (visible removal, busy refusal, not-found, cross-instance reopen), the JSONL spec covers directory and lease deletion, the Workspace registry spec covers forget idempotence and restart, `delete.host.spec.ts` covers the refusal order and both live and cold paths, the Client specs cover the row button, dialog arms, selection clearing, and notice, and `apps/web/tests/session-delete.e2e.ts` proves both entry points, a live conversation releasing its Agent, and durability across reload.

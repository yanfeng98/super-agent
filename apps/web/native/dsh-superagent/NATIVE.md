# Native dsh-superagent

This directory is the source-vendored copy of [`yanfeng98/nano-dsh-superagent-prototype`](https://github.com/yanfeng98/nano-dsh-superagent-prototype)
that ships with this Super Agent checkout as a built-in capability. It is not installed from npm and it is not a third-party
profile plugin.

It is a proactive coding agent that takes requests from local tickets created in its panel or by its main Session, or from
Feishu when a bot is configured, evaluates them under a Policy, turns authorized ones into Tickets, and runs each Ticket in
its own DSH Session and directory. Every
local ticket is archived as a Markdown file that also receives the agent's replies and results. The **Superagent** entry in
the Web sidebar opens its panel.

## Baseline

- Upstream commit: `8c32a9d` (2026-10-31, "feat: add the dsh-superagent research prototype")
- Package version: `0.1.0`
- License: MIT; the upstream `LICENSE` is preserved here.
- Upstream docs, prompts, skills, examples, locale files and tests are kept unchanged unless listed below.

## How it is mounted

- `pnpm-workspace.yaml` lists `apps/web/native/dsh-superagent`.
- `packages/bundle/web-app/package.json` depends on `dsh-superagent` as a workspace package.
- `packages/bundle/web-app/cordis.patch.yml` inserts the `dsh-superagent` row (Host plugin plus the browser panel from its
  `dsh.client` declaration), the `local` Source, and the `feishu-watch` Source (disabled).
- `packages/bundle/web-app/presets/superagent-worker.patch.yml` and `superagent-router.patch.yml` define the two agent presets
  its Sessions run under (`dsh-superagent-worker`, `dsh-superagent-router`); the Web bundle lists them in `dsh.bundle.patch`.
  Their sandbox rows use `superagent-` ids because a preset row id must not repeat a host row's.
- `packages/bundle/web-app/tests/superagent-bundle.spec.ts` guards the dependency, the mounted rows and both presets.
- `scripts/build.ts` runs the package's own `tsdown` build before the Web shell build, producing `lib/index.js`,
  `lib/client.js`, `lib/feishu-watch.js` and `lib/local.js`.
- No profile edit, plugin-market install, or `dsh plugin add` is needed. A fresh clone needs the usual
  `pnpm install && pnpm run build`, then `pnpm dsh web` (or Desktop).

The runtime starts without a repository or an account: the local Source accepts tickets from **Execution → New ticket** or
from the main Session in the DSH chat, and Feishu stays off until a bot CLI is configured in the panel (or with the
Management API of `docs/006-deployment.md`).

The state directory defaults to `<DSH_HOME>/state/superagent` (with `DSH_HOME` defaulting to `~/.dsh`), so the built-in
copy starts without machine configuration. Set `stateDir` in the profile's `cordis.patch.yml` to relocate it.

## Local modifications from upstream

1. **Workspace package.** `package.json` is private, declares the `@deepseek-ai/*` peer packages as `workspace:*`
   dependencies, pins the repository's `typescript`, `tsdown`, `vitest`, `react` and `jsdom` ranges, and drops the
   `dsh.bundle` declaration (the Web bundle mounts the rows instead). The client `inject` list names the 0.2.2 client
   packages the panel needs.
2. **Default state directory.** `src/deployment.ts` defaults `stateDir` to `<DSH_HOME>/state/superagent` through
   `defaultStateDir()`, so the plugin starts in a fresh profile without configuration; the absolute-path check accepts POSIX
   roots, Windows drive roots and UNC paths. `tests/deployment.spec.ts` covers the default.
3. **DSH 0.2.1-alpha.2 → 0.2.2 API adaptation:**
   - `tests/bundle-patch.spec.ts` reads the patched entry list from `applyEntryPatches(...).data`; Loader 1.0.0-rc.9
     returns `{ data, index }`, and Include 1.2.0 parses the patch as YAML 1.2.
   - Session persistence gained `remove`, which the SQLite backend refuses; the plugin deletes nothing and needs no change.
   - The `working_directory` model tool and the `subagent` `cwd` parameter are gone; the preset rows already name neither,
     and `enter_worktree`/`exit_worktree` replace `create_worktree`.
   - `src/app/resources.ts` reads a Ticket's transcript from the live Agent's Session (`ctx.agents.get(...).session.snapshotEvents()`),
     and falls back to the stored log through `sessionPersistence.open(id, 'read')` when an earlier process left the Session
     unloaded; the 0.2.1 `sessionController.inspect()` method no longer exists.
   - `src/client/deployment.tsx` reads the read-only hint from `ConfigFormSnapshot.writable`; the snapshot's `mode` field was
     removed.
   - `tests/role-models.spec.ts` exercises the 0.2.2 model-selection flow: the role model is assembled per request, and a
     later manual selection wins after the next `system-prompt/assemble`.
   - `tests/runtime.spec.ts` brands the emitted `SessionId` for `agent-preset/selected`; the event signature is typed.
   - `tests/bundle-patch.spec.ts` parses `cordis.patch.yml` with Include's own `parseEntryYaml`, which owns the `!!js` tag.
   - `tests/fixtures/panel-server.ts` and `tests/panel-deployment.spec.tsx` use the 0.2.2 snapshot fields (`writable`, no
     `mode`).
   - `src/app/http.ts` tolerates a Web server whose `port` is not assigned yet, and follows the bound Web host
     (loopback aliases included) instead of refusing every address but `127.0.0.1`; DSH's Web authentication, the
     `x-orchestrator-request` header and the same-origin check still protect it.
4. **Feishu consumer health.** `src/sources/feishu-loops.ts` no longer lets a `[event] ready` line overwrite an error that
   the event pipe already reported: the two pipes are read independently, so a malformed event line can arrive first. The
   error stays visible until the consumer reconnects.
5. **Management API bind.** `src/app/http.ts` follows the Web server's bound host (loopback aliases included) instead of
   requiring `127.0.0.1`, so a container or LAN bind keeps its panel. DSH's Web authentication, the
   `x-orchestrator-request` header and the same-origin check still protect every route.
6. **Local tickets.** `src/sources/local.ts` (exported as `./local`) adds a panel-driven Source: the
   management routes `POST local/requests` and `POST local/followups` write `<archive>/<request id>.md`, ingest Triggers whose
   actor is `operator`, and append every outbound message to the same file; the ticket page lists those receipts under
   **Replies and results** and offers a reply box for the follow-ups (`src/app/views/tickets.ts`, `src/app/store.ts`). Setup no longer requires a repository and an account: `resolveFeishuBot` resolves only the optional Feishu bot and
   `writeEnvironment` writes the command shims and the neutral author `dsh-superagent <dsh-superagent@localhost>`. Receipts record the time of their last delivery state, and the overview counts a kind's default
   Policy part (`src/app/views/overview.ts`) so a fresh install is not told that nobody may ask. `local.spec.ts`,
   `deployment.spec.ts`, `environment.spec.ts`, `http.spec.ts`, `views.spec.ts` and `panel-execution.spec.tsx` cover it.
7. **Local-only worker prompt.** `prompts/work.md`, `prompts/worker.md` and `prompts/team/delivery.md` tell a worker that there is no remote account, so it commits locally (setting `user.name` and `user.email` itself when git has none), does not push, and reports ready without waiting for a merge; `prompts/orchestrator.md` says the main Session merges or undoes a merge only on the person's explicit request.
9. **Ticket review, reopening and searchable results.** The Executor records a Ticket workspace's base commit at its first
   launch (`src/executor/dsh.ts`); when the Ticket ends, Runtime captures the branch, the commits since that base, the changed
   files and any uncommitted work (`src/app/review.ts`) onto `ticket.review`, and the local archive appends a **Closed**
   section with the outcome, summary and those facts (`src/sources/local.ts`). `POST tickets/:id/reopen` and the panel's
   **Reopen** action restore an ended Ticket under its still-valid Mandate unless its branch was merged or its worktree was
   discarded or cleaned up (`src/core/tickets.ts`, `src/core/engine.ts`,
   `src/app/http.ts`, `src/client/ticket.tsx`), and the archive records the reopening; a person's message that resumes an ended
   Ticket clears its previous review, check and review decision, while a merged, discarded or cleaned Ticket does not resume and
   its message goes back to Eval; the Changes section reads the committed diff through `GET tickets/:id/diff` (`src/app/review.ts`, `src/client/ticket.tsx`). The new-ticket form sends an
   idempotency key, so a retry after a lost response returns the same Ticket; panel search matches the request text, a
   Ticket's latest result, its replies and its labels, and accepts `is:`, `tag:`, `label:`, `author:`, `created:` and
   `updated:` qualifiers (a date or timestamp, with an optional comparison); matching Tickets page fifty at a time
   (`src/app/views/threads.ts`, `src/app/store.ts`). An ended Ticket whose workspace is a linked worktree offers
   **Merge** (fast-forward, squash or merge commit into the repository's checked-out branch), **Rebase** (rewrite the Ticket branch onto a moved
   target, move the recorded base and clear the check) and **Discard** (remove the worktree and branch), plus
   **Clean up worktree** after a merge; `POST tickets/:id/merge`, `/rebase`, `/cleanup` and `/discard` run through
   `workspaceRepo`, `mergeWorkspace`, `rebaseWorkspace` and `discardWorkspace` (`src/app/review.ts`), record the
   corresponding Ticket fields (`src/app/runtime.ts`) and append matching archive sections (`src/sources/local.ts`).
   `src/app/check.ts` runs `executor.check` at each ending and `Runtime.mergeTicket` refuses a merge without a passing run
   dated after that ending (`POST tickets/:id/check`); check runs keep a capped history (`ticket.checkHistory`); a review decision (`Tickets.review`, `POST tickets/:id/review`, `executor.requireReview`) gates a merge and is archived; reviewer notes (`Tickets.addNote`/`removeNote`/`resolveNote`/`replyToNote`,
   `POST tickets/:id/notes`, `/notes/:noteId/remove`, `/notes/:noteId/resolve`, `/notes/:noteId/replies`) can be resolved, reopened, or sent to the
   Executor as one follow-up, and an open note blocks a merge; related Tickets (`Tickets.link`,
   `POST tickets/:id/related`) and the person-action history appear on the page. `review.spec.ts`, `check.spec.ts`,
   `local.spec.ts`, `views.spec.ts`, `http.spec.ts` and `panel-execution.spec.tsx` cover it.
10. **Ticket numbers and command-line intake.** Every Ticket gets a stable `T-000N` number from its creation order
   (`src/app/store.ts`, shown by `src/app/views/records.ts`, `src/client/rows.tsx` and `src/client/ticket.tsx`, and written
   into the archive's close by `src/sources/local.ts`). A request may also carry a `project` label, which is archived and
   exposed as `DSH_SUPERAGENT_PROJECT` (`src/app/execution-environment.ts`) so a prepare command can pick a repository per
   project, and a configured project row may supply its own execution root and prepare command (`projectRoot`/`projectPrepare`
   on the Trigger, `Execution.prepare`); a request may also ask to be held (`hold`), in which case the Executor launches nothing until **Approve**
   (`ticket.hold`, `POST tickets/:id/approve`, stage `person/approval`); up to eight `tags` are stored in the archive, shown on the row and page, matched by search, and usable as an
   Execution filter (`GET tags`, `GET tickets?tag=`); `POST tickets/:id/tags` replaces them, so labels stay editable after
   creation, and the archive appends a Tags section for each change. The local Source's optional webhook receives a JSON POST when a
   local Ticket asks a question or ends (`src/app/webhook.ts`, `ticket.attention` event). Up to five attachments, 8 MB total, travel base64 with
   `POST local/requests` and are written to `<archive>/<request id>.files/` (`src/sources/local.ts`), with their paths in
   the request text and archive. **Edit request** replaces the request text a Ticket shows and appends a Request section (`Tickets.editRequest`, `POST tickets/:id/request`), and **Edit title** renames a Ticket without rewriting its request
   (`Tickets.rename`, `POST tickets/:id/title`), updates the local Thread label and appends a Title section to the archive.
   A panel inbox records a notification when a Ticket asks a person a question or ends (`src/app/notifications.ts`, `GET notifications`, `POST notifications/:id/read`, `POST notifications/read-all`), and the header shows the unread count. At startup, `Runtime.reconcileArchives` completes close, rebase, merge, cleanup, discard, rename, label, request and review sections that a crash left unwritten. `scripts/dsh-superagent ticket --title … --text …` drops a JSON
   request into `<stateDir>/inbox`; the local Source's poller (`drainInbox` in `src/sources/local.ts`) turns it into a Ticket
   within two seconds, writes `inbox/done/<file>` and moves bad input to `inbox/rejected/`, with the file name as the
   idempotency key. `scripts/dsh-superagent backup` copies the database online and the ticket archives into `backups/<timestamp>/`.
   `local.spec.ts` and `service-script.spec.ts` cover it.
11. **Built-in documentation.** `docs/006-deployment.md` opens with a **Built-in copy** section (no profile, default state
   directory, API follows the bind), and `README.md` and `SETUP.md` point at the built-in path instead of the standalone
   install. The rest of the upstream documents describe the standalone deployment and still apply.

12. **Main Session requests and answers.** The main Session can record a person's explicit request from its own
    conversation as a local ticket request (`orchestrator_request` in `src/app/tools/main.ts`) and send their answer to a
    local ticket's Thread (`orchestrator_answer`). Both reuse the local Source's `createLocalRequest`/`createLocalFollowup`
    through `Runtime.localRequest` (`src/app/runtime.ts`), which seeds the local Policy part with the operator on first use
    and refuses a disabled Source or an empty list; the answer tool only accepts a ticket with a local Channel and marks
    its Trigger `mainSession`, so the main Session is not notified of its own follow-up (`src/sources/local.ts`,
    `src/app/notices.ts`). `prompts/orchestrator.md` limits both to the person's own messages. `tests/chat-tools.spec.ts`
    covers creating, answering, resuming, and the refusals. The same session merges an ended Ticket with `orchestrator_ticket_merge`, which runs the panel's gates locally, rebases a branch the repository moved past, runs a configured check that has not passed, and removes the worktree afterwards when asked with `cleanup`. The Executor records the main working tree's branch at launch (`Execution.target`) and a merge or rebase refuses a repository that moved to another branch (`src/app/review.ts`); `tests/merge.spec.ts` covers the sequencing.

13. **Ticket dependencies and review visibility.** A Ticket records the Tickets whose merge comes first (`Ticket.blockedBy`
    in `src/core/model.ts`, set through `Tickets.block` and `Engine.blockTicket`), refusing a dependency that would close a
    cycle; `Runtime.mergeTicket` refuses while one of them has not merged, and the panel sets them with
    `POST tickets/:id/blockers` (`src/app/runtime.ts`, `src/app/http.ts`). A worker's `ticket/<id>` read omits the list,
    since it names Tickets that worker cannot read (`src/app/resources.ts`). Separately, an ending anchors the review at
    `merge-base HEAD <target>` when the branch merged or rebased the repository in (`workingBase`, `finishTicket`), a merge
    captures the review again when the branch moved after it (`recaptured`), and the main Session reads the same committed
    diff as the panel through `ticket/<id>/diff`. `tests/merge.spec.ts`, `tests/review.spec.ts`, `tests/tools.spec.ts` and
    `tests/http.spec.ts` cover them.

14. **Conflict handback, chat dependencies and optional push.** A rebase that conflicts now raises `RebaseConflictError`
    (`src/app/review.ts`) instead of a bare `Error`, and `Runtime.rebaseTicket` resumes the Ticket with the conflicting files
    through `Tickets.resumeWithNote`/`Engine.resumeWithNote`, so its worker resolves the conflict in its own worktree without
    the person sending the work back. The note is delivered ahead of what waited for the Ticket, and a stopped, merged or
    cleaned Ticket, a paused Policy, one whose Mandate no longer holds, and a rebase git refuses to start keep the plain
    report. `orchestrator_ticket_dependency` records `blockedBy` from the DSH chat, the same dependency
    `POST tickets/:id/blockers` edits; both it and the route resolve a Ticket by id or display number, and the route answers
    `ticket_id` so the tool can read the card. `orchestrator_ticket_merge` takes `push`, which runs `pushWorkspace` after the
    merge is recorded and reports `pushed` or `pushError` rather than undoing the merge. `tests/merge.spec.ts`,
    `tests/chat-tools.spec.ts`, `tests/review.spec.ts` and `tests/http.spec.ts` cover them.
15. **Merge audit, review history and standing merges.** A merge records `via` (`chat`, `panel`, or `auto`), the `head`
    commit the target branch holds afterwards and, when a push was part of the request, its `push` outcome, so a failed
    push is no longer only a chat report; the archive's merge and review sections carry the same facts, and a separate push
    section records the remote's answer. Every review round is kept in `Ticket.reviews` with the commit it covered, where a
    resume or a rebase clears only `Ticket.reviewed`, the decision that gates merging. A note records the commit it was
    written on, and the panel marks it once the commits move past. `orchestrator_ticket_automerge` records a standing merge
    request (`Ticket.autoMerge`) that a pass carries out: `Runtime.runAutoMerge` merges through the same gates, retries with
    a growing wait, notifies the main Session of the first closed gate and of a merge or a give-up after eight attempts, and
    is cleared by the merge, by `orchestrator_ticket_automerge` with `cancel`, by `POST tickets/:id/automerge`, or by a
    person stopping the Ticket. `tests/merge.spec.ts`, `tests/core-engine.spec.ts`, `tests/chat-tools.spec.ts`,
    `tests/panel-execution.spec.tsx` and `tests/views.spec.ts` cover them.
16. **Undoing a merge, merging several Tickets, and the wider inbox.** A merge records the repository root it moved, so
    `Runtime.revertTicket` can add a reverting commit after the worktree was cleaned up: `revertWorkspace` reverts a merge
    commit against its first parent, aborts a conflicted revert without leaving it half-applied, and names a missing commit
    or a dirty or wrong branch; `Ticket.reverted` and the archive's **Reverted** section keep the reverting commit and the
    merge it undoes. `orchestrator_ticket_merge` takes `ticket_ids` for several Tickets, which merge in the order given and
    are each answered on their own, and `orchestrator_ticket_revert` undoes one from the chat; `POST tickets/<id>/revert`
    does the same from the panel. Panel notifications now cover `merged`, `reverted`, `review` (a deployment that requires an
    approval) and `automerge` (a standing
    merge given up after its attempts), and each also reaches a local Source's webhook. `tests/review.spec.ts`,
    `tests/merge.spec.ts`, `tests/chat-tools.spec.ts`, `tests/runtime.spec.ts` and `tests/panel-execution.spec.tsx` cover
    them. The router prompt now says the main Session merges or undoes a merge only on the person's explicit ask, within
    its 1,400-character budget.
17. **Durable merge queue, merged-target check, push retry and wider desktop notices.** `src/app/merge-queue.ts` holds the
    Tickets a person asked to merge one after another in a KV record, so the queue survives a restart; a pass drains it
    (`Runtime.runMergeQueue`), merges each Ticket with the usual gates as an `auto` merge, and keeps merges, reverts and
    pushes one at a time in a repository (`Runtime.serialized`), reports that one result to the
    main Session, and takes the entry off even when it failed, while `orchestrator_merge_queue` adds or cancels entries and
    the ticket page shows a place and a way out. After a merge, `mergeOnce` runs the configured check on the target branch,
    records `Ticket.targetCheck`, and announces a failure as a `check` notification with the way to undo the merge. A push
    that failed is retried from the ticket page (`POST tickets/<id>/push`), which records the remote's answer and leaves a
    `push` notification. The desktop watcher now also announces the merge events in the inbox it cannot see in a ticket's
    state. `tests/merge.spec.ts`, `tests/chat-tools.spec.ts`, `tests/notify.spec.ts` and
    `tests/panel-execution.spec.tsx` cover them.
18. **Per-file review diff, a review worklist, a named reviewer and the queue on the page.** `GET tickets/<id>/diff` now
    returns `files`, one entry per `diff --git` block (`parseDiff` in `src/app/review.ts`: paths, status, added/removed
    counts, the patch), and the Ticket page lists them, opens one file at a time and keeps a browser-local **Viewed** mark
    per Ticket and commit. `TICKET_FILTERS` gains `review`: the ended Tickets that are not merged, discarded, cleaned or
    approved, which the panel shows as a **To review** worklist. `TicketReviewRound.by` records the reviewer's name, which
    the runtime fills with the deployment's operator label and the page shows in the review history. The merge queue has its
    own panel section (`GET merge-queue`, `POST merge-queue/pause`) with the order, a pause and each entry's way out, and a
    check that fails on a merged target now pauses the queue instead of letting the next Ticket merge into a red branch.
    `tests/review.spec.ts`, `tests/views.spec.ts`, `tests/merge.spec.ts`, `tests/core-engine.spec.ts` and
    `tests/panel-execution.spec.tsx` cover them.
19. **Whole-merge revert, a checked push target and fewer repeated checks.** `mergeWorkspace` records the target commit it
    moved from (`TicketMerge.base`), and `revertWorkspace` reverts `base..head` for a fast-forward merge, since undoing only
    the last commit left the rest of the branch's work on the target; `pushWorkspace` refuses to push when the repository is
    on another branch than the merge moved. `mergeOnce` skips the check on the merged target when a fast-forward merge left
    exactly the tree the gate's check covered, reports tracked changes a target check left behind, and an automatic attempt
    reuses a failure recorded within ten minutes for the same ending. `rebaseTicket` fingerprints the ticket's own patch
    with `patchFingerprint` before and after, so a rebase that moved the same changes onto a newer commit keeps the
    approval and names the new commit; `Tickets.review` keeps the history's rounds as copies for that. `tests/review.spec.ts`
    and `tests/merge.spec.ts` cover them.
20. **Changed-since-review diffs, named checks and kept merge failures.** `GET tickets/<id>/diff?since=<commit>` returns
    the diff from a reviewed commit (`Runtime.ticketDiff(id, since)`), which the ticket page offers as **Changed since the
    review**; the browser keeps its viewed marks per diff. `executor.check` holds one command per line, each optionally
    `name: command`; `parseChecks` reads them and `workspaceChecks` runs each in order, recording `Ticket.checkRuns` beside
    the combined `Ticket.checks` that the merge gate reads, so a failure names its check on the page and in the
    notification. A merge attempt that failed keeps `Ticket.mergeError` until a merge succeeds, and the page shows it, so a
    chat that has scrolled away is no longer the only record. `tests/check.spec.ts`, `tests/merge.spec.ts` and
    `tests/panel-execution.spec.tsx` cover them.
21. **Retryable queue entries, range notes and a re-mergeable revert.** A queue entry whose merge failed keeps its place
    with `error` (removed only by `dequeueMerges` or replaced by a re-queue), the drain takes `nextQueuedMerge` rather than
    the head, and `retryMerges` clears the failure from the chat tool or `POST merge-queue/retry`. `TicketNote.endLine`
    lets a review note cover a range, checked in `Tickets.addNote` and shown with the notes sent to the worker.
    `Runtime.revertTicket` now deletes `merged` after recording the revert, so a corrected Ticket can merge again; the page
    shows the revert only while nothing merged after it, and a record that still holds both (written before that change) is
    refused a second revert. Re-queueing a Ticket replaces its entry, which clears the failure, and a failed entry whose
    Ticket was merged, discarded or cleaned elsewhere is dropped while an open one keeps waiting. The notes route keeps its
    `UserError` code, so the panel words `invalidNoteRange` in the interface language. `tests/merge.spec.ts`,
    `tests/core-engine.spec.ts`, `tests/chat-tools.spec.ts`, `tests/http.spec.ts` and `tests/panel-execution.spec.tsx` cover
    them.
22. **A chat status report, a standing request that waits, and a queue entry retried on its own.**
    `orchestrator_merge_queue` with no `ticket_ids` reports the queue and the ended Tickets still waiting for a merge or a
    person's decision (`Runtime.pendingMerges`), each with the gate it waits on, so the main Session answers "what is left"
    without the panel. A standing merge request checks the gates another Ticket or the person opens before merging
    (`Runtime.mergeGate`), and records such a wait without adding to the attempts that end the request
    (`Tickets.autoMergeBlocked(..., counts)`), so a dependency that takes an hour no longer cancels it; a gate that needs new
    work still counts and still ends after eight attempts, and so does a dependency that was discarded, which can never merge. A failed queue entry records the gate it failed on
    (`MergeFailureCode`), and a drain retries it once that gate opens (`recoverableMerges`), such as a dependency that merged
    or a review that was approved, while the page says so. `tests/merge.spec.ts`, `tests/chat-tools.spec.ts` and
    `tests/panel-execution.spec.tsx` cover them.
23. **Pushing what a merge or a revert moved, and a comment on a diff line.** A merge made from the panel takes `push`
    (the ticket page's **Push after merging**), and `Runtime.pushTicket` completes a merge or a revert that was never pushed
    or whose push failed: `TicketRevert` keeps `root` and `push`, so `POST tickets/<id>/push` and the page's **Push** work
    after the worktree is gone. `Runtime.revertTicket` pushes the reverting commit when asked and records a failed push
    without undoing the revert, `orchestrator_ticket_revert` takes `push` and records `via: chat` like the other tools, and
    the archive keeps the push section either way. The review diff is drawn line by line (`src/client/diff.ts`): each line of
    the file after the change offers a comment that fills the note's file and line, and existing notes render under the line
    they anchor on, while a removed line has none. `patchLines` treats `---`/`+++` as file headers only before the first hunk,
    so a changed line whose content starts with `--` or `++` keeps its line, and a push records its answer on the merge
    timestamp or revert commit it acted on. `tests/merge.spec.ts`, `tests/chat-tools.spec.ts`, `tests/views.spec.ts`,
    `tests/panel-execution.spec.tsx`, `tests/review.spec.ts` and `tests/diff-lines.spec.ts` cover them.

24. **Review and check hand-back from the chat, and note freshness per line.** `orchestrator_ticket_note` takes `review`
    and `check` in addition to `text`; the runtime hands the unsent open notes or the failing check's output to the worker
    as one note and resumes an ended Ticket with `resumeWithNote`, marking the notes `sent` so the same note is not sent
    twice (a reply or a reopen clears it). `POST tickets/<id>/handback` and the ticket page's **Send notes to the Executor**
    and **Hand the failure to the worker** use the same path, and **Resolve all** resolves every open note. `ticket/<id>`
    now carries the review summary, the notes with their state, the check and its output, the per-check results and the
    merge error; `ticket/<id>/notes` reads the notes whole, and the diff resource's header summarizes each changed file.
    `remapNoteAnchors` diffs a note's commit to the review head with `--unified=0`, moves the note's line and range with the
    code, and marks only the notes whose own lines changed, so the page no longer marks every note whenever the branch
    moves. `tests/core-engine.spec.ts`,
    `tests/runtime.spec.ts`, `tests/chat-tools.spec.ts`, `tests/tools.spec.ts`, `tests/review.spec.ts`,
    `tests/cards.spec.ts` and `tests/panel-execution.spec.tsx` cover them.

25. **Note anchors that follow the code, start dependencies, and merge messages.** `remapNoteAnchors` maps a note's old
    line and range through the diff hunks to the current head, so the panel anchors it on the line the code now occupies;
    a hunk that replaces or deletes the covered lines marks the note **Changes moved** and keeps it out of the inline diff.
    `remapNoteAnchors` reads the whole diff rather than only the notes' paths, so a rename is seen; the panel draws a note
    inline only while its commit is the current review head. `Ticket.startAfter` records a dependency that holds the first
    Assignment: `orchestrator_ticket_dependency` and the ticket page's **Wait before starting** checkbox set it, the
    Executor does not launch while a blocker is unmerged, and the merge that lands the last blocker starts the waiting
    Assignment; a start dependency is refused once the Ticket started or ended, and a discarded blocker is marked
    **Discarded**. `mergeWorkspace` takes a message, and a squash or merge commit carries the Ticket's title, the worker's
    report summary, and the Ticket number. `tests/core-engine.spec.ts`, `tests/runtime.spec.ts`, `tests/merge.spec.ts`,
    `tests/review.spec.ts` and `tests/panel-execution.spec.tsx` cover them.

26. **Stopping, approving and rechecking a Ticket from the chat.** `orchestrator_ticket_abandon` stops a live
    Ticket through `Tickets.stop`, which ends its Assignment and withdraws a standing merge request, removes the Ticket
    from the merge queue, and with `discard` removes the ended Ticket's worktree and branch through `discardWorkspace`. `orchestrator_ticket_review`
    records an approval through `reviewTicket` with origin `chat`, which is what `executor.requireReview` waits for,
    or resolves every open note through `resolveAllNotes`, which is what an open note blocks the merge on.
    `orchestrator_ticket_check` reruns the configured checks through `checkTicket`, so a flaky failure no longer needs
    a new Assignment or the ticket page. A stopped Ticket cannot be queued or given a standing merge request; an
    explicit merge still runs, because a person's stop withdraws the standing requests, not the work. The three tools resolve an id or a display number such as `T-0007` through
    `Runtime.ticketId`; `pendingMerges` now carries `ended.outcome`, so a stopped or failed Ticket is not read as
    ready to merge. `tests/chat-tools.spec.ts` covers the tools, the reference resolution and the waiting list; the
    orchestrator model-surface budget moved from 64,700 to 65,500 after trimming existing descriptions, measured
    65,390 characters.

27. **Anchored notes, rejections, and the merge aftermath from the chat.** `orchestrator_ticket_note` takes `file` with
    optional `line` and `end_line` and records a review note of up to 2,000 characters through `Engine.addNote` instead of
    sending a message, so a comment made in the chat appears on the diff and blocks the merge like one made on the page; a
    message without `file` keeps the 100-word limit, and `review` still hands the unsent notes over.
    `orchestrator_ticket_review` takes `changes` and `note`, recording a `changes-requested` decision that blocks merging
    until a later approval, and `orchestrator_ticket_finish` retries a failed push through `pushTicket` or removes a merged
    worktree through `cleanupTicket`. `orchestrator_merge_queue` takes `paused`, so a queue paused by a failed target check
    can be released from the chat. The review, push and cleanup records name `chat` as their interface and reach the
    archive. `tests/chat-tools.spec.ts` covers them; the orchestrator model-surface budget moved from 65,500 to 67,500 for
    the new tool and options.

28. **Removal of the GitHub Source and account integration.** The vendored copy no longer contains `src/sources/github*.ts`,
    the GitHub skill, the GitHub account and repository setup, the pull-request binding and its CI gate, or the GitHub
    prompt text and UI. `Deployment` carries only `stateDir` and the optional legacy Feishu bot; `SETUP_FIELDS` probes only
    that legacy CLI; `stateView.accounts` has only `feishu`; `orchestrator_report` has no `pr_number`; `orchestrator_eval_read`
    reads only Thread context; `orchestrator_read` exposes `feishu/` originals and no `github/` resources; and the merge
    gate is local: review decision, `executor.requireReview`, open notes, configured checks, dependencies, target
    containment, rebase, queue and the post-merge target check. No migration or startup cleanup handles GitHub records:
    a database written by a GitHub-enabled build is not cleaned, so start with a new state directory if its old records
    must not remain. The plugin locale metadata no longer names GitHub. The worker environment still keeps `gh`
    unavailable and clears `GH_TOKEN`/`GITHUB_TOKEN`, so a worker cannot reach the operator's GitHub credentials.
    `tests/local.spec.ts`, `tests/migration.spec.ts`, `tests/deployment.spec.ts`, `tests/tools.spec.ts` and
    `tests/merge.spec.ts` cover the local behavior.

29. **Feishu intake through dsh-im.** The Feishu Source is now `src/sources/feishu-im.ts`, exported as `./feishu-watch`.
    It consumes `ctx.dshIm`'s exclusive text consumer after the operator turns on **交给 Superagent** in a dsh-im Feishu
    bot's settings; `src/sources/dsh-im.ts` types that service; the watcher polls `inboundStatus` and calls `describeBot` only when registering. It stops using the separate `lark-cli` bot, app secret,
    `FeishuBot` setup and `dsh-superagent-feishu` opt-in: the Source is mounted by default and stays idle until the
    handoff. Replies go through `replyChecked` and context through `historyChecked`; item 30 adds history recovery,
    images, bot-authored messages, member sync and original-message links on top of dsh-im's native reads, so no
    `lark-cli` resource read is involved. dsh-im owns the switch and the
    `superagent`/`superagentActive` status; turning it off releases the consumer and restores normal IM chat. Each
    Trigger records its source `botId` so a later Source bot change cannot route an old ticket through the new bot, and
    `describeBot` carries the Feishu/Lark domain for chat links. The watcher polls the cheap `inboundStatus`; `describeBot`
    runs only when registering. A dsh-im `botId` also skips the legacy lark-cli setup probe.
    `tests/feishu-im.spec.ts` covers the mapping, scope, policy, replies, history, the `feishu-im/` resource and consumer registration; dsh-im's
    `tests/bot-general-settings.test.mjs` and `tests/channels/feishu/multi-bot-controller.test.mjs` cover the switch.

30. **Feishu recovery and content through dsh-im.** `src/sources/feishu-im-loops.ts` reads each configured chat and each
    known direct chat from a stored cursor every 60 seconds, recovering the replies of the Topics those chats show
    (Feishu returns only the message a Topic was opened on in chat history) and polling subscribed threads, so messages
    missed while superagent or the Host restarted are recovered; re-received messages deduplicate on their Trigger id.
    Pages are dsh-im's largest (50) up to 2 000 messages per chat per round, so a long outage catches up in a few
    rounds; a Topic a subscribed Thread already polls is skipped there, and one quiet for the polling horizon is
    forgotten. A Topic's own history is read oldest-first through `threadHistoryChecked` (`historyChecked` on an older
    dsh-im), so the message that opened it anchors the first page an assessment window reads. `imTrigger`
    substitutes mention names, marks bot senders non-human, and records `messageType`/`images`; `readFeishuImImage`
    downloads an image through `resourceChecked` and commits it as a model-visible attachment. Policy regains `bots` and
    `syncChat` (`chatMembersChecked`), `sourceLink` resolves the message's own AppLink through `messageChecked` with the
    `feishu-im/` resource as fallback, and `sendFeishuIm` passes the receipt id as Feishu's `uuid`, so the outbox can retry
    an ambiguous (`reply-result-unknown`) send and Feishu deduplicates it instead of producing a second reply. Replies
    render as rich-text cards in the source message's topic (`format: 'markdown'`, `replyInThread: true`), so a Ticket
    answer looks like the removed lark-cli Source's `--markdown --reply-in-thread` reply, and a bot-authored source can be
    answered because dsh-im accepts bot senders on the retained route. `root`, `parent` and `message` context reads the
    message by id through `messageChecked({ messageId })` instead of searching the nearby window, so a distant parent still
    resolves, and `around` reads the chat around a message even when it sits in a Thread; events carry `appLink`, the
    Trigger keeps it as `url`, and so do context messages. An image of any message those reads return is downloaded from
    that message (`resourceChecked({ messageId })`, validated against the retained route), not only from the Trigger's own
    message. The live path describes each Thread with its chat name and chat link, `linkTimeoutMs` bounds link resolution,
    and `DSH_SUPERAGENT_FEISHU_BOT` names the bot once its account is cached. The first backfill round for a chat records
    its position instead of replaying the conversation dsh-im handled before the handoff. The backfill loop merges the chats
    the bot joined (`chatListChecked`) and dsh-im's group target names into the stored label map every round, and
    `refreshLabels` persists what it reads, so the chat picker offers every joined chat. A Thread without a topic learns it
    from its root message once per activity, and the lookup is remembered in `KV.feishu.topic` so a topic-less Thread costs
    one read, not one per round. dsh-im never hands the bot its own replies back as recovered messages, expands a merged
    forward into its sub-messages, and cuts a body over 16 000 characters with a marker instead of dropping it, so the
    Source receives the forward's contents and no self-authored Triggers. An assessment reads any image of the Thread back as an attachment and any file, voice message, or video back as a saved path (`files`/`file` on `ThreadContext`, resource `file` in `orchestrator_eval_read`), because no provider accepts those bytes natively; a message whose handling fails stays in the `feishu` health entry for ten minutes, since dsh-im only logs the rejection. The deployment view names the dsh-im bot the Source is bound to (its `botId` and stored label), and the Source card shows that name under its bot field. A failing attempt backs off from one second to one minute (`consumerRetry` in `poll.ts`),
    measured from the attempt itself so a long wait does not reset it, and the known states report health codes the panel
    localizes instead of raw English details. The Source declares the `history` health entry. `tests/feishu-im.spec.ts`
    covers mention rendering, idempotent replies, attachment reads, a failed message kept in health, backfill and policy sync.

`cordis.patch.yml` in this directory is the upstream bundle layer, kept as the package's own definition and covered by
`tests/bundle-patch.spec.ts`. The Web bundle's rows and preset files are what the built-in copy actually mounts; keep the
two in step when changing a preset.

`scripts/baseline-inventory.ts` and `.oxlintrc.json` treat `apps/web/native/` as vendored source, as they already do
`vendor/`: the inventory gates (such as `verify-no-unknown-casts`) and lint keep this copy close to upstream instead of
rewriting its test idioms.

## Checks

- `pnpm --filter dsh-superagent run build` builds the Host entries and the browser client.
- `pnpm --filter dsh-superagent exec vitest run` runs the vendored suite (1,323 tests).
- `pnpm run typecheck`, `pnpm run build:check`, `pnpm run lint` and `pnpm run test:docs` cover the repository gates; the
  vendored directory is excluded from the inventory and lint corpora, as `vendor/` is.
- `pnpm dsh web` boots the Web profile: the sidebar shows **Superagent**, `GET /orchestrator/api/state` answers without a
  setup problem, and `<DSH_HOME>/state/superagent` is created.
- Usage starts at [docs/006-deployment.md#quick-start](docs/006-deployment.md#quick-start): local tickets, the dsh-im-backed
  Feishu Source, the three settings groups, the first request, everyday follow-ups, Feishu enabling and troubleshooting.
- A clean worktree `pnpm install && pnpm run build` includes the `Superagent plugin` gate before the Web build.

## Syncing upstream

1. Copy the upstream `src/`, `tests/`, `docs/`, `prompts/`, `skills/`, `locale/`, `examples/`, `scripts/`, `README.md`,
   `SETUP.md`, `cordis.patch.yml` and `LICENSE` over this directory, and update the baseline commit above.
2. Re-apply every local modification in the list.
3. Run `pnpm --filter dsh-superagent run build`, then its `vitest` suite, then the repository's Web build and a boot check
   (`pnpm dsh web`; the sidebar shows **Superagent**, and `GET /orchestrator/api/state` answers).

## Boundaries

- Executor Sessions run with the profile's permissions as the operator's OS user. The command shims and git configuration
  under the state directory are defaults, not a security boundary; use a dedicated OS account for isolation.
- The Feishu Source stays idle until a dsh-im bot's **交给 Superagent** switch is on; local tickets need no account.
- The upstream documents describe installing the bundle into a separate `superagent` profile. The built-in copy needs no
  such profile; `docs/006-deployment.md` starts with the built-in path.
- The management API follows the Web server's bind address. On a non-loopback bind it is reachable from that network;
  DSH's Web authentication and the same-origin request check remain the boundary.

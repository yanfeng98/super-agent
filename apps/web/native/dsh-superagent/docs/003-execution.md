# 003 Execution

This document follows an accepted Ticket through execution, delivery and recovery. It builds on [001 Domain model](001-domain.md) and [002 Authorization](002-authorization.md).

## Ticket stages

Each open Ticket has one stage, determined by what it is waiting for. Read the table from top to bottom; the first matching row wins.

| Stage | Condition | Panel label |
| --- | --- | --- |
| person / approval | The request asked to wait for a person's approval, so its first Assignment has not launched | Awaiting approval |
| person / delivery | Delivering an Input to the Executor Session failed; it is retried after 2 seconds, then after a delay that doubles up to a minute, and a success, a new Assignment or the Ticket's end clears it | Input undelivered |
| person / answer | It has an Attention | Awaiting answer |
| person / review | The latest Report is `ready` | Awaiting review |
| person / blocked, failed | The latest Report is `blocked` or `failed`, and no person's message reached the Ticket since | Blocked · Failed |
| working / queued | No Assignment, or a pending one | Queued |
| idle | The latest Report is `waiting`, or the Assignment ended | Waiting for a Trigger |
| working | Everything else | Working |

A `person` stage means the Ticket cannot continue without human input. A new human message releases a `blocked` or `failed` Ticket to its Executor, so it moves to the next matching stage, usually `working`.

An `idle` Ticket has finished its turn without an outstanding question. A Trigger on a subscribed Thread can continue it.

## The Executor

Each Ticket gets an Executor Session named `worker-<ticket id>` and a directory named after the Ticket's short name. Directories are created under the configured execution root, which defaults to `<stateDir>/work`; a request that names a configured project uses that project's root and prepare command instead.

Executor Sessions run commands without approval as the operator's OS user. Their `dsh-superagent-worker` preset contains the tools needed for work and moves foreground shell commands to background jobs after 5 seconds. This lets a worker receive new messages at the next step instead of waiting for a long command to finish.

The main Session uses `dsh-superagent-router`, with the same 5-second shell and job-wait limits and a smaller tool set for routing. When the person asks for work in its own conversation and the local Source is on, it records a local request with `orchestrator_request`, and it sends a person's answer to a local Ticket with `orchestrator_answer`, which the worker reads as an Input; on an explicit request it merges an ended Ticket with `orchestrator_ticket_merge`, with the panel's gates. That merge also does the two steps the ticket page would need: it rebases a branch the repository moved past, and it runs the configured check when the check has not passed since the ending. A rebase that conflicts aborts in the Ticket's worktree, names the conflicting files, and resumes the Ticket with them, so its worker resolves the conflict in its own worktree and reports ready again; a stopped Ticket, a paused Policy, or one whose Mandate no longer holds keeps the plain report and its commits, and so does a rebase git refuses to start, such as over uncommitted changes. With `cleanup`, the same call removes the merged worktree and branch; with `push`, it sends the merged branch to its upstream and reports a push it could not make without undoing the merge, which a later `git push` in the repository completes. `orchestrator_ticket_dependency` records that one Ticket waits for another before merging, which the ticket page shows and edits as well; with `start` it also holds the waiting Ticket's first Assignment until the other merges, so that work begins from the merged target. `orchestrator_ticket_automerge` records the person's request to merge once the gates pass instead of now: a pass retries it with a growing wait, tells the conversation when the first gate stays closed, gives up after eight attempts that failed on their own, since a gate that waits for another Ticket or for the person does not use them up unless that dependency was discarded and can never merge, and the request clears itself when the merge lands, when it is cancelled, or when a person stops the Ticket. Sessions retain the preset selected at creation. Worker subagents use DeepSeek Harness's `standard` preset.

### Workspace review

Merges, reverts, a deployment that requires a review, failing configured checks and a standing merge the host gave up on each leave a panel notification and, for a local Ticket, a webhook event. When a Ticket first launches, the Executor records the workspace's current commit as its base. When the Ticket ends, it collects the workspace's branch, head, the commits added since that base, the files they changed, and any work left uncommitted, and stores them on the Ticket. When the branch rebased or merged the repository's branch in meanwhile, the base moves first to the commit the branch forked from it, so the review lists only the Ticket's own commits, and its diff leaves the repository's changes out. The panel shows them under **Changes**, with **View diff** reading the committed diff on demand; the main Session reads the same diff from the `ticket/<id>/diff` resource, whose header summarizes each changed file, so it can say what changed without opening the panel. A local Ticket's archive receives the same facts with its ending. A workspace that is not a git repository records the reason instead, so a Ticket without version control still ends normally. Reopening a Ticket, or resuming it with a person's message, clears the review, the check and the review decision, and captures them again at the next end; a Ticket whose branch was merged, or whose worktree was discarded or cleaned up, cannot be reopened or resumed.

A held Ticket launches no worker until **Approve** clears `ticket.hold`; the approval is recorded on the Ticket and in a local Ticket's archive. A held Ticket that is stopped and later reopened returns to approval, because the hold is still set.

A crash between an ending and its archive write can leave the Markdown behind; after the next startup, each ended local Ticket whose archive lacks its close section gets it (with the review and the check), and recorded rebases, merges, cleanups, discards, renames, label, request and review changes are completed, so the archive catches up without repeating work.

An ended Ticket whose workspace is a linked git worktree offers **Merge**, **Rebase** and **Discard**. Merge takes the chosen strategy: fast-forward, a squashed commit, or a merge commit. All three refuse when that tree has tracked changes, when the Ticket branch is checked out there, when a review note is still open, when a review requested changes, when `executor.requireReview` needs an approval that is missing, or when the configured checks have not passed since the ending, and all three require the repository's commit to be in the Ticket branch, so none leaves a conflict state. Merge alone also waits while a Ticket it is blocked by has not merged; Rebase and Discard do not. A Ticket whose dependency was recorded with **Wait before starting** does not launch its first Assignment until those Tickets merge, including a launch that was queued just before the dependency was recorded, and the merge that lands the last one starts it; a Ticket that already started keeps working, so the setting only gates the first Assignment. Only an open Ticket that has not begun its first Assignment can record it; once the Ticket started, the page and `orchestrator_ticket_dependency` refuse it, and a discarded blocker is marked **Discarded** rather than **Not merged yet**; it still holds the first Assignment until a person clears the dependency. A branch that fell behind the repository is refused with the target branch named; **Rebase** rewrites only the Ticket branch onto that branch and captures the review again, and a conflict aborts the rebase so the original commits stay. A merge from the main Session takes the same path but completes the two steps in one request: it rebases the branch when the repository moved and runs the configured check when it has not passed, then merges, and it reports `rebased` when it had to rebase. A squash or merge commit carries the Ticket's title, the worker's report summary, and the Ticket number, while a fast-forward keeps the worker's own commits. A merge records which interface asked for it, the commit the target branch holds afterwards, and, when a push was part of the request, the remote's answer, which leaves the merge itself untouched. A revert records the same way, and a merge or a revert that was never pushed can be pushed from the ticket page or `POST tickets/<id>/push`; a failed push leaves the merge or the revert in place. An attempt that did not land keeps its reason on the Ticket, where the page shows it until a merge succeeds, so a failure the chat has scrolled past is still readable. A branch that gained commits after its ending is captured again before merging, with the check run on the new tree, and the result reports `recaptured`; with **Require review approval** on it refuses instead, because the merged work needs a review a person has not given. Discard removes the linked worktree and deletes its branch, including uncommitted work; the repository and its other worktrees stay. The main Session does the same with `orchestrator_ticket_abandon` and `discard: true`, and stops a live Ticket without it; either call withdraws a standing or queued merge request; a stopped Ticket merges only on an explicit request, since a queued or standing merge of it is refused.

Rebasing moves the recorded base to the target's current commit, so the review and diff then list only the ticket's own commits; it also clears the recorded check, because the branch now runs on a different tree, and a merge waits for a fresh run. The ticket's own patch is fingerprinted before and after the rebase: when it did not change, an approval of it survives and names the new commit, and when it did change, the approval is cleared as before. The rebase is recorded on the Ticket, listed in its history, and appended to a local Ticket's archive. A merge, a revert and a push run one at a time in a repository, so two of them never move one checkout at once. A person who asks to merge several Tickets gets a queue in the store: the host merges them in the order given, one at a time, reports each result to the main Session, keeps the queue across a restart, and leaves the rest to run when one fails or hands its conflict to its worker; taking one out of the queue stops it only if it has not started, an entry whose attempt failed keeps its place with the reason and waits for a retry rather than running again on its own, an entry whose Ticket can no longer merge (such as one a person merged elsewhere) is dropped, and a check that fails on a merged target pauses the queue so the next Ticket does not merge into a red branch. After a merge, the configured check also runs on the target branch in the repository's own checkout, unless a fast-forward merge left the target with exactly the tree the gate's check already covered; a target check that leaves tracked changes behind is reported, and a failure is announced with the way to undo the merge. A push that failed can be retried from the Ticket page, or with `orchestrator_ticket_finish` and `push`, which records the remote's answer again; the same tool with `cleanup` removes a merged worktree, as **Clean up worktree** does, and `orchestrator_merge_queue` with `paused` holds or releases the queue, such as after a failed target check. A merge can be undone with **Undo merge** on the page or `orchestrator_ticket_revert` in the chat: the host reverts the merge on the branch it moved, using the repository root the merge stored, so it works after the worktree was cleaned up; a fast-forward merge is undone commit by commit from the target commit it moved from, a merge commit is reverted against its first parent, the revert is recorded with the commit it undid, and a conflict aborts with nothing left half-applied and is the person's to resolve with git. Undoing a merge leaves the Ticket mergeable again, so a corrected branch can land after a revert; the archive keeps both sections, the revert names the merge commit it undid, and pushing the reverting commit is the repository's own step, since the page's **Retry push** belongs to the merge record the revert cleared. After a merge, **Clean up worktree** removes the linked worktree and deletes the merged branch with git's safe deletion check, so the commits stay on the target branch and no stale worktrees accumulate; a merge from the main Session does the same when it is asked for with `cleanup`. After a squash merge the branch is not an ancestor of the target, so cleanup removes it without that ancestry check; the squash commit keeps the changes. The Ticket records each action's time and the interface that carried it out, and appends a section to a local Ticket's archive with the same facts, including a review's commit and a push's answer.

### Review notes

A person reviewing the captured changes can leave notes anchored to a file and optional line; the main Session records the same note with `orchestrator_ticket_note`, naming its `file` and optionally a `line` or `end_line`; a note holds up to 2,000 characters, while a message without a file keeps the 100-word chat limit. Notes are Ticket metadata, not Triggers; **Send notes to the Executor** hands the unsent open notes to the Ticket's worker as one note, resuming an ended Ticket under its Mandate, and the main Session does the same with `orchestrator_ticket_note` and `review: true`. A failing configured check offers **Hand the failure to the worker**, which sends the check's output the same way; `orchestrator_ticket_note` with `check: true` does the same. A note records when it was handed over, so the same note is not sent twice; a reply or a reopen makes it due again. A local Ticket's archive receives a **Handback** section with the text the worker got. Each note can be marked resolved from the Ticket page, **Resolve all** resolves every open note, and an open note keeps the Ticket from merging until it is resolved or removed. The main Session resolves every open note with `orchestrator_ticket_review` and `resolve: true`.

A Ticket ends and waits for a person's review until it is approved; the Ticket list's **To review** filter is that worklist, and an approved Ticket leaves it. Each note may cover one line or a range of lines, and also keeps a discussion: a person replies to it from the Ticket page, newest replies shown with the note, so a review thread does not need a separate Trigger. The same section records the person's review decision, **Approve changes** or **Request changes**, with the reviewer's name; the main Session records an approval with `orchestrator_ticket_review` and `approve: true`, which is what a deployment with `executor.requireReview` waits for, and a rejection with `changes: true`, both taking an optional sentence recorded with the decision. A changes-requested review blocks merging until it is replaced by an approval, and with `executor.requireReview` on, an ended Ticket needs an approved review before it merges at all. Reopening or resuming the Ticket, or rebasing its branch, clears the decision, because the work it approved changed; the archive receives a **Review** section. Every round stays on the Ticket after that clearing, each naming the commit it covered. A note's line and range move with the code through the commits after it was written, so the page anchors it on the line the code now occupies; when a commit replaces or deletes the lines it covers, the note is marked **Changes moved** and stays in the list instead of under a line. While the review is cleared, such as after a resume, an anchored note is treated as moved until the next capture.

### Workspace check

When `executor.check` is set, each ending runs it in the Ticket's directory, with the Execution environment variables, and records pass or fail, the exit code, and the end of its output (up to 4000 characters) on the Ticket. The setting holds one command per line, and a line written `name: command` names that check; each check runs in order, keeps its own result beside the combined one, and the merge requires all of them to pass. The page shows the latest result, keeps the earlier runs in its history, and offers **Run check** for another run, which is how a person confirms a repair; the main Session reruns it with `orchestrator_ticket_check`. With a check configured, **Merge** refuses a branch without a passing check run after the ending, whether the merge comes from the panel or the management API; a Ticket that ended before the command was configured has no such result and can run the check from its page first. A changes-requested review, or a missing approval while `executor.requireReview` is on, refuses the merge in the same way. An automatic attempt that already saw this ending's check fail reports that failure instead of running the same command again for ten minutes; a person asking gets a fresh run. A timeout counts as a failure. The check never replaces the worker's report: the worker still says what it did, and the check says whether the result passes. The page and the main Session's `ticket/<id>` view carry the result and its output; a failing check can be handed to the worker with **Hand the failure to the worker** or `orchestrator_ticket_note` with `check: true`, which sends the output and resumes an ended Ticket.

### Background work and worker slots

When a turn ends with shell jobs or subagents still running, the Ticket keeps its worker slot and stays working. It is not reported as waiting, and the main Session receives no waiting report. A completed job or subagent wakes the Executor with its result in the same slot. Once a turn ends with no background work, the Executor reports waiting normally.

After each turn, background work has at most 30 minutes to finish. At that limit, shell jobs are killed and the Ticket is reported waiting. The report includes the Executor's last words and which jobs were killed or subagents remain active. Subagents are allowed to continue. The Executor reads killed-job results on its next turn.

A handoff to a person frees the slot immediately and leaves the Report in effect. This applies after a `ready` Report or a question, unless another Input arrived since, and after any `blocked` or `failed` Report.

At handoff, subagents stop. Shell jobs, such as a server offered for review, may continue until the Assignment ends. They hold no slot and are exempt from the 30-minute limit, including after later turns. Their completion does not wake the Executor; it reads the results on its next turn.

A person can also start a turn by writing directly in the Executor Session. Its background work follows the same rules.

When an Assignment ends, its shell jobs are killed without waking the Session. Its subagents are released, and any completion turns they wake are dropped. The replacement Assignment and its waiting Inputs start after that shutdown finishes.

Stopping or restarting the plugin inside the DeepSeek Harness process uses the same shutdown path. Inputs not yet taken into Session history are retained. Shutdown waits up to 10 seconds for each worker's background work; a process restart ends that work outright. An Assignment interrupted while busy restarts with a recovery note.

Worker subagents, including nested ones, use DeepSeek Harness's `standard` preset with the worker's permissions. They retain only the worker's allowed work tools, and lose those tools when the Ticket ends. Under `standard`, a shell command moves to the background after 60 seconds. A subagent's jobs are killed when it ends.

### Preparing a Ticket directory

Before the first Assignment, if the Ticket directory does not exist, the prepare command runs once with `sh` in the execution root. It has ten minutes to finish. Without a prepare command, the worker starts in an empty directory.

A failed command fails the Ticket and records its output. A retry or a person's message on one of the Ticket's Threads tries preparation again. A typical command uses `git worktree` to create a checkout.

### Concurrency

A Ticket holds a worker slot from the start of a launch, including preparation, or from the delivery that wakes its idle Executor. It also holds the slot throughout an active turn.

The configured concurrency limit applies to these slots, with one extra slot for a person's message or a status request. Such a wake can use the extra slot when normal slots are full, and takes the next free slot before a new launch. New work therefore cannot prevent a person from continuing an existing Ticket.

Other Inputs, such as later messages, wait for a normal slot. A launch waits in `working / queued` until a slot is free. Launches run concurrently, so a slow prepare command delays only its own Ticket.

### Execution variables

The prepare command and Executor instructions receive the same execution variables. The command reads environment variables; the Executor reads their values and descriptions in its prompt. None contains credentials.

| Variable | Registered by | Value |
| --- | --- | --- |
| `DSH_SUPERAGENT_DIR`, `DSH_SUPERAGENT_NAME`, `DSH_SUPERAGENT_TICKET` | Executor | The Ticket's directory, short name and id |
| `DSH_SUPERAGENT_BLOCKED_COMMANDS` | Executor | Commands the Executor may not run |
| `DSH_SUPERAGENT_FEISHU_BOT`, `DSH_SUPERAGENT_FEISHU_CHAT` | Feishu | The bot, and the chat that started the Ticket |

Ticket-specific values come from its directory and originating Trigger. Other values come from the current deployment settings. Only the panel can change execution environment settings; text sent to the main Session cannot change the configured shell command.

Workers reach Feishu through the bot's `lark-cli`. Blocked commands fail when invoked by name.

These wrappers and directories establish defaults, not a security boundary. A worker can still execute a program by absolute path.

## Merging

Merging is local and always the person's decision: the panel's **Merge**, the main Session's `orchestrator_ticket_merge` on an explicit request, the standing request or the merge queue. The Delivery requirements prompt tells a worker never to merge its own work and to commit on the ticket branch without pushing.

The host applies the same gates wherever the request comes from: no open review note, no changes-requested review, the approval that `executor.requireReview` requires, every configured check passed since the ending, and every Ticket it waits on merged. The merge also needs the repository's checked-out commit in the Ticket branch: a branch behind it is rebased first on an explicit request, or refused with the target named. A merge, a revert and a push run one at a time in a repository.

A merge moves the repository's checked-out branch by fast-forward, a squashed commit or a merge commit. After it, the configured check runs again on the target unless a fast-forward left exactly the tree the gate covered; a failed target check is announced with the way to undo the merge and pauses the queue. The Ticket records the interface that asked for the merge, the commit the target holds afterwards and, when it pushed, the remote's answer; **Undo merge** adds a reverting commit on the same branch.

## How a Ticket ends

- **Ready:** a `ready` Report ends the Ticket as completed.
- **Nothing to do:** an Executor that finds no work for a person to act on, such as a request that does not reproduce or is superseded, reports `dropped`. The Ticket ends without a message, unless a matched rule asks for one, and a person's later message on its Threads resumes it.
### Delivering the result

A `ready` Report ends the Ticket as completed when it follows no Thread. Its work stays on the ticket branch in its local workspace, and the Ticket page and archive hold the changes, the check and the review. A merged, discarded or cleaned Ticket does not resume; a person's later message there goes back to Eval.

A Source whose Thread lifecycle determines whether work is done can make a Ticket follow those Threads, with `bindThread` and `threadClosed`. No built-in Source does this now: local tickets end when their worker reports ready, and Feishu Threads carry conversation rather than completion.

"Awaiting review" therefore means a `ready` Report with local changes ready for review.

### Follow-ups and context Threads

The Executor decides human follow-ups on its Ticket's own Threads. It handles in-scope requests and hands back requests beyond its scope or for another Ticket. Once no Executor owns the message, it becomes a candidate for the main Session. The main Session may otherwise act on the follow-up only to stop work at the person's request.

An Input counts as read after the Session takes it into a step and subsequently reports or ends a turn. When a Ticket ends, unread messages that were not acknowledged, answered or handed back return to Eval, as described in [002 Authorization](002-authorization.md). Read messages left without action, such as thanks, do not.

Threads followed only for context still deliver messages, but do not give the Ticket ownership of human follow-ups or resume ended Tickets. Those messages can be assessed independently.

Sources poll subscribed Threads while a live Ticket subscribes, and for two weeks after the later of the last Trigger and the subscribing Tickets' end. Afterwards, messages arrive only through Feishu live events.

## Failure and recovery

### Executor blocked or failed

When an Executor Session reports `blocked` or `failed`, or fails to start or errors, which records `failed`, the Ticket takes no automatic Input.

A passing failure of the model provider or the network, an HTTP 408, 429 or 5xx or a refused, reset or timed-out connection, is tried again first, three times with a growing backoff: a launch runs again, and a worker whose turn failed is told to go on; a failed prepare command is not tried again.

Once failed, bot messages are recorded on it, but they are not delivered and do not wake the Session.

Its tools stay available, so a person can continue in the Session directly.

A launch that failed is not tried again on its own.

A person's message, a later Report that is neither `blocked` nor `failed`, or a retry ends the hold.

It is worked out from the stored Reports and Inputs, so a restart keeps it.
### Retry

Available while an open Ticket is in `person / delivery`, `person / blocked` or `person / failed`: a new Assignment, which starts its worker again and takes the Inputs that wait, is the remedy for each.

Once a person's message reached a blocked or failed Ticket, the Executor has the message, so the Ticket shows as working, or as queued when a failed launch starts again, and offers no retry, unless delivering that message failed.

A retry ends the current Assignment and starts a new one in the same Executor Session, directory and branch.

The Executor model applies only when a Session is created; an existing Session keeps its model.
### Restart

Running Assignments end with "host restarted" and new ones start with a recovery note that tells the Executor to check its directory and git history before repeating any outside action, oldest Ticket first.

Assignments that are waiting, ready, blocked or failed stay as they are, unless an Input reached them after that Report, in which case they start again too.

Sessions, directories, Subscriptions, pending messages and the stored work below survive a restart, and so does the record of which messages each Executor Session took into its history, which a step does once it claimed them and before its model call: a message taken by a turn the restart cut off counts as read once a Report or the end of a turn follows, while one a step claimed but a cancel or error cut off before the history counts as unread.

A retry or a restart gives the new Assignment again the messages delivered to the Assignment it replaces that its Executor Session did not take, which a cancel or a cut-off step may have dropped; a copy still waiting in the Session, kept by a restart or persisted across a crash, is taken out before the new Assignment's first message, so the new delivery is the only copy.
### Stored work

What dsh-superagent does after a decision, such as relaying a candidate, a follow-up or a report to the main Session, launching an Assignment or delivering an Input, is stored first and done afterwards.

A relay or a delivery that fails, such as when its Session cannot be reached, is retried after 2 seconds, then after a delay that doubles up to a minute; a launch that fails fails its Ticket, unless the failure was a passing one, as above.

Each kind of work reports an error among the overview's system problems only after one item failed three times in a row; a failed Input delivery shows on its Ticket instead.

Relays to the main Session keep their order, and so do notices; a failed one holds back the later ones of its kind.
### Source failure

A Source that polls reads each subscribed Thread on its own: a failed read keeps its error on its Thread, is read again on the next poll, and counts towards an error of the Source, which it reports after three failed polls in a row by default; any successful poll resets the count. Its read position stays where it was, so a later poll of its Thread reads what the failed reads missed.

The Feishu event consumer reports every failure, and polling of subscribed topics keeps each failure on its Thread.

A malformed item in a Feishu page is skipped, and the Source's state counts it.
### Channel failure

Outgoing messages are stored and sent as stored work, retried with backoff from 5 seconds, doubling up to 300.

A Ticket's messages to one conversation go out in the order they were queued, so its acknowledgement comes before its later answer or result: a message waits while an earlier one of the same Ticket to the same conversation has not gone out, until that one has failed three times in a row and shows as a problem.

Messages of other Tickets, and messages of no Ticket, do not wait for it.

A message with failed attempts is a system problem until it goes out or is withdrawn.

A message is dropped only when its Source no longer listens to the conversation, or when the Source answers that the conversation can never receive it; while its Source is off, it waits.

A Source answers that for a recalled message or one Feishu no longer finds on Feishu.

A dropped message keeps its record, with when and why it was dropped and the Ticket it was for, so it is never queued again; a dropped acknowledgement reads as dropped, not sent.
### Pause processing

Stops dispatch, new Evals, Eval timeouts, Input delivery and replies; a launch already under way waits until processing resumes.

Sources keep recording Triggers, and a running Executor turn is not interrupted.

A person's message to an ended Ticket waits: the first pass after the pause resumes the Ticket, if its Mandate holds under the Policy then.

An Eval in progress does not expire during a pause; its deadline moves later by the pause's length.
### Invalid configuration

A stored Source configuration that its kind rejects disables that Source instance.

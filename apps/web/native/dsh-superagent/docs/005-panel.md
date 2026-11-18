# 005 Panel

Open **Superagent** in the DeepSeek Harness Web sidebar to supervise the agent. The panel prioritizes work that needs a person, then system health and history. This document builds on [000 Philosophy](000-philosophy.md) through [004 Prompts](004-prompts.md).

## Three questions

Opening the panel answers three questions, in this order:

1. Does anything need me now?
2. Is the system healthy and doing the right things?
3. What is the history of one piece of work, from Trigger through Eval to Ticket?

## Tabs

The tabs follow the workflow: **Overview**, **Source**, **Trigger**, **Eval**, **Execution**, and **Settings**. Until repository and account setup succeeds, Overview shows what is missing and links to the relevant setting.

| Tab | Shows |
| --- | --- |
| Overview | Tickets needing action, system problems, activity counts for the last 24 hours, and Executor load. Each block links to its detailed page |
| Source | Source state, setup problems and non-zero counts. Filtered Triggers and dropped messages cover the same 24-hour window as Overview |
| Trigger | Threads filtered by Subscribed, Evaluating, New Triggers in 24 h, or All Threads. Each Thread shows its Triggers newest first, Decisions and Subscriptions |
| Eval | Decisions with their reason and matched rules |
| Execution | Executor state, New ticket (title, optional project, tags and attachments, hold for approval, request), Pause processing, a tag filter, and Tickets filtered by Open, Pending, Working, Waiting for a Trigger or Ended. Selecting Tickets offers bulk stop for open ones, bulk cleanup for merged ones, and a bulk label edit. Tags appear on each Ticket's row and page, and **Edit labels** changes them. A held Ticket shows **Awaiting approval** and an **Approve** button; approving starts the worker. Ticket details show actions, the request in full in a **Request** section, authorization, waiting Inputs, Subscriptions, Assignments, the replies the Ticket sent to its Source, the archive file of a local ticket, the review captured at the Ticket's end with **Rebase** and, once merged, **Clean up worktree**, line-anchored review notes that can be resolved, reopened, removed, or sent to the Executor as one follow-up, the review decision (**Approve changes** or **Request changes**), the configured check's result, its earlier runs and **Run check**, related Tickets linked by number, the history of person actions, and, for local tickets, a reply box |
| Settings | Sources, Eval, Execution (directory root, prepare and check commands, blocked commands), Prompts and Deployment. Each group includes its save history |

### Reading individual records

A dropped message links to its Ticket, or to its Thread when it has no Ticket. The Source page explains whether its conversation disappeared or the Source stopped listening to it.

A human follow-up still owned by an Executor is marked **Delivered to Ticket**, without an Eval. The Thread also shows hand-back reasons and context-only Subscriptions. **Evaluate again** can send that follow-up to Eval, or retry a Trigger with no effective judgement: one filtered by its Source, an unavailable Eval, or a delivered message with neither a non-stop Decision nor an Executor acknowledgement.

The **Execution** page also offers **Enable desktop notifications** while the browser does not yet have permission. Once enabled, the Web app announces a Ticket that starts waiting on a person (awaiting an answer, review, delivery, or blocked/failed) and a Ticket that ends, even when another tab is in front; a Ticket that keeps waiting the same way is announced once.
Ticket details show the originating Trigger, Decision and reason, scope and Policy version. Assignments and secondary details are folded below the immediate actions. **Retry** is available for failed Input delivery or a Ticket waiting for a person after a blocked or failed Report. Ended Tickets show outcomes such as **Pull request merged** or **Stopped by a person**. An ended Ticket whose Mandate the current Policy still holds, whose branch was not merged and whose worktree still exists also offers **Reopen**, which starts a fresh Assignment under that Mandate. Its number, such as `T-0007`, appears on the row and the page, and **Edit title** renames it without changing the recorded request; the new title goes to the Thread label and the archive. Its review lists the branch, the commits since the Assignment's base, the changed files, and any work left uncommitted, and **View diff** shows the committed diff without leaving the panel; a workspace that is not a git repository says so instead. **Merge** fast-forwards the repository's checked-out branch to the Ticket branch and refuses while a review note is open, a review requested changes, an approval required by the settings is missing, or the configured check has not passed since the ending, **Rebase** rewrites the Ticket branch onto it after the repository moved (a conflict aborts and keeps the commits), and **Discard** removes the Ticket's linked worktree and branch; after a merge, **Clean up worktree** removes the worktree and its merged branch. Each requires confirmation and refuses unsafe states.

## Settings groups

- **Sources:** one card per Source with its switch, fields and verified account facts; the local Source also holds its archive directory, notification webhook and per-project execution root/prepare rows. GitHub also offers receive-all switches for new Issues, pull requests, comments and default-branch CI. Polling and read limits are under Advanced.
- **Eval:** the Policy version and rules, followed by **On each Source** and the Eval settings. Each Source tab contains its allowed list, membership sync, admission table and bot list. Sync runs at startup and on **Sync now**. The admission table distinguishes receiving scope, sender membership and whether the message addresses the agent. Eval settings select the accept mode, timeout and assessment model.
- **Execution:** worker model, concurrency, directory root, prepare command, check command, required review approval and blocked commands. Clicking an available variable inserts it into the prepare command. The check command runs in the worktree when a Ticket ends, and a passing run after the ending is required to **Merge**. **Require review approval** makes an approved review of the ending's changes a merge condition too; a changes-requested review always blocks.
- **Prompts:** **Edit** changes team sections, built-in parts and skills. **Full prompt** shows the assembled text and compares it with saved text or defaults. **History** lists versions. Edited built-in parts are marked changed and can be reset; templates list their required placeholders.

  An upgrade that changes an overridden default marks it **Default updated** (默认内容已更新). The panel compares the old and new defaults above the team's text. If the old default was not recorded, it compares the team's text with the new default instead. Choose **Keep my text** or **Use the new default**. Overrides for removed parts are listed below the tab bar and dropped on the next save.
- **Deployment:** startup problems with **Check again**, the state directory and machine limits. Saved limits apply without restart. The DeepSeek Harness Plugin Manager exposes the same page.

## Search

The header search finds Triggers by text, id, Feishu message id or a stored link, such as a GitHub comment URL. A Feishu link missing from the original message becomes searchable only after the host reads it during the current process lifetime.

Search also finds Threads by references such as `#12` or a GitHub URL, and Tickets by title, labels, a matching Thread, the request text, the latest result, and the text of a reply the Ticket sent to its Source (the same text its archive holds). Ticket results are paged fifty at a time with **Load more**. Qualifiers are `is:open`, `is:ended`, `tag:<name>` or `label:<name>`, `author:<id or name>`, and `created:` or `updated:` with a date such as `2026-11-18`, an optional operator (`>=`, `<=`, `>`, `<`, `=`), or a full timestamp. Words without a qualifier are matched as text; an unknown `is:` value or a malformed date stays text.

## Principles

1. **Focus on what matters.** Every element answers one of the three questions, or it is removed. A number appears only when it changes what the supervisor does.
2. **Precise definitions.** Categories have checkable definitions, are mutually exclusive and cover every case, as the Ticket stages do. A label says whom a row waits for and what it needs, not an internal status code.
3. **One place per concept.** Source state belongs in Source; Threads and Triggers in Trigger; Decisions in Eval; Tickets in Execution. Settings are grouped by the concept they configure. Overview summarizes these pages and links to them instead of becoming another detailed view.
4. **Domain structure.** The panel uses the domain terms. Internal identifiers appear only in folded details, and names replace identifiers where they can be resolved.
5. **Honest state.** A count equals the length of the list it links to, and no state contradicts another fact.
6. **Quiet by default.** Healthy states take no extra space. Problems must be actionable; a one-off network error is not enough. Rows needing a person offer their action directly. Keep page explanations to one sentence and put further detail in the (?) tooltip. Destructive actions require confirmation.
7. **Consistent controls.** Use DeepSeek Harness design tokens and control behavior. The same information should look the same throughout the panel. Each settings block has one save bar, visible only while changes exist, a save runs or a save fails.

# 006 Deployment

Use this guide to install, configure and operate dsh-superagent. It covers the optional dsh-im-backed Feishu intake, startup checks, upgrades and troubleshooting, building on [001 Domain model](001-domain.md) through [005 Panel](005-panel.md).

## Built-in copy

This checkout mounts dsh-superagent in its Web profile: after `pnpm install && pnpm run build`, `pnpm dsh web` serves the **Superagent** panel, with the local ticket Source on and the dsh-im-backed Feishu Source idle until its handoff switch is on. Restart an already running `pnpm dsh web` after a rebuild so the host loads the new plugin build. This build has no GitHub Source: there is no repository connection, pull request or PR CI, and review and merge are local. Setup needs no repository and no account: work starts from the panel or the main Session (see Local tickets). There is no separate profile and no `dsh plugin add`. The state directory defaults to `<DSH_HOME>/state/superagent`; set `stateDir` in the profile's `cordis.patch.yml` to move it. Configure the Policy and Execution in the panel; the Management API below is the same one the panel uses and follows the Web server's bind address. The rest of this document describes the standalone installation and its operations, which apply to the built-in copy too.

### Quick start

1. `pnpm install && pnpm run build`, then `pnpm dsh web` (or Desktop), restarting it if it is already running, and open **Superagent** in the sidebar. A new state directory starts empty — no Tickets, Triggers, Decisions or saved settings — and no Source has to be turned on: the local ticket Source is ready at the first request.
2. In **Settings → Execution**, set the directory root, a prepare command when each Ticket needs its own checkout (see the prepare-command example under Settings below), and a check command when a merge should wait for tests: each line runs in the worktree when a Ticket ends, and a merge requires it to pass since the latest ending. The default root is `<stateDir>/work`.
3. Create the first ticket under **Execution → New ticket**, with a title and request plus, when they apply, a project label, tags, up to five attachments and **hold for approval**, or ask the main Session, opened by **Open main Session** in the panel header. The first request seeds the local Source's Policy part with the operator on the allowed list, so no extra configuration is needed, and both paths write the same Thread and Markdown archive.
4. Optionally hand a dsh-im Feishu bot to superagent (see Enabling Feishu) and mention it in a configured chat.
5. Follow the run on the ticket page: the **Replies and results** the Ticket sent, the review with **View diff** and line-anchored notes, and **Approve changes** or **Request changes**.
6. **Merge** the ticket once its gates pass (see the merge gates under Local tickets), or ask the main Session to merge it, push it, undo the merge or stop it.

A worker runs with `danger-full-access` as the operator's OS user, and its command shims block names instead of isolating the process; see Permissions and isolation before handing it a shared machine.

### Local tickets

With the `local` Source on, **Execution → New ticket** takes a title and a request, and asking the main Session in the DSH chat for work records the same kind of request: it archives the request and tells you the ticket id once accepted. The request becomes a Trigger, the main Session accepts it, and its Executor works in a directory under the Execution settings (an empty prepare command leaves it empty). Every ticket has one Markdown file: `<archive>/<request id>.md`, where `<archive>` is the local Source's **Archive directory**, or `<stateDir>/tickets` when empty. The file holds the request, then one section for each reply the agent sends (acknowledgement, result, notice), so the run's outcome is recorded next to the request.

- On a fresh install, the first request saves the Source's Policy part with the operator on the allowed list, so no extra configuration is needed. Removing that entry stops new tickets until it is added back.
- The ticket page lists the same replies under **Replies and results**, with each message's delivery status, so the panel and the file agree.
- When the Ticket ends, the file receives a **Closed** section with the outcome, the reason, the last summary, the git facts of the workspace (branch, commits since launch, changed and uncommitted files), and the configured check's status and command. A person who reopens the Ticket from its page appends a **Reopened** section, and the next end appends a new close. Each section is idempotent, so a retried host operation does not duplicate it.
- Reopening is available on an ended Ticket whose Mandate the current Policy still holds and whose branch was not merged and whose worktree still exists; it starts a fresh Assignment under that Mandate. Stopping stays permanent for later messages on the Thread, so an accidental stop is undone with **Reopen**, not with a comment.
- The panel's new-ticket form sends an idempotency key with each draft. A retry after a lost response returns the same Ticket instead of creating a second one; editing the title or request starts a new key.
- A local ticket's page also has a reply box. Its message is appended to the same file as a follow-up, delivered to the Ticket's Executor as an Input, and resumes an ended Ticket. The main Session can send the same kind of follow-up with `orchestrator_answer` when the person answers in the DSH chat, and it can merge an ended Ticket with `orchestrator_ticket_merge` on an explicit request; that merge applies the same review, note, check and dependency gates as the panel, rebases a branch the repository moved past, runs the configured check when it has not passed, pushes only when asked with `push`, and removes the worktree afterwards when asked with `cleanup`; a rebase that conflicts resumes the Ticket with the conflict, which its worker resolves. The main Session records with `orchestrator_ticket_dependency` the same "waits on before merging" dependencies the ticket page edits, and with `start` it also holds the waiting Ticket's first Assignment until they merge. Renaming, labelling and a separate rebase stay on the ticket page; the main Session also stops or discards a Ticket, withdrawing its queued merge, records an anchored note, approves or rejects its changes, resolves its review notes, reruns its configured check, retries a failed push, cleans up a merged worktree, and pauses or releases the merge queue; a stopped Ticket merges only on an explicit request, not from the queue or a standing one. Use either to answer the agent's question when a Ticket waits for a person. A stopped Ticket does not resume on a message; reopen it first. A merged, discarded or cleaned Ticket does not resume either: its message goes back to Eval.
- Search reaches the request text, the reply text, the title, the labels and the latest result, so a phrase the agent wrote in its result finds the Ticket and the archive that holds it. Qualifiers narrow the Ticket list: `is:open`, `is:ended`, `tag:<name>` (or `label:<name>`), `author:<id or name>`, and `created:` or `updated:` with a date or a full timestamp after an optional `>=`, `<=`, `>` or `<`, for example `is:open tag:backend created:>=2026-11-01`.
- An ended Ticket whose directory is a linked git worktree also offers **Merge**, **Rebase** and **Discard**. Merge takes the chosen strategy: fast-forward, a single squashed commit, or a merge commit; a squash or merge commit carries the Ticket's title, the worker's report summary, and its number. Each moves the branch checked out in the main working tree to the Ticket branch and refuses when that tree has uncommitted tracked changes, an open review note remains, a review requested changes, an approval required by **Require review approval** is missing, the configured check has not passed since the ending. All three require the Ticket branch to contain the repository's commit, so none leaves a conflict state; a branch left behind by newer repository commits is refused with the target named, and **Rebase** rewrites the Ticket branch onto it (a conflict aborts and keeps the commits). Discard removes the worktree and deletes the branch, uncommitted work included. After a merge, **Clean up worktree** removes the worktree and its merged branch with git's safe deletion check; after a squash merge it removes the branch without that ancestry check because the squash commit is not an ancestor. The prepared worktree example above produces exactly this layout; a plain directory gets none of these actions.
- The local Source also has an optional **Notification webhook** (http(s) URL). When a local Ticket asks a person a question, ends, is merged or reverted, needs a required review, fails the configured check, or gives up a standing merge, the host POSTs a small JSON body (`event`, `at`, `ticket{id,number,title}`, plus `question`, `outcome`/`reason`, or `text`) with a five-second timeout. It is best effort: a failure is logged and never changes the Ticket. Keep any secret in the URL and use a private receiver; the host sends no credentials.
- **Execution** offers **Enable desktop notifications**: the Web app then announces a Ticket that starts waiting on a person or ends, while the app is open. Permission is per browser; turning it on stores the choice in that browser only. The header's notification bell keeps the same events in a persistent inbox with unread counts, per-notification links and **Mark all read**.
- The new-ticket form can **hold for approval**: the Ticket is accepted and acknowledged, but no worker launches until **Approve** on its page. Use it for work queued from a script or for requests that should be reviewed before any command runs.
- The new-ticket form accepts up to five attachments, 8 MB in total. They are copied beside the archive under `<request id>.files/` with mode 600, their absolute paths are listed in the request and in an **Attachments** section of the Markdown file, and the worker reads them like any local file. A retried submission with the same key does not duplicate them.
- **Edit title** on a Ticket changes its display title and the local Thread label; the original request text stays, and the archive receives a **Title** section. Request changes belong in the reply box, which appends a follow-up.
- A Ticket's **Changes** section takes review notes anchored to a file and optional line, such as `src/login.css:42`, and the review decision **Approve changes** or **Request changes**. **Send notes to the Executor** hands the unsent open notes to the Ticket's worker as one note, resuming an ended Ticket; the main Session does the same with `orchestrator_ticket_note` and `review: true`, and hands a failing check with `check: true`. A note records when it was handed over, so it is not sent twice, and a reply or a reopen makes it due again; notes stay on the Ticket until removed. A note can also be replied to from the page, and replies stay in its discussion. A note can be marked resolved or opened again, **Resolve all** resolves every open note, and an open note keeps the Ticket from merging; a changes-requested review does too, and **Require review approval** in Execution settings makes an approval necessary.
- A Ticket's page can link another Ticket by its number, such as `T-0003`. Both pages list the link, which records related or dependent work but changes no execution state. The page also lists the person actions taken on the Ticket, and the Execution list's selection stops open Tickets, cleans up the worktrees of merged ones, or replaces the labels of any selected Tickets in one action.
- The Execution list filters by tag when any request has one.
- A crash between an ending and its archive write is repaired after the next startup: the close, approval, rebase, merge, cleanup, discard, rename, label, request and review sections are completed from the database. A deleted or moved archive file is not recreated.
- A request may carry up to eight **tags** (for example a component and a priority), stored in the archive's front matter and shown on the Ticket's row and page. **Edit labels** on the Ticket replaces them (up to eight, each 1 to 24 characters), and the archive receives a **Tags** section with the new list; the front matter keeps what the request was created with. They label work for a person; they grant no authority, are not sent to the worker, and are matched by search.
- **Settings → Sources → 本地工单 → 项目** maps a project label to its own execution root and prepare command. A ticket whose request names that project is created under `<root>/<ticket name>` and runs the row's prepare command instead of the global one; an empty root keeps the Execution root, and an empty command keeps the global command. Worker model and concurrency stay global.
- A request may carry an optional **project** label (letters, digits, dot, dash, underscore). It is written to the archive's front matter and exposed to the prepare command and worker as `DSH_SUPERAGENT_PROJECT`, so one prepare command can create a worktree from a different repository or directory per project, for example a `case "$DSH_SUPERAGENT_PROJECT"` branch. A configured project row can also set the execution root and prepare command, as described above. The worker model and concurrency stay global.
- Every Ticket has a stable display number in creation order, such as `T-0007`, on its list row, its page and the archive's closing section. Tickets are never deleted, so a number does not change.
- A command-line client can queue a request without the panel: `scripts/dsh-superagent ticket --title "..." --text "..."` writes a JSON file into `<stateDir>/inbox`, and the running service turns it into a Ticket within a couple of seconds. `--text-file -` reads standard input, `--state-dir` selects a non-default state directory, and `--wait 30` waits for the service and prints the result (request id, Thread, Trigger and archive file). Processed files move to `inbox/done/`; invalid or refused ones move to `inbox/rejected/` with the reason. The file name is the idempotency key, so re-queueing the same file returns the same Ticket.
- `scripts/dsh-superagent backup` copies the state database and the ticket archives into `<stateDir>/backups/<timestamp>/`. It uses SQLite's online backup, so the service may keep running; the default `tickets/` directory, or the local Source's configured archive directory, is copied beside it. It does not prune old backups, and an archive on another machine's disk is only as safe as that disk.
- Set **Execution → Directory root** to a checkout or workspace when the work needs files; the archive directory may point into that checkout to keep tickets with the code. To give each ticket a checkout of an existing repository, leave the root at its default and use a prepare command such as:

  ```sh
  git -C /path/to/the/project worktree add "$DSH_SUPERAGENT_DIR" -b "agent/$DSH_SUPERAGENT_NAME" HEAD
  ```
- The worker git configuration carries the neutral author `dsh-superagent <dsh-superagent@localhost>`; a worker that does not set its own identity commits under it, and the Delivery requirements prompt can name another author.

### Enabling Feishu

The Feishu Source uses a bot that dsh-im already owns; it needs no `lark-cli`, app secret or separate bot in superagent.

1. In dsh-im, configure the Feishu bot under **IM 机器人 → 飞书** and keep it connected.
2. In superagent, open **Settings → Sources → Feishu**, choose that bot, and set the chats, direct messages and other-chat mode. **刷新群列表** reads every chat the bot has joined into the picker, which also lists the chats saved as dsh-im delivery targets; while the handoff is off only the delivery targets are named, so enter a chat ID directly if the one you want is missing.
3. In the same bot's dsh-im settings, open **通用** and turn on **交给 Superagent**. The bot's messages now become Triggers for this Source; superagent acknowledges, works and replies through dsh-im. Check that bot's **访问设置** too: the handoff keeps that policy, and a bot migrated from an older version starts with an empty allowlist that admits only its owner, so switch it to allow all users or list the team.
4. Turn the same switch off to release the bot and return it to normal dsh-im conversation. The Source then stays idle until the switch is on again.

Each Ticket keeps the bot that created it, so changing the selected bot later affects new Tickets only. The handoff is exclusive while it is on: dsh-im's own Session bridge, cards and slash commands for that bot do not run.

superagent recovers what the live stream misses: every minute it reads each configured chat and each known direct chat from a stored cursor and polls subscribed threads, so a restart does not drop messages. What the bot answered while it was not handed over is not answered again: the backfill leaves those messages out. An upgrade repairs itself on start: a Feishu Topic Thread an earlier version recorded under its Topic id is re-keyed to the message that opened the Topic, so a Ticket that followed one receives the Topic's replies again. The bot's own access policy still gates what reaches superagent, and dsh-im's group archive keeps recording while it is handed over, so the bot's settings are not bypassed. A Feishu bot migrated from an older version keeps an empty allowlist, which admits only its owner: switch its access mode to all users or list the team, or nothing but the owner reaches superagent. A refused message gets no reply; the bot's log has one warning per refusal. Messages from a bot or app are not judged by that list; the Policy's bot list decides them. It receives text, rich-text (`post`) bodies, images, and a voice message as what was said when the bot's speech-to-text is on, plus a one-line placeholder (with its resource key) for files, untranscribed voice, video, stickers, cards, and shared chats and users; a merged forward is expanded into its sub-messages when the bot can read them, a body over 16 000 characters keeps its beginning and says it was cut, and the bot's own messages are never taken as new work, though they stay in a context window so a follow-up can see what the bot already answered; images — of the source message or of any message the context reads return — are downloaded through dsh-im and shown to the model as attachments, and a file, voice message, or video of any of those messages can be read the same way: dsh-im downloads it, superagent stores it verbatim, and the model receives a saved path to read with its file tools, so only text-like contents are useful. A message whose handling fails stays in the Source's health for ten minutes instead of disappearing behind the next "Listening" report, because dsh-im only logs the rejection. A recalled message reads as recalled (`deleted`) instead of looking like an empty one, keeping its place in a Thread window with no text, and once the Trigger's own message was recalled its context reads answer `unavailable`, except a `/thread` listing of a message inside a Thread, which still reads; a message that sits in no Thread reads as the surrounding chat, which the read's `status` names. Replies go back as rich-text cards inside the source message's Thread. When a worker needs an answer, its question is posted in that Thread as soon as it is asked, so nobody has to watch the panel to notice it. A person who sends `/stop` in a chat — written, spoken, or as a rich-text message with nothing else — ends the Ticket that follows that Thread: the Source stops it directly, under the same Policy admission as any message there, and answers with the Ticket it stopped, or says that nothing was running (a fixed English line, since the Source has no interface language to follow). Bots listed under **Policy → Bots** may contribute inferred candidates, **Sync now** copies the chat's member list into the allowed users, and a Ticket's source link opens the original Feishu message. Reading every group message and downloading images needs the app permissions dsh-im's **补全权限** / `/repair` flow adds: `im:message:readonly`, `im:resource`, and the all-messages permission for group messages that do not mention the bot.

## What runs

- **A dedicated DeepSeek Harness profile** (standalone installation; the built-in copy uses the Web profile), called `superagent` in this guide. Create it from the Web profile template. It loads the bundle and serves its panel on `127.0.0.1`. Keep it separate from the profile you work in: the plugin changes process-wide `PATH` and git settings, and its process needs independent restarts and upgrades.
- **Optional:** a dsh-im Feishu bot handed to superagent for chat requests. Without one, local tickets are the only intake.

Plugin configuration holds the state directory and machine limits. Use the panel for everything else: Sources for the local archive and the dsh-im-backed Feishu Source, Policy for allowed lists, and Execution for worker directories and commands.

### Permissions and isolation

The bundle defines `dsh-superagent-worker` for Executors and `dsh-superagent-router` for the main Session. They appear in the DeepSeek Harness preset list but are intended only for dsh-superagent. Do not select them for personal Sessions or disable their configuration rows. The application needs them to create and resume its Sessions.

Opening a blank managed Session in the Web UI can select the profile's default preset. dsh-superagent restores its role-specific preset before the first turn.

Both presets run with `danger-full-access` as the operator's OS user and read messages from subscribed Threads. The command shims block listed commands; they do not isolate the process. A worker can execute programs by path and read the operator's home directory. For isolation, use a dedicated OS account containing only the agent's credentials.

## Prerequisites

- Node.js 22.19+ or 24+, pnpm, git 2.32+, Python 3.8+ and tmux
- A local clone of the repository the agent works in, when the prepare command creates a worktree per Ticket. Install `git-lfs` if that repository uses Git LFS.
- A model provider configured for DeepSeek Harness. dsh-superagent uses the profile's default model unless the Eval or Execution settings name another.
- For Feishu: dsh-im with a connected Feishu or Lark bot

## Accounts

### Feishu bot (optional)

Configure the Feishu or Lark bot in dsh-im, not here: create the app in the Feishu or Lark console, give dsh-im its App ID and App Secret, and keep the bot connected. dsh-im owns the credentials, the long connection and the bot identity. superagent only consumes the text events after the bot's **交给 Superagent** switch is on.

## Install

Run this block once, on a machine that has no `superagent` profile yet:

```sh
npm install -g @deepseek-ai/dsh@0.2.1-rc.3
git clone <this repository> dsh-superagent && cd dsh-superagent
pnpm install && pnpm build
dsh superagent --from-default-profile web --dump-config > /dev/null
dsh plugin --profile superagent add "link:$PWD"
```

The pinned version above is public; other versions must satisfy `engines.dsh` in `package.json`.

The fourth command creates `~/.dsh/profiles/superagent` from the DeepSeek Harness Web template. The fifth installs this bundle. Set `DSH_HOME` to relocate the default `~/.dsh` directory.

Run the profile-creation command only for a new installation. If the profile already exists, that command replaces it and deletes its `cordis.patch.yml`, including machine configuration. Do not use it for upgrades.

## Machine configuration

Copy [examples/profile.cordis.patch.yml](../examples/profile.cordis.patch.yml) to `~/.dsh/profiles/superagent/cordis.patch.yml`, replacing that file, and replace every `<placeholder>`. The Feishu Source is mounted by default and waits for its dsh-im handoff; it needs no profile override. The `webserver` entry sets the panel's port.

| Key | Required | Meaning |
| --- | --- | --- |
| `stateDir` | no (built-in copy; defaults to `<DSH_HOME>/state/superagent`), yes otherwise | Absolute path: the database, the lock, the command shims in `worker-bin`, the agent's git configuration and, by default, the Ticket directories in `work`. Back it up before upgrades |
| `promptMaxChars`, `admissionContextMaxPages`, `admissionContextTimeoutMs` | no | The largest saved prompt configuration, and how many Thread pages one assessment reads within what time |

After the first start, edit machine limits in **Settings → Deployment** or the DeepSeek Harness Plugin Manager. Changes apply without restart. `stateDir` cannot be changed there. Invalid values or unknown configuration keys prevent startup; the log identifies the key.

## Start and stop

```sh
scripts/dsh-superagent start
scripts/dsh-superagent status
```

`start` runs `dsh --profile superagent` in tmux session `dsh-superagent`, or `dsh-<profile>` for another profile. It waits up to one minute for the runtime to become ready or report a setup requirement. On a first run it prints the missing setup information. It refuses duplicate instances, including ones started outside this script.

`stop` and `restart` request graceful shutdown and wait up to five minutes for Executors. They never force-kill the process or touch other tmux sessions.

`status` prints process ids, health and setup problems. For a different profile or port, use `--profile` and `--url`, or set `DSH_SUPERAGENT_PROFILE` and `DSH_SUPERAGENT_URL`.

`status`, `start` and `restart` exit with one of four codes:

| Code | Meaning |
| --- | --- |
| 0 | The service runs normally |
| 1 | It does not run, runs twice, does not answer, or its runtime reports an error; a restart may fix it |
| 2 | It runs, but a part other than the runtime, such as a Source or a Policy sync (`sync:<source id>`), reports an error; a restart does not fix it |
| 3 | It runs and waits for setup; a restart does not fix it |

A Source that is still starting counts as normal operation. So does a Feishu Source waiting for its dsh-im handoff.

`status` returns failed parts and their details under `errors`, and setup requirements under `setup`. `start` and `restart` print those problems as lines; they exit 1 if the service never comes up. Only an intentional `stop` pauses automatic recovery.

Each profile stores logs and control files under `.artifacts/`:

- Startup logs: `dsh-superagent-<profile>-<date>-<time>-<pid>.log`, where the pid belongs to the script. The newest 20 are kept.
- Control files: `control/profiles/<profile>/`, containing the stop marker, lock and process id.

Open the URL printed in the log, then choose **Superagent** in the sidebar.

### Hourly check

Create the hourly check in the DeepSeek Harness profile you normally use, not in `superagent`. It must keep running when the agent service is down. Ask an agent in that profile to create an hourly schedule using this prompt, with the checkout path filled in:

```text
Check dsh-superagent: run `scripts/dsh-superagent --profile superagent status` in <checkout>. Exit 0: do nothing and say nothing. Exit 2: do not restart; tell me each part under `errors` with its detail, unless you already told me about the same ones. Exit 3: do not restart; tell me what `setup` says setup needs, unless you already told me. If it reports manually_stopped: true, leave it stopped. Otherwise (exit 1) run `scripts/dsh-superagent --profile superagent --respect-stop restart`, which exits the same way: 0 means it recovered, so say nothing; 2 and 3 are reported as above; if it exits 1 or fails otherwise, read the newest .artifacts/dsh-superagent-superagent-*.log and the Troubleshooting section of docs/006-deployment.md, fix what you can, and tell me what was wrong and what you did.
```

`--respect-stop` preserves an intentional stop. Only `stop` writes the marker; `start` clears it. `restart` writes no marker, so the check can recover a restart whose shutdown timed out.

## Settings

Setup must succeed before polling or work starts. Overview lists missing requirements and links to their settings. A settings change runs setup again. If you fix something outside the application, such as CLI authentication, choose **Check again**.

1. **Sources.** The local Source sets the Markdown archive directory, an optional notification webhook and its per-project rows. For Feishu, choose the dsh-im bot, chats, each chat's receive-all or mentions-only mode, and whether to receive direct messages.
2. **Policy.** The two example rules can be edited or removed. Under **On each Source**, add the people who may ask for work: the local Source lists the operator, and Feishu lists its users; until a list has someone, no request can become a Ticket. With Feishu, **Sync now** copies the chosen chat's members into the list; users can also be added manually.
3. **Execution.** The directory root, a prepare command, and optionally one check command per line, each named by writing `name: command`. The checks run in the worktree when each Ticket ends, and each keeps its own result beside the combined one; once configured, a merge requires all of them to pass after the Ticket's latest ending. To give each Ticket a checkout of the repository:

   ```sh
   git -C /home/me/src/widgets worktree add "$DSH_SUPERAGENT_DIR" -b "agent/$DSH_SUPERAGENT_NAME" HEAD
   ```

   List personal CLIs as blocked commands.
4. **Prompts.** Set the team sections: the commit author, the delivery evidence a UI change needs, and the reviews a change must request.

### Management API

The panel uses an HTTP API on the Web server that agents can also call. `SETUP.md` uses it to save settings, and the service script reads `GET state`.

Send requests to `http://<bound host>:<port>/orchestrator/api/` with the DSH Web authentication cookie and `x-orchestrator-request: 1`; the built-in copy follows the Web server's bind address, which defaults to `127.0.0.1`. The `Host` must be the server's own authority (loopback aliases included). Missing headers, a missing session, or an `Origin` from another site receive 401 or 403. On a non-loopback bind the API is reachable from that network, protected by DSH's Web authentication and the same-origin check.

API saves have the panel's permissions. They can change execution environment and setup account fields, unlike saves made through the main Session's tools.

Agents may rely on these routes and fields. The other routes in `src/app/api.ts` serve the panel and change with it.

| Request | Answer |
| --- | --- |
| `GET state` | What every page reads: `setup`, with `problem`, `null` or a setup problem with `place`, `code`, `values` and `detail`, and, once setup resolved them, `accounts` with only `feishu`, the dsh-im bot the Feishu Source is bound to (`botId` and its stored name), or `null`; `health`, an entry per part, below, with `state` (`starting`, `ready`, `paused` or `error`), `detail` and `at`; `load`, the Executor Sessions `running`, the Tickets `queued` and `maxWorkers`; `paused`; and `masterId` |
| `GET settings` | `blocks`: `policy`, `eval` and `executor`, each with its `value` and its `revision`; `sources`: the Source instances, each with `id`, `kind`, `installed`, its block's `value`, `{"enabled": …, "config": {…}}`, and `revision`, and its kind's form; `policyVersion`; and `environment`, the execution environment's `root` and `variables` |
| `GET sources` | Each Source's status: `state` (`missing`, `off`, `error`, `setup`, `starting`, `paused` or `ready`), the `health` of its parts, the last day's `stats`, the Triggers it `filtered` in that day, and the messages it `dropped` in that day, newest drop first, each with its `channel`, `preview`, `ticket` (`id` and `title`) when it was for one, its `thread` when the store has it, and `dropped.at` and `dropped.reason`, `channelGone` or `channelIgnored` |
| `PUT settings/<block>` with `{"value": …, "revision": n}` | Saves one block: `saved`, the block's new `revision`, `policy_changed` and `restarting`. `value` is the whole block, as `GET settings` holds it; a missing key takes its default |
| `POST local/requests` with `{"title", "text"}` and optional `key`, `project`, `tags`, `hold`, `files` | Creates a local ticket and archives it: `request`, `thread`, `trigger` and `file`. `key` makes a retry idempotent; `files` are up to five base64 `{name, data}` attachments; `hold` waits for approval |
| `POST local/followups` with `{"thread": …, "text": …}` | Appends a person's message to a local ticket's Thread and archives it: `trigger` and `thread` |
| `GET tickets?state=<filter>&tag=<tag>` | Ticket rows filtered by stage and, optionally, an exact request tag |
| `GET tags` | Every tag on a request, sorted, for the Execution filter |
| `GET merge-queue` | The merge queue the panel shows: `paused`, and the waiting entries with `ticket_id`, `number`, `title`, `position`, `strategy` and, for a failed one, `error` with its time, message and gate |
| `POST merge-queue/pause` with `{"paused": true}` | Holds or releases the queue, which keeps its order: `paused` |
| `POST merge-queue/retry` with `{"ticket_ids": ["T-0003"]}` | Clears the recorded failure of queued Tickets so the next pass tries them again: `retried` and the `queue` |
| `GET notifications` | The newest panel notifications with `unread`; `POST notifications/<id>/read` marks one read, and `POST notifications/read-all` marks every one read |
| `POST tickets/<id>/reopen`, `POST tickets/<id>/approve` | Reopens an ended Ticket under its Mandate unless its branch was merged or its worktree was discarded or cleaned up, or approves a held Ticket: `reopened: true` / `approved: true` |
| `POST tickets/<id>/title` with `{"title": …}` | Renames a Ticket without changing its request: `title` |
| `POST tickets/<id>/request` with `{"text": …}` | Replaces the request text the Ticket shows; the originating Trigger is unchanged: `request`, with `text` and `at` |
| `POST tickets/<id>/tags` with `{"tags": []}` | Replaces a Ticket's labels (at most eight, each 1 to 24 characters; an empty list clears them): `tags` |
| `POST tickets/<id>/retry`, `POST tickets/<id>/stop` | Retries or stops a Ticket, as the panel does |
| `POST tickets/<id>/merge?strategy=ff\|squash\|merge` with `{"push": true}` to push afterwards, `POST tickets/<id>/discard` | Merges the Ticket branch into the repository by the chosen strategy (default `ff`) (`merged: true`, `branch`, `target`, the `head` commit the target branch holds afterwards, and `rebased` or `recaptured` when the merge had to rebase the branch or capture its review again, plus `pushed` or `pushError` when it pushed; a squash or merge commit carries the Ticket's title and report summary), refusing while a review note is open, a review requested changes, an approval required by `requireReview` is missing, a Ticket it is blocked by has not merged, or the configured check has not passed since the ending; a rebase that conflicts resumes the Ticket for its worker and answers `400` with the conflict, or removes its linked worktree and branch (`discarded: true`) |
| `POST tickets/<id>/rebase`, `POST tickets/<id>/cleanup` | Rebases the Ticket branch onto the repository's checked-out branch (`rebased: true`, `branch`, `target`; a conflict resumes the Ticket for its worker and answers `400`), or removes a merged Ticket's worktree and merged branch (`cleaned: true`; a squash-merged branch is force-deleted because its commit is not an ancestor) |
| `POST tickets/<id>/revert` with `{"push": true}` to push the reverting commit | Undoes the Ticket's recorded merge on the branch it moved: `ticket_id`, `branch`, `target` and `commit`, plus `pushed` or `pushError` when a push was asked for; a fast-forward merge is undone commit by commit from the base the merge recorded; refused when the Ticket was not merged or was already reverted, the repository is on another branch or dirty, the merge commit is gone, or the revert conflicts |
| `POST tickets/<id>/push` | Pushes the branch a merge or a revert moved, for one that was never pushed or whose push failed: `ticket_id` and `pushed: true`; refused when the repository is on another branch, and a failure is recorded on the merge or the revert and answered `400` |
| `POST tickets/<id>/dequeue` | Takes the Ticket out of the merge queue: `removed` and the `queue` after it, each entry with `ticket_id` and `position` |
| `POST tickets/<id>/review` with `{"decision": "approved"}` or `"changes-requested"`, optional `note` | Records a person's review of an ended Ticket's changes: `reviewed`, `decision`, `at` |
| `POST tickets/<id>/check` | Runs every configured check in the Ticket's worktree: `checks`, the combined record with `command`, `ok`, `exitCode`, `at`, `output` and `truncated`; the per-check runs appear on the Ticket page, and the run is refused when the Ticket changed while they ran |
| `POST tickets/<id>/related` with `{"ticket": "T-0003", "linked": true}` | Links or unlinks another Ticket, named by id or number, on both pages: `related`, each with `id`, `number` and `title` |
| `POST tickets/<id>/automerge` with `{"cancel": true}` | Clears the standing merge request the main Session recorded with `orchestrator_ticket_automerge`, which merges once the gates pass: `auto_merge: false` |
| `POST tickets/<id>/blockers` with `{"ticket": "T-0003", "blocked": true, "start": true}` | Records or clears that the Ticket merges only after another Ticket's branch has merged, the other named by id or number, and with `start` also waits to begin its first Assignment until then: `ticket_id` and `blocked_by`, each with `id`, `number`, `title`, `merged` and `start`; refused for the Ticket itself or a dependency that would close a cycle |
| `POST tickets/<id>/notes` with `{"file", "line", "endLine", "text"}`; `POST tickets/<id>/notes/<noteId>/remove`; `POST tickets/<id>/notes/<noteId>/resolve` with `{"resolved": true}` or `false`; `POST tickets/<id>/notes/resolve` with `{"resolved": true}` or `false`; `POST tickets/<id>/notes/<noteId>/replies` with `{"text": …}` | Adds, removes, resolves one or all, or replies to a review note anchored to a file, an optional line and an optional range end: `notes` |
| `POST tickets/<id>/handback` with `{"text": …}`, `{"notes": true}`, `{"check": true}`, or a combination | Hands text, the unsent open notes, or the failing check's output to the Ticket's worker, resuming an ended Ticket: `ticket_id`, `resumed`, and `sent`, the notes handed over |
| `GET tickets/<id>/diff?since=<commit>` | The committed diff since the Ticket's base, or since `since` when a review covered an earlier commit: `diff`, `truncated`, one `files` entry per file (paths, status, added/removed counts and the patch), and `error` when git cannot read it |
| `POST executor/pause` with `{"paused": true}` or `false` | Pauses or resumes processing: `paused` |
| `POST setup/check` | Runs setup again by restarting dsh-superagent: `restarting: true`. Refused when setup has nothing to fix |
| `POST policy-sync/<source id>` | Syncs that Source's allowed list now: `policy`, the Source's part as saved |
| `GET prompts`, `PUT prompts` with `{"config", "baseRevision"}` | Reads the prompt configuration: `current`, with its `revision` and `config`, `defaults`, `sections`, the team sections in the order Sessions read them, `skillName`, the pattern of a team skill's name, `builtin`, the names a team skill cannot take, and `skillPrefix`, which Sessions load a skill's name under. Or saves it: the new `revision` |

Health entries are grouped by responsibility:

| Entry | Reports |
| --- | --- |
| `runtime` | The application itself |
| `models` | Sessions waiting on a model-budget retry |
| `evals` | Unavailable Evals from the last hour with no later judgement or dismissal |
| `assessment`, `launch`, `delivery`, `relay`, `notice`, `reply` | Stored work described in [003 Execution](003-execution.md); delivery counts waiting work, while reply covers Channel messages |
| `worker` | Executor Session event handling; appears after its first failure |
| `feishu`, `history` | Feishu events and history recovery |
| `sync:<source id>` | Allowed-list synchronization |

A refusal has a status and `error`, its English text:

- 400: an invalid value (`issues`, each with its `path`), a failed account check (`setup`), a fixed refusal (`code` and `values`), or a body that is not JSON or is too large.
- 404: something that does not exist.
- 409: a stale revision.
- 500: anything else.

The blocks:

- `source:local`, `source:feishu`: `{"enabled": true, "config": {…}}`. The local Source's config has `archive` (an absolute path, or empty for `<stateDir>/tickets`), `webhook` (an http(s) URL, or empty) and `projects`, each `{"name", "root", "prepare"}`; Feishu's has `botId` (a bot configured in dsh-im), `chats` as `[{"chatId": "oc_…", "name": "…", "listen": "mentions"}]` (or `"all"`), `directMessages`, `otherChats` as `"none"` or `"mentions"`, and the advanced `readPageSize` (at most 50, the dsh-im and Feishu page limit; the default is 20) and `linkTimeoutMs`.
- `policy`: `{"rules": […], "sources": {"local": {"allowed": [{"id": "operator", "name": "Operator"}]}, "feishu": {"allowed": [{"id": "ou_…", "name": "…"}], "syncChat": "oc_…", "admission": {"oc_…": {"unlistedAddressed": "explicit"}}}}}`. `bots` lists the bots whose messages may become inferred candidates. An `admission` row is keyed by chat id, `other` or `direct`; an unset cell takes its default (see [002 Authorization](002-authorization.md)).
- `eval`: `mode`, `timeoutSeconds` and `model`.
- `executor`: `model`, `maxWorkers`, `directory`, `prepare`, `check`, `requireReview` and `blockedCommands`. The processing pause is a setting of its own, `paused`, which `POST executor/pause` sets.

A save while setup is pending returns `restarting: true`, and the API is temporarily unavailable while the application restarts. Changing the Feishu bot needs no restart: the watcher re-registers the consumer on the next poll.

```sh
api=http://127.0.0.1:3082/orchestrator/api
rev=$(curl -s -H 'x-orchestrator-request: 1' $api/settings | jq '.sources[] | select(.id == "local") | .revision')
curl -s -X PUT -H 'x-orchestrator-request: 1' -H 'content-type: application/json' $api/settings/source:local \
  -d "{\"value\": {\"enabled\": true, \"config\": {\"archive\": \"/home/me/tickets\", \"webhook\": \"\"}}, \"revision\": $rev}"
```

## Setup checks

Setup uses the DeepSeek Harness subprocess service, as worker shells do. It removes environment variables whose names contain KEY, PASSWORD, SECRET or TOKEN.

Setup writes the worker environment only. Feishu credentials and the bot connection belong to dsh-im; superagent reports only whether the handoff is on and the consumer is registered, and its workers have no Feishu CLI of their own.

The process environment that setup writes:

- **Command shims** in `<stateDir>/worker-bin`, first on `PATH`: blocked commands fail, as do `gh` and `lark-cli` — dsh-im owns the Feishu bot, so a worker reaches Feishu through the orchestrator tools.
- **git configuration** in `<stateDir>/worker-gitconfig`: replaces the operator's global and system git configuration with Git LFS filters, a neutral author and an empty credential helper, so a worker commits under the identity it sets and git never prompts for credentials. Personal git settings such as an HTTP proxy do not apply.

A file in `worker-bin` or `worker-gitconfig` that dsh-superagent did not write stops setup. A Ticket directory inside a git repository whose own configuration adds credentials fails the Ticket.

## Try it

Create the ticket under **Execution → New ticket**, or ask the main Session in the DSH chat. Overview should show it at once; the agent acknowledges the request, works in its own Session and directory, and delivers in the original conversation. Its replies and results appear on the ticket page and in its Markdown file. For Feishu, mention the bot in a configured chat. An ended ticket can be merged from the panel or by asking the main Session; both run the same local merge and gates.

### Everyday use

- A Feishu message on a Ticket's Thread reaches its Executor without another mention; a person's message resumes an ended Ticket under its Mandate, unless a person stopped it.
- **Overview** lists what needs a person: an answer, a review, a blocked or failed report, an undelivered input. **Execution** offers Stop, Retry and Request status on a Ticket.
- **Pause processing** on the **Execution** tab holds dispatch, evaluation, delivery and replies; Sources keep recording events, and a running Executor turn is not interrupted.
- Keep Eval's accept mode at `observe` until the recorded decisions look right, then switch to `auto`. Only enabled rules can accept proactive work; a rule's instructions override the default process and communication for that Ticket.

## Upgrade

1. Wait until no Executor is in the middle of a turn.
2. Back up `<stateDir>/orchestrator.sqlite`.
3. Run `git pull && pnpm install && pnpm build`.
4. Run `scripts/dsh-superagent restart` and inspect its status.

A restart ends running Assignments and starts replacements with recovery notes. Team text and overridden prompt parts are retained; parts without overrides follow the new defaults. Review parts marked **Default updated** after upgrading.

A Source saved by an older build kept a `cli` command instead of a dsh-im `botId`. That key is ignored, so the Source shows **Choose an IM Feishu bot** until you pick the bot under **Settings → Sources → Feishu**; its old tickets keep their text, but their original messages read as unavailable and their replies are dropped as `channelGone`, because dsh-im keeps no reply route for a message read before the handoff.

A database migrated by a newer version cannot be opened by an older one. Rollback therefore requires the pre-upgrade backup. This build writes schema 11 and migrates none: a database that records another version is refused with an instruction to start a new state directory. A state directory written by a GitHub-enabled build is not cleaned either, since this build has no GitHub Source and does not interpret GitHub records. Saved prompt revisions keep the text they were saved with; restore the built-in team sections if an earlier revision still mentions a removed Source. After upgrading DeepSeek Harness itself, restart the running instance so the panel loads matching artifacts.

### Agent presets

The worker and router presets move shell commands to the background after 5 seconds. A Session cannot load without its preset. Keep the `dsh-superagent-worker` and `dsh-superagent-router` rows in `cordis.patch.yml` while their Sessions are in use; otherwise resume fails with `agent-preset/not-found`.

## Troubleshooting

Setup problems appear on the overview with a link to the fix, in the panel's language; the log and the API show the English text below.

| Symptom | Cause and fix |
| --- | --- |
| A Ticket fails with `The prepare command failed: …` | The output names the cause, such as a wrong clone path; fix the command, then retry the Ticket |
| A Ticket fails with `The git configuration of the repository in … sets …` | Remove the named credential or `http.*.extraheader` settings from the clone's `.git/config` |
| `… was not written by dsh-superagent` | Move the named file out of `<stateDir>/worker-bin` or `<stateDir>/worker-gitconfig`, or choose another `stateDir` |
| `The local Source is off; turn it on in Settings → Sources` | Enable the local Source on the Sources settings |
| `The local Source allows nobody; add the operator under Policy → On each Source` | Add `operator` to the local Source's allowed list |
| `The local ticket Thread is unknown` | The page names a Thread another Source owns or one that no longer exists; reopen the Ticket from Execution |
| `The local Source filtered the request` | The Source was turned off while the request was created, or its scope no longer listens; check the Source |
| A local Ticket's archive file cannot be written | Set **Archive directory** to an absolute, writable directory; the default is `<stateDir>/tickets` |
| The Feishu Source says `dsh-im is not mounted` | dsh-im is not loaded in this profile; mount it, or leave the Feishu Source off |
| The Feishu Source says `Choose an IM Feishu bot` | Pick the dsh-im bot under **Settings → Sources → Feishu** |
| The Feishu Source says `Hand to Superagent is off for this bot` | Turn on **交给 Superagent** in that bot's dsh-im settings |
| The Feishu Source says `Another consumer already owns this Feishu bot` | Another process owns the dsh-im consumer; stop it, then retry |
| The Feishu Source says `This dsh-im does not expose Feishu backfill` | The mounted dsh-im predates the checked history reads; upgrade dsh-im, or accept that messages are only received live |
| A Feishu reply does not appear | Check the Source's dropped messages on the overview: a reply is dropped when its chat is no longer received, when dsh-im no longer finds the source message (recalled, deleted, or a route that names another author or topic), or when the Ticket's Trigger predates the dsh-im handoff and kept no route; otherwise the outbox retries it and the Ticket shows the attempts |
| `status` exits 2 | A part reports an error, which a restart does not fix: `errors` names each part, as listed under Management API, with its detail; each part clears its own entry once it works again |
| `status` exits 3 | Setup waits: `setup` says what is missing, and the overview links to the setting that fixes it |
| `status` exits 1 | The service does not run, runs twice, does not answer, or its runtime reports an error; the output and the newest log say which |
| `… timed out after 90 s`, `… was stopped by …`, `… exited with code …` or `… could not run: …` | A CLI failed without printing why; run the named program by hand as the operator's user to see its own message |
| The panel says dsh-superagent is not answering | It is restarting, or did not start because of its configuration; the DeepSeek Harness log names the cause |
| `dsh-superagent configuration has keys it does not use` (log) | Remove the named keys; the message says where each value lives now |
| The log says the plugin is incompatible with the running dsh | Install a DeepSeek Harness version within the `engines.dsh` range |
| The panel shows "Failed to load plugins" after a DeepSeek Harness upgrade | Restart the running instance |
| Nothing happens after a Feishu mention | Check that the author is on the Source's allowed list, the Source is on, and processing is not paused. The bot receives mentions only in the chats it is configured for |

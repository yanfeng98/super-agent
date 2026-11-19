# Native dsh-superagent

This directory is the source-vendored copy of [`yanfeng98/nano-dsh-superagent-prototype`](https://github.com/yanfeng98/nano-dsh-superagent-prototype)
that ships with this Super Agent checkout as a built-in capability. It is not installed from npm and it is not a third-party
profile plugin.

It is a proactive coding agent that takes requests from local tickets created in its panel, GitHub, or Feishu, evaluates
them under a Policy, turns authorized ones into Tickets, and runs each Ticket in its own DSH Session and directory. Every
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
  `dsh.client` declaration), the `github-watch` and `local` Sources, and the `feishu-watch` Source (disabled).
- `packages/bundle/web-app/presets/superagent-worker.patch.yml` and `superagent-router.patch.yml` define the two agent presets
  its Sessions run under (`dsh-superagent-worker`, `dsh-superagent-router`); the Web bundle lists them in `dsh.bundle.patch`.
  Their sandbox rows use `superagent-` ids because a preset row id must not repeat a host row's.
- `packages/bundle/web-app/tests/superagent-bundle.spec.ts` guards the dependency, the mounted rows and both presets.
- `scripts/build.ts` runs the package's own `tsdown` build before the Web shell build, producing `lib/index.js`,
  `lib/client.js`, `lib/github-watch.js`, `lib/feishu-watch.js` and `lib/local.js`.
- No profile edit, plugin-market install, or `dsh plugin add` is needed. A fresh clone needs the usual
  `pnpm install && pnpm run build`, then `pnpm dsh web` (or Desktop).

Without a GitHub repository and agent account the runtime still starts: the GitHub Source reports that it is unconfigured
and polls nothing, while the local Source accepts tickets from **Execution → New ticket** or from the main Session in the
DSH chat. Configure the repository and the agent CLI in the panel (or with the Management API of `docs/006-deployment.md`)
to use the GitHub Source too.

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
6. **Local tickets and optional GitHub.** `src/sources/local.ts` (exported as `./local`) adds a panel-driven Source: the
   management routes `POST local/requests` and `POST local/followups` write `<archive>/<request id>.md`, ingest Triggers whose
   actor is `operator`, and append every outbound message to the same file; the ticket page lists those receipts under
   **Replies and results** and offers a reply box for the follow-ups (`src/app/views/tickets.ts`, `src/app/store.ts`). Setup no longer requires a repository and an account: `resolveDeployment`
   resolves an empty GitHub deployment when the Source names neither, `writeEnvironment` writes a failing `gh` shim, no
   GitHub credential helper, and the neutral author `dsh-superagent <dsh-superagent@localhost>`, and the GitHub Source
   reports `githubUnset`. Receipts record the time of their last delivery state, and the overview counts a kind's default
   Policy part (`src/app/views/overview.ts`) so a fresh install is not told that nobody may ask. `local.spec.ts`,
   `deployment.spec.ts`, `environment.spec.ts`, `http.spec.ts`, `views.spec.ts` and `panel-execution.spec.tsx` cover it.
7. **No-GitHub worker prompt.** `prompts/work.md` tells a worker that an empty `$DSH_SUPERAGENT_GITHUB_ACCOUNT` means there is no GitHub account, so it commits locally (setting `user.name` and `user.email` itself when git has none) and does not push, open a pull request or call `gh`; without this, a local ticket's worker would follow the upstream push/PR instructions and fail.
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
   dated after that ending (`POST tickets/:id/check`) and a bound pull request's failed CI refuses it too; check runs keep a capped history (`ticket.checkHistory`); a review decision (`Tickets.review`, `POST tickets/:id/review`, `executor.requireReview`) gates a merge and is archived; reviewer notes (`Tickets.addNote`/`removeNote`/`resolveNote`/`replyToNote`,
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
    the person sending the work back; a stopped, merged or cleaned Ticket, a paused Policy, or one whose Mandate no longer
    holds keeps the plain report. `orchestrator_ticket_dependency` records `blockedBy` from the DSH chat, the same dependency
    `POST tickets/:id/blockers` edits. `orchestrator_ticket_merge` takes `push`, which runs `pushWorkspace` after the merge is
    recorded and reports `pushed` or `pushError` rather than undoing the merge. `tests/merge.spec.ts` and
    `tests/chat-tools.spec.ts` cover them.

`cordis.patch.yml` in this directory is the upstream bundle layer, kept as the package's own definition and covered by
`tests/bundle-patch.spec.ts`. The Web bundle's rows and preset files are what the built-in copy actually mounts; keep the
two in step when changing a preset.

`scripts/baseline-inventory.ts` and `.oxlintrc.json` treat `apps/web/native/` as vendored source, as they already do
`vendor/`: the inventory gates (such as `verify-no-unknown-casts`) and lint keep this copy close to upstream instead of
rewriting its test idioms.

## Checks

- `pnpm --filter dsh-superagent run build` builds the Host entries and the browser client.
- `pnpm --filter dsh-superagent exec vitest run` runs the vendored suite (1,298 tests).
- `pnpm run typecheck`, `pnpm run build:check`, `pnpm run lint` and `pnpm run test:docs` cover the repository gates; the
  vendored directory is excluded from the inventory and lint corpora, as `vendor/` is.
- `pnpm dsh web` boots the Web profile: the sidebar shows **Superagent**, `GET /orchestrator/api/state` answers without a
  setup problem even when GitHub is unconfigured, and `<DSH_HOME>/state/superagent` is created.
- Usage starts at [docs/006-deployment.md#quick-start](docs/006-deployment.md#quick-start): local tickets, the agent account
  and `gh` wrapper for GitHub, the three settings groups, the first request, everyday follow-ups, Feishu enabling and
  troubleshooting.
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
- Feishu stays disabled until a bot CLI is configured; the GitHub Source needs an agent account with write access to the
  repository, and stays quiet without one.
- The upstream documents describe installing the bundle into a separate `superagent` profile. The built-in copy needs no
  such profile; `docs/006-deployment.md` starts with the built-in path.
- The management API follows the Web server's bind address. On a non-loopback bind it is reachable from that network;
  DSH's Web authentication and the same-origin request check remain the boundary.

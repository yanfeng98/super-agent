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
and polls nothing, while the local Source accepts tickets from **Execution → New ticket**. Configure the repository and the
agent CLI in the panel (or with the Management API of `docs/006-deployment.md`) to use the GitHub Source too.

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
3. **DSH 0.2.1-alpha.2 → 0.2.2-rc.3 API adaptation:**
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
   **Reopen** action restore an ended Ticket under its still-valid Mandate (`src/core/tickets.ts`, `src/core/engine.ts`,
   `src/app/http.ts`, `src/client/ticket.tsx`), and the archive records the reopening; the Changes section reads the committed diff through `GET tickets/:id/diff` (`src/app/review.ts`, `src/client/ticket.tsx`). The new-ticket form sends an
   idempotency key, so a retry after a lost response returns the same Ticket; panel search matches a Ticket's latest result
   and the text of its replies (`src/app/views/threads.ts`, `src/app/store.ts`). An ended Ticket whose workspace is a
   linked worktree offers **Merge** (fast-forward the repository's checked-out branch only) and **Discard** (remove the
   worktree and branch); `POST tickets/:id/merge` and `POST tickets/:id/discard` run through `workspaceRepo`,
   `mergeWorkspace` and `discardWorkspace` (`src/app/review.ts`), record `ticket.merged` or `ticket.discarded`
   (`src/app/runtime.ts`) and append the matching archive section (`src/sources/local.ts`). `review.spec.ts`,
   `local.spec.ts`, `views.spec.ts`, `http.spec.ts` and `panel-execution.spec.tsx` cover it.
10. **Ticket numbers and command-line intake.** Every Ticket gets a stable `T-000N` number from its creation order
   (`src/app/store.ts`, shown by `src/app/views/records.ts`, `src/client/rows.tsx` and `src/client/ticket.tsx`, and written
   into the archive's close by `src/sources/local.ts`). A request may also carry a `project` label, which is archived and
   exposed as `DSH_SUPERAGENT_PROJECT` (`src/app/execution-environment.ts`) so a prepare command can pick a repository per
   project; a request may also ask to be held (`hold`), in which case the Executor launches nothing until **Approve**
   (`ticket.hold`, `POST tickets/:id/approve`, stage `person/approval`); up to eight `tags` are stored in the archive, shown on the row and page, matched by search, and usable as an
   Execution filter (`GET tags`, `GET tickets?tag=`). Up to five attachments, 8 MB total, travel base64 with
   `POST local/requests` and are written to `<archive>/<request id>.files/` (`src/sources/local.ts`), with their paths in
   the request text and archive. **Edit title** renames a Ticket without rewriting its request
   (`Tickets.rename`, `POST tickets/:id/title`), updates the local Thread label and appends a Title section to the archive.
   At startup, `Runtime.reconcileArchives` completes close, merge, discard and rename sections that a crash left unwritten. `scripts/dsh-superagent ticket --title … --text …` drops a JSON
   request into `<stateDir>/inbox`; the local Source's poller (`drainInbox` in `src/sources/local.ts`) turns it into a Ticket
   within two seconds, writes `inbox/done/<file>` and moves bad input to `inbox/rejected/`, with the file name as the
   idempotency key. `scripts/dsh-superagent backup` copies the database online and the ticket archives into `backups/<timestamp>/`.
   `local.spec.ts` and `service-script.spec.ts` cover it.
11. **Built-in documentation.** `docs/006-deployment.md` opens with a **Built-in copy** section (no profile, default state
   directory, API follows the bind), and `README.md` and `SETUP.md` point at the built-in path instead of the standalone
   install. The rest of the upstream documents describe the standalone deployment and still apply.

`cordis.patch.yml` in this directory is the upstream bundle layer, kept as the package's own definition and covered by
`tests/bundle-patch.spec.ts`. The Web bundle's rows and preset files are what the built-in copy actually mounts; keep the
two in step when changing a preset.

`scripts/baseline-inventory.ts` and `.oxlintrc.json` treat `apps/web/native/` as vendored source, as they already do
`vendor/`: the inventory gates (such as `verify-no-unknown-casts`) and lint keep this copy close to upstream instead of
rewriting its test idioms.

## Checks

- `pnpm --filter dsh-superagent run build` builds the Host entries and the browser client.
- `pnpm --filter dsh-superagent exec vitest run` runs the vendored suite (1,217 tests).
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

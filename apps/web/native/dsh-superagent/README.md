# dsh-superagent

> **Research prototype for reference only, not an official DeepSeek product.** This code grew out of our team's day-to-day development work, with much of the engineering done by coding agents. Treat it as research code, not production-ready software. It may still contain bugs, and its coding style and engineering practices may be inconsistent. Review and adapt it before using it in your own environment.
>
> We have adapted it for publication, so it is not identical to our internal version or the implementation described in *Exploring Super-agent Design: Towards a More Autonomous Agent System*. Some designs discussed in the article are directions for future development and are not implemented in this prototype.

dsh-superagent is a proactive coding agent for one local workspace and repository, packaged as a bundle for [DeepSeek Harness](https://www.deepseek.com/en/harness/). It takes requests from local tickets created in its panel or by its main Session, and optionally from Feishu. When someone allowed asks the agent for work, it opens a work item, works on it in its own agent session and directory, commits the change on a local branch, and answers in the original conversation. A web panel shows what needs a person.

## Built-in copy in this checkout

This checkout ships dsh-superagent as a built-in Web capability. After `pnpm install && pnpm run build`, `pnpm dsh web` serves the **Superagent** entry in the sidebar; no profile, `dsh plugin add`, or separate checkout is needed. Restart an already running `pnpm dsh web` after a rebuild so the host loads the new plugin build. The local ticket Source is on by default; Feishu is optional and uses a bot already configured in dsh-im, staying idle until Hand to Superagent is on in that bot's settings. This build has no GitHub Source: tickets, branches, review and merge are local, with no repository connection, pull request or PR CI. The panel's Management API follows the Web server's bind address and still requires the Web authentication cookie and the same-origin request header. [NATIVE.md](NATIVE.md) lists the local modifications and how to sync upstream.

### Quick start

1. `pnpm install && pnpm run build`, then `pnpm dsh web` (or Desktop), restarting it if it is already running, and open **Superagent** in the sidebar.
2. Use **Execution → New ticket**, or open the main Session with **Open main Session** in the panel header and ask it for the work and for an ended ticket's merge; it rebases a branch the repository moved past, runs the configured checks before merging, hands a conflicting rebase to the ticket's worker, and pushes and cleans up when you ask. Ask it to merge once the gates pass and the host records that standing request, retries it, reports back when it merges or gives up, and merges unattended; ask it to merge several tickets and they enter a queue that merges them one after another, each answered on its own, even across a restart, and ask it to undo a merge and it commits the inverse on the branch the merge moved; it can also stop or discard a ticket, record a line-anchored review note, approve or reject its changes, resolve its review notes, rerun its check, retry a failed push, clean up a merged worktree and pause or release the merge queue, and a stopped ticket merges only on an explicit request. The ticket accepts a title, request, optional project and tags, up to five attachments, and an optional **hold for approval**; its Markdown archive and every reply live under `<stateDir>/tickets/` by default. Each Ticket works in a linked worktree of the repository this checkout belongs to by default; set **Execution → Directory root** to move the ticket directories and **Execution → Repository** to work in another git repository, each Ticket still getting its own worktree. A prepare command covers a workspace git cannot prepare that way.
3. Optionally hand a dsh-im Feishu bot to superagent: configure the bot in dsh-im, choose it under **Settings → Sources → Feishu**, then turn on **交给 Superagent** in that bot's dsh-im settings and mention it there. The bot keeps its own access settings: a bot migrated from an older version starts with an empty allowlist that admits only its owner, so switch its access mode to all users or list the team. The Source recovers messages missed during a restart from chat history, receives text, rich text, images, merged forwards expanded into their sub-messages and a one-line placeholder for the other message types, never takes the bot's own messages as new work, reads any image of the Thread back as an attachment and any file, voice message, or video back as a saved path, answers as Markdown cards in the source message's Thread, keeps consecutive images from one sender in a single Thread, and links each Ticket to its original Feishu message. See [006 Deployment](docs/006-deployment.md#enabling-feishu).
4. A Ticket can be inspected after it ends: the request in full, the committed diff file by file (each file opens on demand, can be marked viewed, and offers a comment on each line of the file after the change) and, after a review, only what changed since it, review notes on a line or a range (each resolvable, marked sent once handed to the worker, with **Resolve all**), a review decision (approve or request changes, recorded with the reviewer's name) with note discussions, editable labels and request text, the **To review** worklist that lists ended tickets still waiting for a decision, the Tickets it waits on before merging, optionally also before starting (also recordable from the DSH chat), related Tickets and the person-action history, which says whether the main Session, the panel or an automatic merge acted moves review notes with the code, and marks **Changes moved** when their own lines changed; a standing merge request is listed there and can be cancelled, a failing check names the check that failed, the merge queue on the Execution page shows its order, pause and retry for a failed entry, **Undo merge** reverses the whole merge, commit by commit after a fast-forward, and leaves the ticket mergeable again with its reverting commit pushable afterwards, **Push** sends a merge or a revert that was never pushed or whose push failed (a merge offers **Push after merging**), and the page shows the check the merge ran on the merged target; the notification bell also records merges, reverts, required reviews, failing checks and a given-up standing merge; merge it (fast-forward, squash or merge commit, the latter carrying the ticket title and worker summary; configured checks, open notes and rejected reviews must be clear first, and an approval may be required), rebase it after the repository moved, discard or clean up its worktree, and edit its title, labels or request text. A merged, discarded or cleaned Ticket cannot be reopened or resumed. A held Ticket waits for **Approve**; the header inbox, a configured webhook or a desktop notification reports work that needs a person.

The state directory defaults to `<DSH_HOME>/state/superagent`. [docs/006-deployment.md](docs/006-deployment.md#quick-start) covers setup, settings, the command-line inbox, backups, everyday use, Feishu and troubleshooting in full.

It is also a way of working with an agent: **take as little human attention as possible, and give as much value as possible.** An agent that can act for a whole team can also waste a whole team's attention, so it acts only when asked or when a rule the team wrote allows it, and speaks only when someone must act or can use a result. [docs/000-philosophy.md](docs/000-philosophy.md) explains the principle and what it asks of the operator.

## Setup

The built-in copy is already mounted: skip the installation in this section and configure the Sources in the panel. The optional Feishu bot is configured in dsh-im; the steps below cover the standalone installation.

Setup has many steps, so an agent does it with you. Ask your coding agent:

> Read SETUP.md in this repository and set up dsh-superagent with me.

It asks one short question at a time, shows the full plan once, and then installs and configures everything. [SETUP.md](SETUP.md) is what it follows.

## Documentation

Read the numbered documents in order; each builds only on the ones before it.

- [000 Philosophy](docs/000-philosophy.md)
- [001 Domain model](docs/001-domain.md)
- [002 Authorization](docs/002-authorization.md)
- [003 Execution](docs/003-execution.md)
- [004 Prompts](docs/004-prompts.md)
- [005 Panel](docs/005-panel.md)
- [006 Deployment](docs/006-deployment.md): manual setup, configuration, upgrades and troubleshooting
- [007 Architecture](docs/007-architecture.md)
- [008 Cookbook](docs/008-cookbook.md): adding a Source

## Development

`pnpm check` runs the typecheck, the tests and the build.

## License

[MIT](LICENSE)

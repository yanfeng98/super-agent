# dsh-superagent

> **Research prototype for reference only, not an official DeepSeek product.** This code grew out of our team's day-to-day development work, with much of the engineering done by coding agents. Treat it as research code, not production-ready software. It may still contain bugs, and its coding style and engineering practices may be inconsistent. Review and adapt it before using it in your own environment.
>
> We have adapted it for publication, so it is not identical to our internal version or the implementation described in *Exploring Super-agent Design: Towards a More Autonomous Agent System*. Some designs discussed in the article are directions for future development and are not implemented in this prototype.

dsh-superagent is a proactive coding agent for one workspace and one GitHub repository, packaged as a bundle for [DeepSeek Harness](https://www.deepseek.com/en/harness/). It takes requests from local tickets created in its panel, from GitHub, and optionally from Feishu. When someone allowed asks the agent for work, it opens a work item, works on it in its own agent session and directory, and answers in the original conversation, for example with a pull request. A web panel shows what needs a person.

## Built-in copy in this checkout

This checkout ships dsh-superagent as a built-in Web capability. After `pnpm install && pnpm run build`, `pnpm dsh web` serves the **Superagent** entry in the sidebar; no profile, `dsh plugin add`, or separate checkout is needed. GitHub is optional: without a repository and an agent account, the local ticket Source still runs, and the panel shows which GitHub setup is missing. The panel's Management API follows the Web server's bind address and still requires the Web authentication cookie and the same-origin request header. [NATIVE.md](NATIVE.md) lists the local modifications and how to sync upstream.

### Quick start

1. `pnpm install && pnpm run build`, then `pnpm dsh web` (or Desktop), and open **Superagent** in the sidebar.
2. **Without GitHub:** use **Execution → New ticket**. The ticket accepts a title, request, optional project and tags, up to five attachments, and an optional **hold for approval**; its Markdown archive and every reply live under `<stateDir>/tickets/` by default. Set **Execution → Directory root** or a prepare command when the work needs files.
3. **With GitHub:** sign in a dedicated agent account through a `gh` wrapper, set the repository and wrapper under **Settings → Sources → GitHub**, add at least one allowed login under **Settings → Eval**, and set a prepare command under **Settings → Execution**. Then comment `@<agent login> <request>` from an allowed account; Overview shows the Ticket within the polling interval.
4. A Ticket can be inspected after it ends: the request in full, changes and diff, review notes (each resolvable), a review decision (approve or request changes), labels, related Tickets and the person-action history; merge it (configured checks, open notes and rejected reviews must be clear first, and an approval may be required), rebase it after the repository moved, discard or clean up its worktree, and edit its title or labels. A merged or discarded Ticket cannot be reopened. A held Ticket waits for **Approve**; a configured webhook or desktop notification reports work that needs a person.

The state directory defaults to `<DSH_HOME>/state/superagent`. [docs/006-deployment.md](docs/006-deployment.md#quick-start) covers accounts, settings, the command-line inbox, backups, everyday use, Feishu and troubleshooting in full.

It is also a way of working with an agent: **take as little human attention as possible, and give as much value as possible.** An agent that can act for a whole team can also waste a whole team's attention, so it acts only when asked or when a rule the team wrote allows it, and speaks only when someone must act or can use a result. [docs/000-philosophy.md](docs/000-philosophy.md) explains the principle and what it asks of the operator.

## Setup

The built-in copy is already mounted: skip the installation in this section and configure the Sources in the panel. Preparing the accounts and CLIs still follows the steps below.

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

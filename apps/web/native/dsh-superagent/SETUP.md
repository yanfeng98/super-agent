# SETUP

> **Built-in copy:** this checkout already mounts dsh-superagent in its Web profile. Skip the install, profile and plugin steps. The local Source creates tickets in the panel under **Execution → New ticket** and needs no repository or account; Feishu is optional, and its bot preparation below applies only when the operator wants it.

Instructions for an agent that sets up dsh-superagent with its operator. If you are a person, ask your coding agent to read this file and follow it.

## How to run the setup

The setup follows [docs/000-philosophy.md](docs/000-philosophy.md): it takes as little of the operator's attention as possible.

- **Before the first question,** read only this file and `docs/000-philosophy.md`, and run the probe below once. Read nothing else until every answer is collected.
- **Ask one question per message,** about one thing. Wait for the answer before the next one. Skip a question the probe already answers.
- **Keep each message under 300 words,** unless the operator asks for more. Give each question a recommended answer the operator can accept with one word, and one sentence on why it matters.
- **Do not execute while asking.** Collect every answer, then send one message with the full plan, and execute only after the operator confirms it.
- **Never ask for a secret in the chat.** Steps that need the operator's hands, such as creating a bot app or signing a CLI in, go into one message with the exact commands to run in their own terminal.
- **Teach the philosophy through the questions,** one sentence at a time, not as a lecture.

## 1. Probe

Run from this checkout, once, and keep the result to yourself:

```sh
node -v; pnpm -v; git --version; tmux -V; python3 --version; dsh --version
command -v lark-cli
grep -l -E 'id: dsh-superagent\b' "${DSH_HOME:-$HOME/.dsh}"/profiles/*/cordis.patch.yml 2>/dev/null
```

- **Prerequisites:** Node.js 22.19+ or 24+, pnpm, git 2.32+, tmux, Python 3.8+, and DSH within the `engines.dsh` range of `package.json`. A missing one becomes an item of the plan, not a question. DSH also needs a model provider; if the operator has never run DSH with a model, configuring one is an operator step.
- **Existing profile:** a profile with a `dsh-superagent` entry is an earlier setup: ask whether to continue it, keep its configuration, and read Upgrade in `docs/006-deployment.md`.

## 2. Questions

Ask in this order, one per message, following the flow of work in [docs/001-domain.md](docs/001-domain.md): first where Triggers come from, then who may ask for work there, then what the Executor does with a Ticket. Recommendations are in brackets.

**Sources: where requests come from.**

1. **Sources.** Should the agent take requests from the panel and the main Session only, or also from Feishu chats? [Panel and main Session only, unless the team works in Feishu.]
2. With Feishu: **which chats,** by name? The agent looks up their ids after the bot is configured.

**Policy: who may ask for work.**

3. **In the panel.** Which people may create tickets in the panel? [The operator alone, as `operator`: a short list keeps the team's noise low while the operator learns how the agent behaves.]
4. With Feishu: **who may ask in Feishu?** [Everyone in the first chat, kept in sync with its members.]

**Execution: what the agent does with a Ticket.**

5. **Workspace.** Which local checkout should each Ticket work in, or from which clone should the prepare command create a worktree? [A new clone under `~/src/<name>`; the prepare command runs `git worktree add` in it.]
6. **Commit identity.** Who should the agent's commits name as author? [The neutral `dsh-superagent <dsh-superagent@localhost>`, so its work is visibly its own.] Another identity, such as the operator's, goes into the Delivery requirements prompt section.
7. **Check.** Which command should run in a Ticket's worktree when it ends, and again before a merge? [None; the operator can add one later under **Execution → Check command**.]

Do not ask about values with a good default; list them in the plan instead:

- profile `superagent`, a copy of DSH's Web profile kept apart from the profile the operator works in, so it restarts without closing their Sessions; port `3082`; state directory `<home>/.dsh/state/superagent` as an absolute path;
- an hourly check from the operator's own profile, which repairs dsh-superagent when it is not running;
- branch prefix `agent/`;
- accept mode `observe`, which records proactive work without doing it, and the default rules and prompt sections;
- with Feishu: the bot reads only mentions, and direct messages are on.

## 3. Plan

Send one message, under 300 words: what will be installed, the optional bot and its CLI, the settings to save, the operator steps, and the defaults. Ask for one confirmation.

## 4. Execute

1. **Operator steps first.** Read the Accounts section of `docs/006-deployment.md`, write the CLI wrapper yourself, then send one message with what only the operator can do: for Feishu, create and publish the bot app and run the `config init` and `strict-mode` commands with its secret; for the workspace, clone the repository if question 5 asked for a new clone. Wait until the operator says it is done, then check the bot with `lark-cli auth status`.
2. **Install and start.** Follow the Install, Machine configuration, and Start and stop sections of `docs/006-deployment.md`. Skip the `git clone` of this repository: you are in its checkout. With an existing profile, skip the `dsh superagent --from-default-profile` command, which would replace the profile and delete its `cordis.patch.yml`, and edit that file instead of replacing it.
3. **Settings.** Save them through the Management API of `docs/006-deployment.md`, in this order: the `local` Source, the `feishu` Source if used, the `policy` block with the allowed lists (for Feishu, look up the chat ids with the bot's CLI first), and the `executor` block with the directory root, a prepare command from the answer to question 5 and the check of question 7. Then save the prompts with the commit identity of question 6 in Delivery requirements. A save that returns `restarting: true` restarts dsh-superagent: wait until `scripts/dsh-superagent status` no longer exits 1 before the next request, and read the revisions again from `GET settings`.
4. **Failures.** A refused save names the setup problem in its response body. Find it in the Troubleshooting table of `docs/006-deployment.md`, fix it, and continue. Ask the operator only for what you cannot fix yourself.

## 5. Verify and hand over

`scripts/dsh-superagent status` exits 0 with no setup problem. If you run in the operator's own DSH profile, create the hourly check of the Hourly check section of `docs/006-deployment.md` with your schedule tool; otherwise give the operator its prompt to schedule there. Then send one message, under 300 words:

- The panel URL, and that the **Superagent** entry in the sidebar is where to look.
- How to try it: create a Ticket under **Execution → New ticket**, or ask the main Session in the DSH chat for the work, and review its result on the Ticket page. With Feishu, mention the bot in a configured chat. A Ticket appears on the overview at once.
- Three rules for the operator, from the Responsibility section of `docs/000-philosophy.md`: start narrow, tune for less noise, own what the agent says.
- Where to read more: the numbered documents in `docs/`, in order.

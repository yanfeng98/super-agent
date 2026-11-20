# Agent Note: Vendored engineering workflow skills

Status: implemented

English | [中文](2026-11-20-vendored-engineering-workflow-skills.zh.md)

## Problem

The engineering workflow skills this session used (grilling, specs, tickets, TDD, code review, domain modeling) were not part of the Harness. They reached a session only because the user's `~/.agents/skills` and `~/.claude/skills` held symlinks into a separate clone of `yanfeng98/nano-mattpocock-skills`, a Simplified-Chinese fork of [mattpocock/skills](https://github.com/mattpocock/skills). Three consequences:

| Symptom | Cause |
|---|---|
| A fresh checkout elsewhere has none of these skills | Distribution was local symlinks into an external repository |
| Disabling or moving that clone silently removes the skills from every session | The catalog came from `skill-filesystem` user roots, not from a shipped provider |
| The installed text drifts from the repository under review | A second copy of the corpus lived outside the checkout |

The requirement is that a clone of this repository carries the skills and turns them on by itself, without the external project.

## Decision

A dedicated bundled provider ships the corpus inside the Harness and the base bundle mounts it.

- **Package.** `@deepseek-ai/dsh-skill-mattpocock` at `packages/skill/skill-mattpocock/` registers provider `dsh-mattpocock` on `ctx.skills` at the bundled rank (600), with `source: 'bundled'`. It follows `dsh-skill-office`'s provider shape: a fixed directory-name list, a packaged `assets/` root, and per-load reads.
- **Composition.** The `skill-mattpocock` row in `packages/bundle/base/cordis.patch.yml` makes every base-backed profile (`web`, `headless`, `acp`, `sdk`) mount it with no configuration. A profile removes it by replacing the row with `disabled: true`. The provider attaches through a deferred `skills` injection, so a composition that disables the skill registry keeps the row idle instead of reporting a pending service.
- **Content.** 27 skills from the `engineering/` and `productivity/` buckets of the translation. Eleven are model-invoked and appear in the session catalog; sixteen carry `disable-model-invocation: true` and are reachable only through the `/` skill picker. `ask-matt` remains the router over the user-invoked set.
- **Frontmatter contract.** The directory name is the skill identity, so a declared `name` that disagrees is invalid metadata. `description` is required. `disable-model-invocation` and `user-invocable` are optional booleans and project into the registry's invocation policy.
- **Failure reporting.** A missing (`ENOENT`/`ENOTDIR`) or invalid skill is omitted from a complete catalog, reported once, and a load returns `undefined` after invalidating the catalog. Any other read error leaves discovery incomplete and propagates from a load.
- **Precedence.** Local discovery wins a same-name skill because it registers into the nearer preset layer; the bundled rank decides only within one layer. An existing project or user `tdd` therefore overrides the shipped one without configuration.
- **Adaptations for this Harness.** The vendored prose names the `skill` tool, drops the Codex `agents/openai.yaml` files and plugin manifests, and replaces `/clear` with opening a new session (the Harness has `/compact` but no `/clear`). The translation keeps every machine identifier: names, slash commands, labels, paths, and template field names.
- **Source.** Assets come from `yanfeng98/nano-mattpocock-skills` branch `260922-dev` at `a7b4cdd`, tracking upstream `mattpocock/skills` v1.3.x. `assets/LICENSE` carries the upstream MIT notice. The package README owns the re-sync procedure.

## Alternatives considered

- **Keep the symlink install.** This was the status quo. It lost because it requires a second clone, breaks on a fresh checkout, and makes the catalog depend on the user's home directory.
- **Populate the preset's `skills/` directory instead of a package.** `packages/preset/agent-preset/skills/` reaches only presets that mount it as a custom directory, so headless and SDK compositions would miss the skills, and third-party content would live in the preset package's own asset tree. A provider package reaches the global layer of every profile.
- **Download the corpus at startup, like `upstream-skills`.** It lost because it needs network access and an upstream host, and the requirement is that a clone works offline with no external project.
- **Keep the upstream plugin/marketplace route.** It lost because it requires an explicit install step, which contradicts turning the capability on by default.
- **Vendor the English upstream instead of the translation.** It lost because the reviewed and installed corpus is the translation, and English would regress the user's working text; a second English provider remains possible later.
- **Prefix the names (`mp-tdd`).** It lost because every cross-skill reference, `/name`, and upstream document names the unprefixed skill; renaming would fork the corpus and increase sync cost for a collision that layer precedence already solves.

## Consequences

The corpus now travels with the repository, loads offline, and cannot be removed by moving a home-directory clone.

Costs that remain:

- Every base-backed profile gains eleven catalog entries (descriptions only; bodies load on demand) and one plugin row.
- Synchronization is manual: copying the promoted buckets, re-applying the adaptations, and updating the `SKILL_NAMES` list is a reviewable diff, not a build step.
- The translation can drift from upstream English, and no gate detects it; the drift test only compares the asset directories with the provider's fixed list.
- The content is Chinese-only, so a session in English receives Chinese descriptions and bodies.
- A user-installed same-name skill silently shadows the bundled one, which is the intended precedence but can surprise during debugging.

## Testing

`packages/skill/skill-mattpocock/tests/skill-mattpocock.spec.ts` covers the provider contract at 100% statement, branch, function, and line coverage: catalog contents and invocation policy, packaged-body loading, asset-directory drift, a real `cordis.yml` composition through the Loader, persistent and transient failure reporting, deduplication, disposal, and abort. `verify-config-catalog` records the `assetRoot` field. Existing keyless scenarios republish the eleven catalog entries; `pnpm run test:snapshot:refresh` regenerated them because the replay input stayed valid.

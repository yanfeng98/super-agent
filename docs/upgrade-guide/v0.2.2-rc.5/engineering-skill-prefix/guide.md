---
kind: upgrade-guide
description: "The 27 shipped engineering skills take an eng- prefix, so /tdd and skill({ name: 'tdd' }) no longer load the built-in skill."
---

# Shipped engineering skills take an `eng-` prefix

English | [中文](guide.zh.md)

## Change

Every skill the `skill-engineering` plugin ships is renamed with an `eng-` prefix: `tdd` is `eng-tdd`, `code-review` is `eng-code-review`, `grill-with-docs` is `eng-grill-with-docs`, and so on for all 27. The prompt text, descriptions, and invocation split are unchanged.

The prefix separates the shipped text from a same-named copy in a project root (`./.dsh/skills`, `./.agents/skills`) or a user root (`~/.dsh/skills`, `~/.agents/skills`). The registry resolves a duplicate name to the nearest layer, so before the rename a personal copy replaced the shipped skill and the shipped one never appeared in the `/` menu. Both are addressable now.

The composer's `/` menu also groups entries by where they came from: `Built-in · mattpocock/skills` for the shipped set, `Built-in` for a shipped provider whose text has no upstream project, `Plugin · <plugin>` for skills that a plugin registers at runtime, and `Project · .agents/skills`, `User · ~/.agents/skills`, or `Custom` for filesystem roots. A typed query still returns the shared name ranking without headings.

The `skills/list` Remote additionally reports the contributing plugin (`owner`) and whether that plugin's provider ships with the harness (`shipped`); a `SkillProvider` may declare `shipped`, and a runtime registration may declare `owner` when its scope has no plugin name.

## Migration

1. Replace the 27 shipped commands with their `eng-` names — `/tdd` becomes `/eng-tdd`, `/ask-matt` becomes `/eng-ask-matt`. The [engineering skills guide](../../../user/guide/engineering-skills.md) and the [package README](../../../../packages/skill/skill-engineering/README.md) list the full set.
2. Update anything that addresses a shipped skill by name the same way: prompts, `AGENTS.md`, scripts, and `skill({ name })` calls.
3. Reload a profile and type `/` in the composer. The shipped set appears under `Built-in · mattpocock/skills`, beside your own copies rather than in place of them.

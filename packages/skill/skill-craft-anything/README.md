---
description: "The bundled craft-anything skill: the evidence-first investigation loop it carries, where it is mounted, and the upstream text maintainers re-sync."
kind: "package-reference"
---

# @deepseek-ai/dsh-skill-craft-anything

English | [中文](README.zh.md)

## Summary

This provider ships one bundled skill, `dsh-craft-anything`, teaching the [craft-anything](https://github.com/storytold/craft-anything) investigation loop: open a shipped target, keep a brief, plan, hypotheses, findings and handoff in a project folder, cite evidence ids for claims, enable only the toolsets a question needs, and separate observed facts, derived facts and inference. The text derives from `crates/setup/src/skill.md` and `skill_recovery.md` vendored at [`native/craft-anything`](../../../native/craft-anything/NATIVE.md) under MIT OR Apache-2.0.

The Web `standard` agent preset mounts it beside the craft-anything MCP server, so a session with the `mcp__craft-anything__*` tools has the skill that drives them; the Web host patch also mounts it for every other profile.

## Table of Contents

- [Use this package](#use-this-package)
- [Understand the implementation](#understand-the-implementation)
- [Further Exploration](#further-exploration)
- [Model Experience](#model-experience)
- [Known Limitations and Deferred Work](#known-limitations-and-deferred-work)
- [Dev Note](#dev-note)

-----

<a id="use-this-package"></a>
## Use this package

Mount the provider beside the skill registry and `dsh-tool-skill`. It registers into the agent's skill layer under the provider name `dsh-craft-anything` at the bundled skill rank (600) and declares `shipped`, so the composer lists it with the rest of the harness.

```yaml
- id: skill-craft-anything
  name: '@deepseek-ai/dsh-skill-craft-anything'
```

| Field | Default | Meaning |
|---|---|---|
| `assetRoot` | packaged `assets/` | Absolute directory holding the `dsh-craft-anything` skill directory. Set it to serve a relocated copy, as tests do. |

A relative `assetRoot` fails at load. To remove the skill from a profile, replace the mounting row with `disabled: true`.

### What the catalog contains

One entry, `dsh-craft-anything`, reachable by the model and by the user: its description matches requests about how a shipped program, bundle, firmware image or capture works, and `/dsh-craft-anything` loads the body on demand. The entry carries the upstream project `storytold/craft-anything`, so the composer groups it under `内置 · storytold/craft-anything` apart from the harness-only skills.

A same-name skill from a nearer layer wins: a project or user skill directory supplies the catalog entry and body instead of this provider, and within one layer the bundled rank sorts last.

### Observable success and failures

The skill appears once in the catalog with its description, loads the packaged body and recovery reference, and resolves its resource directory against the packaged assets. A missing or invalid `SKILL.md` is omitted from a complete catalog and reported once through `ctx.logger.warn`. Any other read error leaves discovery incomplete, so the registry retries it, and a load rejects rather than serving a partial body. Disposing the plugin removes the candidate.

-----

<a id="understand-the-implementation"></a>
## Understand the implementation

<details>
<summary>Implementation internals — click to expand</summary>

The provider holds the skill directory name, reads `assets/<name>/SKILL.md` during discovery and again on each load, and validates the frontmatter contract before publishing a candidate. The directory name is the identity: a declared `name` that disagrees is invalid metadata. `description` is required and non-empty. `disable-model-invocation` and `user-invocable` are optional booleans and project into the registry's invocation policy; the packaged file declares neither, so the skill stays model- and user-invocable.

The provider attaches through a deferred `skills` injection, so a profile that removes the skill registry leaves the row idle instead of reporting a pending dependency.

Failures separate two cases. `ENOENT` and `ENOTDIR` persist until the file appears, so the skill is omitted from a complete catalog and a load returns `undefined` after invalidating the catalog. YAML, field, or naming errors are also persistent. Every other read error leaves the observation incomplete and propagates from a load.

| File | Role |
|---|---|
| [`src/index.ts`](src/index.ts) | Plugin entry, frontmatter contract, provider registration, failure reporting |
| [`assets/dsh-craft-anything/SKILL.md`](assets/dsh-craft-anything/SKILL.md) | The evidence-first loop, its toolset loading notes, and the built-in connection section |
| [`assets/dsh-craft-anything/references/recovery.md`](assets/dsh-craft-anything/references/recovery.md) | Diagnosis of a missing tool or a failed connection, adapted from upstream's recovery guide |
| [`assets/LICENSE`](assets/LICENSE), `LICENSE-APACHE`, `NOTICE` | The upstream dual-license notices covering the adapted skill text |

</details>

-----

<a id="further-exploration"></a>
## Further Exploration

- [craft-anything mount](../../mcp/craft-anything/README.md) — the MCP server and CLI this skill drives, mounted by the same preset.
- [Skill subsystem reference](../../../docs/subsystems/skills.md) — the registry, provider contract, and catalog rendering this provider plugs into.
- [skill](../skill/README.md) — the registry and invocation policy types.
- [Native source baseline](../../../native/craft-anything/NATIVE.md) — the pinned upstream tag, kept paths and sync procedure.

-----

<a id="model-experience"></a>
## Model Experience

Indirectly, through `dsh-tool-skill`, which renders the catalog entry and the loaded skill body.

#### KV Cache effect

The entry's description sits in the session catalog from the first request. The body enters the conversation only at the load point, so a session that never asks about shipped software pays for the description alone. Enabling a toolset mid-session changes the tool catalog, which invalidates a warm prefix from that point on.

## Known Limitations and Deferred Work

<a id="known-limitations-and-deferred-work"></a>

These limits define what the bundled provider does not do. They are current package constraints, not a task backlog.

- **One installation's wording** — the body names the staged `native/craft-anything/bin/craft-anything-cli` executable and the Web `standard` preset. Its recovery guide deliberately ignores upstream's `setup` and `uninstall` commands, which register servers in other agent apps and do not apply to this built-in copy.
- **English only** — the skill text is the upstream English corpus; the plugin metadata is localized, but the catalog entry and body are not, so a localized catalog needs a local override.
- **Manual synchronization** — upstream changes arrive by copying `native/craft-anything/crates/setup/src/skill.md` and `skill_recovery.md` and re-applying the built-in connection wording; no build step tracks the upstream repository.
- **No per-skill configuration** — the provider exposes one asset root and always publishes the packaged skill.

<a id="dev-note"></a>
### Dev Note

<details>
<summary>Working context for maintainers — click to expand</summary>

The packaged text is the upstream `skill.md` and `skill_recovery.md` copied from the vendored Rust workspace, renamed to `dsh-craft-anything`, with the "Connect only when you need it" section replaced by built-in mounting, toolset loading and recovery instructions. To re-sync:

1. Copy `native/craft-anything/crates/setup/src/skill.md` and `skill_recovery.md` into `assets/dsh-craft-anything/` after an upstream bump.
2. Re-apply the built-in adaptations and set the frontmatter `name` to `dsh-craft-anything`.
3. Run the package tests, which compare the asset directory with the provider's skill name.

`assets/LICENSE`, `LICENSE-APACHE` and `NOTICE` carry the upstream dual-license notices, and `locale/*.json` supplies the component title and description in each UI language for plugin inventories.

</details>

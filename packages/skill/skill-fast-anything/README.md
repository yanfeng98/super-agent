---
description: "The bundled fast-anything skill: the hot-path porting loop it carries, where it is mounted, and the upstream text maintainers re-sync."
kind: "package-reference"
---

# @deepseek-ai/dsh-skill-fast-anything

English | [中文](README.zh.md)

## Summary

This provider ships one bundled skill, `dsh-fast-anything`, teaching the [fast-anything](https://github.com/EL4CTEO/fast-anything) loop: scan and rank functions by time share and portability, record one function's real calls, write its Rust port, then verify, enable, bench and report it — and disable it again when needed. It moves only the slow part; the rest of the app stays put. The text derives from the upstream `skills/fast-anything` directory vendored at [`native/fast-anything`](../../../native/fast-anything/NATIVE.md) under the MIT license.

The Web `standard` agent preset mounts it beside the fast-anything MCP server, so a session with the `mcp__fast-anything__*` tools has the skill that drives them.

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

Mount the provider beside the skill registry and `dsh-tool-skill`. It registers into the agent's skill layer under the provider name `dsh-fast-anything` at the bundled skill rank (600) and declares `shipped`, so the composer lists it with the rest of the harness.

```yaml
- id: skill-fast-anything
  name: '@deepseek-ai/dsh-skill-fast-anything'
```

| Field | Default | Meaning |
|---|---|---|
| `assetRoot` | packaged `assets/` | Absolute directory holding the `dsh-fast-anything` skill directory. Set it to serve a relocated copy, as tests do. |

A relative `assetRoot` fails at load. To remove the skill from a profile, replace the mounting row with `disabled: true`.

### What the catalog contains

One entry, `dsh-fast-anything`, reachable by the model and by the user: its description matches make-it-faster, profiling, and move-the-slow-part-to-Rust requests, and `/dsh-fast-anything` loads the body on demand. The entry carries the upstream project `EL4CTEO/fast-anything`, so the composer groups it apart from the harness-only skills.

A same-name skill from a nearer layer wins: a project or user skill directory supplies the catalog entry and body instead of this provider, and within one layer the bundled rank sorts last.

### Observable success and failures

The skill appears once in the catalog with its description, loads the packaged body, and resolves its resource directory against the packaged assets. A missing or invalid `SKILL.md` is omitted from a complete catalog and reported once through `ctx.logger.warn`. Any other read error leaves discovery incomplete, so the registry retries it, and a load rejects rather than serving a partial body. Disposing the plugin removes the candidate.

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
| [`assets/dsh-fast-anything/SKILL.md`](assets/dsh-fast-anything/SKILL.md) | The porting loop, its tool list, and the installation notes |
| [`assets/LICENSE`](assets/LICENSE) | The upstream MIT notice covering the adapted skill text |

</details>

-----

<a id="further-exploration"></a>
## Further Exploration

- [fast-anything mount](../../mcp/fast-anything/README.md) — the MCP server and CLI this skill drives, mounted by the same preset.
- [Skill subsystem reference](../../../docs/subsystems/skills.md) — the registry, provider contract, and catalog rendering this provider plugs into.
- [skill](../skill/README.md) — the registry and invocation policy types.
- [skill-engineering](../skill-engineering/README.md) — the other upstream-derived bundled provider, with the same failure conventions.
- [Speed-up guide](../../../docs/user/guide/fast-anything.md) — the user-facing walkthrough.

-----

<a id="model-experience"></a>
## Model Experience

Indirectly, through `dsh-tool-skill`, which renders the catalog entry and the loaded skill body.

#### KV Cache effect

The entry's description sits in the session catalog from the first request. The body enters the conversation only at the load point, so a session that never asks for a speed-up pays for the description alone.

## Known Limitations and Deferred Work

<a id="known-limitations-and-deferred-work"></a>

These limits define what the bundled provider does not do. They are current package constraints, not a task backlog.

- **One installation's wording** — the body names the staged `native/fast-anything/bin/fast-anything` executable and the Web `standard` preset. A profile without that mount lists no skill, because the preset mounts the provider together with the tools.
- **English only** — the skill text is the upstream English corpus; the plugin metadata is localized, but the catalog entry and body are not, so a localized catalog needs a local override.
- **Manual synchronization** — upstream changes arrive by copying `native/fast-anything/skills/fast-anything/` and re-applying the installation wording; no build step tracks the upstream repository.
- **No per-skill configuration** — the provider exposes one asset root and always publishes the packaged skill.

<a id="dev-note"></a>
### Dev Note

<details>
<summary>Working context for maintainers — click to expand</summary>

The packaged text is the upstream `skills/fast-anything/SKILL.md` copied from the vendored crate, renamed to `dsh-fast-anything`, and changed in five places: the frontmatter name, the MCP tool names (`mcp__fast-anything__*`) beside the CLI subcommands, the `path` argument, the staged build path with "do not install a global copy", and the trailing repository section (Rust 1.88, the `.fast/target` cache, and the `enable`/`disable` source-rewrite warning). To re-sync:

1. Copy `native/fast-anything/skills/fast-anything/` into `assets/dsh-fast-anything/` after an upstream bump.
2. Re-apply those five adaptations and set the frontmatter `name` to `dsh-fast-anything`.
3. Run the package tests, which compare the asset directory with the provider's skill name.

`assets/LICENSE` carries the upstream MIT notice, and `locale/*.json` supplies the component title and description in each UI language for plugin inventories.

</details>

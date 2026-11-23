---
description: "The bundled rea skill: the evidence-based reverse-engineering workflow it carries, where it is mounted, and the upstream text maintainers re-sync."
kind: "package-reference"
---

# @deepseek-ai/dsh-skill-rea

English | [中文](README.zh.md)

## Summary

This provider ships one bundled skill, `dsh-rea`, teaching the [rea](https://github.com/morluto/rea) investigation workflow: route the target by supplied artifact, read the evidence tools return, separate observations from inferences and unknowns, and use normal coding tools to build a reconstruction. It covers native binaries, managed assemblies, Electron/JavaScript applications, Android packages, firmware, websites and runtime observation. The text derives from the upstream `.agents/skills/reverse-engineer-anything` directory vendored at [`native/rea`](../../../native/rea/NATIVE.md) (MIT), adapted for this built-in copy.

The Web `standard` agent preset mounts it beside the rea MCP server, so a session that can load `mcp__rea__*` tools has the skill that explains how to reach them.

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

Mount the provider beside the skill registry and `dsh-tool-skill`. It registers into the agent's skill layer under the provider name `dsh-rea` at the bundled skill rank (600) and declares `shipped`, so the composer lists it with the rest of the harness.

```yaml
- id: skill-rea
  name: '@deepseek-ai/dsh-skill-rea'
```

| Field | Default | Meaning |
|---|---|---|
| `assetRoot` | packaged `assets/` | Absolute directory holding the `dsh-rea` skill directory. Set it to serve a relocated copy, as tests do. |

A relative `assetRoot` fails at load. To remove the skill from a profile, replace the mounting row with `disabled: true`.

### What the catalog contains

One entry, `dsh-rea`, reachable by the model and by the user: its description matches requests to explain or reconstruct shipped behavior, compare application versions, or recover a feature from a binary, and `/dsh-rea` loads the body on demand. The entry carries the upstream project `morluto/rea`, so the composer groups it apart from the harness-only skills.

A same-name skill from a nearer layer wins: a project or user skill directory supplies the catalog entry and body instead of this provider, and within one layer the bundled rank sorts last.

### Observable success and failures

The skill appears once in the catalog with its description, loads the packaged body, and resolves its `references/` guides against its directory. A missing or invalid `SKILL.md` is omitted from a complete catalog and reported once through `ctx.logger.warn`. Any other read error leaves discovery incomplete, so the registry retries it, and a load rejects rather than serving a partial body. Disposing the plugin removes the candidate.

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
| [`assets/dsh-rea/SKILL.md`](assets/dsh-rea/SKILL.md) | The investigation workflow, the on-demand tool loading steps, and this checkout's CLI path |
| [`assets/dsh-rea/references/`](assets/dsh-rea/references/) | The target-family, runtime-observation, evidence and environment guides the body links to |
| [`assets/LICENSE`](assets/LICENSE) | The upstream MIT notice covering the adapted skill text |

</details>

-----

<a id="further-exploration"></a>
## Further Exploration

- [rea mount](../../mcp/rea/README.md) — the MCP server, catalog profile and vendored build this skill drives, mounted by the same preset.
- [Skill subsystem reference](../../../docs/subsystems/skills.md) — the registry, provider contract, and catalog rendering this provider plugs into.
- [skill](../skill/README.md) — the registry and invocation policy types.
- [skill-rust-anything](../skill-rust-anything/README.md) — the other upstream-derived bundled provider, with the same failure conventions.

-----

<a id="model-experience"></a>
## Model Experience

Indirectly, through `dsh-tool-skill`, which renders the catalog entry and the loaded skill body, and through the rea MCP server's on-demand catalog.

#### KV Cache effect

The entry's description sits in the session catalog from the first request. The body enters the conversation only at the load point, so a session that never mentions reverse engineering pays for the description alone.

## Known Limitations and Deferred Work

<a id="known-limitations-and-deferred-work"></a>

These limits define what the bundled provider does not do. They are current package constraints, not a task backlog.

- **Built-in wording** — the body names this repository's `native/rea` launcher and the Web `standard` preset. A profile without that mount lists no skill, because the preset mounts the provider together with the tools.
- **English only** — the skill text is the upstream English corpus; the plugin metadata is localized, but the catalog entry and body are not, so a localized catalog needs a local override.
- **Manual synchronization** — upstream changes arrive by copying the new `.agents/skills/reverse-engineer-anything/` into the package and re-applying the built-in adaptations; no build step tracks the upstream repository.
- **No per-skill configuration** — the provider exposes one asset root and always publishes the packaged skill.
- **External analysis engines** — most native operations need a separately installed Hopper, Ghidra, IDA, JADX or similar; the skill reports unavailability, it does not install providers.
- **Target execution** — runtime observation and environment recording execute the analyzed target with the Host user's permissions; keep production credentials out of the environment and work on copies.

<a id="dev-note"></a>
### Dev Note

<details>
<summary>Working context for maintainers — click to expand</summary>

The packaged text is the upstream `.agents/skills/reverse-engineer-anything/` copied from the vendored tree, renamed to `dsh-rea`, and changed in two places: the connection section (the preset mount, on-demand `load_tools` and `discover_tools` steps, the vendored CLI path and the Node floor instead of `npx`, `rea setup` and plugin registration), and a note at the top of `references/connection-and-recovery.md` saying the registration steps do not apply. To re-sync:

1. Copy `native/rea/.agents/skills/reverse-engineer-anything/` into `assets/dsh-rea/` after an upstream bump.
2. Re-apply those adaptations and set the frontmatter `name` to `dsh-rea`.
3. Run the package tests, which compare the asset directory with the provider's skill name.

`assets/LICENSE` carries the upstream MIT notice, and `locale/*.json` supplies the component title and description in each UI language for plugin inventories.

</details>

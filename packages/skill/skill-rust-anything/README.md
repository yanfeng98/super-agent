---
description: "The bundled rust-anything skill: the unit-by-unit migration loop it carries, where it is mounted, and the upstream text maintainers re-sync."
kind: "package-reference"
---

# @deepseek-ai/dsh-skill-rust-anything

English | [中文](README.zh.md)

## Summary

This provider ships one bundled skill, `dsh-rust-anything`, teaching the [rust-anything](https://github.com/erkamyaman/rust-anything) migration loop: profile what to move, plan units, bridge and record one unit, write its `port.rs`, then prove it with `diff`, `bench`, `shadow` and `gate`. A whole-app rewrite follows `whole-app.md`: grill the decisions, put a Rust front door before the original, port every unit, then lift the fallback. The text derives from the upstream `skills/rust-anything` directory vendored at [`native/rust-anything`](../../../native/rust-anything/NATIVE.md) under the MIT license.

The Web `standard` agent preset mounts it beside the rust-anything MCP server, so a session with the `mcp__rust-anything__*` tools has the skill that drives them.

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

Mount the provider beside the skill registry and `dsh-tool-skill`. It registers into the agent's skill layer under the provider name `dsh-rust-anything` at the bundled skill rank (600) and declares `shipped`, so the composer lists it with the rest of the harness.

```yaml
- id: skill-rust-anything
  name: '@deepseek-ai/dsh-skill-rust-anything'
```

| Field | Default | Meaning |
|---|---|---|
| `assetRoot` | packaged `assets/` | Absolute directory holding the `dsh-rust-anything` skill directory. Set it to serve a relocated copy, as tests do. |

A relative `assetRoot` fails at load. To remove the skill from a profile, replace the mounting row with `disabled: true`.

### What the catalog contains

One entry, `dsh-rust-anything`, reachable by the model and by the user: its description matches rewrite, port, or Rust-speedup requests, and `/dsh-rust-anything` loads the body on demand. The entry carries the upstream project `erkamyaman/rust-anything`, so the composer groups it apart from the harness-only skills.

A same-name skill from a nearer layer wins: a project or user skill directory supplies the catalog entry and body instead of this provider, and within one layer the bundled rank sorts last.

### Observable success and failures

The skill appears once in the catalog with its description, loads the packaged body, and resolves the `whole-app.md` reference against its directory. A missing or invalid `SKILL.md` is omitted from a complete catalog and reported once through `ctx.logger.warn`. Any other read error leaves discovery incomplete, so the registry retries it, and a load rejects rather than serving a partial body. Disposing the plugin removes the candidate.

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
| [`assets/dsh-rust-anything/SKILL.md`](assets/dsh-rust-anything/SKILL.md) | The migration loop, its tool list, and the installation notes |
| [`assets/dsh-rust-anything/whole-app.md`](assets/dsh-rust-anything/whole-app.md) | The whole-app rewrite reference the body links to |
| [`assets/LICENSE`](assets/LICENSE) | The upstream MIT notice covering the adapted skill text |

</details>

-----

<a id="further-exploration"></a>
## Further Exploration

- [rust-anything mount](../../mcp/rust-anything/README.md) — the MCP server and CLI this skill drives, mounted by the same preset.
- [Skill subsystem reference](../../../docs/subsystems/skills.md) — the registry, provider contract, and catalog rendering this provider plugs into.
- [skill](../skill/README.md) — the registry and invocation policy types.
- [skill-engineering](../skill-engineering/README.md) — the other upstream-derived bundled provider, with the same failure conventions.
- [Migration guide](../../../docs/user/guide/rust-anything.md) — the user-facing walkthrough.

-----

<a id="model-experience"></a>
## Model Experience

Indirectly, through `dsh-tool-skill`, which renders the catalog entry and the loaded skill body.

#### KV Cache effect

The entry's description sits in the session catalog from the first request. The body enters the conversation only at the load point, so a session that never mentions a Rust migration pays for the description alone.

## Known Limitations and Deferred Work

<a id="known-limitations-and-deferred-work"></a>

These limits define what the bundled provider does not do. They are current package constraints, not a task backlog.

- **One installation's wording** — the body names the staged `native/rust-anything/bin/rust-anything` executable and the Web `standard` preset. A profile without that mount lists no skill, because the preset mounts the provider together with the tools.
- **English only** — the skill text is the upstream English corpus; the plugin metadata is localized, but the catalog entry and body are not, so a localized catalog needs a local override.
- **Manual synchronization** — upstream changes arrive by copying `native/rust-anything/skills/rust-anything/` and re-applying the installation wording; no build step tracks the upstream repository.
- **No per-skill configuration** — the provider exposes one asset root and always publishes the packaged skill.

<a id="dev-note"></a>
### Dev Note

<details>
<summary>Working context for maintainers — click to expand</summary>

The packaged text is the upstream `skills/rust-anything/SKILL.md` and `whole-app.md` copied from the vendored crate, renamed to `dsh-rust-anything`, and changed in four places: language coverage (`Kotlin`, `PHP` and `C#` are named, matching this build's `bridge`), the PHP and C# paragraph (upstream still says those units cannot be bridged), the CLI sentence (this installation's staged executable instead of a global install), and the trailing installation section (the preset mount, the shared cargo target folder, and `enable`/`disable` editing the user's sources). To re-sync:

1. Copy `native/rust-anything/skills/rust-anything/` into `assets/dsh-rust-anything/` after an upstream bump.
2. Re-apply those four adaptations and set the frontmatter `name` to `dsh-rust-anything`.
3. Run the package tests, which compare the asset directory with the provider's skill name.

`assets/LICENSE` carries the upstream MIT notice, and `locale/*.json` supplies the component title and description in each UI language for plugin inventories.

</details>

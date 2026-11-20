---
description: "The built-in engineering workflow skills: shipped catalog entries, invocation split, plugin metadata, and the asset root maintainers update."
kind: "package-reference"
---

# @deepseek-ai/dsh-skill-engineering

English | [中文](README.zh.md)

## Summary

This provider ships the Harness's built-in engineering workflow skills: 27 skills covering grilling, specs and tickets, implementation, test-driven development, code review, domain modeling, architecture review, and the smaller workflows around them. A fresh checkout has them without a separate install, symlink, or plugin download. Eleven are model-invoked and appear in the session catalog; sixteen are user-invoked and reachable only by typing `/` in the composer. The text derives from [mattpocock/skills](https://github.com/mattpocock/skills) `main` at v1.3.2 (`abeeaf9`) under the MIT license, with the small Harness wording adjustments listed in the dev note.

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

The base bundle inserts this provider, so every base-backed profile (`web`, `headless`, `acp`, `sdk`) ships the skills and no configuration is required. The provider registers into the global skill layer under the provider name `dsh-engineering` at the bundled skill rank (600).

### What the catalog contains

| Invocation | Skills |
|---|---|
| Model-invoked, listed in the session catalog | `code-review`, `codebase-design`, `diagnosing-bugs`, `domain-modeling`, `grilling`, `pr`, `prototype`, `research`, `tdd`, `wizard`, `writing-for-agents` |
| User-invoked, reachable by typing `/` only | `ask-matt`, `grill-me`, `grill-with-docs`, `handoff`, `implement`, `implement-spec`, `improve-codebase-architecture`, `retro`, `setup-matt-pocock-skills`, `teach`, `to-questionnaire`, `to-spec`, `to-tickets`, `triage`, `wait-what`, `wayfinder` |

`ask-matt` is the router: it maps the user-invoked skills onto the main flow (`grill-with-docs` → `to-spec` → `to-tickets` → `implement` → `pr` → `retro`), the on-ramps (`triage`, `diagnosing-bugs`, `wayfinder`), and the vocabulary skills (`domain-modeling`, `codebase-design`). `setup-matt-pocock-skills` writes the per-repository issue-tracker, triage-label, and domain-doc configuration the engineering skills read.

A same-name skill from a nearer layer wins: a preset's local discovery (project `.dsh/skills`, project `.agents/skills`, the configured custom directory, or the user skill directories) supplies the catalog entry and body instead of this provider. Within one layer the bundled rank sorts last.

### Configuration

| Field | Default | Meaning |
|---|---|---|
| `assetRoot` | packaged `assets/` | Absolute directory holding one skill directory per name. Set it to serve a relocated copy, as tests do. |

```yaml
- id: skill-engineering
  name: '@deepseek-ai/dsh-skill-engineering'
  config:
    assetRoot: /opt/skills/engineering
```

A relative `assetRoot` fails at load. To remove the skills from a profile, replace the base row with `disabled: true`.

### Observable success and failures

Each packaged skill appears once in the catalog with its translated description, loads its own instruction body, and resolves relative references (`tests.md`, `PHASE-BOUNDARIES.md`, `template.sh`) against its directory. A missing or invalid `SKILL.md` is omitted from a complete catalog and reported once through `ctx.logger.warn`. Any other read error leaves discovery incomplete, so the registry retries it, and a load rejects rather than serving a partial body. Disposing the plugin removes every candidate.

-----

<a id="understand-the-implementation"></a>
## Understand the implementation

<details>
<summary>Implementation internals — click to expand</summary>

The provider holds a fixed list of skill directory names, reads `assets/<name>/SKILL.md` during discovery and again on each load, and validates the frontmatter contract before publishing a candidate. The directory name is the identity: a declared `name` that disagrees is invalid metadata. `description` is required and non-empty. `disable-model-invocation` and `user-invocable` are optional booleans and project into the registry's invocation policy; the vendored files use the first for the user-invoked set.

The provider attaches through a deferred `skills` injection, so a profile that removes the skill registry leaves the row idle instead of reporting a pending dependency.

Failures separate two cases. `ENOENT` and `ENOTDIR` persist until the file appears, so the skill is omitted from a complete catalog and a load returns `undefined` after invalidating the catalog. YAML, field, or naming errors are also persistent. Every other read error leaves the observation incomplete and propagates from a load.

| File | Role |
|---|---|
| [`src/index.ts`](src/index.ts) | Plugin entry, frontmatter contract, provider registration, failure reporting |
| [`assets/`](assets/) | One directory per skill, the upstream license, and each skill's reference files |

</details>

-----

<a id="further-exploration"></a>
## Further Exploration

- [Skill subsystem reference](../../../docs/subsystems/skills.md) — the registry, provider contract, and catalog rendering this provider plugs into.
- [skill](../skill/README.md) — the registry and invocation policy types.
- [tool-skill](../tool-skill/README.md) — the catalog and loader the model uses.
- [skill-office](../skill-office/README.md) — the other bundled provider, with the same failure conventions.
- [mattpocock/skills](https://github.com/mattpocock/skills) — upstream English source and its design rationale.

-----

<a id="model-experience"></a>
## Model Experience

Indirectly, through `dsh-tool-skill`, which renders the provider's catalog entries and the loaded skill body.

#### KV Cache effect

The eleven model-invoked entries add their descriptions to the session catalog; loaded bodies enter the conversation at the load point. A user-invoked skill adds nothing until the user references it.

## Known Limitations and Deferred Work

<a id="known-limitations-and-deferred-work"></a>

These limits define what the bundled provider does not do. They are current package constraints, not a task backlog.

- **English only** — the skill text is the upstream English corpus; the plugin metadata is localized, but the catalog and bodies are not, so a localized catalog needs a local override.
- **Manual synchronization** — upstream changes arrive by copying the promoted buckets, re-applying the adaptation below, and updating the fixed name list; no build step tracks the upstream repository.
- **No per-skill configuration** — the provider exposes one asset root and always publishes the packaged set; deployments that want a subset author their own provider.
- **The skills assume a workspace** — engineering skills read `GLOSSARY.md`, ADRs, and an issue tracker; `setup-matt-pocock-skills` writes that configuration, and without it the skills ask for it.

<a id="dev-note"></a>
### Dev Note

<details>
<summary>Working context for maintainers — click to expand</summary>

The package is part of this repository; its skill text is synced from [`mattpocock/skills`](https://github.com/mattpocock/skills), branch `main` at `abeeaf9` (v1.3.2), and the `main` branch of `yanfeng98/nano-mattpocock-skills` mirrors it byte for byte. To re-sync:

1. Copy each promoted skill directory into `assets/`, excluding the Codex `agents/openai.yaml` files.
2. Re-apply the Harness adaptations: name the `skill` tool, replace `/clear` with opening a new session (the Harness has `/compact` but no `/clear`), and drop installers, marketplace manifests, the bucket READMEs, and the Codex `agents/openai.yaml` files.
3. Update `SKILL_NAMES` in `src/index.ts`; the drift test compares it with the asset directories.
4. Run the package tests and the package README gates.

`assets/LICENSE` carries the upstream MIT notice, and `locale/*.json` supplies the component title and description in each UI language for plugin inventories.

</details>

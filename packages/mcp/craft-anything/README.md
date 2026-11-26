---
description: "Mounts the craft-anything MCP server so an agent can investigate shipped software with evidence-backed, on-demand toolsets."
kind: "package-reference"
---

# @deepseek-ai/dsh-craft-anything

English | [中文](README.zh.md)

## Summary

Connect an agent to [craft-anything](https://github.com/storytold/craft-anything): a clean-room investigation harness for programs, app bundles, packages, firmware, crash dumps and captures. It keeps a durable brief, plan, hypotheses, findings and handoff in a project folder, and records evidence ids so claims can be checked. Tools are grouped into 17 toolsets; the server starts with the Core toolset and enables others on request. The CLI is compiled from the Rust workspace vendored at [`native/craft-anything`](../../../native/craft-anything/NATIVE.md) by `pnpm run build`, so a checkout needs no global install, published binary or per-machine registration. The Web bundle mounts it in its `standard` agent preset.

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

Mounted by the Web `standard` agent preset as row `mcp-craft-anything`; its tools appear under `mcp__craft-anything__*` after an agent loads them. A task shaped like "how does this shipped app implement search", "compare these two builds" or "what does this firmware image contain" reaches the [`dsh-craft-anything`](../../skill/skill-craft-anything/assets/dsh-craft-anything/SKILL.md) bundled skill. The agent opens a target with `core_target_open`, writes a brief and plan, then enables a toolset with `enable_toolset` and loads the returned tools with `mcp__craft-anything__load_tools`. Every conclusion cites an evidence id, and `core_handoff_write` leaves a note for the next turn.

| Field | Default | Meaning |
|---|---|---|
| `command` | staged vendored build | Executable to run instead of `native/craft-anything/bin/craft-anything-cli[.exe]`, the path `pnpm run build` stages from the vendored workspace; `CRAFT_ANYTHING_BIN` is the environment fallback. |
| `toolLoading` | `on-demand` | Publishes only `mcp__craft-anything__load_tools`; the system-prompt catalog lists the Core toolset and the toolsets an agent has enabled. |
| `toolCallTimeoutMs` | 15 minutes | Investigations can open large archives, run analysis engines and build reconstruction projects. |
| `startupTimeoutMs` | 20 s | Budget for the stdio probe, connection and initial tool sync. |
| `env` | `{}` | Extra environment for the server process. |

The server is an ordinary child process and is not confined by the shell sandbox. In Clean room mode (the default) it refuses to read a target's own files or source; Deep mode must be enabled explicitly. Tools that run a target's code or change a connected device ask for consent. Work on a copy and keep production credentials out of the environment.

-----

<a id="understand-the-implementation"></a>
## Understand the implementation

<details>
<summary>Developer reference: resolution and mounting</summary>

`vendoredBinary(platform)` returns `native/craft-anything/bin/craft-anything-cli[.exe]`, resolved from this package's own `import.meta.url`; [`scripts/build-craft-anything.ts`](../../../scripts/build-craft-anything.ts) compiles the vendored Rust workspace and stages that file, and `pnpm run build` runs it as the `craft-anything CLI` gate. `resolveCommand` prefers an explicit `command`, then `CRAFT_ANYTHING_BIN`, then that path. `apply` mounts one `@deepseek-ai/dsh-mcp-client` instance with `serverName: 'craft-anything'`, `args: ['mcp']` and the configured timeouts. Enabling a toolset emits `notifications/tools/list_changed`, which the client resynchronises into its on-demand catalog. That client is a peer dependency: the host composition provides the instance, so a published install never carries a second copy of the plugin.

</details>

-----

<a id="further-exploration"></a>
## Further Exploration

- [craft-anything](https://github.com/storytold/craft-anything): the engine, its toolsets and its consent model.
- [`@deepseek-ai/dsh-mcp-client`](../../mcp/mcp-client/README.md): tool naming, reconnection and on-demand loading.
- [`@deepseek-ai/dsh-skill-craft-anything`](../../skill/skill-craft-anything/README.md): the bundled skill mounted beside these tools.

-----

<a id="model-experience"></a>
## Model Experience

### MCP tools from the server

#### What the model sees

The server advertises 30 tools at `v0.6.1`: the Core toolset plus `list_toolsets` and `enable_toolset`, out of 323 total. With `toolLoading: eager` they join the tool list under `mcp__craft-anything__*`; with the default `on-demand`, the agent sees one `mcp__craft-anything__load_tools` schema and a system-prompt catalog. The server's initialize result carries instructions, so the system prompt gains an `### MCP server: craft-anything` section, and the shared MCP resource tools name the server in their own section.

#### Token effect

The 30 initial schemas total about 31 KB (roughly 7.8k tokens) of fixed request context, plus the 874-byte instruction section and the three shared MCP resource tools that appear once a server registers. `on-demand` replaces those schemas with a catalog and one loader; enabling a toolset adds the tools of that toolset after the client resynchronises.

#### KV Cache effect

Mounting happens before the first turn. Enabling a toolset changes the system-prompt catalog and tool list mid-session, so it invalidates a warm prefix from that point on. The skill tells the agent to enable only the toolsets a question needs.

## Known Limitations and Deferred Work

<a id="known-limitations-and-deferred-work"></a>

- The plugin mounts inside an agent preset, so each agent using it starts its own server process.
- The executable is a repository build artifact: run `pnpm run build` (or `pnpm run build:craft-anything`) before launching Web, and run it again after changing the vendored workspace. Nothing is downloaded from npm, so this package serves this checkout only, not a published install.
- Packaged runtimes cannot ship it: the executable lives outside every package, so the Desktop app and the Python SDK executable stage the plugin without its CLI and expose no craft-anything tools. Shipping them needs per-platform staging, as `native/system` does for its launcher.
- Building the CLI needs Rust 1.97 or newer through rustup; the vendored copy pins that channel.
- Deep investigations depend on engines the user installs separately: Ghidra, JADX, Apktool, adb, a Chromium browser, Frida, GDB/LLDB and others. `craft-anything-cli capabilities` reports availability, and the tools include an `engines_install` helper with checksum verification for the engines that can be fetched.
- At `v0.6.1` the server defaults to Clean room mode, which refuses to read inside a target. Ask the user before switching to Deep mode; a tool's `details.reason` names `clean_room` when that is the blocker.
- The tools can execute the target's code or change a connected device and require explicit consent; do not bypass a `consent_required` result.

### Dev Note

<details>
<summary>Working context for maintainers — click to expand</summary>

`tests/mount.spec.ts` connects to the executable `pnpm run build:craft-anything` stages: it asserts the staged path, the three command-resolution branches, the full initial eager tool set and the single on-demand loader. A missing build, a renamed initial tool or a broken transport fails the suite instead of degrading silently at runtime; `pnpm run test` builds the binary first.

</details>

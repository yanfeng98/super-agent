---
description: "Mounts the rea MCP server so an agent can reverse engineer binaries, applications and runtime behavior from evidence."
kind: "package-reference"
---

# @deepseek-ai/dsh-rea

English | [中文](README.zh.md)

## Summary

Connect an agent to [rea](https://github.com/morluto/rea): one MCP server that inspects native binaries, managed assemblies, Electron/JavaScript applications, Android packages, firmware, websites and recorded runtime behavior, and returns code, references, limitations and unknowns instead of an unqualified answer. Static JavaScript and .NET inspection need no analysis engine; deep native work uses an installed Hopper, Ghidra, IDA, JADX or Kuna. The server is built from the source vendored at [`native/rea`](../../../native/rea/NATIVE.md) by `pnpm run build`, so a checkout needs no published tarball, no global install and no `rea setup`. The Web bundle mounts it in its `standard` agent preset.

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

Mounted by the Web `standard` agent preset as row `mcp-rea`; the tools appear in that agent as `mcp__rea__*` with no setup step. A task shaped like "how does this app's search work", "compare these two builds", or "recover this feature from the shipped binary" reaches the matching [`dsh-rea`](../../skill/skill-rea/assets/dsh-rea/SKILL.md) skill, which routes the target, reads the Evidence a tool returns, and separates observations from inferences and unknowns.

| Field | Default | Meaning |
|---|---|---|
| `command` | the Host's Node.js runtime | Node.js executable that runs the launcher. Set it to pin a different supported runtime. |
| `launcher` | vendored `native/rea/scripts/rea.mjs` | Entry script to run instead of the vendored build. |
| `catalogProfile` | `hot-cold` | `full` advertises all 263 tools on connect; `hot-cold` advertises five and loads the rest through `discover_tools`. |
| `toolLoading` | `on-demand` | `on-demand` lists the advertised tools in the system prompt and loads the ones an agent asks for. `eager` puts every schema in every request; with `full` that is about 353 KB of schemas. |
| `toolCallTimeoutMs` | 15 minutes | Decompilation, runtime observation and whole-app capture outlive the MCP default of 60 s. |
| `startupTimeoutMs` | 20 s | Budget for the stdio probe, connection and initial tool sync. |
| `env` | `{}` | Extra environment for the server process, merged after the catalog profile. |

With the default profile the agent loads `mcp__rea__load_tools`, calls `discover_tools` with task words or exact names, then loads the returned tools before calling them; DSH re-syncs its tool catalog when the server reports that the list changed. The server runs as an ordinary child process: runtime observation, capture and environment recording execute the analyzed target with the Host user's permissions, so point it at a copy and keep production credentials out of the environment.

-----

<a id="understand-the-implementation"></a>
## Understand the implementation

<details>
<summary>Developer reference: resolution and mounting</summary>

`vendoredLauncher()` returns `native/rea/scripts/rea.mjs`, resolved from this package's own `import.meta.url`; [`scripts/build-rea.ts`](../../../scripts/build-rea.ts) installs the vendored npm dependencies (`npm ci`, once per lockfile) and runs the upstream build, and `pnpm run build` runs it as the `rea MCP server` gate. `resolveLauncher` prefers an explicit `launcher`, then that path. `apply` mounts one `@deepseek-ai/dsh-mcp-client` instance with `serverName: 'rea'`, `args: [launcher, 'mcp']`, the configured catalog profile and the configured timeouts. That client is a peer dependency: the host composition provides the instance, so a published install never carries a second copy of the plugin. The upstream engines floor (Node.js `^22.23.4 || ^24.22.1 || >=26.12.1`) is below this repository's own floor, so a supported Host runtime is enough.

</details>

-----

<a id="further-exploration"></a>
## Further Exploration

- [rea](https://github.com/morluto/rea): the tool, its supported targets, analysis providers and evidence model.
- [rea integration report](../../../rea-integration.zh.md): why the source is vendored, how the build and mount were verified, and the known costs.
- [`@deepseek-ai/dsh-mcp-client`](../mcp-client/README.md): tool naming, reconnection, on-demand loading and catalog re-sync.

-----

<a id="model-experience"></a>
## Model Experience

### MCP tools from the server

#### What the model sees

With the default `hot-cold` profile and `toolLoading: on-demand`, the system prompt lists five tools under `On-demand tools from MCP server rea`: `open_binary`, `binary_session`, `close_binary`, `get_evidence_bundle` and `discover_tools`, plus the `mcp__rea__load_tools` loader. `discover_tools` loads a page of the remaining catalog; the server then reports a changed tool list and the client re-syncs, so newly loaded definitions join the catalog and can be loaded with `load_tools`. The server's instructions appear once a connection registers.

#### Token effect

The five initial schemas total a few kilobytes. The on-demand catalog starts at five lines and grows only with the tools an agent actually loads; each loaded tool adds its own description and input schema to the request. Selecting `catalogProfile: full` with `on-demand` instead lists all 263 tools in the prompt catalog (about 140 KB of names and first-line descriptions), and `eager` puts the full 353 KB schema set in every request.

#### KV Cache effect

Mounting happens before the first turn and does not change mid-session. Loading a tool or discovering a new page changes the tool list and system prompt, so it invalidates a warm prefix from that point; the default catalog keeps that cost to the tools actually used.

## Known Limitations and Deferred Work

<a id="known-limitations-and-deferred-work"></a>

- The plugin mounts inside an agent preset, so each agent using it starts its own server process.
- The runtime is a repository build artifact: run `pnpm run build` (or `pnpm run build:rea`) before launching Web, and again after changing the vendored source. The first build runs `npm ci` inside `native/rea` and needs network access; later builds reuse the installed tree until `package-lock.json` changes.
- Building and running require Node.js 24.22.3 or newer, matching this repository's engine floor. An older runtime fails the build and the launcher refuses to start with an actionable message.
- Packaged runtimes cannot ship it: `native/rea` lives outside every package, so the Desktop app and the Python SDK executable stage the plugin without the server and expose no rea tools. Shipping them needs per-platform staging.
- Most native operations need a separately installed analysis provider (Hopper, Ghidra, IDA, JADX, Kuna and similar). The tools report provider availability; they do not install engines.
- Runtime observation and environment recording execute the analyzed target and may capture clipboard, UI or network data. Work on copies, and do not leave production credentials in the environment.
- Loading a tool mutates the session's tool list, which is recorded in the session log; a resume re-mounts the server and starts a fresh catalog, so previously loaded tools must be rediscovered.

### Dev Note

<details>
<summary>Working context for maintainers — click to expand</summary>

`tests/mount.spec.ts` connects to the launcher the `build:rea` gate prepares: it asserts the vendored path, launcher resolution, the five-tool hot-cold advertisement under `eager`, the single on-demand loader, and fail-soft agent creation when the runtime is missing. A missing build, an upstream release that changes the initial catalog, or a broken transport fails the suite instead of degrading silently at runtime; `pnpm run test` builds the vendored tree first.

</details>

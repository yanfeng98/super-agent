---
description: "Mounts the fast-anything MCP server so an agent can port an app's hottest pure functions to Rust and prove each port against recorded calls."
kind: "package-reference"
---

# @deepseek-ai/dsh-fast-anything

English | [中文](README.zh.md)

## Summary

Connect an agent to [fast-anything](https://github.com/EL4CTEO/fast-anything): a profiler and porting harness that finds where an app spends its time, records what they do on real calls, and has the agent rewrite only those in Rust. A port is enabled only after `verify` replays every recorded call, the derived edge cases and the shrunk inputs against it, and `bench` shows the swap is not slower. The CLI is compiled from the crate vendored at [`native/fast-anything`](../../../native/fast-anything/NATIVE.md) by `pnpm run build`, so a checkout needs no global install, published binary or per-machine configuration. The Web bundle mounts it in its `standard` agent preset.

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

Mounted by the Web `standard` agent preset as row `mcp-fast-anything`; the tools appear in that agent as `mcp__fast-anything__*` with no setup step. A task shaped like "make this Python script faster", "find the hot path in this app" or "move the slow part to Rust" reaches the matching [`dsh-fast-anything`](../../skill/skill-fast-anything/assets/dsh-fast-anything/SKILL.md) skill, which drives the loop: `fast_scan` ranks the functions by time share and portability, `fast_port` records real calls and writes a one-function Rust crate plus `TASK.md`, the agent writes the port, then `fast_verify`, `fast_enable`, `fast_bench` and `fast_report`, with `fast_disable` and `fast_status` for later runs.

| Field | Default | Meaning |
|---|---|---|
| `command` | staged vendored build | Executable to run instead of `native/fast-anything/bin/fast-anything`, the path `pnpm run build` stages from the vendored crate; `FAST_ANYTHING_BIN` is the environment fallback. |
| `toolLoading` | `eager` | `on-demand` lists the 8 tools in the system prompt and loads the ones an agent asks for. |
| `toolCallTimeoutMs` | 15 minutes | `scan`, `port`, `verify` and `bench` profile the app, replay recorded calls and release-build crates, so the MCP default of 60 s is short. |
| `startupTimeoutMs` | 20 s | Budget for the stdio probe, connection and initial tool sync. |
| `env` | `{}` | Extra environment for the server process. |

Each tool takes an optional `path`: the project folder, defaulting to the server process's working directory. The CLI runs as an ordinary child process: it runs the app under a profiler, records its calls, writes a `.fast/` directory and Rust crates into the project, and `fast_enable` rewrites the call sites of the ported functions. It is not confined by the shell sandbox, so point it at a project copy and keep production credentials out of the environment.

-----

<a id="understand-the-implementation"></a>
## Understand the implementation

<details>
<summary>Developer reference: resolution and mounting</summary>

`vendoredBinary(platform)` returns `native/fast-anything/bin/fast-anything[.exe]`, resolved from this package's own `import.meta.url`; [`scripts/build-fast-anything.ts`](../../../scripts/build-fast-anything.ts) compiles the vendored crate and stages that file, and `pnpm run build` runs it as the `fast-anything CLI` gate. `resolveCommand` prefers an explicit `command`, then `FAST_ANYTHING_BIN`, then that path. `apply` mounts one `@deepseek-ai/dsh-mcp-client` instance with `serverName: 'fast-anything'`, `args: ['mcp']` and the configured timeouts. That client is a peer dependency: the host composition provides the instance, so a published install never carries a second copy of the plugin.

</details>

-----

<a id="further-exploration"></a>
## Further Exploration

- [fast-anything](https://github.com/EL4CTEO/fast-anything): the harness, its supported languages, and what a port can and cannot move.
- [`@deepseek-ai/dsh-mcp-client`](../../mcp/mcp-client/README.md): tool naming, reconnection and on-demand loading.
- [`@deepseek-ai/dsh-skill-fast-anything`](../../skill/skill-fast-anything/README.md): the bundled skill mounted beside these tools.

-----

<a id="model-experience"></a>
## Model Experience

### MCP tools from the server

#### What the model sees

With `toolLoading: eager` the 8 fast-anything tools join the agent's tool list under `mcp__fast-anything__*`; their descriptions mirror the MCP server. The server's initialize result carries instructions, so the agent's system prompt gains an `### MCP server: fast-anything` section, and the shared MCP resource tools name the server in their own section.

#### Token effect

The 8 schemas total about 3 KB (roughly 800 tokens) of fixed request context, plus the server's instruction section (392 bytes) and the three shared MCP resource tools that appear once a server registers. `on-demand` replaces the 8 schemas with a system-prompt catalog and one `mcp__fast-anything__load_tools` schema.

#### KV Cache effect

Mounting happens before the first turn and does not change mid-session, so it does not invalidate a warm prefix; switching an agent between `eager` and `on-demand` does.

## Known Limitations and Deferred Work

<a id="known-limitations-and-deferred-work"></a>

- The plugin mounts inside an agent preset, so each agent using it starts its own server process.
- The executable is a repository build artifact: run `pnpm run build` (or `pnpm run build:fast-anything`) before launching Web, and run it again after changing the vendored crate. Nothing is downloaded from npm, so this package serves this checkout only, not a published install.
- Packaged runtimes cannot ship it: the executable lives outside every package, so the Desktop app and the Python SDK executable stage the plugin without its CLI and expose no fast-anything tools. Shipping them needs per-platform staging, as `native/system` does for its launcher.
- Building the CLI needs Rust 1.88 or newer through rustup, and `verify`/`bench` compile the generated port crates with the same toolchain. `scan` itself does not need Rust; `fast-anything doctor` reports which state the machine is in.
- Porting runs the target app's own runtime: Python 3.10+, Node.js 22+, Ruby 3.3+, Java 22+, Go 1.22+, PHP 8.1+ with `ext-ffi`, a C/C++ compiler, .NET 8+ or R 4.0+, depending on the app. `fast-anything doctor` reports each one.
- The tools default to the server process's working directory when `path` is omitted; pass `path` to target another project.
- `fast_enable` rewrites call sites in the project's sources, and both `scan` and `port` execute the project's code with the agent's full environment. Review `enable`'s diff and run it on a copy.

### Dev Note

<details>
<summary>Working context for maintainers — click to expand</summary>

`tests/mount.spec.ts` connects to the executable `pnpm run build:fast-anything` stages: it asserts the staged path, the three command-resolution branches, the full eager tool set and the single on-demand loader. A missing build, a renamed tool or a broken transport fails the suite instead of degrading silently at runtime; `pnpm run test` builds the binary first.

</details>

---
description: "Mounts the rust-anything MCP server so an agent can port TypeScript/JavaScript, Python, C, Go, Ruby, Java or Kotlin code to Rust one verified unit at a time."
kind: "package-reference"
---

# @deepseek-ai/dsh-rust-anything

English | [中文](README.zh.md)

## Summary

Connect an agent to [rust-anything](https://github.com/erkamyaman/rust-anything): a migration harness that records what the original code does, checks every unit of the Rust rewrite against that recording, and refuses to call a port done until `gate` passes. The CLI is compiled from the crate vendored at [`native/rust-anything`](../../../native/rust-anything/NATIVE.md) by `pnpm run build`, so a checkout needs no global install, no published binary and no per-machine configuration. The Web bundle mounts it in its `standard` agent preset.

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

Mounted by the Web `standard` agent preset as row `mcp-rust-anything`; the tools appear in that agent as `mcp__rust-anything__*` with no setup step. A task shaped like "rewrite this CLI in Rust", "move this hot loop to Rust", or "plan a migration of `src/parser/`" reaches the matching [`dsh-rust-anything`](../../../.agents/skills/dsh-rust-anything/SKILL.md) skill, which drives the loop: `analyze`, `bridge`, `record`, write `port.rs`, `build`, `diff`, `bench`, `gate`, and `app_*` for whole apps.

| Field | Default | Meaning |
|---|---|---|
| `command` | staged vendored build | Executable to run instead of `native/rust-anything/bin/rust-anything`, the path `pnpm run build` stages from the vendored crate; `RUST_ANYTHING_BIN` is the environment fallback. |
| `toolLoading` | `eager` | `on-demand` lists the 16 tools in the system prompt and loads the ones an agent asks for. |
| `toolCallTimeoutMs` | 15 minutes | `gate`, `build`, `bench` and `app record` compile crates and replay corpora, so the MCP default of 60 s is short. |
| `startupTimeoutMs` | 20 s | Budget for the stdio probe, connection and initial tool sync. |
| `env` | `{}` | Extra environment for the server process, e.g. a `RUSTUP_TOOLCHAIN` for the crates it generates. |

The CLI runs as an ordinary child process: it writes `rust/<unit>-rs` crates into the repository and executes the code it records. It is not confined by the shell sandbox, so point it at a copy and keep production credentials out of the environment.

-----

<a id="understand-the-implementation"></a>
## Understand the implementation

<details>
<summary>Developer reference: resolution and mounting</summary>

`vendoredBinary(platform)` returns `native/rust-anything/bin/rust-anything[.exe]`, resolved from this package's own `import.meta.url`; [`scripts/build-rust-anything.ts`](../../../scripts/build-rust-anything.ts) compiles the vendored crate and stages that file, and `pnpm run build` runs it as the `rust-anything CLI` gate. `resolveCommand` prefers an explicit `command`, then `RUST_ANYTHING_BIN`, then that path. `apply` mounts one `@deepseek-ai/dsh-mcp-client` instance with `serverName: 'rust-anything'`, `args: ['mcp']` and the configured timeouts. That client is a peer dependency: the host composition provides the instance, so a published install never carries a second copy of the plugin.

</details>

-----

<a id="further-exploration"></a>
## Further Exploration

- [rust-anything](https://github.com/erkamyaman/rust-anything): the harness, its decisions, and what a recording cannot catch.
- [`@deepseek-ai/dsh-mcp-client`](../../mcp/mcp-client/README.md): tool naming, reconnection and on-demand loading.

-----

<a id="model-experience"></a>
## Model Experience

### MCP tools from the server

#### What the model sees

With `toolLoading: eager` the 16 rust-anything tools join the agent's tool list under `mcp__rust-anything__*`; their descriptions mirror the MCP server. The server's initialize result carries instructions, so the agent's system prompt gains an `### MCP server: rust-anything` section, and the shared MCP resource tools name the server in their own section.

#### Token effect

The 16 schemas total roughly 21 KB (about 6k tokens) of fixed request context, plus the server's instruction section (a few hundred bytes) and the three shared MCP resource tools that appear once a server registers. `on-demand` replaces the 16 schemas with a system-prompt catalog and one `mcp__rust-anything__load_tools` schema.

#### KV Cache effect

Mounting happens before the first turn and does not change mid-session, so it does not invalidate a warm prefix; switching an agent between `eager` and `on-demand` does.

## Known Limitations and Deferred Work

<a id="known-limitations-and-deferred-work"></a>

- The plugin mounts inside an agent preset, so each agent using it starts its own server process.
- The executable is a repository build artifact: run `pnpm run build` (or `pnpm run build:rust-anything`) before launching Web, and run it again after changing the vendored crate. Nothing is downloaded from npm, so this package serves this checkout only, not a published install.
- Packaged runtimes cannot ship it: the executable lives outside every package, so the Desktop app and the Python SDK executable stage the plugin without its CLI and expose no rust-anything tools. Shipping them needs per-platform staging, as `native/system` does for its launcher.
- Building needs Rust 1.97: `native/rust-anything/rust-toolchain.toml` pins that channel and rustup installs it on demand. A missing cargo fails the build with an actionable error.
- Generated crates declare `rust-version = "1.97"`. An older default toolchain fails at build time; `rust-anything doctor` reports it.
- Upstream 2.1.0 fails `build`, `diff`, `gate` and `bench` on a **relative** crate path (its `run` helper re-parents the child to a temp directory while passing a relative `--manifest-path`). Pass absolute paths; the skill says so.
- The tool executes the code it records and writes crates into the repository. Treat a run as code execution with the agent's full environment.

### Dev Note

<details>
<summary>Working context for maintainers — click to expand</summary>

`tests/mount.spec.ts` connects to the executable `pnpm run build:rust-anything` stages: it asserts the staged path, the three command-resolution branches, the full eager tool set and the single on-demand loader. A missing build, a renamed tool or a broken transport fails the suite instead of degrading silently at runtime; `pnpm run test` builds the binary first.

</details>

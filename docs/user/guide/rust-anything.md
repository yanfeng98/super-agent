# Port code to Rust with rust-anything

English | [中文](rust-anything.zh.md)

The Web `standard` agent preset mounts the [rust-anything](https://github.com/erkamyaman/rust-anything) MCP server, so a default Session can rewrite TypeScript/JavaScript, Python, C, Go, Ruby, Java, or Kotlin code as Rust one unit at a time. Every unit is checked against a recording of what the original code returned, and a port is not done until `gate` passes. Nothing is installed globally: the CLI is compiled from the crate vendored in this repository.

## What ships

| Phase | Tools |
|---|---|
| Plan | `analyze`, `map`, `profile` |
| Port one unit | `bridge`, `refresh`, `record`, `build`, `diff`, `bench`, `shadow`, `gate` |
| Port a whole app | `app_record`, `app_replay`, `app_front_door`, `app_wire`, `app_lift` |

## Before the first Session

`pnpm install && pnpm run build` compiles the CLI and stages it as `native/rust-anything/bin/rust-anything`. The build needs Rust 1.97: the crate pins that channel in `native/rust-anything/rust-toolchain.toml`, and rustup installs it on demand. `pnpm dsh web` does not rebuild, so build once (and again after the vendored crate changes) and restart the server; Sessions that were already open keep the tools they started with.

## Ask for a port

Describe the work and the matching [skill](../../../packages/skill/skill-rust-anything/assets/dsh-rust-anything/SKILL.md) loads, or type `/` in the composer and pick `dsh-rust-anything` from the **Built-in · erkamyaman/rust-anything** group to load it first. For example:

- "Plan a migration of this repository to Rust and list the units in order."
- "Port the parser in `src/parser/` to Rust, one function at a time."
- "This loop is the bottleneck: profile it and move the hot part only."
- "Rewrite this CLI in Rust and prove it matches the recorded behavior."

The loop is `analyze`, `bridge`, `record`, write `src/port.rs`, `build`, `diff`, `bench`, `gate`; whole apps go through `app_*`. Ask the agent to list the tools starting with `mcp__rust-anything__` when you want to confirm the mount.

## What it leaves behind

Each unit gets a crate under `rust/<unit>-rs/` with its `src/port.rs`, the recording lives beside it as `cases.jsonl`, `map --write` produces `rust/PORT_MAP.md`, and `shadow` generates a drop-in module that runs the original and the port side by side.

## Use the CLI directly

```sh
./native/rust-anything/bin/rust-anything analyze <repository>
./native/rust-anything/bin/rust-anything map <repository> --write
./native/rust-anything/bin/rust-anything doctor
```

Pass absolute crate paths to `build`, `diff`, `gate`, and `bench`: version 2.1.0 re-parents its child process and fails on a relative `--manifest-path`.

## Tune it

| Field | Default | Effect |
|---|---|---|
| `toolLoading` | `eager` | `on-demand` replaces the 16 schemas with a system-prompt catalog and one `mcp__rust-anything__load_tools` tool |
| `command` | staged build | executable to spawn instead; `RUST_ANYTHING_BIN` is the environment fallback |
| `toolCallTimeoutMs` | 15 minutes | per-call budget; `gate`, `bench`, and `app record` compile crates and replay corpora |
| `env` | `{}` | extra environment for the server, such as `RUSTUP_TOOLCHAIN` |

Edit the `mcp-rust-anything` row in `packages/bundle/web-app/presets/standard.patch.yml`.

## Costs and limits

- The 16 tool schemas plus the three shared MCP resource tools cost about 7k request tokens per agent.
- Each Session that mounts the row starts its own server process.
- Only the Web and Desktop `standard` preset mounts it, together with the bundled `dsh-rust-anything` skill; `minimal`, `ptc`, and the headless, ACP, and SDK profiles mount neither.
- The packaged Desktop app and the Python SDK executable stage the plugin without the CLI, so they expose no tools; the skill is still listed and tells the agent to stop rather than install one.
- Generated crates declare `rust-version = "1.97"`; build them with that toolchain or newer.
- The tool executes the code it records and writes crates into the workspace: treat a run as running that code with the agent's environment.

## When something is missing

- No `mcp__rust-anything__*` tools: run `pnpm run build`, restart `dsh web`, and check that `native/rust-anything/bin/rust-anything` exists.
- The build stops with `rustc 1.97 ... not supported`: install the toolchain with `rustup toolchain install 1.97.0`, or make it the default.
- `cargo` is not found: install rustup, then build again.
- Anything else: `rust-anything doctor` reports the language runtimes, the server, and the command that fixes each problem.

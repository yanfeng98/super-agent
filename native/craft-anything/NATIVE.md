# Native craft-anything

This directory is the source-vendored copy of [`craft-anything`](https://github.com/storytold/craft-anything) that ships with this checkout. It is compiled here rather than installed from a published binary or `cargo install`, and it is not a plugin-market package.

Craft Anything investigates software that was *built and shipped*: executables, libraries, app bundles, APKs, JavaScript bundles, WebAssembly modules, firmware, crash dumps and network captures. It records evidence in a project folder and exposes its tools over an MCP server whose toolsets load on demand. The Web bundle's `standard` agent preset mounts the MCP server; the [skill](../../packages/skill/skill-craft-anything/assets/dsh-craft-anything/SKILL.md) drives the investigation loop.

## Baseline

- Upstream tag: `v0.6.1` (`48a9c5a60ff83ff620f98661339be6281a7debc6`, 2026-11-11)
- Workspace version: `0.6.1`, Rust edition 2024, `rust-version = "1.97"`
- License: MIT OR Apache-2.0; `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` and `ATTRIBUTION.md` are preserved here.
- Source: the whole workspace manifest, lockfile, all `crates/`, `apps/`, `xtask/`, `assets/`, the local `vendor/wgpu-hal` patch, and the English README are copied verbatim from the tag.

Kept: `Cargo.toml`, `Cargo.lock`, `.cargo/`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, `about.toml`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `ATTRIBUTION.md`, `README.md`, `crates/`, `apps/`, `xtask/`, `assets/`, `vendor/`.

Left out, because this checkout builds only the CLI and mounts only its MCP server:

- `docs/`, `examples/`, `packaging/`, `.github/`, and the other translated READMEs — upstream documentation and distribution files. The Chinese README is left out because the repository's pairing gate excludes vendored trees and requires a single-language source there.
- `apps/craft-anything/` (the egui desktop app) and `apps/craft-anything-web/` are kept because Cargo's workspace glob includes them; they are never built by this checkout.
- No prebuilt binaries: `pnpm run build` produces `bin/craft-anything-cli[.exe]` from source.

## Local modifications from upstream

One file is added: `rust-toolchain.toml` pins channel `1.97.0` with the minimal profile. Upstream only declares `rust-version = "1.97"`; the pin makes plain `cargo` select the supported toolchain on hosts whose default is older, matching `native/rust-anything`. No upstream source bytes are changed.

## How it is built and mounted

- `pnpm run build` runs the `craft-anything CLI` gate, which calls `pnpm run build:craft-anything` ([`scripts/build-craft-anything.ts`](../../scripts/build-craft-anything.ts)).
- That script runs `cargo build --release --locked -p craft-anything-cli` from this directory and stages the executable as `native/craft-anything/bin/craft-anything-cli[.exe]`.
- [`packages/mcp/craft-anything`](../../packages/mcp/craft-anything/README.md) resolves that staged executable and mounts one `@deepseek-ai/dsh-mcp-client` instance with `args: ['mcp']`. `CRAFT_ANYTHING_BIN` or the plugin's `command` overrides it.
- `packages/bundle/web-app/presets/standard.patch.yml` inserts the `mcp-craft-anything` row; `cordis.patch.yml` mounts the bundled `skill-craft-anything` beside the other built-in skills for every Web profile.
- The server advertises 30 tools initially (the Core toolset plus `list_toolsets` and `enable_toolset`) out of 323 total at `v0.6.1`. Enabling a toolset emits `notifications/tools/list_changed`, which DSH's MCP client resynchronises. The preset uses `on-demand` loading, so only the tools an agent asks for enter its tool list.

A fresh clone needs `pnpm install && pnpm run build`, then `pnpm dsh web`. The first build downloads crates.io dependencies and compiles about 1 GB of intermediate output under `target/`, which `.gitignore` excludes; later builds are incremental. Rust 1.97 or newer is required. The CLI is invoked directly as `native/craft-anything/bin/craft-anything-cli <command>` from the repository root when MCP is unavailable.

Because the executable sits outside every workspace package, packaged runtimes (Desktop, Python SDK executable) stage the plugin without it and expose no tools; shipping them needs per-platform staging like `native/system`. Deep investigations still depend on engines the user installs separately (Ghidra, JADX, adb, a Chromium browser, Frida, and so on); the tools report availability.

## Checks

- `pnpm run build:craft-anything` — compile and stage the CLI.
- `pnpm vitest run packages/mcp/craft-anything` — mount suite against the staged executable.
- `pnpm vitest run packages/skill/skill-craft-anything` — bundled skill provider suite.
- `native/craft-anything/bin/craft-anything-cli capabilities` — reports the 323 tools, 16 toolsets and engine availability of this build.
- `cd native/craft-anything && cargo test --release --locked -p craft-anything-cli` — upstream's inline unit tests; upstream also runs `cargo fmt --check` and clippy in CI.

## Upstream sync

1. Fetch the new tag in a scratch clone and confirm its `rust-version` and tool count.
2. Replace this directory's contents with the same path list used for the current baseline; keep `rust-toolchain.toml` and update it if upstream raises the minimum Rust version.
3. Update the baseline table above. If upstream adds git-sourced dependencies, decide whether to vendor them or document the network requirement before switching tags.
4. Re-run `pnpm run build:craft-anything`, the mount and skill suites, and refresh the tool list in the MCP plugin's mount test and package README.

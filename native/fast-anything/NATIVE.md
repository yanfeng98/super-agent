# Native fast-anything

This directory is the source-vendored copy of the [`fast-anything`](https://github.com/EL4CTEO/fast-anything) performance harness that ships with this checkout. It is compiled here rather than installed from npm or crates.io, and it is not a plugin-market package.

The CLI profiles an app, records the calls its hottest pure functions really receive, has the agent port only those functions to Rust, and enables a port only after it matches every recorded call, derived edge case and shrunk input, and is not slower per call. The Web bundle's `standard` agent preset mounts the MCP side; the [skill](../../packages/skill/skill-fast-anything/assets/dsh-fast-anything/SKILL.md) drives the loop.

## Baseline

- Upstream tag: `v0.8.6` (`ba571858d222836843fcd0bccbd94a4290c36ac2`, 2026-11-24, "fast-anything 0.8.6")
- Crate version: `0.8.6`
- License: MIT; the upstream `LICENSE` is preserved here.
- Upstream `src/`, `Cargo.toml`, `Cargo.lock`, `rustfmt.toml`, `README.md`, `CHANGELOG.md` and `skills/fast-anything/SKILL.md` are copied verbatim.

Kept: `Cargo.toml`, `Cargo.lock`, `LICENSE`, `README.md`, `CHANGELOG.md`, `rustfmt.toml`, `src/`, `skills/`.

Left out, because this checkout builds the crate directly instead of distributing it:

- `npm/` — the npm publish wrapper and its launcher.
- `commands/` and `.claude-plugin/` — the Claude Code slash command and plugin manifests.
- `scripts/` — the README GIF and logo renderers.
- `examples/` — the demo applications (about 320 KB).
- `docs/` — README media only; the markdown links still point at upstream's raw URLs.
- `.github/`, `.gitignore`, `.mcp.json` — upstream CI, ignore rules and agent-client configuration.

## How it is built and mounted

- `pnpm run build` runs the `fast-anything CLI` gate, which calls `pnpm run build:fast-anything`
  ([`scripts/build-fast-anything.ts`](../../scripts/build-fast-anything.ts)).
- That script runs `cargo build --release --locked` from this directory and stages the executable as
  `native/fast-anything/bin/fast-anything[.exe]`.
- [`packages/mcp/fast-anything`](../../packages/mcp/fast-anything/README.md) resolves that staged executable through
  `vendoredBinary(platform)`, and mounts one `@deepseek-ai/dsh-mcp-client` instance (`args: ['mcp']`).
- `packages/bundle/web-app/presets/standard.patch.yml` inserts the `mcp-fast-anything` row, so the default Web agent gets the 8
  `mcp__fast-anything__*` tools plus the three shared MCP resource tools. The `skill-fast-anything` row mounts the bundled skill beside them.
- `FAST_ANYTHING_BIN`, or the plugin's `command` field, overrides the staged executable.

A fresh clone needs the usual `pnpm install && pnpm run build`, then `pnpm dsh web`. The executable sits outside every workspace package, so packaged runtimes (the Desktop app, the Python SDK executable) stage the plugin without it and expose no tools; shipping them needs per-platform staging like `native/system` uses. The first build compiles about 55 MB of intermediate output under `target/`, which `.gitignore` excludes; later builds are incremental.

The crates a port generates compile into the migrated project's own `.fast/target` cache, outside this checkout.

## Local modifications from upstream

None: every byte in the kept paths is upstream's. Two details are worth recording because a sync can surface them:

1. `src/php/bridge.php` has no final newline upstream; it stays that way so the embedded string the crate compiles is unchanged.
2. Upstream's `Cargo.toml` declares `rust-version = "1.88"` and ships no `rust-toolchain.toml`. This copy deliberately adds no pin: any Rust 1.88 or newer works, and the 1.97.0 the repository installs for rust-anything also satisfies it. A missing or older toolchain fails the build with the path to this file.

## Checks

- `pnpm run build:fast-anything` — compile and stage the executable.
- `pnpm vitest run packages/mcp/fast-anything` — the mount suite connects to the staged executable.
- `pnpm vitest run packages/skill/skill-fast-anything` — the bundled skill provider suite.
- `cd native/fast-anything && cargo test --release` — upstream's 84 inline unit tests (`#[cfg(test)]` in `src/`); upstream's CI also runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and the npm launcher tests, which are outside the kept paths. The Python cases use the first `python3` on `PATH`; a Python 3.12 conda build failed one multiprocessing-worker case here while the system 3.10 passed.

## Upstream sync

1. Fetch the new tag in a scratch clone and confirm its `Cargo.toml` version.
2. Replace this directory's contents with the same path list used for the current baseline:
   `git archive <tag> Cargo.toml Cargo.lock LICENSE README.md CHANGELOG.md rustfmt.toml src skills | tar -x -C native/fast-anything`.
3. Update the baseline table here. Re-check whether `rust-toolchain.toml` is still unnecessary and whether `src/php/bridge.php` gained a final newline.
4. Copy the new `skills/fast-anything/` into `packages/skill/skill-fast-anything/assets/dsh-fast-anything/` and re-apply the built-in adaptations described in that package's README.
5. Run the checks above, then `pnpm run build:check` and `pnpm run doc-sync`.

`verify-translation-pairing`, `verify-concrete-terms`, `verify-repository-references`, `baseline-inventory` and the syntax inventory exclude this subtree, so upstream wording, commit references and sources stay verbatim.

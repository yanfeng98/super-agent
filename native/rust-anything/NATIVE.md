# Native rust-anything

This directory is the source-vendored copy of the [`rust-anything`](https://github.com/erkamyaman/rust-anything) migration harness that ships
with this checkout. It is compiled here rather than installed from npm, and it is not a plugin-market package.

The crate is a CLI plus an MCP server: it records what the original TypeScript/JavaScript, Python, C, Go, Ruby, Java or Kotlin code does,
bridges and ports one unit at a time into Rust, and refuses to call a port done until `gate` passes against that recording. The Web bundle's
`standard` agent preset mounts the MCP side; the [skill](../../packages/skill/skill-rust-anything/assets/dsh-rust-anything/SKILL.md) drives the loop.

## Baseline

- Upstream tag: `v2.1.0` (`71c78e10c5beb39acecd52c2eb3f675a199f3cea`, 2026-11-20, "Release 2.1.0")
- Crate version: `2.1.0`
- License: MIT; the upstream `LICENSE` is preserved here.
- Upstream `src/`, `tests/`, `docs/`, `directory/`, `skills/` and the crate manifests are copied verbatim.

Copied: `ACCESSIBILITY.md`, `CHANGELOG.md`, `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, `Cargo.lock`, `Cargo.toml`, `DECISIONS.md`, `LICENSE`,
`README.md`, `SECURITY.md`, `server.json`, `directory/`, `docs/`, `skills/`, `src/`, `tests/`.

Left out, because this checkout builds the crate directly instead of distributing it:

- `npm/` — the publish wrapper that resolves a prebuilt platform binary from the npm package.
- `scripts/` — upstream release and marketing tooling (`npm-packages.mjs`, `gifs.mjs`, `hero.mjs`, social preview).
- `demo/` — the demo applications (about 20 MB).
- `.github/`, `.githooks/`, `.claude-plugin/`, `.mcp.json`, `.coderabbit.yaml`, `CLAUDE.md`, `.gitignore`, `.gitattributes` — upstream CI,
  agent-client and editor configuration.

## How it is built and mounted

- `pnpm run build` runs the `rust-anything CLI` gate, which calls `pnpm run build:rust-anything`
  ([`scripts/build-rust-anything.ts`](../../scripts/build-rust-anything.ts)).
- That script runs `cargo build --release --locked` from this directory and stages the executable as
  `native/rust-anything/bin/rust-anything[.exe]`.
- [`packages/mcp/rust-anything`](../../packages/mcp/rust-anything/README.md) resolves that staged executable through
  `vendoredBinary(platform)`, and mounts one `@deepseek-ai/dsh-mcp-client` instance (`args: ['mcp']`).
- `packages/bundle/web-app/presets/standard.patch.yml` inserts the `mcp-rust-anything` row, so the default Web agent gets the 16
  `mcp__rust-anything__*` tools plus the three shared MCP resource tools.
- `RUST_ANYTHING_BIN`, or the plugin's `command` field, overrides the staged executable.

A fresh clone needs the usual `pnpm install && pnpm run build`, then `pnpm dsh web`. The executable sits outside every workspace package, so packaged runtimes (the Desktop app, the Python SDK executable) stage the plugin without it and expose no tools; shipping them needs per-platform staging like `native/system` uses. The first build compiles about 430 MB of intermediate
output under `target/`, which `.gitignore` excludes; later builds are incremental.

## Local modifications from upstream

1. **`rust-toolchain.toml` (new file).** Pins channel `1.97.0` with the `minimal` profile. Upstream only declares `rust-version = "1.97"` in
   `Cargo.toml` and relies on the caller's default toolchain; with the pin, rustup selects (and installs) the channel for this directory,
   so `pnpm run build` does not fail on an older default. The oxc dependencies require the same version.

2. **Two trailing blank lines removed.** `docs/how-it-works.md` and `tests/fixtures/python/src/textkit/stack.py` end with a blank line upstream; both now end with one newline, because the repository's staged whitespace check rejects a blank line at end of file. No other byte differs.

## Checks

- `pnpm run build:rust-anything` — compile and stage the executable.
- `pnpm vitest run packages/mcp/rust-anything` — the mount suite connects to the staged executable.
- `cd native/rust-anything && cargo test --release` — upstream's own suite; not part of `pnpm run build`, but available against
  the vendored source.

## Upstream sync

1. Fetch the new tag in a scratch clone and confirm its `Cargo.toml` version.
2. Replace this directory's contents with the same path list used for the current baseline:
   `git archive <tag> ACCESSIBILITY.md CHANGELOG.md CODE_OF_CONDUCT.md CONTRIBUTING.md Cargo.lock Cargo.toml DECISIONS.md LICENSE README.md SECURITY.md server.json directory docs skills src tests`.
3. Re-apply the `rust-toolchain.toml` pin (or update it when upstream's `rust-version` moves), trim the trailing blank line from the two files listed above when the sync reintroduces it, and record the new baseline here.
4. Run the checks above, then `pnpm run build` and the Web preset test.

`verify-translation-pairing`, `verify-concrete-terms`, `verify-repository-references` and the syntax inventory exclude this subtree, so
upstream wording, commit references and fixtures stay verbatim.

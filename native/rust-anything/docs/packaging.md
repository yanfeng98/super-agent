# Ship a port as a package

Once a port passes `gate`, `rust-anything package <crate>` lays it out the way end users install
Rust extensions: maturin wheels for Python, napi-rs's per-platform npm packages for Node, one npm
package for wasm, and for a whole Node CLI's front door, an npm command backed by a binary per
platform ([below](#a-whole-cli-front-door)). It writes files only. It builds nothing for other
platforms (the generated GitHub Actions workflow does that) and never publishes. Back to the
[README](../README.md).

```sh
rust-anything package rust/fuzzy-rs                     # writes rust/fuzzy-rs/package/
rust-anything package rust/fuzzy-rs --dry-run           # lists the files instead
rust-anything package rust/textkit-rs --name textkit-fast --version 2.0.0 --out dist/textkit
```

| Flag | Default |
| --- | --- |
| `--name` | the crate name (`fuzzy-rs`): one ported unit is not the whole original package |
| `--version` | the original package's: the nearest `package.json`, or `pyproject.toml` (`setup.cfg`, `__version__`) |
| `--out` | `<crate>/package` |
| `--fallback` | napi only: the module loaded where no binary fits |
| `--dry-run` | write nothing, list the files |

The output folder holds `rust-anything-package.json`, the list of files the last run wrote; a
rerun replaces those and leaves anything else you added. A non-empty folder without it is
refused.

## Python (PyO3)

A self-contained maturin project: `pyproject.toml`, a copy of the crate (`Cargo.toml` with an
empty `[workspace]`, `Cargo.lock`, `build.rs`, `src/`) and the wrapper package, renamed to the
original's module. The port of `src/textkit/` installs as `import textkit`; a single module such
as `src/textkit/words.py` installs as `textkit.words`. The bridge already builds against PyO3's
`abi3-py39`, so one wheel per platform covers every CPython from 3.9 on.

```sh
pip install rust/textkit-rs/package                       # builds with maturin from PyPI
cd rust/textkit-rs/package && maturin build --release     # or a wheel in target/wheels
```

`.github/workflows/<name>-wheels.yml` builds wheels with `PyO3/maturin-action` for macOS
(arm64, x86_64), Linux (manylinux x86_64, aarch64) and Windows x86_64, plus an sdist, installs and
imports the wheel where the runner can run it, and uploads everything as artifacts.

## Node (napi-rs)

The napi-rs layout:

```
package/
  package.json        main package: optionalDependencies on every <name>-<platform>
  index.cjs           the Rust port for this platform, or the fallback
  native.cjs          picks the binary: ./npm/<key>/ (a local build), then <name>-<key>
  binding.cjs         the crate's index.cjs (codecs, error classes) over that binary
  fallback/           the original, when it is one plain JS file that imports nothing
  npm/<key>/          one package per platform, each holding <lib>.<key>.node
```

Keys are napi-rs's: `darwin-arm64`, `darwin-x64`, `linux-x64-gnu`, `linux-arm64-gnu`,
`win32-x64-msvc`. npm installs only the platform package whose `os`, `cpu` and `libc` match. Where
none does (musl, FreeBSD, 32-bit), `index.cjs` loads the fallback: the original file copied in,
or the module given with `--fallback` (for example the original npm package). A TypeScript
original is not copied, since Node does not strip types under `node_modules`; pass built JS with
`--fallback`. Without a fallback the package throws, naming the platform and the builds it has.
`RUST_ANYTHING_MODE=old` loads the fallback on any platform.

When the crate was built (`rust-anything build`), this machine's binary is copied into its
platform folder, so the package can be tried before CI runs:

```sh
npm pack rust/fuzzy-rs/package/npm/darwin-arm64 && npm pack rust/fuzzy-rs/package
npm install ./fuzzy-rs-darwin-arm64-0.1.0.tgz ./fuzzy-rs-0.1.0.tgz
```

`.github/workflows/<name>-npm.yml` builds every platform with cargo (macOS x86_64 is
cross-compiled on the arm64 runner; Linux is built on Ubuntu 22.04, so it needs glibc 2.35 or
newer), checks that the binary loads where the runner can load it, and packs every package into
the `npm-tarballs` artifact. Publish the platform packages before the main one, and only under
names you own: until `<name>-<key>` exists on npm, anyone could publish it.

## Node (wasm-bindgen)

One package for every platform: `index.cjs`, the built `pkg/` and its type declarations. Run
`rust-anything build` first; `package` refuses without `pkg/`.

## A whole CLI (front door)

Pointed at a front door (`rust/app`, made by `app front-door` and `app wire`) of a Node CLI,
`package` lays out the command itself, the way esbuild and Biome ship theirs: one npm package per
platform holding the front door binary, and a main package that installs the command and depends
on the original at the exact version that was recorded. Every run the Rust port covers is answered
by the binary; every other run goes to that original, so the command prints what the original
prints either way.

```sh
rust-anything package rust/app --name prettier-rs              # original from ./node_modules
rust-anything package rust/app --original prettier@3.6.2 --out dist/prettier-rs
```

| Flag | Default |
| --- | --- |
| `--name` | `<app>-rs` (`prettier-rs`) |
| `--version` | the original's version (`3.6.2`) |
| `--bin` | the package name without its scope; repeat for more commands |
| `--original` | the package the front door runs (`node node_modules/<package>/...`), at the version installed in the repository's `node_modules` (`RUST_ANYTHING_REPO` or two folders above the front door). Only an exact version is accepted |

```
dist/prettier-rs/
  package.json        bin, dependencies: { prettier: "3.6.2" }, optionalDependencies on <name>-<key>
  bin/prettier        a Node launcher (see below)
  lib/native.cjs      finds this platform's binary and the pinned original
  lib/install.cjs     postinstall: puts the binary in place of the launcher
  LICENSE*            the notices beside the front door and in the folder above it
  npm/<key>/          one package per platform: bin/<binary>, the same notices
```

How a run reaches the original:

- The launcher runs the binary with `RUST_ANYTHING_ORIGINAL_BASE` set to the folder Node resolved
  the pinned original from (it skips a copy of another version, such as a project's own
  `prettier`). With no binary for the platform, or with `RUST_ANYTHING_MODE=old`, it runs the
  original itself.
- Outside Windows, the postinstall script copies the binary over the launcher, so a run the port
  covers starts no Node process. The binary then looks for `node_modules/<original>` in the
  folders above itself, nearest first, which is the main package's own `node_modules` or the
  folder npm hoisted it to. A dev build outside `node_modules` keeps running the original from
  the repository. With install scripts off (`--ignore-scripts`, pnpm's default for dependencies)
  the launcher stays: same output, plus Node's start-up on every run.

When the front door was built (`cargo build --release` in `rust/app`), this machine's binary is
copied in:

```sh
npm pack dist/prettier-rs/npm/darwin-arm64 && npm pack dist/prettier-rs
npm install -g ./prettier-rs-darwin-arm64-3.6.2.tgz ./prettier-rs-3.6.2.tgz
```

`.github/workflows/<name>-npm.yml` builds the binary for every platform and packs every
package. This repository's own [`ports-npm.yml`](../.github/workflows/ports-npm.yml) does it for
the Prettier and ESLint ports, then installs the Linux tarballs and compares a run of each
command with the original's.

## Where it was checked

On macOS arm64 (Node 24, CPython 3.9), on copies of the demos, each installed
outside its source folder and replayed with `diff --module` or the Python driver:

| Demo | Installed from | Recorded cases |
| --- | --- | --- |
| `demo/fuzzy-napi` | `npm install` of the main and darwin-arm64 tarballs, offline | 2000/2000 |
| `demo/fuzzy-rs` (wasm) | `npm install` of the one tarball, offline | 2000/2000 |
| `demo/fixtures/python-textkit-rs` | `pip install` of the maturin wheel (abi3, cp39) into a venv | 1144/1144 |

The two CLI packages were installed from their tarballs (`npm pack` of the main and darwin-arm64
packages, `npm install -g` into an empty prefix) and the recorded runs replayed through the
installed command with `app replay --build`:

| Package | Installed command | Recorded runs |
| --- | --- | --- |
| `prettier-rs` (Prettier 3.6.2) | binary in place of the launcher | 17,224/17,224, 1,363 of them (7.9%) handed to Prettier |
| `prettier-rs` | `--ignore-scripts`: the Node launcher | 17,224/17,224, 1,363 (7.9%) handed to Prettier |
| `eslint-rs` (ESLint 10.15.0) | binary in place of the launcher | 10,492/10,585, 381 (3.6%) handed to ESLint; the 93 others print ESLint's own install path |

The 93 ESLint runs are stack traces and "There was a problem loading formatter" messages that
name the file ESLint was loaded from, which moved from the recording's folder to the install
prefix (and some of ESLint's dependencies were hoisted). ESLint 10.15.0 run straight from the
same install fails the same 93 runs, and its replay report is identical to the installed
command's apart from the build line.

On real projects with their own config files, run from the project root against the original
from the same install:

| Project | Run | Same output | In Rust |
| --- | --- | --- | --- |
| [analogjs](https://github.com/erkamyaman/analogjs-erkam) (a fork, `99b78c3`): `.prettierrc` with an override, `.editorconfig`, `.prettierignore` | `prettier-rs --stdin-filepath <file>` on 1,610 files (TS, JSON, Markdown, CSS, SCSS, Less, YAML, JS) | 1,610/1,610 | 1,150; the 460 others include all 237 Markdown files, since `.editorconfig` sets `max_line_length = off` for them, which the port declines |
| [ngmd](https://github.com/erkamyaman/ngmd) (`632da43`): `.prettierrc.json` with an override on `./.prettierrc.json`, `.prettierignore` | the same on 161 files | 161/161 | none: an override pattern starting with `./` sends the whole project to Prettier |
| ngmd | `prettier-rs --check .` | identical | none (folder arguments) |
| [hinge-guess](https://github.com/erkamyaman/hinge-guess) (`6e50fd0`): `eslint.config.js` with typescript-eslint and angular-eslint | `eslint-rs .`, `-f json .`, `--rule ... -f json .` | identical | none (config lookup) |

Timed on the analogjs copy, one process per file, the installed commands (Apple M4, 10 cores,
Node 24.15.0, load 3.0 to 4.6, two runs, every output compared): all 1,610 files took 132.5 s and
143.2 s with Prettier, 49.8 s and 51.3 s with `prettier-rs` (2.66x and 2.79x); the 1,150 files the
port answers took 100.7 s and 99.7 s against 6.6 s and 8.2 s (15.2x and 12.1x). Each of the other
460 runs starts the binary and then Node.

`tests/package.rs` repeats the napi round trip in `cargo test`, including the fallback with the
platform package removed, and the CLI layout with a stand-in binary: the launcher, the
`RUST_ANYTHING_MODE=old` path to the original, the postinstall swap, and a refused version range. The generated workflows have not been run on GitHub yet: the other
platforms' binaries are untested until they are.

## Where the workflow goes

GitHub runs workflows only from the repository's `.github/workflows/`. `package` writes the
workflow inside the output folder, with `working-directory` set to that folder relative to the
repository root, and says where to move it.

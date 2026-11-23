# Status

What works, and how each part was last checked. Updated 2026-11-22.

**Verified**: run end to end (bridge, record, port, diff, gate) on real code someone else wrote,
named below. **Tested**: covered by this repository's tests on fixtures written for them, not yet
run on a real project. **Whole apps**: `analyze` and the whole-app flow (`app record`, `app replay`,
front door); units cannot be bridged yet.

## Porting units

| Language | Status | Last real run | Result |
| --- | --- | --- | --- |
| TypeScript/JavaScript | Verified | [ESLint](real-world.md#eslint) (2026-11-21), [Prettier](real-world.md#prettier) (2026-11-18), [json-server](real-world.md#json-server) (2026-11-18), [Fuse.js](real-world.md#fusejs) (2026-11-18), [vercel/ms](real-world.md#vercelms) (2026-11-18), x18n's `packages/core` | ESLint: `eslint-stdin-rs` 10,192/10,192, gate passes, front door replay 10,585/10,585 at 3.6% fallback (CommonJS module bridged); Prettier: `doc-printer-rs` 2,848/2,848 and `js-format-rs` 15,753/15,753, gate passes on both; json-server: six units, 3,399 cases, gate passes on each; Fuse.js, vercel/ms, x18n: 3,414/3,414, 975/975 and 7,925 cases match |
| Python | Verified | `colorsys.py` from CPython 3.13 (2026-11-16) | 2,400/2,400; gate passes |
| Go | Verified | `path` from Go 1.24 (2026-11-16) | 2,642/2,642; gate passes |
| Ruby | Verified | `shellwords.rb` from Ruby 3.3 (2026-11-16) | 1,617/1,617; gate passes |
| Java | Verified | JDK 21 behind a facade (2026-11-16) | 4,492/4,492; gate passes |
| C | Verified | [ffmpeg](real-world.md#ffmpeg)'s libavutil n7.1 (2026-11-20), glibc behind a facade (2026-11-16) | ffmpeg: 10 units (six with libavutil's own signatures, four through one-shot wrappers over context APIs), 226,447/226,447, gate passes on each; glibc: 1,859/1,859 after dropping one undefined-behavior case |
| Kotlin | Tested | none | `analyze`, detection, JNI-shaped exports, bridge, record and diff on a Gradle fixture (`tests/loop.rs`); needs `kotlinc` on PATH, so CI skips it |
| Kotlin for Android | Tested | none | `build --profile android` builds one installed ABI per test run into `jniLibs/` with every JNI entry point and the 16 KB alignment checked; all three ABIs were also built by hand, and the `armeabi-v7a` library was loaded on an Android 11 TV and called through JNI. `record --profile android` replayed the Kotlin unit under ART on that TV (291 cases) and `diff --profile android` matched 279/291 against the Android `.so`. Needs the NDK/device, so CI skips it |
| PHP | Verified | PEAR `XML_Util` 1.4.5 behind a facade (2026-11-22) | 1,838/1,838; gate passes. Top-level functions with scalar, string, list and assoc-array parameters, through `ext-ffi`; static methods, classes, objects, references, variadics, callables and generators stay in PHP with the reason; no `shadow`, no `--batch` |
| C# | Verified | Google.Protobuf 3.29.0's `CodedOutputStream.ComputeSize.cs` (2026-11-22) | 16 public static methods (6 kept in C#), 2,941/2,941 with 2 lone-surrogate cases listed in `known-diffs.jsonl`; gate passes. Records and POCOs as arguments, defaults, nullable, lists and dictionaries are tested on a fixture (`tests/loop.rs`); `shadow` and `--batch` are not available for C#; needs `dotnet` on PATH, so CI skips it |

Details of every run: [real-world.md](real-world.md) (TS/JS) and
[real-world-languages.md](real-world-languages.md) (the others).

## Commands

| Command | Status | Notes |
| --- | --- | --- |
| `analyze` | Verified | every language above, x18n's 142 units |
| `bridge`, `record`, `diff`, `gate`, `build` | Verified | the runs above; `bridge --as-json` on json-server's `paginate`; PHP and C# units in `tests/loop.rs` (skipped without `php` with FFI or without `dotnet`) |
| `build --profile android` | Tested | Kotlin fixture, one ABI per test run (all three built by hand), entry points and 16 KB alignment checked; `armeabi-v7a` library loaded on an Android 11 TV and called through JNI |
| `record`/`diff --profile android` | Tested | Kotlin fixture on an Android 11 TV: record 291 cases under ART, then diff against the Android `.so` (12 of them did not match yet, not investigated); guarded test `tests/android_device.rs` skips without a device |
| Types for untyped JS in `bridge` and `analyze` (sibling `.d.ts`/`.d.mts`/`.d.cts`, then JSDoc) | Tested | `tests/untyped_js_types.rs`; `analyze` uses `bridge`'s rules; tested there with a `.d.ts`, a class method and an alias (`tests/analyze.rs`) |
| `bench` | Verified | the runs above; numbers in [real-world.md](real-world.md) |
| `fuzz` | Tested | TS/JS: finds a planted mismatch the recording missed, `--keep` makes `gate` fail (`tests/fuzz.rs`); other languages share the record/diff path untested |
| `record --pin-clock` | Tested | TS/JS (napi): a unit using `Date.now()` and `Math.random` recorded and matched by a Rust port (`tests/fuzz.rs`); Python driver shares the algorithm, untested on a real port |
| `record --from-tests` | Verified (TS/JS), Tested (Python) | vercel/ms's Jest suite |
| Coverage of the original in `record` and `gate` | Verified (TS/JS) | Fuse.js: cases run 81% of the lines it imports; other languages do not measure it yet |
| `--batch` | Verified | Fuse.js, vercel/ms, x18n search |
| wasm bridge (`--kind wasm`) | Verified | x18n search |
| `shadow` | Verified | x18n's `packages/core` |
| `profile` | Tested | |
| `go` | Verified (fixtures) | Run with `--agent claude` (Claude Code 2.1.308) on macOS arm64, 2026-11-22. Python `tests/fixtures/go-fuzzy`: picks `dedupe` over `levenshtein`, 207 cases, ported in one turn, 207/207, gate passes, switched on. TS `tests/fixtures/go-search-ts`: picks `score`, called per item by `main`, with `--batch score:text`, 1,233 cases (1,000 captured from the run), ported in one turn, 1,233/1,233, gate passes; `bench` times it 7.6x slower per call, so it stays off and `go` points at `ScoreBatch`, which the fixture's app then used with the same 1,494,560 hits. Not run with `codex` |
| `enable`, `disable` | Tested | TS/JS (napi, ESM) and Python: callers get the stand-in Rust module, the kill switch brings the original back, disable restores the file byte for byte (`tests/switch.rs`); wasm units untested |
| `report` | Tested | Markdown and SVG in `src/card.rs`; the `go` fixture above by hand (207/207, `bench` on a loaded machine, so its number is not published) |
| Shared build cache, `clean` | Tested | one owner per package name and per-crate copies in `src/cache.rs`; two crates named alike built into one shared folder keep their own library in `tests/switch.rs`. Front doors keep their own `target/` |
| `map` | Tested | ports, upstream changes and `--crates` in `tests/port_map.rs` |
| `app record`, `app replay` | Verified (servers and CLIs) | json-server: 514 HTTP cases with `--fresh`, `--arg`, `--pace`; Prettier: 17,224 CLI cases with `--inputs`, 63 of them with config files around the input; CLIs in `tests/app.rs`, CLI `--fresh` in `tests/app_fresh.rs` |
| Side effects (`app record --effects`, `--db-reset`, `--db-dump`) | Tested | email, webhooks and SQLite rows for a CLI and a server in `tests/effects.rs` |
| Front door (`front-door`), `wire` | Verified (TS server and CLI) | Prettier: one unit wired, 17,224/17,224 at 7.9% fallback, plus 355 cases from fuzzing and mutation testing (`demo/real-world/prettier-rs/equiv`) (JS, TS, JSX, JSON, GraphQL, YAML, Markdown, CSS, SCSS and Less in Rust), glue declining per case with `original::run_with_stdin`; json-server: reads wired first (22.6% fallback), then every route (0%); a `rust/` that is a symlink in `tests/app_fresh.rs`; servers in eight languages in `tests/http.rs` |
| `lift` | Verified (TS server), Tested (CLIs) | json-server: pure Rust binary, 514/514; CLIs in all eight languages in `tests/app.rs` |
| `package` | Tested (libraries), Verified (CLI front doors) | napi, wasm and PyO3 demos installed from their packages on macOS arm64 (2000/2000, 2000/2000, 1144/1144); Prettier and ESLint front doors installed from their npm tarballs on macOS arm64 and replayed (17,224/17,224 at 7.9% fallback; 10,492/10,585, the other 93 print ESLint's install path exactly as ESLint 10.15.0 from the same install does); napi tarball install and fallback, and the CLI launcher, postinstall swap and pinned original in `tests/package.rs`; the generated GitHub workflows and `ports-npm.yml` have not run yet |
| MCP server (`mcp`) | Tested | protocol test in `src/mcp.rs` |
| `setup` | Tested | registers the MCP server (and the skill, where the agent reads skills) with Claude Code, Codex, Cursor, Gemini CLI and OpenCode; per-agent tests in `src/setup.rs`, read back in `src/doctor.rs` and `tests/doctor.rs`. The OpenCode entry was also confirmed by OpenCode 2.3.2 itself (`mcp.servers`, `command` as an array) |

## Which command covers which language

The tables above say how far each part has been proven. This one says what each language can be
asked for today, so a reader does not have to open the code to find out that `enable` is TS/JS and
Python only, or that `profile` has no C entry point. `yes` means the command does its job for that
language; `part` means it works with a restriction named in the row; `no` means it refuses and
prints what to use instead.

| Language | `analyze` | whole app | unit bridge | `record`/`diff` | `shadow` | `enable` | `profile` | `package` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TypeScript/JavaScript | yes | yes | napi-rs, wasm-bindgen | yes | yes | yes (napi, wasm) | yes (Node 24) | napi, wasm, CLI front door |
| Python | yes | yes | PyO3 | yes | yes (module) | yes (PyO3) | yes | PyO3, CLI front door |
| C | yes | yes | `extern "C"` drop-in | yes | yes (`CAbi` file) | no | no (`perf record -g`) | no |
| Go | yes | yes | dispatcher + generated driver | yes | yes | no | yes | no |
| Ruby | yes | yes | magnus, FFI | yes | yes | no | yes | no |
| Java | yes | yes | JNI + binary wire | yes | yes | no | yes | no |
| Kotlin | yes | yes | JNI through the Java bridge | yes (`kotlinc` on PATH) | yes | no | no (JFR) | no |
| PHP | yes | yes (`php -S` front door) | JSON dispatcher over `ext-ffi` | yes (`php` with FFI) | no | no | no (SPX or Xdebug) | no |
| C# | yes | yes (ASP.NET) | JSON dispatcher over P/Invoke | yes (`dotnet` on PATH) | no | no | no (`dotnet-trace`) | no |
| C++ | no | yes (any program: `app record`/`app replay`) | no | no | no | no | no | no |
| Swift | no | yes (any program: `app record`/`app replay`) | no | no | no | no | no | no |
| Dart | no | yes (any program: `app record`/`app replay`) | no | no | no | no | no | no |
| PowerShell | no | yes (any program: `app record`/`app replay`) | no | no | no | no | no | no |
| CUDA | no | yes (any program: `app record`/`app replay`) | no | no | no | no | no | no |
| Assembly | no | yes (any program: `app record`/`app replay`) | no | no | no | no | no | no |

A `no` in one column is a scope decision, not a gap in the row: `package` covers napi-rs,
wasm-bindgen and PyO3 crates plus CLI front doors, and says so when handed a `c-abi` crate; `enable`
switches TS/JS and Python ports in place and points everything else at `shadow`.

Where the cells come from: entry points in `whole_app.rs`, the unit-bridge guard in `lang/mod.rs`,
the per-shape branches in `shadow.rs`, `enable.rs`'s `is_supported`, `profile.rs`'s alternatives for
C, Kotlin, PHP and C#, and `package.rs`'s layout match.

C++ through Assembly have no adapter yet, so `analyze` does not read their source; their
programs still run through the whole-app flow, which compares any runnable program's output.
CMake is read only as part of C project discovery. Requested in #200, where the order is tracked.
`go` needs `profile`, so it runs where `profile` does.

## Where the tests run

CI installs Node, Python, Go, Ruby and Java on macOS and Windows. It does not install PHP, .NET,
Kotlin, the Android NDK or an Android device, so the PHP, C#, Kotlin and Android tests skip there
and run only on a machine that has them.

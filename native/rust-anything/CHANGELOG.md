# Changelog

## Unreleased

## 3.3.0 (2026-11-22)

- `go` keeps the file's other classes in the original as well as its other functions. It kept
  only functions, so porting `get_close_matches` from `difflib` also bridged `HtmlDiff`, and
  `diff` and `gate` could not pass on stubs the task said to leave alone.
- Generated Python bindings allow `clippy::too_many_arguments`: a function or method with many
  parameters made `gate` fail on rust-anything's own `lib.rs`.
- `go` takes the repository root as its first argument, as `profile` does
  (`rust-anything go . -- node bench.mjs`); `--root` still works.
- `docs/STATUS.md`'s table of which command covers which language (XiaoBinGan, #201) has rows
  for C++, Swift, Dart, PowerShell, CUDA and Assembly (whole apps only, no adapter yet), from
  his audit in #200.
- PHP units cross into Rust: `bridge` no longer refuses them. A PHP unit's top-level functions
  (namespaced or global) with scalar, string, list and assoc-array parameters get a JSON
  dispatcher crate and `php/<crate>.php`, which redeclares each function in its namespace with the
  original parameters and calls the Rust library through `ext-ffi`. `record` runs the original
  under a generated `driver.php` (after `vendor/autoload.php` when present; omitted middle
  arguments are passed by name so their defaults apply), `build` puts the library beside the
  wrapper, and `diff` and `bench` call it through the wrapper. Results are compared as PHP values
  (int and float, key order, `[]` and `{}` as PHP sees them), exceptions by class, printed output
  too. Static methods, classes, object-typed parameters or returns, references, variadics,
  callables and generators stay in PHP with the reason; `shadow` and `--batch` refuse PHP units.
  A bare `array` parameter or return now gives way to its phpdoc type (`list<float>` is
  `Vec<f64>`), and an undocumented one is a `serde_json::Value`, since it may be a list or a map.
- Real-code run: PEAR `XML_Util` 1.4.5 behind a facade of seven top-level functions, PHP 8.5.12:
  1,838/1,838 recorded cases, gate passes; through the JSON bridge every function is slower than
  PHP (0.09x geometric mean on a busy machine), as `bench` reports
  (`demo/real-world/php-xml-util-rs`, `docs/real-world-languages.md`).
- C# units can be bridged. `bridge` takes the public static methods of a unit's public classes
  and generates a JSON dispatcher over the C ABI (`Kind::Ffi`) plus `cs/<Lib>.cs`: the same
  classes and signatures (defaults kept) under `<namespace>.Rust`, each calling Rust through
  P/Invoke. Arguments may be scalars, strings, arrays, lists, string-keyed dictionaries, nullable
  values, and records or plain classes from the project (public auto-properties, no constructor
  arguments), which cross as JSON objects. Generic, `ref`/`out`/`in`, `params`, overloaded and
  async methods, and `decimal`, `char`, `ulong`, `object`, delegates, tuples and spans stay in
  C# with the reason. `record`, `diff` and `bench` build a console driver with `dotnet build`
  from the owning project's sources and package references, call each method by reflection, and
  compare results as typed JSON, exceptions by type and `Console.Out` output. An input that does
  not convert to a parameter type (a `long` for an `int`) is not recorded. `build` copies the
  library into `cs/`. `shadow` and `--batch` refuse C# units with the reason. Verified on
  Google.Protobuf 3.29.0's `CodedOutputStream.ComputeSize.cs`: 16 methods, 2,941/2,941 with two
  lone-surrogate strings listed in `known-diffs.jsonl` (a Rust `String` cannot hold them), gate
  passes. Calls cost 0.56-3.3 µs, so these 29-87 ns functions are 0.03x as fast through Rust.

## 3.2.0 (2026-11-22)

- Python `record --from-tests` keeps the calls that pass arguments by keyword where Python does
  not bind them by position: keyword-only parameters (`def f(x, *, flag)`) and an argument named
  after a skipped default (`f(x, fill='*')`). They were dropped from `test-inputs.jsonl`; they
  are now written with a `kwargs` object, which the driver already replays.
- Python `diff` and `bench` pass keyword arguments to the Rust bindings under the bindings'
  snake_case names (`scaleBy=10` as `scale_by=10`); a keyword-only parameter named in camelCase
  failed every case with `TypeError: unexpected keyword argument`. A Python mismatch shows its
  keyword arguments after the positional ones, as Ruby's does, so a `known-diffs.jsonl` entry
  for `f(x)` no longer explains `f(x, flag=True)`.
- JS/TS `record --from-tests` encodes the captured arguments with the codec `record` and `diff`
  use, instead of a copy of it that had fallen behind. A `Buffer` was captured as a `Uint8Array`
  (so `b.toString('hex')` recorded `"104,105"`), a `Float64Array` or other typed array as an
  object with numeric keys, and an object shaped like a marker (`{ $date: 'x' }`) was replayed as
  a `Date`. A call is now left out, as one that passes a function or a cycle already was, when an
  argument is a RegExp, an Error, a `URL`, an `ArrayBuffer`, a boxed primitive or an object with
  `toJSON`: those were captured as `{}`, their own fields or their `toJSON` result, which replay
  passed as a different value. A class instance is still captured as its own fields.
- `analyze` judges each export with the same functions `bridge` uses. It had its own copy of
  the rules and put units in `start_here` that `bridge` then refused ("every export ... must
  stay in TypeScript/JavaScript"). The copy missed three of them: a JS unit's sibling `.d.ts` (a
  callback declared there), a class method or constructor that takes a function, and a type
  alias such as `type Pat = RegExp`. Such a unit is now `caution` and left out. Both commands
  share `bridge::ts_functions` and `bridge::class_reason`, so in Python, Ruby, Java, Kotlin and
  Go `analyze` now also checks class methods.

## 3.1.0 (2026-11-22)

- `go` runs `bench` after gate and switches a unit on only when it is not slower as its callers
  call it; otherwise it stops at `slower` with the measured factor and where the speed is (the
  batch class to call from the loop, or the caller to port with it). A wasm port called once per
  item from a JS loop was 7.6x slower than the JIT, and `go` had switched it on. `go` also runs
  `refresh` before building a crate with batches, so a `<Name>Index` the agent writes is used.
  Both found by running `go --agent claude` on a TS fixture (`tests/fixtures/go-search-ts`).
- wasm builds write `pkg/package.json` with `"type": "commonjs"`: inside a `"type": "module"`
  project Node loaded wasm-bindgen's CommonJS output as an ES module and `diff` failed.
- `profile` batches over the innermost loop: with nested loops it named the outer loop's
  variable (`--batch score:query`) instead of the one that runs over the items.
- README: "Only the slow part" in all 20 translations. Website: "Only the slow part" and
  "Switch on and off" pages (English).

- `go`: the hot-path loop in one command. `rust-anything go --agent claude -- python3 app.py`
  profiles the workload, picks the outermost exported function of a portable unit that takes
  arguments (bridged with `--batch` where `profile` advises it; `--function` picks another),
  bridges it alone into `rust/<function>-rs`, records generated cases plus the calls the workload
  makes, writes `TASK.md`, and hands the port to the agent (`claude`, `codex`, or any command
  reading the prompt on stdin, with up to `--rounds` turns that get the failing cases back). It
  then builds, diffs and gates, and switches the unit on with `enable` only when every case
  matches and gate passes. Without `--agent` it stops at the task; run it again after porting and
  it keeps every finished step. Also an MCP tool.
- `enable` and `disable`: switch a TS/JS or Python unit on in place, for every caller, with one
  marked block in its original files that rebinds the ported functions to the Rust module (no
  import changes, no absolute paths); `disable` removes exactly those lines. The unit's
  `RUST_ANYTHING_UNIT_<NAME>=original` or `RUST_ANYTHING_MODE=old` keeps the original for a run,
  and `record`, `diff`, `bench` and `fuzz` set it for the original's side. `enable` refuses
  while recorded cases differ (`--force`). Every switch is listed in one place,
  [Switch a port on and off](docs/how-it-works.md#switch-a-port-on-and-off). MCP tools too.
- `report`: `report/REPORT.md` and `report/card.svg` for a pull request or README, in the v3
  card's light style: the unit, recorded cases matched (`diff`), speedup with the call included
  (`bench`), fallback rate, on or off, and the machine with its core count. MCP tool too.
- One build cache: ports and bridges build into one cargo target folder (`CARGO_TARGET_DIR`, else
  `rust-anything/target` in the user's cache folder), so dependencies compile once. A package
  name belongs to one crate folder there, so a second crate with the same name builds in its own
  `target/` and cannot replace the first one's library; libraries are copied to each crate's
  `target/release`. `clean` removes the folder (`--dry-run`, `--force` for a `CARGO_TARGET_DIR`).
  Front doors keep their own `target/`.
- `profile` reports `portable`, `params` and the `batch` spec per hot function, and its workload's
  stdout goes to stderr, so the MCP `profile` tool no longer writes the workload's output into
  the protocol stream.

- Python: `__all__` was ignored when an earlier top-level statement held no assignment (a
  docstring or an import, as in most modules), so `analyze`, `bridge`, library ports and
  `profile` took every name without a leading underscore: names left out of `__all__` were
  bridged and underscore names listed in it were dropped. `__all__` is now read after such
  statements, with or without a type, follows `__all__ += [...]` and a later `__all__ = [...]`,
  and reads prefixed strings (`r'name'`). An `__all__` built or changed any other way
  (`+ other.__all__`, `sorted(...)`, `__all__.extend(...)`, inside an `if` or `try` block) falls
  back to the names without a leading underscore rather than a partial list. Functions and
  classes a module's `__all__` leaves out are no longer bridged.
- When `build` of a Ruby bridge fails with rb-sys's `Stable API is needed but could not find a
  candidate` and the Ruby rb-sys used (`RUBY`, else `ruby`) is older than `doctor`'s 3.0, the
  error now ends with that Ruby's version and `doctor`'s install command (macOS's system Ruby is
  2.6), instead of only rb-sys's hint to enable `stable-api-compiled-fallback`.

## 3.0.0 (2026-11-21)

- `package` makes a whole Node CLI installable: pointed at a front door (`rust/app`), it lays out
  a main npm package whose command runs the front door binary from `<name>-<platform>` (macOS
  arm64 and x64, Linux x64 and arm64 with glibc, Windows x64), with the original as a dependency
  pinned to the exact recorded version for every run the port hands back. A Node launcher runs
  the binary (or the original where no binary fits, or with `RUST_ANYTHING_MODE=old`); outside
  Windows a postinstall script puts the binary in its place, so a covered run starts no Node.
  `--bin` names the commands, `--original <package>@<version>` the original when it is not in
  the repository's `node_modules`. A generated workflow builds and packs every platform.
- Front doors find the original next to themselves when installed: inside `node_modules`, the
  generated `original.rs` resolves `node_modules/...` from `RUST_ANYTHING_ORIGINAL_BASE` (set by
  the launcher) or the nearest folder above the binary, before the repository it was built in.
  The Prettier and ESLint front doors carry the change.
- The ESLint port on npm as `eslint-rs` (ESLint 10.15.0; macOS and Linux binaries, Windows runs
  ESLint itself), the Prettier port packaged the same way as `prettier-rs` but not published, and
  `.github/workflows/ports-npm.yml`, which builds both front doors on every platform, packs them
  with `package`, and checks the Linux install against the originals.
  Installed from their tarballs on macOS arm64: `prettier-rs` replays 17,224/17,224 recorded runs
  at 7.9% fallback, through the binary and through the launcher; `eslint-rs` replays
  10,492/10,585, the 93 others printing ESLint's own install path, as ESLint 10.15.0 run from the
  same install does (`docs/packaging.md`). The Prettier front door's `Cargo.lock` now lists
  `ignore`, which `--locked` builds needed.

- Prettier demo: about 75 differences from Prettier 3.6.2 outside the recorded corpus, found by
  differential fuzzing, mutation testing and a Doc-level comparison, are fixed or handed to
  Prettier. 355 new cases and the checks are in `demo/real-world/prettier-rs/equiv`. The
  recorded corpus now runs 15,861 of 17,224 cases fully in Rust (7.9% fallback, was 15,837 and
  8.1%).
- `package` (CLI and MCP tool) lays out a gated port as packages end users install, and writes
  files only: PyO3 ports become a maturin project (an abi3 wheel importable under the original's
  module name), napi ports the napi-rs per-platform npm layout (a main package with
  `optionalDependencies` on `<name>-<platform>`, a loader that falls back to the original JS where
  no binary fits), wasm ports one npm package. PyO3 and napi ports also get a GitHub Actions
  workflow building macOS (arm64, x86_64), Linux (x86_64, aarch64) and Windows. Installed from
  their packages, `demo/fuzzy-napi` and `demo/fuzzy-rs` pass 2000/2000 recorded cases and
  `demo/fixtures/python-textkit-rs` 1144/1144 (`docs/packaging.md`).
- `fuzz`: a live differential test. It generates fresh inputs from a new seed (the record
  generator, skipping inputs already recorded), runs them through the original and the built
  port, and reports the mismatches the recorded cases missed. `--cases N` per function, `--seed S`
  (random by default and printed, so a run replays), `--time SECS` for rounds until the budget is
  spent, and `--keep` (`--keep=all`) appends the mismatching (all) fresh cases to `cases.jsonl` so
  the next `diff` and `gate` check them. Works wherever `record` and `diff` do on this machine:
  TS/JS, Python, C, Go, Ruby, Java and Kotlin. Also an MCP tool.
- `record --pin-clock` (TS/JS with the napi bridge, Python): units that read the clock or random
  numbers can be recorded. While pinned (`pinned.json`, until `--no-pin-clock`), record, diff,
  fuzz and gate run with `RUST_ANYTHING_NOW_MS`, `RUST_ANYTHING_SEED` and `TZ=UTC`; `Date.now()`,
  `new Date()`, `Date()` and `Math.random` (`time.time`, `datetime.now`, `date.today` and the
  `random` module) return that instant and a mulberry32 sequence restarted at every case. The port
  reads the same values from `src/port/pinned.rs` (`now_ms()`, `random()`, `getrandbits()`,
  `randbelow()`), written by the first `--pin-clock`. Recordings without the flag are unchanged.
- `analyze`: reads of the clock and random numbers (`Date.now()`, `new Date()`, `Math.random`,
  `performance.now()`; `time.time`, `datetime.now`, `random.*`, `uuid4`) are a new low-severity
  risk, "Clock and random numbers", whose advice points to `record --pin-clock`.
- C bridge: structs of scalar fields cross by value, as parameters and returns (ffmpeg's
  `AVRational av_mul_q(AVRational b, AVRational c)`). The struct is read from the project's
  headers (`typedef struct ... Name;` or `struct tag`), is a `#[repr(C)]` struct at the crate root
  (`crate::AVRational`), and is an object of its fields in cases and recorded results. Structs
  with a pointer, array, bit-field or nested struct field stay in C.
- C bridge: a `char **` / `const char **` out-parameter that the function points into a string
  argument (`av_strstart(str, pfx, &ptr)`, `strtol`'s `endptr`) is recorded as the offset into
  the first `char` string or buffer parameter, and is `Option<&mut Option<usize>>` in the port.
  In cases, give its starting offset, `true` for a `NULL` start, or `null` for a `NULL` pointer.
  A `**` the function advances as its own string (`av_get_token(const char **buf, ...)`) still
  stays in C, as does anything returning memory it allocates.
- `record` (C): generated cases give struct parameters objects of their fields (zeros, ones,
  minus ones, each field's minimum and maximum, each field zero with the others one, and small
  random values) instead of values that were recorded as `TypeError`s, and `char **`
  parameters `true`, `null` or offset 0.
- `diff` (C): a case where the original returned or wrote a pointer outside its arguments (read
  past a string, say) is undefined behavior like a crash, and any answer of the port passes it.
- Case study: the ffmpeg ports add `av_mul_q`, `av_div_q`, `av_add_q`, `av_sub_q`, `av_d2q`,
  `av_nearer_q`, `av_q2intfloat`, `av_rescale_q`, `av_rescale_q_rnd`, `av_strstart` and
  `av_stristart`; 226,447/226,447 recorded calls, gate passes on all 10 crates.
- `diff` (C): a case the original recorded as an error (an argument ctypes could not convert)
  passes when the port gives the same error, instead of always failing.
- Contributors: `mise.toml` pins the toolchains CI uses, and `hk.pkl` runs the secrets check,
  `cargo fmt` and `cargo clippy` before each commit (suggested by u/touristtam on Reddit).
- `setup --no-backup`: changes settings files in place without saving a
  `<file>.rust-anything.bak` copy first.
- `record`/`diff` (TS/JS): plain data shaped like a marker (`{"$date": ...}`, Mongo extended JSON)
  is kept as data instead of being read as a Date, undefined or bigint, and a marker key next to
  other keys is ordinary data. Write such an object in `--inputs` as `{"$object": {...}}` (#121).
- `record`/`diff` (TS/JS): typed arrays other than `Uint8Array` (`Float64Array`, `Int32Array`,
  `Uint8ClampedArray`, ...) are recorded with their kind, so a port returning the wrong one
  fails (#122).
- `app replay`: lines compared in any order are paired by value within `--float-tolerance`, not
  by sorted position (#123, #125 by @sameermankotia).
- `analyze` (C): a unit depends on what its own header includes, so `a.c` that reaches `b()`
  through `a.h` waits for `b.c` (#126).
- README: right under the tagline, it now says the tool doesn't translate code (your agent or you
  write the Rust) and what PHP and C# support today. From a new-user QA review of the README by
  [Mashhood Ali](https://github.com/MASHHOOD2019), an independent AI evaluation and QA specialist
  who tests products and docs the way a new user would ([LinkedIn](https://www.linkedin.com/in/mashhood-ali-514862296)) (#118).
- `app record --fresh` works for CLIs: every case starts from a snapshot of the given files or
  folders (kept in `app-cases/files/`; a path ending in `/` gives its contents, so a whole project
  can be the working directory), written files are what differs from it, and files a case
  removes are recorded in `<case>.removed` and compared on replay. An `--inputs` case can also
  `"remove"` files of the snapshot before it runs. `--fresh` alone no longer implies `--http`.
- Front doors built by rust-anything find the original when `rust/` is a symlink to a folder
  outside the repository (`RUST_ANYTHING_REPO` is set at build time).
- Case study: ESLint 10.15.0 behind a Rust front door, 90 rules (`eslint:recommended` and 26
  more), 10,585/10,585 recorded CLI runs at 3.6% fallback, gate passes on `eslint-stdin-rs`
  (`demo/real-world/eslint-rs`, `docs/real-world.md#eslint`).
- `bridge` and `analyze` read CommonJS exports: `module.exports = { a, b: c }`,
  `module.exports.a = a` and `exports.a = a`, for functions declared in the file. ESLint (and
  most Node code before ESM) exports this way, so its units showed no functions to bridge.
- The `timestamps` normalizer also matches JavaScript's `Date#toString()`
  (`Sat Nov 21 2026 01:30:12 GMT+0300 (...)`). ESLint's HTML report stamps its time that way:
  the two recording runs differed only in the seconds, so only the seconds became a wildcard and a
  replay ten minutes later failed.
- Case study: ffmpeg's libavutil (n7.1), 10 units, 135,773/135,773 recorded calls, gate passes
  on each (`demo/real-world/ffmpeg-rs`, `docs/real-world.md#ffmpeg`).
- C bridge: pointer parameters cross it with the original signatures. A `const` byte pointer
  followed by an integer is a buffer of that length (`&[u8]` in the port); a byte pointer the
  function writes, paired with the next integer or one named like a size, is `&mut [u8]` and its
  contents after the call are recorded; a mutable pointer to one scalar is `Option<&mut T>` and
  its value after the call is recorded; a returned pointer into a parameter is the offset into
  it (`Option<usize>`). In cases, buffers are hex text (or a size, for that many zero bytes) and
  length parameters are left out. Typedefs of scalars and enums resolve through the project's
  headers. Found porting ffmpeg's libavutil, whose APIs all take pointers.
- C bridge: a port can take a `const char *` as `Option<crate::CChars<'_>>` instead of
  `Option<&[u8]>`: the bytes are read as the port goes, without the `strlen` the slice needs
  first. Change the parameter's type in `port.rs` and run `refresh`; `lib.rs` defines `CChars`
  (an iterator over the bytes, with `peek` and `to_bytes`) only when a port uses it. On short
  words the two `strlen` calls cost more than ffmpeg's whole `av_strcasecmp`; with `CChars` the
  port went from 0.23x to 0.68x of C's speed, `av_stristr` from 0.44x to 1.4x.
- `record` and `bench` for C: a folder holding a header named like a C standard one (ffmpeg's
  `libavutil/time.h`) is passed with `-iquote`, so `<time.h>` stays the system's; flags in
  `CFLAGS` are added (`-DHAVE_AV_CONFIG_H`, `-I<tree>`).
- `build`, `diff`, `bench`, `shadow`, `gate`, `refresh` and `record` take the crate path as
  given and make it absolute: `rust-anything build .` on a C crate failed, since the compiler and
  cargo run in a scratch folder.
- `analyze` for C: files under a CPU-architecture folder (`x86/`, `aarch64/`, `arm/`, ...) stay
  in C; `goto` no longer marks a unit blocked as dynamic code; `#include "libavutil/x.h"`
  resolves when the folder analyzed is `libavutil` itself, so its `tests/*.c` count as tests; a
  unit with no bridgeable export gets a note about the C bridge instead of the JS one.
- `gate` for C: the size check counts the project functions the ported functions call, since the
  port does their work too.
- C `char` parameters are `c_char` in generated code (signed or unsigned as the platform's C
  `char`), not `c_schar`.
- `doctor` on Windows: a missing `cargo` gets the C++ Build Tools, then
  `winget install Rustlang.Rustup`, instead of the `sh.rustup.rs` script, which needs a Unix shell.
  The Build Tools come first because winget runs `rustup-init -y`, which skips them with a warning.
  The Build Tools fix also adds the C++ workload (and the ARM64 tools on Windows on ARM): without
  `--add`, winget installs them with no compiler, linker or Windows SDK. Since winget adds nothing
  to Build Tools already installed, the fix also says how to add the workload by hand. A fix of
  several commands prints one per line.
- `analyze --whole-app` lists a library once when the code imports it under one name and the
  manifest declares it under another (`yaml` and `PyYAML`, `androidx.room` and
  `androidx.room:room-runtime`, `active_support` and `activesupport`): names the crate table
  resolves to the same row are one dependency. The JSON keeps `name` (the manifest's spelling
  when declared) and adds `names`, every spelling; `declared` is true if any of them is, and
  `used_by` counts each unit once. The Markdown plan shows the other spellings next to the name.
  Names the table does not know stay separate.
- `analyze --whole-app` reads declared dependencies from each language plugin's own manifest
  parser instead of a second copy, so `analyze` and `--whole-app` agree, and every manifest in
  the workspace counts, not only the root's (a dependency on another package of the workspace is
  not an external one). The plugin parsers gained what only the copy had: Gradle and Maven
  comments are skipped, a Gradle declaration on the same line as `dependencies {` is read,
  requirements files follow `-r` includes and skip VCS, URL and path lines, Poetry dependencies
  count, a gem inside a `group` block is a development dependency, and `add_dependency` is read
  on any receiver. TS/JS counts `peerDependencies` and `optionalDependencies` as declared too.

## 2.1.0 (2026-11-20)

- `app record --float-file PATH=f32le|f64le`: a written file of raw floats is compared value by
  value within `--float-tolerance` instead of byte for byte, and a mismatch reports the count,
  the first index and the largest differences (#114).
- Kotlin/JVM: detection, `analyze`, bridge, record and diff for Gradle modules of top-level
  functions and `@JvmStatic` methods, through the Java bridge (#117).
- Android: `build --profile android` builds the JNI library for the NDK ABIs into `jniLibs/`,
  and `record`/`diff --profile android` run the unit under ART on a connected device; `doctor`
  checks the NDK and Rust targets (#117).
- Java: every overload of an exported name is blocked, since the reflection driver keys methods
  by name; the generated Java runtime uses plain reflection, which ART also runs (#117).
- `record`, `diff` and `bench`: the original and the port run in a temporary working folder,
  removed afterwards, so a function that writes to a path it is given no longer leaves files
  named after the generated inputs in the caller's folder (#115).
- `analyze`: Python functions with numpy array parameters (`np.ndarray`, `NDArray`) carry a new
  risk, since `record` cannot generate arrays for them yet: record the app instead (#113).
- README: the 2.0 headline says how many Prettier runs are fully in Rust (15,837 of 17,224)
  rather than only that none mismatch (#118).
- `lift`: the lifted app or library gets `rust-version` and a release profile with `lto = true`
  and `codegen-units = 1`, unless it sets them already.
- `bridge`: generated crates set `rust-version` and warn on `unused_lifetimes` and
  `trivial_numeric_casts` (so `gate` fails on them).
- `gate`: two checks that never fail it: `lockfile` warns when `Cargo.lock` is missing or out of
  date, and `advisories` runs `cargo deny check advisories` or `cargo audit` when one is
  installed.
- Skill: typed errors inside `port.rs`, Rust's `is_alphanumeric` against the original's letter
  rule, `#[expect(lint, reason = "...")]` over `#[allow]`, and `cargo test` with no tests proving
  nothing.
- `bridge --allocator rusty_alloc`: an opt-in pure-Rust global allocator for batch crates (wasm
  too), kept by `refresh` and taken by the MCP `bridge` tool. mimalloc stays the default (#112).

## 2.0.2 (2026-11-19)

- The npm package carries `mcpName`, and `server.json` describes the MCP server, so it can be
  listed in the official MCP Registry as `io.github.erkamyaman/rust-anything`.

## 2.0.1 (2026-11-19)

- `record`: string traps now include characters that languages classify differently (`²`, `Ⅰ`,
  `½`, `①`, vowel signs such as `ि`, Arabic-Indic digits and letters beyond U+FFFF), so a port
  that tests letters and digits with Rust's `char::is_alphanumeric` instead of the original's
  rule fails `diff`.
- Java demo: `slugify` in `demo/fixtures/java-text-rs` now matches Java's
  `Character.isLetterOrDigit` per UTF-16 unit, and the Java benchmark claims (3x batch) are
  withdrawn: the batch ran on 4 cores against a serial loop. Reported in #110.
- `analyze`: Python packages in a workspace match their dependencies by the normalized name, so
  a package named `core_lib` is found when another one depends on `core_lib>=1` and the port
  order puts it first.
- `setup`: a symlinked settings file (a dotfiles repo) is written at its target instead of being
  replaced by a plain file, and an existing file keeps its permissions (a `600` file stays `600`).
- `setup`: a Codex entry written as an inline table (`rust-anything = { ..., env = { ... } }`)
  keeps its other keys on reinstall, as a `[mcp_servers.rust-anything]` table already did.

## 2.0.0 (2026-11-19)

Highlights:

- **Prettier, ported to Rust.** Prettier 3.6.2 behind a Rust front door: 15,837 of 17,224
  recorded runs format fully in Rust across JavaScript, TypeScript, JSX, JSON, GraphQL, YAML,
  Markdown, CSS, SCSS and Less, and all 17,224 match Prettier byte for byte. 19x faster on JS/TS
  from the command line (Apple M4). See `docs/real-world.md#prettier`.
- **json-server, ported to a pure Rust binary**: 514/514 HTTP cases, 3.5 MB of memory at idle
  instead of 74 MB.
- **Side effects are compared**: outgoing HTTP calls, email and the database's state, per case.
- **`map`**: the whole port at a glance, and every piece whose original changed upstream.
- **Plain JavaScript gets types** from JSDoc and `.d.ts`; **OpenCode** joins the agents `setup`
  registers.

Changed: `diff` exits non-zero when the recording is stale (it printed a warning and exited 0).

- `app record --effects http,smtp`, `--effect-env VAR=KIND`, `--db-reset` and `--db-dump`: record
  what leaves the process per case (outgoing HTTP calls, email, the database's state) and compare
  it on replay, for CLIs and servers. A rewrite that sends an email twice or writes different rows
  now fails the case even when its output matches.
- `map` (CLI and MCP tool): every unit of the original with the crate that ports it, functions
  in Rust and kept in the original, cases and last `diff`, the whole app's glue, units not ported
  yet, and every port whose original changed since it was recorded. `--write` saves
  `rust/PORT_MAP.md`; `--check` exits 1 while a recording is stale.
- Docs: what a real port takes, keeping the original's shape, keeping up with the original, and
  checking a rewrite in any language with `app record` and `app replay --build`.
- `bench` and `record` pass `--experimental-transform-types` only when Node accepts it: Node 26
  removed the option and refused to start (#99).
- `profile` keeps a Python workload script given as a relative path (`python3 bench.py`); its
  functions were dropped and the table came out empty (#101).
- `diff` exits non-zero when the recording is stale, as `gate` already refuses it, and the
  warning says "Stale recording" (#102).
- `analyze` says what a wasm-bindgen bridge needs installed, and that `--kind napi` runs with
  what Node already has (#102).
- `bridge` reads types for plain JavaScript: from a sibling `.d.ts`/`.d.mts`/`.d.cts`, then
  JSDoc `@param {T} name` (with `[name]`, defaults and `opts.field` shapes) and `@returns {T}`.
  Before, only parameters with a literal default were typed and the rest became `unknown`, so
  `record` generated mostly wrong-typed inputs (#100).
- TypeScript analysis and recording resolve path aliases against the inherited `baseUrl`,
  including when the two options come from separate tsconfig files.
- `setup` also registers OpenCode: the MCP server in `~/.config/opencode/opencode.json`
  (`mcp.servers`, the whole argv in `command`) and the workflow skill in
  `~/.config/opencode/skills/`, and `doctor` reads that entry back.
- A port of [Prettier](https://github.com/prettier/prettier) 3.6.2 behind a Rust front door in
  `demo/real-world/prettier-rs` and `docs/real-world.md`: 17,224/17,224 recorded CLI cases match,
  15,837 of them (91.9%) formatted fully in Rust and byte-identical to Prettier, the rest handed to
  Prettier per file (8.1% fallback). JavaScript, TypeScript (849 of 929 cases), JSX, JSON in
  all four flavours (995 of the 1,019 JSON cases Prettier formats), GraphQL (all 787 cases
  Prettier formats, with graphql-js's parser ported) YAML (3,309 of 3,310, with yaml's CST
  parser and yaml-unist-parser ported), Markdown (6,305 of 6,350, with remark-parse ported and
  code blocks formatted by the other ports) and CSS, SCSS and Less (all 2,004 cases Prettier
  formats, with postcss, postcss-scss, postcss-less and the value, selector and media query
  parsers ported) run in Rust, as do all 59 real repository
  files. Ported: the doc printer (`gate` 2,848/2,848), the printer on oxc's ESTree with
  comments, classes, decorators, template literals, ternaries and `prettier-ignore` (`gate`
  15,753/15,753 on `format()`), and the CLI's config resolution (`.prettierrc` JSON,
  `package.json`, overrides, `.editorconfig`, ignore files). On an M4 against Node 24: 19x faster
  per file from the command line, 7x on the median JavaScript/TypeScript file in process (18x on
  the median GraphQL file, 13x on the median YAML file, 25x on the median Markdown file, 9.5x on
  the median CSS/SCSS/Less file); per file from the command line 8.5x for style files and 15.6x
  for Markdown. No file was slower in a first run; one per language was in a second run (a one-off
  stall, 135 µs against 8.5 µs in the first).
- Glue can decline one case after reading stdin: `original::run_with_stdin(args, input)` hands the
  case and its bytes to the original; `lift` refuses while any glue still calls `original::`.
- `bridge` reads `export { a, b as c }` lists.
- Fix: with `CARGO_TARGET_DIR` set, every front door built into the same directory and replays
  ran another app's binary; the front door builds into its own `rust/app/target`.
- Fix: case and input lines nested deeper than 128 levels failed to parse; the harness and
  generated `from_json` read them without a depth limit.
- Fix: `--inputs` case names were cut to 24 characters and collided; given names keep 96.

- A port of [json-server](https://github.com/typicode/json-server) to a pure Rust binary in
  `demo/real-world/json-server-rs` and `docs/real-world.md`: 514/514 recorded HTTP cases match
  with no fallback, and six units pass `gate`. Against Node 24 on an M4: first response in 8 to
  10 ms instead of about 80, 3.5 MB of memory idle instead of 74, 2 to 4x the requests per second
  on one connection and 5 to 13x on 50; 1.1 to 1.2x on writes and on the whole list of a 10,000-post
  database.
- `app record --http --fresh <path>` copies a file or folder the server writes (its database)
  into a fresh working directory for every start, from a snapshot kept with the cases;
  `--arg <arg>` passes the original's arguments to every build too; `--pace <ms>` waits after each
  response for work a server does after answering. json-server needed all three: it rewrites its
  database on every write, and its file watcher can undo a write that follows quickly.
- `bridge --as-json T` bridges a generic the app only fills with data (`paginate<T>(items: T[])`)
  as JSON values instead of keeping it in TS.
- Fix: the HTTP front door's routes with parameters answered axum's own 400 when an escape was not
  UTF-8 (`/posts/%E0`); parameters are now read from the URI and the unit gets the request.
- Fix: `app lift` left out the crates the wired units depend on by path (shared code, other
  units), so the staged build failed; they are staged, stripped of their bindings and synced back
  with the units.
- Fix: a lifted unit that keeps generated modules (`js.rs`) got a `lib.rs` that failed
  `cargo fmt --check`: modules are now listed in the order rustfmt keeps.

- A port of [Fuse.js](https://github.com/krisk/Fuse) string-list search in
  `demo/real-world/fuse-rs` and `docs/real-world.md`: 3,414/3,414 recorded cases match, results
  byte-identical to Fuse, 32x faster on 50,000 strings (5.3x on one core).
- `gate`'s size check counts the files a TS unit imports from the repository, not only the unit.
- Batch crates with string items but other results no longer get an unused helper (clippy failed).
- TypeScript analysis and recording keep inherited path aliases when a later tsconfig base
  omits `compilerOptions.paths`.

- Faster single calls on napi: every sync function crosses the binary wire with no napi
  arguments (each argument cost about 55 ns), behind one JS wrapper instead of two;
  `serde_json::Value` parameters cross as tagged values of any shape, and errors come back in the
  wire buffer. vercel/ms per call: 0.39x to 0.56x of JS over its recorded cases, and `parse` on a
  normal string at JS speed (79 ns).

- `record --from-tests -- <test command>` runs the original's own test suite and records every call
  it makes to the unit's functions: TS/JS with any runner (Jest, Vitest, `node --test`), Python
  with pytest or unittest. On vercel/ms it captured 340 calls from 167 Jest tests, more than
  twice the 149 inputs copied by hand.
- `record` saves a fingerprint of the original unit (`recorded-from.json`); `diff` warns and `gate`
  fails when the original changed since.
- `known-diffs.jsonl` lists cases where the port is meant to differ, each with the result it gives
  and a reason. `diff` and `gate` count them as passing only with exactly that result, and `gate`
  fails on an entry that no longer differs.
- `app replay`'s report no longer carries temporary paths, so CI can commit it and fail on
  `git diff --exit-code`.
- A port of [vercel/ms](https://github.com/vercel/ms), end to end, in `demo/real-world/vercel-ms-rs`
  and `docs/real-world.md`: 975/975 recorded cases match; slower per call, 19x faster as a batch
  for `parse` (3.6x with the handover), 1.3x for `format(x, { long: true })`.
- Fix: a napi batch method with a structured parameter did not compile; crates with batch classes
  now enable napi's `serde-json` feature.
- napi batches pass strings joined: items cross as one string plus UTF-16 end offsets and string
  results come back the same way, about 2x faster each way. The JS API is unchanged.
- napi batches of numbers take their items as one `Float64Array`: `new Batch(numbers)` costs
  0.1 ms instead of 3.3 ms for 100,000 values.
- The generated `js.rs` gains `number_to_string` (JS `String(n)`, digits from serde_json's fast
  printer) and `math_round` (JS `Math.round`), tested against Node.
- `analyze` flags `Math.round`, `toFixed`, and `/i` regexes without `u` where Rust's case folding
  matches more (`ſ`, the Kelvin sign).

## 1.5.1 (2026-11-18)

- Fix ([#80](https://github.com/erkamyaman/rust-anything/issues/80)): `record` could stall for good
  when a generated input made the original run away (a trap like `2147483647` in `[w] * n`
  allocates gigabytes, a loop that never ends). Every recorder (TS/JS, Python, C, Go, Ruby, Java)
  now runs under a watchdog: a case that runs past the limit (10 seconds, or
  `RUST_ANYTHING_CASE_TIMEOUT`) stops the original, the cases recorded so far are kept, and that
  input is left out and listed. Drivers now write each case as it finishes instead of at the end.

## 1.5.0 (2026-11-18)

Twenty-two contributions from the community, Windows in CI, and a README in 16 languages.

- From @rudycelekli, 20 fixes, each with a test that fails without it:
  - TypeScript: keyword-shaped export names survive `refresh` (#57); objects with more than 255
    properties fall back to dynamic wire keys (#58); recording works from a folder with `#` in its
    path (#59); large strings with lone surrogates replay without overflowing Node's argument
    limit (#60); exact `tsconfig` path aliases beat wildcards (#61); `.mjs`/`.cjs` specifiers
    find the authored `.mts`/`.cts` (#62); `bench` awaits async exports (#63).
  - C, Go, C#, PHP, Java: a C function or parameter named `c_str` no longer breaks the bridge
    (#64); private nested Go struct fields stay out of the wire schema (#65); `go.mod` directives
    with tabs and comments are read (#66); a root C# test project's sources count as tests (#67);
    grouped PHP `use` imports keep their namespace (#68); Go packages that use cgo can be recorded
    (#69); a comment in a Java package declaration no longer hides the class (#70).
  - Whole-app and HTTP: HTTP cases sharing a name are all kept (#71); curl `--data @file` bodies
    match curl (#72); an invalid `Content-Length` is an error, not a body read until close (#73);
    Node profiles work from a folder with spaces (#74) and decode `file://` URLs (#75); re-recording
    keeps a configured port (#76).
- From @atontini: native Windows CI for the bridge tests, with Windows fixes for absolute paths in
  run commands, Python's UTF-8 output and line endings (#78). On top of it: the Go driver finds the
  Rust DLL on Windows, and the Ruby test skips there, since RubyInstaller's MinGW Ruby cannot load
  an MSVC extension.
- From @lyufeng000: README translations in Chinese (simplified and traditional), German, French,
  Japanese, Korean and Russian (#36). Turkish, Azerbaijani, Uzbek, Kazakh, Kyrgyz, Turkmen, Tatar,
  Uyghur and Ukrainian followed, every translation is marked community-maintained, and each README
  links all 16 languages.
- CI runs on Node 24. `profile` on Node needs it, since Node 22 refuses `--cpu-prof` in
  `NODE_OPTIONS`.
- A social preview image (`docs/assets/social-preview.png`) without a version or star count, so
  link cards never go stale.

## 1.4.0 (2026-11-18)

Fixes from the first round of outside issues and pull requests, and the project's community files.

- Fix ([#31](https://github.com/erkamyaman/rust-anything/issues/31)): `bridge` takes a single source
  file inside a unit (`rust-anything bridge . pkg/core.py`) and ports just that file, so a helper
  script next to the real module can stay in the source language.
- Fix ([#32](https://github.com/erkamyaman/rust-anything/issues/32)): a name defined twice in one
  unit (a wrapper in `__init__.py` and the function it wraps in a submodule, say) made one
  definition silently replace the other. `bridge` now refuses with both locations and how to
  bridge the module you want on its own.
- Fix ([#33](https://github.com/erkamyaman/rust-anything/issues/33)): Python command-line entry
  points are kept in Python instead of offered as ports: functions the
  `if __name__ == "__main__":` block (or a package's `__main__.py`) calls, and functions returning
  an `argparse.ArgumentParser`. `app front-door` takes the command line over in a whole-app port.
- Fix ([#34](https://github.com/erkamyaman/rust-anything/issues/34)): a Python parameter typed with a
  dataclass now gets real instances in recorded cases, built from the dataclass's fields (defaulted
  fields sometimes left out), instead of only `None` and shapes that can never fit.
- From @elfenomenokim: optional dataclass parameters bind as `Option` so the generated crate
  compiles (#28), and Python's whole standard library resolves as stdlib, with unknown dependencies
  named by the language's registry instead of "npm" (#29).
- Community files: CONTRIBUTING, SECURITY (with private vulnerability reporting), CODE_OF_CONDUCT
  and ACCESSIBILITY, issue and pull request templates. The README names its author up top, opens
  with the speed demo, and lists its sponsors.

## 1.3.0 (2026-11-18)

Two new languages in preview (PHP and C#), a stricter `diff`, a sturdier napi bridge, and 18
fixes from the first outside contributor.

- 18 pull requests from @rudycelekli, merged with their history: HTTP responses that end early
  or carry informational headers (#7, #8), empty recordings no longer pass (#9), float tolerance
  keeps key order (#10), own `__proto__` keys survive (#11), curl examples with a query right
  after the host (#12), TS projects without a manifest and `.mts`/`.cts` (#13), `setup` keeps
  custom MCP server options (#14), replay deadlines cover child processes (#15), colliding case
  names (#16), binary CLI cases keep both streams exactly (#17), the Rust 1.97 minimum is declared
  and checked (#18), a missing port source fails `gate` (#19), Ruby recordings are published
  only when complete (#20), JNI names with `$` and non-ASCII (#21), Go internal packages and
  versioned modules (#22), generated Go names that clash with parameters (#23), and invalid Java
  replay arguments (#24).
- PHP, in preview: detected from `composer.json` or `.php` files. `analyze` reads Composer
  packages and PSR-4 roots, resolves `require` and `use`, finds functions and public static
  methods with native and phpdoc types, flags PHP's risks (loose `==`, `/` returning floats, byte
  strings against `mb_*`, PCRE features the `regex` crate lacks, `eval`, magic methods) and maps
  common Composer packages to crates. `analyze --whole-app` finds CLIs (`$argv`, composer `bin`)
  and servers (front controllers, `$_SERVER`). `app record`, `app replay` and `app front-door`
  work on PHP CLIs and on servers run with `php -S`. Bridging PHP units into Rust (PHP FFI over a
  C ABI) comes next; `bridge` says so for now.
- Fix ([#3](https://github.com/erkamyaman/rust-anything/issues/3)): `.gitignore` files in folders
  above the repository (a `*` in a home-folder `.gitignore`, for one) no longer hide every source
  file. Ignore files are read the way git reads them: from the repository root down, and not from
  parent folders at all outside a repository. When ignore rules still exclude every source file,
  `analyze` says so instead of returning an empty plan.
- PHP's built-in server echoes a `Host` header with the port it ran on; HTTP recordings now skip
  it, like `Date`.
- Fix ([#4](https://github.com/erkamyaman/rust-anything/issues/4)): Python units bridged on macOS
  failed to link (`Undefined symbols ... _Py_IsInitialized`). The PyO3 crate now gets the same
  `build.rs` the Ruby bridge has, which lets the interpreter provide Python's symbols at load time.
  `rust-anything refresh <crate>` adds it to crates bridged before this fix.
- Fix: a Rust panic no longer passes `diff` where the original also threw. An unfinished port
  (`unimplemented!`) used to "match" every recorded error case in TS, Ruby (a panic arrives as
  `RuntimeError`), Java (`RuntimeException`) and Go (a panic where the original panicked). Python's
  driver crashed on the first `PanicException` instead of reporting it, and `shadow` let it reach
  the app. Each language now has a test that an unported crate passes no case.
- The napi bridge, after a study of napi-rs:
  - errors keep their JS class: a port's `Err("TypeError: ...")` throws a real `TypeError`, a
    wrong argument type is a `TypeError`, and a panic carries `code: 'RUST_PANIC'`;
  - async exports reject their promise instead of throwing at the call;
  - `Uint8Array`, `Buffer`, `Map`, `Set`, `Date` and `bigint` cross the bridge with the markers
    recordings use, instead of turning into plain objects or throwing;
  - a string with a lone surrogate is refused with a `TypeError` instead of silently becoming
    U+FFFD in a Rust `String`;
  - `build` replaces the `.node` (and the PyO3 module) by renaming, and `diff` refuses to run when
    the built module is older than the crate's sources.
- From Reddit feedback:
  - `analyze` refuses a compiled app (APK, JAR, .exe and the like) with an explanation instead of
    an empty plan, and warns when a folder holds only compiled files.
  - Platform-dependent C (inline assembly, `volatile`, bitfields, packed layouts, compiler
    extensions, integers cast to addresses) is a new `platform-specific` risk, and its unit is
    kept in C: a recording on this machine says nothing about the target.
  - `analyze --whole-app` recognizes Electron apps and plans the main process, keeping the web UI.
  - `gate` has a `size` check: the port fails above 3x the original's lines (plus 50).
  - README FAQ: APKs, battle-tested C, Electron, C#, and code growth.
- C#, in preview: detected from a `.csproj` or `.sln`. `analyze` reads projects and their NuGet
  packages (mapped to crates, test projects left out), resolves `using` directives to folders,
  finds `public static` methods with their types, and flags C#'s risks (`decimal`, `dynamic`,
  LINQ queries, `Task`, `lock`, `unsafe`, reflection, regex features the `regex` crate lacks).
  `analyze --whole-app` finds CLIs (`<OutputType>Exe`) and ASP.NET servers (the web SDK,
  `WebApplication.CreateBuilder`). `app record`, `app replay` and `app front-door` run built C#
  apps (`dotnet out/app.dll`). Bridging C# units (P/Invoke over a C ABI) comes next; `bridge` says
  so for now.
- Fix ([#25](https://github.com/erkamyaman/rust-anything/issues/25)): a bridge crate in its default
  place inside the project (`rust/<crate>`) was read as source, so Java `record` compiled its
  generated wrappers next to the originals ("duplicate class"). Every source walk now skips
  folders holding a `rust-anything.json`.
- Fix ([#26](https://github.com/erkamyaman/rust-anything/issues/26)): on Windows, paths handed to
  `java -cp`, `-D` library paths and the other runtimes carried the `\\?\` verbatim prefix, so
  Java `record`, `diff` and `bench` could not find their classes. Paths are now made absolute
  without it everywhere.
- Fixes found by gating every demo port ([#27](https://github.com/erkamyaman/rust-anything/issues/27)
  reported the first): the generated binary wire tripped the new
  `chunks_exact_to_as_chunks` lint (every Go and Java port failed `gate`), and on macOS the
  Python and Ruby extension's test binary failed to link; the generated `build.rs` now covers it,
  and `refresh` adds it to Ruby crates too. CI now gates all demo ports on macOS.
- CI also runs every test on macOS, and a new `tests/native.rs` builds a napi addon and a PyO3
  extension for real, loads them from Node and Python and replays the recording, so link problems
  like #4 fail CI instead of reaching users.
- A `.github/FUNDING.yml` turns on the repository's Sponsor button.

## 1.2.0 (2026-11-17)

Numeric code, already compiled code, and the edges of what a recording proves, after the first
round of feedback. rust-anything is now on crates.io: `cargo install rust-anything`.

- Float tolerance for numeric code: `record --float-tolerance 1e-9` (and `app record`) lets floats
  match when they differ by at most that share of the larger value (absolute below 1); `diff`,
  `gate`, `app replay` and the lift use it. Integers, text, structure and formatting (`3` against
  `3.0`) still match exactly. `diff --float-tolerance` tries one for a single run.
- Noise is sorted by kind: when the original's two runs disagree, `app record` reports `values`
  (now `[..]`), `order` (the same lines in another order: replay now compares them in any order
  instead of wildcarding them, so a port must still produce exactly those lines) and `lines` (a
  `...` that matches anything), by case and field. The lift's trycmd test skips `order` cases and
  says to run `app replay` in CI for them.
- Regex traps: functions in files that use regexes get extra string inputs where engines
  disagree (Turkish i, Kelvin sign, long s, joiners, non-ASCII digits and spaces, line separators,
  final sigma), in all six languages. Every string trap now also includes a dotless `ı`, a
  zero-width-joiner emoji and a non-breaking space.
- C: recording runs the library in the "C" locale, as a C program has unless it calls
  `setlocale`. It ran in the user's locale (Python sets it at startup), so `isspace` and friends
  could record a different answer per machine; the new non-breaking-space trap caught it.
- The pitch says what passing means: the Rust matches the original's recorded behavior. The
  README has a "What it can't catch" section.
- Already compiled code: `profile` and `analyze` flag Python functions compiled by numba
  (`@njit`, `@jit`, `@vectorize`, ...), Cython or JAX, and the hot functions that call them, since
  a Rust port gains little speed there.
- Go: crates no longer build a static library. Go 1.26.11 and 1.27.4 refuse to link archives
  that carry LLVM bitcode, which every Rust static library does; cgo links the shared library
  anyway. Run `rust-anything refresh <crate>` on a Go crate bridged by an earlier version.

## 1.1.0 (2026-11-16)

Whole-app ports: rust-anything now takes a TypeScript/JavaScript, Python, C, Go, Ruby or Java app
all the way to a pure Rust app, proving at every step that it still behaves the same. CLIs and HTTP
servers in all six languages, and libraries (proven end to end in TypeScript, Python and C so
far); every `app` command is also an MCP tool (`app_record`, `app_replay`, `app_front_door`, `app_wire`, `app_lift`).

- Plan: `analyze --whole-app` gives the app's shape (CLI, server or library) and entry points,
  every external dependency (imported, or declared in package.json, pyproject or
  requirements.txt, go.mod, Gemfile or gemspec, pom.xml or Gradle) with its crate and fit, the port
  order as pieces (units that import each other move together), and numbered decisions with a
  recommended answer for the user. POSIX headers and `com.sun.*` now map to their Rust
  counterparts.
- Record: `app record` runs the original on its own documented examples, an `--inputs` file and
  semantic traps (non-ASCII and emoji, negative division and modulo, overflow, float extremes,
  map order, missing files), twice; what differs between the two runs becomes a wildcard, and
  named normalizers (`timestamps`, `uuids`, `key-order`, `name=regex`) are saved with the cases.
  CLI cases are trycmd files in `rust/app-cases/` (argv, stdin, env and files in; stdout, stderr,
  exit code and files out). With `--http` (or `--port`) the command starts a server, and requests
  from the `curl` lines in its docs, `--inputs` and traps are stored in `rust/app-cases/http/`.
- Replay: `app replay` runs any build (the original, the front door, the final binary) against
  the cases, prints `passed/total` and the fallback rate, writes a report with a diff per failing
  case, and exits non-zero on any mismatch, so CI can run it. Any build can report its fallbacks
  through the file named by `RUST_ANYTHING_FALLBACK_LOG`. `--case <name>` reruns one
  failing case, the fast loop for fixing a mismatch.
- Front door: `app front-door` scaffolds `rust/app`, which owns the entry point from day one and
  hands what is not ported yet to the original, counting every fallback: for a CLI a clap binary
  that runs the original with the same argv, stdin and environment; for a server an axum app that
  starts the original on its own port and proxies to it; for a library (no `main`, no server, or
  `--library`) a Rust crate with the public API, while the original package keeps its callers.
  Replay against a fresh front door passes 100% at 100% fallback.
- Wire: after a unit passes `gate`, `app wire` adds its crate to `rust/app` and routes commands
  (`--command`), routes (`--route 'GET /users/{id}'`, `*`) or, for a library, the unit's exports
  to it, behind a kill switch, `RUST_ANYTHING_UNIT_<NAME>=original`; it writes a glue file to fill
  in once, then builds and replays. For a library it writes the unit's shadow drop-in for the
  original package to import, and shadows in every language but C (switched at build time) honor
  the kill switch.
- Lift: once the fallback rate is 0 and every unit passes `gate`, `app lift` makes it pure Rust.
  It refuses until then and lists every blocker; it works on a copy, removes the fallback path and
  the kill switches, turns each unit crate into a plain library (no `cdylib`, bindings or
  generated wrappers), runs the lift checklist (fmt, clippy, tests, every target in
  `rust/DECISIONS.md`, assets, replay in CI) and replays the pure binary, and writes the result
  back only when all of it passes. CLIs get a trycmd test that runs the cases on `cargo test`.
  The original is never touched.
- `gate` checks error paths: every place the original fails on purpose (`throw`, `raise`, an
  `error` result) needs a recorded failing case, and `port.rs` may not `unwrap`, `expect` or
  `panic!` in a function whose original returns an error, unless marked `// INFALLIBLE: <why>`.
- `record` adds the semantic traps to every function: each trap value of a parameter's type, one
  parameter at a time, next to the generated cases.
- Proved end to end on a small CLI and a small HTTP server written in each of the six languages,
  replayed with the original moved away, and on libraries (a C library lifted to one pure Rust
  crate; TypeScript and Python libraries moved unit by unit).
- The skill gains a whole-app guide (`whole-app.md`, installed by `setup`): it grills the user in
  rounds with a recommended answer for each question, researches dependencies against primary
  sources, and keeps the port going across sessions with `rust/MAP.md` (claim one unit at a
  time), ADRs in `rust/DECISIONS.md`, `rust/GLOSSARY.md` and a `rust/HANDOFF.md` at each
  checkpoint. It fixes red replays one case at a time, can port a wave in parallel with
  subagents, and ends with `rust/cutover.sh` for the steps only the user can do. Grilling,
  research and these practices are adapted from Matt Pocock's agent skills.
- The pitch is now rewriting a whole app in Rust; making the slow part fast stays as a smaller use
  of the same tool.

## 1.0.1 (2026-11-16)

- npm: one `rust-anything` package that carries the binaries for macOS, Linux and Windows.

## 1.0.0 (2026-11-16)

First version: the full loop for TypeScript/JavaScript, Python, C, Go, Ruby and Java.

- Python: PyO3 crates, typed exceptions, literal defaults and keyword-only parameters kept.
- C: drop-in `extern "C"` replacements and a header; crash-safe recording through ctypes; shadow
  builds with `-D` renames.
- Go, Ruby, Java: one JSON dispatcher and generated wrappers (cgo package, Fiddle file, JNI
  classes) with the original API; generated drivers record and replay in the original runtime;
  shadow drop-ins per language.
- Direct bridges: PyO3 conversion without JSON and `pynum::Num`, a magnus extension for Ruby,
  typed JNI and cgo functions, and a generated binary wire for lists, maps, structs and several
  results in Go and Java, and for TS through napi-rs and wasm-bindgen; `bench` times calls in process and ends with a verdict.
- `profile`: hot functions of a real workload (node `--cpu-prof`, cProfile, `go test` pprof, JFR,
  a Ruby tracer) with callers, call counts and advice. Batch classes (`--batch fn:param`) for TS,
  Python, Ruby, Go and Java keep items in Rust and run on all cores, with mimalloc; a
  `<Function>Index` in `port.rs` prepares the items once. x18n's search: 20x faster than TS (11.5x in wasm), with an index that ranks by itself; Go
  and Java batches 4x and 3x faster than the originals.
- Run end to end on Go's `path`, Ruby's `shellwords`, Python's `colorsys`, and JDK and glibc
  behavior; fixes from those runs: exact float parsing, crash isolation, `module_function :name`,
  numeric inference for untyped parameters, ints in Python inputs.

- `analyze`: units, port order, risks with `file:line`, bridge per unit, npm-to-crate fit,
  readiness and priority; counts bridgeable exports; follows tsconfig paths and workspace packages.
- `bridge` / `refresh`: crates with napi-rs or wasm-bindgen bindings generated from `port.rs`;
  functions, async functions and classes; JS-only exports stay in TS with a reason.
- `record` / `diff` / `bench`: recorded TS behavior from shaped, edge-case inputs and class
  scenarios, replayed against Rust; timing with the bridge included.
- `shadow`: drop-in modules running TS and Rust side by side, `RUST_ANYTHING_MODE=old|shadow|new`.
- `gate`: fmt, clippy, tests, `unsafe` audit, nothing unported, build, API, behavior.
- `setup`: registers the MCP server and the workflow skill with Claude Code, Codex, Cursor and
  Gemini CLI, keeping other settings and backing up each file it changes; `--remove` and
  `--dry-run`.
- `mcp`: every step as an MCP tool; agent skill in `skills/rust-anything`.
- Install with `npm install -g rust-anything` (prebuilt binaries for macOS, Linux and Windows) or
  from the binaries attached to the GitHub release.

# Changelog

## Unreleased

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

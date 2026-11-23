# Tools

Generated from the MCP server's tool list by `tests/tools.rs`; do not edit by hand (`UPDATE_TOOLS_DOC=1 cargo test --test tools` rewrites it). Every tool is also a CLI command with the same options.

## Analyze repository

Tool `analyze` · CLI `rust-anything analyze`

Map a repository (TypeScript/JavaScript, Python, C, Go, Ruby, Java or Kotlin): units, port order (waves), risks with file:line, bridge per unit, dependency-to-crate fit, and `start_here`: the best first ports.

Read-only.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `root` | string | yes | Repository root. |
| `lang` | string |  | Source language; detected from the repository when omitted. |
| `whole_app` | boolean |  | Plan a whole-app port: entry points and shape (cli, server, library), every dependency imported or declared with its crate and fit, pieces in port order (units that import each other together), and numbered decisions with a recommended answer for the grilling round. |

## Scaffold unit crate

Tool `bridge` · CLI `rust-anything bridge`

Scaffold a Rust crate for one unit: napi-rs or wasm-bindgen bindings in src/lib.rs, stubs to port in src/port.rs, UTF-16 helpers when the unit uses string offsets. Never overwrites an existing folder.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `root` | string | yes |  |
| `unit` | string | yes | Unit id from analyze. |
| `kind` | string |  | Defaults to the plan's bridge. |
| `out` | string |  | Crate folder; defaults to <root>/rust/<name>. |
| `name` | string |  | Crate name; defaults to `<unit>-rs`. |
| `force` | boolean |  | Bridge a unit the plan marks blocked or keep. |
| `keep` | string[] |  | Exports to keep in the source language: `name` or `name: reason`. |
| `batch` | string[] |  | Batch classes, `function:param`: the function runs over many `param` values kept in Rust, on all cores, in one call. A `<Function>Index` struct in port.rs replaces the default per-item loop. |
| `as_json` | string[] |  | Type parameters the app only fills with data (`T` in `paginate<T>(items: T[])`): bridged as JSON values instead of keeping the export in TS. |
| `allocator` | string |  | Global allocator for crates with batches. mimalloc (default) is C and leaves wasm alone; rusty_alloc is pure Rust (no C compiler) and also serves wasm. |

## Regenerate bindings

Tool `refresh` · CLI `rust-anything refresh`

Regenerate a crate's bindings (lib.rs, index.cjs, js.rs) from the signatures in src/port.rs, after changing them, adding any new batch classes. Never touches port.rs.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `batch` | string[] |  | Batch classes to add, `function:param`: the function runs over many `param` values kept in Rust, on all cores, in one call. A `<Function>Index` struct in port.rs replaces the default per-item loop. |

## Record original behavior

Tool `record` · CLI `rust-anything record`

Run the original functions on generated edge-case inputs (non-ASCII, emoji, -0, NaN, ...) plus optional extra inputs, and save the results as cases.jsonl: the behavior the port must match.

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `count` | integer |  | Generated cases per function (default 200). |
| `inputs` | string |  | JSONL file of extra cases: {"fn": name, "args": [...]}. |
| `module` | string |  | JS only: import the original from this JS module instead of the TS source. |
| `resolve_from` | string |  | Resolve npm packages from this folder's node_modules. |
| `float_tolerance` | number |  | Numeric code: floats match when they differ by at most this times the larger magnitude (absolute below 1), e.g. 1e-9. Saved with the crate for diff and gate; 0 removes it. |
| `node_args` | string[] |  | Extra arguments for node, e.g. ["--import=tsx"]. |
| `test_command` | string[] |  | TS/JS or Python: also record every call the original's own test suite makes to the unit's functions, e.g. ["npx", "jest"] or ["pytest"]. Runs in the original's folder. |
| `profile` | string |  | Record on this machine (default) or on a connected Android device (JVM crates). |
| `device` | string |  | Android device serial; picked automatically when only one is connected. |
| `pin_clock` | boolean |  | TS/JS (napi) and Python: true pins the clock and random numbers while recording and diffing (`Date.now()`, `new Date()`, `Math.random`; `time.time`, `datetime.now`, `random`) to the values in pinned.json, and writes src/port/pinned.rs for the port to read them (`mod pinned;` in port.rs). Kept for later runs; false stops pinning. |

## Build crate

Tool `build` · CLI `rust-anything build`

Build the crate: the napi-rs .node, wasm-bindgen output, Python extension or shared library. With `profile: "android"`, build the JNI library for Android ABIs into `jniLibs/` (needs the NDK and the Rust Android targets; returned as {ndk, api, libraries} instead of {artifact}).

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `profile` | string |  | Build for this machine (default) or for Android. |
| `abi` | string[] |  | Android ABIs; defaults to arm64-v8a, armeabi-v7a and x86_64. |
| `api` | integer |  | Android API level (at least 26; default 26). |
| `ndk` | string |  | Android NDK root; found from ANDROID_NDK_HOME or the SDK when omitted. |
| `jni_libs` | string |  | Where to install the Android libraries; defaults to <crate>/jniLibs. |

## Diff port against recording

Tool `diff` · CLI `rust-anything diff`

Replay cases.jsonl against the built Rust module and list mismatches. With `profile: "android"`, replay on the device against the .so built for its ABI.

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `profile` | string |  | Diff on this machine (default) or on a connected Android device (JVM crates). |
| `device` | string |  | Android device serial; picked automatically when only one is connected. |
| `jni_libs` | string |  | Android libraries folder to diff; defaults to <crate>/jniLibs. |
| `ignore_key_order` | boolean |  |  |
| `module` | string |  | Test this JS module instead of the crate's `index.cjs`. |
| `node_args` | string[] |  | Extra arguments for node, e.g. ["--import=tsx"]. |
| `float_tolerance` | number |  | Floats match within this tolerance for this run (the one saved by record otherwise). |

## Profile workload

Tool `profile` · CLI `rust-anything profile`

Run a real workload (a command: node or python3 script, `go test -bench`, java, ruby) under the language's profiler and rank the repository's hot functions by inclusive time, with their callers, call counts and advice: what to port together, and which loops to batch (`bridge` with `batch`). Profile before porting: leaf functions called one at a time are slower in Rust.

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `root` | string | yes |  |
| `command` | string[] | yes | The workload, run in root: ["node", "bench.mjs"]. |
| `lang` | string |  | Detected from the repository when omitted. |

## Port the hot path end to end

Tool `go` · CLI `rust-anything go`

The hot-path loop in one call: profile the workload, pick the top portable function (the outermost loop; batched where profile says so), bridge it into rust/<function>-rs, record generated cases plus the calls the workload makes, write TASK.md, hand the port to `agent` (claude, codex, or a command reading the prompt on stdin), then build, diff and gate, and switch the unit on with `enable` only when every case matches, gate passes and `bench` shows it is not slower as its callers call it. Without `agent` it stops at `waiting_for_port`: port src/port.rs as TASK.md says and call go again; finished steps are kept. Returns the stage (waiting_for_port, mismatch, gate_failed, slower, on, passed), the speedup, the pick, the steps and what to do next.

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `root` | string | yes | Repository root. |
| `command` | string[] | yes | The workload, run in root: ["python3", "app.py"]. |
| `agent` | string |  | claude, codex, or a command that reads a prompt on stdin. Omit to port it yourself. |
| `function` | string |  | Port this hot function instead of the profile's pick. |
| `lang` | string |  | Detected from the repository when omitted. |
| `count` | integer |  | Generated cases per function (default 200). |
| `rounds` | integer |  | Agent turns: the port, then one per failing diff or gate (default 3). |

## Switch a unit on

Tool `enable` · CLI `rust-anything enable`

Switch a ported TS/JS or Python unit on: one marked block in each original source file rebinds the unit's functions to the built Rust module, for every caller, with no import changed. RUST_ANYTHING_MODE=old or RUST_ANYTHING_UNIT_<NAME>=original runs the original for one run. Refuses while recorded cases differ unless `force`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `force` | boolean |  |  |

## Switch a unit off

Tool `disable` · CLI `rust-anything disable`

Switch a unit off: removes exactly the lines enable added, leaving the source files byte for byte as they were.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |

## Write a report card

Tool `report` · CLI `rust-anything report`

Write <crate>/report/REPORT.md and card.svg for a pull request or README: the unit, recorded cases matched (diff), speedup call included (bench), fallback rate, on or off, and the machine with its core count. Builds first; every number comes from this run.

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `out` | string |  | Output folder; defaults to <crate>/report. |
| `min_ms` | integer |  | Minimum measured time per function and side, in milliseconds (default 200). |
| `node_args` | string[] |  | Extra arguments for node, e.g. ["--import=tsx"]. |

## Benchmark port

Tool `bench` · CLI `rust-anything bench`

Time the recorded cases on the original and on the built Rust module, bridge included; speedup below 1 means the port is slower as its callers call it. Returns the rows and a verdict with advice: ship, keep the slower functions, or port a bigger unit.

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `resolve_from` | string |  |  |
| `min_ms` | integer |  | Minimum measured time per function and side, in milliseconds (default 200). |
| `node_args` | string[] |  | Extra arguments for node, e.g. ["--import=tsx"]. |

## Fuzz port against original

Tool `fuzz` · CLI `rust-anything fuzz`

Live differential test: generate fresh inputs from a new seed (the record generator, without the cases already recorded), run them through the original and the built port, and report mismatches the recording missed. Returns the seed to replay, counts and the first mismatches; `keep` appends mismatching (or all) fresh cases to cases.jsonl so the next diff and gate check them. Works wherever record and diff do on this machine (TS/JS, Python, C, Go, Ruby, Java, Kotlin).

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `cases` | integer |  | Fresh cases per function (and scenarios per class) in each round (default 200). |
| `seed` | integer |  | Seed of the first round; random when omitted (returned as `seed`). Later rounds count up from it. |
| `time` | number |  | Keep running new rounds until this many seconds have passed; one round when omitted. |
| `keep` | string |  | Append the mismatching fresh cases, or all of them, to cases.jsonl. |
| `resolve_from` | string |  | Resolve npm packages the original imports from this folder's node_modules. |
| `node_args` | string[] |  | Extra arguments for node, e.g. ["--import=tsx"]. |

## Generate shadow module

Tool `shadow` · CLI `rust-anything shadow`

Generate a drop-in shadow (a TS or Python module, Go package, Ruby file, Java classes or C file) that runs the original and the Rust port side by side and reports mismatches; RUST_ANYTHING_MODE=old|shadow|new switches at runtime.

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `out` | string |  |  |

## Map the port

Tool `map` · CLI `rust-anything map`

Port map of a repository: each unit of the original with the crate that ports it, its functions in Rust and kept in the original, its cases and last diff, the whole app's glue (Rust only, or still handing cases to the original), the units not ported yet in the order analyze suggests, and every port whose original changed since it was recorded (`changed_upstream`: re-record it and diff). Read-only; the CLI's `map --write` saves it as rust/PORT_MAP.md.

Read-only.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `repo` | string | yes | Repository root. |
| `crates` | string[] |  | Folders with crates kept outside the repository. |

## Lay out packages

Tool `package` · CLI `rust-anything package`

Lay out installable packages for a gated port; writes files only, never builds other platforms or publishes. PyO3: a maturin project (abi3 wheel importable under the original module's name). CLI front door (`rust/app` of a Node CLI): a main npm package whose command runs `<name>-<platform>`'s front door binary, with the original pinned to its exact version as a dependency for every case the port hands back. napi: a main npm package whose loader picks `<name>-<platform>` from optionalDependencies and falls back to the original JS, one package per platform (this machine's binary copied in when built). wasm: one npm package with the built pkg/. PyO3 and napi also get a GitHub Actions workflow building macOS (arm64, x86_64), Linux (x86_64, aarch64) and Windows.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `name` | string |  | Package name on npm or PyPI; defaults to the crate name. |
| `version` | string |  | Package version; defaults to the original package's. |
| `out` | string |  | Output folder; defaults to <crate>/package. |
| `fallback` | string |  | napi: module to load where no binary fits; defaults to the original file when it is plain JS that imports nothing. |
| `bin` | string[] |  | CLI front door: the commands the package installs; defaults to the package name without its scope. |
| `original` | string |  | CLI front door: the original npm package it falls back to, as <name>@<version>; defaults to the one in the repository's node_modules. |
| `dry_run` | boolean |  | List the files, write nothing. |

## Check definition of done

Tool `gate` · CLI `rust-anything gate`

Definition of done: fmt, clippy -D warnings, cargo test, unsafe with SAFETY comments only, no unimplemented functions, build, API present, recorded behavior matches.

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `crate` | string | yes | Folder of the generated crate (holds rust-anything.json). |
| `node_args` | string[] |  | Extra arguments for node, e.g. ["--import=tsx"]. |

## Record whole app

Tool `app_record` · CLI `rust-anything app record`

Record the whole CLI app's behavior: run the original on its own documented examples, an inputs file and semantic traps (non-ASCII and emoji, negative division, overflow, float extremes, map order), twice each. What differs between runs is reported by kind in `noise`: values (now [..] wildcards), order (lines compared in any order) and lines (a ... that matches anything): show every one to the user. Cases are trycmd files in <repo>/rust/app-cases/ (argv, stdin, env, files in; stdout, stderr, exit code, files out).

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `repo` | string | yes | Repository root. |
| `run` | string[] |  | Command that runs the original, paths relative to repo: ["python3", "cli.py"]. Optional after the first record. |
| `inputs` | string |  | JSONL of extra cases: {"name", "args": [...], "stdin", "env": {...}, "files": {path: text}}. |
| `normalize` | string[] |  | Normalizers: timestamps, uuids, key-order, or name=regex. |
| `name` | string |  | Name the app is invoked by in its docs. |
| `timeout` | integer |  | Seconds before a run is killed (default 30). |
| `traps` | boolean |  | Add semantic-trap cases (default true). |
| `http` | boolean |  | The app is a server: `run` starts it (it takes its port from PORT or {port}); cases are requests from curl lines in its docs, `inputs` ({"name", "method", "path", "headers", "body"}) and traps. |
| `port` | integer |  | A port the server always listens on (implies http). |
| `effects` | string[] |  | Record what leaves the process: "http" (a local sink set as HTTP_PROXY, http_proxy and RUST_ANYTHING_HTTP_SINK) and "smtp" (a local mail server, SMTP_HOST and SMTP_PORT). Each case stores what reached them, in order, and replay compares it. |
| `effect_env` | string[] |  | Point the app's own settings at a sink: VAR=http-url, VAR=smtp-host, VAR=smtp-port or VAR=smtp-url. |
| `db_reset` | string |  | Shell command that resets the database before each case (each start for servers); cases then run one at a time. |
| `db_dump` | string |  | Shell command that prints the database's state (e.g. `sqlite3 app.db .dump`), run after each case or request and recorded with the effects. |
| `args` | string[] |  | Servers: arguments every start gets after its command, the original's and every build's (["db.json"]). Implies http; saved. |
| `fresh` | string[] |  | Repository files or folders the app reads or writes (a project tree, a database file), snapshotted and copied into a fresh working directory before every case (CLIs) or start (servers); a CLI's removed files are compared too. Saved. |
| `pace` | integer |  | Servers: milliseconds to wait after each response, for work the server does after answering (a file watcher). Implies http; saved and used by replay; 0 removes it. |
| `float_tolerance` | number |  | Numeric output: numbers match when they differ by at most this times the larger magnitude (absolute below 1), e.g. 1e-9. Saved with the cases; 0 removes it. |
| `float_file` | string[] |  | Written files of raw floats, compared value by value within float_tolerance: `PATH=f32le` or `PATH=f64le`. Saved with the cases. |

## Replay app cases

Tool `app_replay` · CLI `rust-anything app replay`

Replay <repo>/rust/app-cases/ against a build (the original, the front door, the final Rust binary): passed/total, fallback rate when the build reports it, and a report with a diff per failing case (rust/app-cases/replay-report.md).

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `repo` | string | yes | Repository root. |
| `build` | string |  | Command line of the build to test; the original when omitted. |
| `port` | integer |  | Servers: the port the build always listens on; otherwise it gets a free one through PORT and {port}. |
| `timeout` | integer |  | Seconds before a run is killed (default 30). |
| `case` | string |  | Replay only this case, by the name the report gives it (a library: only this unit): the fast loop for fixing one mismatch. |

## Scaffold Rust front door

Tool `app_front_door` · CLI `rust-anything app front-door`

Scaffold <repo>/rust/app, the Rust binary that owns the CLI's entry point: a clap parser for the recorded commands and flags, every command running the original with the same argv, stdin and env (each fallback counted). For a server: an axum app that starts the original on a port of its own and proxies every route no unit answers. Builds it and replays the cases: 100% passed at 100% fallback. Needs app_record first; never overwrites rust/app.

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `repo` | string | yes | Repository root. |
| `check` | boolean |  | Build and replay (default true). |
| `library` | boolean |  | Treat the repository as a library even if it has a main: the front door is a Rust crate with the public API, re-exporting each wired unit's port. Found by default when there is no main and no server. |

## Wire unit into app

Tool `app_wire` · CLI `rust-anything app wire`

After a unit passes gate, hand its commands to its port.rs: adds the unit crate to rust/app as a library, writes a glue stub (src/units/<unit>.rs, yours to write: parse args, call <crate>::port, print what the original prints), regenerates the routing with a kill switch RUST_ANYTHING_UNIT_<NAME>=original, then builds and replays. Run it again after writing the glue: 100% passed and a lower fallback rate.

Runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `repo` | string | yes | Repository root. |
| `unit` | string | yes | Unit crate folder, its name under rust/, or the unit id it was bridged from. |
| `commands` | string[] |  | Commands the unit answers; kept from the last wire when omitted. |
| `routes` | string[] |  | Servers: routes the unit answers, axum syntax: "GET /users/{id}", "/path" for any method, "*" for everything else; kept from the last wire when omitted. |
| `skip_gate` | boolean |  | Do not run gate on the unit first. |
| `check` | boolean |  | Build and replay (default true). |

## Lift to pure Rust app

Tool `app_lift` · CLI `rust-anything app lift`

Make rust/app a pure Rust app. Refuses (and lists every blocker) unless replay passes with fallback rate 0 and every unit passes gate. Then, on a copy first: removes the fallback and kill switches, turns each unit into a plain library (no cdylib, bindings or generated wrappers), adds a trycmd test running rust/app-cases on cargo test, checks fmt/clippy/tests, DECISIONS.md targets and assets, and replays the pure binary. Never touches the original.

May overwrite files, runs the original's code or commands.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `repo` | string | yes | Repository root. |

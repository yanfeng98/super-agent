# Pitfalls

Mistakes already made in this repository, each with the rule that prevents it. Add one in the
same change whenever a mistake is worth not repeating.

## Generated code and bridges

**A helper in `port.rs` became part of the bridged API.** Making `parse_options` `pub` exported it
through the bridge. Keep helpers private; only the unit's exports are `pub`.

**Wiring every napi function broke crates without `serde_json`.** `required_deps` must add each
crate the generated code uses. After changing codegen, bridge a unit with no JSON parameters and
build it.

**Generated helpers unused by a crate failed clippy.** Emit each helper only when the crate needs
it (as `JOINED_IN_RS` and `JOINED_OUT_RS` are), and run `gate` on a crate that uses none.

**Number formatting picked the wrong digit on ties.** A hand-written shortest-digits routine gave
`...813` where V8 gives `...812`. Take digits from `serde_json` (zmij), which rounds to the closest
value as V8 does, and test ties.

**Two front doors overwrote each other's binary.** With `CARGO_TARGET_DIR` set, every app built
into the same `target/`, so replays ran another app's binary. The front door now builds into its
own `rust/app/target`; run `cargo test` with the variable set after touching the build.

**Glue could not hand a single case back.** A wired unit that covers only part of the input (a
formatter with unported syntax) had no way to decline after reading stdin. Glue calls
`crate::original::run_with_stdin(args, input)` to pass the case and the bytes it already read;
`lift` refuses to finish while any glue still contains `original::`.

**Deep JSON stopped recording at 128 levels.** serde_json's recursion limit rejected Prettier's
doc trees. Case and input lines are read with `unbounded_depth` and the limit disabled, in the
harness and in generated `from_json`; a test reads a 200-deep line.

**`analyze` suggested a unit `bridge` refused.** It judged exports with its own copy of
`bridge`'s rules, which lacked the sibling `.d.ts`, alias resolution and the class-method check,
so a unit `bridge` keeps in JS went into `start_here`. Both now call `bridge::ts_functions` and
`bridge::class_reason`; a new rule for what crosses the bridge goes there, not into one command.

**A bridge found no functions in a module that exports a list.** `export { a, b as c }` was not
read. `ts/exports.rs` reads export lists; check `bridge` on a unit written that way.

## Recording

**Capture code tripped the project's coverage threshold.** Code `record --from-tests` appends to
the original counted against Jest's limit. Wrap anything added to user files in
`/* c8 ignore start */` and `/* istanbul ignore next */`.

**A test command's output corrupted the MCP stream.** The child wrote to the server's stdout.
Send every child's stdout to stderr or a pipe, never to the MCP server's stdout.

**A project header shadowed the C library's.** ffmpeg ships `libavutil/time.h`, and with that
folder on the `-I` path, `#include <time.h>` in `parseutils.c` found it instead of the system's.
A header folder holding a file named like a C standard header goes on `-iquote`; check that a
C project's originals compile before reading anything into a failed `record`.

**A relative crate path pointed nowhere.** The compiler and cargo run in a scratch folder, so
`rust-anything build .` looked for `./Cargo.toml` there. The CLI makes crate paths absolute;
anything new that passes a path to a child process should take it absolute too.

**A test runner in watch mode never ends.** Run every outside program through `crate::process`,
which closes stdin and stops it at a time limit; `from_tests` also sets `CI=1`.

**Coverage listed one file three times.** Paths from import walking kept `..` segments. Compare
and print canonical paths.

**A CLI's recorded writes were invisible because the fixture already held them.** A CLI that
regenerates project files was recorded on a project it had just generated, so every case rewrote
files with the bytes they already had, and a front door that wrote nothing passed. Record such
CLIs from a fixture whose generated files are stale (overwrite them with a marker first), and
check that the `*.out` folders are not empty.

**A front door could not find the original through a symlinked `rust/`.** The generated
`original.rs` resolved the repository as `CARGO_MANIFEST_DIR/../..`, which is the link target's
parent. Builds by rust-anything now set `RUST_ANYTHING_REPO`; set it yourself when you build such
a front door with `cargo`.

**A server's recording carried state from its previous run.** json-server rewrites its database
on every POST, so the second recording run started from the first one's writes. Record servers
that write files with `--fresh <path>`, and `--pace` when they do work after answering.

**Mail cases timed out on macOS CI.** Python's `smtplib` looks up the machine's full host name
for its greeting, and on GitHub's macOS runners that lookup outlasted the 30-second case limit.
Test apps that send mail pass `local_hostname="localhost"`; an app under test that hangs the same
way needs it too, or a longer `--timeout`.

**Recorded case names were cut to 24 characters.** Hundreds of `--inputs` names shared a prefix
and collided. Given names keep 96 characters (`given_slug`); look at the case count after
recording against the number of input lines.

**A Java case failed on a JVM warning.** With many JVMs starting at once, one found its
`/tmp/hsperfdata_<user>/<pid>` file locked and printed `[warning][perf,memops] Cannot use file`
to stdout, so the replay's stdout differed. Java test apps run with `-XX:-UsePerfData`; an app
recorded on a busy machine needs it too.

**A broken port passed because its case was never recorded.** Recording drops a case whose
result JSON cannot hold (a PHP string that is not UTF-8). The only `html_entity_decode` input with
`&#39;` also decoded `&nbsp;` to a lone Latin-1 byte, so a port that wrongly decoded `&#39;` passed.
After writing inputs for a behavior, check that `cases.jsonl` holds them, and break the port once
to see `diff` fail.

## Numbers and claims

**Benchmarks taken while other builds ran were off by a wide margin.** Run `bench` on a quiet
machine, run it twice, and name the hardware and core count next to every published number.

**A slow server row was the network, not the code.** json-server's lifted binary served a large
list at 33 req/s while the same handler took 7 ms in process: no `TCP_NODELAY`, so the second
write of each response waited for a delayed ACK. Time the handler in process next to every HTTP
number; generated servers set `TCP_NODELAY`.

**Every number in the docs is measured.** Say what was run, on what, and the result, including the
slow cases (the single-call row is slower than JS on some units). No rounding up for a headline.

**Recorded CLI cases never saw a config file.** `app record` runs each case in an empty
directory, so Prettier's `.prettierrc`, `.editorconfig`, `package.json` `"prettier"` key and
`.gitignore`/`.prettierignore` never applied, and a front door that ignored them passed every
case; run from inside a project it formatted differently from Prettier. A CLI that reads files
around its input needs cases recorded with those files, or glue that sends such cases back.

**A replay reported the original, not the front door.** `app replay` without `--build` runs the
recorded command (the Node CLI), passes everything, and prints no fallback rate; `app wire
--no-check` replays nothing. Pass `--build <front door binary>` and read the fallback line.

**`gate` checked less than it seemed for a split port.** Its unwrap, panic and size checks read
`port.rs` only, and `size` counts the whole import closure (node_modules included). When the port
lives in a dependency crate, run clippy with `-D warnings` on that crate and grep it for
`unwrap`/`panic!` yourself, and do not quote the size ratio.

**A port followed the wrong path key.** Prettier's `path.match(...)` gives predicate *i* the node
*i* levels up and the key under which the level below sits. Port the semantics and test a
`needs-parens` case before reusing it.

**A port compiled its regexes on every call.** Prettier's `printNumber` is four regex replaces;
ported with `Regex::new` inside the function, an array of 100 numbers took 23 ms instead of 0.7.
Compile once (`OnceLock` or a thread-local cache), and look at the slowest cases of every bench,
not only the mean.

**A port read its AST through hashed maps.** Bridge crates turn on serde_json's
`preserve_order`, and Cargo unifies features across the binary, so every field lookup in a
`serde_json::Value` tree hashed the key. Reading oxc's ESTree into a small vector-backed value
type with interned keys made the printer about 1.7x faster. Profile before blaming the algorithm.

**A benchmark timed a stale binary.** `cargo build --example bench | tail -1 && run` took the
pipe's status from `tail`, so a missing example went unnoticed and the old binary skipped every
CSS file (`total_us 0`). Check the build's own exit status (`set -o pipefail`) and that the
timed run covered as many files as the list it was given.

**A filter hid real cases from a count.** The corpus checker skipped every name containing
`config-` to leave out the config cases, and four real files (`vitest.config.ts`, ...) went with
them. Match the exact prefix you mean, and check a group's total against the corpus.

**A harvester dropped whole rules.** ESLint's tests for no-unused-vars, prefer-const and
no-useless-assignment give their `RuleTester` a test-only plugin, and the script that turned
RuleTester cases into CLI runs skipped every case of a tester with plugins: 715 cases of three
of the most used rules were missing, and the rules showed "0 cases" instead of an error. Skip
only the cases that use what you cannot express, and check every unit's case count after
harvesting.

**A copy of a crate built into the same target folder replaced it.** A scratch copy of the port,
with the same package name and version, was built with the same `CARGO_TARGET_DIR`; cargo gave
both the same artifact name, and examples of the real crate linked the copy's library and
reported stale numbers. Give a copy its own package name or its own target folder.

**A grep hid the bug it was meant to show.** Filtering diffs with a pattern that drops comment-like
lines removed the one line that differed (`/* name */ value`). Write the expected and actual
output to files and diff them whole.

**An exhaustive check covered a different input set from the one the port builds.** The Prettier
doc printer was checked on every doc up to size 5, read from JSON through `from_value`, which
turns `align: ""` into a no-op as Prettier does. The YAML and Markdown printers build
`Align::Text("")` directly, and the printer indented for it. Enumerate the values the code really
passes (native `Doc` values), not a serialised form of them.

**Mutation jobs tested each other's binaries.** With `CARGO_TARGET_DIR` set, every
`cargo mutants -j N` job built into one target dir, so 401 of 693 logs ran a binary another job
had built. Run cargo-mutants with `CARGO_TARGET_DIR` unset, and check that every log under
`mutants.out/log` compiles the mutated crate.

**Recorded crash messages held Prettier's install path.** Prettier prints a stack trace when it
crashes, so cases recorded with one copy of Prettier fail when the front door falls back to
another. Record with the copy the front door's `ROOT` resolves to.

**Sorting cannot pair lines that are matched loosely.** An `unordered` output was compared by
sorting both sides as text and pairing the lines by position, while the lines were then matched
within `float_tolerance`: `10.0` sorts before `2.0`, so it met `2.0` instead of the
`9.999999999999999` it matched, and the weaker check failed output the ordered one accepted.
Sorting by value instead only moved the problem -- `[..]` and `2` against `2.0` then paired wrong,
because a loose match is not transitive and no order agrees with it. Pair the lines against each
other (identical ones first, then a search that gives a line up when another needs it), and keep
the sort for showing the diff.

**A demo's lockfile missed a dependency.** The Prettier front door gained `ignore` for config
resolution, but its `Cargo.lock` was not updated: the next plain build rewrote it, and a
`cargo build --locked`, as the release workflow runs, would have failed. After adding a dependency
to a demo crate, build it and keep the lockfile it writes.

**An installed CLI failed replays on paths it prints about itself.** ESLint's stack traces and
"There was a problem loading formatter" name the file ESLint was loaded from, so 93 recorded runs
differed once ESLint lived in an install prefix. Replay the original from the same install too:
a run that fails both ways is the path, not the port.

## Language plugins

**A tree-sitter supertype never appears in the tree.** `tree-sitter-kotlin-ng` lists
`class_member_declaration` as a child of `class_body`, but it is a hidden supertype: the tree
holds the concrete `function_declaration` directly. Filter on the concrete kind you parse, and
check a small fixture's S-expression before trusting `node-types.json` child lists.

**Kotlin's JVM owner is not the file name.** A top-level function compiles to a static method on
`<File>Kt` (`NativeLib.kt` → `NativeLibKt`); only `@JvmStatic` members of an `object` or
`companion object` live on the enclosing class. Emit prototypes in that Java shape, or the
generated JNI symbol and the reflection driver look for a class that does not exist.

**The generated Java runtime needs API 26, not API 21.** `RustAnythingError` resolves exception
constructors with plain reflection instead of `MethodHandle.invokeExact` (which Android only
supports from API 26), but the runtime also uses `java.util.Optional` and `java.util.function`
(API 24) and `ThreadLocal.withInitial` (API 26). `build --profile android` therefore rejects
`--api` below 26: a lower library would load but fail in the wrapper before reaching Rust.
Link with `-Wl,-z,max-page-size=16384 -Wl,-z,common-page-size=16384` for 64-bit ABIs, or Play
rejects the library on 16 KB devices.

**ART is not a desktop JVM, even when the bytecode is Java 8.** The generated driver uses
`Paths.get`, not `Path.of`, and reads/writes files with `Files.readAllBytes`/`Files.write`, not
Java 11's `Files.readString`/`writeString`: those methods throw `NoSuchMethodError` on Android even
at API 30. The driver is dexed with `d8 --min-api 26` and started with `app_process`.

**On-device recording needs Android SDK build-tools.** `d8` and a platform's `android.jar` come
from the SDK, which can live somewhere other than `ANDROID_HOME` (the variable often points at an
old SDK); the device scratch directory under `/data/local/tmp` is removed after every run.

**NDK discovery must ignore half-installed NDKs.** `--ndk` wins, then `ANDROID_NDK_HOME` /
`ANDROID_NDK_ROOT`, then the newest *complete* NDK under `ANDROID_HOME` / `ANDROID_SDK_ROOT`, then
the usual `~/Android/Sdk`, `~/Library/Android/sdk` and `~/Documents/Android/Sdk`. Completeness
means `toolchains/llvm/prebuilt/<host>/bin` exists: an interrupted `sdkmanager` download leaves a
newer directory with only `source.properties`, which must not shadow a working NDK. The linker is
the NDK's clang, so `cargo-ndk` is not needed, and cargo runs in the crate's directory so the
caller's `.cargo/config.toml` cannot leak into the generated crate.

**Two parsers for one manifest drifted apart.** `whole_app::declared()` re-read go.mod, pom.xml,
Gradle files, Gemfiles and requirements files with its own code, and five comment fixes landed
there while the plugins' `discover()` kept the bugs. Read a manifest in one place, the plugin's
`discover()`, and have every other consumer take `Package.deps` from it.

**A name join split one library in two.** `--whole-app` matched imported names (`yaml`) against
declared ones (`PyYAML`) as strings, so most libraries showed up twice. Join spellings by the crate
table row they resolve to (`CrateMatch::row`), not by the name or the `crates` text.

## Repository

**A local path reached a committed manifest.** Demo manifests keep a placeholder `root`; check
`git diff --cached` for `/Users/` and `/home/` before committing (`scripts/check-secrets.sh`
does this in the pre-commit hook).

**A new translation was listed right after English.** Language lines list English, Türkçe,
Українська first, then the other existing languages in their order, then new ones at the end (see
CONTRIBUTING.md).

**Two sessions edited one checkout.** When another agent is working in the repository, work in
a separate worktree and bring the change over when its files do not overlap.

**The Ruby tests failed on a clean macOS shell.** `/usr/bin/ruby` is Ruby 2.6, which rb-sys's
stable API does not support (`Stable API is needed but could not find a candidate`). A test that
builds a Ruby extension checks for Ruby 3 with `common::have_ruby_3()`, not `common::have("ruby")`,
and skips without it; to run those tests, put a current Ruby first on `PATH` (Homebrew's
`/opt/homebrew/opt/ruby/bin`) before `cargo test`.

**A test failed on Linux CI with "Text file busy".** It wrote a fake `nm` script and ran it while
other tests were starting processes; a child forked during the write still held the file open for
writing, so exec failed. Write a script that a test runs from a separate process (`cp` from a text
file) and never rewrite one that already ran.

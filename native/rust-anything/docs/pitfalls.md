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

**A bridge found no functions in a module that exports a list.** `export { a, b as c }` was not
read. `ts/exports.rs` reads export lists; check `bridge` on a unit written that way.

## Recording

**Capture code tripped the project's coverage threshold.** Code `record --from-tests` appends to
the original counted against Jest's limit. Wrap anything added to user files in
`/* c8 ignore start */` and `/* istanbul ignore next */`.

**A test command's output corrupted the MCP stream.** The child wrote to the server's stdout.
Send every child's stdout to stderr or a pipe, never to the MCP server's stdout.

**A test runner in watch mode never ends.** Run every outside program through `crate::process`,
which closes stdin and stops it at a time limit; `from_tests` also sets `CI=1`.

**Coverage listed one file three times.** Paths from import walking kept `..` segments. Compare
and print canonical paths.

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

**A grep hid the bug it was meant to show.** Filtering diffs with a pattern that drops comment-like
lines removed the one line that differed (`/* name */ value`). Write the expected and actual
output to files and diff them whole.

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
stable API does not support (`Stable API is needed but could not find a candidate`). Put a current
Ruby first on `PATH` (Homebrew's `/opt/homebrew/opt/ruby/bin`) before `cargo test`.

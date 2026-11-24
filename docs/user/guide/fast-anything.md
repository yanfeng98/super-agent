# Make the slow part fast with fast-anything

English | [中文](fast-anything.zh.md)

The Web `standard` agent preset mounts the [fast-anything](https://github.com/EL4CTEO/fast-anything) MCP server, so a default Session can speed up a Python, Node.js/TypeScript, Ruby, Java, Kotlin, Go, PHP, C, C++, C# or R app by moving only its hottest pure functions to Rust. Each port is checked against the calls the original really made, and nothing is switched on until every recorded call, derived edge case and shrunk input matches. Nothing is installed globally: the CLI is compiled from the crate vendored in this repository.

## What ships

| Phase | Tools |
|---|---|
| Find the hot functions | `fast_scan` |
| Port one function | `fast_port`, `fast_verify` |
| Switch a port on | `fast_enable`, `fast_disable` |
| Measure and report | `fast_bench`, `fast_report`, `fast_status` |

## Before the first Session

`pnpm install && pnpm run build` compiles the CLI and stages it as `native/fast-anything/bin/fast-anything`. The build needs Rust 1.88 or newer on `PATH`; a newer toolchain, such as the 1.97.0 the repository's other Rust build installs, also works, so no toolchain file pins this crate. `pnpm dsh web` does not rebuild, so build once (and again after the vendored crate changes) and restart the server; Sessions that were already open keep the tools they started with.

## Ask for a speed-up

Describe the work and the matching [skill](../../../packages/skill/skill-fast-anything/assets/dsh-fast-anything/SKILL.md) loads, or type `/` in the composer and pick `dsh-fast-anything` from the **Built-in · EL4CTEO/fast-anything** group to load it first. For example:

- "This script takes five seconds; find the hot function and make it faster."
- "Profile this command and port the slowest pure function to Rust."
- "The dedupe step dominates the run: move it to Rust and prove it behaves the same."
- "Undo the Rust port for `fuzzy.levenshtein` and measure the difference."

The loop is `fast_scan`, `fast_port`, write the Rust the returned task describes, `fast_verify`, `fast_enable`, `fast_bench` and `fast_report`; `fast_disable` switches a function back, and `fast_status` lists what has been scanned and ported. Ask the agent to list the tools starting with `mcp__fast-anything__` when you want to confirm the mount.

## What it leaves behind

Scanning and porting write a `.fast/` directory into the project: the recorded calls and the generated one-function crate for each port, a `TASK.md` describing what to write, and `report.md` plus `report.svg` after `fast_report`. Every generated crate compiles into the shared `.fast/target` cache; `fast-anything clean` removes it, and `clean --all` also switches every port back. An enabled port is one marked block at the function's call sites, and `fast_disable` removes it byte for byte. `FAST_ANYTHING_DISABLE=1` runs the original code for a single run without touching the project.

## Use the CLI directly

```sh
./native/fast-anything/bin/fast-anything scan -- python3 app.py
./native/fast-anything/bin/fast-anything port fuzzy.levenshtein
./native/fast-anything/bin/fast-anything verify fuzzy.levenshtein
./native/fast-anything/bin/fast-anything enable fuzzy.levenshtein
./native/fast-anything/bin/fast-anything report
```

The MCP tools take an optional `path` for the project folder and otherwise use the server's working directory; the CLI uses the current directory.

## Costs and limits

- Porting needs Rust 1.88 or newer and the runtime of the app being measured. `fast-anything doctor` reports which parts are ready: `scan` needs only the app's runtime, `verify` and `bench` build with Rust.
- The tools execute the app's code with the Session's environment and write into the project, and `fast_enable` edits the project's sources. Run them on a project copy when the code is not disposable, review the diff `enable` leaves, and keep production credentials out of the environment.
- fast-anything ports the outermost loop it can, so a function whose helpers do I/O, randomness, globals, or that lives in the entry script is reported as not portable with the reason. Moving the pure logic into its own module is usually what makes it portable.
- This is not a whole-app rewrite: the rest of the app stays as it is. For rewriting an app in Rust, use the [rust-anything migration guide](./rust-anything.md).
- Escape hatch: `fast-anything clean --all`, `fast_disable`, or `FAST_ANYTHING_DISABLE=1` returns the app to its original code.

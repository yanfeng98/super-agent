---
name: dsh-fast-anything
description: Speed up a slow Python, Node.js/TypeScript, Ruby, Java, Kotlin, Go, PHP, C, C++ or C# app by porting only its hottest pure functions to Rust and proving each port against the real calls the original made - scan, port, write the Rust, verify, enable, bench and report, then undo with disable. Use when asked to make code faster, find hot paths, profile an app, or move the slow part to Rust.
---

# Port the hot paths to Rust with fast-anything

fast-anything measures, records, builds, checks and swaps. You write the Rust.
The steps are the same as MCP tools (`mcp__fast-anything__fast_scan`, `fast_port`,
`fast_verify`, `fast_enable`, `fast_disable`, `fast_bench`, `fast_report`,
`fast_status`) or as CLI subcommands (`fast-anything scan|port|verify|enable|disable|bench|report|status`).
Pass the project root as `path` on every tool call; it defaults to the server process's
working directory.

If the MCP tools are not available, use this repository's build at
`native/fast-anything/bin/fast-anything` (produced by `pnpm run build`). Do not install a
global copy.

## Loop

1. **Scan**: `fast_scan` with the command the user runs their app with (a representative
   workload, not the test suite), e.g. `["python", "app.py"]`. Pick the top function marked
   as worth porting. Skip anything under 2% of the run: porting it cannot make a visible
   difference. Read `why_not` for the functions that stay.
2. **Port**: `fast_port` with that id. It re-runs the app, records up to 500 real calls, and
   creates `.fast/ports/<id>/` with a stub `src/lib.rs` and a `TASK.md`. Read TASK.md fully.
3. **Write the Rust** in the returned task's crate:
   - Same semantics: Unicode code points (`.chars()`; JavaScript compares UTF-16 units),
     integer ranges, float operation order, error cases.
   - Fast: borrow, no needless allocation, ASCII fast paths, `with_capacity`.
   - Keep the function and module names. Run `cargo check` in the crate.
4. **Verify**: `fast_verify` builds in release mode and replays every recorded call, plus
   derived edge cases and shrunk inputs. On mismatch, read the failing call, fix, verify
   again. Never edit the recordings.
5. **Enable**: `fast_enable`, then `fast_bench` and `fast_report`. `fast_disable` switches
   the function back; `FAST_ANYTHING_DISABLE=1` runs the original for one run.
6. Tell the user the end-to-end speedup, the per-call speedup, and how to undo.

## Don't

- Don't edit the original function by hand: the block fast-anything appends and removes does
  the swap, and a manual edit breaks `disable`.
- Don't enable a port that has not passed verify.
- Don't start another agent (`--agent`): you are the agent that writes the port.
- If a function is marked ✗ (I/O, randomness, globals, lives in the entry script), say why it
  cannot be ported yet, and suggest moving pure logic into its own module when that is the
  blocker.

## In this repository

- Porting needs Rust 1.88 or newer on `PATH`; `scan` does not. `fast-anything doctor` reports
  whether the machine can scan, can port, and which app runtimes are present.
- Each port's crate compiles into the project's shared `.fast/target` cache; `fast-anything
  clean` removes it, and `clean --all` also switches every port back.
- `fast_enable` rewrites call sites in the project's sources, and scanning and recording
  execute the project's code with your environment. Work on a copy when the project is not a
  throwaway, review the diff `enable` leaves, and keep production credentials out of the
  environment.
- The enabled block is marked, and `fast_disable` removes it byte for byte; if the user asks
  why a function got slower, check `fast_bench`'s per-call result before blaming the port — a
  right port that is slower per call is not enabled.

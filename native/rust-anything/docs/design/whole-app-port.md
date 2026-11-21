# Design: whole app to a pure Rust app

Status: built, all six milestones (CLIs, HTTP servers and libraries, in all six languages).

## Goal

rust-anything ports a whole app (TypeScript/JavaScript, Python, C, Go, Ruby or Java) to a
standalone Rust app, with the original kept only as reference. The user's coding agent writes the
Rust; rust-anything decides the order, scaffolds, and proves at every step that the app still
behaves the same. All six languages ship in the first version.

The workflow the agent follows is in `skills/rust-anything/whole-app.md`. The evidence behind it
is in `docs/research/whole-app-port.md`. This document is about what the tool needs so the agent
can follow that workflow without hand-written scripts.

## What already exists and stays

`analyze` (units, waves, risks, dependencies), `bridge`, `record`, `port.rs`, `build`, `diff`,
`bench`, `shadow`, `gate`, `profile`, and the MCP server. Every unit still goes through them. They
prove a unit equals the original at the function level.

## What is missing

1. **App-level proof.** Today's proof stops at a unit's functions. A whole port needs proof that
   the whole program behaves the same: what goes in and out of the process, or over HTTP.
2. **A Rust entry point early.** Today the original calls Rust. A whole port needs the reverse:
   Rust owns `main` and calls the original for what is not ported yet (Turborepo's Rust shim,
   Mercurial's `rhg` fallback).
3. **The lift.** Nothing turns unit crates plus bindings into one pure Rust binary.
4. **Whole-app planning.** `analyze` ranks units for a first port; it does not find entry points,
   map every dependency to a crate for every language, or say what a `keep` unit becomes.

The app-level parts (1 to 3) treat the app as a black box, so they work the same for all six
languages. That is what makes "all six at once" realistic: the per-language work is already in
`bridge`, `record` and `diff`.

## New commands

Each is also an MCP tool with the same name.

### `rust-anything app record <repo>`

Runs the original on app-level cases and stores them in `rust/app-cases/`.

- **CLI cases**: argv, stdin, env and input files in; stdout, stderr, exit code and files written
  out. One case per file, in a format a person can read and edit (trycmd-style).
- **HTTP cases**: the agent or user gives a start command and a port; requests come from a list
  (method, path, headers, body) and the responses are stored.
- Inputs from the app's tests and examples, `--inputs` files, and a built-in set of semantic traps
  per language (string length and offsets, division and modulo signs, overflow, float printing,
  map order, regex features).
- Runs the original **twice**. Fields that differ between the two runs are marked as noise and
  normalized (Diffy's idea). The user can add normalizers by name (timestamps, uuids, key order).

### `rust-anything app replay <repo> [--build <command>] [--case <name>]`

Runs any build (the original, the front door, the final binary) against `rust/app-cases/` and
prints `passed/total` and the **fallback rate** (cases still answered by the original). It writes a
report with a diff per failing case. Exit code non-zero on any mismatch, so CI can run it.
`--case` reruns one case (for a library, one unit): the fast loop for fixing one mismatch.

### `rust-anything app front-door <repo>`

Scaffolds `rust/app`, the Rust binary that owns the entry point from day one:

- **CLI**: a `clap` parser for the original's commands and flags (taken from `analyze` and the
  recorded cases), where every command starts as "run the original with the same argv, stdin and
  env and pass its output and exit code through".
- **HTTP**: an `axum` server where every route starts as a proxy to the original server.
- A **kill switch** per unit: `RUST_ANYTHING_UNIT_<NAME>=original` sends that unit back to the
  original until the lift.
- Every fallback is counted, so `replay` can report the fallback rate.

### `rust-anything app wire <repo> <unit>`

After a unit passes `gate`, connects its `port.rs` code to the front door: the commands or routes
it covers stop falling back. The agent writes the glue; this command generates the call sites and
the kill switch and runs `replay`.

### `rust-anything app lift <repo>`

Only when the fallback rate is zero and every unit passes `gate`:

- removes the fallback path and the kill switches,
- turns each unit crate into a plain library dependency of `rust/app` (drops `cdylib`, bindings,
  generated wrappers),
- runs the lift checklist: embedded assets, every target in `DECISIONS.md` builds, `replay` in CI,
- runs `gate` on everything and `replay` on the pure binary.

## Changes to existing commands

- `analyze --whole-app`: entry points (bin scripts, `main`, server start), every external
  dependency mapped to a Rust crate with a fit for all six languages (today only the npm mapping is
  complete), `keep` units listed as decisions for the grilling round, and a wave plan that groups
  tangled units into one piece.
- `gate`: flags `unwrap`, `expect` and `panic!` added in `port.rs` where the original handled an
  error, and fails when a recorded error case is missing.
- `record`: adds the semantic-trap inputs to the generated cases.

## Files the tool writes in the user's repo

```
rust/
  MAP.md              goal, ADR index, every unit with its state and claim, open questions
  DECISIONS.md        one ADR per decision from the grilling round (the agent writes it)
  GLOSSARY.md         the original's terms and their Rust names
  HANDOFF.md          where the port stands, for the next session (at each checkpoint)
  cutover.sh          interactive steps only the user can do at cutover
  research/           one cited file per dependency or risk (the agent writes them)
  app-cases/          recorded app-level cases, normalizers, last replay report
  app/                the front door, later the whole Rust app
  <unit crates>/      as today
```

## Milestones

1. `app record` and `app replay` for CLIs, all six languages (black box, so one implementation).
2. `app front-door` and `app wire` for CLIs, with fallback rate and kill switches.
3. `app lift` for CLIs. A CLI app can now be ported end to end.
4. HTTP: `app record` and `app replay` for servers, the axum proxy front door.
5. `analyze --whole-app` and the `gate` and `record` changes.
6. Libraries (no `main`): front door is the library's public API in Rust, and the original's
   package re-exports it until the lift.

Each milestone ships as a release, is proved on the repository's fixtures in all six languages,
and on one real open-source CLI per language before it is called done.

## Product changes

The pitch changes from "make the slow part fast, without rewriting the app" to rewriting the app
in Rust, one piece at a time, with proof. That touches the GitHub description, the README intro,
the hero image tagline, `Cargo.toml`, the npm description, and `--help` (which still says
"TypeScript/JavaScript to Rust" only). Making the slow part fast stays as a smaller use of the same
tool.

## Decisions (2026-11-16)

1. **CLIs first**, then servers: the milestone order above stands.
2. **Case format: trycmd-compatible files**, readable and editable, and understood by existing Rust
   tooling.
3. **Front door HTTP framework: axum** by default; the user can choose another in the grilling
   round.
4. **New pitch drafted now, published with milestone 1**, so the README never promises what does
   not exist yet.

Taken while building, without a question to the user (the conventional option, with the reason):

5. **The original's command goes after `--`** (`app record . -- python3 cli.py`), as `profile`
   already takes its workload; it is saved with the cases, so later runs need no arguments.
6. **`--inputs` is JSONL** (`{"name", "args", "stdin", "env", "files"}`), like `record --inputs`.
7. **Settings live in `rust/app-cases/rust-anything.json`**, not a TOML file, so a trycmd
   `*.toml` glob never reads them as a case.
8. **Re-recording overwrites cases of the same name and deletes none**: hand-written cases stay.
9. **The fallback hook is the file named by `RUST_ANYTHING_FALLBACK_LOG`**: one line per
   fallback, created empty to report none. Any build can report through it, not only ours.
10. **The front door's clap parser only routes**: the original always gets the raw argv, and
    anything clap rejects falls back, so the parser never has to match the original exactly.
11. **Argv that names no command routes to the app's own name**, so a unit can take the usage
    and unknown-command paths; without it no CLI reaches fallback rate 0.
12. **`wire` takes `--command` (repeatable), runs `gate` unless `--skip-gate`, and writes the glue
    file once**; only `src/units/mod.rs` is regenerated, so the agent's code is never overwritten.
13. **`front-door` and `wire` build and replay by default** (`--no-check` to skip): the design asks
    for replay after each step.
14. **Lift is all or nothing**: it works on a copy and writes back only when every check passes,
    so a failed lift leaves the repository as it was.
15. **Replay in CI is a trycmd test in `rust/app`** (`tests/app_cases.rs`, run by `cargo test`):
    CI then needs only cargo, and decision 2 chose trycmd files for this. No workflow file is
    written; where CI runs stays the user's.
16. **Lift removes generated wrappers only from crates `bridge` made** (they have its manifest);
    any other crate only loses `cdylib` and binding dependencies.
17. **Asset findings are warnings, cross targets are checked, not linked**: neither can be judged
    for certain locally, and an uninstalled target is skipped with the command that adds it.
18. **HTTP cases live in `rust/app-cases/http/`**: `<case>.toml` with `[request]` and `[response]`
    (status, headers), `<case>.body` for the response body with trycmd's `[..]` and `...`. A
    subfolder keeps them out of trycmd's `*.toml` glob; TOML matches the CLI cases.
19. **A server gets its port from `PORT` and `{port}`**, a free one per start, so runs and tests
    never collide; `--port` is for originals that cannot be told. The front door needs the first
    kind, since it runs the original next to itself.
20. **Requests go in name order to one fresh server per run**, so state between requests (create,
    then read) is recorded and replayed the same way.
21. **Per-response headers are not compared** (date, server, connection, content-length, etag,
    last-modified, x-powered-by and the like): they differ between any two servers. The rest are,
    and the user deletes the ones that do not matter.
22. **Examples are `curl` lines to localhost** in the docs, the form servers document themselves in.
23. **rust-anything speaks HTTP/1.1 itself** (no client crate): requests are small and local, and
    every dependency is a cost for all users.
24. **The front door uses axum 0.8, tokio and reqwest without default features** (plain HTTP to
    the original, no TLS); reqwest does not follow redirects there, so the original's answer
    passes through as it is.
25. **`wire --route` takes axum routes**; `*` is the router's fallback, and a wired path sends
    other methods back to the original (front door) or to `*` (lifted).
26. **Lift of a server has no trycmd test**: trycmd runs command lines only, so the checklist
    asks for `app replay --build` in CI instead of writing a client into the app.
27. **`--whole-app` adds to the analyze@1 JSON** (a `whole_app` object) instead of a new schema:
    existing readers keep working.
28. **Shape is server, else CLI, else library**: a server that also has a `main` is ported as a
    server; the library shape waits for milestone 6.
29. **Every dependency means imported or declared**: a declared one no unit imports is listed with
    `used_by: 0` (often an import name that differs from the package name, or a dead dependency).
30. **Decisions are generated only where a fact cannot decide**: the shape, each keep and blocked
    unit, each dependency with fit `none`, `unknown` or `partial`, and each tangled piece; each
    comes with the recommended answer, as the grilling round asks.
31. **"The original handled an error" means it fails on purpose**: `throw` (TS, Java), `raise`
    (Python, Ruby), an `error` result (Go), found in the export's body; C has no such signal and
    relies on recorded results. A function with a recorded failing case counts too.
32. **A panic is allowed with `// INFALLIBLE: <why>`**, on the line or just above, as `unsafe` is
    with `// SAFETY:`: the convention reviewers already know.
33. **Traps go one parameter at a time**, the others as in the first generated case, so a
    mismatch points at one value; structured parameters keep their generated, well-typed values.
34. **A library's front door is a Rust crate re-exporting each wired unit's port as a module**
    (`pub use textkit_rs::port as textkit;`), not a flat namespace: units may export the same name,
    and glob re-exports of both would be ambiguous.
35. **The original package reaches Rust through the units' shadows**, which already keep the
    original API in every language; `wire` writes the shadow and the user or agent points the
    package at it. rust-anything does not edit the original's own files.
36. **Shadows honor `RUST_ANYTHING_UNIT_<NAME>=original`** before `RUST_ANYTHING_MODE`, so a library
    unit has the same kill switch as a CLI or server unit. C's shadow is chosen at build time and
    keeps its flags.
37. **A library's fallback rate counts public exports**, not calls: each export not yet answered in
    Rust (an unwired unit, or one `bridge` keeps in the original language) is one fallback.
38. **A library's replay is `diff` on every wired unit**: the recorded cases through the bindings.
    After the lift the bindings are gone; the lift does not change any `port.rs`, so those results
    still describe it, and the checklist asks for Rust tests in CI.
39. **The public API is every exported function and class of every non-keep unit**, found the way
    `bridge` finds them; `--library` treats a repository with a demo `main` as a library.
40. **A PyO3 unit gets a `.cargo/config.toml` in `rust/app`** that lets macOS link its `cdylib` with
    Python's symbols left to load time (PyO3's documented setting); the lift removes it with the
    bindings.
41. **The port is carried across sessions by `rust/MAP.md`** (after Matt Pocock's `wayfinder`
    skill): a session claims one unit before it starts, so parallel sessions never take the same
    one. The agent writes it; the tool does not parse it.
42. **`DECISIONS.md` holds one numbered ADR per decision** (Status, Context, Decision,
    Consequences), superseded rather than edited, and `GLOSSARY.md` maps the original's terms to
    Rust names (after `grill-with-docs` and `domain-modeling`). One file keeps the targets `lift`
    reads where they were.
43. **A red replay is fixed one case at a time** with `app replay --case <name>` (after
    `diagnosing-bugs`): a fast, repeatable failing test before any fix.
44. **Each checkpoint writes `rust/HANDOFF.md`** (after `handoff`): pointers, not copies.
45. **A wave can be ported in parallel by subagents** (after `chief-of-staff`), wired one at a time
    by the coordinator; every mistake becomes an ADR, glossary entry or test.
46. **Cutover steps only the user can do go in `rust/cutover.sh`** (after `wizard`), an interactive
    script with a way back at each step.
47. **Floats can match within a tolerance the user chooses**, saved with the recording (the
    crate manifest, or `rust/app-cases`), so `diff`, `gate`, `replay` and the lift all use the same
    bar. Relative to the larger value, absolute below 1; integers, text and `3` against `3.0` stay
    exact, so the tolerance forgives rounding, never formatting. Output lines with `[..]` stay exact.
48. **Already compiled Python (numba, Cython, JAX) is flagged, not refused**: a risk in `analyze`
    and advice in `profile`, from the decorators in the source, so it works without numba
    installed. Porting it still makes sense for a pure Rust app, just not for speed.
49. **Noise is classified, not just wildcarded** (from a reader's review): same lines in another
    order are compared as a multiset, so a port that produces different lines still fails; only
    changed values become `[..]`, and a changed line count (`...`) is reported as the riskiest
    kind. Every kind goes to the user, since hash order and concurrency are what really change.
50. **Regex traps go where the regexes are**: per file, by a text check for each language's regex
    API, so every function in that file gets them; cheaper than per-function analysis, and extra
    cases cost little.
51. **The pitch claims what is checked**: "checked against the original's recorded behavior",
    with a README section on what recording cannot catch.

## Draft pitch

- One line (GitHub description, npm, `Cargo.toml`): "Rewrite your app in Rust, one piece at a
  time, checked against the original's recorded behavior at every step. TypeScript, Python, C, Go, Ruby, Java."
- Hero tagline: "Rewrite anything in Rust. Keep it working the whole way."
- Sub-line: "rust-anything ports your app piece by piece, checks every step against the original,
  and hands you a pure Rust app."
- `--help`: "Rewrite TypeScript/JavaScript, Python, C, Go, Ruby or Java apps in Rust, one verified
  piece at a time".

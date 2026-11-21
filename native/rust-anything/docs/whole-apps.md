# Port a whole app

Port a whole CLI, server or library, unit by unit, until it is pure Rust. Back to the [README](../README.md).

## Port a whole app

The goal is a pure Rust app that behaves like the original, with the original kept as reference.
Your agent writes the Rust; rust-anything orders the work, scaffolds it, and proves every step.
The [whole-app skill](../skills/rust-anything/whole-app.md) walks the agent through it, starting
with the decisions only you can make:

- **Grilling**: the agent asks in rounds, each question with its recommended answer, until every
  decision the port depends on is settled, and writes each one as an ADR in `rust/DECISIONS.md`,
  with `rust/GLOSSARY.md` mapping the original's terms to Rust names.
- **Research**: background agents read each dependency's primary sources and write
  `rust/research/<topic>.md`, with every semantic trap they found.
- **Across sessions**: `rust/MAP.md` lists every unit with its state; a session claims one unit
  before it starts, and each checkpoint leaves a `rust/HANDOFF.md` for the next session. Units in
  one wave can be ported in parallel by subagents.
- **Cutover**: the agent does what it can and writes `rust/cutover.sh`, an interactive script for
  the steps only you can do (deploys, DNS, secrets), with a way back at each step.

Grilling, research and these practices are adapted from
[Matt Pocock's skills](https://github.com/mattpocock/skills).

**Checking a rewrite in any language.** `app record` and `app replay` only run programs: the
recording is what the original did from the outside (arguments, stdin, files, HTTP), and
`app replay --build "<command>"` runs any build against it. A rewrite in Go, or by hand, is
checked the same way as the Rust front door:

```sh
rust-anything app record . -- java -jar app.jar     # what the original does
rust-anything app replay . --build "./app-go"       # does the rewrite do the same?
```

The unit tools (`bridge`, `record`, `diff`, `gate`) and `lift` are Rust-specific.

### Side effects: email, webhooks and databases

A rewrite can answer with the same JSON and still send an email twice or write different rows.
`app record` can record what leaves the process too, per case, and `app replay` compares it:

```sh
rust-anything app record . --effects http,smtp --effect-env WEBHOOK_URL=http-url \
  --db-reset "sh scripts/reset-db.sh" --db-dump "sqlite3 app.db .dump" -- python3 app.py
```

- **`--effects http`** runs a local HTTP sink that answers every request with `200 {}`, set as
  `HTTP_PROXY`, `http_proxy` and `RUST_ANYTHING_HTTP_SINK`. Each request is recorded as method,
  URL, content type and body. HTTPS through the proxy is recorded as `CONNECT host:443` and refused,
  since the sink cannot read it: point HTTPS settings at the sink with `--effect-env` instead.
- **`--effects smtp`** runs a local mail server (`SMTP_HOST`, `SMTP_PORT`, `RUST_ANYTHING_SMTP_SINK`)
  that accepts everything, and records sender, recipients and the message.
- **`--effect-env VAR=KIND`** points one of the app's own settings at a sink: `http-url`,
  `smtp-host`, `smtp-port` or `smtp-url`. Most apps read their webhook URL or mail host from a
  setting like this.
- **`--db-reset` and `--db-dump`** are shell commands: reset puts the database in a known state
  before each case (each start, for servers), and dump prints its state after each case (each
  request, for servers). Cases then run one at a time.

What reached the sinks is stored as `<case>.effects` next to the case and compared like stdout:
the count and order of effects matter, values that differed between the original's two runs
(dates, ids) become wildcards, and effects the original sends in varying order are compared in any
order. The settings are saved with the cases, so replay starts the same sinks for every build.

Plan first: `rust-anything analyze . --whole-app` adds the app's shape (CLI, server or library)
and entry points, every external dependency (imported or declared) with its Rust crate and fit,
the port order as pieces (units that import each other move together), and numbered decisions
with a recommended answer: keep units, blocked units, dependencies with no or a partial crate.

Then prove the whole program from the outside. `record` and `diff` prove one unit; `app record`
and `app replay` prove the app, for CLIs and servers in any of the six languages (the app is a
black box):

```sh
rust-anything app record . -- python3 cli.py          # the command that runs the original
rust-anything app replay .                            # the original must pass 100%
rust-anything app replay . --build "target/release/mytool"
```

- `app record` runs the original on the app's own documented examples (`$ mytool ...` lines in
  its Markdown and shell files, with the files they name), an `--inputs` JSONL file
  (`{"name", "args", "stdin", "env", "files"}`), and semantic traps derived from those cases
  (non-ASCII and emoji strings, negative division and modulo, overflow, large and tiny floats,
  map order, missing files). Each case runs twice; what differs between the two runs becomes a
  `[..]` wildcard. `--normalize timestamps|uuids|key-order` or `--normalize name=regex` adds
  normalizers, saved with the cases.
- Cases are [trycmd](https://docs.rs/trycmd) files in `rust/app-cases/`: `<case>.toml` (argv, env,
  exit code), `.stdin`, `.stdout`, `.stderr`, `.in/` (files in) and `.out/` (files written). Read
  and edit them by hand; a Rust test can run them with `trycmd` once the port has a binary.
- `app replay` runs any build against the cases, prints `passed/total`, writes
  `rust/app-cases/replay-report.md` with a diff per failing case, and exits non-zero on any
  mismatch. A build that appends to the file named by `RUST_ANYTHING_FALLBACK_LOG` (one line per
  fallback to the original), as the front door below does, also gets its fallback rate reported.
- `app replay --case <name>` reruns one case by the name the report gives it: the fast,
  repeatable loop for fixing a mismatch before running everything again.
- The report has no timings or temporary paths: two replays of the same build write the same
  file. Commit it and run `app replay` then `git diff --exit-code rust/app-cases/replay-report.md`
  in CI, and a half-ported app fails CI on any change in what passes, better or worse, until the
  report is updated with it.

Then move the entry point to Rust and hand it commands one unit at a time:

```sh
rust-anything app front-door .                            # rust/app: every command runs the original
rust-anything app wire . textkit --command upper          # after textkit-rs passes gate
# write the glue in rust/app/src/units/textkit.rs, then:
rust-anything app wire . textkit                          # 100% again, lower fallback rate
```

- `app front-door` scaffolds `rust/app`, a Rust binary named after the app: a clap parser for the
  commands and flags in the recorded cases, where every command runs the original with the same
  argv, stdin and environment and passes its output and exit code through. Each fallback is
  counted, so replaying the fresh front door gives 100% passed at 100% fallback (it builds and
  replays to show it). It never overwrites `rust/app`: the code is yours from then on.
- `app wire <repo> <unit> --command <name>` runs `gate` on the unit crate (`--skip-gate` if it just
  passed), adds it to `rust/app` as a library, writes `src/units/<unit>.rs` once for you or your
  agent to fill in (parse the arguments, call `<crate>::port`, print what the original prints),
  routes the commands to it, then builds and replays. `RUST_ANYTHING_UNIT_<NAME>=original` sends a
  wired unit back to the original until the lift.

Wire the app's own name as a command (`--command mytool`) for argv that names no command: no
arguments, unknown commands, usage errors.

When replay passes with fallback rate 0 and every unit passes gate, make it a pure Rust app:

```sh
rust-anything app lift .
```

- `app lift` refuses until both hold, and says what blocks it: the cases still falling back,
  glue still a stub, units failing gate.
- On a copy first, it removes the fallback path and the kill switches, turns each unit crate into
  a plain library of `rust/app` (no `cdylib`, bindings or generated wrappers), and adds
  `rust/app/tests/app_cases.rs`, which runs `rust/app-cases` with trycmd on `cargo test`, so CI
  needs nothing but cargo.
- Then it runs the lift checklist: fmt, clippy `-D warnings` and tests on every unit and on
  `rust/app`, every target named in `rust/DECISIONS.md` built or checked where it is installed,
  files read at run time and asset folders not embedded, and replay on the pure binary. Only when
  all of it passes are the changes written back. The original is never touched.

### Servers

The same commands prove HTTP servers. `--http` says the command starts a server; it takes its
port from `PORT` or `{port}` in the command (`--port` for one that always listens on the same
port):

```sh
rust-anything app record . --http -- node server.js
rust-anything app replay .
rust-anything app front-door .                                    # axum: every route proxied
rust-anything app wire . api --route 'GET /users/{id}' --route '*'
rust-anything app lift .
```

- Requests come from `curl` lines in the app's docs (`curl -X POST -d ... http://localhost:8080/x`),
  `--inputs` (`{"name", "method", "path", "headers", "body"}`) and traps: path segments and query
  values swapped for the same traps as CLI arguments, bodies for text and JSON traps, and an
  unknown route. Each run starts the server fresh and sends the requests in name order, so state
  between requests is kept.
- A server that keeps state in files gets a fresh copy for every start: `--fresh db.json` (a file
  or folder, repeatable) is snapshotted into `http/files/` and copied into a new working
  directory each time the original, the front door or a build starts; `--arg db.json` is an
  argument every start gets after its command, so the Rust binary is started as the original is.
  `--pace 50` waits 50 ms after each response, for work a server does after answering (a file
  watcher reloading what it just wrote).
- Cases are `rust/app-cases/http/<case>.toml` (the request, the status and the headers that matter)
  and `<case>.body` (the response body, with `[..]` where two runs differed). Date, server and
  other per-response headers are left out; delete any header line you do not care about.
- The front door is an axum server that starts the original on a port of its own and proxies
  every route no unit answers. `wire --route` takes axum routes: `METHOD /path/{param}`, `/path`
  for any method, `*` for everything else; the glue gets the route, its parameters and the
  request. `lift` removes the proxy; HTTP cases then run in CI with `app replay`.

### Libraries

A library has no process to replay: its public API is the front door, and each unit's recorded
cases are the proof.

```sh
rust-anything app front-door .        # rust/app: a Rust crate with the public API
# each unit through the loop: bridge, record, port.rs, build, diff, gate; then:
rust-anything app wire . textkit-rs   # re-exported from rust/app; the original imports its shadow
rust-anything app replay .            # every wired unit's cases; fallback = exports still in the original
rust-anything app lift .
```

- `app front-door` finds the public API (every unit's exports) when the repository has no `main`
  and no server (`--library` otherwise), and scaffolds `rust/app` as a library crate. Each wired
  unit becomes a module of it: `pub use textkit_rs::port as textkit;`.
- `app wire` takes a crate made by `bridge`, runs `gate`, and writes the unit's shadow drop-in:
  the original package imports it in place of the unit, so its callers keep the original API and
  reach the Rust. `RUST_ANYTHING_UNIT_<NAME>=original` sends the unit back (C: build without the
  shadow's flags).
- `app replay` runs every wired unit's recorded cases through its bindings. The fallback rate is
  the share of public exports Rust does not answer yet: unwired units, and exports `bridge` keeps
  in the original language.
- `app lift` needs fallback 0 and every unit through `gate`; it strips the bindings, wrappers and
  shadows from the units and leaves `rust/app` as one pure Rust library. It does not touch any
  `port.rs`, so the cases that matched through the bindings still describe it; the checklist
  warns about files of the original that still import the shadows.

All five are MCP tools too: `app_record`, `app_replay`, `app_front_door`, `app_wire` and
`app_lift`.

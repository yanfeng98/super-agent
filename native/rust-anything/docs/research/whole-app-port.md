# Porting a whole application to pure Rust: research notes

Question: how should rust-anything help a coding agent port a whole application (TypeScript/JavaScript,
Python, C, Go, Ruby, Java) to a standalone Rust binary, piece by piece, while proving behavior did not
change, with the original kept only as a reference at the end?

Method: primary sources only (project blogs and postmortems by the people who did the work, official
docs, papers, specs). Claims marked **unverified** could not be traced to a primary source. Small
semantic probes in section 4 were run locally (rustc 1.99.0, Node, CPython 3, Go) on 2026-11-16.

## Summary: conclusions for the tool's design

1. **Every successful whole-codebase port in this survey was incremental and kept the program runnable
   at every step.** fish (C++ to Rust) moved "component by component" so the end-to-end suite could
   keep running ([fish blog](https://fishshell.com/blog/rustport/)); Turborepo (Go to Rust) rejected a
   rewrite because it was not confident a rewrite would keep the same behavior
   ([Vercel](https://vercel.com/blog/how-we-continued-porting-turborepo-to-rust)); librsvg kept its
   public C API through four years of porting
   ([2.46.0 announcement](https://lists.gnome.org/archives/ftp-release-list/2019-September/msg00117.html)).
   The tool should treat "app still builds and passes its end-to-end suite" as a gate after every unit.
2. **Move the entry point to Rust early, not last.** Turborepo's first Rust code was a shim that owned
   `main` and CLI parsing (clap) and handed off to the Go code
   ([Vercel](https://vercel.com/blog/how-turborepo-is-porting-from-go-to-rust)). Mercurial's `rhg`
   is a Rust binary that silently falls back to Python `hg` for anything unsupported
   ([D10093](https://phab.mercurial-scm.org/D10093)). With a Rust front door, the final "lift" is
   deleting the fallback rather than a risky flip. rust-anything needs a second mode besides today's
   "host calls Rust": "Rust calls host for what is not ported yet".
3. **The behavior oracle must move up to the process boundary.** Unit-level recordings do not cover
   config load order, CLI parsing edge cases, globbing or log output, which Turborepo names as the
   behaviors that had to be pinned down
   ([Vercel](https://vercel.com/blog/how-turborepo-is-porting-from-go-to-rust)). Turborepo ran both
   code paths in CI and failed the build on any hash divergence, and kept the Rust path behind a flag
   until output matched "log line for log line"
   ([Vercel](https://vercel.com/blog/finishing-turborepos-migration-from-go-to-rust)). Add app-level
   record/replay: CLI golden cases (argv, stdin, env, files in, stdout, stderr, exit code, files out),
   and HTTP request/response replay with noise cancellation.
4. **Run two copies of the original to separate real diffs from noise.** Twitter's Diffy sends each
   request to the candidate and to two instances of the known-good code; disagreement between the two
   known-good instances is treated as non-determinism and filtered out
   ([Diffy](https://github.com/twitter-archive/diffy)). Whole apps have timestamps, ids and ordering
   noise that unit recording never sees.
5. **Pick one schema-based wire format for the transition boundary on day one.** Turborepo started with
   JSON, moved to protobuf, then Cap'n Proto so hashes were identical across languages, and lists "a
   single serialization format from the start" among its lessons
   ([Vercel](https://vercel.com/blog/finishing-turborepos-migration-from-go-to-rust)). Prisma found
   that serializing data from JS to Rust and back added overhead that negated the benefit
   ([Prisma](https://www.prisma.io/blog/from-rust-to-typescript-a-new-chapter-for-prisma-orm)). Chatty
   fine-grained boundaries are a cost during a whole-app port, so cut at subsystem seams.
6. **Port first, idiomatize second.** fish kept the C++ structure to make comparison easy
   ([fish](https://fishshell.com/blog/rustport/)); YJIT was translated "more or less directly"
   ([Shopify](https://shopify.engineering/porting-yjit-ruby-compiler-to-rust)); c2rust's translator
   targets functional identity and leaves safe idioms to a separate refactoring step
   ([c2rust manual](https://c2rust.com/manual/)). The plan should have two passes per unit, with the
   behavior gate between them.
7. **Order by dependency, leaves first, but take entangled cores in big bites.** LLM systems that work
   at project scale translate in dependency order with a check per step (Syzygy,
   [arXiv:2412.14234](https://arxiv.org/abs/2412.14234)). fish started with self-contained builtins but
   ported the 13k-line reader in large chunks to avoid throwaway FFI glue
   ([fish](https://fishshell.com/blog/rustport/)).
8. **Agents need small units, a type-mapping table and an executable oracle; repo-level one-shot does
   not work.** Without predefined feature-mapping rules, Oxidizer's Go-to-Rust translation failed on
   all benchmarks; with them it reached 73% I/O-equivalent functions
   ([arXiv:2412.08035](https://arxiv.org/abs/2412.08035)). Repository-level benchmarks still top out
   near 51.5% Pass@1 ([arXiv:2411.13990](https://arxiv.org/abs/2411.13990)). Regenerating from scratch
   beat feeding fuzzer counterexamples back
   ([arXiv:2405.11514](https://arxiv.org/html/2405.11514v3)).
9. **Generate inputs that target known semantic traps.** UTF-16 vs UTF-8 lengths, integer overflow,
   floor vs truncating division, float formatting, map ordering and regex features all differ (section
   4, probes run locally). `record` should seed these per language pair, not rely on random inputs.
10. **Every ported unit needs a kill switch until the end.** Deliveroo kept each Ruby class alive behind
    feature flags with a master switch, and ran one shared RSpec suite against both implementations
    ([Deliveroo](https://deliveroo.engineering/2019/02/14/moving-from-ruby-to-rust.html)); Turborepo used
    Go build tags to toggle implementations and shipped via a 72-hour internal canary
    ([Vercel](https://vercel.com/blog/how-we-continued-porting-turborepo-to-rust),
    [Vercel](https://vercel.com/blog/finishing-turborepos-migration-from-go-to-rust)).
11. **Error paths are where ports break.** fish's early translations used `unwrap()`/asserts where C++
    handled errors differently, causing crashes ([fish](https://fishshell.com/blog/rustport/)). Gate
    should flag new `unwrap`/`expect`/`panic!` in ported code and record failing inputs (exit codes,
    stderr, exception types) as first-class cases.
12. **The FFI stage is itself a source of build and platform bugs that disappear only at the lift.**
    Turborepo hit MinGW vs MSVC and Go-runtime segfaults under static musl, and had to ship two binaries
    ([Vercel](https://vercel.com/blog/how-turborepo-is-porting-from-go-to-rust)); fish lost Cygwin
    support ([fish](https://fishshell.com/blog/rustport/)). Plan the lift as a milestone with its own
    checklist (assets, packaging, targets), not as an afterthought.

## 1. Real whole-codebase ports and how they were sequenced

### fish shell (C++ to Rust, fish 4.0)

Source: [fish blog, "Fish 4.0: The Fish Of Theseus"](https://fishshell.com/blog/rustport/).

- Incremental. Component by component, keeping fish working so the end-to-end suite could run. Builtins
  first (small, self-contained); the reader (about 13k lines of Rust) later in bigger chunks to avoid
  throwaway FFI code.
- FFI via autocxx/cxx. Per call site they chose: add glue so Rust calls C++ and port the caller later,
  move the callee, or keep both alive (C++ getopt "stuck around until it had no more callers").
  Problems: autocxx could not handle some constructs (they forked it for wstring), extra copies through
  `unique_ptr` hurt performance temporarily, generated code slowed rust-analyzer.
- Timeline: initial PR merged February 2023, last C++ removed January 2024, first beta December 2024.
  About 57k lines of C++ became about 75k lines of Rust, plus about 400 lines of C.
- Equivalence: mainly the end-to-end suite (littlecheck scripts, pexpect interactive tests), since unit
  tests were thin. Testing the "second 90%" found many bugs.
- What went wrong: `unwrap()` where C++ had other error handling; missed subtleties; portability gaps
  (hand-maintained `cfg` lists, they ported musl's printf for format strings); Cygwin dropped.
- Lift: assets (functions, completions) embedded in the binary; musl static build. CMake was not fully
  removed (still installs docs and assets).

### Turborepo (Go to Rust), the most detailed whole-app playbook found

Sources: [part 1](https://vercel.com/blog/how-turborepo-is-porting-from-go-to-rust),
[part 2](https://vercel.com/blog/how-we-continued-porting-turborepo-to-rust),
[part 3](https://vercel.com/blog/finishing-turborepos-migration-from-go-to-rust).

- Explicitly a port, not a rewrite: a rewrite would stall features, risk edge cases, and they were not
  confident it would keep behavior.
- Stage 1, Rust shim: Rust owns the binary and CLI parsing (clap); Go compiled as a C static library via
  cgo; arguments passed as JSON. Windows (MinGW vs MSVC) and Alpine (Go cannot be a static C library
  under musl) forced a two-binary design: Rust binary execs the Go binary with JSON args. Shipped 1.7.0.
- Stage 2, "Go sandwich": Rust binary calls Go, which calls Rust libraries (`turborepo-ffi`) via cgo,
  with protobuf types. Pieces sized to one PR, Go build tags toggle old/new. Outgrown because async,
  multi-threaded code across languages "wasn't going to be viable".
- Stage 3, "run outline": an all-Rust `run` with most parts stubbed, behind a flag, filled in piece by
  piece. CI ran both paths and compared hashes; mismatches failed the build and found real bugs
  (`null` in hashing, change detection, `--filter`). Integration tests ran on the Rust path behind
  `--experimental-rust-codepath` until output matched log line for log line.
- Cut-over: canary in Vercel's monorepo, 72 hours with no errors, shipped 1.11. About 70k lines of Go
  over 15 months.
- Lessons stated by the authors: one serialization format from the start, ship each strategy sooner,
  clean up the Go code before porting, document edge cases (globbing, hashing).

### Mercurial `rhg` (Python to Rust, front-door pattern)

Source: [Phabricator D10093](https://phab.mercurial-scm.org/D10093).

- `rhg` is a Rust executable for a subset of `hg`. With `rhg.on-unsupported = fallback`, any unsupported
  command, repo format or enabled extension silently runs `rhg.fallback-executable` (Python `hg`). The
  stated goal: whether Python is used "should be an implementation detail not apparent to users".
- Guards against recursive fallback. Known cost: the Python `hg` must stay installed; embedding Python
  in the binary was named as a possible future fix.
- This is the cleanest documented model for "Rust main, original as fallback" in a CLI.

### librsvg (C to Rust, library with a frozen C API)

Sources: [rustification branch start, Oct 2016](https://mail.gnome.org/archives/commits-list/2016-October/msg08290.html),
[2.46.0 announcement, Sept 2019](https://lists.gnome.org/archives/ftp-release-list/2019-September/msg00117.html).

- Goal from the first commit: rewrite internals in Rust while preserving the public C API. By 2.46.0,
  "All of librsvg.so is now implemented in Rust", except "a very thin wrapper over the public API
  functions". For a library the lift ends with a thin ABI shell kept on purpose.

### YJIT (C to Rust inside CRuby)

Source: [Shopify Engineering](https://shopify.engineering/porting-yjit-ruby-compiler-to-rust).

- Translated more or less directly, porting functions and structs one by one while commenting out C,
  no major architecture change; three months. Rust built as a static library linked into CRuby.
- Pain points: bindgen silently skipped definitions; offline builds; many string types at the C
  boundary; an `Rc`/`RefCell` cycle bug with machine-code references; many `unsafe` blocks.
- The post does not describe a separate equivalence harness (unverified how parity was checked).

### Deliveroo dispatcher (Ruby to Rust, native extension)

Source: [Deliveroo Engineering](https://deliveroo.engineering/2019/02/14/moving-from-ruby-to-rust.html).

- Slowest parts replaced one at a time through a native extension (rutie, after rejecting Helix); an
  external service was rejected because call overhead would cancel the gain.
- Each migrated class kept its Ruby version; feature flags chose per class, plus a master switch. One
  shared RSpec suite ran against both implementations. Near 1:1 port.
- This is a hot-path port (today's rust-anything use case), not a whole-app port.

### Datadog static analyzer (Java to Rust, big-bang)

Source: [Datadog Engineering](https://datadoghq.com/blog/engineering/how-we-migrated-our-static-analyzer-from-java-to-rust).

- Whole analysis infrastructure migrated within about a month, all customers moved to Rust. Acceptance
  criterion: a Rust scan produces the same violations as the Java scan, no performance regression. No
  phased rollout is described. A small, well-bounded service with an output-comparison oracle.

### Discord Read States (Go to Rust)

The primary post ([Discord blog](https://discord.com/blog/why-discord-is-switching-from-go-to-rust))
could not be fetched (TLS error). Details of how it was rolled out are **unverified** here.

### Ports away from Rust and to other targets (for contrast)

- **Prisma** is moving its query engine from Rust to TypeScript plus a small WASM part, incrementally,
  because JS-to-Rust-and-back serialization overhead negated benefits and binary size limited
  deployment ([Prisma](https://www.prisma.io/blog/from-rust-to-typescript-a-new-chapter-for-prisma-orm)).
  Lesson: the transition boundary has a cost that can dominate; do not leave a whole app split forever.
- **Rome/Biome** was a decision to rewrite a JS toolchain in Rust (2021), not a port; the original
  announcement is only available via mirrors (**unverified** wording).
- **TypeScript's native compiler** is a port of the existing codebase to Go, not a rewrite
  ([TypeScript blog](https://devblogs.microsoft.com/typescript/typescript-native-port/)). The post does
  not say how behavior is compared.
- **pyca/cryptography** moved "the entire X.509 layer" to Rust behind its Python API by 36.0.0
  ([release announcement](https://mail.python.org/archives/list/python-announce-list@python.org/thread/27MPXC5FZL34ATBLD2KSSQ5XQAX7XGSC));
  a library keeping a Python shell, not a whole-app port.
- Not ports: uv, Ruff, SWC, Turbopack (written in Rust from the start). No primary-source whole-app
  Java-to-Rust or Ruby-app-to-Rust port with an incremental playbook was found.

### Large-vendor statements on C/C++ to Rust

- **Google/Android:** "Interoperability is the new rewrite". The strategy is new code in memory-safe
  languages plus safe interop, not rewriting old code; memory-safety bugs fell from 76% of Android
  vulnerabilities (2019) to 24% (2024), and vulnerability density decays with code age
  ([Google Security Blog, 2024](https://security.googleblog.com/2024/09/eliminating-memory-safety-vulnerabilities-Android.html?m=1)).
  In 2025: below 20%, about 1000x lower memory-safety vulnerability density for Rust, about 4x lower
  rollback rate and 25% less review time for Rust changes
  ([Google, 2025](https://blog.google/security/rust-in-android-move-fast-fix-things/)).
- **DARPA TRACTOR:** automate C-to-Rust translation that reaches "the same quality and style that a
  skilled Rust developer would produce", combining static and dynamic analysis with LLMs; MIT Lincoln
  Laboratory runs test and evaluation
  ([DARPA](https://www.darpa.mil/research/programs/translating-all-c-to-rust)).
- **c2rust:** produces unsafe Rust that mirrors the C and aims to be "functionally identical"; provides
  clang/rustc plugins to run both binaries and check they behave identically at the level of function
  calls (cross-checking), and a separate refactoring tool toward safer idioms
  ([c2rust manual](https://c2rust.com/manual/)).
- **Microsoft:** a December 2025 LinkedIn post by a Microsoft distinguished engineer set a "1 engineer,
  1 month, 1 million lines" research goal for AI-assisted C/C++-to-Rust translation, later clarified as
  a research project, not a company plan. Only secondary coverage was reachable
  ([The Register](https://www.theregister.com/2025/12/24/microsoft_rust_codebase_migration)); treat as
  **unverified**.

## 2. Behavior-equivalence techniques for whole apps

| Technique | What it proves | Primary source | Fit for rust-anything |
| --- | --- | --- | --- |
| End-to-end suite kept green at every step | No regressions in covered scenarios | [fish](https://fishshell.com/blog/rustport/) | Gate step "app e2e" after each unit |
| Dual-path CI with output/hash comparison | Old and new paths agree on real workloads | [Turborepo](https://vercel.com/blog/finishing-turborepos-migration-from-go-to-rust) | App-level `diff`: run original and Rust binaries on the same cases |
| CLI snapshot tests (argv, stdin, stdout, stderr, exit status, file fixtures in/out) | Process-level behavior | [trycmd](https://docs.rs/trycmd/latest/trycmd/) | Record the original's runs into trycmd-style cases, replay against Rust |
| Conformance suite of the original as oracle | Drop-in compatibility | [uutils](https://github.com/uutils/coreutils) ("Differences with GNU are treated as bugs"; tracks GNU tests passed) | Reuse the source project's own test suite against the Rust binary |
| HTTP traffic replay to candidate plus two known-good copies | Service behavior with noise filtered | [Diffy](https://github.com/twitter-archive/diffy) | `shadow` at app level with a second original to learn noisy fields |
| Function-level cross-checking | Same call sequence/values | [c2rust](https://c2rust.com/manual/) | Today's unit `record`/`diff` |
| Differential fuzzing | I/O equivalence on generated inputs, no tests needed | [arXiv:2405.11514](https://arxiv.org/html/2405.11514v3) | Feed fuzzer inputs to both sides; needs a shared serialization |
| Property-based testing with shrinking | Minimal counterexamples | [proptest](https://docs.rs/proptest/latest/proptest/) | Shrink a diverging recorded case before showing it to the agent |
| Coverage-guided fuzzing | Crash and divergence discovery | [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) (libFuzzer) | Optional deep mode for parsers |

Notes:

- Differential fuzzing across languages fails mostly on plumbing: in FLOURINE, 52.6% of failures were
  the fuzzer being unable to (de)serialize the data types, versus 7.0% compile failures and 40.3% real
  counterexamples ([arXiv:2405.11514](https://arxiv.org/html/2405.11514v3)). A stable, schema-backed
  value encoding for both sides is a prerequisite, not a detail.
- Turborepo got hash equality across languages only after moving to Cap'n Proto
  ([Vercel](https://vercel.com/blog/finishing-turborepos-migration-from-go-to-rust)).

## 3. LLM/agent-assisted translation to Rust

| Work | Setting | Result | What made it work / failure modes |
| --- | --- | --- | --- |
| Oxidizer, PLDI 2025 ([arXiv:2412.08035](https://arxiv.org/abs/2412.08035)) | Whole Go projects to Rust | Up to 6,600 LOC / 369 functions; 73% of functions pass I/O equivalence | Split into fragments; feature-mapping rules (e.g. `(value, error)` to `Result`); type-compatibility checks at signatures. Success drops beyond about 100 lines per unit |
| Syzygy ([arXiv:2412.14234](https://arxiv.org/abs/2412.14234)) | C to safe Rust, Zopfli (~3k LOC, 98 functions) | Test-validated full translation | Translate code and tests together, dynamic analysis, dependency order, check each step |
| FLOURINE ([arXiv:2405.11514](https://arxiv.org/html/2405.11514v3)) | Real C and Go samples, 5 LLMs | Best model 47% | Differential fuzzing as oracle; restart from scratch beat counterexample repair by 7 to 21%; counterexamples "may actually confuse the LLM" |
| ENCRUST ([arXiv:2604.04527](https://arxiv.org/abs/2604.04527)) | C to safe Rust, 15 programs | Fewer unsafe constructs, test vectors still pass | ABI-preserving shim plus safe inner function; failed units roll back automatically; project builds at every intermediate state |
| RustRepoTrans ([arXiv:2411.13990](https://arxiv.org/abs/2411.13990)) | 375 repo-level tasks, C/Java/Python to Rust | Best 51.5% Pass@1, a 22.2-point drop from function-level | Cross-module dependencies and repository context are the hard part |
| Lost in Translation, ICSE 2024 ([arXiv:2308.03109](https://arxiv.org/abs/2308.03109)) | 1,700 samples, several languages | Real-world projects: GPT-4 8.1%, others 0% | 15 bug categories; choosing data types that preserve behavior and finding equivalent APIs are common causes |
| RustAssistant, ICSE 2025 ([Microsoft Research](https://www.microsoft.com/en-us/research/publication/rustassistant-using-llms-to-fix-compilation-errors-in-rust-code/)) | Fixing Rust compile errors | About 74% on real-world errors | Iterate compiler errors with the LLM |

Takeaways for the tool:

- Unit size matters more than model choice: every result that scales uses small, dependency-ordered
  units with a per-unit oracle (Oxidizer, Syzygy, ENCRUST).
- Keep the project buildable after each unit and roll back failures automatically (ENCRUST), which
  matches the existing `bridge` + `gate` loop.
- Provide the agent an explicit per-language mapping table (section 4) rather than hoping it infers
  idioms; Oxidizer reports total failure without rules.
- On a failed behavior diff, prefer a fresh attempt with a short, minimized counterexample over pasting
  raw fuzzer inputs (FLOURINE finding), so shrink first.

## 4. Mapping ecosystems and semantic traps

### Building blocks

| Need | Common source-side choices | Rust crate (docs) | Note |
| --- | --- | --- | --- |
| HTTP server | Express, Flask/Django/FastAPI, net/http, Rails/Sinatra, Spring | [axum](https://docs.rs/axum/latest/axum/) (tokio + hyper, tower middleware), [actix-web](https://docs.rs/actix-web/latest/actix_web/) | Middleware ordering and default error bodies differ; record full responses including headers |
| HTTP client | fetch/axios, requests/httpx, net/http, Faraday, HttpClient | [reqwest](https://docs.rs/reqwest/latest/reqwest/) | |
| Async runtime | Node event loop, asyncio, goroutines, EventMachine, virtual threads | [tokio](https://docs.rs/tokio/latest/tokio/) | Turborepo found async multi-threaded code across the FFI boundary not viable ([Vercel](https://vercel.com/blog/finishing-turborepos-migration-from-go-to-rust)): port async subsystems whole |
| CLI parsing | commander/yargs, argparse/click, flag/cobra, OptionParser/thor, picocli | [clap](https://docs.rs/clap/latest/clap/) | Turborepo moved CLI parsing first and pinned its edge cases with tests ([Vercel](https://vercel.com/blog/how-turborepo-is-porting-from-go-to-rust)); help text and error wording will differ unless forced |
| JSON / serialization | JSON.parse, json, encoding/json, JSON gem, Jackson | [serde](https://docs.rs/serde/latest/serde/) + [serde_json](https://docs.rs/serde_json/latest/serde_json/) | serde_json maps are sorted by key unless the `preserve_order` feature is enabled ([Map docs](https://docs.rs/serde_json/latest/serde_json/struct.Map.html)) |
| SQL | Prisma/Knex, SQLAlchemy/Django ORM, database/sql, ActiveRecord, JPA/JDBC | [sqlx](https://docs.rs/sqlx/latest/sqlx/) ("compile-time checked queries without a DSL"), [diesel](https://docs.rs/diesel/latest/diesel/) | ORMs generate SQL implicitly; record the SQL actually issued and compare at the query level |
| Regex | JS RegExp, Python `re`, Go `regexp`, Ruby Onigmo, `java.util.regex` | [regex](https://docs.rs/regex/latest/regex/), [fancy-regex](https://docs.rs/fancy-regex/latest/fancy_regex/) | See traps |

### Semantic traps (each should seed `record` inputs)

- **Regex features and Unicode.** The `regex` crate lacks look-around and backreferences, guarantees
  worst-case `O(m * n)` search, and makes `\w`, `\d`, `\s` Unicode-aware by default
  ([regex docs](https://docs.rs/regex/latest/regex/)). `fancy-regex` adds backreferences and
  look-around with a backtracking engine and delegates simple patterns to `regex`
  ([fancy-regex](https://docs.rs/fancy-regex/latest/fancy_regex/)). Seed non-ASCII digits and letters.
- **String length and indexing.** JS strings are UTF-16 code units, and indexes refer to code units;
  strings can hold lone surrogates ([MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String)).
  Rust `String` is UTF-8 and cannot hold lone surrogates. Local probe for `"é😄"`: Rust `len()` 6
  (bytes), JS `.length` 3, Python `len` 2 (code points), Go `len` 6 (bytes).
- **Integer overflow.** Rust panics on overflow in debug and wraps (two's complement) in release; use
  `wrapping_*`, `checked_*`, `saturating_*` to be explicit
  ([Rust Book](https://doc.rust-lang.org/book/ch03-02-data-types.html),
  [Reference](https://doc.rust-lang.org/reference/expressions/operator-expr.html)). Python ints have unlimited
  precision ([Python docs](https://docs.python.org/3/library/stdtypes.html#numeric-types-int-float-complex)),
  JS numbers are doubles ([MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number)); the original's integer model must be chosen per value, not per
  language.
- **Float to int casts.** Rust `as` rounds toward zero, maps NaN to 0 and saturates out-of-range values
  ([Reference](https://doc.rust-lang.org/reference/expressions/operator-expr.html)).
- **Division and modulo of negatives.** Local probe for `-7 / 2` and `-7 % 2`: Rust `-3`, `-1`; Go
  `-3`, `-1`; Python `//` gives `-4` and `%` gives `1`; JS `-7 % 2` is `-1`. Rust `rem_euclid` gives 1.
- **Float formatting.** Local probe: Rust `{}` prints `1e21` as `1000000000000000000000` and `1e-7` as
  `0.0000001`, and `1.0` as `1`; `{:?}` prints `1e21`, `1e-7`, `1.0`. JS prints `1e+21`, `1e-7`, `1`;
  Python `repr` prints `1e+21`, `1e-07`, `1.0`. Any output path that formats floats needs a dedicated
  formatter matching the original, plus recorded cases at the exponent boundaries. Go constant
  expressions are exact (`0.1+0.2` printed `0.3` in the probe because it was a constant), so a Go
  original may not even show the rounding a Rust port shows.
- **Map iteration order.** `HashMap` iterates in arbitrary order with a randomly seeded hasher
  ([std docs](https://doc.rust-lang.org/std/collections/struct.HashMap.html)). Python dicts preserve insertion order
  ([Python docs](https://docs.python.org/3/library/stdtypes.html#dict)), JS objects list integer-like
  keys first and then strings in insertion order
  ([ECMA-262](https://tc39.es/ecma262/#sec-ordinaryownpropertykeys)), and Go map order is unspecified
  ([Go spec](https://go.dev/ref/spec#For_statements)). Use `IndexMap`/`BTreeMap` to match
  the original, and make `diff` order-insensitive only where the original was unordered.
- **Exceptions vs `Result`.** Exception types, messages and exit codes are behavior. fish's crash bugs
  came from `unwrap()` replacing different error handling ([fish](https://fishshell.com/blog/rustport/));
  Oxidizer needed an explicit rule for Go's `(value, error)` pattern
  ([arXiv:2412.08035](https://arxiv.org/abs/2412.08035)). Record error outcomes as cases.
- **FFI memory rules during transition.** Go pointers passed to C must point to pinned memory, checked
  at runtime by `cgocheck` ([cgo docs](https://pkg.go.dev/cmd/cgo)); this constrains the Go bridge
  until the lift.

## 5. The final lift: from hybrid to a pure Rust binary

Documented approaches, in the order they would occur:

1. **Inside-out (today's rust-anything model).** Host program calls Rust units through bindings. Good
   for hot paths; Deliveroo, pyca/cryptography and librsvg stayed here or ended with a thin host-facing
   shell ([Deliveroo](https://deliveroo.engineering/2019/02/14/moving-from-ruby-to-rust.html),
   [librsvg](https://lists.gnome.org/archives/ftp-release-list/2019-September/msg00117.html)). It never
   yields a pure Rust app by itself, because `main`, I/O and the framework stay in the host.
2. **Flip the front door.** A Rust binary owns `main` and argument parsing, and delegates unported work
   to the original. Turborepo did this first through a linked static library, then as a separate
   process with JSON-serialized args when linking failed on Windows/musl
   ([Vercel](https://vercel.com/blog/how-turborepo-is-porting-from-go-to-rust)). `rhg` does it per
   command with automatic fallback ([D10093](https://phab.mercurial-scm.org/D10093)). The original can
   also be embedded in-process from Rust: PyO3 can initialize a Python interpreter from a Rust binary
   (`auto-initialize`, linking libpython)
   ([PyO3 guide](https://pyo3.rs/latest/building-and-distribution.html)); magnus has an `embed` module
   for initializing the Ruby VM ([magnus](https://docs.rs/magnus/latest/magnus/)); the `jni` crate's
   `invocation` feature launches a JVM from Rust ([jni](https://docs.rs/jni/0.21.1/jni/struct.JavaVM.html)).
   For Node and Go, subprocess delegation is the documented path found here (Turborepo); no primary
   source for embedding Node from Rust was checked.
3. **Sandwich for shared internals.** While the front is Rust and the middle is still the original,
   ported internals are linked back into the original (Turborepo's Rust-Go-Rust sandwich with protobuf
   types) ([Vercel](https://vercel.com/blog/how-we-continued-porting-turborepo-to-rust)). Expect to
   outgrow it for async/multi-threaded code.
4. **Pure-Rust path behind a flag.** Build the full Rust code path with stubs, fill it in, run both
   paths in CI and compare outputs, then flip the default after a canary
   ([Vercel](https://vercel.com/blog/finishing-turborepos-migration-from-go-to-rust)).
5. **Delete the fallback and package.** fish embedded its script assets into the binary and built a
   static musl binary, but kept some CMake for installing docs/assets
   ([fish](https://fishshell.com/blog/rustport/)). The original is kept as the reference oracle for the
   app-level replay suite, not as a runtime dependency.

Implications for rust-anything (design proposals, not sourced claims):

- Add a `lift` plan that tracks three states per app: host-main (bindings), rust-main with fallback,
  pure. Each state has its own gate.
- In rust-main state, log every fallback to the original (command, route, or unit) and report a
  "fallback rate" over recorded traffic; the lift is allowed when it is zero over the app-level suite.
- Keep one serialization schema (from `record`'s case format) as both the FFI/IPC wire format and the
  replay format, so hashes and diffs are comparable across languages.
- Make the app-level case format a superset of trycmd-style CLI cases and HTTP exchanges, and run the
  original twice to learn noisy fields (Diffy pattern) before diffing against Rust.

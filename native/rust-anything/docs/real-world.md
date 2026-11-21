# Real-world runs

What happened when rust-anything met real code, and what changed in the tool because of it.

## x18n (the case-study repository)

| Target | What ran | Result |
| --- | --- | --- |
| `packages/core`, 9 units, 29 functions | analyze, bridge, record, diff, bench, shadow, gate | every gate passes; 7,925 recorded cases match; slower than TS through a bridge ([report](../demo/x18n-core/README.md)) |
| `packages/builtins/src/formats/shared/`, 7 functions | same | 3,672/3,672 cases match, 960 of them built from 40 real files of the repository (YAML locales, JSON, Markdown, TS), in LF and CRLF |
| whole repository | analyze | 142 units; `start_here` now lists only units with bridgeable exports |

Found in the tool, and fixed:

- `analyze` recommended the format parsers first, but their only exports are plugin factories
  (`properties(): Plugin`). `analyze` now counts bridgeable exports with the rules `bridge`
  applies, and leaves units with none out of `start_here`.
- Types imported from workspace packages (`@x18n/core`) were not followed, so `Plugin` looked
  like data. Type lookup now follows workspace packages to their source.
- `record` could not run a module importing `@x18n/core` at runtime: the package has no build
  output. The resolve hook now maps workspace packages to their source.
- Shaped inputs found a real bug in the `text.ts` port (Infinity in a range), then an order
  dependency on V8's sort with NaN comparators; `js::sort` now reproduces V8.

## vercel/ms

[vercel/ms](https://github.com/vercel/ms) (5.5k stars, commit `4ff48ce`) turns `"2 days"` into
milliseconds and back; most Node projects pull it in through some dependency. Ported end to end
with rust-anything 1.5.1; the crate is in
[`demo/real-world/vercel-ms-rs`](../demo/real-world/vercel-ms-rs).

| Step | Result |
| --- | --- |
| `analyze` | 1 unit (`src/index.ts`), ready, readiness 96 |
| `bridge --kind napi` | `ms`, `parse`, `parseStrict` and `format` bridged, none kept in TS |
| `record --from-tests -- jest` | 975 cases: generated ones, traps, and 340 calls captured from vercel/ms's 167 Jest tests (the first port used 149 inputs copied from them by hand) |
| port | 200 lines of Rust for 184 of TS, including a `FormatIndex` that reads `options` once per batch |
| `diff` | 784/784 matched on the first build; 975/975 with the captured calls |
| `gate` | passes (fmt, clippy, tests, error paths, size, behavior) |
| `bench`, per call | 0.56x over the recorded cases (0.39x at first): `parse` on a normal string takes 79 ns, level with JS, but edge cases and thrown errors still cost more through the bridge |
| batch, `parse` x100,000 | JS 11.8 ms, Rust 0.65 ms: **18x**; 3.4x counting the one-time handover of the strings (2.1x before strings crossed joined) |
| batch, `format` x100,000 | JS 2.1 ms, Rust 2.5 ms with the handover: **0.8x** (0.2x at first); `format(x, { long: true })` 1.2x |

What the port had to get right, all caught by recorded cases or traps:

- `/.../i` without the `u` flag folds only ASCII letters, while Rust's `regex` folds Unicode, so
  `"1 ſ"` (long s) and the Kelvin sign would have parsed as seconds. The port matches units by
  hand, ASCII only.
- `Math.round` rounds halves up (`-2.5` gives `-2`); Rust's `round` goes away from zero.
- `${n}` prints `1e+21`, `5e-324` and `0` for `-0`; the port reproduces JS number-to-string.
  Its first version also had a tie bug (`25.728225708007812` printed as `...813`) that ms's
  integer outputs never reach.
- `str.length` counts UTF-16 units, so the 100-character limit counts an emoji as two.

Found in the tool, and fixed: a napi batch method with a structured parameter (`format`'s
`options`) did not compile, since the generated binding took a `serde_json::Value` that napi
cannot convert without its `serde-json` feature. Crates with batch classes now enable it.

Taken back into the tool from this port:

- The generated `js.rs` has `js::number_to_string` and `js::math_round`, checked against Node on
  20,000 random doubles; the port uses them instead of its own copies. `number_to_string` takes
  its digits from serde_json's printer: 5x faster than `format!` for integers, 3x for fractions.
- napi batches send string items as one joined string with UTF-16 end offsets, number items as
  one `Float64Array`, and return string results joined (`index.cjs` splits them with `slice`):
  2x faster for strings and 20x for numbers than one JS value per item. `new Batch(items)` and
  `run()` keep their shape.
- `analyze` flags `Math.round`, `toFixed` and `/i` regexes without `u` that mention `s` or `k`,
  with guidance on each.
- Calls cost less: on napi every sync function goes over the binary wire with no napi arguments
  (an argument costs about 55 ns, the whole call 22 ns), one JS wrapper per function instead of
  two, `serde_json::Value` parameters cross as tagged values of any shape, and errors come back
  in the wire buffer. `parse` per call went from 121 to 79 ns.
- `record --from-tests` replaces copying test inputs by hand: it records each call the suite makes
  while the tests run, with the capture code kept out of coverage (ms enforces 100%).

The verdict: `ms` is a good port and a bad speed target. Correct on every recorded case, faster
only when a program parses many durations in one go.

## Fuse.js

[Fuse.js](https://github.com/krisk/Fuse) (20k stars, about 20 million npm downloads a week,
commit `edf2fb6`) is the fuzzy-search library behind countless search boxes. Ported with
rust-anything 1.6 (unreleased); the crate is in [`demo/real-world/fuse-rs`](../demo/real-world/fuse-rs).

`analyze` put every folder of `src/` in one import cycle, so Fuse cannot be ported file by file.
The port takes the path a search spends its time on instead: a 54-line adapter
(`adapter/relevance.ts`) exports Fuse's own score for one string of a string list, built from
Fuse's `BitapSearch`, field norm and `computeScoreSingle`. That function is what a search over
50,000 strings runs 50,000 times. `bridge --batch relevance:text` keeps the strings in Rust and
runs a search over all of them, on all cores.

| Step | Result |
| --- | --- |
| adapter | the adapter's ranking equals `new Fuse(list).search(q)` on 81 queries and option sets before any Rust |
| `record --inputs` | 3,414 cases: titles from Fuse's own fixtures, non-ASCII, emoji, patterns longer than 32 units, every supported option, and wrong-typed options (refused, 149 cases) |
| port | 673 lines of Rust for the 839 lines of Fuse it stands in for |
| `diff` | 3,414/3,414 match on the first build |
| `gate` | passes |
| drop-in | `withRust(Fuse)` answers string-list searches in Rust and passes everything else to Fuse; it returns byte-identical results to Fuse on 360 searches (12 option sets, 15 queries, with and without `limit`) |

Search time per query, Apple M4 (10 cores), Node 24, results checked identical before timing:

| List | Fuse.js | Rust | Speedup |
| ---: | ---: | ---: | ---: |
| 1,000 strings | 1.9 ms | 0.1 ms | 18x |
| 10,000 strings | 19.1 ms | 0.7 ms | 27 to 30x |
| 50,000 strings | 91.2 ms | 2.8 ms | **32x** |
| 50,000 strings, one core | 92.0 ms | 17.5 ms | 5.3x |

What the port had to get right:

- Bitap runs on 32-bit JS integers: `1 << 31` is negative and `(1 << 31) - 1` is a double that
  becomes `2147483647` at the next shift. The port keeps those values as JS has them.
- Text is indexed in UTF-16 code units, so an emoji is two positions, as in Fuse.
- `ignoreDiacritics` is NFD plus a list of combining-mark ranges and a few letters spelled out
  (`ł`, `ß`, `ı`); the port uses the same list.
- `String.prototype.trim` decides which strings Fuse skips: JS whitespace, not Rust's
  `char::is_whitespace` (which includes U+0085 and leaves out U+FEFF).

Found in the tool, and fixed: batch crates whose items were strings but whose results were not
got an unused helper and failed `clippy -D warnings`; and `gate`'s size check counted only the
adapter (54 lines, so 12.5x "too big") instead of the Fuse files it imports.

What it does not cover: object lists with `keys`, extended and token search, logical queries,
`includeMatches`, a custom `sortFn` or `getFn`. The drop-in hands those to Fuse unchanged. The
speedup needs the list to stay in Rust between searches; building it costs about as much as
Fuse's own index (3.4 ms against 2.8 ms for 50,000 strings).

## json-server

[json-server](https://github.com/typicode/json-server) (75k stars, MIT, commit `89a34a4`,
1.0.0-beta.15) serves a REST API from a JSON file: Node, tinyhttp and lowdb. It is the first whole
app ported end to end with the whole-app flow (`app record`, `front-door`, `wire`, `lift`):
the result is one Rust binary that answers what the Node server answers. Everything is in
[`demo/real-world/json-server-rs`](../demo/real-world/json-server-rs).

| Step | Result |
| --- | --- |
| `analyze --whole-app` | a server, 9 units (7 ready, 2 caution), 13 dependencies to replace (tinyhttp, lowdb, sirv, eta, milliparsec, dot-prop, sort-on, inflection...) |
| `app record --http` | 514 cases: 268 requests given with `--inputs` (every route and method, the where operators, sort, pagination, embeds, CORS preflight, static files and ranges, 29 malformed JSON bodies, `/__proto__` and other prototype names, bad escapes) and 245 traps; each recorded twice, random ids left as `[..]` |
| `front-door` | 513/514 at 99.8% fallback; the miss is a raw `ü` in the URL, which Node answers with 400 and the proxy re-encodes |
| units | `where-operators` 207, `parse-where` 289, `matches-where` 1,346, `paginate` 853 cases (calls from json-server's own tests included); `service` 461 and `app` 243 through two small adapters that run Service calls and real HTTP requests; `gate` passes on all six |
| `wire` | reads first: 514/514 at 22.6% fallback, Node answering the writes and Rust reloading the file Node wrote; then every route: 514/514 at 0% |
| `lift` | a pure Rust binary (5.3 MB), 514/514; `bin.ts`'s command line ported by hand (help, version, the file checks, the startup banner) |

After the lift the units have no bindings, so each crate's `tests/recorded.rs` replays its
recorded cases on the port: all 3,399 run with `cargo test`.

What the port had to get right, all caught by recorded cases:

- tinyhttp's router does not dispatch once: after every `next()` it matches all middleware and
  routes again, on the URL the last middleware rewrote, and runs every route for `HEAD`. So
  `HEAD /posts` runs the GET handler, then POST's, and answers POST's 400; `//posts` skips the CORS
  headers and the body parser. The port runs that loop as it is.
- milliparsec parses every POST, PUT, PATCH and DELETE body as JSON, whatever its type, and a bad
  one is a 500 with V8's message: `Expected double-quoted property name in JSON at position 7
  (line 1 column 8)`, `Unexpected token 'x', ..."bbbbbbb": x}" is not valid JSON`. The port
  reproduces V8's parser errors, positions in UTF-16 units included; checked against Node on
  3,806 malformed inputs.
- Sorting uses `localeCompare` (ICU), done with ICU4X's collator and checked on 3,000 pairs;
  `Number()`, `JSON.stringify` (`1e+21`, integer keys first), Node's two query-string parsers
  (which decode `é%E9` differently from WHATWG), dot-prop with its error messages, and inflection
  for embeds (`person` and `people`) are each checked against Node in `json-server-js`.
- `db.data[name]` reads `Object.prototype`: `/__proto__` answers `{}`, `/constructor` an empty
  200, and `PATCH /__proto__` replaces the prototype of the database, which later lookups see.
- Node's HTTP parser answers 400 and closes on raw non-ASCII targets and unknown methods.

Found in json-server: after each of its own writes, its file watcher reloads the file. When the
next request writes before that reload ends, the reload can wipe that write out: the first
recording, sent as fast as the server answered, had a POST answered 201 that later reads did not
show. `app record --pace 50` waits between requests, and the cases are recorded that way. The port
reloads only when someone else changed the file, so it does not lose that write: a deliberate
difference.

Benchmark: Apple M4 (10 cores, 16 GB), macOS 27.3, Node 24.15.0 running `lib/bin.js` as
`tsc` builds it for npm, the Rust binary built with `--release`. Both serve a copy of the same
database on loopback; `ab -k` sends the same requests, 20,000 per row (40 for the slow embed on
the big database) after a warm-up. Two runs, each started with the 1-minute load average under 3;
cells give run 1 / run 2 in requests per second. [`bench/bench.py`](../demo/real-world/json-server-rs/bench/bench.py)
is the script.

The fixture database (6 posts, 3 comments):

| Request | Node, 1 connection | Rust, 1 connection | Node, 50 connections | Rust, 50 connections |
| --- | ---: | ---: | ---: | ---: |
| `GET /posts` | 11.1k / 10.8k | 27.3k / 23.9k | 19.7k / 20.4k | 115.0k / 110.7k |
| `GET /posts/1` | 11.0k / 11.3k | 27.7k / 25.4k | 21.9k / 22.6k | 119.9k / 99.0k |
| filter, sort, page | 10.5k / 10.0k | 25.2k / 21.5k | 18.3k / 16.4k | 107.5k / 96.3k |
| `?_embed=comments` | 8.7k / 6.6k | 22.5k / 21.8k | 13.8k / 11.0k | 98.3k / 100.6k |
| `GET /` (index page) | 7.3k / 6.3k | 27.6k / 21.7k | 10.2k / 9.5k | 135.1k / 119.3k |
| `GET /test.html` (static) | 12.0k / 11.0k | 25.8k / 25.9k | 30.3k / 26.1k | 103.9k / 104.4k |
| `POST /comments`, 10 connections | | | 2,973 / 2,467 | 5,821 / 5,976 |

A generated database (10,000 posts, 20,000 comments, 3.6 MB):

| Request | Node, 1 connection | Rust, 1 connection | Node, 50 (10) connections | Rust, 50 (10) connections |
| --- | ---: | ---: | ---: | ---: |
| `GET /posts/5000` | 8.4k / 9.2k | 14.6k / 14.8k | 15.5k / 16.1k | 69.5k / 67.4k |
| `?_page=3&_per_page=10` | 4.6k / 4.2k | 8.8k / 8.4k | 6.5k / 6.8k | 47.4k / 46.2k |
| `title:contains`, sorted | 431 / 396 | 600 / 560 | 465 / 453 | 2,516 / 2,329 |
| `?_embed=comments`, a page of 5 (10 connections) | 2.4 / 2.4 | 81 / 103 | 2.4 / 2.2 | 253 / 216 |
| `GET /posts`, the whole list (10 connections) | 328 / 319 | 399 / 407 | 381 / 387 | 1,514 / 1,657 |
| `POST /comments` (10 connections) | | | 236 / 241 | 292 / 264 |

| | Node | Rust |
| --- | ---: | ---: |
| Start to first response, fixture database | 85 / 79 ms | 9.9 / 7.9 ms |
| Start to first response, 10,000 posts | 95 / 91 ms | 48 / 47 ms |
| Memory (RSS) idle, fixture database | 74 MB | 3.5 MB |
| Memory (RSS) idle, 10,000 posts | 94 MB | 41 MB |
| Memory (RSS) peak during the runs, fixture database | 424 / 390 MB | 18 MB |
| Memory (RSS) peak during the runs, 10,000 posts | 462 / 485 MB | 289 / 291 MB |

Where the gap is small: one connection reading the whole 10,000-post list (1.2x: both spend the
time printing 1.5 MB of JSON), writes on the big database (1.1 to 1.2x: both rewrite the whole
3.6 MB file on every write, Rust coalescing queued writes as lowdb does), and the sorted title
search on one connection (1.4x). The large gains come from concurrency, which a Node process does
not have, and from the embed, where json-server filters every comment for every post (the port
indexes the comments once per request, same result).

The first lifted binary was slower than Node on the big database (33 against 298 requests per
second for the whole list, 39 against 217 for writes). Profiling found, in order: no
`TCP_NODELAY` (each large response waited about 40 ms for a delayed ACK), the whole database
cloned to add `$schema` before every write, every listed item cloned before printing, one lock
for all requests, and `item.id` hashed with SipHash for each of 10,000 items. All fixed; the 3,399
unit cases and 514 HTTP cases pass after each change.

Found in the tool, and fixed:

- A server that writes its state could not be recorded twice from the same start:
  `--fresh <path>` now copies the database (and `public/`) into a fresh working directory for every
  start, from a snapshot kept with the cases; `--arg` gives every build the original's arguments;
  `--pace` is above.
- `bridge` kept `paginate<T>(items: T[])` in TS (generic over anything, functions included);
  `--as-json T` bridges it as JSON values.
- The front door's routes with parameters answered axum's own 400 for `/posts/%E0`, before the
  unit saw the request; parameters are now read from the URI.
- `lift` staged only the wired unit, not the crates it depends on by path, and wrote a `lib.rs`
  that `cargo fmt --check` rejected; both fixed.
- The servers it generates did not set `TCP_NODELAY`: a response written in two parts waited
  for the client's delayed ACK, about 40 ms. They set it now.

What it does not cover: JSON5 databases (the binary says so and exits), the 100 KiB body limit
(Node's depends on how the body arrives in 64 KiB reads; the port assumes full reads), strings
with lone surrogates (they become U+FFFD), and a database value of `1e400` (Node keeps `Infinity`
in memory until the next reload; the port stores `null`, which is what the file gets).

## Prettier

[Prettier](https://github.com/prettier/prettier) (MIT, tag `3.6.2`, commit `7a8b05f`) is the
first large tool ported with the whole-app flow where the port covers only part of the input:
the front door formats a file in Rust when every construct in it is ported, and hands the whole
file to Prettier otherwise. Everything is in
[`demo/real-world/prettier-rs`](../demo/real-world/prettier-rs); its `DECISIONS.md` records the
choices (whole-file fallback, oxc for parsing, Prettier's own printer code ported rather than an
existing Rust formatter, byte-identical output as the bar).

| Step | Result |
| --- | --- |
| `app record --inputs` | 3,080 CLI cases in 134 s: 1,525 of Prettier's own `tests/format` JS/TS fixtures, 1,196 runs of 200 of those fixtures with six common options (`--no-semi`, `--single-quote`, `--trailing-comma none`, `--use-tabs`, `--print-width 40` and `120`), 59 files of three real repositories (ms, Fuse.js, json-server), and 300 traps the tool adds; then 63 cases with config files, `.editorconfig` and ignore files around the input; stdout, stderr and exit code |
| `front-door` | 3,080/3,080 at 100% fallback (before the config cases) |
| oxc check | Prettier formatting with its own oxc parser gives output byte-identical to its Babel/TypeScript parsers on 1,351/1,351 fixtures oxc parses (64 fixtures do not parse in oxc); so the port reads oxc's ESTree |
| `doc-printer-rs` | `printDocToString` and `getStringWidth` (`src/document/`), recorded on 2,848 docs (2,570 harvested from the fixtures, 180 written to reach the rest); `gate` passes, 608 of the printer's 613 lines reached |
| `prettier-js-rs` | the JavaScript and TypeScript printer, 12,000 lines of Rust: statements, expressions, member chains, call arguments, assignments, binary operators, arrays, objects, functions and arrows, modules, template literals, ternaries (both styles), classes and decorators, JSX, every TypeScript node, `needs-parens`, comments (attachment with the JavaScript and TypeScript handlers, printing, every comment branch in those printers) and `prettier-ignore` |
| `js-format-rs` | Prettier's `format()` bridged and backed by `prettier-js-rs`; recorded on the 15,753 `format()` calls the CLI cases make for files the port covers; `gate` passes 15,753/15,753 |
| `wire` | milestone 1: 3,143/3,143 at 51.9% fallback (1,513 in Rust); milestone 2, TypeScript: 3,143/3,143 at 22.5% (2,436 in Rust); with JSON: 4,621/4,621 at 25.8% (3,431 in Rust); with GraphQL: 5,468/5,468 at 22.9% (4,218 in Rust); with YAML: 8,812/8,812 at 14.6% (7,527 in Rust); with Markdown: 15,162/15,162 at 9.8% (13,676 in Rust); with CSS, SCSS and Less: **17,224/17,224 at 8.1% fallback, 15,837 cases formatted fully in Rust**, byte-identical |
| config | the glue resolves Prettier's config as its CLI does (closest `.prettierrc`/`.prettierrc.json`/`package.json` `"prettier"` key with `overrides`, `.editorconfig` up to the project root, `.gitignore`/`.prettierignore` and `node_modules`), and hands YAML, TOML, JSON5 and JavaScript configs, unknown options and ignored files to Prettier; 44 of the 63 config cases run in Rust |
| JSON | 1,478 more cases (`gen-json-inputs.mjs`): Prettier's 55 JSON fixtures as `.json`, `.jsonc`, `.json5` and `package.json`, ten option variants, 26 real JSON files (`package.json`, `tsconfig.json`, lock files...) and edge snippets; the port picks `json`, `jsonc`, `json5` or `json-stringify` from the file name as Prettier's language list does, parses the value with oxc, checks it with Prettier's `assertJsonNode`, and prints it with the same printer (keys quoted, double quotes, no trailing commas for `json`) or, for `json-stringify`, with a port of `printer-estree-json.js`; 995 of the 1,019 JSON cases Prettier formats run in Rust |
| GraphQL | 847 more cases (`gen-graphql-inputs.mjs`): Prettier's 58 GraphQL fixtures (49 in `tests/format/graphql`, 9 pragma and error fixtures) as `.graphql`, `.gql` and `.graphqls`, eight option variants, `--parser graphql` on a `.txt` path, and edge snippets (block strings, escapes, legacy fragment variables, comments in every position, `prettier-ignore`, CRLF, BOM, 13 invalid inputs); the port is `graphql.rs`: graphql-js 16.11's lexer and parser (with `allowLegacyFragmentVariables`, block string dedent, every syntax error kept as an error), Prettier's generic comment attachment and printing (GraphQL has no comment handlers), and `printer-graphql.js`; **all 787 GraphQL cases Prettier formats run in Rust**, the other 60 are Prettier's own syntax errors and stay on Prettier |
| YAML | 3,344 more cases (`gen-yaml-inputs.mjs`): Prettier's 355 YAML fixtures (348 in `tests/format/yaml`, the rest pragma, parser-inference and error fixtures) as `.yml` and `.yaml`, six option variants (`--prose-wrap always` and `never`, `--tab-width 4`, `--no-bracket-spacing`, `--single-quote`, `--print-width 40`), every fixture with CRLF line ends, 35 real files (Prettier's and the three repositories' GitHub workflows, issue templates, `.codecov.yml`...), YAML file names (`CITATION.cff`, `.clang-format`, `.prettierrc`, `yarn.lock`) and edge snippets (anchors, tags, `%TAG` directives, documents, block scalars, duplicate keys, bad indentation); the port is `src/yaml/`: yaml 1.10.2's CST parser and the checks its `Document#parse` makes (anchors, tags, pairs, duplicate keys, every error it reports), yaml-unist-parser 2.0.5's transform, comment attachment and position updates, and `printer-yaml.js`; **3,309 of the 3,310 YAML cases Prettier formats run in Rust** (the one left is `.prettierrc`, which Prettier prints as JSON), the other 34 are Prettier's own errors |
| Markdown | 6,350 more cases (`gen-markdown-inputs.mjs`): Prettier's 852 Markdown fixtures (821 in `tests/format/markdown`, the rest pragma and front matter fixtures), five option variants (`--prose-wrap always` and `never`, `--tab-width 4`, `--print-width 40`, `--single-quote`), every fixture with CRLF, a quarter of them as `.markdown`, 54 real files (Prettier's docs and READMEs, the three repositories' Markdown) twice, and edge snippets (lists, tasks, tables, links, footnotes, math, liquid, wiki links, entities, `prettier-ignore` ranges, CJK text); the port is `src/markdown/`: remark-parse 8.0.3 with remark-footnotes, remark-math and Prettier's front matter, liquid and wiki link tokenizers, parse-entities, Prettier's preprocess and `printer-markdown.js`; code blocks in JavaScript, TypeScript, JSON, GraphQL, YAML, Markdown, CSS, SCSS and Less, and YAML front matter, are formatted by the other ports as Prettier does; **6,305 of the 6,350 Markdown cases run in Rust**, the rest hold HTML code blocks (not ported) or code oxc does not parse |
| CSS, SCSS, Less | 2,062 more cases (`gen-css-inputs.mjs`): Prettier's 248 style fixtures (109 CSS, 80 SCSS, 36 Less, 23 in `tests/format/misc`), the CSS fixtures again as `.scss` and `.less`, five option variants (`--single-quote`, `--tab-width 4`, `--print-width 40`, `--trailing-comma none`, `--use-tabs`), every fixture with CRLF, 6 real files (Prettier's website styles, Fuse.js's docs styles) and 22 edge snippets in all three languages; the port is `src/css/`: postcss 8.5.6's tokenizer and parser with postcss-scss 4.0.9's changes (`//` comments, `#{}` interpolation, nested properties) and postcss-less 6.0.0's (inline comments, mixin calls, `each`, variables, `@{}` interpolation, `extend`), Prettier's `parseNestedCSS` and its quote masking for Less comments, postcss-values-parser 2.0.1 in loose mode, postcss-selector-parser 2.2.3, postcss-media-query-parser 0.2.3, and `printer-postcss.js` with its value group printers; **all 2,004 CSS, SCSS and Less cases Prettier formats run in Rust** (890 CSS, 711 SCSS, 403 Less), the other 58 are Prettier's own errors |

Formatted fully in Rust, by group: 1,295 of the 1,525 fixtures, 1,016 of the 1,196 option
variants, all 59 real files, 20 of the 300 traps, and 46 of the 63 config cases. Of the 929
TypeScript cases, 849 run in Rust. (The milestone 1 figure of 44 real files was low: the
corpus checker skipped every case whose name contained `config-`, which also hid four real
files such as `vitest.config.ts`; the replay's own fallback rate was not affected.) What still goes to Prettier, by the first thing the port does
not handle: oxc parse errors 379, invalid CLI input 220 (200 traps with a file name Prettier
cannot infer a parser for, 20 with an invalid `--trailing-comma`; errors stay on Prettier by
design), embedded CSS/GraphQL/HTML/Markdown 86, `f<<T>` type arguments that oxc accepts and
TypeScript rejects 5, and 17 of the config cases.

The JSON cases that still go to Prettier are the 459 it rejects itself (invalid JSON, comments
in `package.json`, expressions) and one fixture with keys like `1e400`, which serde_json cannot
read back from oxc's ESTree (24 cases with its variants).

The oxc parse errors split in two. About 290 are inputs Prettier rejects too (invalid syntax in
the fixtures, cursor and range markers); they would fall back anyway. About 90 are Babel proposal
syntax oxc does not parse (`do` expressions, private-name destructuring, module blocks, `::`,
optional chaining assignment); a printer port cannot reach them. oxc also reports TypeScript and
JavaScript checks that Prettier's parsers do not (an initializer in an ambient context,
`'private'` with `'abstract'`, an empty type argument list, `return` outside a function...); the
port accepts 16 of those messages, where oxc still builds the whole tree, and two it tried
(computed enum members, an index signature without a type) were taken back off the list because
oxc's tree came back incomplete.

How a fallback works: the glue resolves the options for the file first (an ignored file, or a
config it does not read, goes to Prettier); then it reads stdin, tries the Rust port, and on any
`Unsupported` calls `original::run_with_stdin` with the bytes already read. The port refuses
rather than guesses: it bails on syntax it does not print and on a comment that would go
unprinted (Prettier's own `ensureAllCommentsPrinted` check).

What the port had to get right, all caught by recorded cases:

- Prettier's docs share subtrees (the same group in two `conditionalGroup` states), and
  `propagateBreaks` marks a shared group once for all its uses. The Rust doc keeps that identity
  with `Rc`, and the harvested docs are taken after `propagateBreaks` so they compare.
- `path.match` hands predicate *i* the node *i* levels up and the key below it; `needs-parens`
  depends on it.
- `.js` files parse as modules (top-level `await`), `.cjs` as scripts.
- Comment attachment depends on node order: children are sorted by start, then end, and a tie
  between two comments is broken by the gap text (whitespace or `(` only).
- Closure type casts (`/** @type {X} */ (y)`) keep their parentheses, so the
  `ParenthesizedExpression` stays in the tree for them.
- JSX children are cleaned up by comparing docs with `===` against the shared `hardline`,
  `softline` and `{" "}` docs; the port keeps those identities as an enum until the final doc is
  built, and treats `{" "}` children as text, as Prettier rewrites them.
- In an override, Prettier matches patterns without a slash against the file's basename, and in
  that pass tests `excludeFiles` against the basename too, so `excludeFiles: "legacy/*.js"` does
  not exclude `legacy/a.js` from `files: "*.js"`.
- TypeScript comes out of oxc with the template element ranges espree uses (delimiters
  included) and `TSImportType.source` where Prettier reads `argument`; Prettier corrects both in
  its own postprocess for its oxc parser, and the port does the same.
- `prettier-ignore` prints the node's original text and marks every comment inside it printed;
  inside unions and mapped types the comment handlers flag the next node instead, and that
  comment is then no longer an ignore comment.
- `cleanDoc` runs on nested binary expressions that carry comments, merging strings and
  flattening arrays, and the next step looks for the first group in what it returns.

Benchmark: Apple M4 (10 cores, 16 GB), macOS 27.3, Node 24.15.0 running Prettier 3.6.2's
`dist` build, the Rust built with `--release`, after milestone 2. Two runs, each started with the
1-minute load average under 3 (2.60 and 2.95); cells give run 1 / run 2. The scripts are in
[`demo/real-world/prettier-rs/bench`](../demo/real-world/prettier-rs/bench) and
`prettier-js-rs/examples/bench.rs`; every CLI output was compared and matched in both runs.

| What | Prettier | Rust | |
| --- | ---: | ---: | ---: |
| CLI, one process per file, 2,360 files the port covers | 128.6 s / 143.4 s | 6.9 s / 7.2 s | 18.8x / 19.8x |
| CLI, the 59 real repository files | 3.63 s / 3.61 s | 0.20 s / 0.20 s | 18x / 18x |
| In process, `format()` warmed up, the same 2,360 files, 20 calls each | 600 / 637 ms | 143 / 150 ms | 4.2x / 4.2x |
| In process, per file (geometric mean / median) | | | 7.4x / 6.9x and 7.4x / 7.0x |
| In process, the slowest file for Rust (`js-arrays-nested`, `js-method-chain-first-long`) | | | 1.47x / 1.52x |
| Doc printer alone, called from JS through the napi bridge, 2,848 docs | 65 / 66 µs a call | 134 / 133 µs | 0.48x / 0.49x |

Most of the CLI gain is Node's start-up (about 55 ms a file against 3 ms). In process, where
Prettier is warmed up, no file was slower in either run. The largest file, Fuse.js's 31 KB
`fuzzy-search.test.js`, is 2.3x faster, ms's `index.test.ts` 3.2x. The doc printer alone is
slower than JS because each call converts the whole doc to JSON and back across the bridge; the
front door does not use it that way.

GraphQL, measured the same way after its port (same machine, two runs started at 1-minute
load 2.81 and 2.23), on the 787 GraphQL cases Prettier formats:

| What | Prettier | Rust | |
| --- | ---: | ---: | ---: |
| CLI, one process per file, 787 files (every output identical) | 36.6 s / 36.9 s | 2.11 s / 2.11 s | 17.3x / 17.5x |
| In process, `format()` warmed up, 781 files, 20 calls each | 102.6 / 87.0 ms | 10.1 / 8.8 ms | 10.2x / 9.9x |
| In process, per file (median) | | | 17.7x / 18.0x |
| In process, the slowest file for Rust | | | 1.61x / 2.9x |

Six whitespace-only cases return before parsing in both and are left out of the in-process rows.
No GraphQL file was slower in either run.

YAML, the same way after its port (two runs started at 1-minute load 2.72 and 2.71), on the
YAML cases Prettier formats:

| What | Prettier | Rust | |
| --- | ---: | ---: | ---: |
| CLI, one process per file, 3,292 files (every output identical) | 160.8 s / 158.9 s | 8.98 s / 8.89 s | 17.9x / 17.9x |
| In process, `format()` warmed up, 3,272 files, 20 calls each | 488.7 / 459.5 ms | 51.4 / 49.6 ms | 9.5x / 9.3x |
| In process, per file (median) | | | 13.2x / 13.2x |
| In process, the slowest file for Rust | | | 2.55x / 2.98x |

Twenty whitespace-only cases are left out of the in-process rows, as for GraphQL. No YAML file
was slower in either run.

CSS, SCSS and Less, and Markdown, the same way after their ports (CSS runs started at 1-minute
load 2.61 and 2.98, Markdown at 2.84 and 2.91; the load reached 3.0 to 3.6 by the end of each
run), on the cases Prettier formats:

| What | Prettier | Rust | |
| --- | ---: | ---: | ---: |
| CSS/SCSS/Less CLI, one process per file, 2,004 files (every output identical) | 107.5 s / 110.9 s | 12.7 s / 13.1 s | 8.5x / 8.5x |
| CSS/SCSS/Less in process, `format()` warmed up, 2,001 files, 20 calls each | 1,116 / 1,101 ms | 203 / 208 ms | 5.5x / 5.3x |
| CSS/SCSS/Less in process, per file (median) | | | 9.7x / 9.5x |
| Markdown CLI, one process per file, 6,305 files (every output identical) | 352.2 s / 357.4 s | 22.5 s / 22.9 s | 15.6x / 15.6x |
| Markdown in process, `format()` warmed up, 6,299 files, 20 calls each | 1,731 / 1,547 ms | 176 / 175 ms | 9.8x / 8.8x |
| Markdown in process, per file (median) | | | 25.2x / 24.6x |

Empty cases (three CSS, six Markdown) return before parsing and are left out of the in-process
rows. In each language's second run one file came out slower than Prettier (0.82x and 0.88x):
`crlf-less-less-comments-less` took 135 µs against 8.5 µs in the first run, and
`single-quote-markdown-linkreference-cjk-md` 182 µs against 35 µs, so both were one-off stalls;
in the first runs no file was slower, and the slowest file for Rust was 3.09x (CSS) and 2.27x
(Markdown). The CSS CLI gain is lower than for the other languages because each style file is
small and process start dominates; the front door's 6.3 ms per file is mostly that.

The benchmarks found three problems, all fixed. First, a 40x slowdown on arrays of numbers:
the port of `printNumber` compiled its four regular expressions on every call (cached, 23 ms
became 0.3 ms). Second, run from inside the Prettier checkout, the Rust path ignored the config
files around the input (Prettier read that repository's `.prettierrc`) because the recorded
cases all ran in an empty folder; the glue now resolves that config itself (the `config` row
above), checked by 63 recorded cases. Its first version doubled the CLI time by compiling the
`.editorconfig` parser's patterns for every directory up to `/`. Third, the remaining time was
in reading oxc's ESTree as `serde_json::Value`: the bridge crates need serde_json's ordered maps,
which hash every field name. The port now reads the tree into its own value type (fields in a
small vector with interned keys), hashes with an Fx-style hasher, and keeps static strings in
the doc as borrowed `Cow`s; the largest file went from 6.7 ms to 4.0 ms, and the median file
from 4.7x to 7x faster than Prettier.

Found in the tool, and fixed:

- With `CARGO_TARGET_DIR` set, every front door built into the same directory, so app tests
  replayed each other's binaries; the front door now builds into its own `rust/app/target`.
- Glue had no way to decline one case after reading stdin; `original::run_with_stdin` passes the
  bytes on, and `lift` refuses while any glue still calls `original::`.
- Case and input lines deeper than 128 levels (Prettier's doc trees) failed to parse; the
  harness and generated `from_json` read them without a depth limit.
- `--inputs` case names were cut to 24 characters and collided; given names keep 96.
- `bridge` found no exports in `export { a, b as c }` lists; it reads them now.

Not fixed yet, and noted in [`pitfalls.md`](pitfalls.md): `gate`'s unwrap, panic and size
checks read only `port.rs`, so a port living in a dependency crate (as `prettier-js-rs` does)
is checked by hand; `size` counts the whole import closure, `node_modules` included; and
`app replay` without `--build` replays the original command and reports no fallback rate.
`analyze --whole-app` counted 1,436 fixture files as parse errors and Prettier's `src/` as one
import cycle.

What it does not cover: YAML, TOML, JSON5 and JavaScript config files, embedded languages,
files oxc cannot parse, options beyond the common ones, and every language other than JavaScript,
TypeScript, JSON, GraphQL, YAML, Markdown, CSS, SCSS and Less. Those files are formatted by Prettier through the front door, and every recorded
case, Rust or fallback, matches.

Other languages beyond JSON, GraphQL, YAML, Markdown, CSS, SCSS and Less are not started. Each needs a Rust parser that produces the AST Prettier's
printer for it expects, a corpus recorded the same way, and the printer itself:

| Language | Prettier printer (lines of JS) | Parser Prettier uses | Fixtures in `tests/format` |
| --- | ---: | --- | ---: |
| JSON | shares the JavaScript printer | Babel's expression parser | 43 |
| GraphQL | 672 | `graphql-js` | 49 |
| YAML | 1,406 | `yaml` with Prettier's own AST | 348 |
| CSS, SCSS, Less | 3,249 | `postcss` with `postcss-scss`/`postcss-less` | 225 |
| Markdown | 2,373 | `remark` | 821 |
| HTML, Vue, Angular | 4,118 | `angular-html-parser` | 285 |

JSON, GraphQL, YAML, Markdown, CSS, SCSS and Less are done (above). GraphQL took one module of
about 2,000 lines with its parser; YAML about 4,400 (CST parser 1,300, document checks and tree
2,100, printer 1,000); Markdown about 5,100 (remark-parse with its plugins 3,500, printer 1,650);
CSS, SCSS and Less about 5,600 (postcss with both syntaxes 1,550, value, selector and media query
parsers 1,850, `parseNestedCSS` and the printer 2,150). HTML is the one left. Embedded CSS, GraphQL and HTML in template literals would come with
those printers (embedded GraphQL needs only the template glue now).

Beyond the recorded cases, the new parsers were fuzzed against Prettier: 13,991 YAML
files, 6,000 GraphQL files, 20,000 Markdown files and 11,997 CSS, SCSS and Less files made by deleting, inserting and re-indenting
pieces of the fixtures. Rust formatted 9,973 of the YAML files, 2,131 of the GraphQL files (every
GraphQL file Prettier accepted) 19,150 of the Markdown files and 8,153 of the 8,223 style files Prettier accepted, each
byte-identical to Prettier, and accepted no file that Prettier rejects. The first Markdown run
found one: `![][x` with no closing bracket, where Prettier puts `null` in its doc and throws; the
port now refuses it too. The style runs found two: `@function f($a, $args\n...)`, where Prettier's
`$x ...` regex matches the first `$` that has `...` after it, not the first `$`; and `pow(, 2)`,
an empty comma group on which Prettier's printer throws (the port now refuses it). One CSS mutant
made Prettier loop forever, so each fuzz file now runs in a child process with a 10 s limit.

## Fixtures written to stress the tool

| Fixture | Exercises | Result |
| --- | --- | --- |
| `tests/fixtures/async` | async functions on both bridges | 900/900, both gates pass |
| `tests/fixtures/classes` | classes: state, constructor errors, `Map` order, `split('')` on emoji | 600/600 scenarios, both gates pass |
| `tests/fixtures/aliases` | tsconfig `paths` with `extends`, comments, trailing commas | analyze and record follow them |
| `tests/fixtures/monorepo` | a package importing a sibling with no build output | record resolves it to source |

Found in the tool, and fixed: napi turned `None` into `null` (JS gives `undefined`); scenarios
too short and too random to catch a stateful bug (now up to 12 calls, reusing earlier strings);
generated `let out = ()` and immediately-called closures that clippy rejects.

## External libraries (static only)

`unjs/scule`, `unjs/ufo` and `unjs/pathe` were cloned and analyzed and bridged. Running their
code (`record` executes the original) was refused by the session's safety policy for code
downloaded from the internet, so these runs stop before `record`.

| Library | analyze | bridge |
| --- | --- | --- |
| scule (case conversion) | 2 units, ready | all 11 functions bridgeable after the fix below |
| ufo (URL utilities) | 7 units, ready; `punycode.ts` ranked first | `toASCII` bridged (untyped JS: crosses as JSON) |
| pathe (path utilities) | 5 units; `_glob.ts` caution (dynamic regex) | 13 functions bridged; `typeof path.join`-typed exports cross as JSON |

Found in the tool, and fixed: constrained generics (`camelCase<T extends string | readonly
string[]>`) were treated as "any value, functions included", which kept 10 of scule's 11
functions in TS. Type parameters with data constraints are now substituted by the constraint.

A port of scule was written (`demo` not included: it was never recorded against the original).

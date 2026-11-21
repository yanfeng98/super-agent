# Real code in six languages

rust-anything was run end to end (analyze, bridge, record, port, diff, gate, bench) on real code in
each language it supports. The code had to be already on this machine: code downloaded from the
internet was not executed. So the targets are standard-library modules shipped with the
installed toolchains, and for Java and C, where no suitable library source was on disk, the JDK
and glibc themselves serve as the oracle behind a thin facade.

The ports, recorded cases and extra inputs are in [`demo/real-world`](../demo/real-world/README.md).

| Language | Code | Functions | Recorded cases | Result |
| --- | --- | ---: | ---: | --- |
| Go | `path` from Go 1.24's standard library | 7 (+1 kept) | 2,642 (542 from Go's own test tables) | 2,642/2,642, gate passes |
| Ruby | `shellwords.rb` from Ruby 3.3 | 3 | 1,617 (446 raise) | 1,617/1,617, gate passes |
| Python | `colorsys.py` from CPython 3.13 | 6 | 2,400 | 2,400/2,400, gate passes |
| Java | JDK 21 behind a facade (`Integer`, `Long`, `Math`, `Double.toString`, `URLEncoder`, `Base64`, `toUpperCase`) | 11 | 4,492 (226 throw) | 4,492/4,492, gate passes |
| C | glibc behind a facade (`strtol`, `atoi`, `strtod`, `strspn`, `strcmp`, ctype) | 9 | 1,860 (6 crash) | 1,859/1,859 after dropping one undefined-behavior case, gate passes |
| C | aklomp/base64 (vendored in mypyc's runtime) | 0 of 7 | - | plan only: every function takes output buffers or a state struct |

## Speed

The first bridges for Python, Go, Ruby and Java sent every argument and result through JSON
text, which cost 10-50x more than the work in these functions. They were rebuilt to convert
values directly. `bench` now times only the calls (arguments decoded once, in process) and says
whether to ship.

| Language | Per call before | Per call now | Rust vs the original (`bench`, geometric mean) | Verdict |
| --- | ---: | ---: | --- | --- |
| Ruby | 6.9-9.5 µs (Fiddle + JSON) | 0.36-1.15 µs (magnus extension) | **4.4x faster** (3.6-7x per function) | ship |
| Python | 11.7 µs (JSON) | about 0.3 µs (direct PyO3 conversion) | **1.35x faster** (1.5-1.9x on most calls) | ship |
| C | - | no crossing: linked directly | 0.87x: ctype 1.1-1.4x faster, glibc's tuned `strtod`, `strspn`, `strcmp` 0.5-0.7x | optimize or keep the slower ones |
| Java | about 1.7 µs (JNI + JSON) | 30-800 ns (typed JNI and binary wire) | 0.18x | port bigger units |
| Go | 2.6 µs (cgo + JSON) | 0.11-0.42 µs (typed cgo and binary wire) | 0.04x | port bigger units |

Lists, maps, structs and several results in Go and Java cross as a binary wire generated per
type: Go's `WordFreq` (a map) went from 5.1 to 1.0 µs, `Midpoint` (a struct) from 7.2 to 1.0 µs;
Java's `frequencies` (a `Map`) from 8.4 to 1.0 µs, and `words` (a `List<String>`) from 6.0 µs to
0.57 µs, 1.39x faster than Java itself.

TypeScript (x18n core, nine units, 27 functions on the same cases): the binary wire cut the
bridge cost by a third on average (`rangeContains` 3.6 -> 1.1 µs, `applyTextEdits` 10.4 -> 5.7
µs, `toWireRange` 5.0 -> 2.6 µs), and `buildTree` went from even to 2.0x faster than TS. The
other functions are leaves that V8 runs in 15-600 ns: still slower in Rust, as for Go and Java.

What decides it:

- **Interpreted originals (Python, Ruby)**: the call into Rust costs about 100-300 ns and the
  Rust code is much faster than the interpreter, so even small functions win.
- **Compiled originals (Go, Java after the JIT)**: the original leaf function takes 4-90 ns; one
  cgo call alone costs about 80 ns, a JNI call with strings more. No bridge can make a 5 ns
  function faster. These languages gain only when one call does a lot of work: port a function
  together with its callers. `bench` says so.
- **C**: there is no crossing; the Rust code is compared with the C code directly, and the result
  is a property of the port.

### Batches: x18n's search

The leaf ports above are measured per call. On x18n's sidebar search (50,000 keys, typed letter
by letter), `rust-anything profile` showed `fuzzyScore` at about a quarter of the run, called once per key by
`matched`, and suggested `--batch fuzzyScore:text`. The generated `FuzzyScoreBatch` keeps the
keys in Rust; a `FuzzyScoreIndex` in `port.rs` lowercases them to UTF-16 once and keeps a 64-bit
mask of the code units in each key, so most keys that cannot match are skipped unread. Results
are identical to the TS pipeline for every query:

| | ms per query | vs TS |
| --- | ---: | ---: |
| TS pipeline | 10 | 1x |
| TS with the same narrowing | 4.1-5.4 | 2x |
| Batch, default index (port per item, 4 cores) | 5.1 | 1.7x |
| Batch with `FuzzyScoreIndex` (masks), napi, 4 cores | 1.2 | 7x |
| Batch with `FuzzyScoreIndex` ranking itself, napi, 4 cores | 0.45-0.56 | 18-22x |
| The same, wasm, 1 thread | 0.95 | 11.5x |

The step from 7x to 20x: of the 1.2 ms, 0.4 ms was the benchmark's own `Array.from(order, i =>
keys[i])` (a plain loop costs 0.02 ms). The rest came from letting the index rank by itself:
it rescores only the last query's matches when the new query extends it (a key that does not
match `s` cannot match `se`), reads one-letter queries from a table of first positions, scores
ASCII keys as bytes, and orders the whole-number scores with a counting sort. Callgrind showed
library calls (`memchr`, `memcmp`) costing more than plain loops on 30-byte keys. Part of the gain
is the algorithm, so the benchmark also runs TS with the same narrowing: Rust is 7-12x faster
than that (wasm 5x).

The same pattern in the other languages: the Rust batch on 4 cores against the original on one
thread, identical results and errors. The Go and Java rows are not a fair race; see the
correction below.

| Language | Workload | Original (1 thread) | Per-call port vs original | Batch (4 cores) vs original |
| --- | --- | ---: | ---: | ---: |
| Python | fixture `slugify`, 20,000 strings | 98 ms | - | 17x |
| Ruby | fixture `slugify`, 20,000 strings | 110 ms | - | 12.7x |
| Ruby | `Shellwords.shellescape`, 50,000 mixed values | 88 ms | - | 11.8x |
| Go | fixture `Slugify`, 50,000 strings | 49 ms | 0.93x | 3.3-4.1x |
| Java | fixture `slugify`, 50,000 strings | 43 ms | 0.68x | 3.0x |

The Go batch first ran slower than one thread: the port allocates per item and glibc's allocator
serialized the threads inside the Go process. Batch crates now use mimalloc. The shellescape batch
also found a port bug that per-call recording had missed: Ruby symbols (`shellescape(:sym)`) came
back as their JSON marker. Fixed in the port.

Go and Java then went from 2.5x and 1.7x to 4x and 3x. Inside the batch, half the time was not
the work: results were joined and freed on one thread, Go copied them a second time, and Java
made one JNI call per string through modified UTF-8. String results now go back in bulk, written
on all cores into Go's own buffer or one Java `char[]`. The other half was the ports: on one
thread the Rust `slugify` was barely faster than Go's and slower than Java's JIT. Lowering ASCII
without table lookups and without an intermediate string fixed that; Java's port keeps the
final-sigma rule by lowering only the whitespace-separated piece that holds a `Σ` as a whole.
Per call, the Go port went from 0.73x to 0.93x, the Java one from 0.46x to 0.68x.

**Correction (2026-11-19, [#110](https://github.com/erkamyaman/rust-anything/issues/110)).** The Java `slugify` numbers above do not hold up:

- **The port was wrong on some text.** Java's `Character.isLetterOrDigit` takes letters and
  decimal digits per UTF-16 unit; the port used Rust's `char::is_alphanumeric`, which also takes
  `²`, `Ⅰ`, `½`, vowel signs such as `ि` and characters beyond U+FFFF. `slugify("a²b", 0)` gave
  `a²b` instead of `a-b`. The 50,000 benchmark strings had none of them, and neither did the
  recording. The port now checks the general category per UTF-16 unit, `record` adds those
  characters to every string trap, and the fixture records 1,596/1,596.
- **The race was not even.** The batch ran on 4 cores against Java's serial loop, the batch was
  built before the timer, and each side reported its best of 15 calls. With JMH, the same inputs,
  the same number of workers and the copies inside the timer, the reporter measured the original
  Java at 9.8 ms and the Rust batch at 24.7 ms on 4 workers, 34.1 ms and 43.0 ms on one (50,000
  strings, Windows, Core i5-12450H). The Java original is faster. The Go row was measured the same
  way and has not been re-run.

## What the runs found

### Bugs in rust-anything, fixed

- **Floats changed by one ULP** (Java run). `serde_json`'s default float parser is not always
  correctly rounded. rust-anything perturbed recorded inputs when it read them, and generated crates
  perturbed them again. 43 `Double.toString` cases failed for a correct port. Fixed with
  `serde_json`'s `float_roundtrip` feature in rust-anything and in every crate it generates (napi-rs,
  wasm and the JSON dispatcher).
- **C crashes killed recording** (C run). Inputs outside a C function's contract (`isalpha(INT_MAX)`)
  segfault glibc. The C driver now runs each case in a forked child and records
  `crash: SIGSEGV`; `diff` accepts any port behavior there, since C's is undefined.
- **`module_function :name` not seen** (Ruby run). `shellwords` declares its module functions
  with symbol arguments after the methods; the exporter only knew the bare `module_function`
  form. Both forms are found now.
- **Untyped code got garbage inputs** (Python run). `colorsys` has no type hints, so 1,709 of
  1,800 generated cases were `TypeError`s. Unannotated parameters used as numbers (`r - g`,
  `max(r, g, b)`, `x < 1`) are now typed `number`, in Python and Ruby: 94 errors remain, all real
  (`ZeroDivisionError`).
- **Ints passed as floats never recorded** (Python run). Python callers pass `1` as often as
  `1.0`, and `colorsys` passes ints through (`hsv_to_rgb(0, 0, 7)` is `(7, 7, 7)`). Recording now
  passes integral numbers as ints in every fourth Python case; that exposed 134 mismatches in a
  port typed `f64`.
- **Messages from the TypeScript days**: "must stay in JS", "kept in TS", "characterization tests
  against the TS code" now name the source language.
- **Reopened core classes** (`class String` in `shellwords.rb`) are now a monkeypatching risk.

### Behavior a port has to match, found by `diff`

Each line was a mismatch in a first port, caught by the recorded cases. Lines marked *fixture*
came from the small test projects in `tests/fixtures/`, written for each language; the rest from
the real-world runs above.

- Go `path.Clean`: Go's `lazybuf` reads the byte at the write index after it moves back, not the
  last kept byte (9 cases, from Go's own test table).
- *fixture* Go `strings.ToLower` and Ruby `downcase` lower each rune alone: `ΣΊΣΥΦΟΣ` becomes `σίσυφοσ`,
  where Rust's `to_lowercase` applies the final-sigma rule. Go also maps `İ` to `i`, where Rust
  gives `i̇`.
- *fixture* Go slices strings by bytes; a cut through a rune reaches JSON as one U+FFFD per byte.
- *fixture* Ruby `slugify` raised `NoMethodError` for a negative `max` (`slug[0, -1]`
  is `nil`): a bug in the original that the port has to keep or the callers have to be fixed first.
- *fixture* Ruby `Array#sum` uses Kahan-Babuska summation and starts from integer 0, so `[-0.0].sum` is `0.0`.
- Ruby `Float#to_s` uses exponent form from 1e15 up (`1e15.to_s` is `1.0e+15`, `1e14.to_s` is
  `100000000000000.0`); `nil.inspect` inside arrays is `nil`.
- *fixture* Java `\s` is ASCII-only; Rust's `split_whitespace` also splits at U+00A0.
- Python's int-or-float results: `colorsys` keeps ints through `+`, `-` and `*`, so the faithful port
  computes with a small int-or-float type instead of `f64` (134 cases once ints were recorded).
- glibc `strtod("-")` converts nothing and returns `+0.0`, not `-0.0`.

Handled in the first port, and confirmed by the recorded cases:

- Java `Integer.parseInt` accepts any Unicode decimal digit (`١٢`, `２`) through `Character.digit`.
- Java `Double.toString` (JDK 19+) prints the shortest decimal, but never fewer than two digits:
  `4.9E-324`, where Rust's shortest form is `5e-324`.
- Python's `max(nan, 1, 2)` is `nan` (the first value wins unless a later one compares greater);
  `-0.0 % 1.0` is `0.0`.
- glibc `toupper(-128)` is `128` (its tables accept signed `char` values); `strtol` with base 0
  reads `077` as octal and `0x` without a hex digit as `0`.

### Limits these runs showed

- **C APIs that pass buffers.** aklomp/base64's public functions take `char *out, size_t *outlen`
  or a `struct base64_state *`. The C bridge carries scalars and `const char *` only, so it keeps
  them all in C, with the reason. Supporting the output-buffer pattern is the next step for C.
- **C macros.** 43 of base64's 51 files define functions through macros
  (`BASE64_ENC_FUNCTION(arch)`); the parser reports errors there and the plan marks those units
  partial. SIMD intrinsics are not called out as a risk yet.
- **Undefined behavior that does not crash.** `ispunct(1000)` reads past glibc's table and
  returns 1. No port should copy that; the one case was removed from `cases.jsonl` by hand. There
  is no built-in way to mark a case as undefined yet.
- **Dynamic typing costs code.** The `shellwords` port reimplements Ruby's `to_s` and `inspect`
  for numbers, nil, arrays and hashes, because `shellescape` accepts anything. The `colorsys`
  port needs Python's numeric tower. A typed port is shorter, but changes behavior for some
  callers; `diff` shows exactly which.
- **The facades.** The Java and C runs test real JDK and glibc behavior, but through facades
  written for the run, not through a library's own API.

## How each language runs

| | Record | Diff | Bridge | Shadow |
| --- | --- | --- | --- | --- |
| Go | generated driver, built as its own module with a `replace` to the original | the same driver over the cgo wrapper | typed cgo with a binary wire + package with the original API | package with the original API, runs both |
| Ruby | generated driver loads the original files | the same driver loads the magnus extension | magnus extension defining the same modules | file to require after the original: `prepend`s both |
| Java | reflection driver compiled with the original sources | the same driver compiled with the JNI wrapper classes | typed JNI with a binary wire + wrapper classes with the original static methods | `<Class>Shadow` classes |
| Python | in-process driver imports the original | the same driver imports the PyO3 package | PyO3 | module with the original API |
| C | `cc -shared` of the original, called through ctypes, one fork per case | ctypes over the Rust `cdylib` | `extern "C"` functions with the original names and a header | shadow `.c` with `-D` renames; the crate's `shadow` feature exports only `rust_anything_` names |

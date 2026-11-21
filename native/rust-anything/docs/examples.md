# Examples and case studies

Real modules ported with rust-anything, with the commands and the numbers. Back to the [README](../README.md).

## Case study: x18n's core

All 29 functions of x18n's core that can move without an API change were ported with rust-anything:
every gate passes and 7,925 recorded cases match exactly. `rust-anything bench` then showed the ports
are slower than the TS as called from JS, because the bridge costs more than the work. Full report:
[`demo/x18n-core`](../demo/x18n-core/README.md). The lesson is built in: measure with `bench` before
shipping a port.

## Real code in six languages

Go's `path`, Ruby's `shellwords`, Python's `colorsys`, JDK and glibc behavior behind small
facades: every port passes its gate, 13,010 recorded cases in all. The runs found and fixed
bugs in rust-anything (floats off by one ULP through `serde_json`, C crashes during recording,
untyped code getting garbage inputs). Called per function, the Ruby port runs 4.4x faster than
Ruby and the Python port 1.35x faster than CPython, while Go and Java leaf functions stay faster
in their own language; as batches, all of them win (see the table above).
Report: [`docs/real-world-languages.md`](real-world-languages.md).

## Example: a real module from x18n

`packages/core/src/fuzzy.ts` from [x18n](https://github.com/x18n-dev/x18n), ported both ways in
[`demo/`](../demo):

```sh
rust-anything analyze ../x18n                                      # fuzzy.ts: ready, wasm-bindgen (core runs in browsers)
rust-anything bridge ../x18n packages/core/src/fuzzy.ts --out demo/fuzzy-rs
rust-anything record demo/fuzzy-rs --count 2000
# write src/port.rs
rust-anything gate demo/fuzzy-rs
```

A first port that indexed by Rust `char` passed simple tests but failed `diff`:

```
198/200 cases match
- fuzzyScore("b😀", "a😀b😀c"): expected {"ok":1997}, got {"ok":1998}
```

JS `indexOf` counts UTF-16 code units, so an emoji is two positions. The port in
`demo/*/src/port.rs` uses the generated `utf16` helpers and passes 2000/2000 cases and the gate,
as napi-rs (`demo/fuzzy-napi`) and as wasm-bindgen (`demo/fuzzy-rs`).

The plan rust-anything writes for all of x18n: [`docs/examples/x18n.md`](examples/x18n.md).

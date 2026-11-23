<p align="center">
  <img src="docs/assets/logo.png" width="160" height="160" alt="The rust-anything crab">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub>English · <a href="docs/README_TR.md">Türkçe</a> · <a href="docs/README_UK.md">Українська</a> · <a href="docs/README_ZH.md">简体中文</a> · <a href="docs/README_ZH-TW.md">繁體中文</a> · <a href="docs/README_DE.md">Deutsch</a> · <a href="docs/README_FR.md">Français</a> · <a href="docs/README_RU.md">Русский</a> · <a href="docs/README_JA.md">日本語</a> · <a href="docs/README_KO.md">한국어</a> · <a href="docs/README_AZ.md">Azərbaycanca</a> · <a href="docs/README_UZ.md">Oʻzbekcha</a> · <a href="docs/README_KK.md">Қазақша</a> · <a href="docs/README_KY.md">Кыргызча</a> · <a href="docs/README_TK.md">Türkmençe</a> · <a href="docs/README_TT.md">Татарча</a> · <a href="docs/README_UG.md">ئۇيغۇرچە</a> · <a href="docs/README_PT.md">Português</a> · <a href="docs/README_VI.md">Tiếng Việt</a> · <a href="docs/README_ES.md">Español</a> · <a href="docs/README_IT.md">Italiano</a></sub></p>

<p align="center"><b>Rewrite your app in Rust, and prove it still behaves the same.</b><br>
Point it at your app: it records what the original does, your coding agent writes the Rust one piece at a time,<br>
and every piece is checked against that recording before it moves on. It doesn't translate code; it proves the rewrite.</p>

<p align="center">
  <img src="https://img.shields.io/github/languages/top/erkamyaman/rust-anything?logo=rust&label=Rust&color=CE422B" alt="Rust share of the code">
  <img src="https://img.shields.io/badge/macOS%20·%20Linux%20·%20Windows-native-2563EB" alt="Native binaries for macOS, Linux and Windows">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-555" alt="MIT license"></a>
  <a href="https://crates.io/crates/rust-anything"><img src="https://img.shields.io/crates/v/rust-anything?color=E2862A&label=crates.io" alt="crates.io version"></a>
  <a href="https://www.npmjs.com/package/rust-anything"><img src="https://img.shields.io/npm/v/rust-anything?color=E2862A&label=npm" alt="npm version"></a>
  <br>
  <a href="https://coderabbit.link/erkamyaman"><img src="https://img.shields.io/coderabbit/prs/github/erkamyaman/rust-anything?labelColor=171717&color=FF570A&label=Reviewed+by+CodeRabbit" alt="Pull requests reviewed by CodeRabbit"></a>
  <a href="https://github.com/rust-unofficial/awesome-rust#development-tools"><img src="https://awesome.re/mentioned-badge.svg" alt="Mentioned in Awesome Rust"></a>
  <a href="https://github.com/AIshihzong/awesome-reverse-engineer-anything"><img src="https://img.shields.io/badge/Featured%20in-Awesome%20REA-f0b429" alt="Featured in Awesome REA"></a>
  <a href="https://github.com/erkamyaman/rust-anything"><img src="https://img.shields.io/github/stars/erkamyaman/rust-anything?style=social" alt="GitHub stars"></a>
</p>

<p align="center"><a href="https://github.com/erkamyaman/rust-anything/discussions/127"><img src="https://img.shields.io/badge/Join%20the%20discussion-6E40C9?style=for-the-badge&logo=github&logoColor=white" alt="Join the discussion"></a></p>

<p align="center"><a href="https://trendshift.io/repositories/363468" target="_blank" rel="noopener noreferrer"><img src="https://trendshift.io/api/badge/trendshift/repositories/363468/daily?language=Rust" alt="erkamyaman/rust-anything: #1 Rust repository of the day on Trendshift" width="250" height="55"/></a></p>

<p align="center"><b><a href="#quick-start">Install</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Releases</a> · <a href="#quick-start">Claude Code plugin</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">MCP Registry</a> · <a href="docs/real-world.md">Real-world ports</a> · <a href="docs/faq.md#faq">FAQ</a><br>
<sub><a href="#languages">Languages</a> · <a href="docs/how-it-works.md#how-it-works">How it works</a> · <a href="docs/whole-apps.md#port-a-whole-app">Whole apps</a> · <a href="#sponsors">Sponsors</a> · <a href="LICENSE">MIT</a></sub><br>
Made in Türkiye 🇹🇷 by <a href="https://github.com/erkamyaman">erKam Yaman</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ Sponsor me</a> · Sponsored by <a href="https://coderabbit.link/erkamyaman"><img src="docs/assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">New in 2.0: <a href="docs/real-world.md#prettier">Prettier, ported to Rust</a>. 19x faster · 15,861 of 17,224 recorded runs fully in Rust · every run byte-identical</p>

<p align="center">
  <img src="docs/assets/prettier.gif" alt="Prettier and its Rust port format the same 59 real files, one process per file: Prettier takes 3.63 s, the Rust port 0.20 s, with the same output byte for byte." width="820">
  <br>
  <sub>Prettier against its Rust port on 59 real files from ms, Fuse.js and json-server (Apple M4). Same output, byte for byte. <a href="docs/real-world.md#prettier">How it was measured</a>.</sub>
</p>

<p align="center"><sub>PHP and C#: unit ports, through FFI and P/Invoke.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>Record</b><br>
      <code>record</code> runs the original on generated edge cases, semantic traps and your own inputs, and saves what it returns or throws.
    </td>
    <td width="25%" valign="top">
      <b>Port</b><br>
      <code>bridge</code> generates the Rust crate and a wrapper, so callers keep the original API. You or your agent write the Rust.
    </td>
    <td width="25%" valign="top">
      <b>Check</b><br>
      <code>diff</code> replays every recorded case against the Rust, and nothing ships until they all match and <code>gate</code> passes.
    </td>
    <td width="25%" valign="top">
      <b>Lift</b><br>
      When replay passes with no fallback and every unit passes gate, <code>app lift</code> makes it a pure Rust app. The original is never touched.
    </td>
  </tr>
</table>

**Rewrite your whole app in Rust, one piece at a time.** rust-anything records what your
Node.js (TypeScript/JavaScript), Python, C, Go, Ruby, Java or Kotlin app does today, then checks every piece of the
Rust rewrite against that recording before it moves on, until the app is pure Rust. CLIs, HTTP
servers and libraries; your coding agent (or you) writes the Rust. It is built on
[napi-rs](https://github.com/napi-rs/napi-rs), [PyO3](https://github.com/PyO3/pyo3) and
[wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen), which carry the calls between your
language and Rust; rust-anything decides what to move and proves it matches. It can only catch what the
recording covers: see [What it can't catch](docs/how-it-works.md#what-it-cant-catch). Whole tools
and servers, or only the slow part, one hot path at a time:

| Real project, same output as the original | Original | With rust-anything |
| --- | ---: | ---: |
| [Prettier](docs/real-world.md#prettier) 3.6.2, 2,360 JS/TS files from the command line | 128.6 s | **6.9 s: 19x faster** |
| Prettier, 6,305 Markdown files from the command line | 352 s | **22.5 s: 15.6x faster** |
| [json-server](docs/real-world.md#json-server), a whole Node server: memory at idle | 74 MB | **3.5 MB**, one 5.3 MB binary |
| json-server, from start to first response | 79 to 85 ms | **8 to 10 ms** |
| [Fuse.js](docs/real-world.md#fusejs) search over 50,000 strings, per query | 91.2 ms | **2.8 ms: 32x faster** |

Prettier: 15,861 of 17,224 recorded runs format fully in Rust across 10 languages (JavaScript,
TypeScript, JSX, JSON, GraphQL, YAML, Markdown, CSS, SCSS, Less), and all 17,224 match Prettier
byte for byte, since anything not ported yet falls back to Prettier. json-server: 514/514
recorded HTTP cases match on a pure Rust binary. Measured on an Apple M4, two runs each, every
output compared before timing ([details](docs/real-world.md)). Calling Rust once per small
function is usually *slower* than a JIT; rust-anything finds the loop worth moving, and moves it. Against
C the win is safety, not speed: [ffmpeg's libavutil](docs/real-world.md#ffmpeg) ported to Rust
matches all 226,447 recorded calls and runs at about C's speed (18 of 42 benchmarks even, 8
faster, 16 slower by 3 to 40%).

## Only the slow part

You don't have to port the whole app. Point `go` at a command that is too slow, and it ports
the one function that takes the time:

```sh
rust-anything go --agent claude -- python3 app.py
```

It profiles the run, picks the outermost function worth moving (the loop, not the small call
inside it, batched where the profile says so), records it on generated inputs plus the calls your
run makes, and hands `src/port.rs` to your agent (`claude`, `codex`, or any command that reads a
prompt on stdin; without `--agent` it writes `TASK.md` for you and stops). It switches the
function on only when every recorded case matches and `gate` passes. Then:

```sh
rust-anything report rust/dedupe-rs     # REPORT.md and card.svg for the pull request
rust-anything disable rust/dedupe-rs    # back to the original, byte for byte
```

The [Fuse.js](docs/real-world.md#fusejs) row above is this shape: Fuse's score for one string,
batched over the list, moved to Rust while object keys, extended search and the rest stay in
JavaScript, for 91.2 ms to 2.8 ms per query over 50,000 strings. Try `go` on
[`tests/fixtures/go-fuzzy`](tests/fixtures/go-fuzzy), a 20-line Python dedupe. Every way to switch
a port on and off: [Switch a port on and off](docs/how-it-works.md#switch-a-port-on-and-off).

<p align="center">
  <img src="docs/assets/json-server.gif" alt="json-server as a Node server and as one Rust binary: memory at idle 74 MB and 3.5 MB, start to first response 79 to 85 ms and 8 to 10 ms, and 514 of 514 recorded HTTP cases match." width="820">
  <br>
  <sub>json-server, a whole Node server, lifted to a pure Rust binary (Apple M4). <a href="docs/real-world.md#json-server">How it was measured</a>.</sub>
</p>

> rust-anything is an independent project, not affiliated with or endorsed by the Rust Project
> or the Rust Foundation. "Rust" is a trademark of the Rust Foundation.

> [!WARNING]
> `record` and `app record` run your original code, calling each function (or the whole app) on
> many generated inputs, and `app record` runs every case twice. `diff`, `gate` and `app replay`
> run the same cases again, on the port or the original. Code that writes files, talks to a
> database or calls the network does that once per case. Run them on a copy of the project, with a
> test database and no production credentials in the environment.

## Install the ports

The ESLint port is an npm command. It runs the Rust port for what it covers and ESLint 10.15.0,
pinned as a dependency, for everything else, so the output is ESLint's either way:

```sh
npm install -g eslint-rs     # ESLint 10.15.0
```

| Command | Answered in Rust | Handed to ESLint 10.15.0 |
| --- | --- | --- |
| `eslint-rs` | `--no-config-lookup --rule ... --stdin` with the 90 ported rules (`stylish` and `json` output, fixes, suggestions) | `eslint.config.*` lookups (`eslint .`), files and folders, the other 202 core rules, plugins, parsers, the other formatters, parse errors |

The binary comes for macOS (arm64, x64) and Linux (x64, arm64, glibc). On Windows and elsewhere,
or with `RUST_ANYTHING_MODE=old`, `eslint-rs` runs ESLint itself: same output, no speedup.
Installed from its tarballs on macOS arm64, `eslint-rs` matches ESLint 10.15.0 installed beside it
on all 10,585 recorded runs ([how](docs/packaging.md#a-whole-cli-front-door)).

The Prettier port is packaged the same way (`prettier-rs`, Prettier 3.6.2) and replays
17,224/17,224 recorded runs from its tarballs, 7.9% handed to Prettier, but it is not on npm.
`rust-anything package` makes this kind of package from any CLI front door.

## Quick start

Paste this into Claude Code, Codex, Cursor, Gemini CLI or OpenCode and let your agent install it:

```text
Install rust-anything: run `cargo install rust-anything`, then `rust-anything setup`.
Tell me when to restart you.
```

Or yourself:

```sh
cargo install rust-anything   # from crates.io
rust-anything setup           # registers it with Claude Code, Codex, Cursor, Gemini CLI and OpenCode
```

No Rust toolchain yet? Prebuilt binaries for macOS, Linux and Windows are attached to each
[GitHub release](https://github.com/erkamyaman/rust-anything/releases), and for JavaScript
projects `npm install -g rust-anything` installs the same binary.

In Claude Code you can also install it as a plugin (the MCP server and the skill; the CLI above
must be on your `PATH`):

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

In Codex, from the agents marketplace in this repo (`.agents/plugins/marketplace.json`, the same
plugin under `plugin/`, with the skill and MCP server):

```sh
codex plugin marketplace add erkamyaman/rust-anything
codex plugin add rust-anything@rust-anything
```

`setup` adds the MCP server to each agent it finds and installs the workflow skill where the agent
reads skills. It keeps your other settings, is safe to run again, and `--remove` undoes it.
Then ask your agent, for example:

- *"Rewrite this CLI in Rust with rust-anything, end to end, and prove it behaves the same."*
- *"This search feels slow. Profile it with rust-anything and move the hot loop to Rust."*
- *"Plan a migration of `src/parser/` to Rust with rust-anything; port the first unit and prove it."*
- *"Bench the Rust port in `rust/text-rs` against the original and tell me whether to ship it."*

### Works with

| Agent | `rust-anything setup` adds |
| --- | --- |
| Claude Code | MCP server in `~/.claude.json`, skill in `~/.claude/skills/` |
| Codex | MCP server in `~/.codex/config.toml`, skill in `~/.codex/skills/` |
| Cursor | MCP server in `~/.cursor/mcp.json` |
| Gemini CLI | MCP server in `~/.gemini/settings.json` |
| OpenCode | MCP server in `~/.config/opencode/opencode.json` (`mcp.servers`), skill in `~/.config/opencode/skills/` |
| Any other MCP client | add `rust-anything mcp` as a stdio server by hand (see [Install](docs/install.md#install)) |

Before changing a file that exists, `setup` saves a copy beside it (`<file>.rust-anything.bak`); `--no-backup` skips the copy.

To wire OpenCode up by hand, put the server in `~/.config/opencode/opencode.json` (`command` is
the whole argv) and copy [`skills/rust-anything/`](./skills/rust-anything) to
`~/.config/opencode/skills/rust-anything/`:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

Or run the steps yourself:

```sh
rust-anything profile . -- node bench.mjs              # what is hot, and who calls it
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# write src/port.rs (or let your agent), then:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # every recorded case must match
rust-anything fuzz rust/fuzzy-rs --time 60 --keep      # fresh inputs: what the recording missed
rust-anything bench rust/fuzzy-rs                      # faster? the verdict says
rust-anything gate rust/fuzzy-rs                       # done means this passes
```

rust-anything does not translate code: your coding agent (Claude Code, Codex, Cursor, OpenCode, ...) or you write
the Rust. rust-anything supplies what an agent cannot guess reliably: where the time goes, what to port
first and in which order, a bridge so the rest of the code calls the Rust with its API unchanged,
the exact behavior the port must match, a safe way to ship it, and a gate that says when a port is
done.

Same output is not enough. Record what the app sends too, and a rewrite that mails twice fails:

<p align="center">
  <img src="docs/assets/effects.gif" alt="app record with --effects smtp records one welcome email per signup; app replay of a rewrite that prints the same output but mails twice reports 0/2 cases passed, failing on effects, and the report shows the extra email." width="820">
  <br>
  <sub>A real run of <code>app record --effects</code> and <code>app replay</code>. See <a href="docs/whole-apps.md#side-effects-email-webhooks-and-databases">Side effects</a>.</sub>
</p>

## Languages

| Language | Unit | Bridge | Recorded and replayed by |
| --- | --- | --- | --- |
| TypeScript/JavaScript | folder or file | napi-rs or wasm-bindgen with a generated binary wire; classes and async included | Node, running the `.ts` sources |
| Python | package or module | PyO3 (abi3), direct conversion, typed exceptions | Python, importing the original |
| C | `.c` file | `extern "C"` functions with the original names and a header | the original compiled to a shared library, called through ctypes, one fork per case |
| Go | package | typed cgo calls and a generated binary wire, in a package with the original API | a generated Go driver, built as its own module |
| Ruby | file or folder of module functions | a magnus extension under the original modules | a generated Ruby driver |
| Java | package of `public static` methods | typed JNI and a generated binary wire, in wrapper classes with the original methods | a generated reflection driver |
| Kotlin | Gradle module of top-level functions and `@JvmStatic` methods | the Java bridge: typed JNI in Java wrapper classes calling the compiled Kotlin statics | a generated reflection driver, with the Kotlin sources compiled by `kotlinc` |
| PHP | file or folder of top-level functions (`public static` methods stay in PHP) | a JSON dispatcher over `ext-ffi`, in a file that redeclares the functions with the original parameters | a generated PHP driver; whole apps with `app record`, `app replay` and the front door, for CLIs and servers (`php -S`) |
| C# | file or folder of `public static` methods | a JSON dispatcher over P/Invoke, in `<namespace>.Rust` classes with the original methods | a generated reflection driver, compiled with the project's sources by `dotnet build`; whole apps with `app record`, `app replay` and the front door, for CLIs and ASP.NET servers (`dotnet <app>.dll`) |

Every language except C, PHP and C# also gets batch classes (`--batch`): the items cross once and stay in
Rust, and each call runs over all of them on all cores.

The language is detected from the repository (`go.mod`, `pom.xml`, `build.gradle`/`build.gradle.kts`,
a `.csproj` or `.sln`, `pyproject.toml`, `Gemfile`, `composer.json`, `package.json`, else the most
common source files); a Gradle module with `.kt` sources and no `.java` sources is Kotlin, mixed
modules stay Java. `--lang` overrides it. Types come from the source: TS types, Python hints, C, Go,
Java, Kotlin and C# signatures, Ruby YARD tags, PHP types and phpdoc; untyped parameters used as
numbers are inferred as numbers.

For Android, `rust-anything build <crate> --profile android` builds the same JNI library with the
NDK for `arm64-v8a`, `armeabi-v7a` and `x86_64` (or the `--abi` values you pass) and installs it
into `<crate>/jniLibs/<abi>/`; `--api` sets the API level (minimum 26, which the generated Java
runtime needs). It finds the NDK from `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT` or the SDK, needs the
Rust targets (`rustup target add aarch64-linux-android armv7-linux-androideabi
x86_64-linux-android`), links 64-bit libraries with 16 KB page alignment, and checks that every
JNI entry point is exported. Gradle looks in `src/main/jniLibs` by default, so pass
`--jni-libs app/src/main/jniLibs` or add `<crate>/jniLibs` to `sourceSets.main.jniLibs.srcDirs`.
`rust-anything record <crate> --profile android` runs the original Kotlin unit under ART on a
connected device (adb), dexes the generated driver with `d8` from the SDK's build-tools, and pulls
the results back as the same `cases.jsonl`; `diff <crate> --profile android` replays them against
the `.so` built for the device's ABI. Both need `adb`, a device, `kotlinc`, an SDK with build-tools
and a platform, and `--device <serial>` when more than one device is connected. The profile covers
logic units: Activity/Compose/XML UI, lifecycle and SDK-only classes stay in Kotlin, and each call
crosses JNI, so prefer the generated batch wrappers for hot loops. On a host, `record` and `diff`
keep using the desktop JVM.

For PHP, `analyze` plans a PHP app (Composer packages and PSR-4 autoloading, risks
such as loose comparison and integer overflow into floats, Composer packages mapped to crates),
and a PHP CLI or server can be recorded, replayed and put behind a Rust front door. A unit's
top-level functions with scalar, string, list and assoc-array parameters are bridged through PHP's
FFI extension (`php` with `ext-ffi`, on by default for the CLI): require the generated
`php/<crate>.php` in place of the original file once `diff` passes. Each call crosses as JSON, about
1 µs, so `bench` shows small functions slower than PHP; there is no `shadow` for PHP yet
([issue #200](https://github.com/erkamyaman/rust-anything/issues/200)).

C# works the same way: `analyze` reads `.csproj` projects and their NuGet packages
(mapped to crates), resolves `using` directives to folders, and flags what C# does differently
(`decimal`, `dynamic`, LINQ's deferred queries, `Task`, `lock`, reflection). A C# CLI or ASP.NET
server can be recorded, replayed and put behind a Rust front door. A unit's public static methods
with scalar, string, array, list, dictionary and record or plain-class parameters are bridged
through P/Invoke (`dotnet` on PATH to record and diff): call the generated `<namespace>.Rust`
classes, or alias them with `using`, once `diff` passes. Each call crosses as JSON, about 1-3 µs,
so `bench` shows small methods slower than .NET; there is no `shadow` for C# yet
([issue #200](https://github.com/erkamyaman/rust-anything/issues/200)).

## Learn more

- [How it works](docs/how-it-works.md): the loop, what `bridge` and `analyze` decide, hot paths, what it can't catch, what a real port takes, and keeping up with the original (`map`)
- [Ship a port as a package](docs/packaging.md): maturin wheels and napi-rs per-platform npm packages, with the CI that builds them
- [Port a whole app](docs/whole-apps.md): servers, CLIs and libraries, from the front door to `app lift`
- [Examples and case studies](docs/examples.md): x18n, and real code in eight languages
- [Real-world ports](docs/real-world.md): json-server, Fuse.js, vercel/ms, with benchmarks
- [Install, update and develop](docs/install.md): every install method, Windows, and building from source
- [FAQ](docs/faq.md) · [Status](docs/STATUS.md) · [MCP tools](docs/tools.md)

## Sponsors

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> is free and open source. If it helps your work, please consider sponsoring it.<br>
  Your support keeps development going.
</p>

<p align="center">
  <a href="https://coderabbit.link/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="docs/assets/sponsors/company-dark.svg">
      <img src="docs/assets/sponsors/company-light.svg" width="800" alt="Company sponsor: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="docs/assets/sponsors/sponsors-dark.svg">
      <img src="docs/assets/sponsors/sponsors-light.svg" width="800" alt="Sponsors: Santosh Yadav, Hakan Soytürk, Ali Yilmaz, Ashley Hunter, Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/Become_a_sponsor-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="Become a sponsor"></a>
  <a href="https://buymeacoffee.com/erkamyaman"><img src="https://img.shields.io/badge/Buy_me_a_coffee-%E2%98%95-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>
</p>

## Contributing

Issues and pull requests are welcome; [CONTRIBUTING.md](./CONTRIBUTING.md) covers the setup, where
tests go, and how to add a language. Design choices and their reasons are in
[DECISIONS.md](./DECISIONS.md). Case studies of real ports are especially welcome. Please report
security problems privately ([SECURITY.md](./SECURITY.md)), and see the
[code of conduct](./CODE_OF_CONDUCT.md).

## License

[MIT](./LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). Ports and generated code in your repository are yours.

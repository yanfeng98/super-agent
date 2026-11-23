<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="rust-anything 게">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · 한국어 · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a></sub></p>
<p align="center"><sub>이 번역은 커뮤니티에서 관리하며 <a href="../README.md">영어 README</a>보다 늦게 반영될 수 있습니다.</sub></p>

<p align="center"><b>앱을 Rust로 다시 작성하고, 여전히 똑같이 동작함을 증명하세요.</b><br>
코드를 자동 번역하지 않습니다. Rust는 코딩 에이전트(또는 여러분)가 작성하고, rust-anything이 원본과 대조해 검증합니다.</p>

<p align="center">
  <img src="https://img.shields.io/github/languages/top/erkamyaman/rust-anything?logo=rust&label=Rust&color=CE422B" alt="Rust share of the code">
  <img src="https://img.shields.io/badge/macOS%20·%20Linux%20·%20Windows-native-2563EB" alt="Native binaries for macOS, Linux and Windows">
  <a href="../LICENSE"><img src="https://img.shields.io/badge/license-MIT-555" alt="MIT license"></a>
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

<p align="center"><b><a href="#빠른-시작">빠른 시작</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Releases</a> · <a href="#빠른-시작">Claude Code plugin</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">MCP Registry</a> · <a href="real-world.md">실제 포팅 사례</a> · <a href="faq.md#faq">자주 묻는 질문</a><br>
<sub><a href="#지원-언어">지원 언어</a> · <a href="how-it-works.md#how-it-works">작동 방식</a> · <a href="whole-apps.md#port-a-whole-app">전체 앱 포팅하기</a> · <a href="../LICENSE">MIT</a></sub><br>
튀르키예 🇹🇷 에서 <a href="https://github.com/erkamyaman">erKam Yaman</a> 제작 · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ 후원하기</a> · 후원: <a href="https://coderabbit.link/erkamyaman"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">2.0의 새 기능: <a href="real-world.md#prettier">Prettier를 Rust로 포팅</a>. 19x 더 빠름 · 기록된 실행 17,224회 중 15,861회가 완전히 Rust · 모두 바이트 단위로 동일</p>

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier와 그 Rust 포팅 버전이 파일당 프로세스 하나로 같은 실제 파일 59개를 포맷하는 모습: Prettier는 3.63 s, Rust 포팅 버전은 0.20 s가 걸리며 출력은 바이트 단위로 동일합니다." width="820">
  <br>
  <sub>ms, Fuse.js, json-server의 실제 파일 59개에서 Prettier와 Rust 포팅 버전을 비교(Apple M4). 출력은 바이트 단위로 동일합니다. <a href="real-world.md#prettier">측정 방법</a>.</sub>
</p>

<p align="center"><sub>PHP와 C#: 현재는 기록, 재생, front door를 지원하며 유닛 포팅은 다음 단계입니다.</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>기록</b><br>
      <code>record</code>는 생성된 엣지 케이스, 의미상의 함정, 사용자가 준 입력으로 원본을 실행하고 반환값이나 발생한 예외를 저장합니다.
    </td>
    <td width="25%" valign="top">
      <b>포팅</b><br>
      <code>bridge</code>가 Rust crate와 래퍼를 생성하므로 호출하는 쪽은 기존 API를 그대로 사용합니다. Rust 코드는 개발자나 에이전트가 작성합니다.
    </td>
    <td width="25%" valign="top">
      <b>검증</b><br>
      <code>diff</code>가 기록된 모든 케이스를 Rust에 대해 재실행하며, 모두 일치하고 <code>gate</code>를 통과하기 전까지는 아무것도 배포되지 않습니다.
    </td>
    <td width="25%" valign="top">
      <b>리프트</b><br>
      폴백 없이 재실행이 통과하고 모든 유닛이 게이트를 통과하면, <code>app lift</code>가 앱을 순수 Rust 앱으로 만듭니다. 원본은 전혀 건드리지 않습니다.
    </td>
  </tr>
</table>

**앱 전체를 한 번에 한 조각씩 Rust로 다시 작성하세요.** rust-anything은 현재 Node.js(TypeScript/JavaScript), Python, C, Go, Ruby 또는 Java 앱이 실제로 어떻게 동작하는지 기록(record)한 뒤, 다음 단계로 넘어가기 전에 Rust로 재작성된 각 조각을 해당 기록과 대조 검증하며 앱이 순수 Rust가 될 때까지 진행합니다. CLI, HTTP 서버, 라이브러리 모두 가능하며, 실제 Rust 코드는 코딩 에이전트(또는 개발자 본인)가 작성합니다. 기록에 포함된 범위만 잡아낼 수 있으므로 [잡아내지 못하는 부분](how-it-works.md#what-it-cant-catch)을 확인하세요. 도구와 서버 전체를, 또는 느린 부분만 핫 패스(hot path) 하나씩 포팅할 수 있습니다:

| 실제 프로젝트 (기존 구현과 동일한 출력) | 기존 구현 | rust-anything 적용 후 |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2, 명령줄에서 JS/TS 파일 2,360개 | 128.6 s | **6.9 s: 19x 더 빠름** |
| Prettier, 명령줄에서 Markdown 파일 6,305개 | 352 s | **22.5 s: 15.6x 더 빠름** |
| [json-server](real-world.md#json-server), Node 서버 전체: 유휴 시 메모리 | 74 MB | **3.5 MB**, 5.3 MB 바이너리 하나 |
| json-server, 시작부터 첫 응답까지 | 79~85 ms | **8~10 ms** |
| 문자열 50,000개에 대한 [Fuse.js](real-world.md#fusejs) 검색, 쿼리당 | 91.2 ms | **2.8 ms: 32x 더 빠름** |

Prettier: 기록된 실행 17,224회 중 15,861회가 10개 언어(JavaScript, TypeScript, JSX, JSON, GraphQL, YAML, Markdown, CSS, SCSS, Less)에서 완전히 Rust로 포맷되며, 아직 포팅되지 않은 부분은 Prettier로 폴백하므로 17,224회 모두 Prettier와 바이트 단위로 일치합니다. json-server: 기록된 HTTP 케이스 514/514개가 순수 Rust 바이너리에서 일치합니다. Apple M4에서 각각 두 번 실행해 측정했으며, 시간을 재기 전에 모든 출력을 비교했습니다([자세히](real-world.md)). 작은 함수 하나를 호출할 때마다 Rust를 부르는 것은 보통 JIT보다 *더 느립니다*. rust-anything은 옮길 가치가 있는 루프를 찾아 그 루프 자체를 옮깁니다.

## 느린 부분만

앱 전체를 포팅할 필요는 없습니다. 너무 느린 명령에 `go`를 지정하면, 시간을 잡아먹는 함수 하나만 포팅합니다:

```sh
rust-anything go --agent claude -- python3 app.py
```

실행을 프로파일링해 옮길 가치가 있는 가장 바깥쪽 함수를 고르고(그 안의 작은 호출이 아니라 루프를, 프로파일이 그렇다고 하면 배치로), 생성된 입력과 실행 중에 일어나는 호출로 그 함수를 기록한 뒤, `src/port.rs`를 에이전트에게 넘깁니다(`claude`, `codex` 또는 stdin으로 프롬프트를 읽는 아무 명령; `--agent`가 없으면 `TASK.md`를 작성하고 멈춥니다). 기록된 모든 케이스가 일치하고 `gate`를 통과할 때만 함수를 켭니다. 그다음:

```sh
rust-anything report rust/dedupe-rs     # pull request용 REPORT.md와 card.svg
rust-anything disable rust/dedupe-rs    # 원본으로 되돌리기, 바이트 단위로 동일
```

위의 [Fuse.js](real-world.md#fusejs) 행이 바로 이 형태입니다. 문자열 하나에 대한 Fuse의 점수 계산을 목록 전체에 대해 배치로 Rust로 옮기고, 객체 키, 확장 검색 등 나머지는 JavaScript에 그대로 두어, 문자열 50,000개에서 쿼리당 91.2 ms가 2.8 ms로 줄었습니다. 20줄짜리 Python 중복 제거 코드인 [`tests/fixtures/go-fuzzy`](../tests/fixtures/go-fuzzy)에서 `go`를 실행해 보세요. 포팅을 켜고 끄는 모든 방법: [Switch a port on and off](how-it-works.md#switch-a-port-on-and-off).

<p align="center">
  <img src="assets/json-server.gif" alt="Node 서버로서의 json-server와 단일 Rust 바이너리로서의 json-server: 유휴 시 메모리 74 MB와 3.5 MB, 시작부터 첫 응답까지 79~85 ms와 8~10 ms, 기록된 HTTP 케이스 514개 중 514개 일치." width="820">
  <br>
  <sub>Node 서버 전체인 json-server를 순수 Rust 바이너리로 리프트(Apple M4). <a href="real-world.md#json-server">측정 방법</a>.</sub>
</p>

> rust-anything은 독립 프로젝트이며 Rust Project나 Rust Foundation과 제휴하거나 보증을 받지 않았습니다. "Rust"는 Rust Foundation의 상표입니다.

> [!WARNING]
> `record`와 `app record`는 원본 코드를 실행하며, 생성된 많은 입력으로 각 함수(또는 앱 전체)를 호출합니다.
> `app record`는 모든 케이스를 두 번 실행합니다. `diff`, `gate`, `app replay`는 같은 케이스를 포팅 결과나
> 원본에서 다시 실행합니다. 파일을 쓰거나, 데이터베이스와 통신하거나, 네트워크를 호출하는 코드는 케이스마다
> 그 일을 합니다. 프로젝트 복사본에서, 테스트 데이터베이스를 사용하고, 환경에 프로덕션 자격 증명이 없는 상태로 실행하세요.

## 빠른 시작

아래 문구를 Claude Code, Codex, Cursor, Gemini CLI 또는 OpenCode에 붙여 넣어 에이전트가 설치하도록 하세요:

```text
rust-anything을 설치해 주세요: `cargo install rust-anything`을 실행한 다음 `rust-anything setup`을 실행하세요.
언제 다시 시작하면 되는지 알려 주세요.
```

직접 설치하려면 다음 명령어를 실행하세요:

```sh
cargo install rust-anything   # crates.io에서 설치
rust-anything setup           # Claude Code, Codex, Cursor, Gemini CLI, OpenCode에 등록
```

아직 Rust 툴체인이 없으신가요? macOS, Linux, Windows용 사전 빌드 바이너리가 각 [GitHub 릴리스](https://github.com/erkamyaman/rust-anything/releases)에 첨부되어 있으며, JavaScript 프로젝트의 경우 `npm install -g rust-anything`으로 동일한 바이너리를 설치할 수 있습니다.

Claude Code에서는 플러그인(MCP 서버 및 스킬 포함, 위 CLI가 `PATH`에 있어야 함)으로도 설치할 수 있습니다:

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup`은 탐지된 각 에이전트에 MCP 서버를 추가하고 에이전트가 스킬을 읽는 위치에 워크플로 스킬을 설치합니다. 기존 설정은 유지되며, 여러 번 다시 실행해도 안전하고, `--remove`로 되돌릴 수 있습니다. 그런 다음 에이전트에게 다음과 같이 요청해 보세요:

- *"rust-anything으로 이 CLI를 처음부터 끝까지 Rust로 다시 작성하고, 동일하게 동작하는지 증명해 줘."*
- *"이 검색이 느린 것 같아. rust-anything으로 프로파일링해서 핫 루프를 Rust로 옮겨 줘."*
- *"rust-anything으로 `src/parser/`의 Rust 마이그레이션 계획을 세우고, 첫 번째 유닛을 포팅해서 검증해 줘."*
- *"`rust/text-rs`의 Rust 포팅 버전을 기존 구현과 벤치마킹해서 배포해도 될지 알려 줘."*

### 지원 에이전트

| 에이전트 | `rust-anything setup`이 추가하는 항목 |
| --- | --- |
| Claude Code | `~/.claude.json`의 MCP 서버, `~/.claude/skills/`의 스킬 |
| Codex | `~/.codex/config.toml`의 MCP 서버, `~/.codex/skills/`의 스킬 |
| Cursor | `~/.cursor/mcp.json`의 MCP 서버 |
| Gemini CLI | `~/.gemini/settings.json`의 MCP 서버 |
| OpenCode | `~/.config/opencode/opencode.json`의 MCP 서버(`mcp.servers`), `~/.config/opencode/skills/`의 스킬 |
| 기타 MCP 클라이언트 | `rust-anything mcp`를 stdio 서버로 직접 추가 ([설치](install.md#install) 참고) |

기존 파일을 수정하기 전에 `setup`은 같은 위치에 백업 복사본(`<file>.rust-anything.bak`)을 저장합니다.

OpenCode를 직접 연결하려면 `~/.config/opencode/opencode.json`에 서버를 넣고(`command`는 argv 전체입니다) [`skills/rust-anything/`](../skills/rust-anything)를 `~/.config/opencode/skills/rust-anything/`에 복사하세요:

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

각 단계를 직접 실행할 수도 있습니다:

```sh
rust-anything profile . -- node bench.mjs              # 어디가 병목이고 누가 호출하는지 확인
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# src/port.rs 작성 (또는 에이전트에게 맡기기), 그다음 실행:
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # 기록된 모든 케이스가 일치해야 함
rust-anything bench rust/fuzzy-rs                      # 더 빠른가? 판정 결과가 알려줌
rust-anything gate rust/fuzzy-rs                       # 이 게이트를 통과해야 완료
```

rust-anything은 코드를 자동 번역하지 않습니다. Rust 코드는 코딩 에이전트(Claude Code, Codex, Cursor, OpenCode 등)나 개발자 본인이 작성합니다. rust-anything이 제공하는 것은 에이전트가 감으로 맞히기 어려운 요소들입니다. 즉, 시간이 어디서 쓰이는지, 무엇을 어떤 순서로 먼저 포팅할지, 나머지 코드가 API 변경 없이 Rust를 호출할 수 있는 브리지, 포팅이 반드시 맞춰야 하는 정확한 동작, 안전한 배포 방법, 그리고 포팅 완료 여부를 판정하는 게이트를 제공합니다.

출력이 같은 것만으로는 충분하지 않습니다. 앱이 외부로 보내는 것까지 기록하면, 메일을 두 번 보내는 재작성은 실패합니다:

<p align="center">
  <img src="assets/effects.gif" alt="--effects smtp를 사용한 app record가 가입마다 환영 메일 한 통을 기록합니다. 같은 출력을 내지만 메일을 두 번 보내는 재작성에 대해 app replay는 0/2 케이스 통과, effects에서 실패라고 보고하고, 보고서에 추가로 보낸 메일이 표시됩니다." width="820">
  <br>
  <sub><code>app record --effects</code>와 <code>app replay</code>의 실제 실행. <a href="whole-apps.md#side-effects-email-webhooks-and-databases">부수 효과</a>를 참고하세요.</sub>
</p>

## 지원 언어

| 언어 | 유닛 | 브리지 | 기록 및 재실행 방식 |
| --- | --- | --- | --- |
| TypeScript/JavaScript | 폴더 또는 파일 | 생성된 바이너리 와이어를 사용하는 napi-rs 또는 wasm-bindgen; 클래스 및 async 포함 | Node가 `.ts` 소스를 직접 실행 |
| Python | 패키지 또는 모듈 | PyO3(abi3), 직접 변환, 타입이 지정된 예외 | Python이 기존 모듈을 import |
| C | `.c` 파일 | 기존 이름과 동일한 `extern "C"` 함수 및 헤더 | 기존 코드를 공유 라이브러리로 컴파일해 ctypes로 호출, 케이스마다 fork |
| Go | 패키지 | 기존 API를 유지하는 패키지 내의 타입 지정 cgo 호출 및 생성된 바이너리 와이어 | 별도 모듈로 빌드된 생성 Go 드라이버 |
| Ruby | 모듈 함수가 담긴 파일 또는 폴더 | 기존 모듈 하위의 magnus 확장 | 생성된 Ruby 드라이버 |
| Java | `public static` 메서드가 담긴 패키지 | 기존 메서드를 유지하는 래퍼 클래스 내의 타입 지정 JNI 및 생성된 바이너리 와이어 | 생성된 리플렉션 드라이버 |
| PHP | 함수 및 `public static` 메서드가 담긴 폴더 | 향후 지원 예정: C ABI 기반 PHP FFI | 현재 전체 앱 지원: CLI 및 서버(`php -S`)용 `app record`, `app replay`, 프론트 도어 |
| C# | `public static` 메서드가 담긴 폴더 | 향후 지원 예정: C ABI 기반 P/Invoke | 현재 전체 앱 지원: CLI 및 ASP.NET 서버(`dotnet <app>.dll`)용 `app record`, `app replay`, 프론트 도어 |

C를 제외한 모든 언어는 배치 클래스(`--batch`)도 지원합니다. 항목들이 한 번만 경계를 넘어 Rust 내부에 머무르며, 각 호출마다 모든 코어에서 전체 항목을 처리합니다.

언어는 저장소 파일(`go.mod`, `pom.xml`, `.csproj` 또는 `.sln`, `pyproject.toml`, `Gemfile`, `composer.json`, `package.json`, 그 외에는 가장 많은 소스 파일)로부터 자동 감지되며 `--lang`으로 재정의할 수 있습니다. 타입 정보는 소스에서 추출합니다(TS 타입, Python 힌트, C/Go/Java/C# 시그니처, Ruby YARD 태그, PHP 타입 및 phpdoc). 타입이 없는 매개변수라도 숫자로 사용되면 숫자로 추론됩니다.

PHP의 경우, `analyze`로 PHP 앱을 계획할 수 있고(Composer 패키지 및 PSR-4 오토로딩, 느슨한 비교나 정수의 float 오버플로 같은 위험 요소, Composer 패키지의 crate 매핑), PHP CLI나 서버를 기록·재실행하고 Rust 프론트 도어 뒤에 둘 수 있습니다. PHP FFI를 통해 유닛을 Rust로 포팅하는 작업이 다음 단계입니다([이슈 #2](https://github.com/erkamyaman/rust-anything/issues/2)).

C#도 같은 방식으로 동작합니다. `analyze`가 `.csproj` 프로젝트와 NuGet 패키지(crate로 매핑)를 읽고 `using` 지시문을 폴더로 해석하며, C#의 고유한 동작 차이(`decimal`, `dynamic`, LINQ 지연 쿼리, `Task`, `lock`, 리플렉션)를 표시합니다. C# CLI나 ASP.NET 서버를 기록·재실행하고 Rust 프론트 도어 뒤에 둘 수 있으며, P/Invoke를 통한 유닛 포팅이 다음으로 지원될 예정입니다.

## 더 알아보기

- [작동 방식](how-it-works.md): 핵심 루프, `bridge`와 `analyze`가 결정하는 사항, 핫 패스, 잡아내지 못하는 부분, 실제 포팅에 필요한 것, 원본 변경 사항 따라가기(`map`)
- [전체 앱 포팅하기](whole-apps.md): 서버, CLI, 라이브러리를 프론트 도어부터 `app lift`까지
- [예제와 사례 연구](examples.md): x18n, 그리고 6개 언어의 실제 코드
- [실제 포팅 사례](real-world.md): json-server, Fuse.js, vercel/ms와 벤치마크
- [설치, 업데이트, 개발](install.md): 모든 설치 방법, Windows, 소스에서 빌드하기
- [자주 묻는 질문](faq.md) · [현황](STATUS.md) · [MCP 도구](tools.md)

## 후원자

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a>은 무료 오픈 소스입니다. 작업에 도움이 되었다면 후원을 고려해 주세요.<br>
  여러분의 지원이 개발을 계속 이어가게 합니다.
</p>

<p align="center">
  <a href="https://coderabbit.link/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="기업 후원자: CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="후원자: Santosh Yadav, Hakan Soytürk, Ali Yilmaz, Ashley Hunter, Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/%ED%9B%84%EC%9B%90%ED%95%98%EA%B8%B0-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="후원하기"></a>
  <a href="https://buymeacoffee.com/erkamyaman"><img src="https://img.shields.io/badge/Buy_me_a_coffee-%E2%98%95-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>
</p>

## 기여하기

이슈와 풀 리퀘스트를 환영합니다. 설정 방법, 테스트 위치, 새 언어 추가 방법은 [CONTRIBUTING.md](../CONTRIBUTING.md)에서 다룹니다. 설계 선택과 그 이유는 [DECISIONS.md](../DECISIONS.md)에 있습니다. 실제 포팅 사례 연구는 특히 환영합니다. 보안 문제는 비공개로 제보해 주시고([SECURITY.md](../SECURITY.md)), [행동 강령](../CODE_OF_CONDUCT.md)을 참고해 주세요.

## 라이선스

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman). 저장소 내 포팅 결과물과 생성된 코드는 여러분의 것입니다.

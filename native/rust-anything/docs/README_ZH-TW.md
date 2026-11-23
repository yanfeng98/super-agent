<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="rust-anything 螃蟹">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · <a href="README_ZH.md">简体中文</a> · 繁體中文 · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a></sub></p>
<p align="center"><sub>本譯文由社群維護，可能落後於<a href="../README.md">英文 README</a>。</sub></p>

<p align="center"><b>用 Rust 重寫你的應用程式，並證明它的行為依然完全相同。</b><br>
它不會自動翻譯程式碼：Rust 由你的程式設計代理（或你自己）撰寫，rust-anything 負責對照原程式進行驗證。</p>

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

<p align="center"><b><a href="#快速開始">快速開始</a></b> · <a href="https://github.com/erkamyaman/rust-anything/releases">Releases</a> · <a href="#快速開始">Claude Code plugin</a> · <a href="https://registry.modelcontextprotocol.io/v0.1/servers?search=rust-anything">MCP Registry</a> · <a href="real-world.md">真實移植案例</a> · <a href="faq.md#faq">常見問題</a><br>
<sub><a href="#支援語言">支援語言</a> · <a href="how-it-works.md#how-it-works">運作原理</a> · <a href="whole-apps.md#port-a-whole-app">整個應用程式</a> · <a href="../LICENSE">MIT</a></sub><br>
由 <a href="https://github.com/erkamyaman">erKam Yaman</a> 在土耳其 🇹🇷 打造 · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ 贊助我</a> · 贊助商：<a href="https://coderabbit.link/erkamyaman"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center">2.0 新功能：<a href="real-world.md#prettier">Prettier 已移植到 Rust</a>。提速 19 倍 · 17,224 次錄製執行中 15,861 次完全由 Rust 執行 · 全部逐位元組一致</p>

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier 與它的 Rust 移植版格式化同樣的 59 個真實檔案，每個檔案一個行程：Prettier 耗時 3.63 s，Rust 移植版 0.20 s，輸出逐位元組相同。" width="820">
  <br>
  <sub>在取自 ms、Fuse.js 與 json-server 的 59 個真實檔案上，Prettier 對上它的 Rust 移植版（Apple M4）。輸出逐位元組相同。<a href="real-world.md#prettier">測量方式</a>。</sub>
</p>

<p align="center"><sub>PHP 與 C#：目前支援錄製、重播和 front door；單元移植即將推出。</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>錄製</b><br>
      <code>record</code> 會以自動產生的邊界案例、語意陷阱以及你自己的輸入執行原版程式，並儲存它回傳的值或擲出的例外。
    </td>
    <td width="25%" valign="top">
      <b>移植</b><br>
      <code>bridge</code> 會產生 Rust crate 與包裝層，讓呼叫端繼續使用原本的 API。Rust 程式碼由你或你的代理程式撰寫。
    </td>
    <td width="25%" valign="top">
      <b>檢查</b><br>
      <code>diff</code> 會對 Rust 重播每一筆錄製案例，在全部吻合且 <code>gate</code> 通過之前，什麼都不會上線。
    </td>
    <td width="25%" valign="top">
      <b>提升</b><br>
      當重播在沒有回退的情況下通過，且每個單元都通過 gate，<code>app lift</code> 就會把它變成純 Rust 應用程式。原版程式完全不會被更動。
    </td>
  </tr>
</table>

**用 Rust 逐塊重寫你的整個應用程式。** rust-anything 會先錄製你現有的
Node.js（TypeScript/JavaScript）、Python、C、Go、Ruby 或 Java 應用程式的行為，然後針對每一塊
Rust 重寫程式碼對照該錄製紀錄進行驗證，確認無誤後再繼續下一步，直到整個應用程式成為純 Rust。CLI、HTTP
伺服器與函式庫皆適用；由你的程式開發代理程式（或你本人）來撰寫 Rust 程式碼。它只能偵測到錄製內容所涵蓋的行為：請參閱[它偵測不到什麼](how-it-works.md#what-it-cant-catch)。可以移植整個工具與伺服器，
也可以只移植慢的部分，一次一條熱點路徑：

| 真實專案（輸出與原版完全一致） | 原版 | 使用 rust-anything |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2，從命令列格式化 2,360 個 JS/TS 檔案 | 128.6 s | **6.9 s：提速 19 倍** |
| Prettier，從命令列格式化 6,305 個 Markdown 檔案 | 352 s | **22.5 s：提速 15.6 倍** |
| [json-server](real-world.md#json-server)，完整的 Node 伺服器：閒置時記憶體用量 | 74 MB | **3.5 MB**，單一 5.3 MB 執行檔 |
| json-server，從啟動到第一個回應 | 79 至 85 ms | **8 至 10 ms** |
| [Fuse.js](real-world.md#fusejs) 在 50,000 個字串中搜尋，每次查詢 | 91.2 ms | **2.8 ms：提速 32 倍** |

Prettier：在 17,224 次錄製執行中，有 15,861 次完全由 Rust 完成格式化，涵蓋 10 種語言（JavaScript、
TypeScript、JSX、JSON、GraphQL、YAML、Markdown、CSS、SCSS、Less），且全部 17,224 次都與 Prettier
逐位元組相同，因為尚未移植的部分會回退給 Prettier 處理。json-server：514/514 筆錄製的 HTTP 案例
在純 Rust 執行檔上全部吻合。以上數據在 Apple M4 上測得，每項執行兩次，計時前先比對每一份輸出
（[詳細資訊](real-world.md)）。每個小函式呼叫一次 Rust 通常比 JIT 更*慢*；rust-anything 會找出
真正值得搬移的迴圈，並將整個迴圈移過去。

## 只移植慢的部分

你不必移植整個應用程式。把 `go` 指向一個太慢的指令，它會移植真正耗時的那一個函式：

```sh
rust-anything go --agent claude -- python3 app.py
```

它會對這次執行進行效能剖析，挑出值得搬移的最外層函式（是迴圈，而不是迴圈裡的小呼叫；剖析結果
顯示需要時還會批次處理），以自動產生的輸入以及你這次執行實際發出的呼叫來錄製它，然後把
`src/port.rs` 交給你的代理程式（`claude`、`codex`，或任何從 stdin 讀取提示詞的指令；不加
`--agent` 時，它會替你寫好 `TASK.md` 後停止）。只有每一筆錄製案例都吻合且 `gate` 通過，它才會
啟用這個函式。接著：

```sh
rust-anything report rust/dedupe-rs     # 供 pull request 使用的 REPORT.md 與 card.svg
rust-anything disable rust/dedupe-rs    # 回到原版程式，逐位元組相同
```

上表中的 [Fuse.js](real-world.md#fusejs) 那一列就是這種形式：Fuse 對單一字串的評分以批次方式套用到
整個清單並搬到 Rust，而物件鍵、延伸搜尋與其餘部分仍留在 JavaScript，在 50,000 個字串上每次查詢
從 91.2 ms 降到 2.8 ms。可以在 [`tests/fixtures/go-fuzzy`](../tests/fixtures/go-fuzzy)（一個 20 行
的 Python 去重程式）上試試 `go`。開啟與關閉移植的所有方式：[Switch a port on and off](how-it-works.md#switch-a-port-on-and-off)。

<p align="center">
  <img src="assets/json-server.gif" alt="json-server 作為 Node 伺服器與作為單一 Rust 執行檔的對比：閒置記憶體 74 MB 與 3.5 MB，從啟動到第一個回應 79 至 85 ms 與 8 至 10 ms，514 筆錄製的 HTTP 案例全部吻合。" width="820">
  <br>
  <sub>json-server，一個完整的 Node 伺服器，整個提升為純 Rust 執行檔（Apple M4）。<a href="real-world.md#json-server">測量方式</a>。</sub>
</p>

> rust-anything 是一個獨立專案，與 Rust Project 或 Rust Foundation 無關，亦未獲其背書。「Rust」為 Rust Foundation 之商標。

> [!WARNING]
> `record` 與 `app record` 會執行你的原版程式碼，以大量自動產生的輸入呼叫每個函式（或整個應用程式），
> 而 `app record` 會將每筆案例執行兩次。`diff`、`gate` 與 `app replay`
> 會在移植版或原版上再次執行相同的案例。會寫入檔案、存取資料庫或呼叫網路的程式碼，
> 每筆案例都會做一次這些事。請在專案的副本上執行它們，使用
> 測試資料庫，且環境中不要有正式環境的憑證。

## 快速開始

將這段話貼給 Claude Code、Codex、Cursor、Gemini CLI 或 OpenCode，讓你的代理程式自動安裝：

```text
安裝 rust-anything：執行 `cargo install rust-anything`，接著執行 `rust-anything setup`。
告訴我什麼時候需要重新啟動你。
```

或是自行安裝：

```sh
cargo install rust-anything   # 從 crates.io 安裝
rust-anything setup           # 向 Claude Code、Codex、Cursor、Gemini CLI 與 OpenCode 註冊
```

還沒有安裝 Rust 工具鏈？每個 [GitHub release](https://github.com/erkamyaman/rust-anything/releases) 都附帶了 macOS、Linux 與 Windows 的預先編譯執行檔；針對 JavaScript
專案，執行 `npm install -g rust-anything` 也能安裝相同的執行檔。

在 Claude Code 中你也可以將它安裝為外掛（包含 MCP 伺服器與工作流程技能；上述 CLI
必須位於你的 `PATH` 中）：

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup` 會將 MCP 伺服器加入它所偵測到的每個代理程式中，並將工作流程技能安裝至各代理程式讀取技能的目錄。它會保留你的其他設定、可安全重複執行，且可用 `--remove` 復原。
接著你就可以直接請你的代理程式協助，例如：

- *「用 rust-anything 將這個 CLI 端到端重寫為 Rust，並證明其行為完全相同。」*
- *「這個搜尋感覺很慢。用 rust-anything 做效能分析，並將熱點迴圈搬移到 Rust。」*
- *「用 rust-anything 規劃將 `src/parser/` 遷移至 Rust；先移植第一個單元並進行驗證。」*
- *「將 `rust/text-rs` 中的 Rust 移植版與原版進行基準測試比對，告訴我是否適合上線。」*

### 支援的代理程式

| 代理程式 | `rust-anything setup` 會加入的設定 |
| --- | --- |
| Claude Code | `~/.claude.json` 中的 MCP 伺服器、`~/.claude/skills/` 中的技能 |
| Codex | `~/.codex/config.toml` 中的 MCP 伺服器、`~/.codex/skills/` 中的技能 |
| Cursor | `~/.cursor/mcp.json` 中的 MCP 伺服器 |
| Gemini CLI | `~/.gemini/settings.json` 中的 MCP 伺服器 |
| OpenCode | `~/.config/opencode/opencode.json` 中的 MCP 伺服器（`mcp.servers`）、`~/.config/opencode/skills/` 中的技能 |
| 其他任意 MCP 用戶端 | 手動將 `rust-anything mcp` 新增為 stdio 伺服器（見[安裝](install.md#install)） |

修改任何已存在的設定檔前，`setup` 都會在同目錄旁留存一份備份（`<file>.rust-anything.bak`）。

若要手動設定 OpenCode，請將伺服器寫入 `~/.config/opencode/opencode.json`（`command` 是完整的
argv），並將 [`skills/rust-anything/`](../skills/rust-anything) 複製到
`~/.config/opencode/skills/rust-anything/`：

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

你也可以自行執行這些步驟：

```sh
rust-anything profile . -- node bench.mjs              # 找出耗時熱點以及誰呼叫了它
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# 撰寫 src/port.rs（或交由代理程式撰寫），接著執行：
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # 每筆錄製案例都必須吻合
rust-anything bench rust/fuzzy-rs                      # 真的更快嗎？讓評估結論說話
rust-anything gate rust/fuzzy-rs                       # 通過此門禁才算完成
```

rust-anything 並不直接翻譯程式碼：Rust 程式碼是由你的程式開發代理程式（Claude Code、Codex、Cursor、OpenCode 等）或你本人所撰寫。rust-anything 提供的是代理程式無法可靠猜測的資訊：時間花在哪裡、該先移植什麼
以及依何種順序移植、讓其餘程式碼無需改變 API 即可呼叫 Rust 的橋接層、
移植版必須完全吻合的精確行為、安全上線的方式，以及判定移植是否達標的門禁。

只有輸出相同還不夠。把應用程式對外送出的內容也錄製下來，寄了兩次信的重寫版本就會被判定失敗：

<p align="center">
  <img src="assets/effects.gif" alt="帶有 --effects smtp 的 app record 為每次註冊錄製一封歡迎信；對一個輸出相同但寄了兩次信的重寫版本執行 app replay，回報 0/2 筆案例通過，失敗原因是副作用不符，報告中列出了多寄的那封信。" width="820">
  <br>
  <sub><code>app record --effects</code> 與 <code>app replay</code> 的一次真實執行。請參閱<a href="whole-apps.md#side-effects-email-webhooks-and-databases">副作用</a>。</sub>
</p>

## 支援語言

| 語言 | 遷移單元 | 橋接層 | 錄製與重播方式 |
| --- | --- | --- | --- |
| TypeScript/JavaScript | 資料夾或單檔 | napi-rs 或 wasm-bindgen 搭配自動產生的二進位傳輸協定；支援類別與非同步 | Node，直接執行 `.ts` 原始碼 |
| Python | 套件或模組 | PyO3（abi3），直接型別轉換，支援型別化例外 | Python，匯入原版模組 |
| C | `.c` 檔案 | 與原函式同名的 `extern "C"` 函式與標頭檔 | 將原版編譯為共享函式庫，透過 ctypes 呼叫，每筆案例獨立 fork |
| Go | 套件（package） | 型別化 cgo 呼叫與自動產生的二進位傳輸協定，封裝於維持原 API 的套件內 | 自動產生的 Go 驅動程式，作為獨立模組建置執行 |
| Ruby | 包含模組函式的檔案或資料夾 | 掛載於原模組名稱下的 magnus 擴充套件 | 自動產生的 Ruby 驅動程式 |
| Java | 包含 `public static` 方法的套件 | 型別化 JNI 與自動產生的二進位傳輸協定，置於維持原方法的包裝類別中 | 自動產生的反射（reflection）驅動程式 |
| PHP | 包含函式與 `public static` 方法的資料夾 | 下一步推出：透過 C ABI 運作的 PHP FFI | 目前支援整套應用程式：`app record`、`app replay` 與前門代理，適用於 CLI 與伺服器（`php -S`） |
| C# | 包含 `public static` 方法的資料夾 | 下一步推出：透過 C ABI 運作的 P/Invoke | 目前支援整套應用程式：`app record`、`app replay` 與前門代理，適用於 CLI 與 ASP.NET 伺服器（`dotnet <app>.dll`） |

除了 C 語言外，每種語言皆可產生批次處理類別（`--batch`）：所有資料只需跨邊界傳入一次並留在
Rust 中，每次呼叫便會在所有 CPU 核心上平行運算全部項目。

系統會根據儲存庫內容自動偵測語言（依序檢查 `go.mod`、`pom.xml`、`.csproj` 或 `.sln`、
`pyproject.toml`、`Gemfile`、`composer.json`、`package.json`，否則依最常見的原始碼檔案判斷）；
也可以透過 `--lang` 手動覆寫。型別資訊直接來自原始碼：TS 型別、Python 型別提示、C、Go、Java 與 C#
函式簽章、Ruby YARD 標籤、PHP 型別與 phpdoc；未標註型別但當作數值使用的參數，會被自動推斷
為數值型別。

對於 PHP，`analyze` 可為 PHP 應用程式規劃遷移（支援 Composer 套件與 PSR-4 自動載入，偵測
寬鬆比較與整數溢位轉浮點數等風險，並將 Composer 套件對應至 crate），
且支援對 PHP CLI 或伺服器進行錄製、重播與套用 Rust 前門代理。透過 PHP FFI 將各個
單元移植到 Rust 是下一步規劃（[issue #2](https://github.com/erkamyaman/rust-anything/issues/2)）。

C# 的運作方式相同：`analyze` 會讀取 `.csproj` 專案與其 NuGet 套件
（對應至 crate）、將 `using` 指示詞解析至對應資料夾，並標記 C# 的特殊差異
（`decimal`、`dynamic`、LINQ 的延遲查詢、`Task`、`lock`、反射）。目前可對 C# CLI 或 ASP.NET
伺服器進行錄製、重播與套用 Rust 前門代理；透過 P/Invoke 移植個別單元則為下一步支援項目。

## 深入了解

- [運作原理](how-it-works.md)：核心流程、`bridge` 與 `analyze` 如何決策、熱點路徑、它偵測不到什麼、一次真正的移植需要哪些工作，以及如何跟上原版的更新（`map`）
- [遷移整個應用程式](whole-apps.md)：伺服器、CLI 與函式庫，從前門代理到 `app lift`
- [範例與案例研究](examples.md)：x18n，以及六種語言的真實程式碼
- [真實移植案例](real-world.md)：json-server、Fuse.js、vercel/ms，附基準測試
- [安裝、更新與開發](install.md)：所有安裝方式、Windows，以及從原始碼建置
- [常見問題](faq.md) · [專案狀態](STATUS.md) · [MCP 工具](tools.md)

## 贊助商

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> 是一款免費且開源的專案。如果它對你的工作有所幫助，歡迎考慮贊助支持。<br>
  你的支持能讓開發持續推進。
</p>

<p align="center">
  <a href="https://coderabbit.link/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="企業贊助商：CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="個人贊助商：Santosh Yadav、Hakan Soytürk、Ali Yilmaz、Ashley Hunter、Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/%E6%88%90%E7%82%BA%E8%B4%8A%E5%8A%A9%E5%95%86-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="成為贊助商"></a>
  <a href="https://buymeacoffee.com/erkamyaman"><img src="https://img.shields.io/badge/Buy_me_a_coffee-%E2%98%95-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>
</p>

## 參與貢獻

歡迎提交 Issue 與 Pull Request；[CONTRIBUTING.md](../CONTRIBUTING.md) 說明了開發環境設定、測試存放位置以及如何新增語言支援。設計決策與理由載於
[DECISIONS.md](../DECISIONS.md)。尤其歡迎真實移植專案的案例研究。若發現安全性問題請私下通報（[SECURITY.md](../SECURITY.md)），並請參閱
[行為準則](../CODE_OF_CONDUCT.md)。

## 授權條款

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman)。你在自己儲存庫中產生的移植程式碼與產物均歸你所有。

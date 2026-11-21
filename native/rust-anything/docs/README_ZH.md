<p align="center">
  <img src="assets/logo.png" width="160" height="160" alt="rust-anything 螃蟹">
</p>

<h1 align="center">rust-anything</h1>

<p align="center"><b>用 Rust 重写你的应用，并证明它的行为依然完全一致。</b></p>

<p align="center">2.0 新功能：<a href="real-world.md#prettier">Prettier 已移植到 Rust</a>。提速 19 倍 · 17,224 次录制运行中 15,837 次完全由 Rust 执行 · 全部逐字节一致</p>

<p align="center">
  <a href="https://crates.io/crates/rust-anything"><img src="https://img.shields.io/crates/v/rust-anything?color=E2862A&label=crates.io" alt="crates.io 版本"></a>
  <a href="https://www.npmjs.com/package/rust-anything"><img src="https://img.shields.io/npm/v/rust-anything?color=E2862A&label=npm" alt="npm 版本"></a>
  <a href="https://github.com/erkamyaman/rust-anything/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/erkamyaman/rust-anything/ci.yml?branch=main&label=tests" alt="测试"></a>
  <a href="https://github.com/erkamyaman/rust-anything"><img src="https://img.shields.io/github/stars/erkamyaman/rust-anything?style=social" alt="GitHub stars"></a>
  <a href="https://github.com/AIshihzong/awesome-reverse-engineer-anything"><img src="https://img.shields.io/badge/Featured%20in-Awesome%20REA-f0b429" alt="Featured in Awesome REA"></a>
</p>

<p align="center">
  <a href="#快速开始">快速开始</a> ·
  <a href="#支持语言">支持语言</a> ·
  <a href="real-world.md">真实项目迁移</a> ·
  <a href="how-it-works.md#how-it-works">工作原理</a> ·
  <a href="whole-apps.md#port-a-whole-app">整应用迁移</a> ·
  <a href="faq.md#faq">常见问题</a>
</p>

<p align="center">由 <a href="https://github.com/erkamyaman">erKam Yaman</a> 在土耳其 🇹🇷 打造 · <a href="../LICENSE">MIT</a> · <a href="https://github.com/sponsors/erkamyaman">❤️ 赞助我</a></p>

<p align="center">赞助商：<a href="https://coderabbit.ai"><img src="assets/brands/coderabbit.png" width="22" height="22" align="absmiddle" alt=""> <b>CodeRabbit</b></a></p>

<p align="center"><sub><a href="../README.md">English</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_UK.md">Українська</a> · 简体中文 · <a href="README_ZH-TW.md">繁體中文</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_RU.md">Русский</a> · <a href="README_JA.md">日本語</a> · <a href="README_KO.md">한국어</a> · <a href="README_AZ.md">Azərbaycanca</a> · <a href="README_UZ.md">Oʻzbekcha</a> · <a href="README_KK.md">Қазақша</a> · <a href="README_KY.md">Кыргызча</a> · <a href="README_TK.md">Türkmençe</a> · <a href="README_TT.md">Татарча</a> · <a href="README_UG.md">ئۇيغۇرچە</a> · <a href="README_PT.md">Português</a> · <a href="README_VI.md">Tiếng Việt</a></sub></p>

<p align="center"><sub>本译文由社区维护，可能落后于<a href="../README.md">英文 README</a>。</sub></p>

<table>
  <tr>
    <td width="25%" valign="top">
      <b>录制</b><br>
      <code>record</code> 在生成的边界用例、语义陷阱和你自己的输入上运行原实现，并保存它的返回值或抛出的异常。
    </td>
    <td width="25%" valign="top">
      <b>移植</b><br>
      <code>bridge</code> 生成 Rust crate 和一层包装，让调用方继续使用原有 API。Rust 代码由你或你的智能体编写。
    </td>
    <td width="25%" valign="top">
      <b>校验</b><br>
      <code>diff</code> 在 Rust 实现上回放每一条录制用例，在全部一致且 <code>gate</code> 通过之前，什么都不会上线。
    </td>
    <td width="25%" valign="top">
      <b>收口</b><br>
      当回放在零回退的情况下通过、且每个单元都通过门禁时，<code>app lift</code> 将其变为纯 Rust 应用。原实现始终不会被改动。
    </td>
  </tr>
</table>

**逐块将你的整个应用重写为 Rust。** rust-anything 会先录制你现有的 Node.js（TypeScript/JavaScript）、Python、C、Go、Ruby 或 Java 应用的行为，然后每重写一块 Rust 代码就对照录制结果进行校验，合格后再继续推进，直到整个应用变成纯 Rust。无论是 CLI、HTTP 服务端还是程序库都能适用，由你的编程智能体（或你本人）来编写 Rust 代码。它只能捕获录制用例覆盖到的行为，详见[它捕获不到什么](how-it-works.md#what-it-cant-catch)。既可以迁移整个工具和服务端，也可以只迁移慢的部分，一次一条热点路径：

| 真实项目（输出与原实现完全一致） | 原实现 | 使用 rust-anything |
| --- | ---: | ---: |
| [Prettier](real-world.md#prettier) 3.6.2，命令行格式化 2,360 个 JS/TS 文件 | 128.6 s | **6.9 s：提速 19 倍** |
| Prettier，命令行格式化 6,305 个 Markdown 文件 | 352 s | **22.5 s：提速 15.6 倍** |
| [json-server](real-world.md#json-server)，完整的 Node 服务端：空闲内存占用 | 74 MB | **3.5 MB**，单个 5.3 MB 二进制文件 |
| json-server，从启动到首次响应 | 79 至 85 ms | **8 至 10 ms** |
| [Fuse.js](real-world.md#fusejs) 在 50,000 个字符串中搜索，每次查询 | 91.2 ms | **2.8 ms：提速 32 倍** |

Prettier：在 17,224 次录制运行中，有 15,837 次完全由 Rust 完成格式化，覆盖 10 种语言（JavaScript、TypeScript、JSX、JSON、GraphQL、YAML、Markdown、CSS、SCSS、Less），并且全部 17,224 次都与 Prettier 逐字节一致，因为尚未移植的部分会回退到 Prettier。json-server：514/514 条录制的 HTTP 用例在纯 Rust 二进制文件上全部一致。以上数据在 Apple M4 上测得，每项运行两次，计时前先比对每一份输出（[详情](real-world.md)）。逐个小函数调用 Rust 通常比 JIT 更*慢*；rust-anything 会找出真正值得迁移的循环，再把整个循环搬过去。

<p align="center">
  <img src="assets/prettier.gif" alt="Prettier 与其 Rust 移植版格式化同样的 59 个真实文件，每个文件一个进程：Prettier 耗时 3.63 s，Rust 移植版 0.20 s，输出逐字节一致。" width="820">
  <br>
  <sub>在来自 ms、Fuse.js 和 json-server 的 59 个真实文件上，Prettier 对比其 Rust 移植版（Apple M4）。输出逐字节一致。<a href="real-world.md#prettier">测量方法</a>。</sub>
</p>

<p align="center">
  <img src="assets/json-server.gif" alt="json-server 作为 Node 服务端与作为单个 Rust 二进制文件对比：空闲内存 74 MB 与 3.5 MB，从启动到首次响应 79 至 85 ms 与 8 至 10 ms，514 条录制的 HTTP 用例全部一致。" width="820">
  <br>
  <sub>json-server，一个完整的 Node 服务端，被整体迁移为纯 Rust 二进制文件（Apple M4）。<a href="real-world.md#json-server">测量方法</a>。</sub>
</p>

> rust-anything 是一个独立项目，与 Rust Project 或 Rust Foundation 没有任何关联或背书关系。“Rust” 是 Rust Foundation 的商标。

> [!WARNING]
> `record` 和 `app record` 会运行你的原始代码，用大量生成的输入调用每个函数（或整个应用），并且 `app record` 会把每条用例运行两次。`diff`、`gate` 和 `app replay` 会在移植版或原实现上再次运行相同的用例。会写文件、访问数据库或调用网络的代码，每条用例都会执行一次这些操作。请在项目的副本上运行它们，使用测试数据库，并确保环境中没有生产环境凭据。

## 快速开始

将下面这段话粘贴给 Claude Code、Codex、Cursor、Gemini CLI 或 OpenCode，让你的智能体自动完成安装：

```text
安装 rust-anything：先运行 `cargo install rust-anything`，再运行 `rust-anything setup`。
完成后告诉我什么时候需要重启你。
```

或者自行执行安装：

```sh
cargo install rust-anything   # 从 crates.io 安装
rust-anything setup           # 自动注册到 Claude Code、Codex、Cursor、Gemini CLI 和 OpenCode
```

还没有安装 Rust 工具链？每个 [GitHub release](https://github.com/erkamyaman/rust-anything/releases) 都附带了 macOS、Linux 和 Windows 的预编译二进制文件；对于 JavaScript 项目，也可以直接运行 `npm install -g rust-anything` 安装相同的二进制文件。

在 Claude Code 中还可以将其作为插件安装（包含 MCP 服务器和工作流技能，需确保上述 CLI 已加入 `PATH`）：

```text
/plugin install rust-anything --marketplace erkamyaman/rust-anything
```

`setup` 会把 MCP 服务器添加到它检测到的每一个智能体中，并把工作流技能安装到对应智能体读取技能的目录。它会保留你的其他配置，可以安全地重复运行，`--remove` 则会撤销这些改动。之后你就可以直接向智能体提需求，例如：

- *"使用 rust-anything 将这个 CLI 端到端重写为 Rust，并证明其行为完全一致。"*
- *"这个搜索感觉很慢。用 rust-anything 做性能分析，把热点循环迁移到 Rust。"*
- *"用 rust-anything 规划 `src/parser/` 向 Rust 的迁移；先迁移第一个单元并验证。"*
- *"将 `rust/text-rs` 中的 Rust 移植版与原实现做基准对比，告诉我是否值得上线。"*

### 适配工具

| 智能体 | `rust-anything setup` 会写入的配置 |
| --- | --- |
| Claude Code | `~/.claude.json` 中的 MCP 服务器、`~/.claude/skills/` 中的技能 |
| Codex | `~/.codex/config.toml` 中的 MCP 服务器、`~/.codex/skills/` 中的技能 |
| Cursor | `~/.cursor/mcp.json` 中的 MCP 服务器 |
| Gemini CLI | `~/.gemini/settings.json` 中的 MCP 服务器 |
| OpenCode | `~/.config/opencode/opencode.json` 中的 MCP 服务器（`mcp.servers`）、`~/.config/opencode/skills/` 中的技能 |
| 其他任意 MCP 客户端 | 手动将 `rust-anything mcp` 添加为 stdio 服务器（见[安装](install.md#install)） |

修改任何已有配置文件前，`setup` 都会在同目录下保存一份备份（`<file>.rust-anything.bak`）。

如需手动接入 OpenCode，将服务器写入 `~/.config/opencode/opencode.json`（`command` 是完整的 argv），并把 [`skills/rust-anything/`](../skills/rust-anything) 复制到 `~/.config/opencode/skills/rust-anything/`：

```json
{
  "mcp": {
    "servers": {
      "rust-anything": { "type": "local", "command": ["rust-anything", "mcp"] }
    }
  }
}
```

你也可以手动执行每一步：

```sh
rust-anything profile . -- node bench.mjs              # 找出热点函数及其调用方
rust-anything bridge . src/fuzzy.ts --batch fuzzyScore:text
# 编写 src/port.rs（或交给智能体编写），然后运行：
rust-anything record rust/fuzzy-rs && rust-anything build rust/fuzzy-rs
rust-anything diff rust/fuzzy-rs                       # 每条录制用例都必须一致
rust-anything bench rust/fuzzy-rs                      # 是否更快？由基准测试结论说了算
rust-anything gate rust/fuzzy-rs                       # 通过该门禁才算完成
```

rust-anything 并不负责自动翻译代码：具体 Rust 代码由你的编程智能体（Claude Code、Codex、Cursor、OpenCode 等）或你本人编写。rust-anything 提供的是智能体难以靠猜测稳妥获得的东西：时间花在哪里、先迁移什么以及按什么顺序迁移、让其余代码无需改动 API 就能调用 Rust 的桥接层、移植代码必须匹配的精确行为、安全的上线方式，以及判断迁移是否达标的门禁。

仅输出一致还不够。把应用对外发送的内容也录制下来，这样发了两次邮件的重写版本就会被判定失败：

<p align="center">
  <img src="assets/effects.gif" alt="带 --effects smtp 的 app record 为每次注册录制一封欢迎邮件；对一个输出相同但发送了两次邮件的重写版本运行 app replay，报告 0/2 条用例通过，失败原因是副作用不一致，报告中列出了多发的那封邮件。" width="820">
  <br>
  <sub><code>app record --effects</code> 与 <code>app replay</code> 的一次真实运行。参见<a href="whole-apps.md#side-effects-email-webhooks-and-databases">副作用</a>。</sub>
</p>

## 支持语言

| 语言 | 迁移单元 | 桥接层 | 录制与回放方式 |
| --- | --- | --- | --- |
| TypeScript/JavaScript | 文件夹或单文件 | napi-rs 或 wasm-bindgen，配合生成的二进制传输协议；支持类和异步 | Node 直接运行 `.ts` 源码 |
| Python | 包或模块 | PyO3（abi3），直接类型转换，保留类型化异常 | Python 直接导入原模块 |
| C | `.c` 文件 | 与原函数同名的 `extern "C"` 函数及头文件 | 将原代码编译为共享库，通过 ctypes 调用，每条用例独立 fork |
| Go | 包（package） | 类型化 cgo 调用与生成的二进制传输协议，封装为保持原 API 的包 | 生成独立的 Go 驱动模块进行构建运行 |
| Ruby | 包含模块函数的文件或文件夹 | 挂载在原模块名下的 magnus 扩展 | 生成的 Ruby 驱动 |
| Java | 包含 `public static` 方法的包 | 类型化 JNI 与生成的二进制传输协议，封装为保持原方法的包装类 | 生成的反射驱动 |
| PHP | 包含函数和 `public static` 方法的文件夹 | 下一步规划：基于 C ABI 的 PHP FFI | 现已支持整应用级工作流：`app record`、`app replay` 和前门代理，适用于 CLI 与服务端（`php -S`） |
| C# | 包含 `public static` 方法的文件夹 | 下一步规划：基于 C ABI 的 P/Invoke | 现已支持整应用级工作流：`app record`、`app replay` 和前门代理，适用于 CLI 与 ASP.NET 服务端（`dotnet <app>.dll`） |

除 C 语言外，每种语言都支持生成批处理类（`--batch`）：数据只需跨边界传入一次并常驻 Rust 侧，每次调用即可在全部 CPU 核心上并行处理所有条目。

语言会根据代码库自动检测（依次检查 `go.mod`、`pom.xml`、`.csproj` 或 `.sln`、`pyproject.toml`、`Gemfile`、`composer.json`、`package.json`，否则统计最常见的源码文件）；也可以通过 `--lang` 手动指定。类型信息直接从源码提取：TS 类型、Python 类型注解、C/Go/Java/C# 函数签名、Ruby YARD 标签、PHP 类型与 phpdoc；对于未标注类型但按数值使用的参数，会自动推断为数值类型。

对于 PHP，`analyze` 可为 PHP 应用生成迁移规划（支持 Composer 包与 PSR-4 自动加载、识别松散比较和整数溢出转浮点等风险、将 Composer 包映射为 Rust crate），并且支持对 PHP CLI 或服务端进行录制、回放与 Rust 前门接管。通过 PHP FFI 将单元逐个迁入 Rust 是下一步计划（[issue #2](https://github.com/erkamyaman/rust-anything/issues/2)）。

C# 的支持方式相同：`analyze` 会读取 `.csproj` 工程及其 NuGet 包（映射为 crate），将 `using` 指令解析到对应目录，并标记 C# 特有的语义差异（`decimal`、`dynamic`、LINQ 延迟查询、`Task`、`lock`、反射）。现已支持对 C# CLI 或 ASP.NET 服务端进行录制、回放与 Rust 前门接管；通过 P/Invoke 迁移单个单元将在后续支持。

## 了解更多

- [工作原理](how-it-works.md)：核心闭环、`bridge` 和 `analyze` 如何做决定、热点路径、它捕获不到什么、一次真实迁移需要做哪些工作，以及如何跟进原项目的更新（`map`）
- [迁移整个应用](whole-apps.md)：服务端、CLI 和程序库，从前门到 `app lift`
- [示例与案例研究](examples.md)：x18n，以及六种语言的真实代码
- [真实项目迁移](real-world.md)：json-server、Fuse.js、vercel/ms，附基准测试
- [安装、更新与开发](install.md)：所有安装方式、Windows，以及从源码构建
- [常见问题](faq.md) · [项目状态](STATUS.md) · [MCP 工具](tools.md)

## 赞助商

<p align="center">
  <a href="https://github.com/erkamyaman/rust-anything">rust-anything</a> 是一款免费开源项目。如果它对你的工作有所帮助，欢迎考虑赞助支持。<br>
  你的支持将助力项目持续开发。
</p>

<p align="center">
  <a href="https://coderabbit.ai">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/company-dark.svg">
      <img src="assets/sponsors/company-light.svg" width="800" alt="企业赞助商：CodeRabbit">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/sponsors/sponsors-dark.svg">
      <img src="assets/sponsors/sponsors-light.svg" width="800" alt="赞助商：Santosh Yadav、Hakan Soytürk、Ali Yilmaz、Ashley Hunter、Fatih Uzuntaş">
    </picture>
  </a>
</p>

<p align="center">
  <a href="https://github.com/sponsors/erkamyaman"><img src="https://img.shields.io/badge/%E6%88%90%E4%B8%BA%E8%B5%9E%E5%8A%A9%E5%95%86-%E2%9D%A4-db61a2?style=for-the-badge&logo=githubsponsors&logoColor=white" alt="成为赞助商"></a>
</p>

## 参与贡献

欢迎提交 Issue 和 Pull Request；[CONTRIBUTING.md](../CONTRIBUTING.md) 说明了开发环境搭建、测试存放位置以及如何新增语言支持。设计决策与原因见 [DECISIONS.md](../DECISIONS.md)。特别欢迎提交真实项目的迁移案例研究。安全漏洞请私下报告（[SECURITY.md](../SECURITY.md)），并请遵守[行为准则](../CODE_OF_CONDUCT.md)。

## 许可证

[MIT](../LICENSE) © [Erkam Yaman](https://github.com/erkamyaman)。你在自己仓库中生成的移植代码与产物均归你所有。

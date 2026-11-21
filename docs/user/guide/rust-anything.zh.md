# 使用 rust-anything 把代码迁移到 Rust

[English](rust-anything.md) | 中文

Web 的 `standard` agent preset 挂载了 [rust-anything](https://github.com/erkamyaman/rust-anything) MCP server，默认会话因此可以按单元把 TypeScript/JavaScript、Python、C、Go、Ruby、Java、Kotlin 代码改写成 Rust。每个单元都会对照"原代码返回什么"的录制做校验，`gate` 不通过就不算完成。它不需要全局安装：CLI 由仓库里 vendored 的源码现场编译。

## 内置能力

| 阶段 | 工具 |
|---|---|
| 规划 | `analyze`、`map`、`profile` |
| 迁移单个单元 | `bridge`、`refresh`、`record`、`build`、`diff`、`bench`、`shadow`、`gate` |
| 迁移整个应用 | `app_record`、`app_replay`、`app_front_door`、`app_wire`、`app_lift` |

## 首次使用前

`pnpm install && pnpm run build` 会编译 CLI 并暂存为 `native/rust-anything/bin/rust-anything`。构建需要 Rust 1.97：crate 在 `native/rust-anything/rust-toolchain.toml` 里固定了该 channel，rustup 会按需安装。`pnpm dsh web` 不会重新构建，所以构建一次（改动 vendored 源码后再构建一次）并重启服务；已经打开的会话仍使用启动时的工具列表。

## 提出迁移需求

描述任务即可，匹配的[技能](../../../.agents/skills/rust-anything/SKILL.md)会自动加载，例如：

- 「给这个仓库做一份迁移到 Rust 的计划，按顺序列出单元。」
- 「把 `src/parser/` 里的解析器迁到 Rust，一次一个函数。」
- 「这个循环是瓶颈：先 profile，只把热点部分搬过去。」
- 「把这个 CLI 重写成 Rust，并证明它与录制行为一致。」

流程是 `analyze`、`bridge`、`record`、写 `src/port.rs`、`build`、`diff`、`bench`、`gate`；整应用走 `app_*`。想确认挂载是否生效，可以让 agent 列出以 `mcp__rust-anything__` 开头的工具。

## 产物落在哪里

每个单元在 `rust/<unit>-rs/` 下得到一个 crate 及其 `src/port.rs`，录制结果以 `cases.jsonl` 存在旁边，`map --write` 生成 `rust/PORT_MAP.md`，`shadow` 生成一个让原实现与移植版并排运行的 drop-in 模块。

## 直接用命令行

```sh
./native/rust-anything/bin/rust-anything analyze <repository>
./native/rust-anything/bin/rust-anything map <repository> --write
./native/rust-anything/bin/rust-anything doctor
```

`build`、`diff`、`gate`、`bench` 请传绝对 crate 路径：2.1.0 会把子进程切到临时目录，相对 `--manifest-path` 会失败。

## 调整行为

| 字段 | 默认值 | 作用 |
|---|---|---|
| `toolLoading` | `eager` | 设为 `on-demand` 时用一段系统提示目录加一个 `mcp__rust-anything__load_tools` 工具替代 16 个 schema |
| `command` | 仓库内构建产物 | 覆盖要启动的可执行文件；环境变量 `RUST_ANYTHING_BIN` 是次级回退 |
| `toolCallTimeoutMs` | 15 分钟 | 单次调用预算；`gate`、`bench`、`app record` 会编译 crate 并回放语料 |
| `env` | `{}` | 提供给 server 的额外环境变量，例如 `RUSTUP_TOOLCHAIN` |

改 `packages/bundle/web-app/presets/standard.patch.yml` 里的 `mcp-rust-anything` 行即可。

## 成本与限制

- 16 个工具 schema 加 3 个共享 MCP 资源工具，每个 agent 约 7k 请求 token。
- 每个挂载该行的会话会各自启动一个 server 进程。
- 只有 Web 与 Desktop 的 `standard` preset 挂载它；`minimal`、`ptc` 以及 headless、ACP、SDK profile 都不挂。
- 打包的 Desktop 应用与 Python SDK 可执行文件只装了插件、没带 CLI，因此那里不会出现这些工具。
- 生成的 crate 声明 `rust-version = "1.97"`，请用该版本或更新的工具链构建。
- 工具会执行被录制的代码并往工作区写 crate：把一次运行当作以 agent 的环境执行那段代码。

## 出问题怎么办

- 看不到 `mcp__rust-anything__*` 工具：先 `pnpm run build`，再重启 `dsh web`，并确认 `native/rust-anything/bin/rust-anything` 存在。
- 构建报 `rustc 1.97 ... not supported`：用 `rustup toolchain install 1.97.0` 安装该工具链，或把它设为默认。
- 找不到 `cargo`：先安装 rustup，再重新构建。
- 其它情况：`rust-anything doctor` 会报告各语言运行时、server 状态以及对应的修复命令。

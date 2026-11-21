---
description: "挂载 rust-anything MCP server，让 Agent 按单元把 TypeScript/JavaScript、Python、C、Go、Ruby、Java、Kotlin 代码迁移到 Rust 并逐单元验证。"
kind: "package-reference"
---

# @deepseek-ai/dsh-rust-anything

[English](README.md) | 中文

## 概述

把 Agent 接到 [rust-anything](https://github.com/erkamyaman/rust-anything)：一个迁移夹具——先录制原代码的行为，再用这份录制逐个单元校验 Rust 重写，`gate` 不通过就不算完成。CLI 由 `pnpm run build` 从 vendored 源码 [`native/rust-anything`](../../../native/rust-anything/NATIVE.md) 编译，检出无需全局安装、无需已发布的二进制、也不需要按机器改配置。Web bundle 把它挂在 `standard` 这个 agent preset 上。

## 目录

- [使用本包](#use-this-package)
- [实现说明](#understand-the-implementation)
- [延伸阅读](#further-exploration)
- [模型体验](#model-experience)
- [已知限制与待办](#known-limitations-and-deferred-work)
- [开发备注](#dev-note)

-----

<a id="use-this-package"></a>
## 使用本包

由 Web 的 `standard` agent preset 以 `mcp-rust-anything` 行挂载，该 preset 的 agent 直接拿到 `mcp__rust-anything__*` 工具，无需任何安装步骤。"把这个 CLI 改写成 Rust""把这个热点循环搬到 Rust""给 `src/parser/` 做迁移计划"这类任务会命中对应的 [`dsh-rust-anything`](../../../.agents/skills/dsh-rust-anything/SKILL.md) 技能，按 `analyze`、`bridge`、`record`、写 `port.rs`、`build`、`diff`、`bench`、`gate`（整应用走 `app_*`）推进。

| 字段 | 默认值 | 含义 |
|---|---|---|
| `command` | 仓库内编译产物 | 覆盖 `native/rust-anything/bin/rust-anything`（`pnpm run build` 从 vendored crate 编译并暂存的位置）；环境变量 `RUST_ANYTHING_BIN` 是次级回退。 |
| `toolLoading` | `eager` | 设为 `on-demand` 时 16 个工具只进系统提示目录，由 Agent 按需加载。 |
| `toolCallTimeoutMs` | 15 分钟 | `gate`、`build`、`bench`、`app record` 会编译 crate 并回放语料，MCP 默认的 60 秒不够。 |
| `startupTimeoutMs` | 20 秒 | stdio 探测、连接与首次工具同步的预算。 |
| `env` | `{}` | 提供给 server 进程的额外环境变量，例如给生成 crate 用的 `RUSTUP_TOOLCHAIN`。 |

CLI 作为普通子进程运行：它会往仓库写 `rust/<unit>-rs` crate，并执行被录制的代码。它不受 shell 沙箱限制，请只在副本上使用，并避免把生产凭据放进环境。

-----

<a id="understand-the-implementation"></a>
## 实现说明

<details>
<summary>开发者参考：解析与挂载</summary>

`vendoredBinary(platform)` 从本包自己的 `import.meta.url` 解析出 `native/rust-anything/bin/rust-anything[.exe]`；[`scripts/build-rust-anything.ts`](../../../scripts/build-rust-anything.ts) 负责编译 vendored crate 并暂存该文件，`pnpm run build` 以 `rust-anything CLI` gate 调用它；`resolveCommand` 依次取显式 `command`、`RUST_ANYTHING_BIN`、该路径；`apply` 用 `serverName: 'rust-anything'`、`args: ['mcp']` 和配置的超时挂载一个 `@deepseek-ai/dsh-mcp-client` 实例。该 client 是 peer 依赖：由宿主组合提供，发布安装不会带上第二份插件副本。

</details>

-----

<a id="further-exploration"></a>
## 延伸阅读

- [rust-anything](https://github.com/erkamyaman/rust-anything)：夹具本身、设计决策，以及录制覆盖不到的行为。
- [`@deepseek-ai/dsh-mcp-client`](../../mcp/mcp-client/README.zh.md)：工具命名、重连与按需加载。

-----

<a id="model-experience"></a>
## 模型体验

### 来自 server 的工具

#### 模型看到什么

`toolLoading: eager` 时 16 个工具以 `mcp__rust-anything__*` 出现在该 agent 的工具列表里；描述与 MCP server 一致。server 的 initialize 结果带 instructions，因此系统提示会多出 `### MCP server: rust-anything` 一段，共享的 MCP 资源工具也会在自己的段落里列出该 server。

#### Token 影响

挂载期间 16 个 schema 约 21 KB（约 6k tokens）固定请求上下文，另加 server 的 instruction 段（几百字节），以及 server 注册后出现的 3 个共享资源工具。`on-demand` 用一段系统提示目录加一个 `mcp__rust-anything__load_tools` schema 取代这 16 个 schema。

#### KV Cache 影响

挂载发生在首轮之前、会话中不变，不会让已热的 prefix 失效；在同一 Agent 上切换 `eager`/`on-demand` 会。

## 已知限制与待办

<a id="known-limitations-and-deferred-work"></a>

- 插件挂在 agent preset 内，因此每个使用它的 agent 会启动自己的 server 进程。
- 可执行文件是仓库构建产物：启动 Web 前先跑 `pnpm run build`（或 `pnpm run build:rust-anything`），改动 vendored crate 后需要重跑。它不从 npm 下载，因此本包只服务本检出，不适用于发布安装。
- 打包运行时无法带上它：可执行文件在所有包目录之外，所以 Desktop 应用与 Python SDK 可执行文件只装了插件、没有 CLI，那里不会出现 rust-anything 工具。要让它们可用，需要像 `native/system` 那样按平台暂存二进制。
- 编译需要 Rust 1.97：`native/rust-anything/rust-toolchain.toml` 固定该 channel，rustup 会在缺失时自动安装；没有 cargo 时构建会给出可操作的报错。
- 生成的 crate 声明 `rust-version = "1.97"`，默认工具链更旧会在构建时失败；`rust-anything doctor` 会指出。
- 上游 2.1.0 在**相对** crate 路径下 `build`、`diff`、`gate`、`bench` 会失败（`run` 把子进程切到临时目录，却传下相对 `--manifest-path`）。请传绝对路径；技能里已写明。
- 工具会执行被录制的代码并往仓库写 crate，等同于以 Agent 的完整环境执行代码，请谨慎。

<a id="dev-note"></a>
### 开发备注

<details>
<summary>维护者上下文——点击展开</summary>

`tests/mount.spec.ts` 连接 `pnpm run build:rust-anything` 暂存的可执行文件：断言暂存路径、三条命令解析分支、eager 下的完整工具集与 on-demand 下唯一的 loader。未构建、工具改名或传输异常都会让测试失败，而不是在运行时静默降级；`pnpm run test` 会先构建该二进制。

</details>

---
description: "挂载 fast-anything MCP server，让 Agent 把应用里最热的纯函数迁移到 Rust，并用真实录制调用验证每个移植。"
kind: "package-reference"
---

# @deepseek-ai/dsh-fast-anything

[English](README.md) | 中文

## 概述

把 Agent 接到 [fast-anything](https://github.com/EL4CTEO/fast-anything)：一个剖析与移植夹具——找出应用真正耗时的那几个函数，录制它们在真实调用里的行为，再让 Agent 只把这几个函数改写成 Rust。只有 `verify` 回放全部录制调用、派生边界用例与收缩输入都通过，且 `bench` 显示替换没有变慢，移植才会被启用。CLI 由 `pnpm run build` 从 vendored 源码 [`native/fast-anything`](../../../native/fast-anything/NATIVE.md) 编译，检出无需全局安装、无需已发布的二进制、也不需要按机器改配置。Web bundle 把它挂在 `standard` 这个 agent preset 上。

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

由 Web 的 `standard` agent preset 以 `mcp-fast-anything` 行挂载，该 preset 的 agent 直接拿到 `mcp__fast-anything__*` 工具，无需任何安装步骤。"让这个 Python 脚本快一点""找出这个应用的热点""把慢的那部分搬到 Rust"这类任务会命中对应的 [`dsh-fast-anything`](../../skill/skill-fast-anything/assets/dsh-fast-anything/SKILL.md) 技能，按 `fast_scan` 按耗时占比与可移植性给函数排序、`fast_port` 录制真实调用并生成单函数 Rust crate 与 `TASK.md`、Agent 写移植、`fast_verify`、`fast_enable`、`fast_bench`、`fast_report`（后续可用 `fast_disable` 与 `fast_status`）推进。

| 字段 | 默认值 | 含义 |
|---|---|---|
| `command` | 仓库内编译产物 | 覆盖 `native/fast-anything/bin/fast-anything`（`pnpm run build` 从 vendored crate 编译并暂存的位置）；环境变量 `FAST_ANYTHING_BIN` 是次级回退。 |
| `toolLoading` | `eager` | 设为 `on-demand` 时 8 个工具只进系统提示目录，由 Agent 按需加载。 |
| `toolCallTimeoutMs` | 15 分钟 | `scan`、`port`、`verify`、`bench` 会剖析应用、回放录制调用并 release 编译 crate，MCP 默认的 60 秒不够。 |
| `startupTimeoutMs` | 20 秒 | stdio 探测、连接与首次工具同步的预算。 |
| `env` | `{}` | 提供给 server 进程的额外环境变量。 |

每个工具都可选传 `path`（项目目录，默认取 server 进程的工作目录）。CLI 作为普通子进程运行：它在剖析器下运行应用、录制调用、往项目里写 `.fast/` 目录与 Rust crate，`fast_enable` 还会改写被移植函数的调用点。它不受 shell 沙箱限制，请只在项目副本上使用，并避免把生产凭据放进环境。

-----

<a id="understand-the-implementation"></a>
## 实现说明

<details>
<summary>开发者参考：解析与挂载</summary>

`vendoredBinary(platform)` 从本包自己的 `import.meta.url` 解析出 `native/fast-anything/bin/fast-anything[.exe]`；[`scripts/build-fast-anything.ts`](../../../scripts/build-fast-anything.ts) 负责编译 vendored crate 并暂存该文件，`pnpm run build` 以 `fast-anything CLI` gate 调用它；`resolveCommand` 依次取显式 `command`、`FAST_ANYTHING_BIN`、该路径；`apply` 用 `serverName: 'fast-anything'`、`args: ['mcp']` 和配置的超时挂载一个 `@deepseek-ai/dsh-mcp-client` 实例。该 client 是 peer 依赖：由宿主组合提供，发布安装不会带上第二份插件副本。

</details>

-----

<a id="further-exploration"></a>
## 延伸阅读

- [fast-anything](https://github.com/EL4CTEO/fast-anything)：夹具本身、支持的语言，以及哪些东西无法被移植搬走。
- [`@deepseek-ai/dsh-mcp-client`](../../mcp/mcp-client/README.zh.md)：工具命名、重连与按需加载。
- [`@deepseek-ai/dsh-skill-fast-anything`](../../skill/skill-fast-anything/README.zh.md)：与这些工具一起挂载的随包技能。

-----

<a id="model-experience"></a>
## 模型体验

### 来自 server 的 MCP 工具

#### 模型看到的内容

`toolLoading: eager` 时 8 个 fast-anything 工具以 `mcp__fast-anything__*` 进入 agent 的工具列表，描述与 MCP server 一致。server 的 initialize 结果带 instructions，因此 agent 的系统提示多出 `### MCP server: fast-anything` 段，共享的 MCP 资源工具也会在自己的段落里点名该 server。

#### Token 影响

8 个 schema 合计约 3 KB（约 800 tokens）的固定请求上下文，另加 server 的 instructions 段（392 字节），以及 server 注册后出现一次的 3 个共享 MCP 资源工具。`on-demand` 用系统提示目录与一个 `mcp__fast-anything__load_tools` schema 替换这 8 个工具。

#### KV Cache 影响

挂载发生在第一个 turn 之前、会话中不再变化，因此不会让热 prefix 失效；在 `eager` 与 `on-demand` 之间切换则会让它失效。

## 已知限制与待办

<a id="known-limitations-and-deferred-work"></a>

- 插件挂在 agent preset 里，因此每个使用它的 agent 都会启动自己的 server 进程。
- 可执行文件是仓库构建产物：启动 Web 前先运行 `pnpm run build`（或 `pnpm run build:fast-anything`），改动 vendored crate 后重新运行。它不从 npm 下载，所以本包只服务当前检出，不服务已发布的安装。
- 打包运行时带不上它：可执行文件在所有包目录之外，因此 Desktop 应用与 Python SDK 可执行文件会挂载插件却没有 CLI，不提供 fast-anything 工具。要让它们带上，需要像 `native/system` 那样按平台暂存二进制。
- 编译 CLI 需要 rustup 安装的 Rust 1.88 或更新版本；`verify`/`bench` 也用同一工具链编译生成的移植 crate。`scan` 本身不需要 Rust，`fast-anything doctor` 会报告机器处于哪种状态。
- 移植要运行目标应用自己的运行时：按应用不同，需要 Python 3.10+、Node.js 22+、Ruby 3.3+、Java 22+、Go 1.22+、PHP 8.1+ 且启用 `ext-ffi`、C/C++ 编译器、.NET 8+ 或 R 4.0+。`fast-anything doctor` 会逐个报告。
- 省略 `path` 时工具以 server 进程的工作目录为目标；要处理别的项目请显式传 `path`。
- `fast_enable` 会改写项目源码里的调用点，`scan` 与 `port` 会以 Agent 的完整环境执行项目代码。请在副本上运行并审查 `enable` 的 diff。

<a id="dev-note"></a>
### 开发备注

<details>
<summary>维护者工作背景——点击展开</summary>

`tests/mount.spec.ts` 连接 `pnpm run build:fast-anything` 暂存的可执行文件：断言暂存路径、三条命令解析分支、完整的 eager 工具集，以及 on-demand 时的唯一 loader。缺构建、工具改名或传输损坏会让这套测试失败，而不是在运行时静默降级；`pnpm run test` 会先构建二进制。

</details>

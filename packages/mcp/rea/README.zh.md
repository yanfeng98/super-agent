---
description: "挂载 rea MCP server，让 Agent 基于证据逆向分析二进制、应用与运行时行为。"
kind: "package-reference"
---

# @deepseek-ai/dsh-rea

[English](README.md) | 中文

## 概述

把 Agent 接到 [rea](https://github.com/morluto/rea)：一个 MCP server，用来检查原生二进制、托管程序集、Electron/JavaScript 应用、Android 包、固件、网站与录制下来的运行时行为，返回代码、引用、限制与未知，而不是一个没有保留条件的结论。静态 JavaScript 与 .NET 检查不需要分析引擎；深度原生分析使用已安装的 Hopper、Ghidra、IDA、JADX 或 Kuna。server 由 `pnpm run build` 从 vendored 在 [`native/rea`](../../../native/rea/NATIVE.md) 的源码构建，因此 clone 后不需要已发布的 tarball、不需要全局安装、也不需要 `rea setup`。Web bundle 把它挂在 `standard` agent preset 里。

## 目录

- [使用本包](#use-this-package)
- [了解实现](#understand-the-implementation)
- [延伸阅读](#further-exploration)
- [模型体验](#model-experience)
- [已知限制与待办](#known-limitations-and-deferred-work)
- [开发备注](#dev-note)

-----

<a id="use-this-package"></a>
## 使用本包

由 Web 的 `standard` agent preset 以 `mcp-rea` 行挂载；工具以 `mcp__rea__*` 出现在该 Agent 里，无需任何安装步骤。形如「这个应用的搜索是怎么实现的」「比较这两个构建」「从已发布的二进制里还原这个功能」的任务会命中对应的 [`dsh-rea`](../../skill/skill-rea/assets/dsh-rea/SKILL.md) 技能，由它按目标物选入口、阅读工具返回的 Evidence，并把观察、推断与未知分开。

| 字段 | 默认值 | 作用 |
|---|---|---|
| `command` | Host 自己的 Node.js | 运行 launcher 的 Node.js 可执行文件。设置为别的受支持运行时可固定版本。 |
| `launcher` | vendored 的 `native/rea/scripts/rea.mjs` | 替代仓库内构建的入口脚本。 |
| `catalogProfile` | `hot-cold` | `full` 在连接时 advertise 全部 263 个工具；`hot-cold` 只 advertise 5 个，其余经 `discover_tools` 加载。 |
| `toolLoading` | `on-demand` | `on-demand` 在系统提示里列出已 advertise 的工具，按 Agent 请求加载。`eager` 把每个 schema 放进每次请求；配 `full` 时约 353 KB schema。 |
| `toolCallTimeoutMs` | 15 分钟 | 反编译、运行时观察与整应用采集超过 MCP 默认的 60 秒。 |
| `startupTimeoutMs` | 20 秒 | stdio 探测、连接与首次工具同步的预算。 |
| `env` | `{}` | server 进程的额外环境，在 catalog profile 之后合并。 |

默认配置下，Agent 先加载 `mcp__rea__load_tools`，用任务词或精确名称调用 `discover_tools`，再加载返回的工具后调用；server 报告工具列表变化时 DSH 会重新同步工具目录。server 以普通子进程运行：运行时观察、采集与环境录制会以 Host 用户权限执行被分析的目标，所以在副本上操作，并且不要把生产凭据留在环境里。

-----

<a id="understand-the-implementation"></a>
## 理解实现

<details>
<summary>开发者参考：解析与挂载</summary>

`vendoredLauncher()` 返回 `native/rea/scripts/rea.mjs`，从本包自己的 `import.meta.url` 解析；[`scripts/build-rea.ts`](../../../scripts/build-rea.ts) 安装 vendored 的 npm 依赖（`npm ci`，每个 lockfile 一次）并运行上游构建，`pnpm run build` 把它作为 `rea MCP server` gate 运行。`resolveLauncher` 优先显式 `launcher`，否则用该路径。`apply` 挂一个 `@deepseek-ai/dsh-mcp-client` 实例，`serverName: 'rea'`、`args: [launcher, 'mcp']`、配置的 catalog profile 与超时。该 client 是 peer 依赖：Host 组合提供实例，已发布安装不会带第二份插件。上游 engines 下限（Node.js `^22.23.4 || ^24.22.1 || >=26.12.1`）低于本仓库自己的下限，因此受支持的 Host 运行时已经够用。

</details>

-----

<a id="further-exploration"></a>
## 进一步探索

- [rea](https://github.com/morluto/rea)：工具、支持目标、分析 provider 与证据模型。
- [rea 集成报告](../../../rea-integration.zh.md)：为什么 vendored 源码、构建与挂载如何验证、已知成本。
- [`@deepseek-ai/dsh-mcp-client`](../mcp-client/README.zh.md)：工具命名、重连、按需加载与目录重新同步。

-----

<a id="model-experience"></a>
## 模型体验

### 来自 server 的 MCP 工具

#### 模型看到什么

默认 `hot-cold` profile 加 `toolLoading: on-demand` 时，系统提示在 `On-demand tools from MCP server rea` 下列出 5 个工具：`open_binary`、`binary_session`、`close_binary`、`get_evidence_bundle`、`discover_tools`，外加 `mcp__rea__load_tools` loader。`discover_tools` 加载剩余目录的一页；server 随后报告工具列表变化，client 重新同步，新加载的定义进入目录，可用 `load_tools` 加载。连接注册后会出现 server 的 instructions。

#### Token 影响

5 个初始 schema 合计几 KB。按需目录从 5 行开始，只随 Agent 实际加载的工具增长；每个已加载工具把自己的描述与输入 schema 加进请求。若选 `catalogProfile: full` 配 `on-demand`，提示目录会列出全部 263 个工具（约 140 KB 的名称与首行描述）；`eager` 则把完整 353 KB schema 集放进每次请求。

#### KV Cache 影响

挂载发生在第一轮之前，会话中途不变。加载工具或发现新的一页会改变工具列表与系统提示，从那时起使热的 prefix 失效；默认目录把这项成本限制在实际用到的工具上。

## 已知限制与待办

<a id="known-limitations-and-deferred-work"></a>

- 插件挂在 agent preset 里，每个使用它的 Agent 各自启动一个 server 进程。
- 运行时是仓库构建产物：启动 Web 前跑 `pnpm run build`（或 `pnpm run build:rea`），改过 vendored 源码后再跑一次。首次构建在 `native/rea` 里执行 `npm ci`，需要网络；之后直到 `package-lock.json` 变化前复用已安装树。
- 构建与运行需要 Node.js 24.22.3 或更新版本，与本仓库 engines 下限一致。更旧的运行时会构建失败，launcher 也会以可操作的错误拒绝启动。
- 打包运行时带不上它：`native/rea` 在所有包目录之外，因此 Desktop 应用与 Python SDK 可执行文件只挂插件、没有 server，不暴露 rea 工具。要随包发布需要按平台暂存。
- 多数原生操作需要另行安装的分析 provider（Hopper、Ghidra、IDA、JADX、Kuna 等）。工具会报告 provider 可用性，但不会安装引擎。
- 运行时观察与环境录制会执行被分析的目标，可能采集剪贴板、UI 或网络数据。请在副本上操作，不要把生产凭据留在环境里。
- 加载工具会改动会话的工具列表，并记进会话日志；resume 会重新挂载 server、从新目录开始，之前加载的工具需要重新发现。

<a id="dev-note"></a>
### 开发备注

<details>
<summary>维护者的工作上下文——点击展开</summary>

`tests/mount.spec.ts` 连接 `build:rea` gate 准备好的 launcher：断言 vendored 路径、launcher 解析、`eager` 下 5 个工具的 hot-cold advertise、按需模式只有 loader，以及运行时缺失时 Agent 创建不受影响。缺少构建、上游版本改变了初始目录或传输损坏都会让用例失败，而不是在运行时静默降级；`pnpm run test` 会先构建 vendored 树。

</details>

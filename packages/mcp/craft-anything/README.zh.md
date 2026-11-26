---
description: "挂载 craft-anything MCP server，让 Agent 通过按需加载的工具集对已发布软件做证据驱动的调查。"
kind: "package-reference"
---

# @deepseek-ai/dsh-craft-anything

[English](README.md) | 中文

## 概述

把 Agent 接入 [craft-anything](https://github.com/storytold/craft-anything)：一套用于调查程序、应用包、固件、崩溃转储与网络抓包的洁净室工具。它在项目目录里维护持续的简报、计划、假设、发现与交接，并用证据编号让每条结论可回溯。工具分为 16 个工具集，server 启动时只列出 Core 工具集，其余按需启用。CLI 由 `pnpm run build` 从 [`native/craft-anything`](../../../native/craft-anything/NATIVE.md) 的 Rust 工作区编译，无需全局安装、发布版二进制或逐机注册。Web bundle 把它挂载在 `standard` agent preset 中。

## 目录

- [使用本包](#use-this-package)
- [理解实现](#understand-the-implementation)
- [延伸阅读](#further-exploration)
- [模型体验](#model-experience)
- [已知限制与后续工作](#known-limitations-and-deferred-work)
- [开发备注](#dev-note)

-----

<a id="use-this-package"></a>
## 使用本包

由 Web `standard` agent preset 以 `mcp-craft-anything` 行挂载；Agent 加载后工具名为 `mcp__craft-anything__*`。「这个已发布的应用是怎么实现搜索的」「比较这两个构建」「这个固件镜像里有什么」这类任务会命中随包的 [`dsh-craft-anything`](../../skill/skill-craft-anything/assets/dsh-craft-anything/SKILL.md) 技能。Agent 先用 `core_target_open` 打开目标，写下简报与计划，再用 `enable_toolset` 启用工具集、用 `mcp__craft-anything__load_tools` 加载返回的工具。每条结论都引用证据编号，最后用 `core_handoff_write` 留下交接记录。

| 字段 | 默认值 | 含义 |
|---|---|---|
| `command` | 仓库编译产物 | 替代 `native/craft-anything/bin/craft-anything-cli[.exe]` 的可执行文件；该路径由 `pnpm run build` 从 vendored 工作区暂存，`CRAFT_ANYTHING_BIN` 是环境变量回退。 |
| `toolLoading` | `on-demand` | 只发布 `mcp__craft-anything__load_tools`；系统提示目录列出 Core 工具集以及 Agent 已启用的工具集。 |
| `toolCallTimeoutMs` | 15 分钟 | 调查可能打开大归档、运行分析引擎并构建重建项目。 |
| `startupTimeoutMs` | 20 s | stdio 探测、连接与首次工具同步的预算。 |
| `env` | `{}` | 传给 server 进程的额外环境变量。 |

server 是普通子进程，不受 shell sandbox 约束。默认的洁净室模式拒绝读取目标自身的文件与源码；深度模式必须显式开启。会运行目标代码或改动连接设备的工具需要用户确认。请在副本上操作，并避免把生产凭据留在环境里。

-----

<a id="understand-the-implementation"></a>
## 理解实现

<details>
<summary>开发者参考：解析与挂载</summary>

`vendoredBinary(platform)` 返回 `native/craft-anything/bin/craft-anything-cli[.exe]`，从本包的 `import.meta.url` 解析；[`scripts/build-craft-anything.ts`](../../../scripts/build-craft-anything.ts) 编译 vendored Rust 工作区并暂存该文件，`pnpm run build` 以 `craft-anything CLI` gate 运行它。`resolveCommand` 优先使用显式 `command`，然后是 `CRAFT_ANYTHING_BIN`，最后是暂存路径。`apply` 挂载一个 `@deepseek-ai/dsh-mcp-client` 实例（`serverName: 'craft-anything'`、`args: ['mcp']` 与配置的超时）。启用工具集会发出 `notifications/tools/list_changed`，客户端会据此刷新按需目录。该客户端是 peer 依赖：由宿主组合提供实例，发布安装不会带上第二份插件。

</details>

-----

<a id="further-exploration"></a>
## 延伸阅读

- [craft-anything](https://github.com/storytold/craft-anything)：引擎、工具集与确认模型。
- [`@deepseek-ai/dsh-mcp-client`](../../mcp/mcp-client/README.zh.md)：工具命名、重连与按需加载。
- [`@deepseek-ai/dsh-skill-craft-anything`](../../skill/skill-craft-anything/README.zh.md)：与这些工具一起挂载的随包技能。

-----

<a id="model-experience"></a>
## 模型体验

### 来自 server 的 MCP 工具

#### 模型看到的内容

`v0.6.1` 的 server 先公布 30 个工具：Core 工具集加上 `list_toolsets` 与 `enable_toolset`，全部工具共 323 个。`toolLoading: eager` 会把它们以 `mcp__craft-anything__*` 加入工具列表；默认的 `on-demand` 只给出 `mcp__craft-anything__load_tools` 与系统提示目录。server 的 initialize 结果带说明文本，因此系统提示会增加 `### MCP server: craft-anything` 一节，共享 MCP 资源工具也会在自己的小节里点名该 server。

#### Token 影响

30 个初始 schema 约 31 KB（约 7.8k tokens）的固定请求上下文，另有 874 字节的说明文本和 server 注册后出现的三个共享 MCP 资源工具。`on-demand` 用目录与一个加载工具替代这些 schema；启用工具集后，客户端同步完成才会加入该工具集的工具。

#### KV Cache 影响

挂载发生在首轮之前。启用工具集会改变会话中途的系统提示目录与工具列表，因此从那一刻起热前缀失效；技能会要求 Agent 只启用问题真正需要的工具集。

## 已知限制与后续工作

<a id="known-limitations-and-deferred-work"></a>

- 插件挂在 agent preset 内，因此每个使用它的 agent 都会启动自己的 server 进程。
- 可执行文件是仓库构建产物：启动 Web 前先运行 `pnpm run build`（或 `pnpm run build:craft-anything`），修改 vendored 工作区后也要重跑。本包不从 npm 下载任何东西，只服务于当前 checkout，不适用于发布安装。
- 打包运行时无法带上它：可执行文件位于所有包之外，因此 Desktop 应用与 Python SDK 可执行文件只挂载插件而没有 CLI，不提供 craft-anything 工具；要发布需按平台暂存，类似 `native/system` 的做法。
- 构建 CLI 需要 rustup 提供 Rust 1.97 或更新版本；vendored 副本固定该 channel。
- 深度调查依赖用户另行安装的分析引擎：Ghidra、JADX、Apktool、adb、Chromium 浏览器、Frida、GDB/LLDB 等。`craft-anything-cli capabilities` 会报告可用性，工具还提供带校验和的 `engines_install` 辅助安装可下载的引擎。
- `v0.6.1` 默认洁净室模式，拒绝读取目标内部。切换到深度模式前先询问用户；工具返回 `details.reason` 为 `clean_room` 时即表示被此限制挡住。
- 工具可能执行目标代码或改动连接设备，需要显式确认；遇到 `consent_required` 不要绕过。

<a id="dev-note"></a>
### 开发备注

<details>
<summary>维护者工作背景 — 点击展开</summary>

`tests/mount.spec.ts` 连接 `pnpm run build:craft-anything` 暂存的二进制：断言暂存路径、三条命令解析分支、完整的初始 eager 工具集与唯一的按需加载工具。缺少构建产物、初始工具改名或传输故障都会让测试失败，而不是在运行时静默退化；`pnpm run test` 会先构建二进制。

</details>

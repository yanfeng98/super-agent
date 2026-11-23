---
description: "内置的 rea 技能：它承载的基于证据的逆向工作流、挂载位置，以及维护者需要重新同步的上游文本。"
kind: "package-reference"
---

# @deepseek-ai/dsh-skill-rea

[English](README.md) | 中文

## 概述

这个提供方只提供一个内置技能 `dsh-rea`，教的是 [rea](https://github.com/morluto/rea) 的调查流程：按用户提供的目标物选择入口、阅读工具返回的证据、区分观察、推断与未知，再用普通编码工具做出重建。覆盖原生二进制与归档、托管程序集、Electron/JavaScript 应用、Android 包、固件、网站与运行时观察。技能文本取自 vendored 在 [`native/rea`](../../../native/rea/NATIVE.md) 的上游 `.agents/skills/reverse-engineer-anything` 目录（MIT），并按这个内置副本做了改编。

Web 的 `standard` agent preset 把这个提供方与 rea MCP server 挂在一起，所以能加载 `mcp__rea__*` 工具的会话同时也有解释如何访问它们的技能。

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

把提供方与 skill 注册表、`dsh-tool-skill` 挂在一起。它以 `dsh-rea` 为提供方名注册进 agent 的 skill 层，使用随包 rank（600），并声明 `shipped`，因此输入框把它和其他内置技能列在一起。

```yaml
- id: skill-rea
  name: '@deepseek-ai/dsh-skill-rea'
```

| 字段 | 默认值 | 作用 |
|---|---|---|
| `assetRoot` | 随包的 `assets/` | 存放 `dsh-rea` 技能目录的绝对路径。需要提供一份搬迁过的副本时设置它，测试就是这样做的。 |

相对路径的 `assetRoot` 会在加载时报错。要把技能从 profile 里去掉，把挂载它的那一行换成 `disabled: true`。

### 目录里有什么

只有一条 `dsh-rea`，模型与用户都能调用：描述匹配解释或重建已发布行为、比较应用版本、从二进制里还原功能的请求，`/dsh-rea` 会按需加载正文。条目带上游项目 `morluto/rea`，因此输入框把它与只属于本仓库的技能分开分组。

更近的一层有同名技能时以它为准：项目或用户技能目录会顶替本提供方的目录条目与正文，同一层内随包的 rank 排在最后。

### 能观察到的成功与失败

技能在目录里出现一次并带上描述，加载时给出随包正文，`references/` 下的指引相对技能目录解析。缺失或非法的 `SKILL.md` 会从完整目录里省略，并通过 `ctx.logger.warn` 报告一次。其它读取错误会让发现过程保持不完整，注册表会重试，加载则直接拒绝而不是给出半份正文。释放插件会移除这条候选。

-----

<a id="understand-the-implementation"></a>
## 理解实现

<details>
<summary>实现内部细节 —— 点击展开</summary>

提供方持有技能目录名，在发现时读取 `assets/<name>/SKILL.md`，每次加载再读一次，并在发布候选前校验 frontmatter 契约。目录名就是身份：声明的 `name` 与它不一致即为非法元数据。`description` 必填且非空。`disable-model-invocation` 与 `user-invocable` 是可选布尔值，映射到注册表的调用策略；随包文件两个都不声明，因此该技能既可由模型调用，也可由用户调用。

提供方通过延迟的 `skills` 注入接入，因此移除 skill 注册表的 profile 只会让这一行闲置，不会报待处理依赖。

失败分两类。`ENOENT` 与 `ENOTDIR` 会持续到文件出现，因此技能从完整目录里省略，加载在使目录失效后返回 `undefined`。YAML、字段与命名错误同样持续。其它读取错误让观察结果不完整，并从加载中抛出。

| 文件 | 作用 |
|---|---|
| [`src/index.ts`](src/index.ts) | 插件入口、frontmatter 契约、提供方注册与失败上报 |
| [`assets/dsh-rea/SKILL.md`](assets/dsh-rea/SKILL.md) | 调查流程、按需工具加载步骤与本检出的 CLI 路径 |
| [`assets/dsh-rea/references/`](assets/dsh-rea/references/) | 正文链接的目标类型、运行时观察、证据与环境指引 |
| [`assets/LICENSE`](assets/LICENSE) | 覆盖改编后技能文本的上游 MIT 声明 |

</details>

-----

<a id="further-exploration"></a>
## 进一步探索

- [rea 挂载](../../mcp/rea/README.zh.md) —— 同一 preset 挂载的 MCP server、目录配置与 vendored 构建。
- [Skill 子系统参考](../../../docs/subsystems/skills.zh.md) —— 本提供方接入的注册表、提供方契约与目录渲染。
- [skill](../skill/README.zh.md) —— 注册表与调用策略类型。
- [skill-rust-anything](../skill-rust-anything/README.zh.md) —— 另一个上游衍生的内置提供方，失败约定相同。

-----

<a id="model-experience"></a>
## 模型体验

间接通过 `dsh-tool-skill` 体现：它渲染目录条目与加载后的技能正文；另通过 rea MCP server 的按需目录体现。

#### KV Cache 影响

条目的描述从第一次请求起就在会话目录里。正文只在加载点进入对话，因此从不涉及逆向的会话只为这段描述付费。

## 已知限制与待办

<a id="known-limitations-and-deferred-work"></a>

这些限制说明内置提供方不做什么，是当前的包约束，不是待办清单。

- **内置措辞** —— 正文点名本仓库的 `native/rea` launcher 与 Web `standard` preset。没有这个挂载的 profile 不会列出技能，因为 preset 把提供方与工具一起挂载。
- **仅英文** —— 技能正文是上游英文语料；插件元数据已本地化，但目录条目与正文没有，本地化目录需要本地覆盖。
- **手工同步** —— 上游变化通过把新的 `.agents/skills/reverse-engineer-anything/` 复制进包并重新套用内置改编来跟进；没有构建步骤跟踪上游仓库。
- **没有单技能配置** —— 提供方只暴露一个资产根，始终发布随包技能。
- **外部分析引擎** —— 多数原生操作需要另行安装的 Hopper、Ghidra、IDA、JADX 等；技能会报告不可用，但不会安装 provider。
- **目标执行** —— 运行时观察与环境录制会以 Host 用户权限执行被分析的目标；不要把生产凭据留在环境里，并在副本上操作。

<a id="dev-note"></a>
### 开发备注

<details>
<summary>维护者的工作上下文——点击展开</summary>

随包的文本是 vendored 树里的上游 `.agents/skills/reverse-engineer-anything/`，改名为 `dsh-rea`，改了两处：连接一节（preset 挂载、按需的 `load_tools` 与 `discover_tools` 步骤、vendored CLI 路径与 Node 版本下限，取代 `npx`、`rea setup` 与插件注册），以及 `references/connection-and-recovery.md` 顶部说明注册步骤不适用。重新同步时：

1. 上游升级后把 `native/rea/.agents/skills/reverse-engineer-anything/` 复制进 `assets/dsh-rea/`。
2. 重新套用上述改编，并把 frontmatter 的 `name` 改成 `dsh-rea`。
3. 运行本包测试，它会比较资产目录与提供方的技能名。

`assets/LICENSE` 带上游 MIT 声明，`locale/*.json` 为插件清单提供各 UI 语言的组件标题与描述。

</details>

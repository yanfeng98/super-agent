---
description: "随包的 fast-anything 技能：它携带的热点移植流程、挂载位置，以及维护者重新同步的上游文本。"
kind: "package-reference"
---

# @deepseek-ai/dsh-skill-fast-anything

[English](README.md) | 中文

## 概述

本提供方随包提供一个技能 `dsh-fast-anything`，教 [fast-anything](https://github.com/EL4CTEO/fast-anything) 的流程：先扫描应用，按耗时占比与可移植性给函数排序；录制某个函数的真实调用；写出它的 Rust 移植；再 verify、enable、bench 与 report，需要时还能 disable。它只处理慢的那一小块，不做整应用重写：应用的其余部分保持原样。文本取自 [`native/fast-anything`](../../../native/fast-anything/NATIVE.md) 中 vendored 的上游 `skills/fast-anything` 目录，MIT 许可。

Web 的 `standard` agent preset 把它与 fast-anything MCP server 一起挂载，因此带 `mcp__fast-anything__*` 工具的会话同时拥有驱动这些工具的技能。

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

把提供方挂在 skill 注册表与 `dsh-tool-skill` 旁边。它以提供方名 `dsh-fast-anything` 注册进 agent 的 skill 层，使用 bundled 技能等级（600）并声明 `shipped`，因此组合器会把它与仓库其余技能一起列出。

```yaml
- id: skill-fast-anything
  name: '@deepseek-ai/dsh-skill-fast-anything'
```

| 字段 | 默认值 | 含义 |
|---|---|---|
| `assetRoot` | 包内 `assets/` | 存放 `dsh-fast-anything` 技能目录的绝对路径。要提供搬迁后的副本时设置它，测试就是这么做的。 |

相对路径的 `assetRoot` 会在加载时失败。要从 profile 移除该技能，把挂载行换成 `disabled: true`。

### 目录内容

只有一条 `dsh-fast-anything`，模型与用户都能触达：描述匹配"让代码更快"、剖析、把慢的部分搬到 Rust 这类请求，`/dsh-fast-anything` 按需加载正文。该条目带上游项目 `EL4CTEO/fast-anything`，因此组合器把它与仅属于仓库的技能分组展示。

更近层的同名技能优先：项目或用户技能目录会取代本提供方提供目录条目与正文，同一层内 bundled 等级排在最后。

### 可观察的成功与失败

技能在目录里出现一次并带描述，能加载包内正文，并把资源目录解析到包内 assets。缺失或非法的 `SKILL.md` 会从完整目录中省略，并通过 `ctx.logger.warn` 只报告一次。其他读取错误会让发现结果不完整，注册表会重试；加载会拒绝，而不是提供部分正文。销毁插件会移除该候选。

-----

<a id="understand-the-implementation"></a>
## 实现说明

<details>
<summary>实现内部——点击展开</summary>

提供方持有技能目录名，在发现时读取一次 `assets/<name>/SKILL.md`，每次加载再读一次，并在发布候选前校验 frontmatter 契约。目录名就是身份：声明的 `name` 不一致即为非法元数据。`description` 必填且非空。`disable-model-invocation` 与 `user-invocable` 是可选布尔值，投影为注册表的调用策略；包内文件两者都没声明，因此技能对模型与用户都可调用。

提供方通过延迟的 `skills` 注入挂载，因此移除了 skill 注册表的 profile 会让该行保持空闲，而不是报告待注入依赖。

失败分两类。`ENOENT` 与 `ENOTDIR` 会一直持续到文件出现，因此技能从完整目录省略，加载会使目录失效后返回 `undefined`。YAML、字段或命名错误同样持续。其他读取错误会让观察不完整，并从加载中抛出。

| 文件 | 作用 |
|---|---|
| [`src/index.ts`](src/index.ts) | 插件入口、frontmatter 契约、提供方注册、失败报告 |
| [`assets/dsh-fast-anything/SKILL.md`](assets/dsh-fast-anything/SKILL.md) | 移植流程、工具列表与安装说明 |
| [`assets/LICENSE`](assets/LICENSE) | 覆盖改编技能文本的上游 MIT 声明 |

</details>

-----

<a id="further-exploration"></a>
## 延伸阅读

- [fast-anything 挂载](../../mcp/fast-anything/README.zh.md) —— 该技能驱动的 MCP server 与 CLI，由同一个 preset 挂载。
- [技能子系统参考](../../../docs/subsystems/skills.zh.md) —— 本提供方接入的注册表、提供方契约与目录渲染。
- [skill](../skill/README.zh.md) —— 注册表与调用策略类型。
- [skill-engineering](../skill-engineering/README.zh.md) —— 另一个源自上游的随包提供方，采用相同的失败约定。
- [提速指南](../../../docs/user/guide/fast-anything.zh.md) —— 面向用户的完整流程。

-----

<a id="model-experience"></a>
## 模型体验

间接地，通过 `dsh-tool-skill` 渲染目录条目与加载后的技能正文。

#### KV Cache 影响

条目描述从第一个请求起就在会话目录里。正文只在加载点进入对话，因此从不要求提速的会话只为描述付费。

## 已知限制与待办

<a id="known-limitations-and-deferred-work"></a>

这些限制界定随包提供方不做什么。它们是当前包约束，不是任务清单。

- **只针对一种安装措辞** —— 正文点名暂存的 `native/fast-anything/bin/fast-anything` 可执行文件与 Web 的 `standard` preset。没有该挂载的 profile 不会列出技能，因为 preset 把提供方与工具一起挂载。
- **仅英文** —— 技能文本是上游英文语料；插件元数据已本地化，但目录条目与正文没有，本地化目录需要本地覆盖。
- **手动同步** —— 上游变更通过复制 `native/fast-anything/skills/fast-anything/` 并重新施加安装措辞完成；没有构建步骤跟踪上游仓库。
- **无按技能配置** —— 提供方只暴露一个 assets 根目录，并且总是发布包内技能。

<a id="dev-note"></a>
### 开发备注

<details>
<summary>维护者工作背景——点击展开</summary>

包内文本是 vendored crate 中上游 `skills/fast-anything/SKILL.md` 的副本，改名为 `dsh-fast-anything`，并在五处做了改动：frontmatter 名称、CLI 子命令旁的 MCP 工具名（`mcp__fast-anything__*`）、`path` 参数、暂存构建路径与"不要全局安装"，以及末尾的本仓库段落（Rust 1.88、`.fast/target` 缓存、`enable`/`disable` 会改写源码的警告）。重新同步：

1. 上游升级后，把 `native/fast-anything/skills/fast-anything/` 复制到 `assets/dsh-fast-anything/`。
2. 重新施加这五处改动，并把 frontmatter 的 `name` 设为 `dsh-fast-anything`。
3. 运行包测试，它会比对 assets 目录与提供方的技能名。

`assets/LICENSE` 带上游 MIT 声明，`locale/*.json` 为插件清单提供各 UI 语言的组件标题与描述。

</details>

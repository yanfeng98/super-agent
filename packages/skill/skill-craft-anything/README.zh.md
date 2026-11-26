---
description: "提供内置 craft-anything 工作流：在调查已发布软件时维护简报、计划、假设、证据结论与交接。"
kind: "package-reference"
---

# @deepseek-ai/dsh-skill-craft-anything

[English](README.md) | 中文

## 概述

本提供方随包提供一个技能 `dsh-craft-anything`，教授 [craft-anything](https://github.com/storytold/craft-anything) 的调查流程：打开一个已发布的目标，在项目目录里维护简报、计划、假设、发现与交接；为每条结论附上证据编号；只启用问题需要的工具集；把观察到的事实、推导结果与推断分开陈述。文本来自 [`native/craft-anything`](../../../native/craft-anything/NATIVE.md) 中的 `crates/setup/src/skill.md` 与 `skill_recovery.md`，采用 MIT OR Apache-2.0 双许可。

Web `standard` agent preset 把它与 craft-anything MCP server 一起挂载，因此持有 `mcp__craft-anything__*` 工具的会话同时拥有驱动这些工具的技能；Web 宿主 patch 还让其他 profile 也能在 `/` 菜单里点名该技能。

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

把本提供方与 skill registry、`dsh-tool-skill` 一起挂载。它以提供方名 `dsh-craft-anything` 注册到 agent 技能层，位于 bundled 技能层级（600），并声明为 `shipped`，因此编排器会把它与 Harness 其他技能一并列出。

```yaml
- id: skill-craft-anything
  name: '@deepseek-ai/dsh-skill-craft-anything'
```

| 字段 | 默认值 | 含义 |
|---|---|---|
| `assetRoot` | 随包 `assets/` | 存放 `dsh-craft-anything` 技能目录的绝对路径。可设为迁移后的副本，测试就是这样做的。 |

相对路径的 `assetRoot` 会在加载时失败。若要从某个 profile 移除该技能，把挂载行改为 `disabled: true`。

### 目录里有什么

一个条目 `dsh-craft-anything`，模型与用户都可调用：其描述会匹配「这个已发布的程序/应用包/固件镜像/抓包是怎么工作的」这类请求，`/dsh-craft-anything` 按需加载正文。条目携带上游项目 `storytold/craft-anything`，因此编排器把它归入 `内置 · storytold/craft-anything`，与 Harness 自有技能分开。

更近层级的同名技能优先：项目或用户技能目录会替代本提供方提供目录条目和正文；同一层级内 bundled 排在最后。

### 可观察的成功与失败

技能在目录中只出现一次并带描述，加载时会读取随包正文与恢复参考，资源目录指向随包 assets。缺失或非法的 `SKILL.md` 会从完整目录中省略，并通过 `ctx.logger.warn` 报告一次。其他读取错误会让发现过程保持不完整，registry 会重试；加载会拒绝而不是返回部分正文。释放插件会移除候选项。

-----

<a id="understand-the-implementation"></a>
## 理解实现

<details>
<summary>实现细节 — 点击展开</summary>

提供方持有技能目录名，在发现时和每次加载时读取 `assets/<name>/SKILL.md`，并在发布候选项前校验 frontmatter 契约。目录名就是身份：声明的 `name` 不一致属于非法元数据。`description` 必需且非空。`disable-model-invocation` 与 `user-invocable` 是可选布尔值，会映射到 registry 的调用策略；随包文件未声明这两项，因此技能同时支持模型调用与用户调用。

提供方通过延迟的 `skills` 注入挂载，因此移除 skill registry 的 profile 只会让该挂载行闲置，而不会报告待决依赖。

失败分两类。`ENOENT` 与 `ENOTDIR` 会持续到文件出现，因此技能会从完整目录中省略，加载返回 `undefined` 并使目录失效。YAML、字段或命名错误同样持续。其他读取错误会让观察结果不完整，并从加载处向上传播。

| 文件 | 作用 |
|---|---|
| [`src/index.ts`](src/index.ts) | 插件入口、frontmatter 契约、提供方注册、失败报告 |
| [`assets/dsh-craft-anything/SKILL.md`](assets/dsh-craft-anything/SKILL.md) | 证据优先流程、工具集加载说明与内建连接章节 |
| [`assets/dsh-craft-anything/references/recovery.md`](assets/dsh-craft-anything/references/recovery.md) | 工具缺失或连接失败的诊断，改编自上游恢复指南 |
| [`assets/LICENSE`](assets/LICENSE)、`LICENSE-APACHE`、`NOTICE` | 覆盖改编技能文本的上游双许可声明 |

</details>

-----

<a id="further-exploration"></a>
## 延伸阅读

- [craft-anything 挂载包](../../mcp/craft-anything/README.zh.md) — 本技能驱动的 MCP server 与 CLI，由同一 preset 挂载。
- [技能子系统参考](../../../docs/subsystems/skills.zh.md) — 本提供方接入的 registry、提供方契约与目录渲染。
- [skill](../skill/README.zh.md) — registry 与调用策略类型。
- [原生源码基线](../../../native/craft-anything/NATIVE.md) — 固定的上游 tag、保留路径与同步流程。

-----

<a id="model-experience"></a>
## 模型体验

通过 `dsh-tool-skill` 间接体现：由它渲染目录条目与加载后的技能正文。

#### KV Cache 影响

条目描述从首个请求起就在会话目录里。正文只在加载时进入对话，因此从不询问已发布软件的会话只需付出描述的成本。会话中途启用工具集会改变工具目录，从那一刻起热前缀失效。

## 已知限制与后续工作

<a id="known-limitations-and-deferred-work"></a>

以下限制描述随包提供方不做什么，是当前包约束，不是待办清单。

- **单一安装的措辞** — 正文点名暂存的 `native/craft-anything/bin/craft-anything-cli` 可执行文件与 Web `standard` preset。恢复指南刻意忽略上游的 `setup` 与 `uninstall` 命令：它们用于向其他 agent 应用注册 server，不适用于这个内建副本。
- **仅英文** — 技能正文沿用上游英文语料；插件元数据已本地化，但目录条目与正文没有，本地化目录需要本地覆盖。
- **手动同步** — 上游变更通过复制 `native/craft-anything/crates/setup/src/skill.md` 与 `skill_recovery.md` 并重新应用内建连接措辞引入；没有构建步骤跟踪上游仓库。
- **无单技能配置** — 提供方只暴露一个 asset root，并始终发布随包技能。

<a id="dev-note"></a>
### 开发备注

<details>
<summary>维护者工作背景 — 点击展开</summary>

随包文本来自 vendored Rust 工作区的 `skill.md` 与 `skill_recovery.md`，改名为 `dsh-craft-anything`，把「Connect only when you need it」一节替换为内建挂载、工具集加载与恢复说明。同步步骤：

1. 上游升级后，把 `native/craft-anything/crates/setup/src/skill.md` 与 `skill_recovery.md` 复制到 `assets/dsh-craft-anything/`。
2. 重新应用内建改法，并把 frontmatter `name` 设为 `dsh-craft-anything`。
3. 运行本包测试，它会比对 asset 目录与提供方技能名。

`assets/LICENSE`、`LICENSE-APACHE` 与 `NOTICE` 携带上游双许可声明，`locale/*.json` 为插件清单提供各 UI 语言的组件标题与描述。

</details>

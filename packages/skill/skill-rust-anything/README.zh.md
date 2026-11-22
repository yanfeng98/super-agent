---
description: "内置的 rust-anything 技能：它承载的按单元迁移流程、挂载位置，以及维护者需要重新同步的上游文本。"
kind: "package-reference"
---

# @deepseek-ai/dsh-skill-rust-anything

[English](README.md) | 中文

## 概述

这个提供方只提供一个内置技能 `dsh-rust-anything`，教的是 [rust-anything](https://github.com/erkamyaman/rust-anything) 的迁移循环：先 profile 找出值得搬的部分，按单元和波次规划，为单个单元 bridge 并 record，写它的 `port.rs`，再用 `diff`、`bench`、`shadow` 与 `gate` 证明它。整应用重写走正文旁边的 `whole-app.md`：先拷问决策、生成整应用用例、在原实现前立一个 Rust 门面、逐单元迁移，最后摘掉回退。技能文本取自 vendored 在 [`native/rust-anything`](../../../native/rust-anything/NATIVE.md) 的上游 `skills/rust-anything` 目录（MIT），并套用下面的安装措辞。

Web 的 `standard` agent preset 把这个提供方与 rust-anything MCP server 挂在一起，所以拿到 `mcp__rust-anything__*` 工具的会话同时也有驱动它们的技能。

## 目录

- [使用本包](#use-this-package)
- [了解实现](#understand-the-implementation)
- [延伸阅读](#further-exploration)
- [模型体验](#model-experience)
- [已知限制与待办](#known-limitations-and-deferred-work)
- [开发者说明](#dev-note)

-----

<a id="use-this-package"></a>
## 使用本包

把提供方与 skill 注册表、`dsh-tool-skill` 挂在一起。它以 `dsh-rust-anything` 为提供方名注册进 agent 的 skill 层，使用随包 rank（600），并声明 `shipped`，因此输入框把它和其他内置技能列在一起。

```yaml
- id: skill-rust-anything
  name: '@deepseek-ai/dsh-skill-rust-anything'
```

| 字段 | 默认值 | 作用 |
|---|---|---|
| `assetRoot` | 随包的 `assets/` | 存放 `dsh-rust-anything` 技能目录的绝对路径。需要提供一份搬迁过的副本时设置它，测试就是这样做的。 |

相对路径的 `assetRoot` 会在加载时报错。要把技能从 profile 里去掉，把挂载它的那一行换成 `disabled: true`。

### 目录里有什么

只有一条 `dsh-rust-anything`，模型与用户都能调用：描述匹配改写、迁移到 Rust、或用 Rust 提速的请求，`/dsh-rust-anything` 会按需加载正文。条目带上游项目 `erkamyaman/rust-anything`，因此输入框把它与只属于本仓库的技能分开分组。

更近的一层有同名技能时以它为准：项目或用户技能目录会顶替本提供方的目录条目与正文，同一层内随包的 rank 排在最后。

### 能观察到的成功与失败

技能在目录里出现一次并带上描述，加载时给出随包正文，`whole-app.md` 引用相对技能目录解析。缺失或非法的 `SKILL.md` 会从完整目录里省略，并通过 `ctx.logger.warn` 报告一次。其它读取错误会让发现过程保持不完整，注册表会重试，加载则直接拒绝而不是给出半份正文。释放插件会移除这条候选。

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
| [`assets/dsh-rust-anything/SKILL.md`](assets/dsh-rust-anything/SKILL.md) | 迁移循环、工具清单与安装说明 |
| [`assets/dsh-rust-anything/whole-app.md`](assets/dsh-rust-anything/whole-app.md) | 正文链接的整应用重写说明 |

</details>

-----

<a id="further-exploration"></a>
## 进一步探索

- [rust-anything 挂载](../../mcp/rust-anything/README.zh.md) —— 本技能驱动的 MCP server 与 CLI，由同一个 preset 挂载。
- [Skill 子系统参考](../../../docs/subsystems/skills.zh.md) —— 本提供方接入的注册表、提供方契约与目录渲染。
- [skill](../skill/README.zh.md) —— 注册表与调用策略类型。
- [skill-engineering](../skill-engineering/README.zh.md) —— 另一个来自上游的随包提供方，失败约定相同。
- [迁移指南](../../../docs/user/guide/rust-anything.zh.md) —— 面向用户的操作步骤。

-----

<a id="model-experience"></a>
## 模型体验

间接地经由 `dsh-tool-skill`：它渲染目录条目与加载后的技能正文。

#### KV Cache 影响

条目的描述从第一次请求起就在会话目录里。正文只在加载点进入对话，所以从不提及 Rust 迁移的会话只付出这段描述。

## 已知限制与延期工作

<a id="known-limitations-and-deferred-work"></a>

这些限制定义随包提供方不做什么，是当前的包约束而不是任务清单。

- **只描述一种安装** —— 正文点名暂存的 `native/rust-anything/bin/rust-anything` 可执行文件与 Web 的 `standard` preset。没有该挂载的 profile 不列这条技能，因为 preset 把提供方与工具挂在一起。
- **只有英文** —— 技能正文是上游英文语料；插件元数据有本地化，目录条目与正文没有，需要本地化目录时要自行覆盖。
- **手工同步** —— 上游变化靠复制 `native/rust-anything/skills/rust-anything/` 并重新套用安装措辞；没有构建步骤跟踪上游仓库。
- **没有单技能配置** —— 提供方只暴露一个资产根，且总是发布随包的那一个技能。

<a id="dev-note"></a>
### 开发备注

<details>
<summary>维护者工作背景 —— 点击展开</summary>

随包文本是 vendored crate 里的上游 `skills/rust-anything/SKILL.md` 与 `whole-app.md`，改名为 `dsh-rust-anything`，并改了三处：语言覆盖（接受 `Kotlin`，与本构建一致）、CLI 那一句（指向本安装暂存的可执行文件而不是全局安装）、以及末尾的安装小节（preset 挂载与绝对路径陷阱）。重新同步的步骤：

1. 上游升级后把 `native/rust-anything/skills/rust-anything/` 复制到 `assets/dsh-rust-anything/`。
2. 重新套用这三处改写，并把 frontmatter 的 `name` 设为 `dsh-rust-anything`。
3. 跑包测试，它会把资产目录与提供方的技能名对上。

`locale/*.json` 提供各界面语言下的组件标题与描述，供插件清单使用。

</details>

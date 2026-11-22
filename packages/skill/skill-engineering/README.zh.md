---
description: "Harness 内置的工程工作流技能：内置目录条目、调用方式划分、插件元数据，以及维护者需要更新的资源根目录。"
kind: "package-reference"
---

# @deepseek-ai/dsh-skill-engineering

[English](README.md) | 中文

## 概述

该提供方提供 Harness 内置的工程工作流技能：27 个技能覆盖拷问、spec 与工单、实现、测试驱动开发、代码评审、领域建模、架构评审，以及围绕它们的较小工作流。一份全新的 checkout 无需单独安装、符号链接或插件下载就能使用它们。27 个都出现在输入框的 `/` 菜单里；其中 11 个同时由模型调用并出现在会话目录中，另外 16 个仅限用户，模型无法自行加载。技能文本取自 [mattpocock/skills](https://github.com/mattpocock/skills) 的 `main` 分支 v1.3.2（`abeeaf9`），以 MIT 许可使用，并带有开发备注里列出的少量 Harness 措辞调整。

## 目录

- [使用本包](#use-this-package)
- [理解实现](#understand-the-implementation)
- [进一步探索](#further-exploration)
- [模型体验](#model-experience)
- [已知限制与延期工作](#known-limitations-and-deferred-work)
- [开发备注](#dev-note)

-----

<a id="use-this-package"></a>
## 使用本包

base bundle 会插入该提供方，因此每个以 base 为基础的 profile（`web`、`headless`、`acp`、`sdk`）都自带这些技能，无需任何配置。该提供方以 provider 名 `dsh-engineering` 注册进全局技能层，排名为随包技能（bundled）的 600，并声明 `shipped`，使其技能与 harness 其余技能归为一组。

### 目录包含什么

| 可调用方 | 技能 |
|---|---|
| 模型与用户 | `eng-code-review`、`eng-codebase-design`、`eng-diagnosing-bugs`、`eng-domain-modeling`、`eng-grilling`、`eng-pr`、`eng-prototype`、`eng-research`、`eng-tdd`、`eng-wizard`、`eng-writing-for-agents` |
| 仅用户 | `eng-ask-matt`、`eng-grill-me`、`eng-grill-with-docs`、`eng-handoff`、`eng-implement`、`eng-implement-spec`、`eng-improve-codebase-architecture`、`eng-retro`、`eng-setup-matt-pocock-skills`、`eng-teach`、`eng-to-questionnaire`、`eng-to-spec`、`eng-to-tickets`、`eng-triage`、`eng-wait-what`、`eng-wayfinder` |

随包技能名都带 `eng-` 前缀。项目或用户目录里的技能保留上游原名，若随包副本不加前缀，就会被同名个人副本替换，内置技能从 `/` 菜单里消失；加前缀后两者都能被寻址，菜单也能把随包这一组归到 `mattpocock/skills` 之下。

`eng-ask-matt` 是路由：它把仅限用户的技能映射到主流程（`eng-grill-with-docs` → `eng-to-spec` → `eng-to-tickets` → `eng-implement` → `eng-pr` → `eng-retro`）、入口匝道（`eng-triage`、`eng-diagnosing-bugs`、`eng-wayfinder`）与词汇技能（`eng-domain-modeling`、`eng-codebase-design`）。`eng-setup-matt-pocock-skills` 写入工程技能读取的仓库级配置：问题跟踪器、分诊标签与领域文档布局。

来自更近层的同名技能优先：某个 preset 的本地发现（项目的 `.dsh/skills`、项目的 `.agents/skills`、配置的自定义目录或用户技能目录）会取代该提供方提供的目录条目与正文。在同一层内，bundled 排名排在最后。

### 配置

| 字段 | 默认值 | 含义 |
|---|---|---|
| `assetRoot` | 随包的 `assets/` | 存放技能的绝对目录，每个名称一个目录。设为重定位后的副本时，测试就是这么做的。 |

```yaml
- id: skill-engineering
  name: '@deepseek-ai/dsh-skill-engineering'
  config:
    assetRoot: /opt/skills/engineering
```

相对路径的 `assetRoot` 会在加载时失败。若要从某个 profile 移除这些技能，把 base 行替换为 `disabled: true`。

### 可观察的成功与失败

每个随包技能在目录中出现一次，带自己的描述和 `upstream` 项目 `mattpocock/skills`，加载自己的指令正文，并把相对引用（`tests.md`、`PHASE-BOUNDARIES.md`、`template.sh`）解析到自己的目录。缺失或无效的 `SKILL.md` 会从完整目录中省略，并通过 `ctx.logger.warn` 报告一次。其他读取错误会让发现处于不完整状态，注册表会重试；加载会拒绝，而不是返回部分正文。销毁插件会移除所有候选。

-----

<a id="understand-the-implementation"></a>
## 理解实现

<details>
<summary>实现内部机制——点击展开</summary>

该提供方持有固定的技能目录名列表，在发现阶段读取 `assets/<name>/SKILL.md`，每次加载时再读一次，并在发布候选前校验 frontmatter 契约。目录名即身份：声明的 `name` 与目录名不一致就是无效元数据。`description` 必填且非空。`disable-model-invocation` 与 `user-invocable` 是可选布尔值，会投影到注册表的调用策略；随包文件对那 16 个仅限用户的技能使用了前者。

该提供方通过延迟的 `skills` 注入挂载，因此移除技能注册表的 profile 会让该行保持空闲，而不是报告待处理的依赖。

失败分为两类。`ENOENT` 与 `ENOTDIR` 会持续到文件出现为止，因此该技能从完整目录中省略，加载在使目录失效后返回 `undefined`。YAML、字段或命名错误同样持续存在。其他任何读取错误都会让这次观察不完整，并从加载中抛出。

| 文件 | 职责 |
|---|---|
| [`src/index.ts`](src/index.ts) | 插件入口、frontmatter 契约、提供方注册、失败报告 |
| [`assets/`](assets/) | 每个技能一个目录，以及上游许可证与各技能的参考文件 |

</details>

-----

<a id="further-exploration"></a>
## 进一步探索

- [技能子系统参考](../../../docs/subsystems/skills.zh.md) —— 该提供方接入的注册表、提供方契约与目录渲染。
- [skill](../skill/README.zh.md) —— 注册表与调用策略类型。
- [tool-skill](../tool-skill/README.zh.md) —— 模型使用的目录与加载器。
- [skill-office](../skill-office/README.zh.md) —— 另一个随包提供方，失败约定相同。
- [mattpocock/skills](https://github.com/mattpocock/skills) —— 上游英文源与其设计理由。

-----

<a id="model-experience"></a>
## 模型体验

间接地，经由 `dsh-tool-skill`，由它渲染该提供方的目录条目与加载后的技能正文。

#### KV Cache 影响

11 个模型调用的条目把描述加入会话目录；加载后的正文在加载点进入对话。仅限用户的技能在被引用前不贡献任何内容。

## 已知限制与延期工作

<a id="known-limitations-and-deferred-work"></a>

这些限制界定了该随包提供方不做的事。它们是当前的包约束，不是任务清单。

- **只有英文** —— 技能文本是上游英文语料；插件元数据已本地化，但目录与正文没有，因此本地化目录需要本地覆盖。
- **手动同步** —— 上游变更通过复制晋升桶、重新套用下面的改编步骤并更新固定名称列表进入本包；没有构建步骤跟踪上游仓库。
- **没有按技能配置** —— 该提供方只暴露一个资源根目录，并且总是发布随包集合；想要子集的部署方应自己编写提供方。
- **这些技能假设存在工作区** —— 工程技能会读取 `GLOSSARY.md`、ADR 与问题跟踪器；`eng-setup-matt-pocock-skills` 写入这些配置，没有它时技能会要求运行它。

<a id="dev-note"></a>
### 开发备注

<details>
<summary>维护者工作背景——点击展开</summary>

该包是本仓库的一部分；技能文本同步自 [`mattpocock/skills`](https://github.com/mattpocock/skills)，分支 `main`，提交 `abeeaf9`（v1.3.2）；`yanfeng98/nano-mattpocock-skills` 的 `main` 与上游文本逐字节一致；本包副本的差异只有 `eng-` 前缀与下面的改编。重新同步时：

1. 把每个晋升技能目录以 `eng-<name>` 之名复制进 `assets/`，排除 Codex 的 `agents/openai.yaml`，并把 frontmatter 的 `name` 改成同一个带前缀的名字。
2. 重新套用 Harness 改编：把 `Skill tool` 改名为 `skill` 工具，把 `/clear` 改为开新会话（Harness 有 `/compact` 但没有 `/clear`），把所有 `/名字` 命令与 `skill` 工具引用改成带前缀的名字，并去掉安装器、市场清单、桶 README 与 Codex 的 `agents/openai.yaml`。
3. 更新 `src/index.ts` 里的 `SKILL_NAMES`；漂移测试会把该列表与资源目录比对。
4. 运行该包的测试与包 README 门禁。

`assets/LICENSE` 保留上游的 MIT 声明，`locale/*.json` 提供各 UI 语言的组件标题与描述，供插件清单使用。

</details>

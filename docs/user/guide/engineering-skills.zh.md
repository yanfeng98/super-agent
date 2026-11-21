# 使用内置工程技能

[English](engineering-skills.md) | 中文

每个以 base 为基础的 profile 都自带 27 个工程工作流技能，因此新建会话即可使用，无需安装插件或复制文件。该组件随 base 组合一起发布，插件元数据里的名称是 **工程工作流技能**；技能文本来自上游 [mattpocock/skills](https://github.com/mattpocock/skills) 项目。

## 内置内容

11 个技能由模型调用，出现在会话技能目录中，任务合适时由模型加载。16 个由用户调用：在输入框里输入 `/` 后选择。

| 调用方式 | 技能 |
|---|---|
| 模型调用 | `eng-code-review`、`eng-codebase-design`、`eng-diagnosing-bugs`、`eng-domain-modeling`、`eng-grilling`、`eng-pr`、`eng-prototype`、`eng-research`、`eng-tdd`、`eng-wizard`、`eng-writing-for-agents` |
| 用户调用 | `eng-ask-matt`、`eng-grill-me`、`eng-grill-with-docs`、`eng-handoff`、`eng-implement`、`eng-implement-spec`、`eng-improve-codebase-architecture`、`eng-retro`、`eng-setup-matt-pocock-skills`、`eng-teach`、`eng-to-questionnaire`、`eng-to-spec`、`eng-to-tickets`、`eng-triage`、`eng-wait-what`、`eng-wayfinder` |

## 调用技能

在输入框里输入 `/`，即可列出用户调用的技能并按名称插入。菜单按技能出处分组：这 27 个排在 `mattpocock/skills` 之下，与项目目录、用户目录提供的技能各自成组。`eng-ask-matt` 是路由：它会按你的处境推荐技能或流程，拿不准时从它开始。

对于模型调用的技能，描述任务即可，模型会加载合适的那个；想指定某个技能时，在消息里点名。

## 主流程

这些技能组合成一条从想法到落地的路径：`eng-grill-with-docs` 把想法磨清并记录领域语言，`eng-to-spec` 与 `eng-to-tickets` 把它变成工作，`eng-implement` 驱动 `eng-tdd` 并以 `eng-code-review` 收尾，`eng-pr` 与 `eng-retro` 完成这次改动。`eng-triage`、`eng-diagnosing-bugs` 与 `eng-wayfinder` 分别是外部 issue、难缠 bug 和单会话装不下的大工程的入口。

## 仓库配置

工程技能会读取 `GLOSSARY.md`、ADR 与问题跟踪器。在每个仓库运行一次 `/eng-setup-matt-pocock-skills`，把跟踪器、分诊标签与文档布局记录到 `docs/agents/` 下。

## 覆盖或移除

项目 `.dsh/skills`、`.agents/skills` 或用户技能目录里的同名技能会覆盖随包版本。因为随包技能名带 `eng-` 前缀，用上游原名的个人副本（例如 `tdd`）会与 `eng-tdd` 并存，而不是替换它，两者都能用。要从某个 profile 移除整个组件，在该 profile 的 patch 里禁用它的行：

```yaml
- id: skill-engineering
  disabled: true
```

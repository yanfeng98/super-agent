# 使用内置工程技能

[English](engineering-skills.md) | 中文

每个以 base 为基础的 profile 都自带 27 个工程工作流技能，因此新建会话即可使用，无需安装插件或复制文件。该组件随 base 组合一起发布，插件元数据里的名称是 **工程工作流技能**；技能文本来自上游 [mattpocock/skills](https://github.com/mattpocock/skills) 项目。

## 内置内容

11 个技能由模型调用，出现在会话技能目录中，任务合适时由模型加载。16 个由用户调用：在输入框里输入 `/` 后选择。

| 调用方式 | 技能 |
|---|---|
| 模型调用 | `code-review`、`codebase-design`、`diagnosing-bugs`、`domain-modeling`、`grilling`、`pr`、`prototype`、`research`、`tdd`、`wizard`、`writing-for-agents` |
| 用户调用 | `ask-matt`、`grill-me`、`grill-with-docs`、`handoff`、`implement`、`implement-spec`、`improve-codebase-architecture`、`retro`、`setup-matt-pocock-skills`、`teach`、`to-questionnaire`、`to-spec`、`to-tickets`、`triage`、`wait-what`、`wayfinder` |

## 调用技能

在输入框里输入 `/`，即可列出用户调用的技能并按名称插入。`ask-matt` 是路由：它会按你的处境推荐技能或流程，拿不准时从它开始。

对于模型调用的技能，描述任务即可，模型会加载合适的那个；想指定某个技能时，在消息里点名。

## 主流程

这些技能组合成一条从想法到落地的路径：`grill-with-docs` 把想法磨清并记录领域语言，`to-spec` 与 `to-tickets` 把它变成工作，`implement` 驱动 `tdd` 并以 `code-review` 收尾，`pr` 与 `retro` 完成这次改动。`triage`、`diagnosing-bugs` 与 `wayfinder` 分别是外部 issue、难缠 bug 和单会话装不下的大工程的入口。

## 仓库配置

工程技能会读取 `GLOSSARY.md`、ADR 与问题跟踪器。在每个仓库运行一次 `/setup-matt-pocock-skills`，把跟踪器、分诊标签与文档布局记录到 `docs/agents/` 下。

## 覆盖或移除

项目 `.dsh/skills`、`.agents/skills` 或用户技能目录里的同名技能会覆盖内置版本。要从某个 profile 移除整个组件，在该 profile 的 patch 里禁用它的行：

```yaml
- id: skill-engineering
  disabled: true
```

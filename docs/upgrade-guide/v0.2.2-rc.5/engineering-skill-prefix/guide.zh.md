---
kind: upgrade-guide
description: "随包工程技能改为 eng- 前缀，`/tdd` 与 `skill({ name: 'tdd' })` 不再加载内置技能。"
---

# 随包工程技能加 `eng-` 前缀

[English](guide.md) | 中文

## 变更

`skill-engineering` 插件在此版本之前分发的每个技能都加上 `eng-` 前缀：`tdd` 变成 `eng-tdd`，`code-review` 变成 `eng-code-review`，`grill-with-docs` 变成 `eng-grill-with-docs`，整个集合都是如此。提示词正文、描述与调用划分都没有变化。同一版本还新增了来自上游 Beta 桶的五个技能——`eng-loop-me`、`eng-setup-ts-deep-modules`、`eng-writing-beats`、`eng-writing-fragments` 与 `eng-writing-shape`——它们只有带前缀的名称。

前缀把随包文本与同名个人副本区分开：后者可能位于项目根（`./.dsh/skills`、`./.agents/skills`）或用户根（`~/.dsh/skills`、`~/.agents/skills`）。注册表把重名解析到最近的层，因此改名之前个人副本会替换随包技能，随包技能根本不会出现在 `/` 菜单里；现在两者都能寻址。

输入框的 `/` 菜单还按来源分组：随包一组是 `内置 · mattpocock/skills`，没有上游项目的随包提供方是 `内置`，插件在运行时注册的技能是 `插件 · <插件名>`，文件系统发现根则是 `项目 · <根目录>` 或 `用户 · <根目录>`，配置的自定义目录是 `自定义`。输入查询后仍回到共享的名称排序，不显示分组标题。

`skills/list` Remote 另外报告贡献该技能的插件（`owner`）以及该插件是否随 harness 分发（`shipped`）；`SkillProvider` 可以声明 `shipped`，运行时注册在其 scope 没有插件名时可以自行声明 `owner`。

## 迁移

1. 把此前随包的命令换成 `eng-` 名称：`/tdd` 变成 `/eng-tdd`，`/ask-matt` 变成 `/eng-ask-matt`。[工程技能指南](../../../user/guide/engineering-skills.zh.md)与[技能包 README](../../../../packages/skill/skill-engineering/README.zh.md)列出了完整清单。
2. 其他按名字引用随包技能的地方同样处理：提示词、`AGENTS.md`、脚本，以及 `skill({ name })` 调用。
3. 重新加载 profile，在输入框输入 `/`。随包技能出现在 `内置 · mattpocock/skills` 之下，与你的副本并存，而不是替换它们。

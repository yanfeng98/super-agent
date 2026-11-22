# Agent Note: Built-in engineering workflow skills

Status: implemented

[English](2026-11-20-built-in-engineering-workflow-skills.md) | 中文

## 问题

本次会话用到的工程工作流技能（拷问、spec、工单、TDD、代码评审、领域建模）原本不属于 Harness。它们能进入会话，只因为用户的 `~/.agents/skills` 与 `~/.claude/skills` 里存有指向上游 `yanfeng98/nano-mattpocock-skills` 另一份 clone 的符号链接，而那是 [mattpocock/skills](https://github.com/mattpocock/skills) 的简体中文 fork。这带来三个后果：

| 症状 | 原因 |
|---|---|
| 在别处全新 clone 的仓库一个这样的技能都没有 | 分发方式是本地符号链接，指向外部仓库 |
| 禁用或移动那份 clone，所有会话里的技能就静默消失 | 目录来自 `skill-filesystem` 的用户根目录，而不是随包提供方 |
| 已安装的文本与正在评审的仓库产生漂移 | 语料在 checkout 之外存在第二份副本 |

要求是：clone 本仓库后自带这些技能并自动开启，不再需要那个外部项目。

## 决定

用一个专门的内置提供方把语料装进 Harness，并由 base bundle 挂载。

- **包。** `@deepseek-ai/dsh-skill-engineering` 位于 `packages/skill/skill-engineering/`，以 provider `dsh-engineering` 注册到 `ctx.skills`，排名为随包技能的 600，`source: 'bundled'`。它沿用 `dsh-skill-office` 的提供方形态：固定的目录名列表、随包的 `assets/` 根目录、每次加载时重新读取。
- **组合。** `packages/bundle/base/cordis.patch.yml` 中的 `skill-engineering` 行使每个以 base 为基础的 profile（`web`、`headless`、`acp`、`sdk`）都能零配置挂载它。profile 把该行替换为 `disabled: true` 即可移除。该提供方通过延迟的 `skills` 注入挂载，因此禁用技能注册表的组合会让该行保持空闲，而不是报告待处理的服务。
- **内容。** 上游 [`mattpocock/skills`](https://github.com/mattpocock/skills) `main`（提交 `abeeaf9`，v1.3.2）`engineering/`、`productivity/` 与 `in-progress/` 三个桶中的 32 个技能。其中 11 个由模型调用，出现在会话目录；21 个带 `disable-model-invocation: true`，只能通过 `/` 技能选择器触达。`ask-matt` 仍是用户调用集合的路由。七个 Beta 技能里有五个随包提供，因为它们不依赖特定 harness 的工具；`claude-handoff` 落选是因为正文要驱动 Claude Code 的 `claude --bg` CLI，`chief-of-staff` 落选是因为会话目标与子 agent 已经覆盖了同一件事。
- **frontmatter 契约。** 目录名就是技能身份，因此声明的 `name` 与目录名不一致属于无效元数据。`description` 必填。`disable-model-invocation` 与 `user-invocable` 是可选布尔值，会投影到注册表的调用策略。
- **失败报告。** 缺失（`ENOENT`/`ENOTDIR`）或无效的技能会从完整目录中省略，只报告一次；加载会先使目录失效再返回 `undefined`。其他任何读取错误都会让发现不完整，并从加载中抛出。
- **优先级。** 本地发现注册到更近的 preset 层，因此同名的本地技能胜出；随包排名只决定同一层内的顺序。于是项目里或用户目录里已有的 `tdd` 无需配置即可覆盖随包版本。
- **针对本 Harness 的改编。** 随包正文点名 `skill` 工具，去掉 Codex 的 `agents/openai.yaml` 与插件清单，并把 `/clear` 改为开新会话（Harness 有 `/compact` 但没有 `/clear`）。机器标识符、名称、斜杠命令、标签、路径与模板字段名都保持上游原样。
- **呈现。** `locale/*.json` 提供八种 UI 语言下的组件标题与描述，供插件清单使用；`docs/user/guide/engineering-skills.md` 面向用户说明调用方式、主流程、仓库配置与覆盖方法。
- **来源。** 资源来自 `mattpocock/skills` 的 `main` 分支提交 `abeeaf9`（v1.3.2）；`yanfeng98/nano-mattpocock-skills` 的 `main` 与该分支逐字节一致。`assets/LICENSE` 保留上游的 MIT 声明。重新同步的步骤由包的 README 负责。

## 考虑过的替代方案

- **保留符号链接安装。** 那是现状。它之所以落选，是因为它需要第二份 clone、在全新 checkout 上失效，并让目录依赖用户主目录。
- **填充 preset 的 `skills/` 目录，而不是新建包。** `packages/preset/agent-preset/skills/` 只能触达把它挂载为自定义目录的 preset，headless 与 SDK 组合会漏掉这些技能，而且第三方内容会住进 preset 包自己的资源树。提供方包能进入每个 profile 的全局层。
- **像 `upstream-skills` 那样在启动时下载语料。** 它之所以落选，是因为需要网络和上游主机，而要求是 clone 下来即可离线使用、不依赖外部项目。
- **沿用上游的插件/市场路线。** 它之所以落选，是因为需要显式安装步骤，与默认开启相矛盾。
- **改为随包中文译本。** `yanfeng98/nano-mattpocock-skills` 的 `260922-dev` 分支提供了同一份语料的简体中文版。它之所以落选，是因为要求的来源是上游 `main`，而且多一层翻译会引入第二份需要跟随上游维护的语料。
- **给名称加前缀（`mp-tdd`）。** 它之所以落选，是因为每个跨技能引用、`/name` 与上游文档都使用不带前缀的名字；改名会分叉语料并提高同步成本，而同名冲突已经由分层优先级解决。

## 后果

语料现在随仓库走、可离线加载，也不会因为移动主目录里的 clone 而消失。

仍然存在的代价：

- 每个以 base 为基础的 profile 会多出 11 个目录条目（只有描述；正文按需加载）和一行插件。
- 同步是手动的：复制选定的目录、重新套用改编、更新 `SKILL_NAMES` 列表，都是可评审的 diff，而不是构建步骤。
- 五个随包技能来自上游的 Beta `in-progress/` 桶，上游可能不加预告地改名或删除，因此重新同步时可能移除其中一个。
- 随包文本可能相对上游 `main` 漂移，且没有门禁能发现；漂移测试只比对资源目录与提供方的固定列表。
- 内容只有英文，因此其他语言的会话收到的是英文描述与正文。
- 用户安装的同名技能会静默遮蔽随包版本；这是预期的优先级，但排查时可能出人意料。

## 测试

`packages/skill/skill-engineering/tests/skill-engineering.spec.ts` 以语句、分支、函数与行四个维度 100% 覆盖提供方契约：目录内容与调用策略、随包正文加载、资源目录漂移、经由 Loader 的真实 `cordis.yml` 组合、持久与瞬态失败报告、去重、销毁与中止。`verify-config-catalog` 记录 `assetRoot` 字段。既有的无密钥场景会重新发布这 11 个目录条目；由于回放输入仍然有效，`pnpm run test:snapshot:refresh` 已重新生成它们。

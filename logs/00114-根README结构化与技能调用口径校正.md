# 00114 — 根 README 结构化校正与技能调用口径

- **日期**：2026-11-22
- **背景**：根 README 的「新增与行为调整」表在三次追加后有三行掉到表外——工程技能、rust-anything 与 pi-ai 三行前各留了一个空行，Markdown 不再把它们当作表格行；同时 `eng-` 改名与菜单分组那一轮写进文档的说法与实现不符：`/` 菜单列出的是全部 27 个技能而不只是 16 个，用户技能根的标题带 `~`，office 的 4 个技能落在没有来源后缀的 `内置` 分组。
- **范围**：只改文档，不改代码。

## 1. 根 README

| 位置 | 修正 |
|---|---|
| 「使用」的 `/` 条目 | 说明 `/` 菜单列出全部 27 个工程技能，其中 16 个模型看不到、菜单里标着「仅用户」；标题清单补齐 `内置`（office 4 个）、`插件 · dsh-superagent` 与 `用户 · ~/.agents/skills`；补一句「输入查询后标题收起」 |
| 「内置能力」表 | 「`/` 菜单可点名 16 个」改为 27 个都可点名、其中 11 个还能由模型自动加载 |
| 「新增与行为调整」表 | 去掉三行前的空行，重新并入表格；工程技能行的调用划分同步改写 |

## 2. 出处口径

依据的实现行为：

- `parseSkill` 把缺省的 `userInvocable` 设为 `true`，27 个随包技能没有任何一个声明 `user-invocable: false`，因此全部进入 `/` 菜单；`userInvocable` 与 `modelInvocable` 互不排斥，`eng-tdd` 一类是两者皆真（见 `tests/skill-engineering.spec.ts` 的策略断言）。
- `SessionSkillCatalog.list` 只按 `isUserInvocable` 过滤。
- `ui-skill` 客户端给 `modelInvocable: false` 的条目加上 `menu.userOnly`（中文「仅用户」）描述前缀。
- `ROOT_LABELS` 把 `user-agents` 标成 `~/.agents/skills`，`project-agents` 标成 `.agents/skills`。

| 文件 | 修改 |
|---|---|
| `docs/user/guide/engineering-skills.md` / `.zh.md` | 调用划分表改为「模型与用户 / 仅用户」；菜单标题补 `~`；入口句不再说菜单只列用户专用技能 |
| `packages/skill/skill-engineering/README.md` / `.zh.md` | 摘要与目录表同步改写，统一用 user-only 指模型看不到的那 16 个 |
| `packages/client/ui-skill/README.md` / `.zh.md` | 标题清单补 `~` 与「命名不出来源时只显示类别」；标题清单与代码的 `heading(label, source)` 一致 |
| `docs/upgrade-guide/v0.2.2-rc.5/engineering-skill-prefix/guide.md` / `.zh.md` | 用户技能根标题补 `~` |

## 3. 验证

| 检查 | 结果 |
|---|---|
| `pnpm run test:docs` | 21/21 通过 |
| `pnpm run doc-sync` | 43/43 通过（含文档站构建与死链检查） |
| `pnpm run verify-translation-pairing --write`（4 对） | 四份配对记录已重录 |

## 4. 取舍

调用划分保留「11 / 16」这组数字，因为它描述的是模型可见性；但不再暗示菜单只有 16 个。菜单标题照实写全，包括没有来源后缀的 `内置`：office 技能组没有上游项目，标题就只有类别。

## 5. 复核后修正

第一轮提交后按源码复核，改掉两处口径问题（另修正两处中文措辞）：

| 问题 | 依据 | 修正 |
|---|---|---|
| README 把 `内置 · mattpocock/skills` 写在 `内置`（office）之前 | `compareSections` 先比 rank，再按标题字符串比较；`内置` 是 `内置 · mattpocock/skills` 的前缀，排在前面 | README 先写类别顺序（内置 → 插件 → 项目 → 用户 → 自定义），再按该顺序举出 `内置`（office 4 个）、`内置 · mattpocock/skills`（工程技能）等标题 |
| 文件系统根的标题写成具体的 `.agents/skills` 与 `~/.agents/skills`，像是只有这两个根 | 标题按发现根取名，`ROOT_LABELS` 有四个：`.dsh/skills`、`.agents/skills`、`~/.dsh/skills`、`~/.agents/skills` | `ui-skill` README 与升级指南改成 `项目 · <根目录>` / `用户 · <根目录>` 并列出四个根 |

另外用一次真实组合验证了「全部 27 个都在 `/` 菜单里」：在 `skill-engineering` 的测试目录临时写一个 spec，用 `Context` 挂上真实的 `SkillRegistry`、`skill-engineering` 与 `skill-office`，`ctx.skills.list()` 得到 31 条——27 个 `eng-*` 全部 `userInvocable`（其中 11 个 `modelInvocable`），4 个 office 条目是 `source=bundled`、`shipped=true`、没有 `upstream`。断言通过后删掉该临时文件，工作区无残留。

## 6. 再复核

第二遍核对没有再发现口径错误，改用机械比对把结论钉住：

| 检查 | 做法 | 结果 |
|---|---|---|
| 文档里的技能清单 | 用脚本解析四份文档的表格，与 `assets/eng-*` 目录及每份 `SKILL.md` 的 `disable-model-invocation` 逐个比对 | 27 个技能、11 个模型可调用、16 个仅用户，四份文档的名字与顺序都与 assets 一致 |
| 根 README 的表格 | 用 `marked` 渲染成 HTML 后数表格与行数 | 4 个表格、分别 5/11/3/7 行（表头加数据），没有落在表格外的竖线段落 |
| 分组标题与顺序 | 跑 `browser-plugin.client.spec.ts` | 35 个用例通过，含「`内置` 排在 `内置 · mattpocock/skills` 之前」与「筛选后不带标题」 |
| 真实 profile 挂载 | `pnpm dsh web --dump-config` | web profile 挂载 `skill-filesystem`（自定义目录指向 agent-preset 包的 `skills`，即 `dsh-doc`）、`skill-engineering`、`skill-office`，与文档列出的六个标题一一对应 |

顺带把「使用」里那条过长的 `/` 条目拆成两条：一条讲菜单内容与 `eng-` 前缀为什么不会互相覆盖，一条讲分组标题与「输入查询后收起」；「按名称排序」改为更准确的「按名称匹配排序」。

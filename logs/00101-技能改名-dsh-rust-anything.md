# 00101 — 技能改名为 `dsh-rust-anything`

- **日期**：2026-11-21
- **背景**：用户提出「给这个项目的 skill 和命令加个标识，比如有个前缀，人工调用的时候更友好」。改名前仓库里 19 个 DSH 自有技能都以 `dsh-` 开头，另有 4 个不带前缀（`agent-experience`、`cordis-styling-nitpicking`、`record-browser-gif`、`rust-anything`）。在输入框敲 `/` 时，`rust-anything` 与 DSH 内置技能混在一起，看不出它属于本仓库的 vendored 集成。
- **结论**：只改技能名，`rust-anything` → `dsh-rust-anything`。CLI 命令、MCP 工具名、`pnpm run build:rust-anything` 脚本保持原样。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `.agents/skills/dsh-rust-anything/` | 目录由 `rust-anything` 重命名（`git mv`，两份文件 `SKILL.md`、`whole-app.md` 内容除 frontmatter 外未动） |
| `.agents/skills/dsh-rust-anything/SKILL.md` | `name: rust-anything` → `name: dsh-rust-anything` |
| `README.md` | 差异表该行的技能链接改名为 `dsh-rust-anything` |
| `docs/user/guide/rust-anything.md` / `.zh.md` | 技能路径改为新目录；「提出迁移需求」段补一句：在输入框输入 `/` 选 `dsh-rust-anything` 可先加载完整流程再描述任务 |
| `packages/mcp/rust-anything/README.md` / `.zh.md` | 技能链接改为新目录并点名 `dsh-rust-anything` |
| `rust-anything-integration.zh.md` | 「技能」行路径更新 |
| 上述两个双语对的 `.i18n.yaml` | 用 `pnpm run verify-translation-pairing --write` 重新录制分节哈希 |

## 2. 行为

- **人工调用**：`.agents/skills/` 下的技能默认 `user-invocable: true`（`packages/skill/skill-filesystem` 的 `parseInvocationPolicy` 只在显式 `user-invocable: false` 时才关闭）。所以在本仓库目录下打开 Web UI，输入 `/` 会列出 `dsh-rust-anything`，选中即加载整份迁移流程；也可以照旧只描述任务让模型按描述命中。
- **模型调用**：描述里的触发条件未变，`analyze`/`bridge`/`record`/`gate` 这条链路的指令未改；会话技能目录里的条目名从 `rust-anything` 变成 `dsh-rust-anything`。
- **不动的三处**：CLI 二进制仍叫 `rust-anything`（vendored 源码保持与上游 `v2.1.0` 一致，`NATIVE.md` 的同步流程依赖这一点）；MCP 工具仍是 `mcp__rust-anything__*`（由 `serverName` 决定，改名会牵动 preset 与文档）；`pnpm run build:rust-anything` 脚本名不变。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm run test:docs` | 21/21（含 `verify-md-links`、`verify-translation-pairing`、`verify-skill-invocation-metadata`、`verify-doc-budgets`） |
| `DSH_GATE_CONCURRENCY=4 pnpm run doc-sync` | 43/43 |
| `pnpm run verify-translation-pairing --write docs/user/guide/rust-anything.md`、`... packages/mcp/rust-anything/README.md` | 两对 `.i18n.yaml` 重新录制，复查通过 |
| 全仓库 `skills/rust-anything` 扫描 | 除 `logs/00097`、`logs/00101` 的记录性引用，以及 `rust-anything-analysis.zh.md` 指上游 `skills/rust-anything/` 的那一处，没有仍指向旧技能目录的活链接 |

## 4. 取舍

- **只加技能前缀，不改其余三处**：用户选定的范围就是技能名。CLI 与工具名是上游契约与本仓库挂载契约的一部分，改名会让 vendored 源码与上游基线产生非必要差异，也会让 `NATIVE.md` 的同步流程多一条本地改写。
- **用 `dsh-` 而不是新造前缀**：与其余 19 个 `dsh-` 技能一致，`/` 补全列表里按前缀聚在一起，一眼能分出「DSH 自带」与「其他」。
- **不改 `SKILL.md` 正文里的 CLI 用法行**：那些行说的是二进制 `rust-anything`，与技能名无关。
- **历史日志不改**：`logs/00099`、`logs/00100` 记录的是当时的名字，作为历史保留；本条记录改名本身。

## 5. 复查

改完自查两轮：第一轮修掉一处遗漏，第二轮修掉日志自身的计数与扫描结论偏差，并确认两处判断：

| 检查 | 结论 |
|---|---|
| `native/rust-anything/NATIVE.md` 的技能链接 | **失效**：仍指向 `.agents/skills/rust-anything/SKILL.md`。已改为 `dsh-rust-anything`。`verify-md-links` 没报，是因为它只扫 `README*.md`、`docs/**`、`packages/**`、`AGENTS.md`、`.agents/skills/**`（见 `scripts/verify-md-links.ts` 的 `PATTERNS`），`native/**` 与根目录非 README 的 `.md` 不在范围内，vendored 目录里的自写文档需要手工维持链接 |
| `/` 菜单能否列出该技能 | 能。`packages/skill/skill-filesystem` 的 `parseInvocationPolicy` 只在显式 `user-invocable: false` 时关闭用户调用，本技能未写该字段；`packages/api/session-controller/src/skill-catalog.ts` 的 `list()` 只按 `isUserInvocable` 过滤，没有「仅内置」这类额外限制，所以 `.agents/skills` 下的项目技能同样进 `/` 列表 |
| 是否出现两个同名技能 | 没有。改名后会话技能目录里只有 `dsh-rust-anything`，说明没有其它来源（内置或上游打包）再提供一份 `rust-anything`；否则旧名被本项目技能覆盖的现象会随改名暴露成两条 |
| 提交形状 | 提交 `refactor(skills): prefix the rust-anything skill with dsh-` 把两份文件识别为纯重命名（`SKILL.md` 相似度 99%，只差 frontmatter 一行；`whole-app.md` 100%），其余 10 个文件是 7 处路径与措辞更新、2 份 `.i18n.yaml` 哈希重录和 1 份新日志 |
| 日志自身的计数与扫描结论 | 第二轮：原写的「其余 9 个文件」在 amend 补入 `NATIVE.md` 后应为 10 个；「全仓库扫描只剩一处」漏掉了 `logs/00097` 里的历史记录。两处已改；英文指南那句插入语也改成 `to load it first` 并重录配对哈希 |

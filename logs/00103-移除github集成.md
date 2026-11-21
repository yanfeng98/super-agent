# 00103 — 移除 dsh-superagent 的 GitHub 集成

- **日期**：2026-11-21
- **背景**：用户确认只用本地工单，要求把 GitHub 那一套直接删掉，而不是仅关闭。目标是保留本地 Source、面板/聊天建单、本地合并、检查、队列、回退与清理，同时不再出现 GitHub Source、账号探测、PR 绑定、PR CI 门禁、GitHub 技能与相关提示词。
- **范围**：`apps/web/native/dsh-superagent`（源码、提示词、技能、测试、文档、`cordis.patch.yml`、`package.json`）、`packages/bundle/web-app`（bundle patch 与测试）、`apps/web/tests/scaffold.ts`、根 `README.md`。
- **最终状态**：本版本不含任何 GitHub Source、GitHub 专用迁移或启动清理；旧 stateDir 中的 GitHub 记录不会被解释或清理。

## 1. 变更

| 区域 | 变更 |
|---|---|
| GitHub Source | 删除 `src/sources/github.ts`、`github-pull.ts`、`github-read.ts`、`github-trigger.ts`、`github-channel.ts`，以及 `skills/dsh-superagent-github`；`tsdown.config.ts`、`package.json` 的 `./github-watch`、插件与 bundle 的 `dsh-superagent-github` 挂载全部移除。 |
| 部署与账号 | `Deployment` 只保留 machine 字段与可选 `feishu`；删除 `GithubAccountsSchema`、`resolveDeployment`、`SETUP_FIELDS.github`、GitHub 的 `SetupError` 码、`stateView.accounts` 的 `repository`/`agent`；`Setup` 只探测可选 Feishu bot。 |
| Worker 环境 | `writeEnvironment` 不再写 GitHub 凭据 helper、SSH 重写或 agent 账号；固定中性提交身份 `dsh-superagent <dsh-superagent@localhost>`。`gh` 仍是一个失败 shim，避免 worker 误用操作者本机的真实 `gh`；`GH_TOKEN`/`GITHUB_TOKEN` 仍被清除，作为凭据隔离而不是 GitHub 集成。 |
| 合并门禁 | 移除绑定 PR 的 CI 检查、PR 关闭/合并通知、`orchestrator_report.pr_number`、PR 绑定与 `orchestrator_eval_read` 的仓库文件/目录读取；本地门禁保留：评审结论、`requireReview`、未解决备注、配置检查、依赖、目标包含、变基、队列、合并后目标检查。 |
| 提示词与技能 | 主 Session、worker、work、dispatch、subscriptions、replies、cli、handoff、team/communication、team/delivery、team/scope 去掉 GitHub/PR/CI/自动合并/评审 bot 内容；内置技能只剩 Feishu；`skill/github` 与 `sourceSkills.github` 删除。 |
| 面板与聊天 | 删除 GitHub 来源卡片、账号字段、PR 图标、GitHub 搜索提示与相关文案；`orchestrator_read` 不再暴露 `github/…`；`outcome.merged`/`closed` 作为本地合并结果保留，所有 ticket 按当前状态展示。 |
| 迁移与清理 | 无 GitHub 专用迁移或启动清理：schema 10→11 只处理其余结构变化；`ticketOf` 不再过滤 GitHub follows/channels；启动不再扫描旧 GitHub 数据。带 GitHub 记录的旧数据库不会被清理，需要新 stateDir。 |
| 文档 | 插件 `README.md`、`SETUP.md`、`NATIVE.md`、`docs/000`–`008`、`examples` 与根 `README.md` 改为本地工单 + 可选 Feishu；`NATIVE.md` 新增第 28 条记录本次移除；`docs/007` 删除已不存在的 `pullEndNotices` 说明；插件 `locale/en.json`、`locale/zh.json` 的元数据也改为本地工单。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 全新安装 | 不需要 GitHub 仓库、账号或 `gh`；默认启用本地 Source，Feishu 关闭，面板「执行 → 新建工单」或主 Session 直接建单即可。 |
| Worker | 在工单 worktree 内本地提交，不 push、不开 PR；完成后 `orchestrator_report` 报 ready，ticket 结束。 |
| 合并 | 主 Session 或面板在明确请求下合并本地分支；门禁只含本地评审/备注/检查/依赖/目标分支，不再等待 PR CI。 |
| 旧数据库 | 本版本不解释也不清理 GitHub 记录；带 GitHub 数据的旧 stateDir 应改用新的 stateDir。 |
| 安全 | worker 的 `gh` 始终失败，GitHub token 不进入 worker 环境；操作者的全局 git 凭据配置仍被隔离。 |

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm typecheck`（插件） | 通过 |
| `pnpm test`（插件） | 67 个文件、1314 个测试通过 |
| `pnpm build`（插件） | 通过；`lib/` 不再生成 `github-watch.*` |
| `pnpm run test:docs`（仓库根） | 21 通过、0 失败 |
| `pnpm vitest run packages/bundle/web-app/tests/superagent-bundle.spec.ts` | 4/4 通过 |
| `pnpm exec tsc --noEmit -p apps/web/tsconfig.json` | 通过 |
| `pnpm exec tsc --noEmit -p packages/bundle/web-app/tsconfig.json` | 通过 |
| `git diff --check`（本次改动范围） | 通过 |

## 4. 说明

- 通用 `Follow`/`ticket.follows`/`bindThread`/`threadClosed` 仍保留在 core，作为 Source 无关的线程订阅能力；GitHub 不再产生它们。`TicketOutcome` 的 `merged`/`closed` 也保留用于读取旧记录。
- 并发会话当时正在修改 `.github/workflows/*` 与 `.gitignore`；这些不属于本次改动，也未纳入本次提交。
- `tests/environment.spec.ts` 的样例 URL 已改为 `git.example.com`，只验证 worker 不继承操作者的全局 git 配置，不再出现 GitHub 字样。
- 第 1–8 轮审计曾围绕 GitHub 迁移/启动清理补强；第九轮按用户要求将这些路径整体删除，最终代码不再包含迁移或启动清理，详见第 6 节。

## 5. 文档与日志同步

- 插件文档 `README.md`、`SETUP.md`、`NATIVE.md`、`docs/000`–`008`、`examples` 已改为本地工单 + 可选 Feishu；根 `README.md` 的 Superagent 段落、bundle patch 测试与 `apps/web/tests/scaffold.ts` 同步移除 GitHub Source。
- `docs/007` 删除了已删除的 `pullEndNotices`；`docs/005` 的通知列表只保留本地检查/合并事件；`docs/006` 写明本版本没有 GitHub Source、不解释也不清理 GitHub 记录，并说明旧 stateDir 需要自行更换。
- 插件 `locale/en.json`、`locale/zh.json` 的 `meta.description` 已不再写 GitHub；`pnpm run verify-locale-metadata` 通过。
- `prompts/admission.md` 不再让 Eval 读取仓库文件；Feishu 技能、迁移提示、搜索提示与相关注释已去掉 PR/Issue/CI 残留。
- 本文档原编号 00100 与并发会话的 `00100-文档同步-rust-anything使用指南.md`、`00101-技能改名-dsh-rust-anything.md`、`00102-改名收尾-文档与日志同步.md` 冲突，因此改为 00103。

## 6. 彻底移除残留（第九轮）

用户确认从未使用过 GitHub 集成、没有需要迁移的数据，要求清除全部残留。因此不再保留任何 GitHub 专用的迁移或启动清理：

- `src/app/migration.ts`：删除 `github-*`/`thread-since:github:` 死键、`github-read:` 缓存清理、`REMOVED_SOURCES`/`withoutRemovedSources` 设置过滤、`LEGACY_FIXED` 的 `github`、旧提示词的 GitHub 替换，以及 `ticketOf` 对 GitHub follows/channels 的过滤。
- `src/app/runtime.ts`：删除 `dropRemovedSourceData()` 及其启动调用；旧数据库的 GitHub 设置、队列、回执、通知与游标不再被扫描或清理。
- `src/app/views/tickets.ts`：删除 `removedGitHubTicket()` 与仅对旧 GitHub ticket 隐藏 merged/closed 结果的条件；所有 ticket 按当前状态展示。
- `src/core/model.ts`：删除 `Ticket.ciNotice`；`src/app/store-read.ts` 不再跳过 `meta.ci` 触发。
- `src/deployment.ts`：删除 `github` 旧配置键的迁移提示，未知键仍会被拒绝。
- 测试：删除旧 GitHub 迁移/启动清理测试，迁移 fixture 不再含 GitHub；新增“未知 Source 的 schema 10 数据库仍能打开并保留该 Source”的回归测试；通用 git 测试的 URL 改为 `git.example.com`。
- 文档：`docs/006` 的迁移说明改为“没有 GitHub 专用步骤，保留 domain settings、keys、ticket channels 与 followed threads”；`commands.ts` 去掉过时的 `gh`/ETag 注释。
- 保留的安全隔离：worker 的 `gh` 仍是失败 shim，`GH_TOKEN`/`GITHUB_TOKEN` 仍被清除，防止 worker 使用操作者本机的 GitHub 凭据。这不是集成代码。

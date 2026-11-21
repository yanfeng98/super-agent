# 00100 — 移除 dsh-superagent 的 GitHub 集成

- **日期**：2026-11-21
- **背景**：用户确认只用本地工单，要求把 GitHub 那一套直接删掉，而不是仅关闭。目标是保留本地 Source、面板/聊天建单、本地合并、检查、队列、回退与清理，同时不再出现 GitHub Source、账号探测、PR 绑定、PR CI 门禁、GitHub 技能与相关提示词。
- **范围**：`apps/web/native/dsh-superagent`（源码、提示词、技能、测试、文档、`cordis.patch.yml`、`package.json`）、`packages/bundle/web-app`（bundle patch 与测试）、`apps/web/tests/scaffold.ts`、根 `README.md`。

## 1. 变更

| 区域 | 变更 |
|---|---|
| GitHub Source | 删除 `src/sources/github.ts`、`github-pull.ts`、`github-read.ts`、`github-trigger.ts`、`github-channel.ts`，以及 `skills/dsh-superagent-github`；`tsdown.config.ts`、`package.json` 的 `./github-watch`、插件与 bundle 的 `dsh-superagent-github` 挂载全部移除。 |
| 部署与账号 | `Deployment` 只保留 machine 字段与可选 `feishu`；删除 `GithubAccountsSchema`、`resolveDeployment`、`SETUP_FIELDS.github`、GitHub 的 `SetupError` 码、`stateView.accounts` 的 `repository`/`agent`；`Setup` 只探测可选 Feishu bot。 |
| Worker 环境 | `writeEnvironment` 不再写 GitHub 凭据 helper、SSH 重写或 agent 账号；固定中性提交身份 `dsh-superagent <dsh-superagent@localhost>`。`gh` 仍是一个失败 shim，避免 worker 误用操作者本机的真实 `gh`；`GH_TOKEN`/`GITHUB_TOKEN` 仍被清除，作为凭据隔离而不是 GitHub 集成。 |
| 合并门禁 | 移除绑定 PR 的 CI 检查、PR 关闭/合并通知、`orchestrator_report.pr_number`、PR 绑定与 `orchestrator_eval_read` 的仓库文件/目录读取；本地门禁保留：评审结论、`requireReview`、未解决备注、配置检查、依赖、目标包含、变基、队列、合并后目标检查。 |
| 提示词与技能 | 主 Session、worker、work、dispatch、subscriptions、replies、cli、handoff、team/communication、team/delivery、team/scope 去掉 GitHub/PR/CI/自动合并/评审 bot 内容；内置技能只剩 Feishu；`skill/github` 与 `sourceSkills.github` 删除。 |
| 面板与聊天 | 删除 GitHub 来源卡片、账号字段、PR 图标、GitHub 搜索提示与相关文案；`orchestrator_read` 不再暴露 `github/…`；`outcome.merged`/`closed` 仅作为旧数据标签保留，已从“待评审”与可合并判定中排除。 |
| 迁移与清理 | schema 10→11 迁移不再构造 GitHub follows/PR 结束记录，并删除 GitHub 设置、Policy part 与 `github-*`/`thread-since:github:` 键；启动时对已经是 schema 11 但仍带 GitHub 设置/键的数据库做一次同样的清理。`ticketOf` 同时丢弃旧记录里的 `follows` 与 GitHub channel。 |
| 文档 | 插件 `README.md`、`SETUP.md`、`NATIVE.md`、`docs/000`–`008`、`examples` 与根 `README.md` 改为本地工单 + 可选 Feishu；`NATIVE.md` 新增第 28 条记录本次移除。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 全新安装 | 不需要 GitHub 仓库、账号或 `gh`；默认启用本地 Source，Feishu 关闭，面板「执行 → 新建工单」或主 Session 直接建单即可。 |
| Worker | 在工单 worktree 内本地提交，不 push、不开 PR；完成后 `orchestrator_report` 报 ready，ticket 结束。 |
| 合并 | 主 Session 或面板在明确请求下合并本地分支；门禁只含本地评审/备注/检查/依赖/目标分支，不再等待 PR CI。 |
| 旧数据库 | schema 10 迁移时丢弃 GitHub 数据；schema 11 数据库启动时清掉残留的 GitHub 设置、Policy part 与读取游标；旧 GitHub ticket 不再出现在待评审列表。 |
| 安全 | worker 的 `gh` 始终失败，GitHub token 不进入 worker 环境；操作者的全局 git 凭据配置仍被隔离。 |

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm typecheck`（插件） | 通过 |
| `pnpm test`（插件） | 67 个文件、1311 个测试通过 |
| `pnpm build`（插件） | 通过；`lib/` 不再生成 `github-watch.*` |
| `pnpm run test:docs`（仓库根） | 21 通过、0 失败 |
| `pnpm vitest run packages/bundle/web-app/tests/superagent-bundle.spec.ts` | 4/4 通过 |
| `pnpm exec tsc --noEmit -p apps/web/tsconfig.json` | 通过 |
| `pnpm exec tsc --noEmit -p packages/bundle/web-app/tsconfig.json` | 通过 |
| `git diff --check`（本次改动范围） | 通过 |

## 4. 说明

- 通用 `Follow`/`ticket.follows`/`bindThread`/`threadClosed` 仍保留在 core，作为 Source 无关的线程订阅能力；GitHub 不再产生它们。`TicketOutcome` 的 `merged`/`closed` 也保留用于读取旧记录。
- 并发会话当时正在修改 `.github/workflows/*` 与 `.gitignore`；这些不属于本次改动，也未纳入本次提交。
- `tests/environment.spec.ts` 仍用 GitHub URL 作为“操作者全局 git 配置”的样例，验证 worker 不继承该配置；这是隔离测试数据，不是 GitHub 集成。
- 审计后补充：启动清理同时去掉旧 ticket 的 GitHub channels/follows 与 `github-issue` 设置，避免旧 ticket 等待一个已不存在的 Source；删除迁移里已无作用的 `prompt-settings-text` 早退。
- 审计后补充（第二轮）：启动清理保留本地 `autoMerge`，只清 GitHub-bound ticket 的 standing merge；移除旧 GitHub 的 merge queue entry，并在队列清空时解除 pause；把发往已删除 Source 的 pending reply 标记为 dropped，避免永远重试。
- 审计后补充（第三轮）：识别旧 GitHub ticket 时也看它的 origin Trigger 的 Source，因此即使绑定已被清过，队列里残留的旧 entry 仍会被移除；本地 ticket 的 origin 是 local/Feishu，不受影响。

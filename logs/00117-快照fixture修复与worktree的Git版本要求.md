# 00117 — 快照 fixture 修复与 worktree 的 Git 版本要求

- **日期**：2026-11-22
- **背景**：`pnpm run test:snapshot` 在补入 5 个上游 Beta 技能之后有 45 个用例失败，需要逐项修掉。排查下来失败分四类：录制里缺少随包技能目录、PTC 场景的工作区期望与自己的录制互相矛盾、本机环境残留污染了指令路径、以及本机 Git 2.34 不支持 worktree 包的扫描选项。
- **结论**：快照套件恢复为 **275 通过 / 0 失败 / 3 跳过**；`worktree` 包的真实限制（需要 Git 2.36）写进它的执行要求与仓库开发文档。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `snapshots/sdk/*` 与部分 `snapshots/session/*`（63 个录制文件） | `pnpm run test:snapshot:refresh` 补齐 11 个 `eng-*` 目录条目；只保留目录相关的行，纯时间戳变动已还原 |
| `snapshots/session/ptc-{node-workspace,persistent,persistent-scoped,reset}/workspace.expected/` | 删除残留的产物文件，改成 `.empty` 标记：录制里 `run_code` 返回 `sandbox-unavailable`，最终工作区必然为空 |
| `snapshots/session/ptc-workspace-context/session.v4.jsonl` | 把被环境残留污染的相对路径改回干净渲染（`AGENTS.md`、`nested/AGENTS.md`、`.` 开头的 `scope`、空的 `projectRoot`） |
| `snapshots/session/headless.snapshot.ts` | 增加 Git 版本能力门禁：低于 2.36 时跳过 `worktree` 场景 |
| `packages/experimental/worktree/README.md` / `.zh.md` + `.i18n.yaml` | 执行要求补一句：检查与清理读取 `git worktree list --porcelain -z`，需要 Git 2.36，旧版本报 `WORKTREE_COMMAND_FAILED` 并保留检出目录 |
| `docs/development.md` / `.zh.md` + `.i18n.yaml` | Git 要求那一条补上该包级限制的说明 |
| `snapshots/AGENTS.md` | 记录环境陷阱：fixture 工作区上方的 Git 标记会改变项目根解析并把随机目录名写进指令路径 |
| `logs/00117-…`（本文件） | 记录本次改动 |

## 2. 原因与行为

- **目录缺失**：加 `eng-` 前缀那次提交里的刷新是在提供方读不到资产的状态下录的——assets 目录已改名为 `eng-*`，而录制进程仍按旧名读取，全部 `ENOENT`，于是把「只剩 office 4 个」的目录写进了 63 个 fixture。今天用源码重放时提供方正常发布 11 个，两边只差这一行。
- **PTC 工作区**：这些场景在本机 `run_code` 直接失败并返回 `sandbox-unavailable`（没有 bubblewrap，Landlock 也不可用），文件根本没写出来；`workspace.expected/` 却保留着上一次「沙箱可用」时录制的产物文件。按 `snapshots/AGENTS.md` 的规定，工作区是独立 oracle、只能记录真实外部效果，所以正确期望是空。
- **`ptc-workspace-context`**：本机 `/tmp/.git` 是个空目录残留，`findProjectRoot` 据此把项目根解析成 `/tmp`，工作区指令的前缀因此被渲染成 `<随机目录名>/AGENTS.md`，`scope` 与 `baselineIdentity.projectRoot` 也带上了它。同一处残留也是 `skill-filesystem` 的 fallback-root 用例一直失败的原因；删除该空目录后实际渲染恢复为 `AGENTS.md` / `nested/AGENTS.md`。
- **`worktree`**：插件用 `git worktree list --porcelain -z` 扫描，而 `-z` 是 Git 2.36 才有的选项。本机 2.34 上扫描以 129 退出，`enter_worktree` 建不出第二个工作树，删除命令在 `/` 里以 128 失败，最后断言的分支自然不存在。该包自身的测试在本机同样以 42/123 失败。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm run test:snapshot` | 275 通过 / 0 失败 / 3 跳过（2 个平台跳过 + `worktree`），连跑两次一致 |
| `npx vitest run packages/skill/skill-filesystem` | 40/40（此前受 `/tmp/.git` 影响） |
| `npx vitest run packages/experimental/worktree` | 本机 Git 2.34 下 42 失败（既有问题，本次未改产品代码） |
| `pnpm run test:docs` | 21/21 |
| `pnpm run typecheck` | 5/5 |

## 4. 备注

- **待定**：`worktree` 的兼容处理有两条路——给 `registration()` 加行模式回退（实验把失败从 42 降到 12，剩余 12 个需要现代 Git 复核），或把仓库的 Git 最低版本从 2.26 提到 2.36。两者都超出「修快照」的范围，留待决定。
- **环境前提**：本次在仓库之外删除了空的 `/tmp/.git`。它若再次出现，`ptc-workspace-context` 与 `skill-filesystem` 的 fallback-root 用例会重新失败。

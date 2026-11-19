# 00064 — 清理 hygiene 遗留

- **日期**：2026-11-19
- **背景**：`pnpm run hygiene` 的 26 道门里有 6 道失败。同步 rc.5 时已修掉其中的 `vendor rescope` 与 `constraints` 清单问题（见 [日志 00063](00063-同步上游0.2.2-rc.5.md) 第 6 节），剩下 5 项都是本仓库自留的旧债。用户要求一次清完，让 hygiene 全绿。
- **范围**：`apps/web/native/dsh-im`、`packages/api/session-controller`、`packages/client/ui-chat`、`python/sdk-runtime`。

## 1. 修改

| 门 | 位置 | 处理 |
|---|---|---|
| `constraints` | `apps/web/native/dsh-im/package.json`、`NATIVE.md` | `devDependencies.@deepseek-ai/cordis` 从固定 `4.0.1` 改为 `workspace:~`，跟随仓库 vendored 的 Cordis 4.1；`NATIVE.md` 的本地改动条目改为说明该链接与既有的 `ctx.root` 修复。IM 的 3,502 个用例在 Cordis 4.1 下全部通过 |
| `optional dependency imports` | `packages/api/session-controller/src/delete.ts` | `dsh-session-persistence` 改为 `import type`，三个错误类在 `removeStored` 的 `catch` 里按需 `await import(...)`。可选的 peer 不再进入模块作用域；删除失败时的归类逻辑不变 |
| `no new unknown casts` | `packages/api/session-controller/tests/agent.host.spec.ts` | 没有把新断言登记进基线，而是把两处完全相同的 `as unknown as SessionObservation` 对象字面量提取成 `preparedObservation(meta)`，断言数量回落到基线记录的 1 处 |
| `empty catch` | `packages/client/ui-chat/src/client/chat/edit-resend.ts` | 两处空 `catch` 补上 `(_error)` 绑定，原有的原因注释保留 |
| `runtime closure` | `python/sdk-runtime/package.json` | 依赖清单补 `@deepseek-ai/dsh-bash-sandbox` 与 `@deepseek-ai/dsh-pwsh-sandbox`：两个 Superagent preset 按平台各引用其一，运行时依赖图因此闭合（6 个 preset、196 个 workspace 包） |

## 2. 验证

- `pnpm run hygiene`：26/26 通过（此前 20/26）。
- `pnpm run typecheck`：5/5；`pnpm run build`：12/12；`pnpm run lint`：0 warning / 0 error（7579 个文件）；`pnpm run doc-sync`：43/43。
- 单测：`session-controller`、`ui-chat`、`ui-settings-models` 132 个文件 / 2749 用例；IM 3,502 用例（Cordis 4.1 下）。
- 行为没有变化：删除命令的错误归类、编辑重发的回退路径、IM 的渠道行为都只换了导入方式与绑定名。

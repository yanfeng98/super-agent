# 00061 — 复核：续做、检查写入与 squash 清理

- **日期**：2026-11-18
- **背景**：对 00059/00060 的修复做复核，确认合并门、评审、续做与合并策略在边界情况下是否仍然自洽。
- **范围**：`src/core/tickets.ts`、`src/app/runtime.ts`、`src/app/review.ts`、`tests/`、`docs/`。

## 1. 续做会继承上一次结束的评审

- `Tickets.wake` 原来只删除 `ended`，上一次结束的 `review`、`checks`、`reviewed` 留在工单上；人员消息续做后，旧批准可能被当成新工作的批准。
- 现在续做时清空这三项；新的结束会重新采集评审与检查。

## 2. 已合并/已丢弃/已清理的工单仍可能被续做

- `wake` 原来只拒绝 `ended.by === 'human'`，已合并、已丢弃或已清理的工单仍可能因一条人员消息恢复执行。
- 现在这些工单不续做；等待中的消息回到 Eval，避免在已合并分支或已删除工作区上继续工作。
- `docs/003`、`docs/006`、`NATIVE.md` 同步说明。

## 3. 检查结果可能写到已经变化的工单

- 手动 `checkTicket` 运行期间，如果工单被续做、变基、合并、丢弃或清理，原来会把结果写回。
- 现在写入前校验 `ended.at`、`execution.base` 与合并/清理状态；不匹配就拒绝并提示重跑。
- `finishTicket` 的自动检查与评审写入也增加相同的 base/结束校验，避免变基或合并期间写入旧结果。

## 4. 通知写入可能回滚工单事务

- 通知原来在域事件事务内写 SQLite；写入失败会连带回滚工单结束或提问。
- 现在在 `store.afterCommit` 后写入，失败只记日志，不影响工单。

## 5. squash 合并后的清理

- squash 产生新提交，工单分支不再是目标分支的祖先，原来的 `git branch -d` 安全检查会拒绝清理。
- 现在 `cleanupTicket` 根据 `merged.strategy === 'squash'` 使用受控的强制删除；普通快进与合并提交仍走 `-d` 安全检查。
- `merge`/`squash` 失败时分别 `git merge --abort` 或 `git reset --hard HEAD`，避免仓库停留在合并中间状态。
- 合并确认文案改为覆盖三种策略。

## 6. 验证

- 包测试：65 个文件、1293 个用例全部通过（清除宿主继承的 `GIT_CONFIG_GLOBAL` 后）。
- `pnpm typecheck`：通过。
- 包 `tests/docs.spec.ts`：6/6；仓库 `pnpm run test:docs`：21/21。
- 仓库 `pnpm run lint`：0 warnings、0 errors。
- 仓库 `pnpm run build`：12/12 gate。

## 7. 范围

- 未改变既有非目标：GitHub 历史 backlog 不导入、auto-merge 由 worker 按 delivery requirements 处理、revert/冲突解决沿用 git。

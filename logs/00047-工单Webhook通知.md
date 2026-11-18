# 00047 — 工单 Webhook 通知

- **日期**：2026-11-18
- **背景**：桌面通知只在 Web 页面打开并手动开启时生效。需要一个不依赖浏览器、由服务端主动发出的提醒通道。
- **范围**：`apps/web/native/dsh-superagent`（`src/sources/local.ts`、`src/app/webhook.ts`、`src/app/runtime.ts`、`src/core/*`、`docs/`、`NATIVE.md`、`tests/`）。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/sources/local.ts` | 本地来源配置新增可选 `webhook`（仅 http/https）；`webhookFor(settings)` 在来源启用时返回地址 |
| `src/core/ports.ts`、`src/core/assignments.ts` | 新增 `ticket.attention` 领域事件，在出现**新的**待回答问题时发出（重复问题不重复通知） |
| `src/app/webhook.ts`（新增） | `sendWebhook`：POST JSON，5 秒超时，非 2xx 抛错 |
| `src/app/runtime.ts` | 本地工单出现新问题或结束时，向配置的 webhook 发送 `{event, at, ticket{id,number,title}, question|outcome/reason}`；失败只记日志，不影响工单 |
| `docs/006/007`、`NATIVE.md` | 配置说明、模块与测试表 |
| `tests/webhook.spec.ts`（新增）、`tests/local.spec.ts` | 本地 HTTP 接收端验证 JSON 与 content-type、非 2xx 报错、webhook 配置仅在来源启用时生效、非法协议被拒 |

## 2. 验证

- `pnpm --filter dsh-superagent exec vitest run`：**63 个文件、1250/1250 通过**。
- `pnpm --filter dsh-superagent run typecheck`、`run build`：通过。
- 仓库 `pnpm run lint`：0 警告 0 错误；仓库 `pnpm run build`：12/12 gate 通过。

## 3. 仍未做

- CI 汇总。
- 多项目独立配置（模型/并发/目录根）。
- 真实浏览器冒烟。

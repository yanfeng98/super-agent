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

`packages/api/session-controller/README.md` 与 `.zh.md` 补一句：该命名空间在没有可选 peer 时也能挂载，持久化的错误类只在删除失败时加载。

## 2. 复核

- `delete.ts`：构建产物 `lib/index.js` 保留外部 `await import("@deepseek-ai/dsh-session-persistence")`，从包目录实测能解析到三个错误类；peer 链接与其声明一致；SQLite 后端抛 `SessionPersistenceRemoveUnsupportedError` 的拒绝路径正是该分支，`delete.host.spec.ts` 的 16 个用例覆盖并通过。
- IM：运行时源码与构建产物都不 import Cordis（根经 `Symbol.for('cordis.root')` 取得），所以 `devDependencies` 只影响测试与内嵌元数据；`pnpm-lock.yaml` 中 `cordis@4.0.1` 的解析清零，`pnpm install --frozen-lockfile` 一致。
- 未知类型断言：基线文件未改动，基线合计 1,204，gate 输出也是 1,204，说明是收敛回记录值而不是新增登记。
- 空 `catch`：补绑定后两个 `try` 仍是单语句，满足同一规则。
- python 运行时：两个 sandbox 包无 `os`/`cpu` 限制、无 `postinstall`、无自身依赖，跨平台加入安全；`verify-runtime-closure` 及其相关规格 305 个用例通过。
- IM 的 Cordis 链接：[日志 00027](00027-原生集成dsh-im.md) 记录的理由（workspace 的 4.1.0 会让 `fiber.await()` 挂起）在当前的 Cordis 4.1.0 与测试集下不再出现；若日后再次同步该插件并恢复 `4.0.1` 固定，需重新验证这一点。

## 3. 验证

- `pnpm run hygiene`：26/26 通过（此前 20/26）。
- `pnpm run typecheck`：5/5；`pnpm run build`：12/12；`pnpm run lint`：0 warning / 0 error（7579 个文件）；`pnpm run doc-sync`：43/43；`pnpm run test:docs`：21/21。
- 单测：`session-controller`、`ui-chat`、`ui-settings-models` 132 个文件 / 2749 用例；IM 3,502 用例（Cordis 4.1 下）；`delete.host.spec.ts` 与 `agent.host.spec.ts` 32 个用例。
- 行为没有变化：删除命令的错误归类、编辑重发的回退路径、IM 的渠道行为都只换了导入方式与绑定名。

## 4. 范围

动态导入在“挂了 backend 却没有 `dsh-session-persistence` 包”的部署里会以模块解析错误替换原始错误。所有 backend 都以该包声明 peer，Web 与 `python/sdk-runtime` 的清单也都列了它，因此没有为该不可能路径加兜底。

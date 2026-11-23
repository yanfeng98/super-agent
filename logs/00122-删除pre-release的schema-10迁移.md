# 00122 — 删除 pre-release 的 schema 10 迁移

- **日期**：2026-11-23
- **背景**：`dsh-superagent` 带着一份 schema 10 → 11 的一次性迁移（`src/app/migration.ts`，517 行）与配套的 `tests/migration.spec.ts`。但这份插件进入本仓库时就已经是 schema 11，`docs/007-architecture.md` 也称 schema 10 为 pre-release 形态；本机状态目录 `~/.dsh/state/superagent/orchestrator.sqlite` 记录的 schema 是 11，`trigger`/`ticket`/`thread`/`decision`/`outbox`/`effect` 全是 0 行。也就是说，这段迁移在当前使用方式下永远不会进入。
- **结论**：只保留一个 schema。`SCHEMA_VERSION`（11）是唯一能打开的版本：版本更高报 `Unsupported orchestrator database version`，版本更低报「predates schema 11」并提示换一个状态目录。删除整套迁移、它独占的导出、`KV.queues`、三处飞书回填与对应的死文案。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/app/migration.ts` | 删除。`migrateTo11` 及其全部步骤、`migrateSettings`、`migrateSettingsRecords`、`migratePrompts`、`ticketOf`、`withoutCopies`、`basisOf`、`reasonsOf`、`knownReason`、`feishuDirectAddresses`、`feishuThreadLinks`、`assessmentTriggers`、`effectQueue`、`triggerTimes`、`queueEarlierWork` 等一并消失；它们只被这份文件自己或测试引用 |
| `src/app/store.ts` | 自带 `SCHEMA_VERSION = 11`；schema 检查只区分「更高」「更低」；打开事务只剩建表与 `addColumns()`，不再有 `migrating` 分支和「unchanged at schema … migration failed」的包装错误；`set(KV.store.schema, 11)` 只在数据库新建时写入 |
| `src/kv.ts` | 删除 `KV.queues`（`relay-queue`、`subscription-results`、`prompt-notification`、`settings-notification`），它唯一的读写方是被删的迁移 |
| `src/app/repair.ts`、`src/app/runtime.ts`、`src/executor/dsh.ts` | 注释里「schema migrations」「double-migrate」「schema 11 migration converted reopen inputs」措辞改为当前事实；`rekeyFeishuTopicThreads` 保留 |
| `src/client/locales.ts` | 删除 `historySource.{sections,vocabulary,settings}-migration` 三条文案，它们标注的来源名只能由迁移产生 |
| `tests/migration.spec.ts` | 删除（499 行） |
| `tests/store.spec.ts` | 新增「refuses a database at an older schema and asks for a new state directory」；`KV` 拼写检查不再豁免迁移模块 |
| `tests/policy.spec.ts` | 删除「settings migration」两个用例与 `prepare` 变量改名的用例（都只测被删的 `migrateSettings`） |
| `tests/queue.spec.ts` | 只保留 notice 去重用例；迁移 relay/subscription/notification 队列的用例随迁移删除 |
| `tests/runtime.spec.ts` | 两个用例的标题与注释不再提迁移；被锁挡住的第二个进程「什么也不读」，而不是「什么也不迁移」 |
| `docs/006-deployment.md` | 升级段落改为「本版本只写 schema 11、不做迁移，记录其它版本的库会被拒绝并要求换状态目录」 |
| `docs/007-architecture.md` | 模块表删 `src/app/migration.ts`，测试表删 `tests/migration.spec.ts`；`effect` 表的写入方、`KV` 说明、`SCHEMA_VERSION` 段落、`afterCommit` 后的「读到的就是存下的」一并改写 |
| `NATIVE.md` | 第 28 条的测试清单去掉 `tests/migration.spec.ts`；新增第 32 条记录这次删除；套件数 1,327 → 1,304 |

## 2. 原因与判断

- **不会有人用它**：迁移只在打开 schema 10 的库时运行；本仓库的这份副本一进来就是 schema 11，且是 `private`、未发布。留着它等于养一条没有调用场景的代码路径，还要连带维护它的 499 行测试和 `KV.queues` 这组只为它存在的键。
- **拒绝比迁移更安全**：遇到比 11 旧的库，直接报「predates schema 11 … start with a new state directory」，比静默改写成当事人不认识的形态更容易排查；用户列表只有一个人，换目录没有协作成本。
- **不做「空迁移」**：没有把旧库的表结构保留成可选模式，也不保留 `OLDEST_SCHEMA`。少一个常量就少一处「到底支持到哪一版」的解释负担。
- **保留 `repair.ts`**：`rekeyFeishuTopicThreads` 处理的是 schema 11 库也可能存在的「按 Topic id 建 key 的飞书 Thread」，与 schema 版本无关，且是幂等、每次启动零成本；删它不在本次范围。
- **`reopen` 输入类型未删**：`TicketInput` 里的 `reopen` 是领域模型的一部分，删除会牵动读工具与序列化；本次只把一个仍然成立的事实写进注释——它不再投递任何内容。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx tsc --noEmit`（dsh-superagent，含测试） | 通过 |
| `npx vitest run --maxWorkers=1 --no-file-parallelism`（dsh-superagent 全量） | 1,301 通过、3 失败（共 1,304）；3 个都是 `tests/role-models.spec.ts` 的既有失败（`Cannot read properties of undefined (reading 'assemble')`），与本改动无关；`tests/docs.spec.ts` 的模块表/测试表逐文件校验通过 |
| `npm run build`（dsh-superagent） | 通过 |
| `pnpm run test:docs`（仓库根） | 通过 |

## 4. 未做

- 下面这些只服务 schema 10 时代数据的形状仍在，删除会牵动读工具、序列化或面板文案，留待单独判断：
  - `MandateBasis` 的 `via: 'listed'` 与 `authority`：只有被删的 `basisOf` 产出过，`src/policy.ts` 与 `src/app/views/tickets.ts` 仍在处理；
  - `TicketInput` 的 `reopen` 种类：`src/app/cards.ts` 与 `orchestrator_read` 仍渲染它；
  - `Trigger.deliveredAt`、`PromptVersion.legacy` 这类「早期版本写入」的字段及其中文/英文文案（`src/client/ticket.tsx`、`locales.ts` 仍在显示）。
- 保留 `src/app/repair.ts` 的飞书 Topic Thread 重键：它处理 schema 11 库也可能存在的、按 Topic id 建 key 的 Thread，与 schema 版本无关，幂等且每次启动零成本。

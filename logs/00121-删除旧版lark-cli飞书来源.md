# 00121 — 删除旧版 lark-cli 飞书来源

- **日期**：2026-11-23
- **背景**：内置 `dsh-superagent`（`apps/web/native/dsh-superagent`）在接管 dsh-im 之前有一套自己的 lark-cli 飞书实现。接管版上线后，那套代码没有删干净：`src/sources/feishu.ts`（老 kind 与插件入口，已无插件加载它）仍在树里，`feishu-loops.ts` 被它间接拖进主插件包（`lib/index.js` 里还留着 `event consume`），`feishu-read.ts` 仍是旧工单 `feishu/<trigger-id>/…` 资源与旧链接查询的唯一入口——而这些调用要走 `lark-cli`，新版配置根本不产生 `deployment.feishu`，所以它们必然抛 `The Feishu Source has no working bot CLI`。同时 lark-cli 的凭据链（`resolveFeishuBot`、`SETUP_FIELDS`、`setup.checkAccounts`、`Commands.lark`、`feishuProblem`）继续留在代码与面板里，形成「看起来还有一套飞书、实际用不了」的状态。
- **结论**：飞书只剩 dsh-im 一套。老实现、老凭据链、老资源前缀、老测试与对应文档一并删除；旧工单里的飞书原文不再可读（本来也已读不了），它的 Trigger 文本仍在。dsh-im 的 handoff、上下文读取、`/stop`、补收等能力不变。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/sources/feishu{,-loops,-read,-trigger,-channel,-image}.ts` | 删除；共享的标识与小工具收进新的 `src/sources/feishu-shared.ts`（`SOURCE`、`feishuTriggerId`、`withMentionNames`、`feishuChatLink`、`admissionImageKeys`、`admissionFileKeys`） |
| `src/sources/feishu-im.ts`、`feishu-im-loops.ts` | 改为从 `feishu-shared.ts` 取共享项；行为不变 |
| `src/deployment.ts` | `FeishuBotSchema`、`SETUP_FIELDS`、`FeishuBot`、`resolveFeishuBot`、`executable`、`workerProbe`、`Probe` 删除；`Deployment` 就是 `Machine` |
| `src/setup.ts`、`src/app/setup.ts` | `SetupPlace` 只剩 `deployment`；飞书相关的 5 个 setup 错误码、`feishuProblem`、`checkAccounts`、`setupChanged` 删除；`check()` 在 setup 已完成时拒绝，其余行为不变 |
| `src/infra/commands.ts` | 删除 `lark()`、`membership()`（232011 改写）、`validItems`、`Tally`；子进程不再注入 `LARKSUITE_*`；`run`/`git`/`CliError` 保留 |
| `src/infra/environment.ts` | `writeEnvironment` 只接收 `stateDir`；`lark-cli` shim 保留但恒失败，提示改为「dsh-im owns the Feishu bot; read Feishu through the orchestrator tools」；随之失效的 PATH 重写 shim 生成器删除 |
| `src/app/{runtime,http,links,resources,settings,views/sources,views/settings,source-context}.ts` | 删除 `feishuPageSize`、`feishuProblem`、`feishu/` 资源路由、旧链接分支、账号校验与 `panelOnlyAccounts` 守卫、`SourceContext.problem`；`accounts.feishu` 仍报 dsh-im 机器人；`sourceLink` 对旧 Trigger 返回空链接而不是不可读的路径 |
| `src/sources/{host,kind}.ts`、`src/core/errors.ts`、`src/client/{deployment.tsx,locales.ts}` | `SourceContext.problem` 与 `panelOnlyAccounts` 及其文案删除；`ChannelLinks` 不再携带部署信息；部署页只显示 `accounts.feishu` |
| `scripts/dsh-superagent` | `status` 输出去掉恒为 `null` 的 `feishu` 字段 |
| `tests/` | `feishu-read.spec.ts` 删除；`sources.spec.ts` 只留 source-context 覆盖；`runtime.spec.ts` 与两份 fixture 改用 `imTrigger` + 假的 dsh-im 服务；其余文件清掉 lark 调用、setup 探针与飞书 setup 问题的断言 |
| `docs/006`、`docs/007`、`docs/008`、`NATIVE.md` | 模块表、测试表、状态载荷、worker shim、部署项与第 31 条本地改动同步 |

## 2. 原因与判断

- **只留一套**：两套飞书来源并存时，面板里「飞书」卡片、`deployment.feishu` 与 `feishuProblem` 会让运维以为还能配 lark-cli；而那条路径在新配置下必然失败，属于「能配、不能用」。删除比保留半死的探测更诚实。
- **旧工单不迁移**：旧 Trigger 的正文、Thread、工单都还在 `Store` 里；丢的只是「按 id 去飞书再读一次原文」的能力，它依赖 lark-cli 凭据与旧回复路由。dsh-im 不给旧 Trigger 补路由，凭空造一条会有读错消息的风险，所以 `sourceLink` 直接返回空，而不是指向读不了的资源。
- **worker 仍然没有飞书 CLI**：`lark-cli` shim 保留为恒失败，比删除更安全——删掉 shim 后，若机器上恰好装了 `lark-cli`，worker 会用它带着操作者身份访问飞书。shim 的提示现在指向 orchestrator 工具。
- **`Commands.run` 保留**：它是 `git` 之外的通用子进程入口，`CliError`、`commandError` 仍被工作区检查与 review 使用；删除 `lark()` 后没有飞书耦合。
- **不保留兼容开关**：`panelOnlyAccounts` 守卫与 `SETUP_FIELDS` 的唯一用途是保护账号 CLI 字段的修改；没有这类字段后守卫没有意义，留着等于假装还有账号要保护。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx tsc --noEmit`（dsh-superagent，含测试） | 通过 |
| `npx vitest run --maxWorkers=1 --no-file-parallelism`（dsh-superagent 全量） | 1,324 通过、3 失败；3 个都是 `tests/role-models.spec.ts` 的既有失败（`Cannot read properties of undefined (reading 'assemble')`），与本改动无关 |
| `npm run build`（dsh-superagent） | 通过；`lib/` 重建后不再含 `event consume`、`chat-members-list`、`messages-mget` 等旧 lark-cli 字符串 |
| `pnpm run test:docs` | 21 通过、0 失败 |


## 4. 校验中发现并修掉的两处

| 问题 | 处理 |
|---|---|
| `scripts/dsh-superagent` 的 `start()` 仍读 `current['feishu']`（`status` 已不再输出该键），KeyError 让 `start` 全部退 1 | 删掉那段「runs without Feishu」提示；`tests/service-script.spec.ts` 17 通过 |
| `tests/panel-settings.spec.tsx` 用「机器人的 lark-cli」输入框验证保存条，字段已不存在 | 改用「IM 飞书机器人」字段，并断言保存来源设置**不再**提示「进行中的派工会中断」（没有字段需要重启） |

## 5. 审计：删除之后仍然存在的问题

| 发现 | 处理 |
|---|---|
| 旧工单的上下文读取走 `routeOf`，对没有 `actorId` 的旧地址抛 `The Feishu reply route is incomplete`；回复则每轮重试同一个无法解析的路由 | `feishuImThreadContext.read` 对无 `actorId` 的 Trigger 直接返回 `unavailable`；`sendFeishuIm` 直接返回 `gone`，收据记 `channelGone` 进丢弃列表，不再无限重试。各补一条测试 |
| `Commands.deployment` 及其构造参数、`ChannelLinks`、`SetupHost.context` 在 `lark()`/旧链接/旧探测删除后没有使用者 | 删除；`new Commands(env, signal)`、`channel.link(address)`、`new Setup({ machine, lifetime, restart })`，调用点与测试同步 |
| `KV.feishu.seen`/`initialized`、`HEALTH_TEXT.pollFailures` 与其中文文案：旧轮询循环删除后没有任何写入者 | 删除 |
| `KV.feishu.topics` 与 schema-11 里把旧话题数组转成映射的那一步：转换后的数据没有任何读者（新循环用 `imTopics`） | 删除该 KV 族、`feishuTopics()` 与测试里的断言；其余 10→11 迁移步骤不变 |
| `SetupHost.lifetime`：探测删除后 `Setup` 不再持有时限 | 删除字段与构造参数 |
| 每个来源的 setup `problem` 字段（`views/sources.ts`、`views/settings.ts`、客户端卡片、panel fixture、006 的 `GET sources`）永远不可能再有值 | 删除该字段与渲染路径，`SourceState` 只看 health |
| 文档与示例残留：007 的 `setupChanged`、008 的 `validItems`、NATIVE 第 6 与第 28 条、`examples/profile.cordis.patch.yml` 的「set the bot's lark-cli」 | 改写为当前事实 |

审计后的校验：`npx tsc --noEmit` 通过；全量 **1,324 通过 / 3 失败**（同样只有 `role-models` 的既有失败，总数 1,327）；`npm run build` 通过；`pnpm run test:docs` 21 通过。

## 6. 第二轮审计：上一轮修复的复查

| 发现 | 处理 |
|---|---|
| 旧工单读附件时仍落到 `routeOf` 的旧错误 `The Feishu reply route is incomplete`，看不出原因 | `routeOf` 对缺 `actorId` 的地址单独报「This Feishu message was recorded before the handoff to dsh-im, so it has no reply route」；补一条图片读取用例 |
| 上一轮只做了函数级测试，没有验证旧工单回复在投递链上的落点 | 补 outbox 用例：旧工单回复以 `dropped`/`channelGone` 落库，且没有调用 dsh-im |
| 文档残留：008 的 `link(address, links)` 示例、示例里的 `pollFailures`、`kind.ts` 的 `link` JSDoc 仍写 `links`、006 `GET settings` 的 per-source `problem`、NATIVE 第 31 条断句 | 逐处改写 |
| 上一轮删除后新增的未使用导入：`route.ts` 的 `SetupPlace`、`source.tsx` 的 `navigate`、`deployment.tsx` 的 `setupText` | 删除 |
| 007 的 `feishu-im/…` 资源行没说旧工单会怎样 | 补「every part answers `unavailable`」 |

`migration.ts` 的 `Key`、`runtime.ts` 的 `reasonText`/`allowedSender`/`triggerMessage`、`host.ts` 的 `ChannelRef` 在改动前就没有使用者（对照 `HEAD~2` 确认），不在本次范围内，未动。

## 7. 文档与日志同步

| 文件 | 改了什么 |
|---|---|
| `docs/006-deployment.md` | 升级一节补一段：旧版只存了 `cli` 的飞书来源升级后要重新选 dsh-im 机器人；它的旧工单保留正文，但原文读为 `unavailable`、回复以 `channelGone` 丢弃（dsh-im 不为接管前读到的消息保留回复路由） |
| `docs/005-panel.md` | Source 页不再有每来源的 setup problem；部署检查问题只在列表上方显示一次 |
| `docs/007-architecture.md` | 测试表补 `tests/outbox.spec.ts` 的 `channelGone` 覆盖与 `tests/feishu-im.spec.ts` 的接管前 Trigger 覆盖 |
| `skills/dsh-superagent-feishu/SKILL.md` | 给模型的说明补一句：接管前写的 Trigger 的 `feishu-im/` 各部分一律 `unavailable`、附件读不了，应当如实说明而不是重试 |
| `SETUP.md` | `restarting: true` 只在 setup 未完成时出现；setup 完成后的来源设置保存不重启 |
| `NATIVE.md` | 测试计数 1,327；第 31 条断句修正 |
| `examples/profile.cordis.patch.yml` | 提示改为「choose the dsh-im bot」 |

校验：`pnpm run test:docs` 21 通过；app 内 `tests/docs.spec.ts` 6 通过（模块表与测试表一致）；全量 1,324 通过 / 3 个既有 `role-models` 失败。

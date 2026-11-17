# 00029 — 移除 fireworks 供应商

- **日期**：2026-11-17
- **背景**：`fireworks` 是 pi-ai 目录里的第三方推理云路由（Fireworks AI），默认端点 `https://api.fireworks.ai/inference`，凭据为 `FIREWORKS_API_KEY`，没有内置 OAuth 或订阅套餐入口。目录快照 22 个对话模型：转售 DeepSeek、Kimi、Qwen、GLM、MiniMax、NVIDIA Nemotron、Thinking Machines Inkling、GPT-OSS 等开源权重模型，同时提供自有 Ember-1 与 `routers/*` 最新版本别名；协议同时支持 OpenAI 兼容对话补全与 Anthropic Messages。用户要求移除该供应商且不能破坏现有功能，因此沿用既有的 `WITHHELD_PROVIDERS` 机制：目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由，不修改 `node_modules` 里的 pi-ai 包。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点与测试）、`docs/user/guide/providers.md`、包 README、升级指南、Web golden 与 e2e 断言、根 README、`logs/`。

> **后续变更**：本日志记录的是移除 `fireworks` 后的状态（18 个可售路由、659 个模型、23 条被移除路由）。随后 [00030](00030-移除groq供应商.md) 移除了 `groq`，当前为 17 个可售路由、652 个模型、24 条被移除路由。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `fireworks`（22 → 23 条，字母序放在 `cloudflare-workers-ai` 之后）；注释说明它是转售其他厂商开源权重模型并托管自有 Ember 系列的第三方推理云、两种协议，以及重新接入方式（`anthropic-messages` 用 `https://api.fireworks.ai/inference`，`openai-completions` 用 `https://api.fireworks.ai/inference/v1`） |
| `tests/catalog.spec.ts` | 「withholds removed catalog routes」用例的被移除路由清单增加 `fireworks` |
| `tests/config.spec.ts` | 参数化的被移除路由用例增加 `fireworks`（每条路由一条用例，strict 与 deferred 在同一条内各验一次） |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单增加 Fireworks；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的被移除路由清单与 `fireworks` 说明同步；重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述与变更清单加入 `fireworks`（22 → 23 条）；迁移段的 OpenAI 兼容端点清单加入 `api.fireworks.ai/inference/v1`；新增一句服务定位说明，正文 500 词（上限 500）；重录 i18n 哈希 |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "fireworks"`（19 → 18 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 19 → 18 |
| 根 `README.md` 差异表 | 22 → 23 个移除路由、19 → 18 个路由、681 → 659 个模型、设置页 22 → 23 个 option，并补 Fireworks 说明 |
| `logs/00009` | 汇总刷新为 18 个路由 / 659 个模型 / 23 条被移除路由，并加入本日志条目 |

## 2. 行为

- 可对话供应商 19 → **18**，目录模型 681 → **659**（只减 `fireworks` 的 22 个模型）。
- `catalogProvider('fireworks')` 返回 `undefined`，`catalogModels('fireworks')` 为空；目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由。
- 存量 profile 点名 `fireworks` 时 `resolveProfiles()` 在 strict 与 deferred 下都抛 `llm-pi-ai: provider "fireworks" is removed from this build; delete the route or select another provider`，同一 `providers` 字典里的其他路由随之不可用，删除该条目即恢复。
- `provider-data.generated.ts` 仍保留 42 个 id 的元数据（`verify-provider-data` 要求与已安装目录一致），它不是可售路由；内置 id 现为 42 = 18 个提供 + `typesafe` + 23 个被移除。
- `packages/experimental/webworker-runtime` 的 `BUILTIN_PROVIDER_IDS` 仍列出 `fireworks`：该列表描述 pi-ai 已安装目录，用于让桥接层拿到合法 id 并让每个请求落到明确的未实现错误，与本次移除无关。
- 未触及 agent loop、模型可见工具、Session 事件或持久化格式；`deepseek-official` 与 `deepseek-account` 原生路由不受影响。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm/llm-pi-ai/tests` | 22 个文件、487 个测试全部通过 |
| `pnpm run typecheck` | 5 passed，0 failed（18.55s） |
| `pnpm run lint` | 0 warning、0 error（90 条规则、7303 个文件） |
| `pnpm run test:docs` | 21 个门禁全部通过（含升级指南词数与中英配对） |
| `pnpm run doc-sync` | 43 个门禁全部通过（54.12s） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致 |
| `pnpm run lint`（修复后复跑） | 0 warning、0 error |
| `pnpm run build` | 11 个门禁全部通过（26.41s，含 host/client 聚合检查与 Web build） |
| `npx tsx scripts/verify-upgrade-guides.ts` | 通过，guide.md 500 词（上限 500） |
| 端点核对 | Fireworks 内置模型使用 `anthropic-messages`，baseUrl 为 `https://api.fireworks.ai/inference`（SDK 追加 `/v1/messages`）；OpenAI 兼容自定义路由经 SDK 追加 `/chat/completions`，因此文档给出 `/inference/v1` |
| Web golden 单项回放 | 用临时 overlay 关闭 `ui-boot-animation` 后回放 `models-settings` 的 `opens the add card over the dormant directory vocabulary`，`option` 列表与 golden 逐项一致；剩余差异只有 `IM机器人` 与 `启动片头` 两个设置项，属于本机 golden 未含的既有并行改动，与本次移除无关。临时 overlay 与测试改动已还原 |

> 本机 Web e2e 整套回放仍为 3/22：`dsh-boot-archive-black` 覆盖层拦截点击，与本改动无关（`00021`–`00026` 各轮记录相同）。

## 4. 复查记录

- 2026-11-17 复查发现自定义模型 API 地址有误：初稿把重新接入地址统一写成 `https://api.fireworks.ai/inference`。内置模型使用 `anthropic-messages`，Anthropic SDK 在该 baseUrl 后追加 `/v1/messages`，因此该地址只适用于 `anthropic-messages` 自定义路由；OpenAI 兼容路由由 SDK 追加 `/chat/completions`，必须使用 `https://api.fireworks.ai/inference/v1`。按此修正了 `catalog.ts` 注释、包 README（中英）、用户指南（中英）、升级指南（中英）、根 README 与 `00009` 汇总，并重录三组 i18n 哈希。
- 复查时确认 Ember 系列的归属：`openrouter` 的 `fireworks/ember-1` 显示名为 “Fireworks: Ember-1”，因此把初稿「而不是第一方模型系列」改为「自有 Ember 系列」，避免被读成 Fireworks 只转售第三方模型。
- 目录复核：18 个提供路由、659 个模型、`WITHHELD_PROVIDERS` 23 条；`catalogProvider('fireworks')` 与 `catalogModels('fireworks')` 均为空；三个 golden 各 18 项且不含 `fireworks`；e2e 下界断言 18。
- 未改动 `provider-data.generated.ts`：`verify-provider-data` 要求生成文件保留已安装目录的全部 42 个 id；同样未改动 `packages/experimental/webworker-runtime` 的 stub 列表，它描述已安装目录而非可售路由。
- 修正后复跑：`vitest run packages/llm/llm-pi-ai/tests`（22 文件 / 487 测试）、`pnpm run typecheck`（5/5）、`pnpm run lint`、`pnpm run test:docs`（21/21）、`pnpm run doc-sync`（43/43）、`pnpm run build`（11/11）、`pnpm run verify-provider-data` 与 `git diff --check`，全部通过。
- 运行态复核：Web 宿主仍是改动前启动的进程，未加载本次源码与构建产物；重启 `pnpm dsh web` 后生效，会话记录在磁盘上不受影响。

## 5. 已知限制与恢复

- 本次只屏蔽可售路由，不删除 pi-ai 包内的 Fireworks 实现与目录元数据；手工声明同名路由仍会被 `resolveProfiles()` 拒绝。
- 恢复时删除 `WITHHELD_PROVIDERS` 中的 `fireworks`，并同步撤销第 1 节列出的 golden、断言与文档改动；升级指南的词数（500，上限 500）需重新核对。

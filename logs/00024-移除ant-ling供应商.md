# 00024 — 移除 ant-ling 供应商

- **日期**：2026-11-16
- **背景**：`ant-ling` 是 pi-ai 目录中的第一方路由（名称 Ant Ling），以 `openai-completions` 接入 `https://api.ant-ling.com/v1`，只有 API key 鉴权（`ANT_LING_API_KEY`），没有 OAuth。目录快照 3 个聊天模型：非推理的 `Ling-2.6-1T` 与 `Ling-2.6-flash`，以及推理模型 `Ring-2.6-1T`；三者上下文均为 262,144、最大输出 65,536，目录价格为每百万 token $0.06/$0.25、$0.01/$0.02、$0.06/$0.25。三个 id 只出现在这一条路由上。用户要求移除，按既有 `WITHHELD_PROVIDERS` 机制处理。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点与测试）、`docs/user/guide/providers.md`、`packages/llm/llm-pi-ai/README.md`、升级指南、Web golden 与 `logs/`；根 README 的差异表另行刷新。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `ant-ling`（19 → 20 条，按目录顺序插在 `amazon-bedrock` 之后、`azure` 之前）；注释说明 Ling 与 Ring 三个模型都没有其他内置路由，以及 `api.ant-ling.com/v1` 的 OpenAI 兼容重新接入方式 |
| `tests/catalog.spec.ts` | 「withholds removed catalog routes」用例的被移除路由清单增加 `ant-ling`；该路由的模型不带请求头，上一轮改借 `github-copilot` 的 headers 用例无需再动 |
| `tests/config.spec.ts` | 参数化的被移除路由用例增加 `ant-ling`（strict 与 deferred 各一次，用例总数 +2） |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单增加 Ant Ling（排在 Amazon Bedrock 之后）；自定义路由清单增加 `api.ant-ling.com/v1`；新增「三个模型没有其他内置路由」的覆盖句；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的被移除路由清单与 `ant-ling` 说明同步，重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述与变更清单加入 `ant-ling`（19 → 20 条），迁移段给出端点与覆盖说明；正文 496 词（硬上限 500） |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "ant-ling"`（22 → 21 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 22 → 21 |
| 根 `README.md` 差异表（后续提交） | 19 → 20 条移除路由、22 → 21 个路由、713 → 710 个模型、设置页移除 20 个 option，并补 `ant-ling` 覆盖句 |

## 2. 行为

- 可对话供应商 22 → **21**，目录模型 713 → **710**（只减 `ant-ling` 的 3 个模型）。
- `catalogProvider('ant-ling')` 返回 `undefined`，`catalogModels('ant-ling')` 为空；目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由。
- 存量 profile 点名 `ant-ling` 时 `resolveProfiles()` 在 strict 与 deferred 下都抛 `llm-pi-ai: provider "ant-ling" is removed from this build; delete the route or select another provider`，同一 `providers` 字典里的其他路由随之不可用，删除该条目即恢复。
- `provider-data.generated.ts` 仍保留 42 个 id 的元数据（`verify-provider-data` 要求与已安装目录一致），它不是可售路由；内置 id 现为 42 = 21 个提供 + `typesafe` + 20 个被移除。
- 文档与 golden 的被移除路由统一为二十条。

## 3. 模型覆盖

3 个模型 id 都只出现在 `ant-ling` 的目录数据里，其余 20 条路由没有同名条目：

| 模型 id | 推理 | 价格（输入/输出） | 其他内置路由 |
|---|---|---|---|
| `Ling-2.6-1T` | 否 | $0.06 / $0.25 | 无 |
| `Ling-2.6-flash` | 否 | $0.01 / $0.02 | 无 |
| `Ring-2.6-1T` | 是（`high`、`xhigh`） | $0.06 / $0.25 | 无 |

同一 Ling 家族在 `openrouter` 上另有承接条目：`inclusionai/ling-3.0-flash`、`inclusionai/ling-3.0-flash-vl` 与 `inclusionai/ling-3.1-flash`，均为 Flash 规模的 3.x 代次，不能当作 `Ling-2.6-1T` 的等价替代；`Ring-2.6-1T` 在目录内没有任何同名或近名条目。三个模型的价格都非零，不像先前移除的 `nvidia` 那样带免费额度标记。

## 4. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、**484** 个测试全部通过（移除前 482，`config.spec` 参数化多两条） |
| `npx vitest run packages/llm` | 78 个文件、**1793** 个测试全部通过（移除前 1791） |
| `pnpm run test:docs` | 21/21 通过 |
| `pnpm run verify-upgrade-guides` | 通过（英文正文 496 词，未触及 500 词硬上限） |
| `pnpm run verify-doc-budgets` | 通过（`7 budgeted docs within ceiling`） |
| `pnpm run verify-provider-data` | 通过：`Provider metadata matches the installed pi-ai catalog.` |
| `pnpm run verify-translation-pairing` | 1301 对全部一致（三个 i18n 文件重新录制后） |
| 目录自检 | 21 个供应商、710 个模型；`isWithheldProvider('ant-ling')` 为 `true`，`catalogProvider('ant-ling')` 为 `undefined`，strict 与 deferred 均报移除错误 |

## 5. 复查记录

- 这是首个「模型数很少但完全独有」的移除：3 个 id 全部失去内置路由，文档与日志如实写明，没有套用「只是少了路由层」的说法；同时点出 `openrouter` 上的同族 Ling 3.x Flash 变体，避免读者误以为整个 Ling 家族不可达。
- 目录顺序核对：`ant-ling` 在内置目录中排在 `amazon-bedrock` 之后、`anthropic` 之前（字母序亦然），因此 `WITHHELD_PROVIDERS` 与各文档清单都插在该位置，而不是列表开头。
- 三个 golden 的 option 行按 `catalogProviderIds()` 顺序比对：删除后 21 项、顺序一致；e2e 下界同步改为 21。
- 根 README 由并行维护的差异表记录，本轮以独立提交刷新，避免与移除提交混杂。

## 6. 已知限制

- 3 个模型失去全部内置路由；要继续使用需以 `openai-completions` 自定义模型 API 接入 `https://api.ant-ling.com/v1`，Provider ID 需避开 `ant-ling`。该路由原本从 `ANT_LING_API_KEY` 环境变量或设置页保存的凭据取 key；自定义路由需在自身配置里指定凭据（`apiKeyEnv` 可继续指向同名变量）。
- 该路由只有 API key 鉴权，没有 OAuth；移除不影响其他路由的登录流程。
- 推理模型只映射 `high` 与 `xhigh` 两档（`off`、`minimal`、`low`、`medium` 在 `thinkingLevelMap` 里为 `null`）；自定义路由若要用推理，需在兼容档位里声明 `thinkingFormat: 'ant-ling'` 与对应的 `thinkingLevelMap`。
- pi-ai 的目录是静态快照，本适配器不调用 `refreshModels`；自定义模型 API 的模型列表同为静态。
- 运行中的 `127.0.0.1:3080` 需重启后才会显示 21 项。

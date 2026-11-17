# 00024 — 移除 ant-ling 供应商

- **日期**：2026-11-16
- **背景**：`ant-ling` 是 pi-ai 目录中的第一方路由（名称 Ant Ling），以 `openai-completions` 接入 `https://api.ant-ling.com/v1`，只有 API key 鉴权（`ANT_LING_API_KEY`），没有 OAuth。目录快照 3 个聊天模型：非推理的 `Ling-2.6-1T` 与 `Ling-2.6-flash`，以及推理模型 `Ring-2.6-1T`；三者上下文均为 262,144、最大输出 65,536，目录价格为每百万 token $0.06/$0.25、$0.01/$0.02、$0.06/$0.25。三个 id 只出现在这一条路由上。用户要求移除，按既有 `WITHHELD_PROVIDERS` 机制处理。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点与测试）、`docs/user/guide/providers.md`、`packages/llm/llm-pi-ai/README.md`、升级指南、Web golden 与 `logs/`；根 README 的差异表另行刷新。

> **后续变更**：`apps/web/native/dsh-im/` 已由 [00027](00027-原生集成dsh-im.md) 与 [00028](00028-移除四渠道.md) 正式纳入原生 IM；仓库文档门禁已为该 vendored 子树增加例外，本记录中提到的相关失败已不再出现。

- **后续变更（00036）**：`openrouter` 已于 [00036](00036-移除openrouter供应商.md) 移除，本记录中以 `openrouter` 作为承接路由的说明随之失效；3 个 id 仍无其他内置路由；第 1 节中「`openrouter` 只有同族 Ling 3.x Flash 变体」的说明失效——该路由同样已移除。现行覆盖见 00036 第 2 节与 [00009](00009-模型供应商清理汇总.md) 顶部的「后续变更（00036）」。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `ant-ling`（19 → 20 条，按目录顺序插在 `amazon-bedrock` 之后、`azure` 之前）；注释说明 Ling 与 Ring 三个模型都没有其他内置路由，以及 `api.ant-ling.com/v1` 的 OpenAI 兼容重新接入方式 |
| `tests/catalog.spec.ts` | 「withholds removed catalog routes」用例的被移除路由清单增加 `ant-ling`；该路由的模型不带请求头，上一轮改借 `github-copilot` 的 headers 用例无需再动 |
| `tests/config.spec.ts` | 参数化的被移除路由用例增加 `ant-ling`（`it.each` 每条路由一条用例，strict 与 deferred 在同一条内各验一次；测试总数 +1） |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单增加 Ant Ling（排在 Amazon Bedrock 之后）；自定义路由清单增加 `api.ant-ling.com/v1`；新增「三个模型没有其他内置路由」的覆盖句；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的被移除路由清单与 `ant-ling` 说明同步，重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述与变更清单加入 `ant-ling`（19 → 20 条），迁移段给出端点与覆盖说明；正文 496 词（硬上限 500） |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "ant-ling"`（22 → 21 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 22 → 21 |
| 根 `README.md` 差异表（已由后续 `docs(readme)` 提交刷新） | 19 → 20 条移除路由、22 → 21 个路由、713 → 710 个模型、设置页移除 20 个 option，并补 `ant-ling` 覆盖句 |

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
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、**484** 个测试全部通过（移除前 483，`config.spec` 参数化多一条） |
| `npx vitest run packages/llm` | 78 个文件、**1793** 个测试全部通过（移除前 1792） |
| `pnpm run test:docs` | 移除轮与文档改动后均为 21/21；补入本节记录后重跑为 19/21，两处失败（`verify-repository-references` 的 10 处 commit-hash 与 `verify-translation-pairing` 的 3 个无配对 README）全部命中并行开发新出现的未跟踪目录 `apps/web/native/dsh-im/`，与本次改动无关 |
| `pnpm run verify-upgrade-guides` | 通过（英文正文 496 词，未触及 500 词硬上限） |
| `pnpm run verify-doc-budgets` | 通过（`7 budgeted docs within ceiling`） |
| `pnpm run verify-provider-data` | 通过：`Provider metadata matches the installed pi-ai catalog.` |
| `pnpm run verify-translation-pairing` | 1301 对全部一致（三个 i18n 文件重新录制后；补录日志后全量检查因并行 WIP 的未跟踪目录失败，三个受影响的配对单独复查仍一致） |
| 目录自检 | 21 个供应商、710 个模型；`isWithheldProvider('ant-ling')` 为 `true`，`catalogProvider('ant-ling')` 为 `undefined`，strict 与 deferred 均报移除错误 |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试通过（用例不枚举路由，无需改动） |
| `pnpm run doc-sync` | 43/43 全部通过（一次通过） |
| `pnpm run lint` | 0 warning、0 error（90 条规则、7302 个文件） |
| `pnpm run typecheck` | 5/5 通过 |
| `pnpm run build` | 10/10 通过（含 host/client 聚合检查与 Web build） |
| `pnpm run test:snapshot -t pi-ai-conversation-updates` | 1 个录制会话回放通过 |
| `npx vitest run --config vitest.expected.config.ts -t "pi-ai"` | 2 个 headless 场景通过，内联快照未变 |
| Web e2e（`DSH_SNAPSHOT=replay`） | 只回放受影响的 `models-settings` 与 `onboarding-usable-provider` 两个文件：3 通过、19 失败；抽查失败堆栈（含 `-t` 单独回放负责 golden 比对的用例），失败点都在 `openSettings()` 的点击处被并行开发中的启动动画覆盖层（`<div class="dsh-boot-archive-black">…</div> … intercepts pointer events`）拦截，尚未执行到供应商断言 |
| golden 与目录一致性 | 三个 golden 的 option 行（含 `minimax-cn` 的 `[selected]`）与 `catalogProviderIds()` 逐项相等（21 项、同顺序） |
| 文档一致性自检 | 6 个中英文档（用户指南、包 README、升级指南各中英）与根 README 均完整枚举 20 条路由，脚本比对无缺失 |

移除提交与其后的根 README 数字刷新已进入 `main` 并推送到 `origin/main`；本节的校验记录随后以独立提交补入，并在一轮复查后补正（见第 5 节）。工作区另有并行开发的未提交改动（未跟踪的 `apps/web/native/`，以及 `pnpm-workspace.yaml`、`pnpm-lock.yaml`、`scripts/build.ts`、`packages/bundle/web-app/package.json`、`packages/bundle/web-app/cordis.patch.yml`），不属于本次移除。

## 5. 复查记录

- 这是首个「模型数很少但完全独有」的移除：3 个 id 全部失去内置路由，文档与日志如实写明，没有套用「只是少了路由层」的说法；同时点出 `openrouter` 上的同族 Ling 3.x Flash 变体，避免读者误以为整个 Ling 家族不可达。
- 目录顺序核对：`ant-ling` 在内置目录中排在 `amazon-bedrock` 之后、`anthropic` 之前（字母序亦然），因此 `WITHHELD_PROVIDERS` 与各文档清单都插在该位置，而不是列表开头。
- 三个 golden 的 option 行按 `catalogProviderIds()` 顺序比对：删除后 21 项、顺序一致；e2e 下界同步改为 21。
- 根 README 由并行维护的差异表记录，本轮以独立提交刷新，避免与移除提交混杂。
- 审计轮用 `git show` 复核移除提交只含预期文件，并复算前后测试数：`packages/llm/llm-pi-ai` 483 → 484、`packages/llm` 1792 → 1793。早先记为「移除前 482 / 1791」并把参数化写成「+2」，原因是没有注意 `it.each` 每条路由只产生一条用例、strict 与 deferred 在同一条里各验一次；已按实际计数更正。
- 「第一方路由」这一表述的依据：上游 pi-ai README 的 Supported Providers 把它与 OpenAI、Anthropic、DeepSeek、NVIDIA NIM 等厂商并列；目录内它只托管 Ling/Ring 自家模型（3 个），没有转售第三方模型，也不具备聚合网关的多厂商目录特征。若上游日后把它改成聚合端点，用户指南与包 README 的这处描述需要同步改。
- 审计轮共修正 6 处：§1 的「+2」与 §4 的两处「移除前」计数（实际 483/1792，各 +1）、§4 的 e2e 核对措辞（由「逐条」改为抽查加单独回放）、§3 的 `provider-data` 表述（它保留的是全部 42 个内置 id 的元数据，不只是被移除路由）、§6 的自定义路由推理指引（配置字段是 `reasoningEfforts`，而不是直接写 `thinkingLevelMap`）。其余数字经脚本复核不变：21 个提供商 / 710 个模型、golden 22 → 21 项、升级指南 496 词、6 个中英文档与根 README 各 20 条 id。

## 6. 已知限制

- 3 个模型失去全部内置路由；要继续使用需以 `openai-completions` 自定义模型 API 接入 `https://api.ant-ling.com/v1`，Provider ID 需避开 `ant-ling`。该路由原本从 `ANT_LING_API_KEY` 环境变量或设置页保存的凭据取 key；自定义路由需在自身配置里指定凭据（`apiKeyEnv` 可继续指向同名变量）。
- 该路由只有 API key 鉴权，没有 OAuth；移除不影响其他路由的登录流程。
- 推理模型只映射 `high` 与 `xhigh` 两档（`off`、`minimal`、`low`、`medium` 在目录的 `thinkingLevelMap` 里为 `null`）；自定义路由若要用推理，需在 `compat` 里声明 `thinkingFormat: 'ant-ling'`，并用 `reasoningEfforts` 声明可用档位与 wire 值（对应 pi-ai 的 `thinkingLevelMap`）。
- pi-ai 的目录是静态快照，本适配器不调用 `refreshModels`；自定义模型 API 的模型列表同为静态。
- 运行中的 `127.0.0.1:3080` 需重启后才会显示 21 项。

# 00022 — 移除 radius 供应商

- **日期**：2026-11-16
- **背景**：`radius` 是 pi 项目自营的模型网关（`https://radius.pi.dev/v1`，首页自述 "Radius: a Pi-native API gateway"），用自有 `pi-messages` 协议把 Anthropic、OpenAI、DeepSeek、Moonshot、Z.ai、xAI 的 29 个模型聚合在一个端点后转售，并额外提供 `balanced`、`cheap`、`precise` 三个意图别名。它不直连任何厂商，而是与 `vercel-ai-gateway`、`huggingface` 同类的第三方聚合层：29 个模型 id 中有 26 个在其他内置路由上原样出现，剩下三个别名指向的也是其他路由已提供的模型。按本仓库既有的聚合层处理方式移除它。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点、测试）、`docs/user/guide/providers.md`、`packages/llm/llm-pi-ai/README.md`、升级指南、Web golden 与 `logs/`。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `radius`（17 → 18 条）；注释补充聚合性质、26/29 的覆盖、三个意图别名、`pi-messages` 协议与替代接入方式 |
| `tests/catalog.spec.ts` | 「withholds removed catalog routes」用例的被移除路由清单增加 `radius` |
| `tests/config.spec.ts` | 参数化的被移除路由用例增加 `radius`（strict 与 deferred 各一次） |
| `tests/loader-composition.spec.ts` | 该用例原本借 `radius` 断言「固定目录端点进入目录」，改用仍提供的 `openrouter` |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单增加 Radius；自定义路由清单增加 `radius.pi.dev/v1`；补覆盖说明；把「网关的 301 个模型 id」改为「Vercel AI Gateway 的 301 个模型 id」，消除出现两个网关后的指代歧义；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的被移除路由清单与说明同步，重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述、变更清单与覆盖说明加入 `radius`，并给出 OpenAI 兼容重新接入方式；英文正文 475 词 |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "radius"`（24 → 23 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 24 → 23 |

## 2. 行为

- 可对话供应商 24 → **23**，目录模型 762 → **733**（只减 `radius` 的 29 个模型）。
- `catalogProvider('radius')` 返回 `undefined`，`catalogModels('radius')` 为空；目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由。
- 存量 profile 点名 `radius` 时 `resolveProfiles()` 在 strict 与 deferred 下都抛 `llm-pi-ai: provider "radius" is removed from this build; delete the route or select another provider`，同一 `providers` 字典里的其他路由随之不可用，删除该条目即恢复。
- `pi-messages` 不在可配置协议表内（`supportedProtocols()` 只有 `openai-completions`、`openai-responses`、`anthropic-messages`、`google-generative-ai`），因此该路由无法按原协议重建。网关端点同时提供 OpenAI 兼容对话补全：实测 `POST https://radius.pi.dev/v1/chat/completions` 与 `/v1/completions` 都返回 401 `Missing gateway API key`（端点存在，只缺凭据），需要时可用 `openai-completions` 自定义模型 API 重新接入，Provider ID 需避开 `radius`。
- `provider-data.generated.ts` 仍保留 42 个 id 的元数据（`verify-provider-data` 要求与已安装目录一致），它不是可售路由；内置 id 现为 42 = 23 个提供 + `typesafe` + 18 个被移除。
- 文档与 golden 的被移除路由统一为十八条。

## 3. 模型覆盖

29 个模型 id 中，**26 个在其他内置路由上以相同 id 出现**（0 个只在换了厂商前缀时出现），其余 3 个是意图别名，没有同 id 路由；但三个别名各自等价于目录中的一个具体型号（上下文窗口、输出上限、输入/输出/缓存价格与厂商字段逐项相同）：

| 别名 | 等价型号 | 其他内置路由 | 目录价格（输入/输出） |
|---|---|---|---|
| `balanced` | `kimi-k3` | `baseten`、`fireworks`、`github-copilot`、`opencode-go`、`openrouter`、`together` | $3 / $15 |
| `cheap` | `deepseek-v4.1-flash` | `baseten`、`nvidia`、`opencode-go`、`openrouter`、`together` | $0.3 / $1.2 |
| `precise` | `gpt-5.6-sol` | `github-copilot`、`openai`、`openai-codex`、`openrouter` | $4 / $20 |

因此本次移除不减少任何模型覆盖：26 个同名模型可直接改用其他路由，3 个别名按上表选择等价型号即可；消失的是一条第三方聚合与计费层，以及它自己的 `pi-messages` 协议和 OAuth 登录。别名与等价型号的对应只来自目录字段比对（别名条目的 `lab`、`cost`、`contextWindow`、`maxTokens` 与等价型号完全一致），网关内部如何在候选上游之间选路不在本仓库控制范围内。

## 4. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、**482** 个测试全部通过（移除前 481，`config.spec` 参数化多一条） |
| `npx vitest run packages/llm` | 78 个文件、**1791** 个测试全部通过（移除前 1790） |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、**483** 个测试全部通过（用例不涉及 `radius`，无需改动） |
| `pnpm run test:docs` | **21/21** 通过（含 `upgrade guides`、`translation pairing`、`markdown wrap`、`markdown links`）；审计轮修改升级指南与日志后重跑仍为 21/21 |
| `pnpm run verify-upgrade-guides` | 通过（英文正文 475 词，未触及 500 词硬上限） |
| `pnpm run verify-doc-budgets` | 通过（`7 budgeted docs within ceiling`） |
| `pnpm run verify-provider-data` | 通过：`Provider metadata matches the installed pi-ai catalog.` |
| `pnpm run build:artifacts` | 8 个构建门禁全部通过（host/client 编译与打包、Desktop、Web，28.25s；e2e 回放依赖此步产物） |
| `pnpm run typecheck` | **5/5** 通过（host/client 编译与聚合检查，8.24s） |
| `pnpm run lint` | **0 warning、0 error**（90 条规则、7279 个文件，79.9s） |
| `pnpm run build` | **10/10** 通过（31.30s，含 host/client 聚合检查与 Web build） |
| `pnpm run doc-sync` | 移除轮 42/43（唯一失败是并行开发中的 `Cordis inspect catalog`，`api-catalog.ts` 过期）；审计轮重跑 **43/43 全部通过**，并行 agent 已重新生成该目录 |
| `pnpm run test:snapshot -t pi-ai-conversation-updates` | 1 个录制会话回放通过（`replays pi-ai-conversation-updates through dsh --profile headless`） |
| `npx vitest run --config vitest.expected.config.ts -t "pi-ai"` | 2 个 headless 场景通过，内联快照未变 |
| 目录自检 | 23 个供应商、733 个模型；`radius` 不在可配置列表、无 provider、无模型；strict/deferred 均报移除错误 |
| golden 与目录一致性 | 三个 golden 的 option 列表与排序后的 `catalogProviderIds()` 逐项相等（23 个、同顺序） |
| Web e2e（`DSH_SNAPSHOT=replay`） | 只回放受影响的 `models-settings` 与 `onboarding-usable-provider` 两个文件：2 个文件 5 通过、17 失败，失败全部来自并行开发中的 `ui-boot-animation`（全窗口启动视频拦截指针事件导致 `locator.fill` 超时，且设置导航新增 `button "启动片头"`）。两个完成真实比对的 golden 与预期差异**只有该按钮一行**，供应商 option 列表逐项一致（23 项） |
| 升级指南词数 | 英文正文 **475** 词（硬上限 500、仓库实践 475），中英双语结构一致 |

**最终状态**：本次移除涉及的文件已全部提交（移除提交已在 `origin/main`；审计轮的修复因分支随后被并行开发推进，改为后续提交而不是改写原移除提交），工作区里其余改动属于并行开发的 `ui-boot-animation` 等，与本移除无关。

## 5. 复查记录

- `tests/loader-composition.spec.ts` 原先用 `radius` 断言「固定目录端点会进入可配置列表」。加入移除后该断言必失败（条目已不存在），改用仍提供且端点固定的 `openrouter`，保持用例意图不变。`tests/native-auth.spec.ts` 对 `PROVIDER_DATA.radius` 的断言保留：`provider-data.generated.ts` 仍覆盖全部 42 个已安装 id，与可售路由无关。
- 出现第二个「网关」后，`catalog.ts` 注释与 `providers.md` 里单独出现的「the gateway」会把读者指向 `vercel-ai-gateway` 还是 `radius` 变得不确定，已分别改为「the Vercel AI Gateway」和「Vercel AI Gateway 的 301 个模型 id」。
- 升级指南加入 `radius` 后英文正文一度到 483 词，超出仓库 475 词实践上限（硬上限 500）。做了等义收紧：把 SGP、AMS、Moonshot、阿里云四个端点收成一处枚举，删去「个人版开放其中九个」「目录无法恢复该路由」等与用户指南重复的细节，`cerebras` 句与 `xiaomi` 句各去掉一个冗余限定词，随后审计轮把补回的 `kimi-coding` 指引计入，最终 **475 词**；中文指南逐句同步同样的删减。
- `pi-messages` 不是可配置协议，因此「可用自定义模型 API 重新接入」不能照抄其他路由的写法。改为给出网关实际提供的 OpenAI 兼容对话补全端点，并实测 `POST /v1/chat/completions`、`/v1/completions`、`/v1/messages` 均返回 401 `Missing gateway API key`（端点存在、只缺凭据），避免写出一个不存在的接入方式。
- 「26 of 29 保持同 id」这一覆盖结论由脚本按「末段名称相同且模型 id 不含前缀」的口径统计得出（同 id 26 个、仅换厂商前缀 0 个、无同 id 3 个），而不是只按名称模糊匹配，避免把 `zai-org/GLM-5.3` 之类的换前缀条目算成同 id。
- 审计轮（提交并推送后）复核：运行时仍是 23 个供应商、733 个模型，`isWithheldProvider('radius')` 为 `true`，`catalogProvider('radius')` 为 `undefined`，strict 与 deferred 都抛移除错误；三个 golden 的 23 项与 `catalogProviderIds()` 排序逐项一致；三个意图别名的等价性改为**全字段比对**（除 id、name、provider、baseUrl、providers 外无任何差异），比原先只比上下文与价格更强。发现并修复两处问题：`logs/00009` 的控制点表格仍写「十七个路由的元数据」（已改十八）；上一轮为压词数把升级指南里的 `kimi-coding` 指引一并删掉，属于迁移信息缺失，已补回一句并把正文重新压回 475 词。
- e2e 首轮 17 个失败先被怀疑与本次 golden 改动有关；逐条比对后确认全部来自并行包的启动视频覆盖层与新增设置按钮，golden 侧只有 `option "radius"` 一处删除，且删除后列表与目录一致。

## 6. 已知限制

- `radius` 的意图别名不属于任何具体厂商：移除后需按等价型号自行选择路由，上表给出对应关系；别名本身没有同 id 的内置路由。
- pi-ai 的 `radiusProvider` 支持 `refresh()` 从 `/v1/config` 覆盖随包基线，而本适配器只读 `getBuiltinModels()` 的静态基线，因此选择器一直显示的是 29 个随包模型。实测未带凭据的 `/v1/config` 当前返回 36 个：比基线多 11 个（`claude-fable-5-2`、`gpt-6.1-sol`、`gpt-6.1-terra`、`kimi-k3.1-pro`、`minimax-m3`、`minimax-m3.1-flash`、`gemini-3.8-flash`、`gemini-4-argon`、`gemini-4-hydrogen`、`mistral-large-4`、`recursive/flint`），基线里的 4 个（`gpt-5.3-codex`、`gpt-5.4`、`gpt-5.4-mini`、`gpt-5.5`）已不在其中。移除该路由后本构建不再具备这份动态目录能力；自定义模型 API 的模型列表是静态的。
- 该路由的 OAuth 登录（pi-gateway 浏览器与设备码流程，回调 `127.0.0.1:1456`）随路由一并移除，重新接入只能使用 API key。
- 运行中的 `127.0.0.1:3080` 需重启后才会显示 23 项。

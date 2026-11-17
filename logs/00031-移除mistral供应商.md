# 00031 — 移除 mistral 供应商

- **日期**：2026-11-17
- **背景**：`mistral` 是 pi-ai 目录里的第一方平台路由（Mistral AI），默认端点 `https://api.mistral.ai`，使用 Mistral 自家的 `mistral-conversations` 协议（对话补全路径为 `/v1/chat/completions`），凭据为 `MISTRAL_API_KEY`，没有 OAuth 登录或订阅套餐入口。目录快照 40 个模型：Mistral Large 4、Large 3 与 Large 2.1，Medium 3 / 3.1 / 3.5，Small 3.2 / 4，Codestral、Devstral、Ministral、Magistral、Pixtral、Voxtral 系列，`open-*` 开源权重条目，以及平台代管的三个 GLM 条目（`glm-5-2` 与 `zai-glm-5-2` 同为 GLM-5.2，`zai-glm-5-3` 为 GLM-5.3）。
- **判断依据**：与之前移除的转售型或聚合型路由不同，`mistral` 是模型原厂入口。40 个条目里 29 个能在其他已提供路由上取到同一模型（`openrouter` 的 `mistralai/*` 条目，GLM 两条另有 `together` 与 `opencode-go` 提供），其余 11 个没有其他内置路由：Pixtral 两个、Magistral Medium、Leanstral 1.5、Devstral Medium、Devstral Small 两个、`labs-devstral-small-2512`、Mistral Large 2.1、Mistral 7B 与 Mixtral 8x7B。因此移除它拿掉的不只是托管入口，还有这 11 个模型的访问，文档与日志都如实记录这一点。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点与测试）、`docs/user/guide/providers.md`、包 README、升级指南、Web golden 与 e2e 断言、根 README、`logs/`。
- **后续变更**：`together` 已于 [00032](00032-移除together供应商.md) 移除，本记录中作为承接路由出现的 `together`（GLM 两条）不再可用；`glm-5.2` 仍由 `opencode-go` 与 `openrouter` 提供，`glm-5.3` 另见 `zai-coding-cn`、`opencode-go` 与 `openrouter`，现行覆盖见 [00009](00009-模型供应商清理汇总.md) 第 8 节。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `mistral`（24 → 25 条，字母序放在 `minimax` 之后）；注释新增 `mistral` 段，说明它是第一方平台入口、29 / 11 的覆盖拆分与 `https://api.mistral.ai/v1` 的 OpenAI 兼容重接入方式；末尾「哪些路由使用 OpenAI 兼容对话补全」的枚举加入 Mistral |
| `tests/catalog.spec.ts` | 「withholds removed catalog routes」用例的被移除路由清单增加 `mistral` |
| `tests/config.spec.ts` | 参数化的被移除路由用例增加 `mistral`（strict 与 deferred 在同一条内各验一次） |
| `tests/compat-upgrade.spec.ts` | 原「withholds Mistral conversation capabilities」用例通过点名 `mistral` 路由触发兼容字段拒绝；路由被屏蔽后该路径改为删除错误，用例改述为「refuses a declared route on the withheld Mistral protocol」，断言手工声明路由无法选择 `mistral-conversations`——该协议也不在四个可选自定义协议内，因此被屏蔽的兼容字段没有可用路由承载 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单增加 Mistral（第一方平台端点）；自定义端点清单加入 `api.mistral.ai/v1`；补一句 29 / 11 的覆盖拆分；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的被移除路由清单与 Mistral 说明同步；重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述与变更清单加入 `mistral`（24 → 25 条）；迁移段的 OpenAI 兼容端点清单加入 `api.mistral.ai/v1`；覆盖句补入 `mistral` 的 11 个独有模型与 29 个可替代条目；为压回 500 词上限，精简了迁移步骤、`fireworks` 句与结尾句；正文 499 词；重录 i18n 哈希 |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "mistral"`（17 → 16 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 17 → 16 |
| 根 `README.md` 差异表 | 24 → 25 个移除路由、17 → 16 个路由、652 → 612 个模型、设置页 24 → 25 个 option，并补 Mistral 说明 |
| `logs/00009` | 汇总刷新为 16 个路由 / 612 个模型 / 25 条被移除路由，并加入本日志条目 |

## 2. 行为

- 可对话供应商 17 → **16**，目录模型 652 → **612**（只减 `mistral` 的 40 个模型）。
- `catalogProvider('mistral')` 返回 `undefined`，`catalogModels('mistral')` 为空；目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由。
- 存量 profile 点名 `mistral` 时 `resolveProfiles()` 在 strict 与 deferred 下都抛 `llm-pi-ai: provider "mistral" is removed from this build; delete the route or select another provider`，同一 `providers` 字典里的其他路由随之不可用，删除该条目即恢复。
- 内置 id 现为 42 = 16 个提供 + `typesafe` + 25 个被移除；`provider-data.generated.ts` 仍保留 42 个 id 的元数据（`verify-provider-data` 要求与已安装目录一致），它不是可售路由。
- `packages/experimental/webworker-runtime` 的 `BUILTIN_PROVIDER_IDS` 仍列出 `mistral`：该列表描述 pi-ai 已安装目录，用于让桥接层拿到合法 id 并让每个请求落到明确的未实现错误，与本次移除无关。
- 自定义路由无法再以 `mistral-conversations` 协议接入：可配置协议只有 `openai-completions`、`openai-responses`、`anthropic-messages` 与 `google-generative-ai`；需要原平台时按 OpenAI 兼容入口 `https://api.mistral.ai/v1` 声明。
- 未触及 agent loop、模型可见工具、Session 事件或持久化格式；`deepseek-official` 与 `deepseek-account` 原生路由不受影响。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm exec vitest run packages/llm/llm-pi-ai/tests` | 22 个文件、489 个测试全部通过 |
| `pnpm exec vitest run packages/llm`（全 llm 包） | 78 个文件、1798 个测试全部通过 |
| `pnpm exec vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过（设置页读取同一份目录） |
| `pnpm run typecheck` | 5 passed，0 failed |
| `pnpm run lint` | 0 warning、0 error（90 条规则、7303 个文件，71.6s） |
| `pnpm run build` | 11 个门禁全部通过（27.15s，含 host/client 聚合检查与 Web build） |
| `pnpm run test:docs` | 21 个门禁全部通过（16.16s，含升级指南词数与中英配对） |
| `pnpm run doc-sync` | 43 个门禁全部通过（51.92s） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件仍保留 42 个 id 的元数据） |
| `pnpm exec tsx scripts/gen-config-catalog.ts --check` | 3 个生成产物均为最新 |
| `pnpm exec tsx scripts/verify-upgrade-guides.ts` | 通过，guide.md 499 词（上限 500） |
| 目录核对（构建产物 probe） | 16 个提供路由、612 个模型；`catalogProvider('mistral')` 为 `undefined`，`catalogModels('mistral')` 为 0，`isWithheldProvider('mistral')` 为真 |
| Web golden 单项回放 | 临时 overlay 关闭 `ui-boot-animation` 后回放 `models-settings` 的 `opens the add card over the dormant directory vocabulary`：provider `option` 列表与 golden 逐项一致（16 项、不含 `mistral`）；唯一差异是金标未含的既有 `IM机器人` 导航按钮，与本改动无关。临时 overlay 与测试改动已还原 |
| 复查后重跑 | `vitest packages/llm/llm-pi-ai/tests` 22 / 489、`vitest packages/llm` 78 / 1798、`vitest packages/client/ui-settings-models` 14 / 483、`typecheck` 5/5、`lint` 0 warning / 0 error（7303 个文件，71.4s）、`build` 11/11（末次 26.23s）、`test:docs` 21/21（末次 15.72s）、`doc-sync` 43/43（末次 52.75s）、`verify-upgrade-guides` 通过（499 词）、`gen-config-catalog --check` 3 个产物最新、`verify-provider-data` 一致 |
| 文档清单核对 | 7 份中英文档（用户指南、包 README、升级指南各一对与根 README）均枚举全部 25 个被移除 id，无缺失、无多余；根 README 的移除清单为 25 项且包含 `mistral` |
| `git diff --check` | 通过 |

## 4. 复查记录

- 覆盖口径：脚本按「同一模型」比对 40 个条目与其余 15 个已提供路由的 id 与显示名，得到 29 个有替代、11 个无替代；11 个逐项列名为 Pixtral 两个、Magistral Medium、Leanstral 1.5、`devstral-medium-2507`、`devstral-small-2505`、`devstral-small-2507`、`labs-devstral-small-2512`、`mistral-large-2411`、`open-mistral-7b`、`open-mixtral-8x7b`。
- 复查修正 1 处：文档初稿把独有的 Devstral 条目写作「Devstral Medium 与 Devstral Small 条目」，未点明 `labs-devstral-small-2512`（Devstral Small 2）也在其中，且与前文的 11 项计数对不上；6 份文档与日志统一改为「Devstral Medium、Devstral Small 的三个条目」。
- 复查修正 1 处：`compat-upgrade.spec.ts` 原用例通过点名 `mistral` 路由验证 `supportsMidConvoSystemMessages` 被拒绝；路由被屏蔽后该路径先触发删除错误，用例改为断言手工声明路由无法选择 `mistral-conversations`（四个可选协议之外），保留对该协议不可配置的覆盖；断言同时收紧为匹配错误文本 `/mistral-conversations/`（实测报错为 `$.providers.gateway.api expected "openai-completions" | … but got "mistral-conversations"`），避免因其他原因抛错而空过。
- 复查修正 1 处：`catalog.ts` 末尾「哪些路由使用 OpenAI 兼容对话补全」的枚举初稿把 `mistral` 与真正使用 `openai-completions` 的路由并列，而它的目录路由用的是 `mistral-conversations`；已改为「Mistral 与 MiniMax 端点在自己的协议之外也接受 OpenAI 兼容请求」，与本段开头「第一方平台端点」的说法一致。
- 复查修正 1 处：新写的 `Z.ai GLM` 与仓库既有的 `Z.AI` 写法不一致，包 README（中英）与 `catalog.ts` 统一为 `Z.AI`；同时把 `catalog.ts` 的 `mistral` 段落与末尾协议小结重排为整句换行，避免出现 `Its` 起头的断行。
- 复查修正 1 处（用户可见）：升级指南中英 frontmatter 的 `description` 仍只列 24 条路由，漏了 `mistral`——正文与 Change 清单已被替换，而描述行不带反引号，前一轮的替换没有命中它；两份描述行补入 `mistral` 后为 25 条，英文正文回到 500 词上限内（再精简一处后为 499 词）。
- 复查核对：`mistral-conversations` 的请求体字段（`top_p`、`max_tokens`、`tool_choice`、`tool_calls`、`stream`）与 OpenAI 形状一致，端点路径为 `/v1/chat/completions`，因此「`api.mistral.ai/v1` 可按 `openai-completions` 接入」的依据是对话框协议与路径，而非仅凭厂商宣传。
- 连带核对：`mistral` 的 GLM 三个条目（`glm-5-2`、`zai-glm-5-2` 同为 GLM-5.2，`zai-glm-5-3` 为 GLM-5.3）在 `openrouter`、`together`、`opencode-go` 上均可取到，故计入「29 个有替代」而非独有。
- 未改动 `provider-data.generated.ts` 与 `packages/experimental/webworker-runtime` 的 stub 列表：两者描述已安装目录而非可售路由，`verify-provider-data` 要求保留全部 42 个 id。
- 升级指南加词后一度为 529 词，按「合并迁移步骤、精简 `fireworks` 句与结尾句」压回 499 词，中英结构保持一致。

## 5. 已知限制与恢复

- 本次只屏蔽可售路由，不删除 pi-ai 包内 Mistral 的实现、目录元数据与 `mistral-conversations` 协议实现；手工声明同名路由仍会被 `resolveProfiles()` 拒绝。
- 恢复时删除 `WITHHELD_PROVIDERS` 中的 `mistral`，把可售路由与模型计数改回 17 / 652，并同步撤销第 1 节列出的 golden、断言与文档改动；`compat-upgrade.spec.ts` 需恢复通过点名路由触发兼容字段拒绝的用例；升级指南词数需重新核对。
- 移除后 11 个模型失去全部内置路由（见第 4 节），需要 Mistral 端点时按 `openai-completions` 自定义模型 API 接入 `https://api.mistral.ai/v1`；`mistral-conversations` 不在自定义协议的四个选项中。
- 运行中的 Web 宿主仍是改动前启动的进程，重启 `pnpm dsh web` 后设置页才会显示 16 项且不含 `mistral`。

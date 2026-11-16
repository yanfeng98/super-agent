# 00023 — 移除 nvidia 供应商

- **日期**：2026-11-16
- **背景**：`nvidia` 是 NVIDIA 自建的托管推理服务（NVIDIA NIM，API Catalog 在 `build.nvidia.com`），以 `openai-completions` 接入 `https://integrate.api.nvidia.com/v1`，只有 API key 鉴权（`NVIDIA_API_KEY`），没有 OAuth。目录快照 20 个模型，id 一律带厂商前缀：NVIDIA 自家的 Nemotron 与 Cosmos 系列，加上代托管的其他厂商开源权重模型（DeepSeek、Google Gemma、Meta Llama、Mistral、Moonshot Kimi、OpenAI gpt-oss、Poolside、Z.ai GLM）。用户要求移除，按既有 `WITHHELD_PROVIDERS` 机制处理。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点与测试）、`docs/user/guide/providers.md`、`packages/llm/llm-pi-ai/README.md`、升级指南、Web golden 与 `logs/`。

- **后续变更**：`baseten` 已于 [00026](00026-移除baseten供应商.md) 移除，本记录中作为承接路由出现的 `baseten` 不再可用；9 个同名模型仍由 `groq`、`opencode-go`、`openrouter`、`together`、`zai-coding-cn` 提供，11 个独有模型的结论不变，现行覆盖见该文件第 3 节。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `nvidia`（18 → 19 条）；注释补充其自建推理云性质、20 个 id 中 9 个在其他内置路由上可取的覆盖、其余 11 个独有模型，以及 `integrate.api.nvidia.com/v1` 的 OpenAI 兼容重新接入方式 |
| `tests/catalog.spec.ts` | 「withholds removed catalog routes」用例的被移除路由清单增加 `nvidia`；原本借 `nvidia` 断言「覆盖模型容量时不丢失目录自带 headers」的用例改用 `github-copilot` |
| `tests/config.spec.ts` | 参数化的被移除路由用例增加 `nvidia`（strict 与 deferred 各一次） |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单增加 NVIDIA；自定义路由清单增加 `integrate.api.nvidia.com/v1`；`huggingface` 覆盖句由「64 个在 `openrouter`、1 个在 `nvidia`、13 个无路由」改为「64 个在 `openrouter`、其余 14 个无路由」；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的被移除路由清单、`nvidia` 说明与 `huggingface` 覆盖数字同步，重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述与变更清单加入 `nvidia`，迁移段给出 OpenAI 兼容端点与 9/20 覆盖说明，`huggingface` 覆盖改为 14；等义收紧后正文回到 475 词 |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "nvidia"`（23 → 22 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 23 → 22 |

## 2. 行为

- 可对话供应商 23 → **22**，目录模型 733 → **713**（只减 `nvidia` 的 20 个模型）。
- `catalogProvider('nvidia')` 返回 `undefined`，`catalogModels('nvidia')` 为空；目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由。
- 存量 profile 点名 `nvidia` 时 `resolveProfiles()` 在 strict 与 deferred 下都抛 `llm-pi-ai: provider "nvidia" is removed from this build; delete the route or select another provider`，同一 `providers` 字典里的其他路由随之不可用，删除该条目即恢复。
- `provider-data.generated.ts` 仍保留 42 个 id 的元数据（`verify-provider-data` 要求与已安装目录一致），它不是可售路由；内置 id 现为 42 = 22 个提供 + `typesafe` + 19 个被移除。
- 文档与 golden 的被移除路由统一为十九条。

## 3. 模型覆盖

20 个模型 id 中，**9 个在其他内置路由上以同名提供**（其中 8 个 id 完全相同，`deepseek-ai/deepseek-v4.1-flash` 在 `baseten`、`opencode-go`、`openrouter` 上前缀不同）：

| 模型 id | 其他内置路由 |
|---|---|
| `deepseek-ai/deepseek-v4.1-flash` | `baseten`（`deepseek-ai/DeepSeek-V4.1-Flash`）、`opencode-go`、`openrouter` |
| `google/gemma-3-12b-it` | `openrouter` |
| `meta/muse-glimmer-30b` | `openrouter` |
| `moonshotai/kimi-k2.6` | `baseten`（`moonshotai/Kimi-K2.6`）、`openrouter` |
| `nvidia/nemotron-3-ultra-550b-a55b` | `openrouter`、`together` |
| `openai/gpt-oss-20b` | `groq`、`openrouter` |
| `poolside/laguna-xs-2.1` | `openrouter` |
| `z-ai/glm-5.3` | `baseten`（`zai-org/GLM-5.3`）、`opencode-go`、`openrouter`、`zai` |
| `z-ai/glm-5.3-flash` | `baseten`、`opencode-go`、`openrouter`、`zai` |

其余 **11 个没有其他内置路由**，是本次移除真正减少的覆盖：Nemotron 系列 5 个（`nvidia/llama-3.1-nemotron-70b-instruct`、`nvidia/llama-3.1-nemotron-ultra-253b-v1`、`nvidia/nemotron-3-nano-omni-30b-a3b-reasoning`、`nvidia/nemotron-3.5-lightning-30b-a3b`、`nvidia/nemotron-3.5-super-vl-120b-a12b`）、Cosmos 1 个（`nvidia/cosmos-reason2-8b`）、其他开源权重 5 个（`google/diffusiongemma-26b-a4b-it`、`google/gemma-3-4b-it`、`meta/llama-3.2-11b-vision-instruct`、`meta/llama-3.2-90b-vision-instruct`、`mistralai/mistral-7b-instruct-v0.3`）。

连带影响：`huggingface` 的 78 个 Hub 名称里，原先只有 `google/gemma-3-4b-it` 落在 `nvidia` 上；该路由移除后它也没有内置路由，所以 `huggingface` 的「无其他内置路由」数字由 13 变为 14，三处文档同步更新。

## 4. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、**483** 个测试全部通过（移除前 482，`config.spec` 参数化多一条） |
| `npx vitest run packages/llm` | 78 个文件、**1792** 个测试全部通过（移除前 1791） |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、**483** 个测试全部通过（用例不涉及 `nvidia`，无需改动） |
| `pnpm run test:docs` | 21/21 通过（审计轮在最终内容上重跑仍 21/21） |
| `pnpm run verify-upgrade-guides` | 通过（英文正文 475 词，未触及 500 词硬上限） |
| `pnpm run verify-doc-budgets` | 通过（`7 budgeted docs within ceiling`） |
| `pnpm run verify-provider-data` | 通过：`Provider metadata matches the installed pi-ai catalog.` |
| `pnpm run verify-translation-pairing` | 1301 对全部一致（审计轮改动 README 与升级指南后重新录制） |
| `pnpm run lint` | 0 warning、0 error（90 条规则、7302 个文件；pre-commit 用 49 条规则的精简配置另报一条与本次无关的旧 disable 告警，见第 5 节） |
| `pnpm run typecheck` | 5/5 通过 |
| `pnpm run build` | 10/10 通过（含 host/client 聚合检查与 Web build） |
| `pnpm run doc-sync` | 43/43 全部通过（移除轮与审计轮各跑一次，均一次通过） |
| 目录自检 | 22 个供应商、713 个模型；`isWithheldProvider('nvidia')` 为 `true`，`catalogProvider('nvidia')` 为 `undefined`，strict 与 deferred 均报移除错误 |
| Web e2e（`DSH_SNAPSHOT=replay`） | 只回放受影响的 `models-settings` 与 `onboarding-usable-provider` 两个文件：3 通过、19 失败；逐条核对失败堆栈，全部是并行开发中的启动动画覆盖层拦截指针事件（`<div class="dsh-boot-archive-black">…</div> … intercepts pointer events`），尚未走到供应商断言 |
| golden 与目录一致性 | 三个 golden 的 option 列表与排序后的 `catalogProviderIds()` 逐项相等（22 个、同顺序） |
| `pnpm run test:snapshot -t pi-ai-conversation-updates` | 1 个录制会话回放通过 |
| `npx vitest run --config vitest.expected.config.ts -t "pi-ai"` | 2 个 headless 场景通过，内联快照未变 |
| 升级指南词数 | 英文正文 **475** 词（硬上限 500、仓库实践 475），中英双语结构一致 |

**最终状态**：本次移除与审计修正已拆成两个提交（移除在前，措辞修正随后），工作区无遗留改动；根 README 的「与上游的差异」表由并行的根 README 提交另行记录，其中 19 条路由、22 个路由、713 个模型与 `huggingface` 13 → 14 的数字与本节一致。

## 5. 复查记录

- `tests/catalog.spec.ts` 里「覆盖模型容量时不丢失目录自带 headers」的用例原先借 `nvidia` 实现。目录中只有 `nvidia` 与 `github-copilot` 的模型带 headers（`nvidia` 是 `NVCF-POLL-SECONDS`，`github-copilot` 是 Copilot 的 UA 与版本头），因此 `nvidia` 一旦被移除该用例必然失败；改用 `github-copilot` 保持用例意图（headers 与容量覆盖互不干扰）不变。
- `huggingface` 的覆盖数字是按「去掉厂商前缀、忽略大小写后同名」的口径重算的：64 个在 `openrouter`、1 个只在 `nvidia`（`google/gemma-3-4b-it`）、13 个无路由。移除 `nvidia` 后 13 变 14，用户指南、包 README 与升级指南三处同步，没有留下「1 个在 `nvidia`」的过期表述。
- 升级指南加入 `nvidia` 后一度到 486 词，超出仓库 475 词实践上限（硬上限 500）。按等义收紧回到 475 词：删去 MiMo UltraSpeed 的型号对照句与「三条 Qwen token 套餐提供相同二十个模型」这类与用户指南重复的细节，把 `huggingface` 的「29 个换了厂商前缀」压成「29 renamed」，`minimax` 句改成并列分号，迁移步骤去掉重复的「在页面中保存的行」与「模型 ID」限定。`kimi-coding` 这条迁移指引保留——上一轮就是在这里被误删过。
- `packages/experimental/webworker-runtime` 的 pi-ai stub 有一个固定的 `BUILTIN_PROVIDER_IDS` 预览列表，里面仍列着 `nvidia`。该列表是「合法已安装 id」的静态子集（且早已不含 `radius`），stub 返回的 provider 没有模型，`servesChat` 会把它们全部过滤，因此不需要随本次移除改动。
- `logs/00022` 的 `cheap` 别名行仍把 `nvidia` 列为 `deepseek-v4.1-flash` 的路由之一。那是 radius 移除时的历史记录，当时确实如此；该模型现在仍在 `baseten`、`opencode-go`、`openrouter`、`together` 上，结论不变，故不改写历史日志。
- Web e2e 的 19 个失败先被怀疑与本次 golden 改动有关；用 `-t` 单独回放负责 golden 比对的用例后确认，失败点是 `openSettings()` 的点击被 `dsh-boot-archive-black` 覆盖层拦截，还没执行到列表断言。随后用脚本把三个 golden 的 `option` 行与 `catalogProviderIds()` 逐项 diff，结果完全一致（22 项、同顺序）。
- 审计轮（提交后）复核：脚本把 6 个文档的 19 条 id 清单与 `WITHHELD_PROVIDERS` 逐项比对，无缺失且顺序一致；把第 3 节「9 个模型 × 声明的每条路由」逐格验证，9/9 成立；把三个 golden 的 option 行与 `catalogProviderIds()` 全量 diff，22 项完全一致；重跑 `packages/llm`（1792）、`test:docs`、`doc-sync`（43/43）、`verify-translation-pairing`（1301 对）、`verify-upgrade-guides`、`verify-doc-budgets` 均通过。发现并修复 3 处：`nine of its twenty ids` 把「8 个同 id + 1 个换前缀」写成九个同 id，JSDoc、两个 README、两份升级指南与两份日志一并改为 `models` 并写明换前缀的那一条；第 6 节误称移除后端点仍读 `NVIDIA_API_KEY`，已改为自定义路由需自带凭据、可用 `apiKeyEnv` 指向同名变量；汇总日志把第 5 节记作「复查 5 处」，实为 7 条，已改。
- pre-commit 钩子在 `catalog.ts` 报一条「unused oxlint-disable」：用同一份 `.oxlintrc.staged.json`（49 条规则）跑移除前的同一文件同样报警，是精简配置把类型感知的 disable 判成无用，与本次改动无关；仓库级 `pnpm run lint`（90 条规则）为 0 warning、0 error。
- 本次与之前几次聚合层移除不同：`nvidia` 有 11 个独有模型，移除确实减少模型覆盖。文档与日志据此如实写明「失去的模型」与重新接入方式，没有套用「只是少了路由层」的说法。

## 6. 已知限制

- 11 个模型失去全部内置路由（见第 3 节）；要继续使用需以 `openai-completions` 自定义模型 API 接入 `https://integrate.api.nvidia.com/v1`，Provider ID 需避开 `nvidia`。该路由原本从 `NVIDIA_API_KEY` 环境变量或设置页保存的凭据取 key；自定义路由需在自身配置里指定凭据（`apiKeyEnv` 可继续指向同名变量）。
- 该路由只有 API key 鉴权，没有 OAuth；移除不影响其他路由的登录流程。
- 目录中除 `nvidia/nemotron-3-ultra-550b-a55b`（$0.5 / $2.5 每百万 token）外各模型价格均记为 0，对应 build.nvidia.com 的免费试用额度；移除后这些「零价」条目也从选择器中消失，不再出现「免费」的误导性价格。
- pi-ai 的目录是静态快照，本适配器不调用 `refreshModels`：实测未带凭据的 `https://integrate.api.nvidia.com/v1/models` 公开返回 33 条，而随包只有 20 条（12 条仍在远端、8 条已下线），另有 21 条新条目从未收录。移除该路由后本构建不再具备这份可刷新来源；自定义模型 API 的模型列表是静态的。
- 运行中的 `127.0.0.1:3080` 需重启后才会显示 22 项。

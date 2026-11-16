# 00026 — 移除 baseten 供应商

- **日期**：2026-11-16
- **背景**：`baseten` 是 pi-ai 目录里的第三方推理云路由，托管其他厂商的开源权重模型（DeepSeek、Kimi、GLM、Nemotron、gpt-oss、Thinking Machines 系列），以 `openai-completions` 接入 `https://inference.baseten.co/v1`，只用 API key（环境变量 `BASETEN_API_KEY`），没有 OAuth 或订阅计划。目录快照 22 个模型，全部支持推理；思考参数走该路由特有的 `chat_template_args` 形式（`thinkingFormat: 'baseten'`）。用户要求移除该供应商，且不能破坏项目现有功能，因此沿用既有的 `WITHHELD_PROVIDERS` 机制：目录、模型解析、发现、登录与可配置供应商列表全部不再出现该路由，而不是修改 `node_modules` 里的 pi-ai 包。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点与测试）、`docs/user/guide/providers.md`、`packages/llm/llm-pi-ai/README.md`、升级指南、Web golden、根 README 与 `logs/`。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `baseten`（21 → 22 条，按字母序放在 `azure` 之后）；注释说明它是托管其他厂商开源权重模型的第三方推理云、22 个模型中 10 个在其他内置路由上保持相同 id、10 个换了 id 写法或显示名对应同一模型、`DeepSeek-V4.1-Flash-Fast` 与 `GLM-5.2-Fast` 两个没有其他内置路由，以及 `https://inference.baseten.co/v1` 的 OpenAI 兼容重新接入方式与 `chat_template_args` 思考格式对自定义路由仍可选 |
| `tests/catalog.spec.ts` | 「withholds removed catalog routes」用例的被移除路由清单增加 `baseten` |
| `tests/config.spec.ts` | 参数化的被移除路由用例增加 `baseten`（`it.each` 每条路由一条用例，strict 与 deferred 在同一条内各验一次；测试总数 +1） |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单增加 Baseten；自定义路由清单增加 `inference.baseten.co/v1`；新增覆盖句；Cerebras 的 `gpt-oss-120b` 承接清单与 `zai` 的 `glm-4.7` 说明去掉 `baseten`；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的被移除路由清单与 `baseten` 说明同步，并去掉 Cerebras、`zai` 段落里的 `baseten` 承接说法；重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述与变更清单加入 `baseten`（21 → 22 条）；迁移段的 OpenAI 兼容端点清单加入 `inference.baseten.co/v1`；独有模型段加入 `baseten` 的两个模型；正文 471 词（硬上限 500） |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "baseten"`（20 → 19 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 20 → 19 |
| 根 `README.md` 差异表 | 21 → 22 条移除路由、20 → 19 个路由、703 → 681 个模型、设置页移除 21 → 22 个 option，并补 `baseten` 覆盖句 |
| `logs/00009` 与相关旧日志 | 汇总日志刷新为 19 个路由 / 681 个模型 / 22 条被移除路由；00019、00022、00023、00025 加「后续变更」提示，指向本文件 |

## 2. 行为

- 可对话供应商 20 → **19**，目录模型 703 → **681**（只减 `baseten` 的 22 个模型）。
- `catalogProvider('baseten')` 返回 `undefined`，`catalogModels('baseten')` 为空；目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由。
- 存量 profile 点名 `baseten` 时 `resolveProfiles()` 在 strict 与 deferred 下都抛 `llm-pi-ai: provider "baseten" is removed from this build; delete the route or select another provider`，同一 `providers` 字典里的其他路由随之不可用，删除该条目即恢复。
- `provider-data.generated.ts` 仍保留 42 个 id 的元数据（`verify-provider-data` 要求与已安装目录一致），它不是可售路由；内置 id 现为 42 = 19 个提供 + `typesafe` + 22 个被移除。
- `thinkingFormat: 'baseten'` 与 `chatTemplateArgs` 仍是可配置的协议选项：手工声明的 OpenAI 兼容路由（包括按本文件重新接入的 Baseten 端点）仍可选择该思考格式，`docs/config-catalog.md` 相应说明不变。
- `packages/experimental/webworker-runtime` 的 `BUILTIN_PROVIDER_IDS` 仍列出 `baseten`：该列表描述 pi-ai 已安装目录，用于让桥接层拿到合法 id 并让每个请求落到明确的未实现错误，与本次移除无关（`zai` 亦同）。
- Baseten 托管的厂商本身仍由其他内置路由提供；失去内置路由的只有 `DeepSeek-V4.1-Flash-Fast` 与 `GLM-5.2-Fast` 两个 id。

## 3. 模型覆盖

移除 `baseten` 后，22 个模型中 10 个在其他内置路由上保持相同 id，10 个换了 id 写法或显示名对应同一模型，2 个没有任何其他内置路由（下表「其他内置路由」列同时给出路由 id 写法）。

| 模型 id | 推理 | 价格（输入/输出，$/M） | 上下文 | 最大输出 | 移除 `baseten` 后的其他内置路由 |
|---|---|---|---|---|---|
| `deepseek-ai/DeepSeek-V4-Flash-0731` | 是 | 0.13 / 0.26 | 1,048,576 | 384,000 | `together`（同 id）、`openrouter`（`deepseek/deepseek-v4-flash-0731`） |
| `deepseek-ai/DeepSeek-V4-Pro` | 是 | 1.74 / 3.48 | 1,048,576 | 262,144 | `deepseek`（`deepseek-v4-pro`）、`opencode-go`（同 id 写法）、`openrouter`（`deepseek/deepseek-v4-pro`） |
| `deepseek-ai/DeepSeek-V4-Pro-0813` | 是 | 1.32 / 3.96 | 1,048,576 | 262,144 | `together`（同 id）、`openrouter`（`deepseek/deepseek-v4-pro-0813`） |
| `deepseek-ai/DeepSeek-V4.1-Flash` | 是 | 0.3 / 1.2 | 1,048,576 | 32,768 | `together`（同 id）、`opencode-go`（`deepseek-v4.1-flash`）、`openrouter`（`deepseek/deepseek-v4.1-flash`） |
| `deepseek-ai/DeepSeek-V4.1-Flash-Fast` | 是 | 0.6 / 2.4 | 1,048,576 | 32,768 | **无** |
| `moonshotai/Kimi-K2.5` | 是 | 0.6 / 3 | 262,000 | 262,000 | `openrouter`（`moonshotai/kimi-k2.5`，仅大小写不同） |
| `moonshotai/Kimi-K2.6` | 是 | 0.95 / 4 | 262,000 | 262,000 | `openrouter`（`moonshotai/kimi-k2.6`，仅大小写不同） |
| `moonshotai/Kimi-K2.7-Code` | 是 | 0.95 / 4 | 262,000 | 262,000 | `github-copilot`、`opencode-go`（`kimi-k2.7-code`）、`openrouter`（`moonshotai/kimi-k2.7-code`） |
| `moonshotai/Kimi-K3` | 是 | 3 / 15 | 1,048,576 | 262,144 | `together`（同 id）、`fireworks`（`accounts/fireworks/models/kimi-k3`）、`github-copilot`、`opencode-go`（`kimi-k3`）、`openrouter`（`moonshotai/kimi-k3`） |
| `nvidia/Nemotron-120B-A12B` | 是 | 0.3 / 0.75 | 202,800 | 202,800 | `openrouter`（`nvidia/nemotron-3-super-120b-a12b`，同一 Nemotron Super 120B-A12B，该路由 262,144 上下文、0.08 / 0.45） |
| `nvidia/NVIDIA-Nemotron-3-Ultra-550B-A55B` | 是 | 0.6 / 2.4 | 202,800 | 202,800 | `openrouter`（`nvidia/nemotron-3-ultra-550b-a55b`，同价同上下文）、`together`（`nvidia/nemotron-3-ultra-550b-a55b`，512,300 上下文） |
| `openai/gpt-oss-120b` | 是 | 0.1 / 0.5 | 128,072 | 128,072 | `groq`、`openrouter`（同 id）、`together`（同 id）、`fireworks`（`accounts/fireworks/models/gpt-oss-120b`） |
| `thinkingmachines/inkling` | 是 | 1 / 4.05 | 1,048,576 | 32,768 | `openrouter`（同 id）、`together`（`thinkingmachines/Inkling`）、`fireworks`（`accounts/fireworks/models/inkling`） |
| `thinkingmachines/inkling-small` | 是 | 0.5 / 1.2 | 1,048,576 | 32,768 | `openrouter`（同 id） |
| `zai-org/GLM-4.7` | 是 | 0.6 / 2.2 | 200,000 | 200,000 | `openrouter`（`z-ai/glm-4.7`） |
| `zai-org/GLM-5` | 是 | 0.95 / 3.15 | 202,800 | 202,800 | `openrouter`（`z-ai/glm-5`） |
| `zai-org/GLM-5.1` | 是 | 1.3 / 4.3 | 202,800 | 202,800 | `openrouter`（`z-ai/glm-5.1`） |
| `zai-org/GLM-5.2` | 是 | 1.4 / 4.4 | 1,048,576 | 262,144 | `together`（同 id）、`opencode-go`（`glm-5.2`）、`openrouter`（`z-ai/glm-5.2`）、`mistral`（`glm-5-2`） |
| `zai-org/GLM-5.2-Fast` | 是 | 2.1 / 6.6 | 1,048,576 | 262,144 | **无** |
| `zai-org/GLM-5.3` | 是 | 1.4 / 4.4 | 1,048,576 | 262,144 | `together`（同 id）、`zai-coding-cn`（`glm-5.3`）、`opencode-go`（`glm-5.3`）、`openrouter`（`z-ai/glm-5.3`） |
| `zai-org/GLM-5.3-Fast` | 是 | 2.1 / 6.6 | 1,048,576 | 262,144 | `fireworks`（`accounts/fireworks/routers/glm-5p3-fast`，同价同上下文；另有 `glm-fast-latest` 指向同一档） |
| `zai-org/GLM-5.3-Flash` | 是 | 0.15 / 0.5 | 1,048,576 | 131,072 | `together`（同 id）、`zai-coding-cn`（`glm-5.3-flash`）、`opencode-go`（`glm-5.3-flash`）、`openrouter`（`z-ai/glm-5.3-flash`） |

## 4. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、486 个测试全部通过 |
| `npx vitest run packages/llm` | 78 个文件、1795 个测试全部通过 |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过（未改动该包，确认无误伤） |
| `pnpm run typecheck` | 5 passed，0 failed（17.93s） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件仍保留 42 个 id 的元数据） |
| `npx tsx scripts/verify-upgrade-guides.ts` | 通过，guide.md 471 词 |
| `npx tsx scripts/verify-translation-pairing.ts` | 3 组改动文档重新记录后一致（`test:docs` 的 translation pairing 门禁通过） |
| `pnpm run test:docs` | 收尾重跑 21/21 全部通过（中间一次为 20/21，唯一失败是 `verify-concrete-terms` 命中当时尚未提交的并行 `apps/web/native/dsh-im/`；并行提交 `refactor(im)` 落地后消失） |
| `pnpm run doc-sync` | 收尾重跑 43/43 全部通过（中间一次为 42/43，同一 `verify-concrete-terms` 失败，原因同上） |
| `pnpm run build` | 11 个门禁全部通过（27.69s，含 host/client 聚合检查与 Web build） |
| Web e2e（`DSH_SNAPSHOT=replay`，`vitest.web.config.ts`） | 只回放受影响的 `models-settings` 与 `onboarding-usable-provider` 两个文件：3 通过、19 失败；失败全部来自并行开发已提交的 `ui-boot-animation` 覆盖层 `<div class="dsh-boot-archive-black">…</div> … intercepts pointer events`，单独回放 `opens the add card over the dormant directory vocabulary` 确认失败点 |
| Web golden | 三个 golden 的 option 列表与 `catalogProviderIds()` 逐项相等（19 个、同顺序） |
| 目录自检 | 19 个供应商、681 个模型；`baseten` 不在目录中，`catalogModels('baseten')` 为空 |
| 文档自检 | 承载清单的 6 个中英文档（用户指南、包 README、升级指南各一对）与根 README 均含 `baseten`，无残留的 703 个模型 / 20 个路由计数 |

## 5. 复查记录

- 复查时发现初稿按「同一厂商前缀」比对，把 `nvidia/NVIDIA-Nemotron-3-Ultra-550B-A55B` 与 `zai-org/GLM-5.3-Fast` 误记为没有其他内置路由。逐路由按显示名、尺寸与价格核对后确认：前者与 `openrouter` 的 `nvidia/nemotron-3-ultra-550b-a55b` 同价同上下文，后者与 `fireworks` 的 `accounts/fireworks/routers/glm-5p3-fast` 同价同上下文，二者都仍可由其他内置路由提供。
- 因此独有模型由初稿的 5 个修正为 2 个（`DeepSeek-V4.1-Flash-Fast`、`GLM-5.2-Fast`），并同步改写 `catalog.ts`、用户指南、包 README、升级指南与根 README 的覆盖句；`nvidia/Nemotron-120B-A12B` 在 `openrouter` 上有同一 Nemotron Super 120B-A12B，但上下文与价格不同，表格中如实标注两条路由的差异。
- 交叉核对旧日志：`00019`、`00022`、`00023`、`00025` 曾把 `baseten` 列为承接路由，历史记录不重写，改为在各文件顶部加「后续变更」提示并指向本文件；`00009` 汇总日志刷新为现行数字。
- 未改动 `packages/experimental/webworker-runtime` 的 stub 列表与 `docs/config-catalog.md` 的 `baseten` 思考格式说明：前者描述 pi-ai 已安装目录，后者描述仍可配置的协议选项。
- 本轮进行中并行开发提交了 `refactor(im): drop Slack, Telegram, Discord, and WhatsApp`，清掉了触发 `verify-concrete-terms` 的 `apps/web/native/dsh-im/` 文件；本次改动基于该提交之上，未修改其内容。
- 首次回放 Web e2e 时误用了真实 API 配置（`pnpm run test:e2e -- <两个文件>`），该配置的 `include` 不接受文件参数，实际执行了整个 e2e 套件（452 个测试、22 个失败，含本机缺少 `bubblewrap` 导致 headless sandbox 用例失败）；随后改用 `DSH_SNAPSHOT=replay` + `vitest.web.config.ts` 只回放两个受影响文件。

## 6. 已知限制

- 运行中的 `127.0.0.1:3080` 进程需要重启才会反映目录变化。
- `DeepSeek-V4.1-Flash-Fast` 与 `GLM-5.2-Fast` 失去全部内置路由；如需继续使用，以 `openai-completions` 自定义模型 API 接入 `https://inference.baseten.co/v1`，并使用 Baseten 账号的 key。
- Baseten 的价格高于多数聚合路由上的同款模型（例如 `nvidia/Nemotron-120B-A12B` 为 0.3 / 0.75，而 `openrouter` 上同款为 0.08 / 0.45），迁移时应按实际路由核对成本。
- 不修改 `node_modules/@earendil-works/pi-ai`，避免上游同步冲突与 `verify-provider-data` 漂移。

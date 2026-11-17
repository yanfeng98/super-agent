# 00032 — 移除 together 供应商

- **日期**：2026-11-17
- **背景**：`together` 是 pi-ai 目录里的 Together AI 路由，默认端点 `https://api.together.ai/v1`，全部 17 个模型都用 `openai-completions` 协议，凭据为 `TOGETHER_API_KEY`，没有 OAuth 登录或订阅套餐入口。它自己不训练前沿模型，卖的是托管推理：目录里 17 个条目全是其他厂商的开源权重模型——DeepSeek 三个（`DeepSeek-V4-Flash-0731`、`DeepSeek-V4-Pro-0813`、`DeepSeek-V4.1-Flash`）、Qwen 四个（`Qwen2.5-7B-Instruct-Turbo`、`Qwen3.5-9B`、`Qwen3.6-Plus`、`Qwen3.7-Max`）、Z.AI GLM 三个（`GLM-5.2`、`GLM-5.3`、`GLM-5.3-Flash`）、MiniMax 两个（`MiniMax-M2.7`、`MiniMax-M3`），以及 `moonshotai/Kimi-K3`、`meta-llama/Llama-3.3-70B-Instruct-Turbo`、`nvidia/nemotron-3-ultra-550b-a55b`、`openai/gpt-oss-120b`、`thinkingmachines/Inkling`。
- **判断依据**：与 `mistral`、`ant-ling` 这类原厂入口不同，`together` 是转售开源权重的托管层，移除它不减少模型覆盖——17 个条目全部能在其他已提供路由上取到同一模型：其中 15 个对应 `openrouter` 上的同一模型（厂商前缀不同，如 `zai-org/*` → `z-ai/*`、`deepseek-ai/*` → `deepseek/*`、`MiniMaxAI/*` → `minimax/*`），另外两个 Together 专有的 Turbo 条目对应 `openrouter` 的非 Turbo 版本 `meta-llama/llama-3.3-70b-instruct` 与 `qwen/qwen-2.5-7b-instruct`。用户要求移除该供应商且不能破坏现有功能，沿用既有的 `WITHHELD_PROVIDERS` 机制：目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由，不修改 `node_modules` 里的 pi-ai 包。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点与测试）、`docs/user/guide/providers.md`、包 README、升级指南、Web golden 与 e2e 断言、根 README、`logs/`。

- **后续变更（00036）**：`openrouter` 已于 [00036](00036-移除openrouter供应商.md) 移除，本记录中以 `openrouter` 作为承接路由的说明随之失效；17 个条目中 7 个仍能对应已提供路由上的模型，10 个没有其他内置路由（含两个 Turbo id）。现行覆盖见 00036 第 2 节与 [00009](00009-模型供应商清理汇总.md) 顶部的「后续变更（00036）」。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `together`（25 → 26 条，字母序放在 `radius` 之后）；注释新增 `together` 段，说明它是托管其他厂商开源权重模型的推理云、17 / 15 / 2 的覆盖拆分与 `https://api.together.ai/v1` 的 OpenAI 兼容重接入方式；同时修正三处因路由被移除而过期的承接说明（`cerebras` 段的「`openrouter` 与 `together`」、`groq` 段的「两个还在 `together`」、`mistral` 段的「GLM 两条另见 `together` 与 `opencode-go`」），删掉 `zai` 段里早已移除的 `baseten` 括注，并把末尾「哪些路由使用 OpenAI 兼容对话补全」的枚举加入 Together |
| `tests/catalog.spec.ts` | 「withholds removed catalog routes」用例的被移除路由清单增加 `together` |
| `tests/config.spec.ts` | 参数化的被移除路由用例增加 `together`（strict 与 deferred 在同一条内各验一次） |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单增加 Together（托管其他厂商开源权重模型的第三方推理云）；自定义端点清单加入 `api.together.ai/v1`；补一句 17 / 15 / 2 的覆盖拆分；同时删去 `cerebras`、`groq`、`mistral` 说明里把 `together` 当作可用承接路由的旧句；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的被移除路由清单与 Together 说明同步，同样清理旧承接句；重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述与变更清单加入 `together`（25 → 26 条）；迁移段的 OpenAI 兼容端点清单加入 `api.together.ai/v1`；覆盖句改为「17 个条目全部能在其他路由上取到」并说明两个 Turbo 条目的对应关系；为压回 500 词上限，精简了 `fireworks` 句、Xiaomi 括注与协议句；正文 498 词；重录 i18n 哈希 |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "together"`（16 → 15 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 16 → 15 |
| 根 `README.md` 差异表 | 25 → 26 个移除路由、16 → 15 个路由、612 → 595 个模型、设置页 25 → 26 个 option，并补 Together 说明；同时清理 `groq`、`mistral` 两处把 `together` 当承接路由的写法 |
| `logs/00009` | 汇总刷新为 15 个路由 / 595 个模型 / 26 条被移除路由，加入本日志条目，并修正已知限制里把 `together`、`groq`、`fireworks` 当作现行承接路由的旧句 |
| `logs/00019`、`00022`、`00030`、`00031` | 「后续变更」提示补记 `together`（及同样已移除的 `groq`、`fireworks`）不再是承接路由，现行覆盖指向 `logs/00009` |

## 2. 行为

- 可对话供应商 16 → **15**，目录模型 612 → **595**（只减 `together` 的 17 个模型）。
- `catalogProvider('together')` 返回 `undefined`，`catalogModels('together')` 为空；目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由。
- 存量 profile 点名 `together` 时 `resolveProfiles()` 在 strict 与 deferred 下都抛 `llm-pi-ai: provider "together" is removed from this build; delete the route or select another provider`，同一 `providers` 字典里的其他路由随之不可用，删除该条目即恢复。
- 内置 id 现为 42 = 15 个提供 + `typesafe` + 26 个被移除；`provider-data.generated.ts` 仍保留 42 个 id 的元数据（`verify-provider-data` 要求与已安装目录一致），它不是可售路由。
- `packages/experimental/webworker-runtime` 的 `BUILTIN_PROVIDER_IDS` 仍列出 `together`：该列表描述 pi-ai 已安装目录，用于让桥接层拿到合法 id 并让每个请求落到明确的未实现错误，与本次移除无关。
- 模型覆盖不受影响：17 个条目分别在 `openrouter` 上可取到同一模型（两个 Turbo 条目为非 Turbo 版本）；上下文、输出上限与价格按路由不同，迁移时需按模型核对（见第 5 节）。
- 未触及 agent loop、模型可见工具、Session 事件或持久化格式；`deepseek-official` 与 `deepseek-account` 原生路由不受影响。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm exec vitest run packages/llm/llm-pi-ai` | 22 个文件、490 个测试全部通过（比上一轮 +1，参数化用例新增 `together` 一条） |
| `pnpm exec vitest run packages/llm`（全 llm 包） | 78 个文件、1799 个测试全部通过 |
| `pnpm exec vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过（设置页读取同一份目录） |
| `pnpm run typecheck` | 5 passed，0 failed（17.06s） |
| `pnpm run lint` | 0 warning、0 error（90 条规则、7303 个文件，69.7s） |
| `pnpm run build` | 11 个门禁全部通过（25.47s，含 host/client 聚合检查与 Web build） |
| `pnpm run test:docs` | 21 个门禁全部通过（16.80s，含升级指南词数与中英配对） |
| `pnpm run doc-sync` | 43 个门禁全部通过（55.31s） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件仍保留 42 个 id 的元数据） |
| `pnpm exec tsx scripts/gen-config-catalog.ts --check` | 3 个生成产物均为最新 |
| `pnpm exec tsx scripts/verify-upgrade-guides.ts` | 通过，guide.md 498 词（上限 500） |
| 目录核对（构建产物 probe） | 15 个提供路由、595 个模型；`catalogProvider('together')` 为 `undefined`，`catalogModels('together')` 为 0 |
| Web golden 单项回放 | 临时 overlay 关闭 `ui-boot-animation` 后回放 `models-settings` 的 `opens the add card over the dormant directory vocabulary`：provider `option` 列表与 golden 逐项一致（15 项、不含 `together`）；唯一差异是金标未含的既有 `IM机器人` 导航按钮，与本改动无关。临时 overlay 与测试改动已还原 |
| 文档清单核对 | 7 份中英文档（用户指南、包 README、升级指南各一对与根 README）均枚举全部 26 个被移除 id，无缺失、无多余 |
| `git diff --check` | 通过 |
| 复查后重跑 | `vitest packages/llm/llm-pi-ai` 22 个文件 / 490 个测试通过（12.71s）；`test:docs` 21/21 通过（16.08s）；构建产物 probe 仍为 15 个路由 / 595 个模型；`git diff --check` 通过 |

## 4. 复查记录

- 覆盖口径：脚本读取 pi-ai 内置目录的 `getBuiltinModels('together')`（走上游目录，不受本构建屏蔽影响），按「同一模型」比对 17 个条目与 `openrouter` 的条目：15 个按模型名对应同一模型，`Llama-3.3-70B-Instruct-Turbo` 与 `Qwen2.5-7B-Instruct-Turbo` 需改按非 Turbo id 比对；因此文档写「17 个条目全部有替代」而非「17 个同 id」。
- 复查 7 处，均已修正：
  1. `catalog.ts` 的 `cerebras`、`groq`、`mistral` 三段注释仍把 `together` 当作现行承接路由（`cerebras` 段「`openrouter` 与 `together`」、`groq` 段「两个还在 `together`」、`mistral` 段「GLM 两条另见 `together` 与 `opencode-go`」）。
  2. 用户指南、包 README、升级指南与根 README 同样残留同类旧承接句，已逐一改写。
  3. `logs/00009` 已知限制里 `nvidia`（`groq`、`together`）、`radius`（`fireworks`、`together`）、`minimax`（`together`、`fireworks`）、`cerebras`（`together`）四条与 `zai` 段的 `baseten` 括注同属过期承接说明，已按本轮状态重写。
  4. `logs/00019`、`00022`、`00030`、`00031` 补记「后续变更」，说明 `together`（与同样已移除的 `groq`、`fireworks`）不再是承接路由，现行覆盖指向 `logs/00009`。
  5. 覆盖口径由「同 id」改为「同一模型」：15 个条目与 `openrouter` 的厂商前缀不同，两个 Turbo 条目需换 `openrouter` 的非 Turbo id。
  6. 升级指南加入 Together 后一度为 517 词，按「精简 `fireworks` 句、压缩 Together 句、删 Xiaomi 括注、改述协议句」压回 498 词，中英结构保持一致。
  7. `logs/00030`、`00031` 插入「后续变更」时误删「范围」要点，已补回。
- 未改动 `provider-data.generated.ts` 与 `packages/experimental/webworker-runtime` 的 stub 列表：两者描述已安装目录而非可售路由，`verify-provider-data` 要求保留全部 42 个 id。
- Web e2e 未整套重跑：本机既有 `dsh-boot-archive-black` 覆盖层会拦截设置入口点击，按前几轮的做法用临时 overlay 单独回放受影响的用例，回放后已还原。
- 本次复查修正（第 4 节的 7 处）已 amend 进同一提交（未推送），未产生额外提交。

## 5. 已知限制与恢复

- 本次只屏蔽可售路由，不删除 pi-ai 包内 Together 的实现与目录元数据；手工声明同名路由仍会被 `resolveProfiles()` 拒绝。
- 恢复时删除 `WITHHELD_PROVIDERS` 中的 `together`，把可售路由与模型计数改回 16 / 612，并同步撤销第 1 节列出的 golden、断言与文档改动，重新记录 i18n 哈希并复核升级指南词数。
- 迁移到 `openrouter` 时不能只改 Provider ID：同一模型在不同路由上的上下文上限、输出上限与价格可以不同（GLM-5.3 在 `together` 为 $1.4/$4.4、输出上限 262,144，在 `openrouter` 为 $0.11/$6、943,718；Nemotron 3 Ultra 550B 在 `together` 为 512,300，在 `openrouter` 为 202,800；`gpt-oss-120b` 输入输出价两端都是 $0.15/$0.6，但 `openrouter` 的缓存读取另计 $0.075，`together` 的目录记为 0），模型 id 也要按 `openrouter` 的厂商前缀改写。
- 需要原端点时以 `openai-completions` 自定义模型 API 接入 `https://api.together.ai/v1`，并使用不同的 Provider ID（`together` 本身会被拒绝）。
- 运行中的 Web 宿主仍是改动前启动的进程，重启 `pnpm dsh web` 后设置页才会显示 15 项且不含 `together`。

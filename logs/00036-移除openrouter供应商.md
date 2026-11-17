# 00036 — 移除 openrouter 供应商

- **日期**：2026-11-17
- **背景**：`openrouter` 是 pi-ai 目录里的聚合路由（名称 OpenRouter，端点 `https://openrouter.ai/api/v1`，模型协议以 `openai-completions` 为主，凭据为 `apiKey` 与「Sign in with OpenRouter」OAuth 两种），内置 396 个模型 id（除裸 `auto` 外都带厂商前缀；381 个以 `openai-completions` 提供于 `https://openrouter.ai/api/v1`，15 个以 `anthropic-messages` 提供于 `https://openrouter.ai/api`），是全部 14 条提供路由里模型最多的一条。它不是模型厂商：一个 key 把 OpenAI、Anthropic、Google、Meta、Mistral、DeepSeek、Qwen、xAI 等几十家的模型转售在一条端点后，计费按量预充值，请求要多经一层第三方。
- **判断依据**：用户要求移除该供应商且不能破坏现有功能。沿用既有的 `WITHHELD_PROVIDERS` 机制：目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由，不修改 `node_modules` 里的 pi-ai 包。移除代价按目录逐条核算：396 个 id 中 95 个在去掉厂商前缀并忽略标点后与另一条已提供路由的模型 id 相同、2 个按显示名对应，其余 299 个没有其他内置路由——18 个 `~…-latest` 浮动别名、14 个 `:free` 变体、5 个 `openrouter/*` 路由 id、1 个 `unbiased/*` 别名、裸 `auto` 路由，以及 36 个厂商前缀下的 260 个条目（其中 77 个是 `:batch` 变体）。这是本系列里第一条「移除面同时包含模型覆盖」的聚合路由：此前移除 `vercel-ai-gateway`、`huggingface`、`together`、`groq` 时，承接方正是 `openrouter`，因此本轮同时重算了全部既有路由的覆盖口径。其端点仍可作 OpenAI 兼容自定义模型 API 接回。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点与测试）、用户指南、包 README、升级指南、根 README、Web golden 与 e2e 断言、`logs/`（含全部旧日志的承接说明校正）。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `openrouter`（27 → 28 条，字母序放在 `opencode` 之后）；注释新增 `openrouter` 段（396 个 id 与两种协议端点、95 + 2 个仍可在已提供路由上取到、299 个没有其他内置路由及其构成、接回时按协议各加一条自定义模型 API），并把 `vercel-ai-gateway`、`huggingface`、`cerebras`、`groq`、`together`、`opencode`、`nvidia`、`zai`、`mistral`、`baseten` 各段中「在 `openrouter` 上可取」的承接说明改成重算后的数字；协议小结把 OpenRouter 并入 OpenAI 兼容一组 |
| `tests/catalog.spec.ts`、`tests/config.spec.ts` | 被移除路由清单增加 `openrouter` |
| `tests/loader-composition.spec.ts` | 原本借 `openrouter` 断言 `defaultBaseURL`，改为断言 `openrouter` 不在目录中（`google` 已覆盖元数据断言） |
| `tests/config.spec.ts` | `resolveProfiles` 的异常传播用例把路由 key 由被移除的 `openrouter` 换成已提供的 `anthropic` |
| `tests/dynamic-config.spec.ts` | 请求级动态 profile 用例的路由由 `openrouter` 换成 `github-copilot`（同属协议混合、带 OAuth 与固定端点，去掉模型仍需显式 `api`，断言随之改为 `https://api.individual.githubcopilot.com` 与 `COPILOT_GITHUB_TOKEN`） |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单加入 OpenRouter；端点清单加入 `openrouter.ai/api/v1`；覆盖段按已提供路由重算（`ant-ling`、`cerebras`、`groq`、`mistral`、`baseten`、`together`、`opencode`、`huggingface`、`vercel-ai-gateway`、`zai` 逐条更新，并新增 OpenRouter 一段）；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的同一条要点同步重算，并新增 OpenRouter 段；重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述、变更清单与迁移端点加入 `openrouter`；覆盖段改为按已提供路由统计的现况并压回 500 词上限（正文 491 词）；重录 i18n 哈希 |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "openrouter"`（14 → 13 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 14 → 13 |
| `apps/web/tests/models-settings-recovery.e2e.ts` 与 `expected/models-settings-recovery/stored-error.expected.md` | 存量配置漂移用例的两条路由由 `openrouter`、`zai` 换成 `github-copilot`、`kimi-coding`（两条原路由均已被移除，插件会整体加载失败）；错误文案、按钮标签与 golden 同步 |
| 根 `README.md` 差异表 | 27 → 28 个移除路由、14 → 13 个路由、581 → 185 个模型、设置页 26 → 27 → 28 个 option，并把整段覆盖说明改为按已提供路由重算的数字 |
| `logs/00009` | 汇总刷新为 13 个路由 / 185 个模型 / 28 条被移除路由；表内新增本行，并加入「后续变更（00036）」重算表 |
| `logs/00015`、`00016`、`00018`、`00019`、`00020`、`00021`、`00022`、`00023`、`00024`、`00025`、`00026`、`00029`、`00030`、`00031`、`00032` | 各加一条「后续变更（00036）」，说明其以 `openrouter` 作为承接路由的结论失效，并给出该路由重算后的覆盖数 |

## 2. 行为

- pi-ai 可对话路由 14 → **13**，目录模型 581 → **185**（只减 `openrouter` 的 396 个模型）；原生 DeepSeek 两条路由不变。
- `catalogProvider('openrouter')` 返回 `undefined`，`catalogModels('openrouter')` 为空；目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由，设置页「添加模型提供商」由 14 项变为 13 项。
- 存量 profile 点名 `openrouter` 时 `resolveProfiles()` 在 strict 与 deferred 下都抛 `llm-pi-ai: provider "openrouter" is removed from this build; delete the route or select another provider`，同一 `providers` 字典里的其他路由随之不可用，删除该条目即恢复。
- 内置 id 现为 42 = 13 个提供 + `typesafe` + 28 个被移除；`provider-data.generated.ts` 仍保留 42 个 id 的元数据（`verify-provider-data` 要求与已安装目录一致），它不是可售路由。
- 登录面：`openrouter` 的 OAuth 入口随路由消失，目录中声明 OAuth 的已提供路由由 8 条变为 7 条（`anthropic`、`github-copilot`、`kimi-coding`、`meta`、`openai`、`openai-codex`、`xai`），其中 `openai` 的登录被本构建关闭，设置页实际出现登录按钮的仍是 6 条。
- 模型覆盖：按模型 id（去掉厂商前缀、忽略标点）或显示名，`openrouter` 的 396 个条目中有 97 个仍能在已提供路由上找到同一模型，其余 **299 个失去内置入口**；这是本系列中第一轮同时削减模型覆盖的移除，也是范围最大的一轮（第二大为 `vercel-ai-gateway` 的 214 个 id）。
- 连带更正：此前移除 `groq`、`cerebras`、`together` 时「只移除托管面」的结论不再成立（三者现分别为 0/7、0/2、7/17 有内置入口）；`mistral` 由 29/40 变为 3/40，`baseten` 由 20/22 变为 7/22，`nvidia` 由 9/20 变为 3/20，`huggingface` 由 78/78 变为 15/78，`vercel-ai-gateway` 的承接由 118 个 id 变为 87 个，`zai` 的 `glm-4.7` 与 `glm-5-turbo` 由「另见 `openrouter`」变为没有其他内置路由。
- 未触及 agent loop、模型可见工具、Session 事件或持久化格式；`deepseek-official` 与 `deepseek-account` 原生路由不受影响。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm exec vitest run packages/llm/llm-pi-ai` | 22 个文件、492 个测试全部通过（参数化用例新增 `openrouter` 一条，动态 profile 用例换成 `github-copilot` 后仍通过） |
| `pnpm exec vitest run packages/llm packages/client/ui-settings-models` | 93 个文件、2306 个测试全部通过 |
| `pnpm run typecheck` | 5 passed，0 failed（15.17s） |
| `pnpm run lint` | 0 warning、0 error（90 条规则、7308 个文件，76.2s） |
| `pnpm run build` | 11 个门禁全部通过（29.80s，含 host/client 聚合检查与 Web build；381 个 client artifact） |
| `pnpm run test:docs` | 21 个门禁全部通过（23.44s） |
| `pnpm run doc-sync` | 43 个门禁全部通过（64.23s） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件仍保留 42 个 id） |
| `pnpm exec tsx scripts/gen-config-catalog.ts --check` | 3 个生成产物均为最新 |
| `pnpm exec tsx scripts/verify-upgrade-guides.ts` | 通过，`guide.md` 491 词（上限 500） |
| `pnpm run verify-translation-pairing` | 1301 对全部一致 |
| 目录核对（源码 probe） | 13 个提供路由、185 个模型；`catalogProvider('openrouter')` 为 `undefined`；枚举出的 id 顺序与三份 golden 的 13 个 option 逐项相等 |
| Web golden 单项回放 | 临时 overlay 关闭 `ui-boot-animation` 后回放 `models-settings` 的 `opens the add card over the dormant directory vocabulary` 与 `models-settings-recovery`：前者与 golden 的唯一差异是金标未含的既有 `IM机器人` 导航按钮（并行 IM 改动，与本轮无关），provider option 列表逐项一致；后者第二个用例（改名后的 `github-copilot`/`kimi-coding` 修复流程）**完整通过**，第一个用例的唯一差异同样是该 `IM机器人` 按钮。回放后已还原 e2e 补丁并删除临时 overlay |
| `git diff --check` | 通过 |

## 4. 复查记录

本轮复查覆盖 6 处，均已随改动修正：

1. `loader-composition.spec.ts` 原本以 `openrouter` 校验目录里的 `defaultBaseURL`，该路由被移除后改为断言它不在目录中；`baseURLEnv` 与 `defaultBaseURL` 的目录级覆盖仍由 `google` 的断言承担。
2. `config.spec.ts` 的「传播意外的目录失败」用例点名 `openrouter`，被移除后会在比较名之前抛「路由已移除」，改用已提供的 `anthropic` 保持原意。
3. `dynamic-config.spec.ts` 需要一条「协议混合、带 OAuth 且端点固定」的已提供路由：先试 `kimi-coding`，因其模型协议统一而被适配器补上默认 `api`，漂移错误不再出现；最终改用 `github-copilot`（三种协议并存），断言改为其端点与 `COPILOT_GITHUB_TOKEN`。
4. `models-settings-recovery.e2e.ts` 的存量配置同时点名 `openrouter` 与 `zai`（后者在 00025 轮已移除），两条都会让插件整体加载失败，用例原意（一条失败路由、一条健康路由、一条自定义路由并列）已无法成立；改名为 `github-copilot`、`kimi-coding` 后重放通过。
5. 覆盖口径统一为「同 id（去掉厂商前缀、忽略标点）或同显示名」，并对 14 条既定路由逐条重算；`mistral` 的 3 个可承接条目是它托管的 Z.AI GLM 条目（`zai-glm-5-2`、`zai-glm-5-3`、`glm-5-2`），不是 `mistralai/*`，文档已按此改写。
6. 升级指南加入 OpenRouter 后一度超出 500 词，按「压缩 `google-vertex` 字段差异、去掉已在前文出现的端点、合并计数句」压到 491 词；中英结构与 28 条 id 清单保持一致。

## 5. 已知限制与恢复

- 本次只屏蔽可售路由，不删除 pi-ai 包内 OpenRouter 的实现与目录元数据；手工声明同名路由仍会被 `resolveProfiles()` 拒绝。要接回原服务，需要按协议各加一条自定义模型 API：381 个 `openai-completions` 条目用 `https://openrouter.ai/api/v1`，15 个 `anthropic-messages` 条目用 `https://openrouter.ai/api`，均使用不同的 Provider ID。
- 恢复时删除 `WITHHELD_PROVIDERS` 中的 `openrouter`，把可售路由与模型计数改回 14 / 581，并同步撤销第 1 节列出的 golden、断言与文档改动；`loader-composition.spec.ts` 的 `defaultBaseURL` 断言可改回 `openrouter`，`dynamic-config.spec.ts` 与恢复用例可分别改回 `openrouter` 与其 golden，重新记录 i18n 哈希并复核升级指南词数。
- `openrouter` 的 299 个无其他内置路由条目里，263 个是厂商模型（36 个厂商前缀，OpenAI 62、Qwen 38、Mistral 25 居前），其余是 18 个 `~…-latest` 浮动别名、14 个 `:free` 免费档、5 个 `openrouter/*` 自有路由 id、1 个 `unbiased/*` 别名与裸 `auto`；`~…-latest` 与 `:free` 在自定义路由上需要逐条声明模型，不能沿用别名语义。
- 计费与隐私随路由消失：原先经过 OpenRouter 转售的模型改为直连各自厂商，或继续走自定义路由；同一模型在两条路由上的上下文、输出上限与价格并不逐项相同，迁移前需按模型核对。
- 并行开发留下的 Web 覆盖层仍会拦截设置入口点击：本机整套 Web e2e 在未处理该覆盖层时失败（与本轮无关），因此 golden 以临时 overlay 单独回放确认，回放后已还原。
- 运行中的 `127.0.0.1:3080` 宿主仍是改动前启动的进程，重启 `pnpm dsh web` 后设置页才会显示 13 项且不含 `openrouter`。

## 6. 第二轮复查（用户要求复核）

用户要求复核本轮修复。逐条重算目录并对照 7 份文档、4 处测试与 2 份日志后，修正 3 处不准确：

1. **「396 个带厂商前缀」不成立**：`openrouter` 里有一个裸 `auto` 路由 id，其余 395 个才带厂商前缀。7 处表述（`catalog.ts` 注释、包 README 中英、用户指南中英、升级指南中英、根 README）与两份日志统一改为「除裸 `auto` 外都带厂商前缀」。
2. **端点只写了 OpenAI 兼容一面**：目录里 396 个条目实际分属两种协议——381 个 `openai-completions` 于 `https://openrouter.ai/api/v1`，15 个 `anthropic-messages` 于 `https://openrouter.ai/api`。原文只给前一个端点，会让只接一条自定义路由的读者以为 15 个 Claude 条目也能照此接回。已改为两个端点并列，并说明按协议各加一条自定义路由；`openai-completions` 与 `anthropic-messages` 都在自定义模型 API 的可选协议内，且 pi-ai 的 `getAnthropicCompat` 仍按 `baseUrl.includes('openrouter.ai')` 打开 OpenRouter 的会话亲和默认值，因此自定义路由不会丢这层兼容。
3. **登录面计数错误**：目录中声明 OAuth 的已提供路由由 8 条（含 `openrouter`）变为 7 条（含被本构建关闭登录的 `openai`），设置页实际出现按钮的是 6 条；原日志写成「由 7 条变为 6 条」。已改正。

复核同时用脚本重跑全部覆盖数字（95 / 2 / 299、260 个厂商条目、77 个 `:batch`、36 个厂商前缀、各路由覆盖数与 `baseten` 7/22、`fireworks` 6/22、`cloudflare-workers-ai` 5/18、`amazon-bedrock` 26/193 等）与 28 条 id 清单，均与文档一致；`gpt-5.1-codex`、`gpt-5.2-codex` 确认不在 `openai` 路由上，`opencode` 的 26 个未覆盖条目表述仍成立。

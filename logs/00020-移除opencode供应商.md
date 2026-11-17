# 00020 — 移除 opencode 供应商

- **日期**：2026-11-15
- **背景**：用户要求移除 `opencode`（OpenCode Zen / Console）、保留 `opencode-go`（OpenCode Go），且不能破坏项目现有功能。两者是同一厂商的两条接入层：`opencode` 是按量付费的 Console 网关（预充值 credits、按 token 扣费），`opencode-go` 是月订阅套餐（Go $10/月、Go Plus $40/月、Go Teams $20/人/月，额度按月美元上限折算）。pi-ai 对两者都使用同一把 `OPENCODE_API_KEY`、同一套会话头 `x-opencode-session`，只是端点分别落在 `https://opencode.ai/inference` 与 `https://opencode.ai/inference/go`。本机 Web profile 只配置了 `opencode-go`，`.credentials.yaml` 也没有点名 `opencode`。
- **范围**：`packages/llm/llm-pi-ai` 的目录、测试与双语 README；Web 设置页与 onboarding golden；`docs/user/guide/providers.md` 与 `.zh.md`；既有升级指南；`logs/`。

- **后续变更（00036）**：`openrouter` 已于 [00036](00036-移除openrouter供应商.md) 移除，本记录中以 `openrouter` 作为承接路由的说明随之失效；84 个 id 中 58 个仍能对应已提供路由上的模型，26 个没有其他内置路由。现行覆盖见 00036 第 2 节与 [00009](00009-模型供应商清理汇总.md) 顶部的「后续变更（00036）」。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `opencode`，共 16 条（按字母序插在 `moonshotai-cn` 之后）。注释新增一段：`opencode` 是 OpenCode 的按量付费 Console 网关，本构建保留同一厂商的订阅路由 `opencode-go`，两者解析同一 `OPENCODE_API_KEY`，且 Zen 的多数闭源模型在其他路由上仍可取到，因此移除它只是去掉一条计费入口；其付费模型仍可用 `https://opencode.ai/inference/v1` 的 OpenAI 兼容自定义模型 API 接入，免费档模型只允许 OpenCode 客户端使用，System One 条目是分类模型而不是对话路由。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | 三处以 `opencode` 为“混合协议、无 provider 级端点”样本的用例改用 `opencode-go`（模型级端点、`openai-completions` 与 `openai-responses` 并存，语义一致）；withheld 回归测试扩到 16 条路由。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 存量 profile 拒绝测试参数化加入 `opencode`（16 条），strict 与 deferred 均覆盖。 |
| `packages/llm/llm-pi-ai/tests/native-auth.spec.ts` | “新装路由不应猜出元数据”的样本由 `catalogProvider('opencode')` 改为 `catalogProvider('opencode-go')`；`PROVIDER_DATA.opencode` 断言保留（生成数据仍须覆盖已安装目录）。 |
| `packages/llm/llm-pi-ai/tests/loader-composition.spec.ts` | 真实组合测试的“无 provider 级端点”断言改为 `opencode-go`。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 更新为 16 条路由不提供，并补一段说明 Zen 与 Go 的关系、共用凭据、付费模型的自定义接入与免费档限制；重新记录双语哈希。 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由列表加入 OpenCode Zen 并注明它是按量付费 Console 网关；替代路径补 `opencode.ai/inference/v1`（付费模型可用、免费档仅限 OpenCode 客户端）；补一句目录覆盖情况，并在“仍然提供”的收尾句里注明 `opencode-go` 是按月订阅、解析同一 `OPENCODE_API_KEY`；重新记录双语哈希。 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.md` / `.zh.md` / `.i18n.yaml` | front matter、变更段、协议段与端点列表加入 `opencode`，并说明 `opencode-go` 仍提供。为守住 500 词上限，把原来的“Cloudflare/Cerebras/Hugging Face/Moonshot/Qwen/Xiaomi 使用 OpenAI 兼容协议”枚举改成“你自有的端点仍可通过自定义模型 API 接入”加具体端点清单，压缩 Qwen token 套餐措辞，删除与首句重复的 Vercel 单句；最终 460 词（8% 余量）。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除 `option "opencode"`（可选项 26 → 25）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |
| `apps/web/tests/models-settings.e2e.ts` | 选择器数量下限由 `>= 26` 改为 `>= 25`；精确列表由 golden 负责。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现 `opencode`，列表为 25 项 |
| 目录、模型解析、发现、登录 | 均不包含 `opencode` |
| `catalogProvider('opencode')` / `catalogModels('opencode')` | 返回 `undefined` / 空 |
| 存量配置点名 `opencode` | 明确报 `provider "opencode" is removed from this build; delete the route or select another provider` |
| `opencode-go` | 不受影响，29 个模型照常提供 |
| 其余 24 条供应商 | 不受影响 |
| 两条 DeepSeek 原生路由 | 不受影响 |

**模型的连带影响**：目录模型数从 849 降为 765，84 个条目退出选择器。按 id 与模型名（忽略厂商前缀、忽略大小写）比对剩余 25 条路由：57 个 id 原样出现在其他路由上（其中 17 个同时也在 `opencode-go` 上），14 个在不同厂商前缀下对应同一模型（如 `glm-5` 对应 `z-ai/glm-5`、`kimi-k2.6` 对应 `moonshotai/kimi-k2.6`），其余 13 个没有其他内置路由——其中 8 个是免费/实验档（`big-pickle`、`deepseek-v4.1-flash-free`、`mimo-v2.6-flash-free`、`nemotron-3-ultra-free`、`nemotron-3.5-lightning-free`、`nemotron-3.5-super-vl-free`、`space-bunny-free`、`muse-spark-1.3-contributor-free`），5 个是付费模型（`claude-sonnet-4`、`qwen3.5-plus`、`gemini-3-flash`、`gemini-3.1-pro`、`gpt-5-codex`）。免费档模型按官方策略只允许 OpenCode 客户端使用，本就不在可用范围内；要继续走 Console 端点（付费模型），以 `openai-completions` 自定义模型 API 接入 `https://opencode.ai/inference/v1`。

**配置级影响**：拒绝发生在整份 `providers` 字典解析时。若同一 profile 同时配置了 `opencode` 与其他 pi-ai 路由，`llm-pi-ai` 插件整体加载失败，其他 pi-ai 路由要等该条目删除后才恢复；本机 profile 只写了 `opencode-go`，不受影响。

**测试与 fixture 影响**：全仓扫描确认没有单元测试、录制会话或 headless fixture 以 `opencode` 作为场景路由；受影响的只有把它当作“混合协议样本”的 5 处断言（`catalog.spec` 3 个用例、`native-auth` 与 `loader-composition` 各 1 处），以及枚举供应商列表的 3 个 golden 与 1 处数量下限。`provider-data.generated.ts` 必须继续覆盖已安装目录（`verify-provider-data` 校验），webworker 的 `BUILTIN_PROVIDER_IDS` 是“必须使用有效已安装 id”的预览子集桩，两者都不修改。`packages/experimental/auto-review` README 提到的 `x-opencode-session` 是 OpenCode 路由共有的会话头，`opencode-go` 仍在使用，保持原样。`typesafe` 因只含分类模型本就被过滤，与本次移除无关；Zen 自带的 4 个 System One 分类模型（`jev-1.13`、`jev-1.13-free`、`gpt-6-luna-decision`、`gemini-decision-1`）同样不在 84 个对话模型内。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm`（全 llm 包） | 78 个文件、1789 个测试全部通过（含两条 DeepSeek 原生路由） |
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、480 个测试全部通过（新增 1 个参数化用例） |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过 |
| `pnpm run verify-upgrade-guides` | 通过（升级指南结构与词数上限） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件保持 42 个 id） |
| 文档门禁（`verify-translation-pairing` / `verify-md-wrap` / `verify-md-links`） | 1300 对、2652 个文件、2614 个文件全部通过 |
| `pnpm run test:docs` | 21 个门禁全部通过 |
| `pnpm run lint`（全仓） | 0 warning、0 error（90 条规则，7266 个文件） |
| `pnpm run build` | 10 个构建门禁全部通过（83.15s，含 host/client 聚合检查与 Web build）；中间态曾因并行进程的 `ui-boot-animation` 包报 `TS18010`/`TS2878`，该进程修复后转绿 |
| `pnpm run doc-sync` | 42/43 通过；唯一失败是 `Cordis inspect catalog`（`api-catalog.ts` 过期），来自同一并行进程正在改的 `packages/extensions/cordis-client-runner/src/client/slot-catalog.ts` 等文件，与本次移除无关 |
| `pnpm run test:snapshot -t pi-ai-conversation-updates` | 1 个录制会话回放通过 |
| `npx vitest run --config vitest.expected.config.ts -t "pi-ai"` | 2 个 headless 场景通过，内联快照未变 |
| Web e2e（`replay`） | 只跑受影响的两个文件：20/22 通过，3 个 golden 比对通过、未发生重写；2 个失败是 WebKit 的 `keeps mobile model actions…` 与 `keeps keyboard reorder…`，缺宿主机共享库（`libavif.so.13`、`libharfbuzz-icu.so.0` 等），与本次改动无关 |
| golden 与目录一致性 | 三个 golden 各 25 项，与 `catalogProviderIds()` 同集合、字母序逐项相等 |
| 目录自检（源码导入） | 25 个供应商、765 个模型；`opencode` 不存在，`catalogProvider()` 为 `undefined`、`catalogModels()` 为空；`opencode-go` 29 个模型照常 |
| 上游定义核对 | pi-ai 1.3.0 中 `opencode` 的 84 个对话模型覆盖四条协议（`anthropic-messages` 18、`openai-completions` 28、`openai-responses` 30、`google-generative-ai` 8），模型自带 `https://opencode.ai/inference` 或 `…/v1`，`auth` 只有 `apiKey`（`OPENCODE_API_KEY`） |
| 文档一致性自检 | 6 个当前文档（用户指南 2、包 README 2、升级指南 2）均完整枚举 16 条路由，无过期计数 |
| 升级指南词数 | 460 词（上限 500，保留 8% 余量） |

## 4. 设计取舍

- **只移除目录暴露，不删上游数据**：不修改 `node_modules/@earendil-works/pi-ai`；`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 仍是仅有的两个控制点，二者都能按原样恢复。
- **保留 Go 而不是保留 Zen**：两条路由共用同一凭据与会话头，Go 是订阅套餐（固定月费、按模型额度），Zen 是按量付费（余额扣费）。保留订阅侧可以让同一份配置在额度内可预测，同时不改变已有的 `OPENCODE_API_KEY` 与模型 id 写法；需要 Zen 独有的闭源模型时，多数能在 `openrouter` 或其他内置路由上取到（54 个闭源模型里 37 个在 `openrouter` 上），其余可用自定义模型 API 接入。
- **升级指南并入既有批次文档**：版本号仍为 0.2.2-rc.3，按 `dsh-create-upgrade-guide` 的要求更新覆盖同一 surfaces 的既有指南，而不是为同一版本再开条目；代价是该文档已接近 500 词上限，本次通过压缩既有的协议枚举与 Qwen 措辞腾出空间。
- **测试样本改用 `opencode-go`**：三处用例需要“混合协议且端点写在模型上”的路由，Go 同时满足（`openai-completions` 18 个、`openai-responses` 5 个，provider 级无 baseURL），语义未变。
- **只回放受影响的 Web 用例**：全量 Web lane 串行执行代价过高；`models-settings` 与 `onboarding-usable-provider` 覆盖了本次改动的全部 3 个 golden 与那处数量下限。

## 5. 复查记录（2026-11-15）

复查方式：从源码重新导入目录，复算供应商数、模型数、withheld 集合与 42 个内置 id 的拆分；在 strict 与 deferred 两种模式下实测 `resolveProfiles({ opencode: {} })` 的拒绝；按 id 与模型名（忽略厂商前缀与大小写）重算 84 个 Zen 模型的去向、与 `opencode-go` 的重合数、以及 54 个闭源模型在各路由上的分布；逐份核对 6 个文档的 16 条路由清单与计数；全仓扫描 `opencode` 的每处引用（客户端代码、快照、fixture、webworker 桩）；重跑文档门禁、`test:docs`、`build`、`lint`、Web e2e 与 `llm-pi-ai` 测试。

本轮复查发现 3 处问题，均已修复：

| # | 问题 | 修复 |
|---|---|---|
| 1 | 注释与包 README 写「Zen 的闭源/付费模型也在 `openrouter` / 其他路由上出现」，实测 54 个闭源模型里 5 个没有其他内置路由（`claude-sonnet-4`、`gemini-3-flash`、`gemini-3.1-pro`、`gpt-5-codex`、`muse-spark-1.3-contributor-free`），37 个在 `openrouter` 上，其余 12 个只在厂商直连路由上 | 注释与中英 README 改为「多数（most / 大多数）闭源或付费模型在其他路由上仍可取到」，不再声称全部 |
| 2 | 升级指南把「除 `vercel-ai-gateway` 外都使用 OpenAI 兼容协议」作为事实，但同段前一句已说明 `amazon-bedrock` 与 `azure` 根本不是自定义模型 API 协议；且 `opencode` 与 `cloudflare-ai-gateway` 原生还带 Anthropic Messages / Responses 协议，「都使用」并不成立 | 改为「你自有的端点仍可通过自定义模型 API 接入」并直接列端点，再由末句说明 `vercel-ai-gateway` 可用 Anthropic Messages 重新接入；同步把 Xiaomi 两个变体的句子压成一句，最终 460 词（8% 余量） |
| 3 | 本日志第 1 节与第 4 节重复了同一处「`openrouter` 上有同名条目」的过宽说法 | 第 1 节改为「多数闭源模型在其他路由上仍可取到」；第 4 节补上「54 个闭源模型中 37 个在 `openrouter` 上」，并点明其余可用自定义模型 API 接入 |

复查另核对（未发现问题）：

- 目录：25 个供应商、765 个模型、42 个内置 id = 25 提供 + `typesafe`（仅分类模型、被过滤）+ 16 移除；`catalogProvider('opencode')` 为 `undefined`、`catalogModels('opencode')` 为空、`opencode-go` 29 个模型。
- 拒绝行为：strict 与 deferred 都抛 `provider "opencode" is removed from this build; delete the route or select another provider`。
- 覆盖口径：84 个模型 = 57 个 id 原样在其他路由 + 14 个换前缀同名 + 13 个无其他路由；13 个中 8 个是免费/实验档，与文档措辞一致；与 `opencode-go` 重合 17 个。
- 引用面：客户端代码、快照、fixture、web 源文件均无 `opencode` 路由引用；webworker 预览桩列的是「有效已安装 id」，`azure`、`cerebras` 等移除路由同样在列，按原样保留。
- 门禁：`test:docs` 21/21、`verify-upgrade-guides`、双语配对 1300 对、`md-wrap` 2652、`md-links` 2614、`verify-provider-data`、`llm-pi-ai` 480、Web e2e 20/22（2 个 WebKit 缺库）、`pnpm run lint` 0 warning 0 error、`pnpm run build` 10/10。

## 6. 已知限制

- 工作区里另有进程在写 `packages/client/ui-boot-animation` 并改动 `slot-catalog.ts`、各 `tsconfig`、`config-catalog` 等文件；本轮复查期间它一度让 `build`、`lint`、`doc-sync` 失败，修复后 `build` 10/10、`lint` 0 错误、`test:docs` 21/21 转绿，`doc-sync` 仍余 1 项 `Cordis inspect catalog` 失败（需其重新生成 `api-catalog.ts`），与本次移除无关。
- 免费档模型（8 个）按官方策略只允许 OpenCode 客户端，即便保留路由也无法从本仓库合法使用；自定义模型 API 只能覆盖付费模型。
- Zen 与 Go 的同名模型价格可能不同（如 `deepseek-v4-pro`：Zen $1.74/$3.84、Go $0.66/$1.98），迁移到 Go 后不要照搬 Zen 的价格估算。
- 4 个 System One 分类模型不属于目录的对话模型，`typesafe` 路由仍因只含分类模型被过滤；恢复 `opencode` 不会改变这两点。
- WebKit 浏览器未安装，两个键盘可访问性 e2e 未在 WebKit 下验证；Chromium 已覆盖 golden。
- 运行中的 `127.0.0.1:3080` 进程需要重启才会反映 25 项目录。
- 模型清单来自 `@earendil-works/pi-ai` 1.3.0 快照，OpenCode 线上模型与价格更新更快。

# 00018 — 移除 huggingface 供应商

- **日期**：2026-11-15
- **背景**：用户要求移除 `huggingface`（Hugging Face），且不能破坏项目现有功能。它不是模型厂商，而是 Hugging Face Inference Providers 的路由：pi-ai 以 `openai-completions` 接入 `https://router.huggingface.co/v1`，把请求转发给第三方推理服务商；目录快照包含 78 个模型、12 个组织，id 是 Hub 仓库名（如 `Qwen/Qwen3.5-397B-A17B`）。其中 64 个模型名在 `openrouter` 上出现（29 个换了厂商前缀）、1 个出现在 `nvidia`，13 个在本构建内没有其他内置路由。凭据为 API key（Web 设置页存为 `HUGGINGFACE_API_KEY`；未配置 `apiKeyEnv` 时回落到环境变量 `HF_TOKEN`），没有 OAuth。本机 Web profile 只配置了 `opencode-go`，未点名该路由。
- **范围**：`packages/llm/llm-pi-ai` 的目录、测试与双语 README；Web 设置页与 onboarding golden；`docs/user/guide/providers.md` 与 `.zh.md`；既有升级指南；`logs/`。

- **后续变更（00036）**：`openrouter` 已于 [00036](00036-移除openrouter供应商.md) 移除，本记录中以 `openrouter` 作为承接路由的说明随之失效；78 个 Hub 名中 15 个仍能对应已提供路由上的模型，63 个没有其他内置路由。现行覆盖见 00036 第 2 节与 [00009](00009-模型供应商清理汇总.md) 顶部的「后续变更（00036）」。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `huggingface`，共 14 条（按字母序插入 `cloudflare-workers-ai` 之后）。注释新增一句：它是 Hugging Face 的路由，78 个条目是第三方推理服务商提供的 Hub 仓库名，其中 64 个模型名也在 `openrouter` 上，因此它只是增加了一层路由与计费而不是另一家厂商的模型；末句协议说明改为 Moonshot、Xiaomi、Qwen 与 Hugging Face 路由使用 OpenAI 兼容协议、网关使用 Anthropic Messages。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | withheld 回归测试扩展到 14 条路由：`getBuiltinProviders()` 仍含该路由，而目录、`catalogProvider()`、`catalogModels()` 与可配置供应商列表均不含。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 存量 profile 拒绝测试参数化加入 `huggingface`，strict 与 deferred 均覆盖。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 更新为 14 条路由不提供；补一句：78 个条目是 Hub 仓库名、由第三方推理服务商提供，其中 64 个也在 `openrouter` 上（29 个换了前缀，1 个在 `nvidia` 上），仍可通过 OpenAI 兼容的自定义模型 API 以不同 Provider ID 接入；重新记录双语哈希。 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由段落加入 Hugging Face 并注明它是转发请求的路由；替代路径补上路由端点 `router.huggingface.co/v1`；补一句 64 个模型名可在 `openrouter` 找到、其余 13 个需依赖自定义路由；重新记录双语哈希。 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.md` / `.zh.md` / `.i18n.yaml` | front matter、变更段与协议段加入 `huggingface`，说明它使用 OpenAI 兼容协议、端点可直接作为自定义模型 API；补 78 / 64 / 13 的模型去向。同时合并同段重复表述（SGP/AMS 与 open 平台同模型的重复描述）并缩短新增句，把该文档从 502 词压回 475 词以内，守住 500 词预算。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除 `option "huggingface"`（可选项 28 → 27）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |
| `apps/web/tests/models-settings.e2e.ts` | 选择器数量下限由 `>= 28` 改为 `>= 27`：该断言只验证「休眠目录整体落入页面」，精确列表由 golden 负责。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现 `huggingface`，列表为 27 项 |
| 目录、模型解析、发现、登录 | 均不包含 `huggingface` |
| `catalogProvider('huggingface')` / `catalogModels('huggingface')` | 返回 `undefined` / 空 |
| 存量配置点名 `huggingface` | 明确报 `provider "huggingface" is removed from this build; delete the route or select another provider` |
| `openrouter` 与其余 26 条路由 | 不受影响 |
| 两条 DeepSeek 原生路由 | 不受影响 |

**模型的连带影响**：目录模型数从 929 降为 851，78 个路由条目退出选择器。64 个模型名在 `openrouter` 上有同名条目：29 个换了厂商前缀（例如 Hub 的 `zai-org/GLM-4.6` 对应 `openrouter` 的 `z-ai/glm-4.6`），其余 35 个前缀相同（4 个 id 完全一致，31 个仅大小写不同）；另有 1 个（`google/gemma-3-4b-it`）只出现在 `nvidia` 上。其余 13 个在本构建内没有其他内置路由（按模型名忽略大小写与厂商前缀比对，`openrouter` 上命名相近但不同的条目如 `qwen/qwen3-235b-a22b-2507` 不计入重合）：DeepSeek-V3、DeepSeek-V3-0324、DeepSeek-V3.1、MiniMax-M1-80k、Kimi-K2-Instruct、Kimi-K2-Instruct-0905、Qwen2.5-Coder-32B-Instruct、Qwen3-235B-A22B、Qwen3-235B-A22B-Instruct-2507、Qwen3-Coder-480B-A35B-Instruct、Qwen3-VL-235B-A22B-Thinking、MiMo-V2-Flash、GLM-4.6V-Flash；要继续使用需以 `openai-completions` 自定义模型 API 接入 `https://router.huggingface.co/v1`。

**配置级影响**：拒绝发生在整份 `providers` 字典解析时。若同一 profile 同时配置了 `huggingface` 与其他 pi-ai 路由，`llm-pi-ai` 插件整体加载失败，其他 pi-ai 路由要等该条目删除后才恢复；DeepSeek 两条原生路由由独立插件提供，不受影响。

**测试与 fixture 影响**：全仓扫描确认没有单元测试、录制会话、headless fixture 或 Web golden 以 `huggingface` 作为场景路由；受影响的只有枚举供应商列表的 3 个 golden 与 1 处数量下限。78 个模型里 77 个带价格元数据，但本仓库不读取 cost（`replay.ts` 将其归零、没有花费展示），因此移除不改变任何计费或推理行为。`packages/experimental/speech-to-text-sensevoice` 与 voice-input 里的 Hugging Face 引用是语音模型下载源，与该供应商目录无关，保持原样。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm`（全 llm 包） | 78 个文件、1787 个测试全部通过（含两条 DeepSeek 原生路由） |
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、478 个测试全部通过（新增 1 个参数化用例） |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过 |
| `pnpm run test:docs` | 21 个门禁全部通过（含双语配对与文档预算） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件保持 42 个 id） |
| `pnpm run build` | 10 个构建门禁全部通过（含 host/client 两个聚合检查） |
| `pnpm run test:snapshot -t pi-ai-conversation-updates` | 1 个录制会话回放通过 |
| `npx vitest run --config vitest.expected.config.ts -t "pi-ai"` | 2 个 headless 场景通过，内联快照未变 |
| Web e2e（Chromium，`replay`） | 只跑受影响的两个文件：20/22 通过；2 个失败均为 WebKit 用例（`keeps mobile model actions…` 与 `keeps keyboard reorder…`），宿主机缺少 WebKit 运行库（`libgstfft-1.0.so.0`、`libavif.so.13`、`libharfbuzz-icu.so.0` 等），与本次改动无关；3 个 golden 比对通过，未发生重写 |
| `npx vitest run packages/llm/llm-pi-ai --coverage`（仅计入 `catalog.ts`、`config.ts`） | 两文件语句/分支/函数/行均 100% |
| 目录自检（源码导入） | 27 个供应商、851 个模型；`huggingface` 不存在，其余路由中含 64 个 `openrouter` 同名模型与 1 个 `nvidia` 同名模型 |
| 上游定义核对 | pi-ai 1.3.0 中 `huggingface` 为 `https://router.huggingface.co/v1`、`openai-completions`、`HF_TOKEN`、78 个模型、12 个组织；`auth` 只声明 `apiKey`，无 OAuth |
| 文档一致性自检（逐文件比对 `WITHHELD_PROVIDERS`） | 6 个当前文档（用户指南 2、包 README 2、升级指南 2）均完整枚举 14 条路由，无过期计数；冻结的 v0.2.1-rc.2 指南与历史日志按原样保留 |
| golden 与目录一致性 | 三个 golden 的 option 列表与 `catalogProviderIds()` 逐项相等（27 个、同集合、字母序） |
| `oxlint` / `git diff --check` | 0 error；空白检查通过 |
| 复查后重跑（`test:docs` / `llm-pi-ai` / `verify-provider-data`） | 21/21、478 项通过、目录元数据一致 |

## 4. 设计取舍

- **移除路由而非某个厂商**：该端点把第三方推理服务商的模型集中转发，78 个条目里 64 个在 `openrouter` 已有同名模型，保留它会给同一批开源模型制造第二个入口，并让请求多经过一层中转、计费落在另一个账户体系；移除后需要聚合能力的用户仍可用自定义路由加回。
- **只移除目录暴露，不删上游数据**：不修改 `node_modules/@earendil-works/pi-ai`；`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 仍是仅有的两个控制点。
- **文档给出可逆路径**：升级指南与用户指南不只写「已移除」，还写明协议、端点与重接入方式，并给出 64 / 13 的模型去向，避免用户误以为 78 个模型全部消失。
- **守住升级指南的 500 词预算**：新增内容后该文档涨到 502 词并被 `verify-upgrade-guides` 拒绝；选择合并重复表述而非提高上限，最终 475 词，保留 5% 余量。
- **只回放受影响的 Web 用例**：全量 Web lane 有 244 个 e2e 文件且串行执行；`models-settings` 与 `onboarding-usable-provider` 两个文件覆盖了本次改动的全部 3 个 golden 与那处数量下限。
- **保留生成元数据与测试桩**：`provider-data.generated.ts` 必须完整覆盖已安装目录（`verify-provider-data` 校验）；webworker 的 `BUILTIN_PROVIDER_IDS` 是「必须使用有效已安装 id」的预览子集桩，两者均不修改。

## 5. 复查记录（2026-11-15）

复查方式：重读 `catalog.ts` 的剔除集合与注释；全仓扫描 `huggingface` / `HF_TOKEN` 的每一处引用；从 pi-ai 上游重新取 `getBuiltinModels` 复算 78 个模型的组织分布与同名关系；逐个 golden 选项与 `catalogProviderIds()` 对比；复算目录与模型数量；逐份核对承载清单的文档；最后重跑门禁。

本轮复查发现 3 处问题，均已修复：

| # | 问题 | 修复 |
|---|---|---|
| 1 | 4 份文档与本日志、汇总日志把 64 个重合模型说成「都以不同厂商前缀出现在 `openrouter`」，实测只有 29 个换了前缀，35 个前缀相同（4 个 id 完全一致、31 个仅大小写不同） | 全部改为「64 个出现在 `openrouter`（29 个换了厂商前缀）」，并补出 29 / 35 / 4 的拆分 |
| 2 | 78 个模型的账缺 1：原文只写 64 + 13，实际 `google/gemma-3-4b-it` 只在 `nvidia` 上同名 | 各处改为「64 在 `openrouter` + 1 在 `nvidia` + 13 无其他内置路由 = 78」 |
| 3 | 升级指南为压预算把「the same four MiMo models」压成「those four models too」，指代不清；本日志第 1、4 节原记的「474 词」已随修正改为 475 词 | 恢复明确表述并压缩同段其它冗余，最终 475 词（500 上限，留 5% 余量），同步修正本日志词数 |

复查另核对（未发现问题）：

- 重算 78 个模型：12 个组织、60 个支持推理、29 个支持图片输入、最大上下文 1,048,576、77 个带非零价格。
- 同名关系：64 个在 `openrouter` 同名（29 换前缀、35 同前缀、4 个 id 完全一致）、1 个在 `nvidia` 同名、13 个无同名条目；`openrouter` 上命名相近但不同的条目（如 `qwen/qwen3-235b-a22b-2507`）不计入重合。
- 13 个无替代路由的模型清单逐项比对，与文档表述一致。
- 6 份承载清单的文档各含 14/14 条 withheld id，无过期计数；`WITHHELD_PROVIDERS` 为 14 条。
- 三个 golden 各 27 项，与 `catalogProviderIds()` 同集合、字母序一致。
- 目录自洽：42 = 27 提供 + `typesafe`（仅分类模型被 `servesChat` 过滤）+ 14 移除。
- 全仓引用复扫：`provider-data.generated.ts`（必须覆盖已安装目录）、webworker 预览桩、`speech-to-text-sensevoice` 与 voice-input 的语音模型下载源、冻结的 v0.2.1-rc.2 指南与历史日志按原样保留。
- 本机 `$DSH_HOME/profiles/web/cordis.patch.yml` 只配置 `opencode-go`，`.credentials.yaml` 无 Hugging Face 引用，移除不会让现有安装加载失败。
- WebKit 两个失败仍是宿主机缺共享库（`libgstfft-1.0.so.0` 等），Chromium 同名用例通过，golden 比对通过。

核对结论：

- 三个 golden 呈现的正是当前在售目录（27 项），没有漏删或误删相邻选项。
- `huggingface` 在 pi-ai 中只声明 `apiKey`（无 OAuth），移除前也只能填 API Key，不存在「用户无法登录」的额外影响面。
- 它只使用 `openai-completions`，与保留路由的协议与推理档位不共享代码路径，移除不影响任何保留路由的报文格式。
- 改正后 `test:docs` 21/21、`llm-pi-ai` 478 项、`verify-provider-data` 通过，本日志与汇总日志的计数与实测一致。

## 6. 已知限制


- 13 个模型名没有其他内置路由，迁移需自定义模型 API；`HF_TOKEN` 仍是该端点的有效凭据。
- `openrouter` 上的 64 个同名模型中，29 个换了厂商前缀（不能照抄 Hub 仓库名），其余 35 个前缀相同（4 个 id 完全一致，31 个仅大小写不同）；`google/gemma-3-4b-it` 在 `nvidia` 上同名。
- WebKit 浏览器未安装，两个键盘可访问性 e2e 未在 WebKit 下验证；Chromium 已覆盖 golden。
- 运行中的 `127.0.0.1:3080` 进程需要重启才会反映目录变化。
- 模型清单来自 `@earendil-works/pi-ai` 1.3.0 快照，Hugging Face 线上路由的模型与价格更新更快。

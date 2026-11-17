# 00019 — 移除 cerebras 供应商

- **日期**：2026-11-15
- **背景**：用户要求移除 `cerebras`（Cerebras Systems），且不能破坏项目现有功能。它是芯片厂商自建的托管推理云，不是路由聚合器：pi-ai 以 `openai-completions` 接入 `https://api.cerebras.ai/v1`，只用 API key（环境变量 `CEREBRAS_API_KEY`），没有 OAuth，也没有 baseURL 环境变量。目录快照只有 2 个模型，都是别人开源的权重、由它在自家硬件上托管：`gpt-oss-120b`（131,072 上下文 / 40,960 输出，$0.35/$0.75，纯文本）与 `qwen-3.8-27b`（同尺寸，$0.99/$1.49、缓存读 $0.99，文本加图片，图片入库前缩放到 2000×2000 ／ 4.5MB），两者都支持推理档位。按模型名（忽略大小写与厂商前缀）比对：`gpt-oss-120b` 以 `openai/gpt-oss-120b` 出现在 `baseten`、`groq`、`openrouter`、`together` 四条路由上；`qwen-3.8-27b` 没有同名条目；`groq` 与 `openrouter` 上另有 `qwen/qwen3.8-27b`（名称为 Qwen3.8 27B，`openrouter` 带 `Qwen:` 前缀），连字符写法与本路由不同，按 id 不计入重合。本机 Web profile 只配置了 `opencode-go`，未点名该路由。
- **范围**：`packages/llm/llm-pi-ai` 的目录、测试与双语 README；Web 设置页与 onboarding golden；`docs/user/guide/providers.md` 与 `.zh.md`；既有升级指南；`logs/`。

- **后续变更**：`baseten` 已于 [00026](00026-移除baseten供应商.md) 移除，`groq` 与 `together` 也分别于 [00030](00030-移除groq供应商.md)、[00032](00032-移除together供应商.md) 移除，本记录中作为承接路由出现的三条路由均不再可用；`gpt-oss-120b` 现只由 `openrouter` 以 `openai/gpt-oss-120b` 提供，`qwen-3.8-27b` 见 `openrouter` 的 `qwen/qwen3.8-27b`，现行覆盖见该文件第 3 节。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `cerebras`，共 15 条（按字母序插入 `azure` 之后）。注释新增一句：它是芯片厂商的推理云，两个开源权重模型在本构建的其他路由上已能取到（`gpt-oss-120b` 以 `openai/` 前缀出现在四条路由、Qwen3.8 27B 出现在两条），因此移除的是托管入口而不是模型覆盖；末句协议说明改为 Moonshot、Xiaomi、Qwen、Cerebras 与 Hugging Face 路由使用 OpenAI 兼容协议、网关使用 Anthropic Messages。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | withheld 回归测试扩展到 15 条路由：`getBuiltinProviders()` 仍含该路由，而目录、`catalogProvider()`、`catalogModels()` 与可配置供应商列表均不含。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 存量 profile 拒绝测试参数化加入 `cerebras`，strict 与 deferred 均覆盖。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 更新为 15 条路由不提供；补一段：两个开源权重模型在 `baseten`、`groq`、`openrouter`、`together` 与 `groq`、`openrouter` 上仍可取到，端点仍可通过 OpenAI 兼容的自定义模型 API 以不同 Provider ID 接入；重新记录双语哈希。 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由段落加入 Cerebras 并注明它是芯片厂商自建的推理云；替代路径补上端点 `api.cerebras.ai/v1`；补一句两个模型在其他路由上的 id；重新记录双语哈希。 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.md` / `.zh.md` / `.i18n.yaml` | front matter、变更段与协议段加入 `cerebras`，端点列为可重新接入的自定义模型 API；补两个模型的去向。新增内容后该文档涨到 490 词，超出 5% 余量，遂合并同段冗余表述（SGP/AMS 与 CN 套餐同模型的重复描述、Moonshot 端点描述、Qwen 端点措辞、迁移步骤 1 与 4），最终 472 词（被合并的 SGP/AMS 与 CN 套餐同模型事实仍保留在包 README 与用户指南中）。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除 `option "cerebras"`（可选项 27 → 26）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |
| `apps/web/tests/models-settings.e2e.ts` | 选择器数量下限由 `>= 27` 改为 `>= 26`：该断言只验证「休眠目录整体落入页面」，精确列表由 golden 负责。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现 `cerebras`，列表为 26 项 |
| 目录、模型解析、发现、登录 | 均不包含 `cerebras` |
| `catalogProvider('cerebras')` / `catalogModels('cerebras')` | 返回 `undefined` / 空 |
| 存量配置点名 `cerebras` | 明确报 `provider "cerebras" is removed from this build; delete the route or select another provider` |
| 其余 25 条供应商 | 不受影响 |
| 两条 DeepSeek 原生路由 | 不受影响 |

**模型的连带影响**：目录模型数从 851 降为 849，2 个条目退出选择器。按同名口径 `gpt-oss-120b` 仍有四条内置路由；`qwen-3.8-27b` 这个 id 只存在于本路由，其他路由写作 `qwen/qwen3.8-27b`。`gpt-oss-120b` 在四条路由上以 `openai/gpt-oss-120b` 提供，价格因路由而异（`groq` 与 `openrouter` 为 $0.15/$0.60、缓存读 $0.075，Cerebras 自身为 $0.35/$0.75、无缓存计价）；`qwen-3.8-27b` 在 `groq` 与 `openrouter` 上写作 `qwen/qwen3.8-27b`，catalog 名称同为 Qwen3.8 27B，但上下文与价格按路由给出（`groq` 131,042 ／ $0.8–$4，`openrouter` 262,144 ／ $0.24–$2.2）。要继续走 Cerebras 自己的端点，以 `openai-completions` 自定义模型 API 接入 `https://api.cerebras.ai/v1`。

**配置级影响**：拒绝发生在整份 `providers` 字典解析时。若同一 profile 同时配置了 `cerebras` 与其他 pi-ai 路由，`llm-pi-ai` 插件整体加载失败，其他 pi-ai 路由要等该条目删除后才恢复；DeepSeek 两条原生路由由独立插件提供，不受影响。

**测试与 fixture 影响**：全仓扫描确认没有单元测试、录制会话、headless fixture 或 Web golden 以 `cerebras` 作为场景路由；受影响的只有枚举供应商列表的 3 个 golden 与 1 处数量下限。`provider-data.generated.ts` 必须继续覆盖已安装目录（`verify-provider-data` 校验），webworker 的 `BUILTIN_PROVIDER_IDS` 是「必须使用有效已安装 id」的预览子集桩，两者都不修改。`packages/experimental/browser-use-stagehand-native` 的 README 提到 Cerebras 是它所固定的 Stagehand SDK 自带的模型目录，与本仓库的 pi-ai 目录无关，保持原样。本仓库不读取 `cost`（回放时归零、没有花费展示），因此移除不改变任何计费或推理行为。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm`（全 llm 包） | 78 个文件、1788 个测试全部通过（含两条 DeepSeek 原生路由） |
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、479 个测试全部通过（新增 1 个参数化用例） |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过 |
| `pnpm run test:docs` | 21 个门禁全部通过（含升级指南结构、链接、换行与双语配对） |
| `pnpm run doc-sync`（复查轮） | 43 个门禁全部通过（含文档站构建与死链检查） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件保持 42 个 id） |
| `pnpm run build` | 10 个构建门禁全部通过（28.57s，含 host/client 两个聚合检查与 Web build） |
| `pnpm run test:snapshot -t pi-ai-conversation-updates` | 1 个录制会话回放通过 |
| `npx vitest run --config vitest.expected.config.ts -t "pi-ai"` | 2 个 headless 场景通过，内联快照未变 |
| Web e2e（`replay`） | 只跑受影响的两个文件：20/22 通过，3 个 golden 比对通过、未发生重写；2 个失败均为 WebKit 用例，宿主机缺 `libavif.so.13`、`libharfbuzz-icu.so.0` 等共享库，与本次改动无关 |
| Web e2e 首次回放失败原因 | 首次回放 17 个失败：e2e 启动的是已构建产物，`lib/` 仍是改动前的目录（供应商列表多出 `cerebras`），且该轮与 `pnpm run build` 并行执行。抽查两个失败用例，差异均为该选项行；把该行单独加回 `empty.expected.md` 后独占回放只失败 3 个用例，所以 17 个失败里只有一部分来自 golden 差异，其余未逐一归因。重建后独占回放 20/22 |
| golden 与目录一致性 | 三个 golden 各 26 项，与 `catalogProviderIds()` 同集合、字母序一致 |
| 目录自检（源码导入） | 26 个供应商、849 个模型；`cerebras` 不存在，`catalogProvider()` 为 `undefined`、`catalogModels()` 为空 |
| 上游定义核对 | pi-ai 1.3.0 中 `cerebras` 为 `https://api.cerebras.ai/v1`、`openai-completions`、`CEREBRAS_API_KEY`、2 个模型；`auth` 只声明 `apiKey`，无 OAuth，无 baseURL 环境变量 |
| 文档一致性自检 | 6 个当前文档（用户指南 2、包 README 2、升级指南 2）均完整枚举 15 条路由，无过期计数 |
| 升级指南词数 | 472 词（上限 500，保留 5.6% 余量） |
| `pnpm run lint` / `git diff --check` | 0 warning、0 error；空白检查通过（提交钩子的 49 条规则集另报 `catalog.ts:435` 一条既有 oxlint 提示，非本次改动引入） |
| 复查后重跑（`test:docs` / `doc-sync` / `build` / Web e2e） | 21/21、43/43、10/10、20/22 全部通过；升级指南 472 词，6 份双语文档各覆盖 15/15 条路由，双语配对哈希已重录 |

## 4. 设计取舍

- **只移除目录暴露，不删上游数据**：不修改 `node_modules/@earendil-works/pi-ai`；`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 仍是仅有的两个控制点，二者都能按原样恢复。
- **移除的是托管入口而非模型覆盖**：两个模型都是别人开源的权重，本构建已在四条与两条路由上覆盖同一批模型；保留它等于为同一批开源模型再开一套账户与计价体系（`CEREBRAS_API_KEY`、另一套单价），而需要其速度特性的用户仍可用自定义模型 API 加回。
- **升级指南并入既有批次文档**：版本号未变，`dsh-create-upgrade-guide` 要求更新覆盖同一 surfaces 的既有指南，而不是为同一版本再开一个条目；代价是该文档的词数预算被进一步压缩。
- **压缩既有表述而不提高词数上限**：新增内容后指南到 490 词，`verify-upgrade-guides` 的 500 词上限虽仍可通过但只剩 2% 余量；选择合并同段重复描述，最终 472 词（复查时又收紧一处措辞），保住 5% 余量。
- **只回放受影响的 Web 用例**：全量 Web lane 串行执行代价过高；`models-settings` 与 `onboarding-usable-provider` 两个文件覆盖了本次改动的全部 3 个 golden 与那处数量下限。

## 5. 复查记录（2026-11-15）

复查方式：从 pi-ai 上游重新取 `builtinProviders()` 与 `getBuiltinModels('cerebras')` 核对端点、鉴权与两个模型的 id、价格、上下文、模态；用同名口径重算它们在本构建其他路由上的条目；重读 `WITHHELD_PROVIDERS` 与注释；全仓扫描 `cerebras` 的每一处引用；逐个 golden 与 `catalogProviderIds()` 对比；复算目录、模型与 42 个 id 的拆分；逐份核对承载清单的文档与日志计数；重跑文档门禁、`doc-sync` 与构建。

本轮复查发现 4 处问题，均已修复：

| # | 问题 | 修复 |
|---|---|---|
| 1 | 本日志与汇总日志把首次 e2e 回放的 17 个失败全部归因为「golden 多出 `cerebras`」；实测把 `option "cerebras"` 单独加回 `empty.expected.md` 后独占回放只失败 3 个用例，而那一轮还与 `pnpm run build` 并行 | 改为只陈述可归因部分：抽查两个失败用例差异均为该行、三个 golden 约对应 9 个用例，其余失败未逐一归因 |
| 2 | 日志称 `qwen/qwen3.8-27b` 的「catalog 名称同为 Qwen3.8 27B」；`openrouter` 实际写作 `Qwen: Qwen3.8 27B` | 分别注明两处 catalog 名称（`openrouter` 带 `Qwen:` 前缀） |
| 3 | 用户指南写「Both Cerebras models stay on other routes」、日志写「没有模型因此失去全部内置路由」，都暗示同名条目仍在；按同名口径 `qwen-3.8-27b` 在其他路由上并不存在，只有连字符写法不同的 `qwen/qwen3.8-27b` | 指南与升级指南改为「remain available elsewhere」并保留目标 id，用户指南另注「`qwen` 后没有连字符」；日志明确写出「`qwen-3.8-27b` 这个 id 只存在于本路由」 |
| 4 | 复查时又收紧一处措辞（「remain on other routes」→「remain available elsewhere」），升级指南由 473 词变为 472 词，日志仍记 473 | 第 1、3、4 节与词数行统一改为 472 词（5.6% 余量） |

复查另核对（未发现问题）：

- 上游定义：`cerebras` 为 `https://api.cerebras.ai/v1`、`openai-completions`，`auth` 只有 `apiKey`（无 OAuth），无 baseURL 环境变量；两个模型的 id、名称、上下文 131,072、输出上限 40,960、价格与模态（`gpt-oss-120b` 纯文本、`qwen-3.8-27b` 含图片）与本日志一致。
- 同名关系：`gpt-oss-120b` 恰好 4 条，都是 `openai/gpt-oss-120b`（`baseten`、`groq`、`openrouter`、`together`）；`qwen-3.8-27b` 同名 0 条，连字符不敏感时命中 `groq` 与 `openrouter` 的 `qwen/qwen3.8-27b`。
- `WITHHELD_PROVIDERS` 为 15 条、字母序正确（`xiaomi` 尾部不排序是既有情况）。
- 目录自洽：42 = 26 提供 + `typesafe` + 15 移除；三个 golden 各 26 项，与 `catalogProviderIds()` 同集合、字母序一致。
- 6 份双语文档各枚举 15/15 条路由；`verify-provider-data` 通过，`provider-data.generated.ts` 与 webworker 预览桩未改动。
- 全仓引用复扫：`browser-use-stagehand-native` 的 Cerebras 是其所固定 Stagehand SDK 自带的模型目录，与本仓库目录无关；`speech-to-text-sensevoice` 的 Hugging Face 引用同理，均按原样保留。
- 门禁：`test:docs` 21/21、`doc-sync` 43/43、`build` 10/10、`llm` 1788、`ui-settings-models` 483、录制会话 1、headless 预期 2、Web e2e 20/22（2 个失败为 WebKit 宿主机缺库）。

核对结论：

- 剔除只发生在 `WITHHELD_PROVIDERS` 与 `resolveProfiles()` 两个控制点，未触碰上游包、生成元数据或运行时分支，保留的 26 条路由不受影响。
- 文档与日志中的每个计数（26、849、15、42、4、0、472）都能从源码或上游数据复现。
- 剩余 2 个 WebKit e2e 失败是宿主机缺共享库，与本次改动无关；Chromium 用例与 golden 比对均通过。

## 6. 已知限制

- `qwen-3.8-27b` 与 `qwen/qwen3.8-27b` 是否完全同一模型只能按名称与模态判断：catalog 未声明两者关系，且上下文与价格按路由不同（131,072 / 131,042 / 262,144）。
- `gpt-oss-120b` 在各路由的缓存能力不同：Cerebras 的 `cacheRead` 为 0（不计缓存价），`groq` 与 `openrouter` 为 $0.075/百万 token；迁移后不要照搬对缓存成本的估算。
- WebKit 浏览器未安装，两个键盘可访问性 e2e 未在 WebKit 下验证；Chromium 已覆盖 golden。
- 运行中的 `127.0.0.1:3080` 进程需要重启才会反映目录变化。
- 模型清单来自 `@earendil-works/pi-ai` 1.3.0 快照，Cerebras 线上模型与价格更新更快。

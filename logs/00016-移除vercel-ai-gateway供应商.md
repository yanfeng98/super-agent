# 00016 — 移除 vercel-ai-gateway 供应商

- **日期**：2026-11-15
- **背景**：用户要求移除 `vercel-ai-gateway`（Vercel AI Gateway），且不能破坏项目现有功能。它是聚合网关而不是模型厂商：pi-ai 以 `anthropic-messages` 接入 `https://ai-gateway.vercel.sh`，目录快照包含 301 个模型、32 个厂商前缀，其中 118 个 id 与 `openrouter` 原样重合；网关把多家厂商的模型集中到一条第三方端点转售，其中一部分本构建已能从厂商原生路由获得。凭据为 API key（Web 设置页存为 `VERCEL_AI_GATEWAY_API_KEY`；未配置 `apiKeyEnv` 时回落到环境变量 `AI_GATEWAY_API_KEY`），没有 OAuth。本机 Web profile 只配置了 `opencode-go`，未点名该路由。
- **范围**：`packages/llm/llm-pi-ai` 的目录、测试与双语 README；Web 设置页与 onboarding golden；`docs/user/guide/providers.md` 与 `.zh.md`；既有升级指南；`logs/`。

- **后续变更（00036）**：`openrouter` 已于 [00036](00036-移除openrouter供应商.md) 移除，本记录中以 `openrouter` 作为承接路由的说明随之失效；301 个 id 中 87 个仍能对应已提供路由上的模型，214 个没有其他内置路由。现行覆盖见 00036 第 2 节与 [00009](00009-模型供应商清理汇总.md) 顶部的「后续变更（00036）」。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `vercel-ai-gateway`，共 13 条（按字母序插入 `xiaomi` 之前）。注释新增一句：它是聚合网关，转售本构建已能从各厂商原生路由获得的模型，301 个条目在厂商前缀下重复了原生路由；末句统一说明 Moonshot、Xiaomi、Qwen 路由使用 OpenAI 兼容协议、网关使用 Anthropic Messages，需要时都可作为自定义模型 API 接入。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | withheld 回归测试扩展到 13 条路由：`getBuiltinProviders()` 仍含该路由，而目录、`catalogProvider()`、`catalogModels()` 与可配置供应商列表均不含。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 存量 profile 拒绝测试参数化加入 `vercel-ai-gateway`，strict 与 deferred 均覆盖。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 更新为 13 条路由不提供；补一句：网关转售本构建已能原生获得的模型，301 个条目带厂商前缀、其中 118 个也在 `openrouter` 原样出现，仍可通过 `anthropic-messages` 自定义模型 API 以不同 Provider ID 接入；重新记录双语哈希。 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由段落加入 Vercel AI Gateway；替代路径说明改为「OpenAI 兼容或 `anthropic-messages` 端点」并补上网关端点 `ai-gateway.vercel.sh`；补一句 118 个 id 仍可在 `openrouter` 原样使用，其余 id 需保留自定义路由；重新记录双语哈希。 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.md` / `.zh.md` / `.i18n.yaml` | front matter、变更段与协议段加入 `vercel-ai-gateway`，说明它使用 Anthropic Messages、可用该协议重新接入；同时修正上一轮留下的过期事实「`openrouter` 与 `vercel-ai-gateway` 都列出 MiMo-V2.6-Pro-UltraSpeed」为仅 `openrouter`；重新记录双语哈希。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除 `option "vercel-ai-gateway"`（可选项 29 → 28）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |
| `apps/web/tests/models-settings.e2e.ts` | 选择器数量下限由 `>= 29` 改为 `>= 28`：该断言只验证「休眠目录整体落入页面」，精确列表由 golden 负责。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现 `vercel-ai-gateway`，列表为 28 项 |
| 目录、模型解析、发现、登录 | 均不包含 `vercel-ai-gateway` |
| `catalogProvider('vercel-ai-gateway')` / `catalogModels('vercel-ai-gateway')` | 返回 `undefined` / 空 |
| 存量配置点名 `vercel-ai-gateway` | 明确报 `provider "vercel-ai-gateway" is removed from this build; delete the route or select another provider` |
| `openrouter` 与其余 27 条路由 | 不受影响 |
| 两条 DeepSeek 原生路由 | 不受影响 |

**模型的连带影响**：目录模型数从 1230 降为 929，301 个网关条目退出选择器。其中 118 个 id 在 `openrouter` 上原样存在（例如 `xiaomi/mimo-v2.6-pro-ultraspeed`），可直接改选 `openrouter` 的同一 id；其余 183 个 id（`openai/…`、`anthropic/…`、`alibaba/…` 等厂商前缀条目）在本构建中没有同名路由，需要以 `anthropic-messages` 自定义模型 API 接入 `https://ai-gateway.vercel.sh` 才能继续使用。

**配置级影响**：拒绝发生在整份 `providers` 字典解析时。若同一 profile 同时配置了 `vercel-ai-gateway` 与其他 pi-ai 路由，`llm-pi-ai` 插件整体加载失败，其他 pi-ai 路由要等该条目删除后才恢复；DeepSeek 两条原生路由由独立插件提供，不受影响。

**测试与 fixture 影响**：全仓扫描确认没有单元测试、录制会话、headless fixture 或 Web golden 以 `vercel-ai-gateway` 作为场景路由；受影响的只有枚举供应商列表的 3 个 golden 与 1 处数量下限。网关有 297/301 个模型带价格元数据，但本仓库不读取 cost（`replay.ts` 将其归零、没有花费展示），因此移除不改变任何计费或推理行为。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm`（全 llm 包） | 78 个文件、1786 个测试全部通过（含两条 DeepSeek 原生路由） |
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、477 个测试全部通过（新增 1 个参数化用例） |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过 |
| `pnpm run test:docs` | 21 个门禁全部通过（含双语配对与文档预算） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件保持 42 个 id） |
| `pnpm run build` | 10 个构建门禁全部通过 |
| `pnpm run build:check` | host/client 两个聚合检查通过 |
| `pnpm run test:snapshot -t pi-ai-conversation-updates` | 1 个录制会话回放通过 |
| `npx vitest run --config vitest.expected.config.ts -t "pi-ai"` | 2 个 headless 场景通过，内联快照未变 |
| Web e2e（Chromium，`replay`） | 只跑受影响的两个文件：20/22 通过；2 个失败均为 WebKit 用例（`keeps mobile model actions…` 与 `keeps keyboard reorder…`），宿主机缺少 WebKit 运行库（`libgstfft-1.0.so.0` 等），与本次改动无关；3 个 golden 比对通过，未发生重写 |
| `npx vitest run packages/llm/llm-pi-ai --coverage`（仅计入 `catalog.ts`、`config.ts`） | 两文件语句/分支/函数/行均 100% |
| 目录自检（源码导入） | 28 个供应商、929 个模型；`vercel-ai-gateway` 不存在，`openrouter` 仍提供 118 个同名 id |
| 上游定义核对 | pi-ai 1.3.0 中 `vercel-ai-gateway` 为 `https://ai-gateway.vercel.sh`、`anthropic-messages`、`AI_GATEWAY_API_KEY`、301 个模型；`auth` 只声明 `apiKey`，无 OAuth |
| 文档一致性自检（逐文件比对 `WITHHELD_PROVIDERS`） | 6 个当前文档（用户指南 2、包 README 2、升级指南 2）均完整枚举 13 条路由，无过期计数；冻结的 v0.2.1-rc.2 指南与历史日志按原样保留 |
| golden 与目录一致性 | 三个 golden 的 option 列表与 `catalogProviderIds()` 逐项相等（28 个、同集合、字母序） |
| `oxlint` / `git diff --check` | 0 error；空白检查通过 |
| 提交后复核 | 重算网关事实（301 / 32 / 118 / 297 / 协议）、脚本比对 13 条 id 的四处列举、复扫全仓引用与迁移建议；4 处文档/日志问题均已修复，`test:docs` 21/21、`llm-pi-ai` 477 项复跑通过 |

## 4. 设计取舍

- **移除聚合网关而非某个厂商**：网关把第三方厂商的模型集中转售，其中一部分本构建已能原生获得，保留它会给这些模型制造第二个入口，并让请求多经过一层第三方中转；移除后每个模型至多对应一条直连路由，需要聚合能力的用户仍可用自定义路由加回。
- **只移除目录暴露，不删上游数据**：不修改 `node_modules/@earendil-works/pi-ai`；`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 仍是仅有的两个控制点。
- **文档给出可逆路径**：升级指南与用户指南不只写「已移除」，还写明网关使用 Anthropic Messages、端点与重接入方式，以及 118 个 id 可在 `openrouter` 原样替代。
- **同步修正上一轮的过期事实**：升级指南里「`openrouter` 与 `vercel-ai-gateway` 都列出 MiMo-V2.6-Pro-UltraSpeed」在本次移除后不再成立，必须改成仅 `openrouter`，否则指南自相矛盾。
- **只回放受影响的 Web 用例**：全量 Web lane 有 244 个 e2e 文件且 `fileParallelism: false` 串行执行；`models-settings` 与 `onboarding-usable-provider` 两个文件覆盖了本次改动的全部 3 个 golden 与那处数量下限。
- **保留生成元数据与测试桩**：`provider-data.generated.ts` 必须完整覆盖已安装目录（`verify-provider-data` 校验）；webworker 的 `BUILTIN_PROVIDER_IDS` 是「必须使用有效已安装 id」的预览子集桩，两者均不修改。

## 5. 复查记录（2026-11-15）

复查方式：重读 `catalog.ts` 的剔除集合与注释，全仓扫描 `vercel-ai-gateway` 的每一处引用，核对 pi-ai 上游的 auth / baseURL / 协议 / 模型定义，比对网关模型与其余路由模型 id 的重合度，逐个 golden 选项与 `catalogProviderIds()` 对比，复算目录与模型数量，检查本机 profile 与凭据文件，最后重跑门禁。

两轮复查共发现 4 处问题，均已修复：

| # | 问题 | 修复 |
|---|---|---|
| 1 | 升级指南（v0.2.2-rc.3）仍称「`openrouter` 与 `vercel-ai-gateway` 列出 MiMo-V2.6-Pro-UltraSpeed」，移除后该路由已不存在 | 中英双语改为仅 `openrouter` |
| 2 | 中文升级指南插入网关句时用了半角分号并截断了「——Xiaomi 开放平台端点…」的列举链，出现 `同样如此;… 。，它与…` 的断句 | 把网关句移到列举结束之后，改用中文标点 |
| 3 | 汇总日志 `00009` 的供应商数、模型数、withheld 列表与校验统计仍停留在上一轮 | 全部更新为 28 / 929 / 13 条与本轮实测结果 |
| 4 | 英文升级指南把网关句插在「That includes the Xiaomi open-platform endpoint…」之前，`That includes` 的指代对象变成网关，且与中文版的句末位置不平行 | 网关句移到端点枚举句之后，与中文版一致；重录 i18n 后 `test:docs` 21/21 通过 |

第二轮复查（提交后复核）另行核对：

- 直接从 pi-ai 的 `getBuiltinModels('vercel-ai-gateway')` 重算：301 个模型、32 个厂商前缀、118 个 id 与 `openrouter` 同名、全部 `anthropic-messages`、297 个带非零价格，与代码注释、README、升级指南和本日志逐项一致。
- 脚本比对 13 条被移除 id 在 `WITHHELD_PROVIDERS`、`catalog.spec.ts`、`config.spec.ts` 与三个 golden 中的列举，四处完全一致，golden 中不含任一被移除 id。
- `providers.md` 所列「自定义模型 API 可选四种协议」与 `provider.ts` 的 `PROTOCOLS`（含 `anthropic-messages`）一致，因此「以 Anthropic Messages 加回网关」的迁移建议可执行。
- `replay.ts` 将 cost 归零，客户端也没有花费展示，因此移除自带价格的网关不改变计费或推理行为。
- 复扫全仓：`website/.dist` 是生成物（未跟踪），`provider-data.generated.ts`、webworker 预览桩、冻结的 v0.2.1-rc.2 指南与历史日志按原样保留；`logs/00015` 记录的是上一轮事实，不由本轮改写。

核对结论：

- 三个 golden 的 option 列表与 `catalogProviderIds()` 逐项相等（28 个、同集合、字母序），页面呈现的正是当前在售目录，没有漏删或误删相邻选项。
- `vercel-ai-gateway` 在 pi-ai 中的 `auth` 只声明 `apiKey`（无 `oauth`），移除前在选择器里也只能填 API Key，不存在「用户无法登录」的额外影响面。
- 网关只使用 `anthropic-messages` 一种协议，与其余路由的协议、推理档位互不共享，移除不影响任何保留路由的报文格式。
- 目录数量自洽：42 个内置 id = 28 个提供 + `typesafe`（仅分类模型被 `servesChat` 过滤）+ 13 个移除。
- 全仓除测试参数、文档与日志外，`vercel-ai-gateway` 只出现在 `provider-data.generated.ts`（必须覆盖已安装目录）、webworker 的预览桩列表、冻结的 v0.2.1-rc.2 升级指南与历史日志中，前者两处刻意保留，后者是已发布的记录。
- 本机 `$DSH_HOME/profiles/web/cordis.patch.yml` 只配置 `opencode-go`，`.credentials.yaml` 无相关引用，移除不会让现有安装加载失败。
- WebKit 的两个失败由宿主机缺共享库引起：同一文件的 Chromium 用例在完整用例顺序下通过，golden 比对也通过。
- 覆盖率无回退：`catalog.ts` 与 `config.ts` 的语句、分支、函数、行均为 100%。

## 6. 已知限制与后续

- 当前 `127.0.0.1:3080` 的进程仍显示旧列表，重启后生效。
- 网关专有的 183 个模型 id 失去内置路由；需要时以 `anthropic-messages` 自定义模型 API 接入 `https://ai-gateway.vercel.sh`，Provider ID 不能复用被移除的 `vercel-ai-gateway`；118 个与 `openrouter` 同名的 id 可直接改用 `openrouter`。
- WebKit 浏览器运行库未安装，两个键盘/触屏可访问性 e2e 未在 WebKit 下验证；Chromium 已覆盖 golden。
- 若将来恢复该路由，需要删除集合项、更新 withheld 测试参数、恢复 3 个 Web golden 与双语文档，并把数量下限改回 29，最后重新记录 i18n 哈希。

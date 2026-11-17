# 00015 — 移除 xiaomi 供应商

- **日期**：2026-11-15
- **背景**：用户要求移除 `xiaomi`（小米 MiMo 的按量计费开放平台接入），且不能破坏项目现有功能。同厂商的中国区 token 套餐 `xiaomi-token-plan-cn` 保留。`xiaomi` 提供 6 个模型，其中 4 个（MiMo-V2.5、V2.5-Pro、V2.6-Flash、V2.6-Pro）与 `xiaomi-token-plan-cn` 逐字段相同，另有两个 UltraSpeed 变体（`mimo-v2.5-pro-ultraspeed`、`mimo-v2.6-pro-ultraspeed`）只在该路由下以裸模型 id 出现（`openrouter` 与 `vercel-ai-gateway` 各有一条 `xiaomi/mimo-v2.6-pro-ultraspeed`）。本机 Web profile 只配置了 `opencode-go`，未点名该路由。
- **范围**：`packages/llm/llm-pi-ai` 的目录、测试与双语 README；Web 设置页与 onboarding golden；`docs/user/guide/providers.md` 与 `.zh.md`；既有升级指南；`logs/`。

- **后续变更（00036）**：`openrouter` 已于 [00036](00036-移除openrouter供应商.md) 移除，本记录中以 `openrouter` 作为承接路由的说明随之失效；`mimo-v2.5-pro-ultraspeed` 与 `mimo-v2.6-pro-ultraspeed` 现均无其他内置路由（本记录中后者「另见 `openrouter`、`vercel-ai-gateway`」的说法失效）；其余四个与 `xiaomi-token-plan-cn` 相同的模型不受影响。现行覆盖见 00036 第 2 节与 [00009](00009-模型供应商清理汇总.md) 顶部的「后续变更（00036）」。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `xiaomi`，共 12 条。注释把三条 Xiaomi 路由放在同一句：两条 token 套餐路由重复 `xiaomi-token-plan-cn` 的四个模型，`xiaomi` 是按量计费开放平台，其两个额外模型是套餐已覆盖模型的 UltraSpeed 变体；末句统一说明 Moonshot、Xiaomi、Qwen 路由都使用 OpenAI 兼容协议，需要时可作为自定义模型 API 接入。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | withheld 回归测试扩展到 12 条路由：`getBuiltinProviders()` 仍含该路由，而目录、`catalogProvider()`、`catalogModels()` 与可配置供应商列表均不含。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 存量 profile 拒绝测试参数化加入 `xiaomi`，strict 与 deferred 均覆盖。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 更新为 12 条路由不提供；Xiaomi 一句改为「`xiaomi` 是按量计费开放平台，端点 `https://api.xiaomimimo.com/v1` 提供与 `xiaomi-token-plan-cn` 相同的四个模型另加两个 UltraSpeed 变体；两条 token 套餐路由仅端点区域不同；`xiaomi-token-plan-cn` 仍提供」；重新记录双语哈希。 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由段落加入 Xiaomi（按量计费开放平台），替代路径补充 `api.xiaomimimo.com/v1` 与 `token-plan-sgp.xiaomimimo.com/v1`、`token-plan-ams.xiaomimimo.com/v1`；结尾说明 `kimi-coding` 与 `xiaomi-token-plan-cn` 仍然提供；重新记录双语哈希。 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.md` / `.zh.md` / `.i18n.yaml` | front matter、变更段与替代路径说明加入 `xiaomi`，并把「SGP 与 AMS 端点」扩写为三条 Xiaomi 路由各自的可达端点；补一句两个 UltraSpeed 模型的现状（V2.6 仍可由 `openrouter`、`vercel-ai-gateway` 使用，V2.5 已无内置路由）；重新记录双语哈希。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除 `option "xiaomi"`（可选项 30 → 29）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |
| `apps/web/tests/models-settings.e2e.ts` | 选择器数量下限由 `>= 30` 改为 `>= 29`：该断言只验证「休眠目录整体落入页面」，精确列表由 golden 负责。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现 `xiaomi`，列表为 29 项 |
| 目录、模型解析、发现、登录 | 均不包含 `xiaomi` |
| `catalogProvider('xiaomi')` / `catalogModels('xiaomi')` | 返回 `undefined` / 空 |
| 存量配置点名 `xiaomi` | 明确报 `provider "xiaomi" is removed from this build; delete the route or select another provider` |
| `xiaomi-token-plan-cn` | 不受影响，4 个模型保持可用 |
| `xiaomi-token-plan-sgp` / `-ams` | 此前已被移除，不受影响 |
| 其他提供商与两条 DeepSeek 原生路由 | 不受影响 |

**模型的连带影响**：`xiaomi` 移除后目录模型数从 1236 降为 1230，六个模型全部退出选择器。其中四个仍可由 `xiaomi-token-plan-cn` 提供；`mimo-v2.6-pro-ultraspeed` 仍能以 `xiaomi/mimo-v2.6-pro-ultraspeed` 从 `openrouter` 与 `vercel-ai-gateway` 使用；`mimo-v2.5-pro-ultraspeed` 在本构建中不再有任何内置路由提供，需要时以自定义模型 API 接入 `https://api.xiaomimimo.com/v1`。

**配置级影响**：拒绝发生在整份 `providers` 字典解析时。若同一 profile 同时配置了 `xiaomi` 与其他 pi-ai 路由，`llm-pi-ai` 插件整体加载失败，其他 pi-ai 路由要等该条目删除后才恢复；DeepSeek 两条原生路由由独立插件提供，不受影响。

**测试与 fixture 影响**：全仓扫描确认没有单元测试、录制会话、headless fixture 或 Web golden 以 `xiaomi` 作为场景路由；受影响的只有枚举供应商列表的 3 个 golden 与 1 处数量下限。`packages/client/ui-settings-models/tests/store.client.spec.ts` 用 `xiaomi-token-plan-cn` 作为 `deriveKeyRef` 示例，该路由仍在售，无需改动。

**推理配置不受影响**：`xiaomi` 的四个共有模型使用 `openai-completions`、`thinkingFormat: deepseek`，与 `xiaomi-token-plan-cn` 完全一致；保留路由的推理档位与报文格式不变。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm`（全 llm 包） | 78 个文件、1785 个测试全部通过（含两条 DeepSeek 原生路由） |
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、476 个测试全部通过（新增 1 个参数化用例） |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过 |
| `pnpm run test:docs` | 21 个门禁全部通过（含双语配对与文档预算） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件保持 42 个 id） |
| `pnpm run build` | 10 个构建门禁全部通过 |
| Web e2e（Chromium，`replay`） | 只跑受影响的两个文件：20 通过、2 失败；2 个失败仅因 WebKit 浏览器未安装（`webkit-2311/pw_run.sh` 缺失），与本次改动无关；3 个 golden 比对通过，未发生重写 |
| `npx vitest run packages/llm/llm-pi-ai --coverage`（仅计入 `catalog.ts`、`config.ts`） | 两文件语句/分支/函数/行均 100% |
| 目录自检（源码导入） | 29 个供应商、1230 个模型；`xiaomi` 不存在，`xiaomi-token-plan-cn` 仍有 4 个模型 |
| 上游定义核对 | pi-ai 1.3.0 中 `xiaomi` 为 `https://api.xiaomimimo.com/v1`、6 个模型；与 `xiaomi-token-plan-cn` 共有的 4 个模型仅 `provider`、`baseUrl`、`cost` 三处不同 |
| 文档一致性自检（逐文件比对 `WITHHELD_PROVIDERS`） | 7 个文档（用户指南 2、包 README 2、升级指南 2、汇总日志 1）均完整枚举 12 条路由，无过期计数 |
| golden 与目录一致性 | 三个 golden 的 option 列表与 `catalogProviderIds()` 逐项相等（29 个、同集合、字母序） |
| `oxlint` / `git diff --check` | 0 error（`catalog.ts` 的 `no-redundant-type-constituents` 警告为改动前既有）；空白检查通过 |

## 4. 设计取舍

- **只移除按量计费入口**：保留 `xiaomi-token-plan-cn`。用户要的是去掉 `xiaomi` 这条路由，而套餐路由是另一条订阅入口，二者凭据与端点互不通用；一并移除会顺带砍掉仍可用的中国区套餐。
- **只在 dsh 层移除**：不修改 `node_modules/@earendil-works/pi-ai`。`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 仍是仅有的两个控制点。
- **注释按厂商归组而非按移除顺序**：三条 Xiaomi 路由写在同一句里，读者一次就能看到「剩下哪一条、为什么」；若只追加一句 `xiaomi`，同一厂商的四种状态会散落在段落不同位置。
- **不迁移 golden 中的其他选项**：三个 golden 只删一行，保持选项字母序与格式不变，便于 review 时确认没有误删邻行。
- **只回放受影响的 Web 用例**：全量 Web lane 有 244 个 e2e 文件，且配置为 `fileParallelism: false` 串行执行（本次跑到 1 小时仍未结束，已中止）；`models-settings` 与 `onboarding-usable-provider` 两个文件覆盖了本次改动的全部 3 个 golden 与那处数量下限，因此只回放这两个文件。
- **保留生成元数据与测试桩**：`provider-data.generated.ts` 必须完整覆盖已安装目录（`verify-provider-data` 校验）；webworker 的 `BUILTIN_PROVIDER_IDS` 是「必须使用有效已安装 id」的预览子集桩，两者均不修改。

## 5. 复查记录（2026-11-15）

复查方式：重读 `catalog.ts` 的剔除集合与注释，全仓扫描 `xiaomi` 与 `XIAOMI` 的每一处列举，核对 pi-ai 上游对 `xiaomi` 的 auth / baseURL / 模型定义，比对它与 `xiaomi-token-plan-cn` 的模型字段差异，逐个 golden 选项与 `catalogProviderIds()` 对比，复算目录与模型数量，检查本机 profile 与凭据文件，最后重跑门禁。

发现 3 处问题，均已修复：

| # | 问题 | 修复 |
|---|---|---|
| 1 | 本日志与汇总日志称两个 UltraSpeed 模型「为该路由独有 / 不再有任何内置路由提供」，但上游目录中 `openrouter` 与 `vercel-ai-gateway` 各有一条 `xiaomi/mimo-v2.6-pro-ultraspeed`，只有 `mimo-v2.5-pro-ultraspeed` 真正失去全部内置路由 | 背景段改为「只在该路由下以裸模型 id 出现」并注明两条网关条目；§2 与 §5 区分两个模型的现状；汇总日志 `00009` 同步 |
| 2 | 汇总日志 `00009` 的 Web e2e 统计仍停留在上一轮的 22/24，与本轮实际回放（两个文件 22 个用例、20 通过）不一致 | 改为「本轮只回放受影响的 `models-settings` 与 `onboarding-usable-provider` 两个文件：20/22 通过」，并注明 3 个 golden 比对通过、2 个失败为 WebKit 未安装 |
| 3 | 设计取舍中把全量 Web lane 写成「865 个测试」，实际 `vitest list` 统计到 1072 个用例；「数小时」亦未实测 | 改为「244 个 e2e 文件、`fileParallelism: false` 串行，本次跑到 1 小时仍未结束已中止」 |

核对结论：

- 三个 golden 的 option 列表与 `catalogProviderIds()` 逐项相等（29 个、同集合、字母序），因此页面呈现的正是当前在售目录，没有漏删或误删相邻选项。
- `xiaomi` 在 pi-ai 中的 `auth` 只声明 `apiKey`（无 `oauth`），移除前它在选择器里也只能填 API Key，不存在「用户无法登录」的额外影响面。
- `xiaomi` 的 6 个模型全部使用 `openai-completions` 与 `thinkingFormat: deepseek`，与 `xiaomi-token-plan-cn` 的四个共有模型逐字段一致，差别只有 `provider`、`baseUrl` 与 `cost`；保留路由的推理行为不变。
- 目录数量自洽：42 个内置 id = 29 个提供 + `typesafe`（仅分类模型被 `servesChat` 过滤）+ 12 个移除。
- 全仓除测试参数、文档与日志外，`xiaomi` 只出现在 `provider-data.generated.ts`（必须覆盖已安装目录）与 webworker 的预览桩列表中，两处都刻意保留。
- 本机 `$DSH_HOME/profiles/web/cordis.patch.yml` 只配置 `opencode-go`，`.credentials.yaml` 无 `xiaomi` 引用，移除不会让现有安装加载失败。
- 覆盖率无回退：`catalog.ts` 与 `config.ts` 的语句、分支、函数、行均为 100%。

修复后重跑：`pnpm exec vitest run packages/llm/llm-pi-ai` 476 项通过、`pnpm run test:docs` 21 项通过、`pnpm run build:check` 2 项通过、`pnpm run test:snapshot -t pi-ai-conversation-updates` 1 项通过、`vitest --config vitest.expected.config.ts -t "pi-ai"` 2 项通过、`oxlint` 0 error、`git diff --check` 通过。

## 6. 已知限制与后续

- 当前 `127.0.0.1:3080` 的进程仍显示旧列表，重启后生效。
- `mimo-v2.5-pro-ultraspeed` 失去全部内置路由（`mimo-v2.6-pro-ultraspeed` 仍可由 `openrouter`、`vercel-ai-gateway` 提供）；如需按量计费直连，以自定义模型 API 接入 `https://api.xiaomimimo.com/v1`，Provider ID 不能复用被移除的 `xiaomi`。
- WebKit 浏览器未安装，两个键盘可访问性 e2e 未在 WebKit 下验证；Chromium 已覆盖 golden。
- 若将来恢复该路由，需要删除集合项、更新 withheld 测试参数、恢复 3 个 Web golden 与双语文档，并把数量下限改回 30，最后重新记录 i18n 哈希。

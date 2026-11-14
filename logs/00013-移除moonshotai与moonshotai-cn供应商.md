# 00013 — 移除 moonshotai 与 moonshotai-cn 供应商

- **日期**：2026-11-14
- **背景**：用户要求移除 `moonshotai`（Moonshot AI 国际站）与 `moonshotai-cn`（Moonshot AI 中国站）两条路由，且不能破坏项目现有功能。二者是同一 Kimi 开放平台的两个区域端点，模型、上下文与价格相同，只差 baseURL 与账号体系。本机 Web profile 只配置了 `opencode-go`，未点名这两条路由。
- **范围**：`packages/llm/llm-pi-ai` 的目录、测试、双语 README；Kimi 相关的单元测试与录制会话 fixture；Web 设置页与 onboarding golden；`docs/user/guide/providers.md` 与 `.zh.md`；既有升级指南；`packages/client/ui-settings-models` 的示例与测试；`logs/`。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `moonshotai` 与 `moonshotai-cn`。注释说明二者是同一个 Kimi 服务的国际站/中国站端点；本构建的 Kimi 入口改为 `kimi-coding`，开放平台端点仍可作为自定义模型 API 接入；并统一说明被移除路由都使用 OpenAI 兼容协议。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | withheld 回归测试扩展到八条路由，逐一断言目录、模型解析、可配置供应商列表均不含该路由。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 存量 profile 拒绝测试参数化加入两条路由，strict 与 deferred 均覆盖。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 更新为八条路由不提供，并说明 Kimi 仍可通过 `kimi-coding` 与自定义模型 API 使用；重新记录双语哈希。 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 提供商示例由 `moonshotai` 改为 `kimi-coding`；未提供路由段落加入两条 Moonshot 路由，替代路径补充 `api.moonshot.ai` / `api.moonshot.cn` 与 `kimi-coding`；重新记录双语哈希。 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.md` / `.zh.md` / `.i18n.yaml` | front matter、变更段与替代路径说明加入两条路由；重新记录双语哈希。 |
| `packages/llm/llm-pi-ai/tests/conversation-updates.spec.ts` | Kimi fixture 从 `moonshotai` / `kimi-k3` 改为 `kimi-coding` / `k3`。两条路由的 `openai-completions` compat 相同（`supportsMidConvoSystemMessages`、`supportsMidConvoToolAdditions`），断言的线上格式不变。 |
| `snapshots/session/pi-ai-conversation-updates/`（`cordis.yml`、`cordis.snapshot.yml`、`session.v4.jsonl`、`system-prompt.expected.md`） | 录制会话的场景路由同步改为 `kimi-coding` / `k3`，环境变量改为 `KIMI_API_KEY`，系统提示词中的模型名同步。 |
| `apps/cli/tests/profiles/headless/tests/fixtures/pi-ai-conversation-updates.patch.yml` 与 `headless.expected.e2e.ts` | 场景 patch 的路由、模型与 `apiKeyEnv` 同步改名，测试注入的环境变量改为 `KIMI_API_KEY`。 |
| `packages/client/ui-settings-models/README.md` / `.zh.md` / `.i18n.yaml` | 目录选择器示例由 `moonshotai` 改为 `kimi-coding`；重新记录双语哈希。 |
| `packages/client/ui-settings-models/tests/store.client.spec.ts` | `deriveKeyRef` 的参数化示例换成仍在售的 `kimi-coding` 与 `qwen-token-plan-cn`，保留连字符与多段 id 的覆盖。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除两条 option（可选项 35 → 33）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现两条路由，列表为 33 项 |
| 目录、模型解析、发现、登录 | 均不包含两条路由 |
| `catalogProvider('moonshotai')` / `catalogModels('moonshotai-cn')` | 返回 `undefined` / 空 |
| 存量配置点名任一被移除路由 | 明确报 `provider "<id>" is removed from this build; delete the route or select another provider` |
| `kimi-coding`（Kimi 编码套餐） | 不受影响，4 个模型保持可用 |
| 其他提供商与两条 DeepSeek 原生路由 | 不受影响 |

**配置级影响**：拒绝发生在整份 `providers` 字典解析时。若同一 profile 同时配置了这两条路由与其他 pi-ai 路由，`llm-pi-ai` 插件整体加载失败，其他 pi-ai 路由要等该条目删除后才恢复；DeepSeek 两条原生路由由独立插件提供，不受影响。

**测试与 fixture 影响**：`conversation-updates` 单元测试、`pi-ai-conversation-updates` 录制会话与 headless 期望用例原本以 `moonshotai` / `kimi-k3` 作为 Chat Completions 的 Kimi 代表。移除后这些路由会被 `resolveProfiles()` 拒绝，因此改挂 `kimi-coding` / `k3`：两者 compat 完全一致（除 `maxTokens` 由 1,048,576 变为 131,072，录制会话的 `request/context` 只记录 `contextWindow`），快照回放与 headless 期望用例在改名后原样通过；这两个用例断言的线上字段（消息角色、对话中系统消息的位置、工具增量的位置、系统提示词的更新方式）没有变化，`maxTokens` 的差异不在其断言范围内（`piAiTranscript` 只投影 `messages`/`tools`，录制会话只记录 `contextWindow`）。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run --config vitest.config.ts packages/llm/llm-pi-ai` | 22 个文件、472 个测试全部通过（新增 2 个参数化用例） |
| `npx vitest run --config vitest.config.ts packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过 |
| `pnpm run test:snapshot -t pi-ai-conversation-updates` | 1 个场景回放通过（fixture 改名后无差异） |
| `vitest --config vitest.expected.config.ts -t "pi-ai Kimi prompt and tool updates"` | 通过，内联快照未变 |
| `pnpm run build` | 10 个构建门禁全部通过 |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致 |
| `pnpm run test:docs` | 21 个文档门禁全部通过（含双语配对与文档预算） |
| Web e2e（Chromium，`replay`） | 20 通过、2 失败；2 个失败仅因 WebKit 浏览器未安装，与本次改动无关；3 个 golden 比对通过 |
| 目录自检 | 33 个供应商、1285 个模型；两条被移除路由均不存在 |
| `pnpm exec oxlint <改动文件>` | 0 warnings / 0 errors |
| `git diff --check` | 通过 |

## 4. 设计取舍

- **只在 dsh 层移除**：不修改 `node_modules/@earendil-works/pi-ai`。`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 仍是仅有的两个控制点。
- **两条一起移除**：`moonshotai` 与 `moonshotai-cn` 是同一服务的两个端点，模型与价格一致；保留其一会让“选国际站还是中国站”的问题继续存在，而 `kimi-coding` 已经提供 Kimi 入口。
- **fixture 改挂而非删除场景**：`pi-ai-conversation-updates` 验证的是 Chat Completions 下系统消息与工具增量在历史中的位置，不是某个厂商；改挂 compat 相同的 `kimi-coding` 保留覆盖，且录制会话与内联快照无需重录。
- **保留生成元数据与测试桩**：`provider-data.generated.ts` 必须完整覆盖已安装目录（`verify-provider-data` 校验）；webworker 的 provider 列表是打包/测试用桩；`packages/web/web-deepseek-api` 测试里的 `moonshotai` 是 web 搜索凭据解析的任意路由名，不经过 pi-ai 目录，均不修改。
- **不改历史升级指南**：`docs/upgrade-guide/v0.2.1-rc.2/pi-ai-model-catalog/` 记录的是已发布版本的目录内容，属冻结历史。

## 5. 复查记录（2026-11-14）

复查方式：重读 `catalog.ts` 的剔除集合与注释、全仓扫描 `moonshot` 的所有列举位置、核对历史同类任务（00005–00012）的改动清单、复算目录与模型数量、比对 golden 差异，并重跑构建与测试。

发现 3 处问题，均已修复：

| # | 问题 | 修复 |
|---|---|---|
| 1 | 初版只改目录与测试，`conversation-updates` 单元测试、`pi-ai-conversation-updates` 录制会话与 headless 期望用例仍点名 `moonshotai`，改动后会被 `resolveProfiles()` 拒绝 | 全部改挂 `kimi-coding` / `k3`，并把 `apiKeyEnv` 与注入的环境变量改为 `KIMI_API_KEY`；快照回放与期望用例均通过 |
| 2 | `docs/user/guide/providers.md` / `.zh.md` 的提供商示例仍以 `moonshotai` 代表 Kimi | 改为 `kimi-coding`，并重新记录 `providers.i18n.yaml` |
| 3 | `packages/client/ui-settings-models` 的 README 示例与 `deriveKeyRef` 测试仍使用被移除的 id | 示例改为 `kimi-coding`；测试改用 `kimi-coding` 与 `qwen-token-plan-cn`，保留多段 id 覆盖 |

第二轮复查（提交后按要求复核）发现 3 处表述问题，均已修复：

| # | 问题 | 修复 |
|---|---|---|
| 4 | 本节称 fixture 改名「说明线上行为未变」，但 `kimi-coding` 的 `k3` 与 `kimi-k3` 的 `maxTokens` 不同（1,048,576 → 131,072），只是不在断言范围内 | 改为按断言范围表述：`piAiTranscript` 只投影 `messages`/`tools`，录制会话只记录 `contextWindow`，被断言的字段无变化 |
| 5 | 用户指南与升级指南写「Kimi models remain offered through `kimi-coding`」，容易被读成同一批模型仍在 | 改为「Kimi 编码套餐（`kimi-coding`）仍然提供」，与目录事实一致 |
| 6 | 升级指南分号后误用大写；汇总日志称两条路由「各自移除」，实际为同批移除 | 修正大小写，并改为「分别移除 … 最后同批移除」 |
| 7 | 汇总日志的控制点表仍写 `provider-data.generated.ts` 保留「六个路由」的元数据 | 改为八个，并同步复查 `docs/` 下的供应商计数与「六条」表述，其余均已是最新 |

核对结论：`catalog.ts` 仍是本包唯一的 pi-ai 供应商枚举点，设置页目录、登录、路由解析、模型发现均经 `catalogProvider(s)` / `catalogProviderIds()` 取用，无旁路；`website/.dist` 与 `website/.generated` 被 `website/.gitignore` 忽略，属本地构建产物；三个 golden 删除前后分别为 35 与 33 项，回放无其他抖动。

## 6. 已知限制与后续

- 当前 `127.0.0.1:3080` 的进程仍显示旧列表，重启后生效。
- Kimi 开放平台（按 token 计费）在本构建的选择器中不再提供；需要时以自定义模型 API 接入 `https://api.moonshot.ai/v1` 或 `https://api.moonshot.cn/v1`，Provider ID 不能复用被移除的两个 id。
- WebKit 浏览器未安装，两个键盘可访问性 e2e 未在 WebKit 下验证；Chromium 已覆盖 golden。
- 若将来恢复这两条路由，需要删除集合项、更新 withheld 测试参数、恢复 3 个 Web golden 与双语文档，并把 fixture 改回（或补回）对应路由，最后重新记录 i18n 哈希。

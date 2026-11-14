# 00014 — 移除 qwen-token-plan 系列三条供应商

- **日期**：2026-11-15
- **背景**：用户要求移除 `qwen-token-plan`、`qwen-token-plan-cn`、`qwen-token-plan-individual` 三条路由，且不能破坏项目现有功能。三者是阿里云百炼 token 套餐的三个入口：`qwen-token-plan` 与 `qwen-token-plan-cn` 从新加坡、北京端点提供**逐字段相同的 20 个模型**，`qwen-token-plan-individual` 从新加坡端点提供其中 9 个（严格子集，无独有模型）。本机 Web profile 只配置了 `opencode-go`，未点名这三条路由。
- **范围**：`packages/llm/llm-pi-ai` 的目录、测试与双语 README；`packages/client/ui-settings-models` 的示例测试；Web 设置页与 onboarding golden；`docs/user/guide/providers.md` 与 `.zh.md`；既有升级指南；`logs/`。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | `WITHHELD_PROVIDERS` 增加三条路由。注释说明它们是同一个阿里云 token 套餐服务的三个入口：个人版从同一端点提供套餐 20 个模型中的 9 个，`-cn` 与 `qwen-token-plan` 仅端点区域不同，行为与已移除的 Xiaomi token 套餐端点同构；末句统一说明 Moonshot、Xiaomi、Qwen token 套餐路由都使用 OpenAI 兼容协议，需要时可作为自定义模型 API 接入。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | withheld 回归测试扩展到 11 条路由；顺带把 `expect(before).toBeGreaterThan(30)` 改为 `expect(before).toBe(catalogProviderIds().length + 1)`，让该断言随目录变化自洽而不是卡在魔数上。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 存量 profile 拒绝测试参数化加入三条路由，strict 与 deferred 均覆盖。 |
| `packages/llm/llm-pi-ai/tests/dynamic-config.spec.ts` | `expect(directory.length).toBeGreaterThan(30)` 改为 `expect(directory.length).toBe(catalogProviderIds().length)`，语义是「交付目录等于在售目录」，不再依赖具体数字。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 更新为 11 条路由不提供，并说明三条 Qwen 路由的端点与模型数量关系；重新记录双语哈希。 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由段落加入三条路由，替代路径补充 `token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1` 与 `token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1`（两端提供相同的 20 个模型，个人版开放 9 个）；重新记录双语哈希。 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.md` / `.zh.md` / `.i18n.yaml` | front matter、变更段与替代路径说明加入三条路由；重新记录双语哈希。 |
| `packages/client/ui-settings-models/tests/store.client.spec.ts` | `deriveKeyRef` 的参数化示例由 `qwen-token-plan-cn` 换成仍在售的 `xiaomi-token-plan-cn`，保留多段连字符 id 的覆盖。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除三条 option（可选项 33 → 30）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |
| `apps/web/tests/models-settings.e2e.ts` | 选择器的数量下限由 `> 30` 改为 `>= 30`：该断言只验证「休眠目录整体落入页面」，精确列表由 golden 负责。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现三条路由，列表为 30 项 |
| 目录、模型解析、发现、登录 | 均不包含三条路由 |
| `catalogProvider('qwen-token-plan')` / `catalogModels('qwen-token-plan-cn')` | 返回 `undefined` / 空 |
| 存量配置点名任一被移除路由 | 明确报 `provider "<id>" is removed from this build; delete the route or select another provider` |
| 其他提供商与两条 DeepSeek 原生路由 | 不受影响 |

**配置级影响**：拒绝发生在整份 `providers` 字典解析时。若同一 profile 同时配置了这些路由与其他 pi-ai 路由，`llm-pi-ai` 插件整体加载失败，其他 pi-ai 路由要等该条目删除后才恢复；DeepSeek 两条原生路由由独立插件提供，不受影响。

**测试与 fixture 影响**：全仓扫描确认没有任何单元测试、录制会话、headless fixture 或 Web golden 以这三条路由作为场景路由，因此本次不需要迁移快照，也没有需要重录的期望输出；受影响的只有枚举供应商列表的 3 个 golden 与 3 处数量断言。

**推理格式不受影响**：`catalog.ts` 的 `THINKING_FORMAT_GATE` 里的 `qwen` 与 `qwen-chat-template` 是**推理报文格式名**，不是供应商路由 id。移除目录路由后，指向 DashScope 的自定义模型 API 路由仍可在 profile 的 `compat` 里声明 `thinkingFormat: qwen`。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、475 个测试全部通过（新增 3 个参数化用例） |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过 |
| `pnpm run test:snapshot -t pi-ai-conversation-updates` | 1 个场景回放通过（未受影响） |
| `vitest --config vitest.expected.config.ts -t "pi-ai Kimi prompt and tool updates"` | 通过，内联快照未变 |
| `pnpm run build` | 10 个构建门禁全部通过 |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件保持 42 个 id） |
| `pnpm run test:docs` | 21 个文档门禁全部通过（含双语配对与文档预算） |
| Web e2e（Chromium，`replay`） | 22 通过、2 失败；2 个失败仅因 WebKit 浏览器未安装，与本次改动无关；3 个 golden 比对通过，未发生重写 |
| 目录自检（源码导入） | 30 个供应商、1236 个模型；三条被移除路由均不在列表、模型为空 |
| 上游定义核对 | pi-ai 1.3.0 中 plan / cn / individual 的模型数分别为 20 / 20 / 9，plan 与 cn 的 id 集合相同，individual 是 plan 的真子集 |
| 文档一致性自检（逐文件比对 `WITHHELD_PROVIDERS`） | 7 个文档（用户指南 2、包 README 2、升级指南 2、汇总日志 1）均完整枚举 11 条路由，且无 `33`/`1285` 过期计数 |
| `git diff --check` | 通过 |

## 4. 设计取舍

- **只在 dsh 层移除**：不修改 `node_modules/@earendil-works/pi-ai`。`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 仍是仅有的两个控制点。
- **三条一起移除**：它们描述同一个订阅服务的三个入口（国际站、中国站、个人版子集），保留任一条都会继续把「该选哪条」的问题留给用户；需要按 token 计费直连 DashScope 的部署可改用自定义模型 API。
- **顺带修掉脆弱断言**：`> 30` 这类魔数在供应商数量下降时会误报为功能回归；改为与 `catalogProviderIds().length` 对齐后，断言表达的是目录一致性和「休眠目录整体呈现」，与具体数量解耦。
- **改示例而非删断言**：`deriveKeyRef` 的测试用一条仍在售的 `xiaomi-token-plan-cn` 继续覆盖多段连字符 id 的派生规则，避免用被移除的 id 当示例。
- **保留生成元数据与测试桩**：`provider-data.generated.ts` 必须完整覆盖已安装目录（`verify-provider-data` 校验）；webworker 的 provider 列表是打包/测试用桩，均不修改。

## 5. 复查记录（2026-11-15）

复查方式：重读 `catalog.ts` 的剔除集合与注释，全仓扫描 `qwen-token-plan` 与 `QWEN_TOKEN_PLAN` 的每一处列举，核对 pi-ai 上游对三条路由的 auth / baseURL / 模型定义，比对 plan 与 cn 的模型字段差异，复算目录与模型数量，检查本机 profile 与凭据文件是否仍点名这三条路由，并逐条重跑门禁。

发现 5 处问题，均已修复：

| # | 问题 | 修复 |
|---|---|---|
| 1 | pi-ai README（英）写「a custom model API reaches any of the three under its own provider id」：自定义路由接的是端点，且 Provider ID 必须不同于被移除的 id；而且 individual 与 plan 共用新加坡端点，并没有三个端点 | 改为「both endpoints stay reachable through an OpenAI-compatible custom model API under a different provider id」 |
| 2 | pi-ai README（中）同样的表述写成「三者的任一端点」 | 改为「这两个端点仍可通过 OpenAI 兼容的自定义模型 API、以不同的 Provider ID 接入」 |
| 3 | `catalog.ts` 注释称 `-cn` 与 `qwen-token-plan`「only by the China endpoint」，并把 individual 一并说成「the way the Xiaomi token plans do」——individual 不是区域端点，而是套餐档位的模型子集 | 改写为「`qwen-token-plan` 与 `-cn` 从新加坡、北京端点提供相同的 20 个模型，individual 从新加坡端点开放其中 9 个，三者描述的是同一个订阅」 |
| 4 | 本日志正文把推理格式字段写成 `thinkingFormat: qwen`，实际它是 profile `compat` 下的字段 | 改为 `compat.thinkingFormat: qwen` |
| 5 | 汇总日志 `00009` 的 Web e2e 统计仍停留在上一轮的 20/22 | 改为 22/24（通过/总数），并注明 2 个失败仍为 WebKit 未安装 |

核对结论：

- 三条路由在 pi-ai 中的 `auth` 只声明 `apiKey`（无 `oauth`），因此移除前它们在选择器里也只能填 API Key，不存在「用户无法登录」的额外影响面。
- `qwen-token-plan` 与 `qwen-token-plan-cn` 的 20 个模型在 id、显示名、上下文、最大输出、输入模态、推理标记与价格上完全一致，确属同一套餐的两个区域端点。
- `qwen-token-plan-individual` 的 9 个模型全部是上述 20 个的子集，且共享 `QWEN_TOKEN_PLAN_API_KEY` 与新加坡端点，与 `qwen-token-plan` 的差别只有模型清单。
- 目录数量自洽：42 个内置 id = 30 个提供 + `typesafe`（仅分类模型被 `servesChat` 过滤）+ 11 个移除。
- 没有发现需要一并修改的旁路：设置页目录、登录、路由解析、模型发现均经 `catalogProvider(s)` / `catalogProviderIds()` 取用。
- 本机 `$DSH_HOME/profiles/web/cordis.patch.yml` 只配置 `opencode-go`，`.credentials.yaml` 也没有这三条路由的凭据引用，移除不会让现有安装加载失败。
- 三个 Web golden 删除前后分别为 33 与 30 项，且全仓只有这三个文件枚举供应商选择器。

修复后重跑：`pnpm exec vitest run packages/llm/llm-pi-ai` 475 项通过、`pnpm run test:docs` 21 项通过（README 双语哈希已重录）、`oxlint` 0 warning 0 error、`git diff --check` 通过，改动已并入同一提交。

## 6. 已知限制与后续

- 当前 `127.0.0.1:3080` 的进程仍显示旧列表，重启后生效。
- 阿里云 token 套餐在本构建中不再作为内置路由提供；需要时以自定义模型 API 接入上述 `compatible-mode/v1` 端点，Provider ID 不能复用被移除的三个 id。
- WebKit 浏览器未安装，两个键盘可访问性 e2e 未在 WebKit 下验证；Chromium 已覆盖 golden。
- 若将来恢复这三条路由，需要删除集合项、更新 withheld 测试参数、恢复 3 个 Web golden 与双语文档，并把数量断言改回固定下限，最后重新记录 i18n 哈希。

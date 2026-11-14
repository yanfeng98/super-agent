# 00012 — 移除 xiaomi-token-plan-sgp 供应商

- **日期**：2026-11-14
- **背景**：用户要求移除 `xiaomi-token-plan-sgp`（小米 MiMo Token 套餐的新加坡区域接入点），且不能影响其他供应商与 DeepSeek 路由。本机 Web profile 只配置了 `opencode-go`，未点名该路由。
- **范围**：`packages/llm/llm-pi-ai` 的目录、测试、双语 README；Web 设置页与 onboarding golden；`docs/user/guide/providers.md` 与 `.zh.md`；既有升级指南；`logs/`。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `xiaomi-token-plan-sgp`。注释改写为 sgp 与 ams 都是 `xiaomi-token-plan-cn` 的区域重复项：三者模型相同，仅端点区域不同；两者使用 OpenAI 兼容协议，需要时可作为自定义模型 API 接入。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | withheld 回归测试扩展到六条路由，逐一断言目录、模型解析、可配置供应商列表均不含该路由。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 存量 profile 拒绝测试参数化加入 `xiaomi-token-plan-sgp`，strict 与 deferred 均覆盖。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 更新为六条路由均不提供，并说明 sgp/ams 端点与 cn 套餐模型一致；重新记录双语哈希。 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 第三方提供商说明加入 Xiaomi Token Plan SGP，并改为说明 SGP 与 AMS 端点均可作为自定义模型 API 接入；重新记录双语哈希。 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.md` / `.zh.md` / `.i18n.yaml` | 升级指南的 front matter、变更段与迁移说明加入第六条路由；重新记录双语哈希。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除 `option "xiaomi-token-plan-sgp"`（可选项 36 → 35）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现 `xiaomi-token-plan-sgp`，列表为 35 项 |
| 目录、模型解析、发现、登录 | 均不包含该路由 |
| `catalogProvider('xiaomi-token-plan-sgp')` / `catalogModels(...)` | 返回 `undefined` / 空 |
| 存量配置点名该路由 | 明确报 `provider "xiaomi-token-plan-sgp" is removed from this build; delete the route or select another provider` |
| `xiaomi`（按量计费） | 不受影响，6 个模型保持可用 |
| `xiaomi-token-plan-cn` | 不受影响，4 个模型保持可用 |
| `xiaomi-token-plan-ams` | 此前已被移除，不受影响 |
| 其他提供商与两条 DeepSeek 原生路由 | 不受影响 |

**配置级影响**：拒绝发生在整份 `providers` 字典解析时。若同一 profile 同时配置了该路由与其他 pi-ai 路由，`llm-pi-ai` 插件整体加载失败，其他 pi-ai 路由要等该条目删除后才恢复；DeepSeek 两条原生路由由独立插件提供，不受影响。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run --config vitest.config.ts packages/llm/llm-pi-ai` | 22 个文件、470 个测试全部通过 |
| `pnpm run build` | 10 个构建门禁全部通过（宿主/客户端编译、聚合检查、Web 构建） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致 |
| `pnpm run test:docs` | 21 个文档门禁全部通过（复查修复后重跑） |
| `pnpm run verify-doc-budgets` | 7 个受预算文档均在限内 |
| Web e2e（Chromium，`DSH_SNAPSHOT=refresh` 后 `replay`） | 20/22 通过，3 个 golden 比对通过；2 个失败仅因 WebKit 浏览器未安装，与本次改动无关 |
| 目录自检 | 35 个供应商、1293 个模型；`xiaomi-token-plan-sgp` 不存在 |
| `pnpm exec oxlint <改动文件>` | 0 warnings / 0 errors |
| `git diff --check` | 通过 |

## 4. 设计取舍

- **只在 dsh 层移除**：不修改 `node_modules/@earendil-works/pi-ai`。`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 仍是仅有的两个控制点。
- **保留生成元数据**：`provider-data.generated.ts` 与 webworker 的 pi-ai stub 仍包含该路由，因为前者必须完整覆盖已安装目录（`verify-provider-data` 校验），后者是测试用桩。
- **只移除 SGP 区域**：`xiaomi` 与 `xiaomi-token-plan-cn` 保持不变，大陆用户仍可直接使用 cn 端点。
- **给出替代路径**：文档说明该端点仍是 OpenAI 兼容接口，需要时可作为自定义模型 API 接入，避免移除后无路可走。

## 5. 复查记录（2026-11-14）

复查方式：重读 `catalog.ts` 的剔除集合与注释、全仓扫描被剔除路由的所有列举位置、核对历史同类任务（00005–00010）的改动清单、复算目录与模型数量、比对 golden 差异，并重跑构建与测试。

发现 2 处问题，均已修复：

| # | 问题 | 修复 |
|---|---|---|
| 1 | `docs/user/guide/providers.md` / `.zh.md` 仍把 SGP 列为可用路由（历史任务 00010 同期更新了该文件，本次初版遗漏），用户指南与目录事实矛盾 | 两条被移除路由改为 SGP 与 AMS 并列，替代路径同步改写，并重新记录 `providers.i18n.yaml` |
| 2 | `logs/00009-模型供应商清理汇总.md` 仍记录 36 个供应商、1297 个模型、五条被移除路由 | 更新为 35 / 1293 / 六条，并补入本任务条目 |
| 3 | 三处文档称「可用自定义模型 API 接入」，但 `resolveProfiles()` 对任何同名 key 都拒绝，包括手工声明的路由；照文档操作会得到 `is removed from this build` 而非成功 | README、用户指南与升级指南补充「自定义路由需使用不同的 Provider ID，被移除的 ID 仍会被拒绝」 |

核对结论：`catalog.ts` 仍是本包唯一的 pi-ai 供应商枚举点，设置页目录、登录、路由解析、模型发现均经 `catalogProvider(s)` / `catalogProviderIds()` 取用，无旁路；`website/.dist` 与 `website/.generated` 被 `website/.gitignore` 忽略，属本地构建产物，无需同步；`packages/experimental/webworker-runtime` 的 provider 列表是打包/测试用桩，不参与产品目录；三个 golden 删除前后分别为 36 与 35 项，刷新后 `git diff` 只有 3 行删除，无其他抖动。

## 6. 已知限制与后续

- 当前 `127.0.0.1:3080` 的进程仍显示旧列表，重启后生效。
- WebKit 浏览器未安装，两个键盘可访问性 e2e 未在 WebKit 下验证；Chromium 已覆盖 golden。
- 若将来恢复该路由，需要删除集合项、更新 withheld 测试参数、恢复 3 个 Web golden 与双语文档，并重新记录 i18n 哈希。

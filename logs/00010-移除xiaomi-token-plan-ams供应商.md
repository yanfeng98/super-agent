# 00010 — 移除 xiaomi-token-plan-ams 供应商

- **日期**：2026-11-13
- **背景**：用户要求移除 `xiaomi-token-plan-ams`（小米 MiMo Token 套餐的阿姆斯特丹区域接入点），且不能影响其他供应商与 DeepSeek 路由。本机 Web profile 未配置该路由。
- **范围**：`packages/llm/llm-pi-ai` 的目录、测试、双语 README；Web 设置页与 onboarding golden；`docs/user/guide/providers.md` 与 `.zh.md`；既有升级指南。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `xiaomi-token-plan-ams`。注释说明它是两条保留的 Xiaomi token-plan 路由的区域重复项：三者模型相同，仅端点区域不同；该端点使用 OpenAI 兼容协议，需要时可作为自定义模型 API 接入。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | withheld 回归测试扩展到五条路由，逐一断言目录、模型解析、可配置供应商列表均不含该路由。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 存量 profile 拒绝测试参数化加入 `xiaomi-token-plan-ams`，strict 与 deferred 均覆盖。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 更新为五条路由均不提供，并说明 AMS 端点与 cn/sgp 套餐模型一致；重新记录双语哈希。 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 第三方提供商说明加入 Xiaomi Token Plan AMS，补充可用自定义模型 API 接入的替代路径；重新记录双语哈希。 |
| `docs/upgrade-guide/v0.2.2-rc.2/llm-pi-ai-removed-providers/guide.md` / `.zh.md` / `.i18n.yaml` | 升级指南的 front matter、变更段与迁移说明加入第五条路由；重新记录双语哈希。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除 `option "xiaomi-token-plan-ams"`（可选项 37 → 36）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现 `xiaomi-token-plan-ams`，列表为 36 项 |
| 目录、模型解析、发现、登录 | 均不包含该路由 |
| `catalogProvider('xiaomi-token-plan-ams')` / `catalogModels(...)` | 返回 `undefined` / 空 |
| 存量配置点名该路由 | 明确报 `provider "xiaomi-token-plan-ams" is removed from this build; delete the route or select another provider` |
| `xiaomi`（按量计费） | 不受影响，6 个模型保持可用 |
| `xiaomi-token-plan-cn` / `xiaomi-token-plan-sgp` | 不受影响，各 4 个模型保持可用 |
| 其他提供商与两条 DeepSeek 原生路由 | 不受影响 |

**配置级影响**：拒绝发生在整份 `providers` 字典解析时。若同一 profile 同时配置了该路由与其他 pi-ai 路由，`llm-pi-ai` 插件整体加载失败，其他 pi-ai 路由要等该条目删除后才恢复；DeepSeek 两条原生路由由独立插件提供，不受影响。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm exec vitest run packages/llm/llm-pi-ai` | 22 个文件、469 个测试全部通过（新增 1 个参数化用例） |
| `pnpm exec tsc -b packages/llm/llm-pi-ai/tsconfig.json` | 退出码 0 |
| `pnpm exec tsdown --config-loader native --env.DSH_BUILD_FACE host` | 重建 host 产物成功 |
| `pnpm run build:check` | host/client 两个聚合检查通过 |
| `pnpm run test:docs` | 21 个文档门禁全部通过 |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致 |
| 目录自检 | 36 个供应商、1297 个模型；`xiaomi-token-plan-ams` 不存在 |
| 覆盖率（`--coverage.reporter=json-summary`） | `catalog.ts`、`config.ts` 语句/分支/函数/行均 100% |
| `pnpm exec tsx scripts/run-oxlint.ts <改动文件>` | 0 warnings / 0 errors |
| Web e2e（Chromium） | 20/22 通过，3 个 golden 比对通过；剩余 2 个 WebKit 用例因浏览器未安装失败，与本次改动无关 |
| `git diff --check` | 通过 |

## 4. 设计取舍

- **只在 dsh 层移除**：不修改 `node_modules/@earendil-works/pi-ai`。`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 仍是仅有的两个控制点。
- **保留生成元数据**：`provider-data.generated.ts` 与 webworker 的 pi-ai stub 仍包含该路由，因为前者必须完整覆盖已安装目录（`verify-provider-data` 校验），后者是测试用桩。
- **只移除 AMS 区域**：`xiaomi` 与 `xiaomi-token-plan-cn`、`xiaomi-token-plan-sgp` 保持不变。三条 token-plan 路由模型相同，AMS 是其中唯一面向欧洲订阅的端点。
- **给出替代路径**：文档说明该端点仍是 OpenAI 兼容接口，需要时可作为自定义模型 API 接入，避免移除后无路可走。

## 5. 复查要点

1. 目录与设置页一致：golden 删除后 Chromium e2e 直接比对通过，未出现 37 → 36 之外的差异。
2. 存量配置：`resolveProfiles()` 的拒绝在 strict 与 deferred 下均有测试；不会以「声明的路由」复活。
3. 文档无残留：双语 README、用户指南与升级指南均同步为五条；repository references 与 markdown wrap 门禁通过。
4. 双语哈希：三个文件对修改后均重新记录，`test:docs` 全量通过。
5. 测试与覆盖率：469 个测试通过，改动文件覆盖率 100%。

## 6. 复查记录（2026-11-14）

复查方式：审计 `builtinProviders()` 是否仍是唯一枚举入口、全仓扫描残留引用、对照上游 pi-ai 元数据核对事实性陈述、比对 golden 选项计数、复核双语哈希，并手工验证存量配置的拒绝行为与 cn/sgp 的可用性。

发现 4 处问题，均为文档与注释的准确性问题，无功能缺陷：

| # | 问题 | 修复 |
|---|---|---|
| 1 | `logs/00009` 仍记录 468 个测试与「新增升级指南」，与当前的 469 个测试、已扩展为五条路由的指南不符 | 更新为 469 与「覆盖五条被移除路由」 |
| 2 | `catalog.ts` 注释用 "the two remaining Xiaomi token-plan routes" 指代保留路由，将来再删任一条即失真 | 改为显式点名 `xiaomi-token-plan-cn` 与 `xiaomi-token-plan-sgp` |
| 3 | 升级指南英文句以嵌套破折号承载从句，单句过长 | 拆为两句，AMS 端点单独成句 |
| 4 | 注释折行宽度不齐（79–87 列） | 重新折行至 80 列以内 |

复查中同时为用户指南补注路由区域（`xiaomi-token-plan-ams`，阿姆斯特丹），避免读者仅凭 `ams` 缩写无法判断端点归属；双语配对已重新记录。

核对结论：`catalog.ts` 是本包唯一的 pi-ai 供应商枚举点，设置页目录、登录、路由解析、模型发现均经 `catalogProvider(s)` / `catalogProviderIds()` 取用，无旁路；上游 pi-ai README 确认 AMS 指阿姆斯特丹；三份 provider JSON 逐字段比对显示 ams/cn/sgp 的 4 个模型除 `provider` 与 `baseUrl` 外完全一致，文档「同模型、仅端点不同」成立；三个 golden 删除前后分别为 37 与 36 项；i18n 哈希仅被改章节发生变化。

复查后重新执行：469 个测试、`tsc -b`、host 产物重建 + `build:check` 2/2、`test:docs` 21/21、`verify-provider-data`、改动文件覆盖率 100%、oxlint 0/0、`git diff --check`，全部通过。

## 7. 已知限制与后续

- 当前 `127.0.0.1:3080` 的进程仍显示旧列表，重启后生效。
- WebKit 浏览器未安装，两个键盘可访问性 e2e 未在 WebKit 下验证；Chromium 已覆盖 golden。
- 若将来恢复该路由，需要删除集合项、更新 withheld 测试参数、恢复 3 个 Web golden 与双语文档，并重新记录 i18n 哈希。

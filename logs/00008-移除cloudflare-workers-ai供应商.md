# 00008 — 移除 cloudflare-workers-ai 供应商

- **日期**：2026-11-13
- **背景**：用户要求在移除 `cloudflare-ai-gateway` 之后，再移除 `cloudflare-workers-ai`（Cloudflare Workers AI 直连接入），且不能影响其他供应商与 DeepSeek 路由。本机 Web profile 未配置该路由。
- **范围**：`packages/llm/llm-pi-ai` 的目录、配置、测试、双语 README；Web 设置页与 onboarding golden；`docs/user/guide/providers.md` 与 `.zh.md`。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `cloudflare-workers-ai`，四条被移除路由改为逐行列出，并更新注释说明认证原因。`catalogProviders()` 在 `servesChat` 前过滤，目录、模型解析、发现、登录随之不再包含该路由。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | withheld 回归测试覆盖 `amazon-bedrock`、`azure`、`cloudflare-ai-gateway`、`cloudflare-workers-ai` 四条路由的全部目录表面。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 存量 profile 拒绝测试参数化加入 `cloudflare-workers-ai`，strict 与 deferred 均覆盖。 |
| `packages/llm/llm-pi-ai/tests/loader-composition.spec.ts` | 目录元数据断言从被移除的路由切换到仍提供的 `google-vertex`，继续验证 `nativeAuth` 与 `baseURLEnv` 的暴露。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 与 Dev Note 更新为四条路由均不提供；重新记录双语哈希。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除 `option "cloudflare-workers-ai"`（可选项 38 → 37）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 第三方提供商说明加入 Cloudflare Workers AI，两条 Cloudflare 路由均不再提供。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现 `cloudflare-workers-ai`，列表为 37 项 |
| 目录、模型解析、发现、登录 | 均不包含该路由 |
| `catalogProvider('cloudflare-workers-ai')` / `catalogModels(...)` | 返回 `undefined` / 空 |
| 存量配置点名该路由 | 明确报 `provider "cloudflare-workers-ai" is removed from this build; delete the route or select another provider` |
| `google-vertex`（仍提供） | 不受影响，目录条目与 14 个模型保持可用 |
| 未配置被移除路由时的其他供应商 | 不受影响 |

**配置级影响**：拒绝发生在整份 `providers` 字典解析时。若同一 profile 同时配置了该路由与其他 pi-ai 路由，`llm-pi-ai` 插件整体加载失败，其他 pi-ai 路由要等该条目删除后才恢复；DeepSeek 两条原生路由由独立插件提供，不受影响。这是有意的“加载时明确失败”。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm exec vitest run packages/llm/llm-pi-ai` | 22 个文件、468 个测试全部通过 |
| `pnpm exec tsc -b packages/llm/llm-pi-ai/tsconfig.json` | 退出码 0 |
| `pnpm exec tsdown --config-loader native --env.DSH_BUILD_FACE host` | 重建 host 产物成功 |
| `pnpm run build:check` | host/client 两个聚合检查通过 |
| `pnpm run test:docs` | 21 个文档门禁全部通过 |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致 |
| 目录自检 | 37 个供应商、1301 个模型；两条 Cloudflare 路由均不存在 |
| 目标文件覆盖率 | `catalog.ts`、`config.ts` 语句/分支/函数/行均 100% |
| `pnpm exec tsx scripts/run-oxlint.ts <改动文件>` | 0 warnings / 0 errors |
| Web e2e（Chromium） | 20/22 通过，3 个 golden 比对通过；剩余 2 个 WebKit 用例因浏览器未安装失败，与本次改动无关 |
| `git diff --check` | 通过 |

## 4. 设计取舍

- **只在 dsh 层移除**：不修改 `node_modules/@earendil-works/pi-ai`。`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 是仅有的两个控制点。
- **保留生成元数据**：`provider-data.generated.ts` 仍包含四条被移除路由的环境变量与端点提示，因为该文件必须完整覆盖已安装 pi-ai 目录，`verify-provider-data` 会校验。
- **Cloudflare 相关路由全部移除**：`cloudflare-ai-gateway` 与 `cloudflare-workers-ai` 均不再提供；需要 Cloudflare 承载的开源模型时，可用其 OpenAI 兼容端点自行添加自定义路由。

## 5. 复查要点

1. 目录与设置页一致：golden 删除后 Chromium e2e 直接比对通过，未出现 38 → 37 之外的差异。
2. 存量配置：`resolveProfiles()` 的拒绝在 strict 与 deferred 下均有测试；不会以“声明的路由”复活。
3. 文档无残留：README 与用户指南均不再声明 Workers AI 可用；Cloudflare 仅出现在被移除路由列表中。
4. 双语哈希：两个文件对修改后均重新记录，`test:docs` 全量通过。
5. 测试与覆盖率：468 个测试通过，改动文件覆盖率 100%。

## 6. 已知限制与后续

- 当前 `127.0.0.1:3080` 的进程仍显示旧列表，重启后生效。
- WebKit 浏览器未安装，两个键盘可访问性 e2e 未在 WebKit 下验证；Chromium 已覆盖 golden。
- 若将来恢复该路由，需要删除集合项、更新 withheld 测试参数、恢复 3 个 Web golden 与双语 README，并重新记录 i18n 哈希。

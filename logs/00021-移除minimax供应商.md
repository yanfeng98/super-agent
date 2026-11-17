# 00021 — 移除 minimax 供应商

- **日期**：2026-11-15
- **背景**：MiniMax 在 pi-ai 目录里占两条路由：`minimax` 是国际站开放平台（`https://api.minimax.io/anthropic`，密钥 `MINIMAX_API_KEY`），`minimax-cn` 是中文站开放平台（`https://api.minimaxi.com/anthropic`，密钥 `MINIMAX_CN_API_KEY`）。两者提供完全相同的三个模型且能力、价格逐字段一致，属于同一家厂商的两套账号与结算体系。本仓库按「同一厂商只保留一条可售路由」的既有做法（00013 Moonshot、00014 Qwen、00020 OpenCode）移除国际站 `minimax`，保留中文站 `minimax-cn`。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点、测试）、`docs/user/guide/providers.md`、`packages/llm/llm-pi-ai/README.md`、升级指南、Web golden 与 `logs/`。

- **后续变更（00036）**：`openrouter` 已于 [00036](00036-移除openrouter供应商.md) 移除，本记录中以 `openrouter` 作为承接路由的说明随之失效；三个模型仍全部由 `minimax-cn` 承接（3/3），表中「另见 `openrouter`、`together`」的承接说明失效。现行覆盖见 00036 第 2 节与 [00009](00009-模型供应商清理汇总.md) 顶部的「后续变更（00036）」。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `minimax`（16 → 17 条），注释补充地区拆分理由与自定义路由替代 |
| `tests/catalog.spec.ts` | 「withholds removed catalog routes」用例的被移除路由清单增加 `minimax` |
| `tests/config.spec.ts` | 参数化的被移除路由用例增加 `minimax`（strict 与 deferred 各一次） |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单增加 `minimax`；补三段式说明（同三模型、`minimax-cn` 承接、`api.minimax.io/anthropic` 自定义路由）；结尾「仍然提供」清单增加 `minimax-cn`；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的被移除路由清单与说明同步，重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述与变更清单加入 `minimax`，补地区拆分与重新接入方式；英文正文 475 词 |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "minimax"`（25 → 24 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 25 → 24 |

## 2. 行为

- 可对话供应商 25 → **24**，目录模型 765 → **762**（只减 `minimax` 的 3 个模型）。
- `catalogProvider('minimax')` 返回 `undefined`，`catalogModels('minimax')` 为空；`minimax-cn` 仍提供 `MiniMax-M2.7`、`MiniMax-M2.7-highspeed`、`MiniMax-M3` 三个模型。
- 存量 profile 点名 `minimax` 时 `resolveProfiles()` 在 strict 与 deferred 下都抛 `llm-pi-ai: provider "minimax" is removed from this build; delete the route or select another provider`，同一 `providers` 字典里的其他路由随之不可用，删除该条目即恢复。
- 文档与 golden 中被移除路由统一为十七条。
- `provider-data.generated.ts` 仍保留 42 个 id 的元数据（`verify-provider-data` 要求与已安装目录一致），它不是可售路由。

## 3. 模型覆盖

| 原模型 | `minimax-cn` 同 id | 其他内置路由 |
|---|---|---|
| `MiniMax-M2.7` | 是 | `openrouter`（`minimax/minimax-m2.7`）、`together`（`MiniMaxAI/MiniMax-M2.7`） |
| `MiniMax-M2.7-highspeed` | 是 | 无（只存在于两条 MiniMax 路由） |
| `MiniMax-M3` | 是 | `fireworks`（`accounts/fireworks/models/minimax-m3`）、`opencode-go`、`openrouter`、`together` |

因此本次移除不减少任何模型：三个模型全部在 `minimax-cn` 上以相同 id、相同上下文与目录价格保留；只有国际站这一条服务面消失。若部署只有国际站账号，可用 `anthropic-messages` 自定义模型 API 接入 `https://api.minimax.io/anthropic`（Provider ID 需避开 `minimax`）。

## 4. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/llm/llm-pi-ai` | 22 个文件、**481** 个测试全部通过（移除前 480，`config.spec` 参数化多一条） |
| `npx vitest run packages/llm` | 78 个文件、**1790** 个测试全部通过（移除前 1789） |
| `npx vitest run packages/client/ui-settings-models` | 14 个文件、**483** 个测试全部通过（用例均使用 `minimax-cn`，无需改动） |
| `pnpm run test:docs` | **21/21** 通过（含 `upgrade guides`、`translation pairing`、`markdown wrap`、`markdown links`） |
| `pnpm run doc-sync` | 42/43；唯一失败是 `Cordis inspect catalog`，报 `packages/extensions/cordis-client-runner/src/client/api-catalog.ts is stale`，来源是并行开发中的 `slot-catalog.ts`，与本次移除无关 |
| `pnpm run verify-provider-data` | 通过：`Provider metadata matches the installed pi-ai catalog.` |
| `npx tsc -b packages/llm/llm-pi-ai/tsconfig.json` | 通过（0 错误） |
| `pnpm run typecheck` | **5/5** 通过（最终轮；中间态曾因并行包的 `packages/client/ui-boot-animation/tests/harness.host.ts:193` `TS2322` 失败一次，该包随后自行修复） |
| `pnpm run lint` | **0 warning、0 error**（90 条规则、7279 个文件；中间态曾出现并行包的 2 处 `unbound-method`，该包随后自行修复） |
| `pnpm run build` | **10/10** 通过（29.27s，含 host/client 聚合检查与 Web build） |
| 目录自检 | 24 个供应商、762 个模型；`minimax` 不在可配置列表、无 provider、无模型；`minimax-cn` 3 个模型；strict/deferred 均报移除错误 |
| golden 与目录一致性 | 三个 golden 的 option 列表与 `catalogProviderIds()` 逐项相等（24 个、同顺序） |
| Web e2e（`DSH_SNAPSHOT=replay`） | 2 个文件 5 通过、17 失败，失败全部来自并行开发中的 `ui-boot-animation`：其全窗口启动视频 `<video class="nVEbha_video">` 拦截指针事件导致点击超时，且设置对话框导航新增 `button "启动片头"`。`native-keyless` 与 `onboarding/dismissed` 两个 golden 完成了真实比对，差异只有该按钮一行，供应商 option 列表逐项一致（真实页面显示 24 项） |

**最终状态**：上表的数字在最后一轮 amend 后重跑仍成立；本次移除涉及的文件已全部提交且工作区干净，其余改动属于并行开发的 `ui-boot-animation`。

## 5. 复查记录

- 首轮文档比对发现升级指南增量会把英文正文推过 500 词硬上限（460 + 31 ≈ 491），同时超出仓库 475 词的实践上限。处理：同时在 `huggingface` 句、Migration 第 2 步与第 4 步做等义收紧，阶段性 **475 词**（后续复核又压到 474 词，见本节第 5 条）。
- 自检 `providers.md` 的中英对照时发现英文「仍然提供」清单漏掉 `minimax-cn`（中文版已补），已补齐，使两边枚举一致。
- 中文升级指南初稿写成「其中文站 `minimax-cn` 提供同样的三个模型并仍然提供」，出现「提供…提供」重叠，已改为「仍然提供同样的三个模型」。
- e2e 首轮 17 失败曾被怀疑与本次 golden 改动有关；逐条查看差异后确认全部由并行包的启动视频覆盖层与新增设置按钮造成，golden 侧只有 `option "minimax"` 一处差异，与目录一致。
- 复核升级指南时发现该句只说「可用 Anthropic Messages 重新接入」而未给出端点，与其他被移除路由的写法不一致（都带端点），已补 `https://api.minimax.io/anthropic`；为守住 475 词实践上限，同时把 `huggingface` 句的「13 on no other route」收为「13 nowhere else」、「78 Hub model names」收为「78 Hub names」，最终 **474 词**（中途一版把 Qwen 句改成「twenty identical models」，读起来像「二十个彼此相同的模型」，已改回「the same twenty models」）。
- 复核价格表述时发现「价格完全一致」会被读成账单金额一致：两站的目录 cost 字段逐字段相同，但结算币种不同（国际站 USD、中文站 CNY），已把文档与注释统一改为「目录价格 / catalog costs」，避免读者按发票金额理解。

## 6. 已知限制

- `MiniMax-M2.7-highspeed` 移除后只剩 `minimax-cn` 一条内置路由；若部署只能访问国际站，需以自定义模型 API 接入 `https://api.minimax.io/anthropic`。
- 目录数据来自上游 pi-ai 1.3.0，仍未收录官方在售的 `MiniMax-M3.1-Flash`；两条 MiniMax 路由都受影响，与本次移除无关。
- 中文站官方文档已把 Anthropic 兼容端点写作 `https://api.minimax.cn/anthropic`，而目录沿用旧域名 `api.minimaxi.com/anthropic`；两个域名当前都可用，需要新域名时可在路由上覆盖 `baseURL`。
- 运行中的 `127.0.0.1:3080` 需重启后才会显示 24 项。

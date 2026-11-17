# 00034 — 移除 google-vertex 供应商

- **日期**：2026-11-17
- **编号**：本轮的 `00033` 已被并行的 Matrix/Office 移除日志占用，本条日志顺延为 `00034`。
- **背景**：`google-vertex` 是 pi-ai 目录里的 Google Cloud Vertex AI 路由，与 `google` 同属 Gemini 模型，但入口完全不同：`google` 走 Gemini Developer API（`https://generativelanguage.googleapis.com/v1beta`，`google-generative-ai` 协议，只认 `GEMINI_API_KEY`），`google-vertex` 走 Google Cloud（端点模板 `https://{location}-aiplatform.googleapis.com`，`google-vertex` 协议），认证可以是一把 Cloud API key（`GOOGLE_CLOUD_API_KEY`，设置页存下的密钥也走这条路径，`google-vertex` 声明了 `apiKey` 方法），也可以是 ADC（`GOOGLE_APPLICATION_CREDENTIALS` 或 `~/.config/gcloud/application_default_credentials.json`），后者还要求 `GOOGLE_CLOUD_PROJECT`（或 `GCLOUD_PROJECT`）与 `GOOGLE_CLOUD_LOCATION`。它的 14 个模型全部是 Gemini 主线：`gemini-2.5-flash`、`gemini-2.5-flash-lite`、`gemini-2.5-pro`、`gemini-3-flash-preview`、`gemini-3.1-flash-lite`、`gemini-3.1-flash-lite-image`、`gemini-3.1-pro-preview`、`gemini-3.1-pro-preview-customtools`、`gemini-3.5-flash`、`gemini-3.5-flash-lite`、`gemini-3.6-flash`、`gemini-3.7-flash`、`gemini-3.8-flash`、`gemini-4-argon`。
- **判断依据**：这与 `together`、`minimax` 一类"同一批模型换一个入口"的情形同构。14 个 id 全部在 `google` 上以相同 id、相同上下文窗口与输入模态提供（`gemini-4-argon` 的输出上限、`gemini-3.1-flash-lite-image` 的缓存价与三个条目的阶梯价不同），`google` 另外还有 9 个 Vertex 没有的条目（两个 Deep Research 预览、`gemini-2.5-computer-use-preview-10-2025`、`gemini-3.1-flash-lite-preview`、`gemini-3.1-flash-live-preview`、`gemini-3.1-flash-lite-image` 之外的 `gemini-flash-latest` 与 `gemini-flash-lite-latest` 两个滚动别名，以及 `gemma-4-26b-a4b-it`、`gemma-4-31b-it`），因此移除它拿掉的是一条托管入口而非模型覆盖。用户要求移除该供应商且不能破坏现有功能，沿用既有的 `WITHHELD_PROVIDERS` 机制：目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由，不修改 `node_modules` 里的 pi-ai 包。
- **范围**：`packages/llm/llm-pi-ai`（目录控制点与测试）、`docs/user/guide/providers.md`、包 README、升级指南、Web golden 与 e2e 断言、根 README、`logs/`。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/catalog.ts` | `WITHHELD_PROVIDERS` 增加 `google-vertex`（26 → 27 条，字母序放在 `fireworks` 之后）；注释新增 `google-vertex` 段，说明它是 Google Cloud 的 Vertex AI 端点而非 `google` 的 Gemini API 端点、ADC 路径需要项目与位置、14 个条目在 `google` 上同 id 且上下文与输入模态一致（并写明输出上限、缓存价与阶梯价三处字段差异），最后补一句 `google-vertex` 协议不可作为自定义协议选择 |
| `tests/catalog.spec.ts` | 「withholds removed catalog routes」用例的被移除路由清单增加 `google-vertex` |
| `tests/config.spec.ts` | 参数化的被移除路由用例增加 `google-vertex`（strict 与 deferred 在同一条内各验一次） |
| `tests/loader-composition.spec.ts` | 目录元数据断言原本借用 `google-vertex`（它是唯一带 `baseURLEnv` 的已提供路由），改借 `google` 断言 `nativeAuth` 与 `defaultBaseURL`，并新增一条断言确认 `google-vertex` 不再出现在目录里 |
| `docs/user/guide/providers.md` / `.zh.md` / `.i18n.yaml` | 未提供路由清单增加 Google Vertex AI（Google Cloud 面向同一批 Gemini 模型的端点）；补一句 14 个条目在 `google` 上同 id、上下文与输入模态一致（并列出输出上限、缓存价与阶梯价差异）、`google-vertex` 不可作为自定义协议选择；自定义协议句加入 `google-vertex`；重录 i18n 哈希 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 的被移除路由清单增加 `google-vertex` 与对应说明；协议集合限制一段改为「Vertex 需要密钥、端点与标头之外的东西（ADC 还要项目与位置），自定义路由不能选它的协议，模型仍由 `google` 提供」；重录 i18n 哈希 |
| `docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/` | 描述与变更清单加入 `google-vertex`（26 → 27 条）；协议句加入 `google-vertex`；覆盖句补「14 个 id 全部在 `google` 上，三处字段差异如实列出，协议不可选因此没有自定义路由可替代」；为压回 500 词上限精简了存量句；正文 499 词；重录 i18n 哈希 |
| `apps/web/tests/expected/` 三个 golden | 删除 `option "google-vertex"`（15 → 14 项） |
| `apps/web/tests/models-settings.e2e.ts` | 供应商选择器下界断言 15 → 14 |
| 根 `README.md` 差异表 | 26 → 27 个移除路由、15 → 14 个路由、595 → 581 个模型、设置页 26 → 27 个 option，并补 Vertex 说明（含三处字段差异） |
| `logs/00009` | 汇总刷新为 14 个路由 / 581 个模型 / 27 条被移除路由，加入本日志条目与 Vertex 已知限制 |
| `logs/00008` | 「后续变更」补记当时借用 `google-vertex` 的目录元数据断言已改借 `google`，原路由不再提供 |

## 2. 行为

- 可对话供应商 15 → **14**，目录模型 595 → **581**（只减 `google-vertex` 的 14 个模型）。
- `catalogProvider('google-vertex')` 返回 `undefined`，`catalogModels('google-vertex')` 为空；目录、模型解析、发现、登录与可配置供应商列表都不再出现该路由。
- 存量 profile 点名 `google-vertex` 时 `resolveProfiles()` 在 strict 与 deferred 下都抛 `llm-pi-ai: provider "google-vertex" is removed from this build; delete the route or select another provider`，同一 `providers` 字典里的其他路由随之不可用，删除该条目即恢复。
- 内置 id 现为 42 = 14 个提供 + `typesafe` + 27 个被移除；`provider-data.generated.ts` 仍保留 42 个 id 的元数据（`verify-provider-data` 要求与已安装目录一致），它不是可售路由。
- `packages/experimental/webworker-runtime` 的 `BUILTIN_PROVIDER_IDS` 仍列出 `google-vertex`：该列表描述 pi-ai 已安装目录，与本次移除无关。
- 模型覆盖不受影响：14 个 id 在 `google` 上同名提供（上下文窗口与输入模态一致，输出上限、缓存价与阶梯价三处字段不同）；需要 GCP 结算或区域驻留的项目只能重新放行该内置路由，自定义模型 API 接不回 Vertex 协议。
- 未触及 agent loop、模型可见工具、Session 事件或持久化格式；`deepseek-official` 与 `deepseek-account` 原生路由不受影响。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm exec vitest run packages/llm/llm-pi-ai` | 22 个文件、491 个测试全部通过（比上一轮 +1，参数化用例新增 `google-vertex` 一条） |
| `pnpm exec vitest run packages/llm`（全 llm 包） | 78 个文件、1800 个测试全部通过（比上一轮 +1） |
| `pnpm exec vitest run packages/client/ui-settings-models` | 14 个文件、483 个测试全部通过（设置页读取同一份目录，本轮的目录变化未改动其断言） |
| `pnpm run typecheck` | 5 passed，0 failed（7.30s） |
| `pnpm run lint` | 0 warning、0 error（90 条规则、7303 个文件，89.9s）；复查轮改注释后按文件重跑 `catalog.ts` 仍为 0/0 |
| `pnpm run build` | 11 个门禁全部通过（30.84s，含 host/client 聚合检查与 Web build） |
| `pnpm run test:docs` | 21 个门禁全部通过（19.84s；补记日志与改号后重跑 21/21，16.05s；复查轮修正文档后重跑 21/21（16.27s），定稿后 21/21（16.46s）） |
| `pnpm run doc-sync` | 43 个门禁全部通过（63.13s；补记日志后重跑 43/43，52.84s；复查轮 43/43（53.85s），定稿后 43/43（52.17s）） |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致（生成文件仍保留 42 个 id 的元数据） |
| `pnpm exec tsx scripts/gen-config-catalog.ts --check` | 3 个生成产物均为最新 |
| `pnpm exec tsx scripts/verify-upgrade-guides.ts` | 通过，guide.md 500 词（上限 500） |
| 目录核对（源码 probe） | 14 个提供路由、581 个模型；`catalogProvider('google-vertex')` 为 `undefined`，`catalogModels('google-vertex')` 为 0 |
| 文档清单核对 | 7 份中英文档（用户指南、包 README、升级指南各一对与根 README）均枚举全部 27 个被移除 id，无缺失、无多余 |
| `git diff --check` | 通过 |
| `pnpm run verify-translation-pairing` | 1301 对全部一致 |
| Web golden 单项回放 | 临时 overlay 关闭 `ui-boot-animation` 后分别回放 `models-settings` 的 `opens the add card over the dormant directory vocabulary` 与 `onboarding-usable-provider` 的 `closes the setup card without discarding the add card beside it`：provider `option` 列表与 golden 逐项一致（14 项、不含 `google-vertex`）；两个用例的唯一差异都是金标未含的既有 `IM机器人` 导航按钮，与本改动无关。临时 overlay 与两处测试补丁已还原 |

## 4. 复查记录

本轮复查覆盖 8 处，均已随改动修正：

1. `loader-composition.spec.ts` 原本以 `google-vertex` 作为目录元数据断言的目标，因为它是唯一带 `baseURLEnv` 的已提供路由；该路由被移除后断言改用 `google`（`nativeAuth` 与 `defaultBaseURL`），并补一条 `google-vertex` 不在目录中的断言。目录级 `baseURLEnv` 的集成覆盖因此只保留在 `native-auth.spec.ts`（`azure`、`cloudflare-ai-gateway`、`google-vertex` 三条生成元数据）与设置页依赖注入的客户端用例中。
2. 覆盖口径按同 id 核对：14 个 Vertex 条目在 `google` 上全部同名提供（`in vertex not google` 为空），`google` 另有 9 个 Vertex 没有的条目。
3. 原先文档写「相同能力与相同目录价格」，与目录字段不符：`gemini-4-argon` 的最大输出 `google` 为 32,768、`google-vertex` 为 65,536；`gemini-3.1-flash-lite-image` 的缓存读取价 `google` 为 0、`google-vertex` 为 0.025；`google` 侧另有三个条目（`gemini-2.5-pro`、`gemini-3.1-pro-preview`、`gemini-3.1-pro-preview-customtools`）记录 200k 输入以上的阶梯价。已把 7 份文档与 `catalog.ts` 注释统一改为「同 id、同上下文窗口、同输入模态」，并逐处列出这三类差异，不再声称能力或价格一致。
4. 原先文档把移除理由写成「Vertex 通过项目、位置与应用默认凭据认证，profile 提供不了」，这不准确：pi-ai 的 Vertex 声明了 `apiKey` 方法，设置页存下的 Cloud API key 会作为请求级 `apiKey` 覆盖，走 `createClientWithApiKey` 时不需要项目与位置；只有 ADC 路径才需要 `GOOGLE_CLOUD_PROJECT` 与 `GOOGLE_CLOUD_LOCATION`。移除理由已改为「同一批模型的第二个托管入口」，ADC 的环境要求作为背景说明保留。
5. 「没有自定义模型 API 可以替代」的准确理由是 `google-vertex` 协议不在自定义模型 API 的可选协议表内，而不是「认证无法描述」；用户指南、包 README、升级指南与 `catalog.ts` 注释均已按此改述。
6. 包 README 的「协议集合更窄」一段原文写「Vertex 与 Codex 的目录路由仍可经自有提供方到达」，Vertex 被移除后不再成立，已改为「Vertex 需要密钥、端点与标头之外的东西（ADC 还要项目与位置），Codex 需要 OAuth grant；自定义路由不能选它们的协议，Vertex 的模型仍由 `google` 提供」。
7. 升级指南加入 Vertex 后一度超过 500 词，按「精简存量配置句、端点清单与协议句」压回 500 词（补记字段差异后再次超限，最终定稿 499 词），中英结构保持一致。
8. 文档清单脚本确认 7 份文档都枚举了全部 27 个被移除 id；三份 golden 的 option 列表与 `catalogProviderIds()` 逐项相等（14 个、同顺序）。

未改动 `provider-data.generated.ts` 与 `packages/experimental/webworker-runtime` 的 stub 列表：两者描述已安装目录而非可售路由，`verify-provider-data` 要求保留全部 42 个 id。

Web e2e 未整套重跑：本机既有 `dsh-boot-archive-black` 覆盖层会拦截设置入口点击，按前几轮的做法用临时 overlay 单独回放受影响的用例，回放后已还原补丁与 overlay。

## 5. 已知限制与恢复

- 本次只屏蔽可售路由，不删除 pi-ai 包内 Vertex 的实现与目录元数据；手工声明同名路由仍会被 `resolveProfiles()` 拒绝。
- 恢复时删除 `WITHHELD_PROVIDERS` 中的 `google-vertex`，把可售路由与模型计数改回 15 / 595，并同步撤销第 1 节列出的 golden、断言与文档改动，重新记录 i18n 哈希并复核升级指南词数；`loader-composition.spec.ts` 的目录元数据断言可以改回 `google-vertex`（它是唯一带 `baseURLEnv` 的已提供路由）。
- 同一模型在两条路由上的目录字段并非逐项相同：`gemini-4-argon` 的最大输出在 `google` 为 32,768、在 `google-vertex` 为 65,536；`gemini-3.1-flash-lite-image` 的缓存读取价 `google` 记 0、`google-vertex` 记 0.025；`gemini-2.5-pro` 与 `gemini-3.1-pro-preview(-customtools)` 在 `google` 侧另有 200k 输入以上的阶梯价，Vertex 侧没有。DSH 不读 cost 元数据，这些只影响迁移时的对照。
- 自定义模型 API 只有四种协议，`google-vertex` 不在其中（该表只收录密钥、端点与标头即可完整描述的协议），因此不能像 `together` 那样用一条自定义路由接回 Vertex；原样使用 Vertex 需要重新放行该内置路由。
- 运行中的 Web 宿主仍是改动前启动的进程，重启 `pnpm dsh web` 后设置页才会显示 14 项且不含 `google-vertex`。

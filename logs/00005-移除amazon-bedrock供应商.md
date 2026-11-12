# 00005 — 移除 amazon-bedrock 供应商

- **日期**：2026-11-12
- **背景**：用户要求从本仓库移除 `amazon-bedrock`，且不能影响其他供应商。该路由只是 pi-ai 内置目录中的一项，本机 Web profile 未配置，移除不应改变 `opencode-go` 与 DeepSeek 路由的可用性。
- **范围**：`packages/llm/llm-pi-ai` 的目录、配置、测试与双语 README；Web 设置页 golden；`docs/user/guide/providers.md` 与 `.zh.md`。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/llm/llm-pi-ai/src/catalog.ts` | 新增 `WITHHELD_PROVIDERS` 与 `isWithheldProvider()`；`catalogProviders()` 在 `servesChat` 前过滤。`catalogProvider()` / `catalogProviderIds()` / `catalogModels()`、目录声明、登录与模型发现随之失效。 |
| `packages/llm/llm-pi-ai/src/config.ts` | `resolveProfiles()` 遇到 withheld provider 直接抛出配置错误；strict 与 deferred 都生效，避免存量配置被当成“手工声明路由”继续展示。 |
| `packages/llm/llm-pi-ai/src/index.ts` | 目录注释由“完整已安装目录”改为“本构建服务的目录”，与实际行为一致。 |
| `packages/llm/llm-pi-ai/tests/catalog.spec.ts` | 删除已不可达的 Bedrock compat 用例；新增四个 catalog 表面都不再暴露 `amazon-bedrock` 的回归测试；相关测试名改为 served catalog。 |
| `packages/llm/llm-pi-ai/tests/config.spec.ts` | 新增 strict/deferred 下拒绝存量 `amazon-bedrock` profile 的测试。 |
| `packages/llm/llm-pi-ai/tests/dynamic-config.spec.ts` | 注释由“每个已安装目录提供方”改为“每个 served 目录提供方”。 |
| `packages/llm/llm-pi-ai/README.md` / `.zh.md` / `.i18n.yaml` | Known Limitations 增加移除说明；注册与目录段落由“完整目录”改为“所服务的目录”，并同步双语哈希。 |
| `apps/web/tests/expected/models-settings/empty.expected.md` | 删除 `option "amazon-bedrock"`（设置页可选项 41 → 40）。 |
| `apps/web/tests/expected/models-settings/native-keyless.expected.md` | 同上。 |
| `apps/web/tests/expected/onboarding-usable-provider/dismissed.expected.md` | 同上。 |
| `docs/user/guide/providers.md` / `.zh.md` | 第三方提供商章节补充“本构建不提供 Amazon Bedrock”的说明，并说明 `bedrock-converse-stream` 不在自定义协议选项中。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 设置页「添加模型提供商」 | 不再出现 `amazon-bedrock` |
| 模型发现、OAuth/密钥登录 | 不再包含该路由 |
| `catalogProvider()` / `catalogModels()` | 返回 `undefined` / 空 |
| 存量配置点名 `amazon-bedrock` | 明确报 `provider "amazon-bedrock" is removed from this build; delete the route or select another provider` |
| 其他供应商（含 `opencode-go`、DeepSeek 两条路由） | 不受影响 |

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm exec vitest run packages/llm/llm-pi-ai` | 22 个文件、469 个测试全部通过 |
| `pnpm exec tsc -b packages/llm/llm-pi-ai/tsconfig.json` | 退出码 0 |
| `pnpm run build:check` | host/client 两个聚合检查通过（host 54.49s，client 4.34s） |
| `pnpm exec tsx scripts/run-oxlint.ts <改动文件>` | 0 warnings / 0 errors |
| `pnpm run test:docs` | 21 个文档门禁全部通过 |
| `pnpm run verify-provider-data` | 与已安装 pi-ai 目录一致 |
| 包内隔离覆盖率 | `catalog.ts`、`config.ts` 均为 100% |
| Web e2e（Chromium） | 20/22 通过；剩余 2 个 WebKit 移动端用例因浏览器未安装失败，与本次改动无关 |
| `git diff --check` | 通过 |

## 4. 复查修正

1. **构建产物未更新导致 e2e 仍显示旧列表**：首次运行 `models-settings` / `onboarding-usable-provider` e2e 时仍出现 `amazon-bedrock`。原因是 `apps/web/tests/scaffold.ts` 通过 `requireBuilt()` 加载 `lib/`，而源码改动尚未进入 bundle。执行 `pnpm exec tsdown --config-loader native --env.DSH_BUILD_FACE host` 重建后 e2e 通过。结论：源码方式只需重启进程；构建产物方式必须重新构建。
2. **存量配置会以“声明的路由”复活**：只过滤 catalog 时，已存的 `amazon-bedrock` profile 会继续出现在设置页，并给出误导性的“补模型”诊断。增加 `resolveProfiles()` 的明确拒绝后，移除才完整。
3. **文档与注释仍写“完整目录”**：`index.ts` 注释、README 段落、两个测试名称已统一改为“served catalog”，避免契约与实现不符。
4. **双语哈希**：`README.md` 与 `providers.md` 修改后用 `pnpm run verify-translation-pairing --write <pair>` 重新记录，随后全量 `test:docs` 通过。

## 5. 已知限制

- `packages/llm/llm-pi-ai/src/provider-data.generated.ts` 仍保留 `amazon-bedrock` 的 AWS 环境变量元数据。该文件由已安装 pi-ai 目录生成，`verify-provider-data` 要求其完整；它描述的是 pi-ai 的上游元数据，不是本构建的可售路由。
- `packages/experimental/webworker-runtime` 的预览 stub 仍列出该 id，但经过同一个 `catalogProviders()` 过滤器，不会出现在预览的供应商列表中。
- `packages/credentials/credentials-local` 的 README 与测试继续以 `llm-pi-ai/amazon-bedrock` 作为凭据记录 fixture；它们验证 key 文法，与供应商是否可售无关，未改动。
- 当前 `127.0.0.1:3080` 的进程是在改动前启动的，仍显示旧列表；重启后生效。
- WebKit 浏览器未安装，两个移动端 e2e 用例未在 WebKit 下验证；Chromium 已覆盖供应商列表 golden。运行 e2e 前执行了 `pnpm --filter @deepseek-ai/dsh-web-frontend exec playwright install chromium`，浏览器落在 `~/.cache/ms-playwright`，未改动仓库。

## 6. 后续

- 上游 pi-ai 升级若新增或移除供应商，`WITHHELD_PROVIDERS` 与 `resolveProfiles()` 是仅有的两个控制点。
- 若将来要恢复 Bedrock，需要删除集合项、恢复 Bedrock compat 测试、更新 3 个 Web golden 与双语 README，并重新记录 i18n 哈希。
- 不直接修改 `node_modules/@earendil-works/pi-ai`，避免上游同步冲突与 `verify-provider-data` 漂移。

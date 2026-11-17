# 00038 — 原生集成 dsh-superagent

- **日期**：2026-11-17
- **背景**：`nano-dsh-superagent-prototype` 是 DSH 团队发布的主动型编码 Agent 研究原型，原先以独立仓库 + bundle 的形式提供，需要单独 clone、`dsh plugin add` 并维护一个 `superagent` profile。本仓库需要把它变成 super-agent 的原生默认能力：新 clone 执行 `pnpm install && pnpm run build` 后启动 Web/Desktop 就能在侧栏看到「Superagent」，不再依赖外部原型仓库，同时不破坏现有功能。
- **范围**：`apps/web/native/dsh-superagent`、`packages/bundle/web-app`、`pnpm-workspace.yaml`、`pnpm-lock.yaml`、`scripts/build.ts`、`scripts/baseline-inventory.ts`、`.oxlintrc.json`、根 `README.md`、`logs/`。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `apps/web/native/dsh-superagent/` | 以源码形式 vendored 上游原型（基线提交 `8c32a9d`，2026-10-31，MIT），保留 `src/`、`tests/`、`docs/`、`prompts/`、`skills/`、`locale/`、`examples/`、`SETUP.md` 与 `LICENSE`；新增 `NATIVE.md` 记录基线、挂载方式、本地修改、检查命令与上游同步流程 |
| `apps/web/native/dsh-superagent/package.json` | 改成 private workspace 包；`@deepseek-ai/*` peer 依赖改为 `workspace:*`；更新 `dsh.client.inject` 为 0.2.2 客户端模块；移除 `dsh.bundle`（由 Web bundle 挂载行）；构建仍走包内 `tsdown` |
| `src/deployment.ts` | `stateDir` 增加默认值 `<DSH_HOME>/state/superagent`（`defaultStateDir()`），内置副本无需机器配置即可启动；绝对路径检查兼容 POSIX、Windows 盘符与 UNC，相对路径仍然拒绝 |
| `src/app/resources.ts` | Session 转录优先从 live Agent 的 `session.snapshotEvents()` 读取；未加载的 Session 通过 `sessionPersistence.open(id, 'read')` 读取已持久化日志；0.2.2 已移除 `sessionController.inspect()` |
| `src/client/deployment.tsx` | 只读判断改用 `ConfigFormSnapshot.writable`，`loading` 优先于只读提示；0.2.2 移除了 `mode` |
| `src/sources/feishu-loops.ts` | 修复事件消费健康竞态：事件管道失败后，迟到的 `[event] ready` 不再覆盖错误；连接级 WARN 仍可被后续 ready 清除 |
| `src/app/http.ts` | 容忍 Web server 尚未分配 `port`；管理 API 跟随 Web 绑定地址（含 `127.0.0.2`、`::1` 等 loopback 别名与容器/LAN 地址），不再只接受 `127.0.0.1`；DSH Web 认证、`x-orchestrator-request` 与同源检查不变 |
| `tests/*` | 按 0.2.2 API 适配：`agent-preset/selected` 的 branded `SessionId`、模型选择流程重写、`parseEntryYaml` 解析 `!!js`、panel 快照字段、转录 fixture、`stateDir` 默认值；新增持久化转录回退、loopback/LAN 绑定与 preset 挂载结构回归 |
| `packages/bundle/web-app/package.json` | 增加 `dsh-superagent`、`@deepseek-ai/dsh-bash-sandbox`、`@deepseek-ai/dsh-pwsh-sandbox` workspace 依赖；`files` 与 `dsh.bundle.patch` 登记两个 preset 文件 |
| `packages/bundle/web-app/cordis.patch.yml` | 插入 `dsh-superagent`（Host + 浏览器面板）、`dsh-superagent-github`、`dsh-superagent-feishu`（disabled）三行 |
| `packages/bundle/web-app/presets/superagent-worker.patch.yml`、`superagent-router.patch.yml` | 新增两个 agent preset：worker 与 router 的 5 秒 shell / job wait、工具面裁剪；sandbox 行使用 `superagent-` 前缀 id，避免与 host 组合行冲突 |
| `packages/bundle/web-app/tests/superagent-bundle.spec.ts` | 新增回归：bundle 依赖、三行挂载、两个 preset 文件、原生包 Host/Client 入口 |
| `scripts/build.ts` | 新增 `Superagent plugin` 构建 gate，并在 Web build 前完成包内 Host/Client bundle |
| `scripts/baseline-inventory.ts`、`.oxlintrc.json` | 把 `apps/web/native/` 纳入既有的 vendored 例外（与 `vendor/`、归档 notes 一致），避免上游 TS 测试习惯触发 `verify-no-unknown-casts` 等 first-party 门禁与 lint |
| `apps/web/native/dsh-superagent/docs/006-deployment.md` | 补充「Built-in copy」、快速开始、飞书启用与日常使用说明；管理 API 改为跟随 Web 绑定地址，并说明认证与同源边界 |
| 根 `README.md` | 新增「原生 Superagent」小节，说明默认挂载、GitHub/Feishu 默认状态、管理 API 绑定行为、使用步骤入口与上游同步入口 |

## 2. 行为

- 新 clone 执行 `pnpm install && pnpm run build` 后，`pnpm dsh web`（或 Desktop）侧栏自动出现「Superagent」，无需 `dsh plugin add`、无需单独 profile、无需再 clone 原型仓库。
- 默认启用 GitHub Source、关闭 Feishu Source。没有配置仓库与 agent 账号时，面板显示 setup 待配置（`repositoryUnset`），runtime 保持 paused：不轮询、不派工、不影响其他 Session 与面板。
- 状态目录默认 `<DSH_HOME>/state/superagent`；可用 profile 的 `cordis.patch.yml` 覆盖 `stateDir`。
- 两个 agent preset 随 Web bundle 的 preset 层加载；`dsh-superagent` 的 Session 创建/恢复依赖它们，需与插件行一起保留。
- 管理 API 与面板同源：默认绑定 `127.0.0.1`；`--host` 指定其他 loopback 别名或容器/LAN 地址时，API 跟随该绑定，仍要求 DSH Web 认证 cookie、`x-orchestrator-request: 1` 和同源 `Host`。未带认证返回 401，缺少头或跨源返回 403。
- 原有 `dsh-im`、Web/Desktop 面板、Session、工具与 profile 行为不变。

## 3. 校验

| 命令 | 结果 |
|---|---|
| vendored `vitest run` | 1,197 个测试通过（59 个文件） |
| `pnpm run typecheck` | 通过（5 个 gate） |
| `pnpm run build:check` | 通过（2 个 gate） |
| 干净 worktree `pnpm install && pnpm run build` | 通过（12 个 gate，147.66s，含 `Superagent plugin`） |
| `pnpm install --frozen-lockfile` | 通过，lockfile 已包含 workspace 包 |
| `pnpm run test:docs` | 通过（21 个 gate） |
| `pnpm exec tsx scripts/verify-cordis-config.ts` | 359 个配置通过 |
| `verify-application-entrypoints`、`verify-client-packages`、`verify-default-product-isolation`、`verify-package-paths`、`verify-config-source-ownership` | 全部通过 |
| `pnpm run lint` | 0 error、0 warning（vendored 子树按既有例外排除） |
| 干净 worktree `pnpm dsh web` | 启动成功；首页模块清单含 `dsh-superagent/client.js`；`GET /orchestrator/api/state` 返回 setup 待配置；`<DSH_HOME>/state/superagent` 自动创建 |
| 浏览器端 bundle 取回 | `plugins/??dsh-superagent/client.js&rev=…` 返回 200（358 KB），模块 id 为 `dsh-superagent` |
| 绑定矩阵 | `127.0.0.1`、`127.0.0.2`、LAN `172.29.86.214` 下插件均激活，面板与 `/orchestrator/api/state` 可用；未带认证 cookie 返回 401 |
| preset 插件配置校验 | 两个 preset 共 39 行插件的 config 逐个通过本地包 `Config` schema |

## 4. 已知问题与边界

- `verify-no-unknown-casts` 在 HEAD 上已有一处未记录的既有断言（`packages/api/session-controller/tests/agent.host.spec.ts:284`），与本次集成无关；vendored 子树已按既有例外排除。
- 上游文档描述的是独立 `superagent` profile 的安装方式；内置副本在 `docs/006-deployment.md` 顶部补充了「Built-in copy」说明，其余账号与运维步骤仍然适用。
- Executor Session 以 profile 权限、操作者 OS 用户身份运行；shim 与 git 配置只是默认值，不构成隔离边界。
- 管理 API 跟随 Web 绑定地址：非 loopback 绑定下它对该网络可达，边界是 DSH Web 认证与同源请求检查；默认绑定仍为 `127.0.0.1`。

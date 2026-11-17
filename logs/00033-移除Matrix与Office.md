# 00033 — 移除 Matrix 与 AI Office

- **日期**：2026-11-17
- **背景**：用户要求把 Matrix 渠道和 AI Office 连接器从原生 IM 中移除，且不能破坏其余渠道的正常功能。目标同样是彻底移除，不是隐藏入口。
- **范围**：`apps/web/native/dsh-im`、根 `README.md`、`pnpm-lock.yaml`、`logs/`。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `src/channels/matrix`、`src/channels/office` | 删除渠道实现、状态与凭据存储、API、加解密、运行时、桥接、Job 执行器与协议 |
| `plugin-src/host/channels/{matrix,office}` | 删除 Host 入口、production、RPC |
| `plugin-src/client/channels/{matrix,office}` | 删除设置页、RPC 客户端与样式 |
| `plugin-src/host/index.mjs` | 删除两个 import、启动项与 `channels` 条目；`office` 跳过投递服务的特例随渠道一起删除 |
| `plugin-src/host/sidebar-settings.mjs` | 移除 `MatrixConfigStore`、`OfficeConfigStore` 与 `office` 单配置判定 |
| `plugin-src/host/delivery-adapter.mjs`、`delivery-suggestions.mjs` | 删除 `matrix` 投递渠道、目标校验、会话键推导与私聊前缀 |
| `plugin-src/host/channels/shared/startup-error.mjs`、`access-policy-production.mjs` | 删除矩阵/Office 的渠道名、配置错误文案与访问策略分支 |
| `plugin-src/client/index.js` | 删除标签页、Logo、RPC、样式安装与面板分支；`CHANNELS` 只剩八个渠道 |
| `plugin-src/client/session-channel-logos.js`、`channel-logos.js`、`styles.js`、`im-channels.js` | 删除两个通道的 Logo、会话前缀映射、样式与列表标签 |
| 共享层 | 删除 `session-channel-labels` 条目；`command-catalog` 文本渠道列表清空；删除 `harness-approval` 的 `matrix`、`connection-error` 的 Office 错误码与 Office 公共错误收敛、`diagnostic-details` 的 `office`；`route.mjs` 删除 Matrix 会话键 codec，`describe.mjs` 删除 `direct:dm:`、`group:room:` 解析；`text-harness-bridge` 删除 Matrix 媒体上传提示；相关注释与 `AGENTS.md` 渠道名列表同步 |
| i18n | 删除 `i18n-en/{matrix,office}.mjs`、`i18n-ru/{matrix,office}.mjs` 及聚合引用 |
| 依赖 | 删除 `@matrix-org/olm`；锁文件同步收缩 |
| 脚本 | `verify-lan-management.mjs`、`verify-injected-context.mjs`、`verify-session-title-prefix.mjs` 的渠道清单移除两项 |
| 文档 | 删除 `docs/AI-Office-Connector{,.en}.md`；包 README 横幅、`NATIVE.md`、`PROACTIVE_DELIVERY{,.en}.md`、`THIRD_PARTY_NOTICES.md`、`locale/*.json`、`docs/client-integration.md`、`docs/preset-workspaces.md`、`docs/bot-commands.md` 与根 README 改为八渠道口径 |

## 2. 行为

- 最终内置渠道：微信、飞书、钉钉、企业微信、企业微信应用、QQ、iMessage、邮箱；不再有 Matrix 与 AI Office。
- 两个通道不再挂载、不再显示、不再有 RPC；旧的 Matrix/Office 配置目录不会被读取，也不会让插件启动失败。
- `/goal`、`/plan`、`/ws` 等命令不受影响，继续在八个渠道上工作。
- 投递目标、访问策略、审批、诊断详情、会话标题前缀、Sidebar 自动显隐都不再识别这两个通道。
- 删除的 `direct:dm:`、`group:room:` 解析只影响已不存在的 Matrix 会话键；其余渠道的键格式不变。

## 3. 校验

| 命令 | 结果 |
|---|---|
| vendored `node --test test/*.test.mjs test/channels/*/*.test.mjs` | 3,548 个测试通过，0 失败（删除两个渠道专属套件后较 3,680 减少 132） |
| `pnpm run build` | 11 个 gate 通过 |
| `pnpm run typecheck` | 5 个 gate 通过 |
| `pnpm run test:docs` | 本改动涉及的文档通过；`packages/api/settings-controller` 的翻译配对失败来自同工作区另一份未提交改动，未触碰 |
| `pnpm run lint` | 0 warning、0 error |
| `pnpm-lock.yaml` | 手工只删除 `@matrix-org/olm` 的 importer、package 与 snapshot 三段，避免把同工作区其它未提交依赖变更带进锁文件 |
| 干净 HEAD worktree + 本改动清单运行 `pnpm install --lockfile-only --frozen-lockfile` | 通过；锁文件只删 `@matrix-org/olm` 的 8 行 |
| `vitest packages/bundle/web-app/tests/im-bundle.spec.ts` | 3 个 bundle 守卫用例通过 |
| Playwright（临时 `DSH_HOME`、3102 端口） | 设置页只显示 8 个渠道；逐一打开 8 个标签无控制台错误；`.dim-logoMatrix`、`.dim-logoOffice` 数量为 0 |
| 管理 RPC 探测（同一临时 Host） | `/api/dsh-im/{feishu,email,qq}` 返回 200；`/api/dsh-im/matrix`、`/api/dsh-im/office` 返回 404 |

## 4. 复查记录

- 首次跑套件 21 个失败，全部来自仍以两个渠道为夹具的测试（Host 组合、启动流程、Harness 连接、网络 peer、语言顺序、消息模式 RPC、会话绑定、上下文档、侧栏配置、通道能力）。逐项改为保留渠道（飞书、QQ、邮箱、企业微信）后全绿。
- `startup.test.mjs` 的语言刷新用例原先借用 `office` 作为“无平台菜单”的通道，改用 `email`。
- 会话键 codec 删除 Matrix 后，把 `conversation-context` 的 codec、路由、语义消息、根上下文、来源窗口用例改为飞书键与一个测试内内存记录，保留对共享语义层的覆盖。
- 客户端 i18n 词典中仍留有 `Matrix`、`AI Office` 文案键（与四渠道移除时的处理一致）；这些键不再被任何组件引用，删除需逐条避开与其它渠道共用的键，留待后续统一清理。
- 共享语义层模块（`ambient-provider`、`context-builder`、`reply-context`、`root-context`、`trigger-policy`、`semantic-message`）本身与渠道无关，未删除，目前主要由测试覆盖；删除会扩大与上游的差异。

## 5. 已知限制

- 变更后尚未重启当前 3080 端口的 Host 进程；重启并刷新后设置页才会只剩八个渠道。
- `lib/` 不提交；新 clone 仍需 `pnpm install && pnpm run build`。
- WeCom app 的 `/goal`、`/plan` 仍走同一委派路径，没有单独用例。

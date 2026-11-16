# DeepSeek Harness

DeepSeek Harness（`dsh`）是由 [DeepSeek AI](https://deepseek.com) 开发的开源 agent harness（智能体框架）。

它构建于**一切皆插件**的架构之上，由 [Cordis](https://github.com/cordiverse/cordis) 驱动，其设计参见论文 [_A Programming Paradigm for Spatiotemporal Composability_](https://arxiv.org/abs/2608.25512)。

文档：[https://deepseek-harness.github.io/deepseek-harness/](https://deepseek-harness.github.io/deepseek-harness/)

## 开发者预览

DeepSeek Harness 处于 _开发者预览_ 阶段，正在快速迭代。**未来将出现破坏兼容性的变更。**

运行本项目前，请阅读[安全说明](SAFETY.zh.md)。

<a id="run"></a>

## 运行

本仓库是 DeepSeek Harness 的个人魔改分支，直接以本仓库的代码和本地构建产物运行，不发布到 npm。

<a id="run-from-source"></a>

### 从源码运行

安装 `Node.js`（≥ 24.9）与 pnpm 11.28.5（推荐 `corepack enable pnpm`），然后：

```sh
git clone https://github.com/yanfeng98/super-agent.git
cd super-agent
pnpm install
pnpm run build
pnpm dsh web
```

`pnpm run build` 会生成运行所需的产物；`pnpm dsh web` 不会重新构建：宿主端代码由 tsx 直接从 `src` 运行，浏览器端加载构建出的 bundle 与 `apps/web/dist`。首次运行前必须构建；之后宿主端改动重启进程即可生效，浏览器端改动需要重新构建。

该命令默认会在 `http://127.0.0.1:3080` 启动 Web UI，本机启动时还会用默认浏览器打开页面。通过 SSH 启动时只打印宿主机 URL，因为本地转发地址由 SSH 客户端或编辑器持有。传入 `--no-open` 可仅运行服务器而不打开浏览器。详见 [Web UI 指南](docs/user/guide/index.zh.md)。

### 日常开发

`pnpm run dev:web` 先构建一次再启动 Web UI：之后浏览器端源码改动会自动重建产物并刷新页面；宿主端包不在监听范围内，改动后重启进程即可（加 `--skip-build` 复用已有产物）。`pnpm run start:web` 直接启动上一次构建的产物，与 `pnpm dsh web` 等价。

需要后台常驻时用 `make web-bg`（tmux 会话 `dsh-web`，启动后打印带 token 的 URL），停止用 `make web-stop`。

## 查找插件

社区插件目录：[DSH Marketplace](https://dshmarketplace.dev/)。它汇总 DeepSeek Harness 的第三方插件，提供安装命令、安装验证与公开 API；DSH 0.2.1+ 还可将其索引 `https://dshmarketplace.dev/market/index.json` 添加到 Plugins 的 Discover 中。列表不代表安全审查，安装前请阅读源码，并优先在一次性 profile 中验证。

## 开发

请先阅读[开发指南](docs/development.zh.md)与[架构文档](docs/architecture.zh.md)。

`make help` 列出 Web 与 Desktop 对应的 Make target；完整表格见开发指南的「应用命令」一节。

面向 agent：请遵循 [AGENTS.md](AGENTS.md)。

## 与上游的差异

对比基线为上游 [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness) 2026-11-13 的 `master`（`0.2.2-rc.3` 发布线），对比时间为 2026-11-16。截至 `nvidia` 移除提交，`main` 相对上游有 37 个提交、361 个文件（+19,858/−895 行）；下表按当前分支统计，不含本次 README 更新。上游继续前进后需要重新比对。逐项实现与校验记录见 `logs/`；需要迁移的破坏性变更另有升级指南。

### 新增与行为调整

| 特性 | 行为 | 源码与文档 |
|---|---|---|
| 启动片头 | Web 与 Desktop 启动时默认播放：先演六幕档案序列（授权对白、鲸标、身份卡、进度条与音效），再播放片头视频；默认可用按钮或 Esc 跳过。设置页可调 36 项序列参数与 6 项播放参数（含阶段顺序、播放时机、适配方式、随机与选中片头，可只播其一）；每个会话可要求重播或固定片头。内置 4 个片头（约 8.7 MiB）与 6 条配音，也支持 `$DSH_HOME/boot-animation/videos/` 中的自有视频。 | 新包 [`ui-boot-animation`](packages/client/ui-boot-animation/README.zh.md)：Host 端 `/dsh-boot-animation` 路由流式提供媒体（Range、ETag、owner-only），浏览器端负责播放与设置页；挂载于 `dsh-web-app`。Windows 标题栏按钮在片头期间隐藏。 |
| 永久删除会话 | 侧栏行悬浮垃圾桶按钮与会话菜单的「Delete session…」打开确认框；确认后删除该 Session 的全部已提交版本（generation），以及 Workspace 账户、归档与置顶引用，并向其他客户端广播；操作不可撤销。运行中的工作、子智能体会话、仍被本控制器之外的 Agent 或存储句柄占用的会话会被拒绝并说明原因。 | [`session/delete`](packages/api/session-controller/src/delete.ts)、[`SessionPersistence.remove`](packages/session/session-persistence/src/index.ts)、`WorkspaceRegistry.forgetSession`、[删除对话框](packages/client/ui-workspace/src/client/session-actions/DeleteSession.tsx)；[升级指南](docs/upgrade-guide/v0.2.2-rc.3/session-deletion/guide.zh.md)。 |
| Composer 文件夹输入历史 | 空草稿时 ↑ 召回当前文件夹最近一条，↑/↓ 前后浏览，↓ 越过最新一条清空；每个文件夹保留最近 100 条。只把成功提交时的文本写入历史，附件、引用 chip 等结构化内容不进入历史；相邻重复去重；多行条目只有在首行/末行才离开光标，斜杠菜单、IME、带修饰键的方向键与附件不触发。 | [`history.ts`](packages/client/ui-conversation/src/client/input/history.ts) 使用 `dsh.conversation.input-history.v1.<cwd>` 存于浏览器 localStorage，配合 `facade.ts`、`keymap.ts`、`hub.ts`；[包契约](packages/client/ui-conversation/README.zh.md)、[用户指南](docs/user/guide/index.zh.md)。 |
| 消息编辑并重发 | 用户消息的 Copy 旁新增 Edit（纯文本可编辑）；弹窗预填原文，确认后从该消息所在 Turn 之前 fork 出新会话并发送编辑文本。源会话的消息不被替换，后续消息留在源会话。首条消息改为在源 Workspace 新建 Session；目标 Turn 运行中先取消；发送失败时打开新会话并恢复编辑文本；附件、非文本或未加载 Turn 起点显示禁用按钮并说明原因，子智能体会话或缺少所需权限时不提供编辑入口。 | [`edit-resend.ts`](packages/client/ui-chat/src/client/chat/edit-resend.ts)、[`EditMessageDialog.tsx`](packages/client/ui-chat/src/client/chat/EditMessageDialog.tsx) 与 Chat slots/locale；[包契约](packages/client/ui-chat/README.zh.md)。 |
| Markdown 表格收进正文列 | 表格不再突破到 transcript 两侧：4 列及以上表格先按自然宽度排版，超过正文列就换行收敛；只有各列 100px 可读下限之和超过容器时，才在表格内部横向滚动。所有 `MarkdownText` 宿主（对话、工具卡、插件文档等）行为一致。 | [`MarkdownText.module.css`](packages/client/ui-primitives/src/markdown/MarkdownText.module.css) 使用 `width: fit-content; max-width: 100%`；删除聊天侧宽表突破规则；[Agent Note](.agents/notes/implemented/bug-fix/2026-11-15-markdown-tables-fit-the-column.md)。 |
| 后台 Web 服务 | `make web-bg` 在 tmux 会话 `dsh-web` 中启动 Web UI，日志写入 `/tmp/dsh-web.log`（权限 600，含 token），最多等待 45 秒以打印带 token 的 URL，未出现时提示查看日志；`make web-stop` 结束会话，会话不存在时幂等退出。 | [`Makefile`](Makefile) 的 `web-bg`/`web-stop`；[开发指南](docs/development.zh.md)。 |

### 移除或收紧的能力

| 项目 | 变化 | 影响与替代 |
|---|---|---|
| 19 个 pi-ai 内置路由 | `amazon-bedrock`、`azure`、`cerebras`、`cloudflare-ai-gateway`、`cloudflare-workers-ai`、`huggingface`、`minimax`、`moonshotai`、`moonshotai-cn`、`nvidia`、`opencode`、`qwen-token-plan`、`qwen-token-plan-cn`、`qwen-token-plan-individual`、`radius`、`vercel-ai-gateway`、`xiaomi`、`xiaomi-token-plan-sgp`、`xiaomi-token-plan-ams` 不再出现在提供商目录、发现、登录与模型解析中。 | pi-ai 目录仍提供 22 个路由、713 个模型；pi-ai 包本身仍带这些路由的元数据，本构建只是屏蔽。存量 profile 点名被移除路由会使 `llm-pi-ai` 加载失败，删除该条目后其余路由恢复；需要继续使用原服务时可按[升级指南](docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.zh.md)改用其他路由，或把兼容端点添加为自定义模型 API。`nvidia` 的 20 个模型中部分 Nemotron/Cosmos 等模型没有其他内置路由，`huggingface` 的无其他内置路由模型数也由 13 变为 14。`deepseek-official` 与 `deepseek-account` 原生路由不受影响。 |
| 宽表突破布局 | 4 列及以上 Markdown 表格不再横向撑满 transcript；`md-table-wide` 只保留滚动条揭示与键盘聚焦语义，不再决定宽度。 | 表格与段落、标题使用同一条宽度轴；长表换行，极多列或窄视口才在表格内部滚动。相关 e2e 与金标已改为断言该几何。 |

### 工程、测试与文档

| 项目 | 内容 |
|---|---|
| 新包与集成 | 新增 `@deepseek-ai/dsh-client-ui-boot-animation`（78 个文件，Host + Client 双面，自带 locales 与 assets），加入 `dsh-web-app` bundle 的启动项与 workspace 资产约束（`assets/`）。 |
| 持久化契约 | `SessionPersistence` 新增必需抽象方法 `remove` 与 `SessionPersistenceBusyError`；JSONL 后端删除整个 session 目录并持有 write lease；共享契约新增删除用例。自定义后端需实现该方法。 |
| 测试 | 新增 `session-delete.e2e.ts` 与 `composer-input-history.e2e.ts`；更新 `message-actions`、`markdown-wide-table`、`models-settings`、`sidebar-alignment` 等 e2e，以及大量 Client 单测与 Web 金标（用户消息新增 Edit、设置页移除 19 个 option 等）。 |
| 文档与记录 | 新增 2 组中英双语升级指南、3 篇 Agent Note 与多项 `logs/` 变更记录；更新 session、persistence、workspace、web-client 子系统页，config/persistence 生成目录，providers 与 Web UI 用户指南，以及开发指南。 |
| 仓库发布面 | 根 README 改为中文单语：原英文 `README.md` 被中文版替换，删除 `README.zh.md` 与 `README.i18n.yaml`，不再保留上游的 Community、Contributing、Citation 与 npm 运行章节，并改写为本仓库的 clone/build/run 流程。删除根 `LICENSE`、`CONTRIBUTING.md`、`CONTRIBUTING.zh.md` 及配对记录；根 `package.json` 仍声明 MIT，第三方依赖许可保留在 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。包级 README 仍按中英成对维护，根 README 在 `scripts/translation-pairing.manifest.json` 中排除。 |
| 权限与兼容性 | LiveShare 将 `session/delete` 归类为 `owner-only`，viewer 角色不可调用。改动未触及 agent loop、模型可见工具或 Session 日志格式；启动片头不注册工具、prompt 段或 Session 事件，输入历史只存在浏览器。 |

## 许可证

第三方依赖及其许可证见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

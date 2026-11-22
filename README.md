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

安装 `Node.js`（≥ 24.9）、pnpm 11.28.5（推荐 `corepack enable pnpm`）与 Rust（通过 [`rustup`](https://rustup.rs) 安装；构建按 crate 固定的 1.97.0 拉取工具链），然后：

```sh
git clone https://github.com/yanfeng98/super-agent.git
cd super-agent
pnpm install
pnpm run build
pnpm dsh web
```

`pnpm run build` 会生成运行所需的产物，其中包含从 [`native/rust-anything`](native/rust-anything/NATIVE.md) 现场编译的 rust-anything CLI（首次几分钟，中间产物在 `native/rust-anything/target/`，不纳入版本控制；缺少 `cargo` 时整个构建会失败）；`pnpm dsh web` 不会重新构建：宿主端代码由 tsx 直接从 `src` 运行，浏览器端加载构建出的 bundle 与 `apps/web/dist`。首次运行前必须构建；之后宿主端改动重启进程即可生效，浏览器端改动需要重新构建。

该命令默认会在 `http://127.0.0.1:3080` 启动 Web UI，本机启动时还会用默认浏览器打开页面。通过 SSH 启动时只打印宿主机 URL，因为本地转发地址由 SSH 客户端或编辑器持有。传入 `--no-open` 可仅运行服务器而不打开浏览器。详见 [Web UI 指南](docs/user/guide/index.zh.md)。

### 日常开发

`pnpm run dev:web` 先构建一次再启动 Web UI：之后浏览器端源码改动会自动重建产物并刷新页面；宿主端包不在监听范围内，改动后重启进程即可（加 `--skip-build` 复用已有产物）。`pnpm run start:web` 直接启动上一次构建的产物，与 `pnpm dsh web` 等价。

需要后台常驻时用 `make web-bg`（tmux 会话 `dsh-web`，启动后打印带 token 的 URL），停止用 `make web-stop`。

## 使用

打开页面后先在**设置 → 模型**填入 DeepSeek API 密钥（或选择其他提供方、自定义 OpenAI 兼容端点），保存即生效、无需重启；再用**选择工作区**添加启动 `dsh` 的目录并选中，然后新建会话。未选中工作区时输入框不可用。

- **直接描述任务**：Agent 会读取和修改工作区文件、运行命令、委派子智能体并维护计划；按当前权限策略需要审批的操作会先询问。
- **在输入框输入 `/`** 打开技能菜单：随包技能名都带 `eng-` 前缀（`eng-tdd`、`eng-pr`…）。27 个工程工作流技能在这里全部可点名，其中 11 个还能由模型按任务自动加载，另外 16 个模型看不到、在菜单里标着「仅用户」；因为名字带前缀，用上游原名放一份 `tdd` 到 `.dsh/skills`、[`.agents/skills`](.agents/skills) 或用户技能目录会与 `eng-tdd` 并存，而不是互相覆盖。菜单按来源分组，标题是「类别 · 来源」，类别按 内置 → 插件 → 项目 → 用户 → 自定义 排列：`内置`（office 4 个）、`内置 · mattpocock/skills`（工程技能）、`插件 · dsh-superagent`、`项目 · .agents/skills`、`用户 · ~/.agents/skills`、`自定义`；输入查询后标题收起，改为按名称排序。
- **空草稿时按 ↑** 召回当前文件夹最近 100 条输入，`↑`、`↓` 前后浏览。
- **插件**页查看已装与实验性插件、添加社区索引，见下一节。

### 内置能力

| 能力 | 入口与说明 |
|---|---|
| IM 机器人 | 设置页「IM 机器人」：微信、飞书、钉钉、企业微信、QQ、iMessage 与邮箱，`/goal`、`/plan` 是原生命令；见 [dsh-im NATIVE.md](apps/web/native/dsh-im/NATIVE.md) |
| Superagent | 侧栏「Superagent」→「执行」→「新建工单」，默认只用本地工单、不需要远端账号、没有 GitHub 集成；见[快速开始](apps/web/native/dsh-superagent/docs/006-deployment.md#quick-start) |
| 工程工作流技能 | 27 个都在 `/` 菜单里可点名，其中 11 个还能由模型按任务自动加载；见[使用指南](docs/user/guide/engineering-skills.zh.md)与[技能包](packages/skill/skill-engineering/README.zh.md) |
| 启动片头 | 默认播放，点按钮或按 Esc 跳过；设置页可调序列参数、换用自己的片头视频，见[启动片头包](packages/client/ui-boot-animation/README.zh.md) |

### 把代码迁移到 Rust

`standard` preset 的会话已经挂载 rust-anything，不需要安装：说一句「把这个模块重写成 Rust」或「给 `src/parser/` 出一份迁移计划」，也可以用 `/` 点名 `dsh-rust-anything` 先加载技能，Agent 随后调用 `mcp__rust-anything__*` 工具推进。一个单元的循环是 `analyze` → `bridge` → `record` → 写 `port.rs` → `build` → `diff` → `bench` → `gate`，`gate` 不通过就不算完成；整应用改写成纯 Rust 可执行文件走 `app_*`。

CLI 由 `pnpm run build` 从 [`native/rust-anything`](native/rust-anything/NATIVE.md) 编译到 `native/rust-anything/bin/rust-anything`，终端里可以直接用同一套命令（`analyze`、`gate`、`doctor` 等）。生成的 crate 写在**你指定的目标仓库**（要迁移的那个，不是本仓库）的 `rust/<unit>-rs/`，直接出现在它的工作树里；`record`、`bench`、`shadow` 和 `app record` 会真的运行目标项目的代码（`record --from-tests -- <测试命令>` 还会跑它自己的测试套件），这些代码以你的用户身份读写文件、访问网络，所以迁移在副本或临时 worktree 上做，并且不要把生产凭据留在环境里。不需要这项能力时，删掉 [`standard.patch.yml`](packages/bundle/web-app/presets/standard.patch.yml) 里的 `mcp-rust-anything` 行；嫌 16 个工具占请求 token，就把该行的 `toolLoading` 改成 `on-demand`，重启后生效。

单元级流程见[rust-anything 使用指南](docs/user/guide/rust-anything.zh.md)，整应用重写见[技能正文](.agents/skills/dsh-rust-anything/whole-app.md)，源码基线与上游同步见 [NATIVE.md](native/rust-anything/NATIVE.md)。

### 命令行模式

```sh
pnpm dsh web                          # 启动 Web UI，等同于 dsh --profile web
pnpm dsh headless "跑一下测试"         # 一次性任务，打印结果后退出（需 DEEPSEEK_API_KEY）
pnpm dsh --profile acp                # 通过 ACP stdio 为自动化客户端提供服务
pnpm dsh --profile sdk                # 以 JSON-RPC SDK 服务方式启动
pnpm dsh web --patch ./extra.yml      # 在 profile 之上再叠一层配置覆盖
pnpm dsh web --dump-config            # 打印合成后的插件树后退出，用于排障
pnpm dsh plugin --profile <名称> add <包名>   # 给已有 profile 安装插件
```

`web`、`headless`、`acp`、`sdk`、`sdk-minimal` 这些 profile 首次使用时会从随附模板初始化。桌面端用 `pnpm run dev:desktop`（先构建再启动）或 `pnpm run start:desktop`（复用已有产物）。各模式自己的参数用 `pnpm dsh <名称> --help` 查看；Python 集成见 [Python SDK 指南](docs/user/guide/python-sdk.zh.md)，其余命令见 [CLI 包说明](apps/cli/README.zh.md)。

## 查找插件

社区插件目录：[DSH Marketplace](https://dshmarketplace.dev/)。它汇总 DeepSeek Harness 的第三方插件，提供安装命令、安装验证与公开 API。要在 DSH 内浏览：打开**插件**，在**实验性插件**下启用**自定义插件索引**，再到**发现**标签页点击**添加索引源**，填入 `https://dshmarketplace.dev/market/index.json`。列表不代表安全审查，安装前请阅读源码，并优先在一次性 profile 中验证；索引用法见[插件索引指南](docs/user/guide/plugin-market.zh.md)。

## 开发

请先阅读[开发指南](docs/development.zh.md)与[架构文档](docs/architecture.zh.md)。

`make help` 列出 Web 与 Desktop 对应的 Make target；完整表格见开发指南的「应用命令」一节。

面向 agent：请遵循 [AGENTS.md](AGENTS.md)。

## 与上游的差异

对比基线为上游 [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness) 2026-11-18 的 `master`（`0.2.2-rc.5` 发布线），对比时间为 2026-11-19。`main` 相对该基线有 108 个提交、1439 个文件（+288,891/−1,045 行），行数主要由两个内置原生包 `apps/web/native/dsh-im` 与 `apps/web/native/dsh-superagent` 构成。上游继续前进后需要重新比对。本次同步的过程、冲突处理与验证见 [`logs/00063`](logs/00063-同步上游0.2.2-rc.5.md)，上一次同步见 [`logs/00057`](logs/00057-同步上游0.2.2-rc.4.md)；逐项实现与校验记录见 `logs/`；需要迁移的破坏性变更另有升级指南。

2026-11-17 起新增的原生 IM 机器人（含 `/goal`、`/plan`）及 Slack/Telegram/Discord/WhatsApp 四渠道移除未计入上述统计，见 [`logs/00027`](logs/00027-原生集成dsh-im.md)、[`logs/00028`](logs/00028-移除四渠道.md)、[`logs/00033`](logs/00033-移除Matrix与Office.md) 与 [`logs/00037`](logs/00037-移除企业微信应用渠道.md)。

### 新增与行为调整

| 特性 | 行为 | 源码与文档 |
|---|---|---|
| 启动片头 | Web 与 Desktop 启动时默认播放：先演六幕档案序列（授权对白、鲸标、身份卡、进度条与音效），再播放片头视频；默认可用按钮或 Esc 跳过。设置页可调 36 项序列参数与 6 项播放参数（含阶段顺序、播放时机、适配方式、随机与选中片头，可只播其一）；每个会话可要求重播或固定片头。内置 4 个片头（约 8.7 MiB）与 6 条配音，也支持 `$DSH_HOME/boot-animation/videos/` 中的自有视频。 | 新包 [`ui-boot-animation`](packages/client/ui-boot-animation/README.zh.md)：Host 端 `/dsh-boot-animation` 路由流式提供媒体（Range、ETag、owner-only），浏览器端负责播放与设置页；挂载于 `dsh-web-app`。Windows 标题栏按钮在片头期间隐藏。 |
| 永久删除会话 | 侧栏行悬浮垃圾桶按钮与会话菜单的「Delete session…」打开确认框；确认后删除该 Session 的全部已提交版本（generation），以及 Workspace 账户、归档与置顶引用，并向其他客户端广播；操作不可撤销。运行中的工作、子智能体会话、仍被本控制器之外的 Agent 或存储句柄占用的会话会被拒绝并说明原因。 | [`session/delete`](packages/api/session-controller/src/delete.ts)、[`SessionPersistence.remove`](packages/session/session-persistence/src/index.ts)、`WorkspaceRegistry.forgetSession`、[删除对话框](packages/client/ui-workspace/src/client/session-actions/DeleteSession.tsx)；[升级指南](docs/upgrade-guide/v0.2.2-rc.3/session-deletion/guide.zh.md)。 |
| Composer 文件夹输入历史 | 空草稿时 ↑ 召回当前文件夹最近一条，↑/↓ 前后浏览，↓ 越过最新一条清空；每个文件夹保留最近 100 条。只把成功提交时的文本写入历史，附件、引用 chip 等结构化内容不进入历史；相邻重复去重；多行条目只有在首行/末行才离开光标，斜杠菜单、IME、带修饰键的方向键与附件不触发。 | [`history.ts`](packages/client/ui-conversation/src/client/input/history.ts) 使用 `dsh.conversation.input-history.v1.<cwd>` 存于浏览器 localStorage，配合 `facade.ts`、`keymap.ts`、`hub.ts`；[包契约](packages/client/ui-conversation/README.zh.md)、[用户指南](docs/user/guide/index.zh.md)。 |
| 消息编辑并重发 | 用户消息的 Copy 旁新增 Edit（纯文本可编辑）；弹窗预填原文，确认后从该消息所在 Turn 之前 fork 出新会话并发送编辑文本。源会话的消息不被替换，后续消息留在源会话。首条消息改为在源 Workspace 新建 Session；目标 Turn 运行中先取消；发送失败时打开新会话并恢复编辑文本；附件、非文本或未加载 Turn 起点显示禁用按钮并说明原因，子智能体会话或缺少所需权限时不提供编辑入口。 | [`edit-resend.ts`](packages/client/ui-chat/src/client/chat/edit-resend.ts)、[`EditMessageDialog.tsx`](packages/client/ui-chat/src/client/chat/EditMessageDialog.tsx) 与 Chat slots/locale；[包契约](packages/client/ui-chat/README.zh.md)。 |
| Markdown 表格收进正文列 | 表格不再突破到 transcript 两侧：4 列及以上表格先按自然宽度排版，超过正文列就换行收敛；只有各列 100px 可读下限之和超过容器时，才在表格内部横向滚动。所有 `MarkdownText` 宿主（对话、工具卡、插件文档等）行为一致。 | [`MarkdownText.module.css`](packages/client/ui-primitives/src/markdown/MarkdownText.module.css) 使用 `width: fit-content; max-width: 100%`；删除聊天侧宽表突破规则；[Agent Note](.agents/notes/implemented/bug-fix/2026-11-15-markdown-tables-fit-the-column.md)。 |
| 后台 Web 服务 | `make web-bg` 在 tmux 会话 `dsh-web` 中启动 Web UI，日志写入 `/tmp/dsh-web.log`（权限 600，含 token），最多等待 45 秒以打印带 token 的 URL，未出现时提示查看日志；`make web-stop` 结束会话，会话不存在时幂等退出。 | [`Makefile`](Makefile) 的 `web-bg`/`web-stop`；[开发指南](docs/development.zh.md)。 |
| 提供商登录入口 | 模型设置页对「没有密钥可填」且 Host 提供登录流程的路由（当前为 `openai-codex`）显示登录按钮而不是密钥框：弹窗逐条显示流程通知（要打开的页面、设备码），回答问题（如浏览器登录 / 设备码二选一），凭据写入本地凭据存储后该路由立即可用。每个凭据 key 同时只运行一次登录；关闭弹窗即撤回，刷新页面会接回仍在运行的那次，Host 已不持有的尝试报告为「已结束」并可重试；登录刚确认时取消不会把成功显示成取消。 | [`AuthorizationController`](packages/api/settings-controller/src/authorization.ts)（`authorization` Remote namespace）、[`sign-in.ts`](packages/client/ui-settings-models/src/client/sign-in.ts)、[`ProviderSignIn.tsx`](packages/client/ui-settings-models/src/client/ProviderSignIn.tsx)；[用户指南](docs/user/guide/providers.zh.md)、[包契约](packages/client/ui-settings-models/README.zh.md)、[日志](logs/00035-接通提供商登录入口.md) |
| 内置工程工作流技能 | 每个以 base 为基础的 profile 自带 27 个工程工作流技能：全部可在输入框的 `/` 菜单里点名，其中 11 个还会进入会话技能目录、由模型按任务加载，另外 16 个模型看不到、菜单里标着「仅用户」；技能名统一带 `eng-` 前缀，项目或用户目录里用上游原名的同名技能与它并存，只有同名 `eng-…` 才会覆盖；需要关闭时在 profile patch 里禁用 `skill-engineering` 行。技能文本取自上游 `mattpocock/skills` 的 `main`（MIT），并套用本仓库的 `skill` 工具名与「开新会话」措辞。 | 新包 [`skill-engineering`](packages/skill/skill-engineering/README.zh.md)（随 `dsh-base` 挂载）：[用户指南](docs/user/guide/engineering-skills.zh.md)、[子系统页](docs/subsystems/skills.zh.md)、[日志 00088](logs/00088-内置工程工作流技能.md)。 |
| rust-anything 迁移能力 | 默认 `standard` preset 的 agent 直接获得 16 个 `mcp__rust-anything__*` 工具与 3 个共享资源工具，用于按单元把 TypeScript/JavaScript、Python、C、Go、Ruby、Java、Kotlin 代码迁移到 Rust：`analyze` → `bridge` → `record` → 写 `port.rs` → `build`/`diff`/`bench`/`gate`（整应用走 `app_*`）。CLI 源码 vendored 在 [`native/rust-anything`](native/rust-anything/NATIVE.md)，`pnpm run build` 现场编译，不需要全局安装、也不需要下载预编译二进制；每次挂载约 7k tokens，`minimal`/`ptc` 不受影响。 | 新包 [`rust-anything`](packages/mcp/rust-anything/README.zh.md)，挂载于 [`standard.patch.yml`](packages/bundle/web-app/presets/standard.patch.yml)；[使用指南](docs/user/guide/rust-anything.zh.md)；技能 [`dsh-rust-anything`](.agents/skills/dsh-rust-anything/SKILL.md)（输入框 `/` 可点名）；[日志 00099](logs/00099-源码vendored集成rust-anything.md)、[集成报告](rust-anything-integration.zh.md)。 |
| pi-ai 目录升级到 1.6.0 | 模型目录随 `@earendil-works/pi-ai` 1.3.0 → 1.6.0：`openai-codex` 新增 GPT-6.1 Terra 与 GPT-6.2 Astra/Sol，移除被 Codex 后端对 ChatGPT 账号拒绝的 `gpt-5.5` 与 `gpt-5.3-codex-spark`；`openai`、`github-copilot`、`google`、`opencode-go` 等路由的 id 随之增删；服务端 `anthropic` 路由默认改用 message threads；新增 `qwen-token-plan-individual-cn` 路由与若干网关 compat 开关 | [`packages/llm/llm-pi-ai`](packages/llm/llm-pi-ai/README.md)、[`升级指南`](docs/upgrade-guide/v0.2.2-rc.5/pi-ai-catalog-model-removals/guide.zh.md)、[`logs/00110`](logs/00110-pi-ai升级到1.6.0.md) |

### 移除或收紧的能力

| 项目 | 变化 | 影响与替代 |
|---|---|---|
| 30 个 pi-ai 内置路由 | `amazon-bedrock`、`ant-ling`、`azure`、`baseten`、`cerebras`、`cloudflare-ai-gateway`、`cloudflare-workers-ai`、`fireworks`、`google-vertex`、`groq`、`huggingface`、`meta`、`minimax`、`mistral`、`moonshotai`、`moonshotai-cn`、`nvidia`、`opencode`、`openrouter`、`qwen-token-plan`、`qwen-token-plan-cn`、`qwen-token-plan-individual`、`radius`、`together`、`vercel-ai-gateway`、`xai`、`xiaomi`、`xiaomi-token-plan-sgp`、`xiaomi-token-plan-ams`、`zai` 不再出现在提供商目录、发现、登录与模型解析中。 | pi-ai 目录仍提供 11 个路由、175 个模型；pi-ai 包本身仍带这些路由的元数据，本构建只是屏蔽。存量 profile 点名被移除路由会使 `llm-pi-ai` 加载失败，删除该条目后其余路由恢复；需要继续使用原服务时可按[升级指南](docs/upgrade-guide/v0.2.2-rc.3/llm-pi-ai-removed-providers/guide.zh.md)改用其他路由，或把兼容端点添加为自定义模型 API。按模型 id（去掉厂商前缀并忽略标点）或显示名核对：`azure` 的 45 个条目全部由 `openai` 承接，`minimax` 的三个由 `minimax-cn` 承接，`google-vertex` 的 14 个在 `google` 上以相同 id 提供（一个输出上限、一个缓存价与三个条目的阶梯价不同，且其协议不能作为自定义协议接入），`zai` 的 7 个中 3 个由 `zai-coding-cn` 承接、`glm-5.2` 另见 `opencode-go`；`radius` 的 29 个中 26 个仍在内置路由上（其余三个是意图别名），`opencode` 84 个中剩 58 个、`cloudflare-ai-gateway` 57 个中剩 44 个、`amazon-bedrock` 193 个中剩 26 个、`huggingface` 78 个中剩 15 个、`vercel-ai-gateway` 301 个中剩 87 个、`baseten` 22 个中剩 7 个、`together` 17 个中剩 7 个、`fireworks` 22 个中剩 6 个、`mistral` 40 个中剩 3 个、`nvidia` 20 个中剩 3 个、`qwen-token-plan`（含 `-cn`）20 个中剩 8 个（个人版 9 个中剩 5 个）；`ant-ling` 的 3 个、`cerebras` 的 2 个与 `groq` 的 7 个没有其他内置路由。`meta` 的 6 个 Muse 模型中，两个 `-contributor` 条目在 `opencode-go` 上以相同 id、能力与目录价格提供，其余 4 个没有其他内置路由，需要时把 `api.meta.ai/v1` 接成 `openai-responses` 自定义模型 API。`xai` 的 4 个 Grok 模型中，`grok-4.5`、`grok-4.6` 与 `grok-4.7` 也在 `github-copilot` 上以相同 id 提供（该路由的输出上限为 128,000，`xai` 记录为 500,000；`grok-4.5` 的缓存读取价在该路由更高），`grok-4.7` 另在 `opencode-go` 上以相同 id、能力与目录价格提供，只有 `grok-4.3` 没有其他内置路由；需要 xAI 直连时把 `https://api.x.ai/v1` 接成 `openai-responses` 自定义模型 API。`openrouter` 是移除面最宽的一条：396 个 id（除裸 `auto` 外都带厂商前缀）中 95 个与已提供路由的模型 id 相同、2 个按显示名对应，其余 299 个没有其他内置路由（18 个 `~…-latest` 浮动别名、14 个 `:free` 变体、5 个 `openrouter/*` 路由 id 与 260 个厂商条目，其中 77 个是 `:batch` 变体），381 个条目的端点 `openrouter.ai/api/v1` 与 15 个条目的 `openrouter.ai/api`（Anthropic Messages）可分别作自定义模型 API 接入；`together` 的 `api.together.ai/v1`、`groq` 的 `api.groq.com/openai/v1`、`mistral` 的 `api.mistral.ai/v1`、`baseten` 的 `inference.baseten.co/v1` 与 `nvidia` 的 `integrate.api.nvidia.com/v1` 同样如此，`fireworks` 的 `api.fireworks.ai/inference` 可作 Anthropic Messages 自定义模型 API。`deepseek-official` 与 `deepseek-account` 原生路由不受影响。 |
| 宽表突破布局 | 4 列及以上 Markdown 表格不再横向撑满 transcript；`md-table-wide` 只保留滚动条揭示与键盘聚焦语义，不再决定宽度。 | 表格与段落、标题使用同一条宽度轴；长表换行，极多列或窄视口才在表格内部滚动。相关 e2e 与金标已改为断言该几何。 |

### 工程、测试与文档

| 项目 | 内容 |
|---|---|
| 新包与集成 | 新增 `@deepseek-ai/dsh-client-ui-boot-animation`（78 个文件，Host + Client 双面，自带 locales 与 assets），加入 `dsh-web-app` bundle 的启动项与 workspace 资产约束（`assets/`）。 |
| 持久化契约 | `SessionPersistence` 新增必需抽象方法 `remove` 与 `SessionPersistenceBusyError`；JSONL 后端删除整个 session 目录并持有 write lease；共享契约新增删除用例。自定义后端需实现该方法。 |
| 测试 | 新增 `session-delete.e2e.ts` 与 `composer-input-history.e2e.ts`；更新 `message-actions`、`markdown-wide-table`、`models-settings`、`sidebar-alignment` 等 e2e，以及大量 Client 单测与 Web 金标（用户消息新增 Edit、设置页移除 30 个 option 等）。 |
| 文档与记录 | 新增 2 组中英双语升级指南、3 篇 Agent Note 与多项 `logs/` 变更记录；更新 session、persistence、workspace、web-client 子系统页，config/persistence 生成目录，providers 与 Web UI 用户指南，以及开发指南。 |
| 仓库发布面 | 根 README 改为中文单语：原英文 `README.md` 被中文版替换，删除 `README.zh.md` 与 `README.i18n.yaml`，不再保留上游的 Community、Contributing、Citation 与 npm 运行章节，并改写为本仓库的 clone/build/run 流程。删除根 `LICENSE`、`CONTRIBUTING.md`、`CONTRIBUTING.zh.md` 及配对记录；根 `package.json` 仍声明 MIT，第三方依赖许可保留在 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。包级 README 仍按中英成对维护，根 README 在 `scripts/translation-pairing.manifest.json` 中排除。 |
| 权限与兼容性 | LiveShare 将 `session/delete` 归类为 `owner-only`，viewer 角色不可调用。改动未触及 agent loop、模型可见工具或 Session 日志格式；启动片头不注册工具、prompt 段或 Session 事件，输入历史只存在浏览器。 |

### 原生 IM 机器人

- 社区插件 `dsh-im` 已以源码形式内置到 `apps/web/native/dsh-im/`（基线 5.10.0），并作为 `dsh-web-app` bundle 的默认依赖挂载；`dsh web` 与 Desktop 首次启动即出现「IM bots」设置页，不需要再安装外部插件。内置渠道为微信、飞书、钉钉、企业微信、QQ、iMessage 和邮箱；Slack、Telegram、Discord、WhatsApp、Matrix、企业微信自建应用以及 AI Office 连接器不包含在这个原生构建中。
- 内置副本适配 DSH `0.2.2-rc.5` 的客户端模块名与服务访问方式，修正了 Cordis 4.1 下 `ctx.root` 的无注入访问问题，并关闭了针对上游 npm 包的自更新，避免把内置实现替换成外部版本；构建与测试在 rc.4、rc.5 同步后无需修改即通过，仅把 `package.json` 的 DSH 兼容版本与发布清单跟进到 rc.5。
- `/goal` 与 `/plan` 已提升为所有内置渠道的原生命令：命令直接委派给当前会话绑定的 DSH Host 命令网关，复用 DSH 的持久化目标与自动续跑，不经过模型提示词。
- 上游同步、构建与本地修改记录见 [`apps/web/native/dsh-im/NATIVE.md`](apps/web/native/dsh-im/NATIVE.md)；变更记录见 [`logs/00027`](logs/00027-原生集成dsh-im.md)、[`logs/00028`](logs/00028-移除四渠道.md)、[`logs/00033`](logs/00033-移除Matrix与Office.md) 与 [`logs/00037`](logs/00037-移除企业微信应用渠道.md)。

### 原生 Superagent

- 研究原型 `nano-dsh-superagent-prototype` 已以源码形式内置到 `apps/web/native/dsh-superagent/`（基线提交 `8c32a9d`），并作为 `dsh-web-app` bundle 的默认依赖挂载；`pnpm install && pnpm run build` 后 `dsh web` 与 Desktop 侧栏自动出现「Superagent」，不需要再 clone 原型仓库、单独 profile 或 `dsh plugin add`。重新构建后重启正在运行的 `dsh web`，插件在启动时加载。
- 使用步骤（面板配置、发起请求、跟进/暂停/重试、主动模式与飞书启用）见 [内置副本快速开始](apps/web/native/dsh-superagent/docs/006-deployment.md#quick-start) 与 [NATIVE.md](apps/web/native/dsh-superagent/NATIVE.md)。
- 默认启用本地工单；飞书 Source 默认挂载，在 dsh-im 飞书机器人的「交给 Superagent」打开后接管该机器人的消息（文本、富文本与图片），并按游标补收重启期间漏掉的消息，关闭即回到普通对话。不需要任何远端账号。本版本没有 GitHub Source：不连接仓库、不创建 PR、不做 PR CI，评审和合并都在本地完成。在「执行 → 新建工单」里写请求即可开工，也可以直接在主会话里说明要做的事（面板右上角「打开主 Session」；由主会话记录成工单，接单后报出工单号），每个工单归档为 Markdown 文件（默认 `<stateDir>/tickets/<请求 id>.md`），请求、回复与结果都写在里面，工单页显示请求全文与归档文件路径，「回复与结果」列出同一批消息，并提供「回复工单」表单用于回答代理提问或补充说明；工单的提问也可以在主会话里直接回答。状态目录默认 `<DSH_HOME>/state/superagent`。
- 工单还支持：结束时的 git 审查（分支/提交/变更文件/未提交改动）与按需 diff、合并（快进/压缩/合并提交，有未解决评审备注、评审要求修改、检查未通过时拒绝，可配置必须批准）、落后时变基、丢弃或清理已合并的 worktree、编辑标题、标签与请求正文、重新打开、结束后的稳定编号 `T-000N`、新建时「先审阅再执行」的批准门、最多 5 个附件（8 MB）、标签筛选、项目级执行根/prepare 命令、结束时检查命令（配置后合并要求通过）、关联工单、按文件/行评审备注（可回复讨论、可标记已解决、可一次发给执行器）、评审结论（批准/要求修改，可配置合并前必须批准）、操作历史与列表批量停止/清理/改标签，来源级通知 Webhook，以及面板通知收件箱（未读数、跳转、全部已读）。搜索支持 `is:`、`tag:`/`label:`、`author:`、`created:` 与 `updated:`。命令行可用 `scripts/dsh-superagent ticket` 投递请求、`backup` 在线备份数据库与归档；细节见 [006 deployment](apps/web/native/dsh-superagent/docs/006-deployment.md)。
- 内置副本适配 DSH `0.2.2-rc.5`：Session 转录优先读 live Agent、未加载时回退持久化日志；设置表单只读判断改用 `writable`；`stateDir` 默认 `<DSH_HOME>/state/superagent` 并兼容 Windows 绝对路径；模型选择测试按 0.2.2 流程重写；修复飞书事件消费的 health 竞态。同步 rc.4 时补上 Loader 1.0.0-rc.9 的 `applyEntryPatches(...).data` 读取。同步 rc.5 无需改动：`workspace-write` 边界改为跟随 Session 目录后，工单 Session 仍以 `danger-full-access` 创建，`git commit` 不受影响；连接器、Forge 评审页与客户端 Config 下发改动都在它之外。
- 管理 API 跟随 Web 绑定地址（默认 `127.0.0.1`，含 `127.0.0.2`、`::1`、容器/LAN 地址），仍受 DSH Web 认证、`x-orchestrator-request` 与同源检查保护。
- 上游同步与本地修改记录见 [`apps/web/native/dsh-superagent/NATIVE.md`](apps/web/native/dsh-superagent/NATIVE.md)；变更记录见 [`logs/00038`](logs/00038-原生集成dsh-superagent.md)、[`logs/00041`](logs/00041-本地工单系统.md)、[`logs/00042`](logs/00042-工单审查通知与重开.md)、[`logs/00044`](logs/00044-工单对账重命名与标签筛选.md)、[`logs/00045`](logs/00045-本地工单附件.md)、[`logs/00046`](logs/00046-工单批准门.md)、[`logs/00047`](logs/00047-工单Webhook通知.md)、[`logs/00048`](logs/00048-项目执行配置.md)、[`logs/00049`](logs/00049-修复审计发现.md)、[`logs/00051`](logs/00051-工单全文变基与清理.md)、[`logs/00052`](logs/00052-工单检查命令.md)、[`logs/00053`](logs/00053-搜索批量关联与历史.md)、[`logs/00054`](logs/00054-工单评审备注.md) 与 [`logs/00055`](logs/00055-修复复核问题.md) 与 [`logs/00056`](logs/00056-文档与日志同步.md) 与 [`logs/00059`](logs/00059-合并检查门标签与评审备注.md) 与 [`logs/00060`](logs/00060-请求编辑备注回复合并策略与通知.md) 、[`logs/00061`](logs/00061-复核续做检查与squash清理.md)、[`logs/00062`](logs/00062-主会话建单与回答.md) 、[`logs/00063`](logs/00063-同步上游0.2.2-rc.5.md) 与 [`logs/00064`](logs/00064-清理hygiene遗留.md)。

## 许可证

第三方依赖及其许可证见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

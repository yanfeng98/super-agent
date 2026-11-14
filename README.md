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

## 开发

请先阅读[开发指南](docs/development.zh.md)与[架构文档](docs/architecture.zh.md)。

`make help` 列出 Web 与 Desktop 对应的 Make target；完整表格见开发指南的「应用命令」一节。

面向 agent：请遵循 [AGENTS.md](AGENTS.md)。

## 许可证

第三方依赖及其许可证见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

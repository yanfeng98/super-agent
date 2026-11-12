# 00003 — 根 README 改为本仓库（fork）的克隆与运行方式

- **日期**：2026-11-12
- **背景**：后续在本仓库（`yanfeng98/super-agent`）魔改，不发布到 npm。根 README 仍按上游发布形态给出 `npx @deepseek-ai/dsh web` 作为首选运行方式，克隆命令也指向 `deepseek-ai/deepseek-harness`，与本仓库的实际用法不符。
- **范围**：只改根 `README.md`。

## 1. 内容变更

| 原内容 | 现内容 |
|---|---|
| 「运行」下首选「通过 `npm` 运行」，命令为 `npx @deepseek-ai/dsh web` | 「运行」首段说明本仓库是个人魔改分支、以本仓库代码和本地构建产物运行、不发布 npm |
| 克隆 `https://github.com/deepseek-ai/deepseek-harness.git`、`cd deepseek-harness` | 克隆 `https://github.com/yanfeng98/super-agent.git`、`cd super-agent` |
| 未说明运行时用源码还是构建产物 | 补充：首次运行前必须构建（浏览器端 `apps/web/dist` 与 client bundle），之后宿主端改动重启进程即可，浏览器端改动需要重新构建 |
| 无日常开发入口 | 新增「日常开发」：`dev:web` 构建并监听重建，`start:web` 等价于 `pnpm dsh web` |
| 无 npm 版本说明 | 新增「官方 npm 版本」：`npx @deepseek-ai/dsh web` 运行官方发布包，不读取本仓库源码 |

## 2. 未改动的位置

`docs/user/guide/python-sdk.md`、`docs/cordis-tutorial/index.md` 等教程页仍给出上游克隆地址。它们是上游教程的正文，改成本仓库地址会与上游同步时的正文产生冲突；需要时再单独处理，并同步更新对应 `*.zh.md` 与 `*.i18n.yaml` 哈希。

`packages/*/package.json` 的 `repository` 字段保持上游地址：它们只影响发布元数据，本仓库不发布，且 `scripts/check-workspace-constraints.ts` 以该地址为发布约束的基准值。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm run test:docs` | 21 个文档门禁全部通过（含 markdown links、markdown wrap、doc budgets、doc refs、translation pairing） |
| `DSH_GATE_CONCURRENCY=1 pnpm run build:check` | host/client 两个聚合检查通过，说明现有产物与源码一致，文档里的 `pnpm run build` 可用 |
| `GIT_TERMINAL_PROMPT=0 git ls-remote https://github.com/yanfeng98/super-agent.git HEAD` | 可匿名访问，HEAD 与本机 `main` 一致 |
| `pnpm dsh web --no-open --port 3099` | 启动成功，日志打印带 token 的 `http://127.0.0.1:3099/?token=…`；不带 token 请求返回 401，带 token 返回 303，与预期一致 |
| `npm view @deepseek-ai/dsh version` | 发布包为 `0.2.2-rc.1`，落后于本仓库的 `0.2.2-rc.2` |

## 4. 第一次复查

- 补上 pnpm 前置条件：`package.json` 的 `packageManager` 为 `pnpm@11.28.5`，只写 Node.js 会让新机器卡在 `pnpm install`。
- 「日常开发」原写「重建浏览器端产物」会让人以为宿主端也自动生效：`scripts/dev-web.ts` 只监听 `dsh.client` 插件包（`discoverPluginDirs`）与被链接的库包（`discoverLibraryDirs`），宿主端包不在其中，watcher 的产物也不会重载已启动的 Node 进程。改为按浏览器端与宿主端分别说明（宿主端的结论见第 5 节）。
- 「开发」一节原有的 `dev:web` 描述与新增的「日常开发」重复，删除重复句，只保留 `make help` 与开发指南链接。
- 「官方 npm 版本」补充版本落后的事实：registry 为 `0.2.2-rc.1`，本仓库为 `0.2.2-rc.2`。

## 5. 第二次复查：运行时的代码来源

上一节之后仍把「改动源码后需要重新执行 `pnpm run build`」当成通例，实际宿主端与浏览器端不同：

- [开发指南「应用命令」](../docs/development.md)写明 Web 的 Host 从源码运行，浏览器加载构建出的 bundle。
- `tsconfig.base.json` 的 `paths` 把 `@deepseek-ai/dsh-*` 映射到各包 `src`，根 `tsconfig.json` 的注释说明 tsx 正是通过它解析工作区导入。
- 实测：在 `packages/llm/llm/src/index.ts` 顶部临时加入 `console.log` 标记，不重新构建直接 `pnpm dsh web --no-open --port 3099`，标记出现在启动日志中；随后还原该文件，`git status` 无残留。

结论：首次运行前必须构建（浏览器端 `apps/web/dist` 与 client bundle 不会由源码提供），之后宿主端改动重启进程即可生效，只有浏览器端改动需要重新构建；重启 `dev:web` 时用 `--skip-build` 可跳过启动构建、复用已有产物。「运行」与「日常开发」两节已按此改写。

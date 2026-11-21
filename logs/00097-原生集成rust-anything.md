# 00097 — 原生集成 rust-anything 迁移能力

> 后续：[`logs/00099`](00099-源码vendored集成rust-anything.md) 把 CLI 从 npm 依赖改为仓库内 vendored 源码 + 现场编译；[`logs/00101`](00101-技能改名-dsh-rust-anything.md) 把技能目录从 `.agents/skills/rust-anything/` 改名为 `dsh-rust-anything`。本文记录的注册、挂载点与 preset 取舍仍然有效。

- **日期**：2026-11-21
- **背景**：用户要求 clone 本仓库即可使用 [rust-anything](https://github.com/erkamyaman/rust-anything)，不要 `npm i -g` / `cargo install` 这类独立安装，也不要每台机器手改 profile。目标形态是：`pnpm install && pnpm dsh web` 之后，Web 默认会话直接能调用它的迁移工具。
- **范围**：新增 `packages/mcp/rust-anything`；`packages/bundle/web-app`（依赖 + `presets/standard.patch.yml`）；`.agents/skills/rust-anything/`；`web-app` 之外还改了 `tsconfig.base.json`、`tsconfig.host.json`、`pnpm-workspace.yaml`、`pnpm-lock.yaml`、`docs/module-graph*.md`、`packages/mcp/README*`。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/mcp/rust-anything/`（新包 `@deepseek-ai/dsh-rust-anything`） | 插件 `src/index.ts`：`bundledBinary(platform, arch)` 用 `createRequire(import.meta.url)` 从本包依赖解析 `rust-anything` 的 `binaries/<platform>-<arch>/rust-anything[.exe]`；`resolveCommand` 依次取 `config.command`、`RUST_ANYTHING_BIN`、该路径；`apply` 用一个 `dsh-mcp-client` 实例挂载 `serverName: rust-anything`、`args: ['mcp']`。`Config` 提供 `command`/`toolLoading`/`toolCallTimeoutMs`（默认 15 分钟）/`startupTimeoutMs`/`env`。 |
| `packages/mcp/rust-anything/package.json` | `dependencies`：`rust-anything@2.1.0`（精确版本，五平台预编译二进制随 npm 包分发）；`peerDependencies`：`@deepseek-ai/dsh-mcp-client`（插件共享实例约定，发布安装不会带副本）；`devDependencies`：`cordis`、`dsh-mcp-client`、`dsh-system-prompt`、`dsh-tools`；`exports`/`files` 增加 `locale/*.json`。 |
| `packages/mcp/rust-anything/tests/mount.spec.ts` | 连接真实二进制：断言平台二进制路径、`command`/环境变量/兜底三条解析分支，以及 eager 下 16 个工具、on-demand 下仅 `load_tools`。 |
| `packages/mcp/rust-anything/locale/{en,zh,ja,es,fr,de,ru,pt}.json` | 包级 `meta.title`/`meta.description` 八语言元数据（`verify-locale-metadata` 要求）。 |
| `packages/mcp/rust-anything/README.md` / `.zh.md` / `.i18n.yaml` | 包契约：配置字段、eager/on-demand 的 token 与 KV cache 影响、已知限制（平台二进制清单、生成 crate 的 1.97 MSRV、上游相对路径 bug、每 agent 一个进程）。 |
| `packages/bundle/web-app/package.json` | 依赖 `@deepseek-ai/dsh-rust-anything: workspace:*`。 |
| `packages/bundle/web-app/presets/standard.patch.yml` | 在 `standard` preset 的插件列表插入 `mcp-rust-anything` 行，显式 `config.toolLoading: eager`，并注明每次挂载约 7k 请求 token。 |
| `.agents/skills/rust-anything/{SKILL.md,whole-app.md}` | 上游技能（MIT）随仓库分发；`SKILL.md` 末尾补「In this checkout」三条本仓库须知（绝对 crate 路径、Rust ≥1.97、执行的是被录制代码）。 |
| `tsconfig.base.json` / `tsconfig.host.json` | 新包的路径映射与 Host 工程引用。 |
| `pnpm-workspace.yaml` / `pnpm-lock.yaml` | `minimumReleaseAgeExclude` 增加 `rust-anything@2.1.0`；锁文件记录 2.1.0 及其完整性哈希。 |
| `docs/module-graph.md` / `.zh.md` | `gen-module-graph` 重新生成：新增 `rust-anything` 节点、包表行与 `rust-anything → mcp-client` 依赖边。 |
| `packages/mcp/README.md` / `.zh.md` / `.i18n.yaml` | 包组表新增一行。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 全新 clone + `pnpm install && pnpm dsh web` | 默认 `standard` preset 的 agent 直接获得 16 个 `mcp__rust-anything__*` 工具 + 3 个共享资源工具（`list_mcp_resources`、`list_mcp_resource_templates`、`read_mcp_resource`）；不需要全局安装，也没有 profile 手工步骤。 |
| `minimal` / `ptc` preset | 工具列表不变（实测无泄漏）。 |
| headless / acp / sdk profile | 默认不挂载（这些 profile 不使用 Web preset；见「已知限制」）。 |
| 系统提示 | agent 的提示新增 server instructions 段与 `## MCP resource servers` 名单；`minimal` 不受影响。 |
| 每次挂载成本 | 约 21 KB（≈6k tokens）的 16 个 schema + 3 个资源工具 schema + instruction 段，合计约 7k tokens；每个使用该 preset 的 agent 各自启动一个 server 进程。 |
| CLI 解析失败（不支持的平台 / 安装损坏） | `mcp-client` 记录连接失败，agent 创建不受影响（`failOnStartupError` 默认 false），只是没有工具。 |
| 生成 crate 的构建 | 需要 Rust ≥1.97（生成的 `Cargo.toml` 声明 `rust-version = "1.97"`）；`rust-anything doctor` 会报当前工具链。 |
| 工具调用超时 | 默认 15 分钟，覆盖 `gate`/`build`/`bench`/`app record` 这类长任务。 |

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm vitest run packages/mcp/rust-anything` | 4/4 通过（eager 16 个工具、on-demand 仅 loader、三条解析分支、平台二进制命名） |
| `pnpm run build` | 12/12 gate（host/client compiler、bundle、desktop、web、artifacts + check） |
| `pnpm exec tsc -b tsconfig.host.json` | 退出码 0 |
| 覆盖率（只统计本包 `src`） | 语句/分支/函数/行均 100% |
| `pnpm exec tsx scripts/run-oxlint.ts packages/mcp/rust-anything` | 0 warnings / 0 errors |
| `pnpm run verify-product-use` / `verify-default-product-isolation` / `constraints` | 通过（299 个默认可达包；363 个 Web 组合插件不含实验包） |
| `pnpm run verify-cordis-config` / `verify-cordis-catalog` | 377 个配置文件、133 个生成区通过 |
| `pnpm run verify-module-graph` | 重新生成后通过 |
| `pnpm run verify-locale-metadata` | 八语言元数据完整 |
| `pnpm run verify-package-dependencies` / `verify-node-next-types` / `publint` | 通过（103 个包依赖策略、403 个声明 API、全部包 publint） |
| `pnpm run verify-package-readme-{model-experience,limitations,summaries}` / `verify-translation-pairing` / `verify-md-wrap` / `verify-doc-refs` | 全部通过（1362 对双语、2785 个文件折行、5433 个引用） |
| `pnpm install --frozen-lockfile` | 通过（精确锁 2.1.0） |
| `apps/cli/tests/web-agent-presets.e2e.ts`（真实 boot Web 组合 + 假模型创建 agent） | `standard` agent 收到 16 个 MCP 工具 + 3 个资源工具；`minimal` 无。该文件在本 fork 上本来就是红的（额外 superagent preset、3 个 IM 工具、skill 目录漂移）；移除本行后重跑，失败集合逐条 `diff` 为空，故本次改动未新增失败用例。 |
| `pnpm vitest run --config vitest.snapshot.config.ts` | 270 通过 / 6 失败 / 2 跳过；6 个失败是 `ptc-*` 与 `worktree` 的本机环境问题（headless 组合中无此插件，改动不可达）。 |
| `git diff --check` | 通过 |

## 4. 设计取舍

- **挂在 Web 的 `standard` preset，而不是 base bundle**：MCP server 会给系统提示增加 instructions 段与资源名单。挂 base 会让 headless/acp/sdk 的模型可见提示一起变化，`snapshots/session` 的 198 个期望文件都要重录；挂 preset 只影响 Web 会话，keyless 快照套件保持不变，且实测不会泄漏到 `minimal`/`ptc`。
- **`toolLoading: eager`**：on-demand 会往系统提示加一段工具目录、并在使用前多一次 `load_tools` 调用；eager 让工具列表稳定、开箱即用。代价是约 7k tokens/agent，已写在 preset 注释与包 README 里，需要省 token 时改一行即可。
- **npm 依赖而不是全局安装**：`rust-anything` npm 包自带 macOS/Linux/Windows 五平台二进制，`pnpm install` 一次到位；插件从自身依赖解析路径，不依赖 `PATH`，也不需要每台机器改 profile。
- **`dsh-mcp-client` 用 peer**：插件之间共享实例是仓库约定；发布安装不会带第二份 `mcp-client`，模块图也会记录依赖边。
- **不用 `mcp-manager` 代挂**：manager 面向 Host 级、可运行时增删的服务器；这是一条随 preset 分发的固定集成，按 agent 挂载更贴合 `standard` 的作用域。
- **技能放在仓库 `.agents/skills`**：clone 后项目根技能目录即可发现（本会话验证了热发现），不需要跑上游的 `setup`（那会改 Claude Code/Codex 等配置）。

## 5. 复查记录（2026-11-21）

三轮复查，全部围绕"改动是否与事实一致、是否真正生效"：

| 轮次 | 发现 | 处理 |
|---|---|---|
| 1 | 包 README 写错挂载点（写成 base bundle）、错误宣称"不贡献系统提示文本" | 按实测改写中英文 README：instructions 段、资源名单、token 影响 |
| 1 | 依赖写 `^2.1.0`，与 `minimumReleaseAgeExclude` 的精确条目不一致，`--frozen-lockfile` 直接失败 | 改为精确 `2.1.0`，重生成锁文件 |
| 1 | web-app 依赖表被重排、pnpm 重写了 `@openai/codex` 豁免条目 | 还原为最小 diff（各 1 行新增） |
| 2 | 缺少包级 locale 元数据，`verify-locale-metadata` 失败 | 补八语言 `locale/*.json` 与 `exports`/`files` |
| 2 | `dsh-mcp-client` 写成普通依赖，违反插件共享实例约定、模块图缺边 | 移到 `peerDependencies`（保留 devDependency），模块图重新生成 |
| 2 | `docs/module-graph*.md` 过期 | `gen-module-graph` 重新生成 |
| 3 | 无法证明 preset 里的行真的挂起来；preset 测试 harness 对本仓库任何 workspace 插件都报 `never started`（用 `dsh-tool-todo` 复现），不是有效证据 | 改跑真实 Web 组合的 `web-agent-presets.e2e.ts`，拿到 16 + 3 个工具的直接证据；并用 A/B 对照证明该文件的既有失败与本次改动无关 |
| 3 | 文档把成本写成 16 个 schema（约 6k），实际是 19 个工具（约 7k） | 修正包 README 中英文与 preset 注释；重记 i18n 哈希 |
| 4 | 为满足 README 结构门禁删掉了 Dev Note，导致 `doc-standard` 与锚点检查失败（00098 记录的「两个 rust-anything 失败」） | 按仓库既有写法把 `### Dev Note` 作为 `## Known Limitations and Deferred Work` 的 H3 补回（中英文同步）；中文摘要标题改为门禁要求的「概述」；重新记录双语哈希 |

## 6. 已知限制与后续

- **headless / acp / sdk 默认未挂载**：若要让 headless 也开箱可用，需要给 `snapshots/session` 的 198 个 `system-prompt.expected.md` 补 MCP 段（可脚本化，但应走带 key 的重录流程确认），或把这些 profile 也接入同一 preset 机制。
- **Web 端浏览器 e2e 的 system-prompt 期望**（`snapshots/web/*/system-prompt.expected.md`，需要 key）会多出 MCP 段，带 key 重录时更新；keyless 快照套件不受影响。
- **`verify-third-party-notices` 仍被 fork 自带的 `@tencent-connect/qqbot-connector`（缺 repository 字段）阻断**，生成器到不了新依赖；修好该包元数据后需 `pnpm run gen-third-party-notices` 把 `rust-anything`（MIT）写入 notices。
- **`apps/cli/tests/web-agent-presets.e2e.ts` 的期望表**若要修，`standard` 需补 16 个 `mcp__rust-anything__*` 与 3 个资源工具名；另外 3 个 IM 工具属于 fork 自身漂移。
- **本机 Rust 1.96 < 1.97**：`build/gate` 前需 `rustup update stable` 或 `rustup default 1.97.0`。
- **上游 2.1.0 的相对路径 bug**：`build/diff/gate/bench` 传相对 crate 路径会报 `manifest path ... does not exist`（`run` 把子进程切到临时目录却传下相对路径）；技能里已要求传绝对路径。
- **版本锁定**：`rust-anything` 精确锁 2.1.0，升级时需同时更新 `pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude`。

分析与集成细节另见仓库根目录的 `rust-anything-analysis.zh.md` 与 `rust-anything-integration.zh.md`。

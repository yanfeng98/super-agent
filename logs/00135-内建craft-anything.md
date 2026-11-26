# 00135 — 内建 craft-anything（源码 vendored）

- **日期**：2026-11-26
- **背景**：用户要求把 <https://github.com/storytold/craft-anything> 像其他项目一样集成进本仓库，作为开箱可用的内建能力，不需要用户自己 `cargo install` 或注册 MCP server。做法沿用 [00127 内建 rea](00127-内建rea.md) 与 [00128 内建 fast-anything](00128-内建fast-anything.md)：上游源码进仓库、由仓库自己的构建编译，插件与技能随 Web preset 挂载。
- **基线**：上游 tag `v0.6.1`，commit `48a9c5a60ff83ff620f98661339be6281a7debc6`（2026-11-11），MIT OR Apache-2.0。
- **范围**：`native/craft-anything/`（新 vendored 子树 + `NATIVE.md` + `rust-toolchain.toml`）、`scripts/build-craft-anything.ts`（+ spec）、根 `package.json`、`scripts/build.ts`、`scripts/run-gates.ts`（+ 相关 spec）、`packages/mcp/craft-anything/`、`packages/skill/skill-craft-anything/`、`packages/bundle/web-app/{package.json,cordis.patch.yml,presets/standard.patch.yml,tests/bundled-skills.spec.ts}`、`python/sdk-runtime/package.json`、`tsconfig.base.json`/`tsconfig.host.json`、`.gitignore`、`.github/actions/setup-rust/action.yml`、`pnpm-lock.yaml`、根 `README.md`、`native/README*`、`packages/{mcp,skill}/README*`、`craft-anything-integration.zh.md`、`logs/`。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `native/craft-anything/`（新） | 上游 `v0.6.1` 工作区，保留 `Cargo.toml`、`Cargo.lock`、全部 `crates/`、`apps/`、`xtask/`、`assets/`、`vendor/wgpu-hal`、双许可证、NOTICE、英文 README；剔除 `docs/`、`examples/`、`packaging/`、`.github/` 和其他语言 README。约 941 个文件 / 20 MB。 |
| `native/craft-anything/NATIVE.md`（新） | 记录基线、保留/剔除清单、构建、挂载、检查与上游同步流程；说明默认洁净室模式和 323 工具/16 工具集的能力面。 |
| `native/craft-anything/rust-toolchain.toml`（新） | 固定 Rust 1.97.0，让仓库默认的 1.96 toolchain 也能通过普通 `cargo` 选中受支持版本（与 rust-anything 相同做法）。 |
| `scripts/build-craft-anything.ts` + `.spec.ts`（新） | 在 vendored 工作区跑 `cargo build --release --locked -p craft-anything-cli`，把产物先复制到 `bin/*.tmp` 再 rename 暂存（避免 ETXTBSY）；失败信息指向 `NATIVE.md`。测试覆盖平台可执行文件名与 cargo/暂存路径。 |
| `scripts/build.ts`、根 `package.json` | artifacts 作用域新增 `craft-anything CLI` gate；新增 `build:craft-anything`；`test`/`test:coverage` 先跑它。 |
| `scripts/run-gates.ts`（+ spec） | coverage、coverage-shard、ci-unit 三个聚合各加 `craft-anything` gate，并让测试/覆盖率 gate 依赖它。 |
| `packages/mcp/craft-anything/`（新） | `@deepseek-ai/dsh-craft-anything`：`src/binary.ts` 解析暂存 CLI，`src/index.ts` 挂一个 `dsh-mcp-client` stdio 实例（`args: ['mcp']`）；默认 `on-demand`，15 分钟单次调用预算；8 种语言的 locale、中英 README 与 mount 用例（30 个初始工具）。 |
| `packages/skill/skill-craft-anything/`（新） | `@deepseek-ai/dsh-skill-craft-anything`：上游 `crates/setup/src/skill.md` + `skill_recovery.md`（MIT OR Apache-2.0）改名 `dsh-craft-anything`，把「自行注册 server」换成内建副本的工具集加载、按需 `load_tools` 与恢复说明；菜单分组 `内置 · storytold/craft-anything`。含双许可证、NOTICE、完整 provider 用例。 |
| `packages/bundle/web-app/presets/standard.patch.yml` | `standard` preset 新增 `mcp-craft-anything` 行（`on-demand`）。 |
| `packages/bundle/web-app/cordis.patch.yml`、Web bundle 与 SDK runtime 的 package.json | 宿主 patch 新增 `skill-craft-anything` 行；两个清单补三个新 workspace 依赖，满足构建可达性与 `verify-runtime-closure`。 |
| `tsconfig.base.json`、`tsconfig.host.json` | 生成器补路径映射；Host 工程引用手工补两条。 |
| `.gitignore`、`.github/actions/setup-rust/action.yml` | 忽略 `native/craft-anything/{target,bin}/`；rust-cache 的 `workspaces` 加入 `native/craft-anything`。 |
| 生成物 / 文档 | `gen-config-catalog`、`gen-module-graph` 重新生成；根 README 的内置能力表与新增小节、`native/README*`、`packages/{mcp,skill}/README*`、`craft-anything-integration.zh.md` 同步。 |

## 2. 关键决定

- **为什么用 `v0.6.1`**：最新 HEAD 把 23 个依赖改为 `storytold/craft-libs` 的 git 引用，首次构建必须联网且需要管理第二个仓库；tag `v0.6.1` 的依赖全在 lockfile 与本地 `vendor/` 中，可从源码离线复现。HEAD 新增的工具集留给后续单独升级。
- **为什么 `on-demand`**：初始 30 个工具约 31 KB schema（约 7.8k tokens），323 个工具不能全量暴露。server 的 `enable_toolset` 发 `notifications/tools/list_changed`，MCP client 会刷新目录，Agent 再按需 `load_tools`。
- **为什么加 toolchain pin**：上游 `oxc_*` 0.153 要求 Rust 1.97，本机默认为 1.96；固定 channel 避免每个贡献者手动 `rustup run`。
- **为什么技能随包而不是调用 `setup`**：上游 `setup` 会写其他 agent 应用的配置与技能目录；内建副本必须在 DSH 自己的 bundle 与 preset 中声明，技能正文的恢复指南也明确忽略 `setup`/`uninstall`。
- **安全默认值**：上游默认 Clean room（Deep 模式关闭），工具拒绝读取目标内部并返回 `clean_room`；会执行目标代码或改动设备的工具逐次要求确认。

## 3. 行为

| 场景 | 结果 |
| --- | --- |
| 新 clone + `pnpm install && pnpm run build` | `craft-anything CLI` gate 编译（本机约 3 分 51 秒，`target/` 约 1 GB，不进 git），`standard` agent 得到按需工具与随包技能。 |
| 运行时 | `craft-anything-cli mcp` 走换行分隔 JSON-RPC 2.0 stdio；工具集启用后通过 `tools/list_changed` 刷新目录。 |
| 缺少构建产物 | `build:craft-anything` 失败并给出可操作错误；运行时缺少可执行文件时 MCP client fail-soft，不阻止 agent 创建。 |
| 打包运行时 | Desktop / Python SDK 可执行文件不带该 CLI，不提供工具（与 rust-anything 相同的已知回退）。 |

## 4. 校验

| 命令 | 结果 |
|---|---|
| `pnpm run build:craft-anything` | 通过；`cargo build --release --locked -p craft-anything-cli` 约 3 分 51 秒；`craft-anything-cli --version` → `0.6.1` |
| `craft-anything-cli capabilities` | 报告 323 个工具、16 个工具集、引擎可用性 |
| 对 `mcp` 子命令 `initialize` + `tools/list` | 初始 30 个工具、约 31 KB schema；`tools.listChanged: true`；含 `list_toolsets`/`enable_toolset` |
| 包单测 | `pnpm vitest run packages/mcp/craft-anything packages/skill/skill-craft-anything scripts/build-craft-anything.spec.ts scripts/build.spec.ts scripts/run-gates.spec.ts packages/bundle/web-app/tests/bundled-skills.spec.ts` | 6 个文件、231 个用例全部通过 |
| 类型检查 | `pnpm run typecheck` | 5/5 通过（约 1089 s） |
| 构建后类型检查 | `pnpm run build:check` | 2/2 通过 |
| Lint | `pnpm run lint` | 0 warning / 0 error（7,800 个文件） |
| 静态检查 | `pnpm run doc-sync` | 42 通过、1 失败：仅 `verify-repository-references` 报出 5 个未改动的既有 log 中的 commit-hash（基线问题） |
| 重复检查 | `pnpm run duplication` | 0 clones（3,437 文件） |
| 组合 | `pnpm dsh --profile web --dump-config` | `standard` preset 含 `mcp-craft-anything`，宿主 patch 含 `skill-craft-anything` |

## 5. 已知限制与后续

- 首次构建约 4 分钟、`target/` 约 1 GB；CI 的 setup-rust action 已把它加入 cargo 缓存。
- 初始工具列表写死在 mount 测试中，上游 bump 会显式失败，需要同步 README 的 token 估算与测试列表。
- 深度调查依赖用户自行安装的 Ghidra/JADX/adb/Chromium/Frida 等引擎；`engines_install` 提供带 SHA-256 校验的下载辅助。
- 运行时观察、截图与环境录制以 Host 用户权限执行目标进程；技能与 README 要求只在副本上操作。
- 上游源码与技能文本为 MIT OR Apache-2.0，vendored 子树与技能 assets 保留两份许可证与 NOTICE。

## 6. 复查记录

- 提交前自审：`git status` 只含本工单预期文件；新包描述、locale 元数据、技能资产许可证与上游一致；`README.i18n.yaml` 三对记录由生成器重写。
- `native/craft-anything/README.zh.md` 未纳入 vendored 子树：翻译配对门禁排除 vendored 树，与 rea/fast-anything 的单语言上游 README 保持一致。
- `pnpm run doc-sync` 唯一失败项是基线上已存在的 `verify-repository-references`（5 个与本次改动无关的 log 含 commit-hash），已在本地核实这些 log 与 HEAD 无差异。
- 本机默认 toolchain 为 1.96；构建与测试均通过 vendored 的 `rust-toolchain.toml` 自动选择 1.97.0。

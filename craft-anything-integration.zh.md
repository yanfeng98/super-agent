# craft-anything 仓库内集成（源码 vendored，clone 即可用）

> 目标：任何人 clone 后 `pnpm install && pnpm run build && pnpm dsh web`，默认 `standard` preset 的会话就带着 craft-anything 的 MCP 工具与 `dsh-craft-anything` 技能——上游源码在仓库里、由仓库自己编译，不需要 `cargo install`、全局 CLI 或向任何 agent 应用注册 MCP server。上游技术：纯 Rust 洁净室调查引擎，覆盖程序、应用包、固件、崩溃转储与网络抓包。
> 结论：上游 `v0.6.1`（commit `48a9c5a60ff83ff620f98661339be6281a7debc6`）源码落在 [`native/craft-anything/`](native/craft-anything/NATIVE.md)，`pnpm run build` 编译 CLI 并暂存 `bin/craft-anything-cli`，插件把 `mcp` 子命令挂到 Web 的 `standard` preset。

## 1. 现在仓库里有什么

| 组成 | 位置 | 作用 |
| --- | --- | --- |
| 上游源码 | `native/craft-anything/`（`v0.6.1`，MIT OR Apache-2.0） | 保留完整 Cargo 工作区、全部 `crates/`、`apps/`、`xtask/`、`assets/`、本地补丁 `vendor/wgpu-hal`、lockfile 与英文 README；剔除只服务于上游文档站/分发/其他 agent 应用的 `docs/`、`examples/`、`packaging/`、`.github/` 与其他语言的 README。基线、剔除清单与同步流程见 [`NATIVE.md`](native/craft-anything/NATIVE.md) |
| 本地修改 | `native/craft-anything/rust-toolchain.toml`（新增） | 固定 Rust 1.97.0，让旧默认 toolchain 的机器也能用普通 `cargo` 构建；上游只声明 `rust-version = "1.97"` |
| 构建 | `scripts/build-craft-anything.ts` + 根脚本 `build:craft-anything` + `scripts/build.ts` 的 `craft-anything CLI` gate | 在 `native/craft-anything` 里跑 `cargo build --release --locked -p craft-anything-cli`，并把可执行文件暂存到同目录的 `bin/`（先复制再 rename，避免覆盖正在运行的二进制） |
| 插件包 | `packages/mcp/craft-anything/`（`@deepseek-ai/dsh-craft-anything`） | `src/binary.ts` 解析暂存 CLI，`src/index.ts` 挂一个 `dsh-mcp-client` stdio 实例；默认 `on-demand`，`CRAFT_ANYTHING_BIN` 或 `command` 可覆盖路径 |
| 技能 | `packages/skill/skill-craft-anything/assets/dsh-craft-anything/` | 上游 `crates/setup/src/skill.md` 与 `skill_recovery.md` 改名 `dsh-craft-anything`，把「自行注册 server」换成内建副本的工具集加载与恢复说明；随包提供方 `@deepseek-ai/dsh-skill-craft-anything`，菜单归入 `内置 · storytold/craft-anything` |
| 挂载点 | `packages/bundle/web-app/presets/standard.patch.yml`、`cordis.patch.yml` | `standard` agent 得到 `mcp-craft-anything` 一行；技能随 Web bundle 挂到所有 profile；`minimal`/`ptc` 与 headless/acp/sdk 不重复挂技能行 |
| 类型/清单 | `tsconfig.base.json`、`tsconfig.host.json`、Web bundle 与 SDK runtime 的 package.json、`.github/actions/setup-rust/action.yml` | 编译路径映射、Host 工程引用、构建可达性与部署闭包；Rust CI action 把新工作区加入 cargo 缓存 |
| 文档 | `README.md`、`native/README*`、`packages/{mcp,skill}/README*`、`craft-anything-integration.zh.md`、`logs/` | 用户说明、包模型体验与维护者同步流程 |

## 2. 关键决定

- **为什么用 `v0.6.1` 而不是最新 HEAD**：HEAD（2026-11-25）把 23 个依赖换成 `storytold/craft-libs` 的 git 引用，首次构建必须联网且要管理第二个仓库；`v0.6.1` 的依赖全在 Cargo.lock 与本地 `vendor/` 里，构建确定且可离线复现。HEAD 多出的工具集可以以后单独升级（见 `NATIVE.md` 的同步流程）。
- **为什么 `on-demand`**：`v0.6.1` 的 server 初始公布 30 个工具（28 个 Core + `list_toolsets`/`enable_toolset`），schema 约 31 KB（约 7.8k tokens）；323 个工具全开更不可承受。server 在 `enable_toolset` 后发 `notifications/tools/list_changed`，DSH 的 MCP client 会自动刷新目录，Agent 再用 `mcp__craft-anything__load_tools` 加载需要的那部分。
- **为什么源码放 `native/craft-anything`**：与 `native/rust-anything`、`native/fast-anything` 同属“导入的上游源码子树 + 仓库构建产物”，`native/` 已被 lint/门禁按既有例外处理；它不是 DSH 插件，而是一个被 spawn 的外部 MCP server。
- **为什么加 `rust-toolchain.toml`**：仓库默认 toolchain 是 1.96，而上游 `oxc_*` 0.153 等依赖要求 1.97；与 rust-anything 一样固定 channel 后，普通 `cargo build` 就会选中已安装的 1.97.0。
- **为什么技能自己随包**：上游 `setup` 命令会写其他 agent 应用的配置文件与技能目录；内建副本不能依赖它，否则用户改的是别的应用而不是 DSH。技能正文和 2 份文档随包提供，恢复指南明确忽略 `setup`/`uninstall`。
- **默认洁净室模式**：工具在 Deep 模式关闭时拒绝读取目标内部，返回 `details.reason: clean_room`。技能引导用户明确开口后再切深度模式，不尝试绕过。

## 3. 行为

| 场景 | 结果 |
| --- | --- |
| 新 clone + `pnpm install && pnpm run build` | `craft-anything CLI` gate 编译 vendored Rust 工作区（本机约 3 分 51 秒，`target/` 约 1 GB，不进 git），`standard` agent 得到按需加载的 craft-anything 工具和随包技能 |
| 运行时 | CLI 以 Host 用户身份运行；`mcp` 子命令走换行分隔的 JSON-RPC 2.0 stdio；工具目录启用后通过 `tools/list_changed` 刷新 |
| 缺少构建产物 | `build:craft-anything` 失败并给出可操作错误；运行时缺少可执行文件时 MCP client fail-soft，不阻止 agent 创建 |
| 打包运行时 | Desktop / Python SDK 可执行文件不会带上 `native/craft-anything/bin`，这两处不提供工具（与 rust-anything 相同的已知回退） |
| 安全边界 | 默认洁净室；执行目标代码或改动设备需要 `consent_required` 逐次确认；深度调查的 Ghidra/JADX/adb/Chromium/Frida 等引擎需要用户自行安装或经 `engines_install` 带校验和下载 |

## 4. 验证记录

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 源码构建与暂存 | `pnpm run build:craft-anything` | 通过；`cargo build --release --locked -p craft-anything-cli` 约 3 分 51 秒，产出 `craft-anything-cli 0.6.1` |
| CLI 能力 | `native/craft-anything/bin/craft-anything-cli capabilities` | 报告 323 个工具、16 个工具集与本机引擎可用性 |
| MCP 工具面 | 对 `mcp` 子命令发 `initialize` + `tools/list` | 初始 30 个工具（约 31 KB schema），`capabilities.tools.listChanged: true`；`list_toolsets`/`enable_toolset` 在列 |
| 包单测 | `pnpm vitest run packages/mcp/craft-anything packages/skill/skill-craft-anything scripts/build-craft-anything.spec.ts scripts/build.spec.ts scripts/run-gates.spec.ts packages/bundle/web-app/tests/bundled-skills.spec.ts` | 6 个文件、231 个用例通过 |
| 组合 | `pnpm dsh --profile web --dump-config` | `standard` preset 含 `mcp-craft-anything`；其他 preset 不重复挂技能行 |
| 门禁 | `pnpm run typecheck`、`pnpm run build:check`、`pnpm run lint`、`pnpm run doc-sync`、`pnpm run duplication` | typecheck 5/5、build:check 2/2、lint 0/0、doc-sync 42/43（唯一失败为既有 log 的 commit-hash 基线问题）、duplication 0 clones |

## 5. 生效方式

- 新 clone：`pnpm install && pnpm run build`，然后 `pnpm dsh web`；默认 preset 的会话直接带工具与技能。
- 已有 profile：bundle 与 preset 文件按仓库实时读取，重启 `dsh web` 后新会话生效；启用过的工具集从新连接重新开始，需要重新启用。
- 改了 `native/craft-anything/` 的源码：重跑 `pnpm run build:craft-anything`（或全量 `pnpm run build`）。
- 想换运行时或入口：在 preset 行配 `command` 或设置 `CRAFT_ANYTHING_BIN`。
- 验证一句话：重启后说「用 craft-anything 分析某个已发布应用是怎么实现 X 的」。

## 6. 已知取舍与可选下一步

- **构建成本**：首次 `cargo build` 约 4 分钟、`target/` 约 1 GB；CI 的 setup-rust action 已把它加入 rust-cache。
- **工具面版本**：初始工具列表写死在 mount 测试里，上游 bump 会显式失败，需要同步 README 的 token 估算与测试列表。
- **引擎依赖**：深度原生分析仍要 Ghidra/Ghidra 等外部引擎；`engines_install` 带 SHA-256 校验，但不能替代用户选择信任的引擎。
- **会执行被分析的目标**：运行时观察、截图与环境录制以 Host 用户权限执行目标进程；技能与包 README 要求只在副本上操作。
- **双许可证**：上游源码与技能文本为 MIT OR Apache-2.0，vendored 子树与技能 assets 保留了两份许可证与 NOTICE。

## 7. 回滚

```sh
git checkout -- package.json pnpm-lock.yaml .gitignore tsconfig.base.json tsconfig.host.json \
  .github/actions/setup-rust/action.yml \
  packages/bundle/web-app/{package.json,cordis.patch.yml,presets/standard.patch.yml,tests/bundled-skills.spec.ts} \
  python/sdk-runtime/package.json scripts/build.ts scripts/build.spec.ts scripts/run-gates.ts scripts/run-gates.spec.ts \
  README.md native/README.md native/README.zh.md packages/mcp/README.md packages/mcp/README.zh.md \
  packages/skill/README.md packages/skill/README.zh.md \
  docs/config-catalog.md docs/config-catalog.zh.md docs/module-graph.md docs/module-graph.zh.md
rm -rf native/craft-anything packages/mcp/craft-anything packages/skill/skill-craft-anything \
  scripts/build-craft-anything.ts scripts/build-craft-anything.spec.ts \
  craft-anything-integration.zh.md logs/00135-内建craft-anything.md
pnpm install
```

`native/craft-anything/target/`、`bin/` 都不进 git，删除目录即可清除构建产物。

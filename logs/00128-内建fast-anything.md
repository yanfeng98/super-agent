# 00128 — 内建 fast-anything（源码 vendored）

- **日期**：2026-11-24
- **背景**：用户要求把 <https://github.com/EL4CTEO/fast-anything> 像 rea 与 rust-anything 一样集成进本仓库，作为开箱可用的内建能力，而不是让用户自己 `npm i -g @el4cteo/fast-anything` 或注册 MCP server。做法沿用 [00099 源码 vendored 集成 rust-anything](00099-源码vendored集成rust-anything.md) 与 [00127 内建 rea](00127-内建rea.md)：上游源码进仓库、由仓库自己的构建用 cargo 编译，插件与技能随 `standard` preset 挂载。
- **基线**：上游 release `v0.8.6`，commit `ba571858d222836843fcd0bccbd94a4290c36ac2`（2026-11-24），MIT。
- **范围**：`native/fast-anything/`（新 vendored 子树 + `NATIVE.md`）、`scripts/build-fast-anything.ts`（+ spec）、根 `package.json`、`scripts/build.ts`、`scripts/run-gates.ts`（+ 相关 spec）、`packages/mcp/fast-anything/`、`packages/skill/skill-fast-anything/`、`packages/bundle/web-app/{package.json,presets/standard.patch.yml}`、`python/sdk-runtime/package.json`、`tsconfig.base.json`/`tsconfig.host.json`、`.gitignore`、`.github/actions/setup-rust/action.yml`、生成的 config catalog / 模块图 / 技能包清单、根 `README.md`、`native/README*`、`packages/{mcp,skill}/README*`、`fast-anything-integration.zh.md`、`logs/`。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `native/fast-anything/`（新） | 上游 `v0.8.6` 源码，保留 `Cargo.toml`、`Cargo.lock`、`rustfmt.toml`、`src/`、`skills/fast-anything/SKILL.md`、`README.md`、`CHANGELOG.md`、`LICENSE`（约 1 MB、12,100 行 Rust）；剔除 `npm/`、`commands/`、`.claude-plugin/`、`scripts/`、`examples/`、`docs/`、`.github/`、`.gitignore`、`.mcp.json`。 |
| `native/fast-anything/NATIVE.md`（新） | 记录基线、保留/剔除清单、构建、挂载、检查与上游同步流程；说明不加 `rust-toolchain.toml` 的理由与 `src/php/bridge.php` 无末尾换行的事实。 |
| `scripts/build-fast-anything.ts` + `.spec.ts`（新） | 以 crate 目录为 cwd 跑 `cargo build --release --locked`，把可执行文件暂存到 `native/fast-anything/bin/fast-anything[.exe]`；用「先复制到 `.tmp` 再 rename」避免覆盖正在运行的二进制（ETXTBSY）；失败信息指向 `NATIVE.md`。测试覆盖可执行文件名、cargo 输出与暂存路径。 |
| `scripts/build.ts`、根 `package.json` | artifacts 作用域新增 `fast-anything CLI` gate；新增 `build:fast-anything`；`test`/`test:coverage` 先跑它。 |
| `scripts/run-gates.ts`（+ spec） | coverage、coverage-shard、ci-unit 三个聚合各加 `fast-anything` gate，并让测试/覆盖率 gate 依赖它。 |
| `packages/mcp/fast-anything/`（新） | `@deepseek-ai/dsh-fast-anything`：`src/binary.ts` 解析暂存产物（`command` → `FAST_ANYTHING_BIN` → 仓库构建），`src/index.ts` 挂一个 `dsh-mcp-client` stdio 实例；默认 `eager`、15 分钟单次调用预算、20 秒启动；8 种语言的 locale、中英 README 与 mount 用例。 |
| `packages/skill/skill-fast-anything/`（新） | `@deepseek-ai/dsh-skill-fast-anything`：上游 `skills/fast-anything/SKILL.md`（MIT）改名 `dsh-fast-anything`，补 MCP 工具名、`path` 参数、暂存路径与「不要全局安装」、Rust 1.88、`.fast/target` 缓存与 `enable`/`disable` 改写源码的警告；随包提供方与 `/` 菜单分组 `内置 · EL4CTEO/fast-anything`。含 `assets/LICENSE` 与完整 provider 用例。 |
| `packages/bundle/web-app/presets/standard.patch.yml` | `standard` preset 新增 `skill-fast-anything` 与 `mcp-fast-anything` 两行（`eager`）。 |
| `packages/bundle/web-app/package.json`、`python/sdk-runtime/package.json` | 补两个 workspace 依赖：Web bundle 需要它们被构建，`verify-runtime-closure` 要求 shipped preset 的插件出现在部署清单。 |
| `tsconfig.base.json`、`tsconfig.host.json` | 生成器补路径映射；Host 工程引用手工补两条（与 rust-anything/rea 的既有做法一致）。 |
| `.gitignore` | 忽略 `native/fast-anything/{target,bin}/`。 |
| `.github/actions/setup-rust/action.yml` | rust-cache 的 `workspaces` 加上 `native/fast-anything`；仍用同一个 1.97.0 toolchain（满足上游 1.88 下限）。 |
| 生成物 / 文档 | `gen-config-catalog`、`gen-module-graph` 重新生成；根 README 的内置能力表与新增小节、`native/README*`、`docs/development*`、MCP/技能组 README、子系统页、`docs/user/guide/{index,fast-anything}*`、`website/docs.ts`、`fast-anything-integration.zh.md` 同步。 |

## 2. 关键决定

- **为什么源码放 `native/fast-anything`**：与 `native/rust-anything` 同属「导入的上游 Rust crate + 仓库构建产物」，`native/` 已被 lint/门禁按既有例外处理；它不是 DSH 插件，而是一个被 spawn 的外部 MCP server。
- **为什么用 cargo 直接编译而不是 npm 包**：上游 npm 包只是平台二进制的下载器；本仓库要先 clone 即用、离线可重建，所以 `cargo build --release --locked` 从 vendored 源码编译，`Cargo.lock` 保证依赖图确定。
- **为什么不加 `rust-toolchain.toml`**：上游只要求 `rust-version = "1.88"`，比仓库 CI 的 1.97.0 旧；固定 channel 会让所有贡献者为一个不需要的新版本付费。缺少或过旧的 `cargo` 由构建脚本给出可操作错误。
- **为什么 `eager`**：8 个工具只有 3,020 字节 schema（约 800 tokens），不值得为它引入目录加载步骤；`on-demand` 仍作为插件配置保留。
- **为什么挂在 `standard` preset**：MCP server 的 instructions 会进入系统提示；挂 base 会让所有 profile 的模型可见提示变化并重录 `snapshots/session`，挂 Web `standard` 只影响 Web 会话。
- **与 rust-anything 的分工**：fast-anything 只搬最外层热点函数，适合「只让慢的那 5% 变快」；整应用改写成 Rust 仍走 rust-anything，两个技能在描述里互相区分。

## 3. 行为

| 场景 | 结果 |
|---|---|
| 新 clone + `pnpm install && pnpm run build` | `fast-anything CLI` gate 用 cargo 编译（冷构建约 30 s、`target/` 约 55 MB，均不进 git），`standard` agent 得到 8 个 `mcp__fast-anything__*` 工具与 `dsh-fast-anything` 技能。 |
| 工具调用 | `fast_scan` 剖析应用并排序；`fast_port` 录制最多 500 次真实调用并生成单函数 crate 与 `TASK.md`；`fast_verify` release 编译并回放；`fast_enable` 写入带标记的调用块；`fast_bench`/`fast_report` 出数字与报告；`fast_disable` 原样移除。 |
| 缺少构建产物 | `build:fast-anything` 失败并给出可操作错误；运行时可执行文件缺失时 MCP client fail-soft，不阻止 agent 创建。 |
| 目标项目副作用 | 扫描、录制、移植会在目标项目里写 `.fast/`（录制、crate、`TASK.md`、报告）并把 crate 编译进 `.fast/target`；`enable` 改写调用点。技能与 README 要求只在副本上操作并审查 diff。 |
| 打包运行时 | Desktop / Python SDK 可执行文件不会带上 `native/fast-anything/bin`，这两处不提供工具（与 rust-anything 相同的已知回退）。 |

## 4. 校验

| 命令 | 结果 |
|---|---|
| `pnpm run build:fast-anything` | 通过；`cargo build --release --locked` 冷构建 30.3 s；暂存 2,676 KiB；`--version` → `0.8.6` |
| `cd native/fast-anything && PATH=/usr/bin:$PATH cargo test --release` | 84/84 通过；PATH 里是 conda Python 3.12.2 时 `pool_context_manager_workers_are_profiled_and_recorded` 失败，换成系统 Python 3.10.12 即通过（环境差异，非 vendored 源码问题） |
| 直接对 `fast-anything mcp` 做 `initialize` + `tools/list` | 8 个工具，schema 3,020 B，instructions 392 B |
| `pnpm vitest run packages/mcp/fast-anything packages/skill/skill-fast-anything scripts/build-fast-anything.spec.ts scripts/build.spec.ts scripts/run-gates.spec.ts` | 5 个文件、228 个用例通过（mount 5、skill 9） |
| `pnpm run typecheck` | 5/5 通过（144.42 s） |
| `pnpm run build` | 15/15 通过（34.51 s，Node 24.22.3）；本机默认 Node 24.21.0 低于仓库与 rea 的引擎下限，rea gate 会先失败 |
| `pnpm run build:check` | 2/2 通过（10.03 s） |
| `pnpm run check:ci:static` | 修复 `gen-plugin-packages` / `gen-doc-graphs` 生成物后只剩 1 项既有失败：`verify-plugin-entry-exports`（`packages/api/session-controller` 的两个导出，非本工单改动） |
| `pnpm run test:docs` | 21/21 通过 |
| `pnpm run lint` | 0 warning / 0 error（7,784 个文件，78.9 s） |
| `pnpm run hygiene` | 26/27 通过（183.96 s），唯一失败仍是既有的 `verify-plugin-entry-exports` |
| `pnpm vitest run --coverage`（只 include 两个包的 `src`） | 语句/分支/函数/行均 100% |
| `pnpm dsh --profile web --dump-config` | `standard` preset 含 `skill-fast-anything` 与 `mcp-fast-anything`；`minimal` 不含 |

## 5. 已知限制与后续

- Desktop / Python SDK 打包运行时带不上这个二进制，需要像 `native/system` 那样按平台暂存；当前与 rust-anything 一样 fail-soft。
- 移植需要目标语言运行时（按应用而定）与 Rust 1.88+；`fast-anything doctor` 逐个报告。
- 上游的 Rust 测试是 `src/` 内的内联单元测试（84 个 `#[test]`），没有独立 `tests/` 目录；`cargo test --release` 可直接在 vendored crate 里跑，未接入 `pnpm test`。
- 仓库既有 duplication 门禁在本次集成前已在基线 HEAD 上失败（`packages/skill` 内 5 处克隆）；新技能提供方沿用同一实现会增加同类克隆，未在本工单内重构共享实现。
- 工具会执行目标项目代码并改写调用点，属于工具本身的能力边界；技能与 README 已写明只在副本上运行并审查 diff。

## 6. 复查记录

- 提交前自审：`git diff --cached --check` 通过；pre-commit 的 translation pairing（staged）、staged lint、whitespace、pi-ai metadata、vendor manifest 全部通过。
- 生成的 config catalog、模块图、技能包清单已用对应生成器重跑；`docs/event-producer-consumer*` 的行号刷新来自同一生成器，属既有生成的陈旧内容，不是本次源码改动。
- 本机默认 Node 24.21.0 低于仓库 `engines`（^24.22.3）与 rea 的下限，全量构建会在 rea gate 先失败；验证时用 Node 24.22.3 运行，fast-anything 自身 gate 在任何受支持版本下通过。
- `native/rea/scripts/lib/` 被根 `.gitignore` 的 `lib/` 规则排除，fresh checkout（含本 worktree）缺这 226 个文件，`build:rea` 的 `prepare` 脚本会失败。验证时从实现 checkout 复制到本地（ignored，未提交），这是既有 rea 接入的缺陷，建议单独修复（加 `!native/rea/scripts/lib/**` 否定并提交这些文件）。
- `verify-plugin-entry-exports` 在基线 HEAD 上已有 `packages/api/session-controller` 的两个导出失败；`jscpd` 基线也已有 5 处克隆。本次未新增门禁类别，但新技能提供方沿用既有实现，会增加同类克隆。

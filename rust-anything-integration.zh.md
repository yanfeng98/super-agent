# rust-anything 仓库内集成（源码 vendored，clone 即可用）

> 使用说明：[使用 rust-anything 迁移到 Rust](docs/user/guide/rust-anything.zh.md)。变更记录：[`logs/00099`](logs/00099-源码vendored集成rust-anything.md)（本轮：改为源码 vendored + 现场编译）；[`logs/00097`](logs/00097-原生集成rust-anything.md)（最初以 npm 依赖接入，已被本轮取代）。根 [README](README.md) 的「与上游的差异」与 [MCP 子系统页](docs/subsystems/mcp.zh.md) 已同步。

> 目标：任何人 clone 后 `pnpm install && pnpm run build && pnpm dsh web` 就带着这个能力——上游源码在仓库里、由仓库自己编译，不依赖全局安装，也不下载预编译二进制。
> 结论：已完成。上游 `v2.1.0` 源码落在 `native/rust-anything/`，`pnpm run build` 用 cargo 编译并暂存可执行文件，插件按固定相对路径解析后挂到 Web 的 `standard` preset。

---

## 1. 现在仓库里有什么

| 组成 | 位置 | 作用 |
| --- | --- | --- |
| 上游源码 | `native/rust-anything/`（`v2.1.0`，基线 `71c78e1`，MIT） | 7.1 MB 源码：`src/`、`tests/`、`docs/`、`directory/`、`skills/` 原样保留；剔除只服务于外部分发的 `npm/`、`scripts/`、`demo/` 与 CI/编辑器配置。基线、改动与同步流程见 [`NATIVE.md`](native/rust-anything/NATIVE.md) |
| 构建 | `scripts/build-rust-anything.ts` + 根脚本 `build:rust-anything` + `scripts/build.ts` 的 `rust-anything CLI` gate | 以 crate 目录为 cwd 跑 `cargo build --release --locked`，把 `target/release/rust-anything` 暂存到 `native/rust-anything/bin/`（约 26 MB） |
| 工具链 | `native/rust-anything/rust-toolchain.toml` | 固定 `1.97.0`（oxc 依赖要求同一版本）；rustup 缺失时自动安装。这是与上游唯一的文件差异 |
| 插件包 | `packages/mcp/rust-anything/`（`@deepseek-ai/dsh-rust-anything`） | `src/binary.ts` 解析暂存产物（`command` → `RUST_ANYTHING_BIN` → 仓库内构建产物），`src/index.ts` 挂一个 `dsh-mcp-client` 实例 |
| 挂载点 | `packages/bundle/web-app/presets/standard.patch.yml` | Web 默认 preset `standard` 的 Agent 得到 16 个 `mcp__rust-anything__*` 工具（eager） |
| 技能 | `.agents/skills/rust-anything/{SKILL.md,whole-app.md}` | 仓库根技能目录，clone 后自动发现；描述命中后加载完整迁移流程（已补 Kotlin 与本仓库须知） |
| 类型/清单 | `tsconfig.base.json` 路径映射、`tsconfig.host.json` 工程引用、`packages/mcp/README.md` 列表 | 让仓库的编译、产品可达性、文档门禁都认这个包 |
| 本地化元数据 | `packages/mcp/rust-anything/locale/{en,zh,ja,es,fr,de,ru,pt}.json` + `exports`/`files` | `verify-locale-metadata` 要求全部 8 种发布语言的 `meta.title`/`meta.description` |
| 依赖形态 | `@deepseek-ai/dsh-mcp-client` 放在 `peerDependencies`（同时 devDependency） | 插件之间按仓库共享实例约定走 peer；模块图里出现 `rust-anything → mcp-client` 边 |
| 部署闭包 | `python/sdk-runtime/package.json` | `verify-runtime-closure` 要求 shipped preset 的插件出现在部署清单里 |
| CI | `.github/actions/setup-rust/action.yml` + 12 个 workflow（19 处） | 会构建/测试的 job 装 Rust 1.97（`dtolnay/rust-toolchain@v1` + `toolchain: 1.97.0`）并缓存 `native/rust-anything` 的 cargo 产物；`coverageGates`/`ciUnitGates` 里也各有 `build:rust-anything` gate |

**为什么挂在 preset 而不是 base bundle**：MCP server 会在系统提示里贡献 `## MCP resource servers` 段和 `### MCP server: rust-anything` instructions。挂在 base 会让每个 profile（含 headless/acp/sdk）的模型可见提示变化，`snapshots/session` 的 198 个期望文件全部要重录；挂在 Web 的 `standard` preset 只影响 Web 会话，keyless 快照套件（acp 16 + sdk 43 + session 216）不受影响。

## 2. 实测证据

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 源码编译 | `pnpm run build:rust-anything` | 通过；冷编译 32.6 s，暂存 26068 KiB，`rust-anything --version` → `2.1.0` |
| 包单测（连真实可执行文件） | `pnpm vitest run packages/mcp/rust-anything` | 5/5：eager 注册出 `analyze`/`gate`/`app_lift`，on-demand 只注册 `load_tools`，三条解析分支，可执行文件缺失时 agent 照常创建 |
| 覆盖率 | 只统计本包 `src` | lines/branches/functions/statements 均 100% |
| 聚合检查 | `pnpm run build:check` | 2/2 通过 |
| 静态门禁 | `pnpm run check:ci:static` | 70/70 通过（含 `verify-plugin-entry-exports`、`verify-runtime-closure`、三个生成物检查、`gen-cli-help --check`） |
| 文档门禁 | `pnpm run test:docs` | 21/21 通过 |
| 仓库卫生 | `pnpm run hygiene` | 26/26 通过（publint、包依赖、NodeNext consumer 等） |
| 其它门禁 | `verify-cordis-config`、`verify-module-graph`、`verify-locale-metadata`、`verify-package-dependencies`、`constraints`、`verify-product-use`、`verify-default-product-isolation` | 全部通过（377 个配置、3 个生成产物、8 语言、103 个包、299 默认 / 53 可选包、363 个 Web 组合插件） |
| 全量构建 | `pnpm run build` | 13/13 通过（77.98 s），含新增的 `rust-anything CLI` gate |
| 组合 | `pnpm dsh --profile web --dump-config` | `standard` preset 含 1 处 `mcp-rust-anything`，`minimal` 不含 |
| workflow 结构 | YAML 解析 + step 断言 | 12 个 workflow + 组合动作全部可解析；19 处 `setup-rust` 均为独立 step，checkout 的 `with:` 与改动前逐项一致 |
| 门禁图 spec | `pnpm vitest run scripts/run-gates.spec.ts` | 168/168；workflow 相关 spec 共 279 通过 |

本轮修掉的、由 00097 遗留的问题：入口导出超出 Loader 协议（`verify-plugin-entry-exports`）、部署清单缺该插件（`verify-runtime-closure`）、`tsconfig.base.json` 路径条目位置与生成器不符、`docs/config-catalog*.md` 与技能包清单未重新生成。

## 3. 生效方式

- 新 clone：`pnpm install && pnpm run build`（编译 Rust 源码），然后 `pnpm dsh web`；默认 preset 的会话直接带工具。
- 已有 profile（比如 `~/.dsh/profiles/web`）：bundle 与 preset 文件按仓库实时读取，**重启 `dsh web` 后**新会话生效；会话持久化，可 resume。
- 改了 `native/rust-anything/` 的源码：重跑 `pnpm run build:rust-anything`（或全量 `pnpm run build`），暂存文件随之更新。
- 想用别的二进制：设 `RUST_ANYTHING_BIN`，或在 preset 行的 `config.command` 指定。
- 验证一句话：重启后问「列出你可用工具里以 `mcp__rust-anything__` 开头的名字」，应看到 16 个；或直接说「用 rust-anything 分析这个仓库」。

## 4. 使用方式（模型侧）

技能会引导：`analyze` → `bridge` → `record` → 写 `port.rs` → `build` → `diff` → `bench` → `gate`，整应用走 `app_*`。技能里额外写了三条本仓库须知：

1. `build/diff/gate/bench` 用**绝对路径**（2.1.0 的相对路径 bug，见分析报告 §6.2）；
2. 生成 crate 需要 Rust ≥ 1.97（`rust-anything doctor` 会报）；
3. 工具会执行被录制的代码并往仓库写 crate，只在副本上跑。

## 5. 已知取舍与可选下一步

- **构建前置条件**：`pnpm run build` 现在需要 Rust 工具链（`rust-toolchain.toml` 固定 1.97.0，rustup 会安装）；冷编译约 30–60 s、`target/` 约 430 MB（已 gitignore），CI 用 `Swatinem/rust-cache` 缓存。
- **每个 agent 的额外成本**：`standard` agent 多 19 个工具（16 个 server 工具 + 3 个资源工具，约 7k tokens/请求）和多一个 server 进程；`minimal`/`ptc` 不受影响。
- **打包运行时（Desktop、Python SDK 可执行文件）**：两者都会挂载这个 preset，但二进制在所有包目录之外，打包不会带上；那里 MCP server 找不到可执行文件时不提供工具（`fail-soft`）。上一版（npm 依赖）能带上，所以这是源码 vendored 的已知回退；后续做法是像 `native/system` 那样按平台暂存二进制。
- **只挂 Web**（现状）：headless/acp/sdk 不挂，keyless 快照不红。若要让 headless 也默认可用，需要给 `snapshots/session` 的 198 个 `system-prompt.expected.md` 补上 MCP 段（应走带 key 的重录流程确认）。
- **eager vs on-demand**：eager 让工具常驻、无需 load 步骤，代价是每次请求约 7k tokens；改 `toolLoading: on-demand` 会往系统提示加一段目录，同时改变 Web e2e 快照期望。
- **上游相对路径 bug**：2.1.0 的 `run` 把子进程切到临时目录却传下相对 `--manifest-path`，`build/diff/gate/bench` 必须传绝对路径（技能已写明）。
- **上游测试**：`native/rust-anything/tests/` 原样保留，`cd native/rust-anything && cargo test --release` 可跑，未接入 `pnpm test`。

## 6. 回滚

```sh
git checkout -- packages/bundle/web-app/package.json packages/bundle/web-app/presets/standard.patch.yml \
  packages/mcp/README.md packages/mcp/README.zh.md packages/mcp/README.i18n.yaml \
  pnpm-lock.yaml pnpm-workspace.yaml tsconfig.base.json tsconfig.host.json \
  python/sdk-runtime/package.json .gitignore scripts/build.ts package.json
git checkout -- .github/workflows .github/actions 2>/dev/null || true
rm -rf native/rust-anything packages/mcp/rust-anything scripts/build-rust-anything.ts .github/actions/setup-rust
git checkout -- docs/config-catalog.md docs/config-catalog.zh.md docs/module-graph.md docs/module-graph.zh.md \
  packages/preset/agent-preset/skills/dsh-doc/references/packages.md
pnpm install
```

`native/rust-anything/target/` 与 `native/rust-anything/bin/` 都不进 git，删除目录即可清除构建产物。

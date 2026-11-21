# rust-anything 仓库内集成（clone 即可用）

> 仓库变更记录见 [`logs/00097`](logs/00097-原生集成rust-anything.md)；根 [README](../README.md) 的「与上游的差异」与 [MCP 子系统页](../docs/subsystems/mcp.zh.md) 已同步。

> 目标调整：不再依赖本机的 `npm i -g` 或手改 profile，而是把集成放进 **super-agent 仓库本身**——任何人 clone 后 `pnpm install && pnpm dsh web` 就带着这个能力。
> 结论：已完成。新增 1 个 workspace 包、1 行 preset 挂载、1 份仓库技能；全部走仓库已有门禁与测试。

---

## 1. 现在仓库里有什么

| 组成 | 位置 | 作用 |
| --- | --- | --- |
| 插件包 | `packages/mcp/rust-anything/`（`@deepseek-ai/dsh-rust-anything`） | 解析 CLI（来自本包 `rust-anything` npm 依赖的 `binaries/<platform>-<arch>/`）并挂载一个 `dsh-mcp-client` 实例 |
| npm 依赖 | `packages/mcp/rust-anything/package.json` → `"rust-anything": "2.1.0"`（精确锁，与 `pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude` 条目一致） | `pnpm install` 即得 CLI，**不需要** `cargo install` / `npm i -g`；五平台二进制都在这一个包里 |
| 挂载点 | `packages/bundle/web-app/presets/standard.patch.yml` | Web 默认 preset `standard` 的 Agent 会得到 16 个 `mcp__rust-anything__*` 工具（eager） |
| 技能 | `.agents/skills/rust-anything/{SKILL.md,whole-app.md}` | 仓库根技能目录，clone 后自动进入技能目录；描述命中后加载完整迁移流程（已补 Kotlin 与本地注意事项） |
| 类型/清单 | `tsconfig.base.json` 路径映射、`tsconfig.host.json` 工程引用、`packages/mcp/README.md` 列表 | 让仓库的编译、产品可达性、文档门禁都认这个包 |
| 本地化元数据 | `packages/mcp/rust-anything/locale/{en,zh,ja,es,fr,de,ru,pt}.json` + package.json 的 `exports`/`files` | `verify-locale-metadata` 要求每个包有全部 8 种发布语言的 `meta.title`/`meta.description` |
| 依赖形态 | `@deepseek-ai/dsh-mcp-client` 放在 `peerDependencies`（同时 devDependency 供编译/测试） | 插件包之间按仓库共享实例约定走 peer；发布安装不会带第二份副本，模块图里出现 `rust-anything → mcp-client` 边 |

**为什么挂在 preset 而不是 base bundle**：MCP server 会在系统提示里贡献 `## MCP resource servers` 段和 `### MCP server: rust-anything` instructions（实测确认，README 已按事实改写）。挂在 base 会让每个 profile（含 headless/acp/sdk）的模型可见提示变化，`snapshots/session` 的 198 个期望文件全部要重录；挂在 Web 的 `standard` preset 只影响 Web 会话，keyless 快照套件（acp 16 + sdk 43 + session 216）不受影响。取舍见 §5。

## 2. 实测证据

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 包单测（连真实二进制） | `pnpm vitest run packages/mcp/rust-anything` | 4/4：eager 注册出 `analyze`/`gate`/`app_lift`，on-demand 只注册 `load_tools` |
| 编译 | `pnpm exec tsc -b tsconfig.host.json` | 通过 |
| 产物构建 | `pnpm run build:artifacts` | 10/10 gate 通过（host/client compiler、bundle、desktop、web） |
| lint | `pnpm exec tsx scripts/run-oxlint.ts packages/mcp/rust-anything` | 0 warning / 0 error |
| 组合 | `pnpm dsh --profile web --dump-config` | `preset-standard.plugins` 内含 `mcp-rust-anything`；headless 计数为 0 |
| 运行期解析 | `createRuntimeResolution` + `installRuntimeInterception`（app-boot 的真实机制） | installation scope 里有 `@deepseek-ai/dsh-rust-anything`（declarer = web-app），解析到 `packages/mcp/rust-anything/lib/index.js`，可 `import` 出 `apply/Config/...` |
| 全量构建 | `pnpm run build` | 12/12 gate（artifacts + check）通过 |
| 生成文档 | `verify-module-graph` / `gen-module-graph` | 新增包会改动模块图；已重新生成 `docs/module-graph*.md`（含依赖边）并复核通过 |
| 本地化门禁 | `verify-locale-metadata` | 补齐 8 种语言后通过 |
| 依赖门禁 | `verify-package-dependencies`、`constraints` | 通过（103 个包符合发布依赖策略） |
| 覆盖率 | 只统计本包 `src` | lines/branches/functions/statements 均 100% |
| 安装可复现 | `pnpm install --frozen-lockfile` | 通过（`rust-anything` 精确锁 2.1.0） |
| keyless 快照 | `pnpm vitest run --config vitest.snapshot.config.ts` | 270 通过 / 6 失败 / 2 跳过；6 个失败全是 `ptc-*` 与 `worktree` 的环境问题（见下） |
| 仓库门禁 | `verify-product-use`、`verify-default-product-isolation`、`verify-cordis-config`、`verify-cordis-catalog`、`verify-export-jsdoc`、三个 README 校验、`verify-translation-pairing`、`constraints`、`publint` | 全部通过 |
| 已知无关失败 | `verify-third-party-notices` | 因 fork 自带的 `@tencent-connect/qqbot-connector` 缺少 repository 字段而失败，与本改动无关 |

第二轮检查另发现并修复了 3 个问题：**缺少包级 locale 元数据**（`verify-locale-metadata` 直接失败）、**`dsh-mcp-client` 写成普通依赖**（违反插件共享实例约定，发布安装会带副本、模块图缺边）、以及 **`docs/module-graph*.md` 过期**（生成文档门禁失败）。另外 `pnpm run test` 在与我并行重负载时出现 2 个失败（`packages/skill/skill/tests/skill.spec.ts` 的重复注册用例、`session-snapshot/tests/harness.spec.ts` 的栈溢出），单独重跑均通过，属于机器负载导致的偶发失败，与本改动无关。

6 个快照失败的独立性依据：headless 组合里根本没有这个插件（`--dump-config` 计数为 0），失败是 `snapshot Git failed: fatal: Needed a single revision`（worktree）与 PTC 运行时相关的本机环境问题，改动无法触达这些路径。

**端到端证据（本轮新增）**：`apps/cli/tests/web-agent-presets.e2e.ts` 用 app-boot 真正 boot 出 shipped Web 组合（base + web-app patch，含 preset），并用假 LlmAdapter 创建 agent，不需要 key。运行后 `standard` agent 的工具列表里出现 **16 个 `mcp__rust-anything__*` + 3 个共享资源工具**（`list_mcp_resources`/`list_mcp_resource_templates`/`read_mcp_resource`）；`minimal` preset 的工具列表**没有**这些，说明挂载没有泄漏到其它 preset。

该文件本身在本 fork 上**已经是红的**：与我的改动无关的 3 类漂移（`dsh-superagent-router`/`dsh-superagent-worker` 两个额外 preset、IM 工具 `dsh_im_group_file` 等 3 个、skill 目录差异）。做了 A/B 对照：移除我这行后重跑，失败集合**完全相同**（11 个用例、33 行 FAIL，逐条 `diff` 为空）。因此本改动没有新增失败用例；但若将来要修这个期望表，`standard` 的期望需补上这 19 个工具名。

## 3. 生效方式

- 新 clone：`pnpm install && pnpm dsh web`，默认 preset 的会话直接带工具。
- 已有 profile（比如你现在的 `~/.dsh/profiles/web`）：bundle 与 preset 文件按仓库实时读取，**重启 `dsh web` 后**新会话生效；会话持久化，可 resume。
- 验证一句话：重启后问「列出你可用工具里以 `mcp__rust-anything__` 开头的名字」，应看到 16 个；或直接说「用 rust-anything 分析这个仓库」。

## 4. 使用方式（模型侧）

技能会引导：`analyze` → `bridge` → `record` → 写 `port.rs` → `build` → `diff` → `bench` → `gate`，整应用走 `app_*`。技能里额外写了三条本仓库须知：

1. `build/diff/gate/bench` 用**绝对路径**（2.1.0 的相对路径 bug，见分析报告 §6.2）；
2. 生成 crate 需要 Rust ≥ 1.97（`rust-anything doctor` 会报）；
3. 工具会执行被录制的代码并往仓库写 crate，只在副本上跑。

## 5. 已知取舍与可选下一步

- **每个 agent 的额外成本**：`standard` agent 多 19 个工具（16 个 server 工具 + 3 个资源工具，约 7k tokens/请求）和多一个 server 进程；`minimal`/`ptc` 不受影响（实测）。
- **只挂 Web**（现状）：headless/acp/sdk 不挂，keyless 快照不红。若要让 headless 也默认可用，需要给 `snapshots/session` 的 198 个 `system-prompt.expected.md` 补上 MCP 段（可脚本化，但那属于"改期望"，应当用带 key 的重录流程确认）。
- **eager vs on-demand**：eager 让工具常驻、无需 load 步骤，代价是每次请求约 6k tokens；改 `toolLoading: on-demand` 会往系统提示加一段目录，同时也会改变 Web e2e 快照期望。
- **工具链**：本机默认为 1.96，生成 crate 会在 build 阶段失败。可选 `rustup update stable`、`rustup default 1.97.0`，或启动宿主时带 `RUSTUP_TOOLCHAIN=1.97.0`（MCP 子进程会继承）。
- **Web e2e 快照**：`snapshots/web/*/system-prompt.expected.md` 由浏览器 e2e 使用（需要 key），默认 preset 现在多了一个 MCP 段；带 key 重录时会被更新。keyless 套件不受影响。

## 6. 回滚

```sh
git checkout -- packages/bundle/web-app/package.json packages/bundle/web-app/presets/standard.patch.yml \
  packages/mcp/README.md packages/mcp/README.zh.md packages/mcp/README.i18n.yaml \
  pnpm-lock.yaml pnpm-workspace.yaml tsconfig.base.json tsconfig.host.json
rm -rf packages/mcp/rust-anything .agents/skills/rust-anything
pnpm install
```

本机此前为验证临时做过的全局安装与配置已清理：`~/.dsh/skills/rust-anything` 已删除，`~/.dsh/profiles/*/cordis.patch.yml` 已还原为备份，`~/.dsh/cordis.patch.yml` 已删除。`npm -g` 的 `rust-anything` 与并行安装的 Rust `1.97.0` 仍在，可自行 `npm uninstall -g rust-anything`、`rustup toolchain uninstall 1.97.0`；仓库集成不再依赖它们。

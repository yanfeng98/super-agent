# fast-anything 仓库内集成（源码 vendored，clone 即可用）

> 目标：任何人 clone 后 `pnpm install && pnpm run build && pnpm dsh web`，默认 `standard` preset 的会话就带着 fast-anything 的 MCP 工具与 `dsh-fast-anything` 技能——上游源码在仓库里、由仓库自己用 cargo 编译，不依赖全局安装，也不下载预编译二进制。
> 结论：已完成。上游 `v0.8.6`（commit `ba571858`）源码落在 [`native/fast-anything/`](native/fast-anything/NATIVE.md)，`pnpm run build` 编译并暂存可执行文件，插件把 `fast-anything mcp` 挂到 Web 的 `standard` preset。

## 1. 现在仓库里有什么

| 组成 | 位置 | 作用 |
| --- | --- | --- |
| 上游源码 | `native/fast-anything/`（`v0.8.6`，commit `ba571858d222836843fcd0bccbd94a4290c36ac2`，MIT） | 保留 `Cargo.toml`、`Cargo.lock`、`rustfmt.toml`、`src/`（约 12,100 行 Rust）、`skills/fast-anything/SKILL.md`、`README.md`、`CHANGELOG.md`、`LICENSE`，约 1 MB；剔除只服务于外部分发的 `npm/`、`commands/`、`.claude-plugin/`、`scripts/`、`examples/`、`docs/`、`.github/`、`.gitignore`、`.mcp.json`。基线、剔除清单与同步流程见 [`NATIVE.md`](native/fast-anything/NATIVE.md) |
| 构建 | `scripts/build-fast-anything.ts` + 根脚本 `build:fast-anything` + `scripts/build.ts` 的 `fast-anything CLI` gate | 以 crate 目录为 cwd 跑 `cargo build --release --locked`，把 `target/release/fast-anything` 暂存到 `native/fast-anything/bin/`（2,676 KiB）；本机冷构建 30.3 s，`target/` 约 55 MB（不纳入版本控制） |
| 工具链 | 上游 `Cargo.toml` 的 `rust-version = "1.88"` | 故意不加 `rust-toolchain.toml`：任何 Rust 1.88+ 都能构建，仓库为 rust-anything 安装的 1.97.0 也满足；缺少或过旧的 `cargo` 在构建时失败，错误信息指向 `NATIVE.md` |
| 插件包 | `packages/mcp/fast-anything/`（`@deepseek-ai/dsh-fast-anything`） | `src/binary.ts` 解析暂存产物（`command` → `FAST_ANYTHING_BIN` → 仓库内构建产物），`src/index.ts` 挂一个 `dsh-mcp-client` stdio 实例；默认 `eager`、单次调用 15 分钟、启动 20 秒 |
| 技能 | `packages/skill/skill-fast-anything/assets/dsh-fast-anything/SKILL.md` | 上游 `skills/fast-anything/SKILL.md` 改名 `dsh-fast-anything`，补 MCP 工具名、`path` 参数、暂存路径与「不要全局安装」、Rust 1.88 与 `.fast/target` 缓存、`enable`/`disable` 改写源码的警告；随包提供方 `@deepseek-ai/dsh-skill-fast-anything`，`/` 菜单归入 `内置 · EL4CTEO/fast-anything` |
| 挂载点 | `packages/bundle/web-app/presets/standard.patch.yml` | `standard` agent 得到 `skill-fast-anything` 与 `mcp-fast-anything` 两行；`minimal`/`ptc` 与 headless/acp/sdk 不挂，keyless 快照不受影响 |
| 类型/清单 | `tsconfig.base.json`、`tsconfig.host.json`、`packages/mcp/README*`、`packages/skill/README*`、生成的 config catalog / 模块图 / 技能包清单 | 让编译、产品可达性与文档门禁都认这两个包 |
| 本地化元数据 | 两个包的 `locale/{en,zh,ja,es,fr,de,ru,pt}.json` | `verify-locale-metadata` 要求 8 种发布语言的 `meta.title`/`meta.description` |
| 依赖形态 | `@deepseek-ai/dsh-mcp-client` 放在 `peerDependencies`（同时 devDependency） | 插件之间按仓库共享实例约定走 peer；模块图里出现 `fast-anything → mcp-client` 边 |
| 部署闭包 | `python/sdk-runtime/package.json` | `verify-runtime-closure` 要求 shipped preset 的插件与技能出现在部署清单里 |
| CI | `.github/actions/setup-rust/action.yml` 的 rust-cache 加上 `native/fast-anything` | 复用已安装的 Rust 1.97.0；不改任何 workflow 的 step 结构。`test`/`test:coverage` 与 `run-gates` 的 coverage / coverage-shard / unit 聚合会自动先跑 `build:fast-anything` |

**为什么挂在 preset 而不是 base bundle**：MCP server 会在系统提示里贡献 `## MCP resource servers` 段和 `### MCP server: fast-anything` instructions。挂在 base 会让每个 profile（含 headless/acp/sdk）的模型可见提示变化，`snapshots/session` 的期望文件全部要重录；挂在 Web 的 `standard` preset 只影响 Web 会话，keyless 快照套件不受影响。

## 2. 实测证据

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 源码编译 | `pnpm run build:fast-anything` | 通过；冷构建 30.3 s，暂存 2,676 KiB，`fast-anything --version` → `0.8.6` |
| 工具面 | 对 `fast-anything mcp` 发 `initialize` + `tools/list` | 8 个工具，schema 合计 3,020 字节，instructions 392 字节 |
| 包单测 | `pnpm vitest run packages/mcp/fast-anything packages/skill/skill-fast-anything scripts/build-fast-anything.spec.ts scripts/build.spec.ts scripts/run-gates.spec.ts` | 见第 3 节 |
| 类型检查 | `pnpm run typecheck` | 见第 3 节 |
| 全量构建 | `pnpm run build` | 见第 3 节 |
| 静态/文档门禁 | `build:check`、`check:ci:static`、`test:docs` | 见第 3 节 |
| 组合 | `pnpm dsh --profile web --dump-config` | `standard` preset 含 `skill-fast-anything` 与 `mcp-fast-anything`，`minimal` 不含 |

## 3. 验证记录

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 源码编译 | `pnpm run build:fast-anything` | 通过；`cargo build --release --locked` 冷构建 30.3 s；暂存 2,676 KiB；`--version` → `0.8.6` |
| MCP 工具面 | 直接对 `fast-anything mcp` 做 `initialize` + `tools/list` | 8 个工具（`fast_scan`、`fast_port`、`fast_verify`、`fast_enable`、`fast_disable`、`fast_bench`、`fast_report`、`fast_status`）；schema 3,020 B；instructions 392 B |
| 包单测 | `pnpm vitest run packages/mcp/fast-anything packages/skill/skill-fast-anything scripts/build-fast-anything.spec.ts scripts/build.spec.ts scripts/run-gates.spec.ts` | 5 个文件、228 个用例通过（其中 mount 5、skill 9） |
| 类型检查 | `pnpm run typecheck` | 5/5 通过（144.42 s） |
| 全量构建 | `pnpm run build` | 15/15 通过（34.51 s，Node 24.22.3；含 `fast-anything CLI` gate）；本机默认 Node 24.21.0 低于仓库与 rea 的引擎下限，rea gate 会先失败 |
| 构建后类型检查 | `pnpm run build:check` | 2/2 通过（10.03 s） |
| 静态门禁 | `pnpm run check:ci:static` | 71/71 通过（集成后先修掉生成物与两个既有失败，见 [logs/00129](logs/00129-修复集成遗留问题.md)） |
| 文档门禁 | `pnpm run test:docs` | 21/21 通过 |
| Lint | `pnpm run lint` | 0 warning / 0 error（7,784 个文件，78.9 s） |
| 仓库卫生 | `pnpm run hygiene` | 27/27 通过（见 [logs/00129](logs/00129-修复集成遗留问题.md)） |
| 覆盖率 | `pnpm vitest run --coverage`（只 include 两个包的 `src`） | 语句/分支/函数/行均 100% |
| 组合 | `pnpm dsh --profile web --dump-config` | `standard` preset 含 `skill-fast-anything` 与 `mcp-fast-anything`；`minimal` 不含 |
| 上游单元测试 | `cd native/fast-anything && PATH=/usr/bin:$PATH cargo test --release` | 84/84 通过（系统 Python 3.10.12；conda 的 Python 3.12.2 下有一个多进程 worker 用例失败，属环境差异） |
| 公开仓库链接 | `pnpm run verify-public-repository-links` | 见复查记录 |

## 4. 生效方式

- 新 clone：`pnpm install && pnpm run build`（用 cargo 编译，需要 Rust 1.88+），然后 `pnpm dsh web`；默认 preset 的会话直接带工具与技能。
- 已有 profile：bundle 与 preset 文件按仓库实时读取，重启 `dsh web` 后新会话生效；会话可 resume。
- 改了 `native/fast-anything/` 的源码：重跑 `pnpm run build:fast-anything`（或全量 `pnpm run build`），暂存文件随之更新。
- 想用别的二进制：设 `FAST_ANYTHING_BIN`，或在 preset 行的 `config.command` 指定。
- 验证一句话：重启后问「列出你可用工具里以 `mcp__fast-anything__` 开头的名字」，应看到 8 个；或直接说「用 fast-anything 把这个脚本变快」。

## 5. 使用方式（模型侧）

技能会引导：`fast_scan`（用应用真实运行方式）→ `fast_port`（录制最多 500 次真实调用，生成 `.fast/ports/<id>/` 的单函数 crate 与 `TASK.md`）→ 写 Rust → `fast_verify` → `fast_enable` → `fast_bench` / `fast_report`；`fast_disable` 回退，`fast_status` 看状态。工具都接受可选 `path`，默认取 server 进程工作目录。技能额外写了四条本仓库须知：

1. 移植需要 Rust 1.88+，`scan` 不需要；`fast-anything doctor` 报告机器处于哪种状态。
2. 生成的 crate 编译进目标项目自己的 `.fast/target`，`fast-anything clean` 清除，`clean --all` 同时把所有移植切回。
3. `fast_enable` 会改写目标项目的调用点，`scan`/`port` 会执行目标项目代码；在副本上跑并审查 diff。
4. `FAST_ANYTHING_DISABLE=1` 可以只为一次运行使用原代码。

## 6. 已知取舍与可选下一步

- **集成后修复**：本轮集成报告里的两项遗留问题已按用户要求修复，见 [logs/00129](logs/00129-修复集成遗留问题.md)：fresh checkout 上 rea 构建缺 `native/rea/scripts/lib/`（`lib/` 忽略规则）已提交补齐；`verify-plugin-entry-exports` 报的 session-controller 入口导出已改走 Host 服务方法。完整构建与静态门禁因此转为全绿。
- **每个 agent 的额外成本**：`standard` agent 多 11 个工具（8 个 server 工具 + 3 个资源工具，约 800 tokens schema 加 392 字节 instructions）和一个 server 进程；`minimal`/`ptc` 不受影响。嫌占上下文可把 preset 行的 `toolLoading` 改成 `on-demand`。
- **打包运行时（Desktop、Python SDK 可执行文件）**：两者都会挂载这个 preset，但二进制在所有包目录之外，打包不会带上；那里 MCP server 找不到可执行文件时不提供工具（fail-soft），与 rust-anything 的已知回退相同。后续做法是像 `native/system` 那样按平台暂存二进制。这是仍待完成的产品能力，不属于“修复”范围。
- **构建前置条件**：`pnpm run build` 需要 Rust 1.88+（已安装的 1.97.0 即可），冷构建约 30 s，`target/` 约 55 MB（已 gitignore）。
- **移植需要目标语言运行时**：Python 3.10+、Node.js 22+、Ruby 3.3+、Java 22+、Go 1.22+、PHP 8.1+（`ext-ffi`）、C/C++ 编译器、.NET 8+ 或 R 4.0+，按应用而定；`fast-anything doctor` 逐个报告。
- **工具会执行目标项目代码并改写源码**：`enable` 的 diff 需要人工审查；技能与 README 都要求只在副本上操作、不要把生产凭据留在环境里。
- **上游测试形态**：Rust 测试是 `src/` 内的内联单元测试（84 个 `#[test]`），没有独立 `tests/` 目录；`cd native/fast-anything && cargo test --release` 可直接跑，但未接入 `pnpm test`（与 rust-anything 的做法一致）。
- **duplication 门禁仍是红的**：基线 HEAD 已有 5 处克隆，集成后为 9 处（新插件/技能提供方沿用既有实现，与 rea 的接入方式一致）。清零需要把 bundled skill provider 与 MCP stdio 挂载抽成共享实现，涉及 5 个以上包，属于后续重构，本次未展开。

## 7. 回滚

```sh
git checkout -- package.json pnpm-lock.yaml .gitignore tsconfig.base.json tsconfig.host.json \
  packages/bundle/web-app/package.json packages/bundle/web-app/presets/standard.patch.yml \
  packages/mcp/README.md packages/mcp/README.zh.md packages/mcp/README.i18n.yaml \
  packages/skill/README.md packages/skill/README.zh.md packages/skill/README.i18n.yaml \
  python/sdk-runtime/package.json .github/actions/setup-rust/action.yml \
  docs/config-catalog.md docs/config-catalog.zh.md docs/module-graph.md docs/module-graph.zh.md \
  packages/preset/agent-preset/skills/dsh-doc/references/packages.md scripts/build.ts scripts/build.spec.ts \
  scripts/run-gates.ts scripts/run-gates.spec.ts scripts/baseline-inventory.ts scripts/check-workspace-constraints.ts \
  scripts/translation-pairing.manifest.json scripts/verify-concrete-terms.ts \
  scripts/verify-package-readme-model-experience.ts scripts/verify-repository-references.ts
git checkout -- README.md docs/development.md docs/development.zh.md docs/development.i18n.yaml \
  docs/subsystems/mcp.md docs/subsystems/mcp.zh.md docs/subsystems/mcp.i18n.yaml \
  docs/subsystems/skills.md docs/subsystems/skills.zh.md docs/subsystems/skills.i18n.yaml \
  docs/user/guide/index.md docs/user/guide/index.zh.md docs/user/guide/index.i18n.yaml \
  native/README.md native/README.zh.md native/README.i18n.yaml website/docs.ts
rm -rf native/fast-anything packages/mcp/fast-anything packages/skill/skill-fast-anything \
  scripts/build-fast-anything.ts scripts/build-fast-anything.spec.ts docs/user/guide/fast-anything.md \
  docs/user/guide/fast-anything.zh.md docs/user/guide/fast-anything.i18n.yaml
pnpm install
```

`native/fast-anything/target/` 与 `native/fast-anything/bin/` 都不进 git，删除目录即可清除构建产物。

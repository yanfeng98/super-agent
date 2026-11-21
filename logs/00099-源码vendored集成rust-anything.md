# 00099 — rust-anything 改为源码 vendored 集成

- **日期**：2026-11-21
- **背景**：00097 把上游 CLI 作为 `rust-anything@2.1.0` npm 依赖引入。用户要求改成"直接把上游源码搞进来"的方式（与 [00027 dsh-im](00027-原生集成dsh-im.md)、[00038 dsh-superagent](00038-原生集成dsh-superagent.md) 同一种做法）：源码进仓库、由 `pnpm run build` 现场编译，而不是下载预编译二进制。选择：构建方式 **A**（进 `pnpm run build`，cargo 现场编译），位置 **`native/rust-anything/`**。
- **后续**：面向用户的使用说明与其验证记录见 [`logs/00100`](00100-文档同步-rust-anything使用指南.md)（站点用户指南 `docs/user/guide/rust-anything`）。
- **范围**：`native/rust-anything/`（新 vendored 子树 + `NATIVE.md` + `rust-toolchain.toml`）、`scripts/build-rust-anything.ts`、`scripts/build.ts`、根 `package.json`、`packages/mcp/rust-anything/`、`python/sdk-runtime/package.json`、`.github/actions/setup-rust/`、12 个 workflow、`.gitignore`、翻译配对清单与三个 vendored 子树例外、`pnpm-lock.yaml`/`pnpm-workspace.yaml`、生成的 config catalog 与技能包清单、文档与 logs。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `native/rust-anything/`（新） | 上游 `v2.1.0`（`71c78e10c5beb39acecd52c2eb3f675a199f3cea`，2026-11-20 "Release 2.1.0"）的源码，7.1 MB：`Cargo.toml`、`Cargo.lock`、`src/`、`tests/`、`docs/`、`directory/`、`skills/` 与上游 Markdown 原样保留，`LICENSE`（MIT）保留。剔除只服务于外部分发的部分：`npm/`（npm 包装器）、`scripts/`（发布与营销脚本）、`demo/`（约 20 MB）、`.github/`、`.githooks/`、`.claude-plugin/`、`.mcp.json`、`.coderabbit.yaml`、`CLAUDE.md`、`.gitignore`、`.gitattributes`。 |
| `native/rust-anything/NATIVE.md`（新） | 记录基线 tag/commit、构建与挂载路径、本地改动、检查命令、上游同步流程（与 dsh-superagent 的 NATIVE.md 同构）。 |
| `native/rust-anything/rust-toolchain.toml`（新） | `channel = "1.97.0"`、`profile = "minimal"`。上游只声明 `rust-version = "1.97"`，靠调用方默认工具链；固定后 rustup 在该目录下自动选择（缺失时安装）。这是与上游唯一的文件差异。 |
| `scripts/build-rust-anything.ts` + `.spec.ts`（新） | 以 `native/rust-anything` 为 cwd 执行 `cargo build --release --locked`，把 `target/release/rust-anything[.exe]` 暂存到 `native/rust-anything/bin/`。cargo 缺失、编译失败或未产出可执行文件都抛出带指引的错误（指向 `NATIVE.md`）。暂存用「先写 `.tmp` 再 rename」，避免覆盖正在运行的二进制时 `ETXTBSY`；同目录 `build-rust-anything.spec.ts` 覆盖平台命名与两处路径。 |
| `scripts/build.ts` | artifacts 作用域新增 `rust-anything CLI` gate（`pnpm run build:rust-anything`），与 native addon 并列。 |
| 根 `package.json` | 新增 `build:rust-anything`；`test`、`test:coverage` 在建 native addon 之后也建该二进制，与既有 native-system 先例一致。 |
| `packages/mcp/rust-anything/src/binary.ts`（新） | `executableName`、`vendoredBinary(platform)`（解析 `native/rust-anything/bin/rust-anything[.exe]`）、`resolveCommand(command, env)`（`command` → `RUST_ANYTHING_BIN` → 仓库内构建产物）。 |
| `packages/mcp/rust-anything/src/index.ts` | 入口只保留 Loader 读取的 `name`/`inject`/`Config`/`apply`；解析逻辑移出（`verify-plugin-entry-exports` 不允许入口导出其它运行时值）。 |
| `packages/mcp/rust-anything/package.json` | 删除 `rust-anything` npm 依赖，改描述。 |
| `packages/mcp/rust-anything/tests/mount.spec.ts` | 断言暂存路径、三条解析分支、eager 16 个工具、on-demand 仅 loader，以及可执行文件缺失时 agent 照常创建（不注册工具）。 |
| `python/sdk-runtime/package.json` | 依赖补 `@deepseek-ai/dsh-rust-anything`：`verify-runtime-closure` 要求 shipped preset 的插件出现在部署清单里（00097 遗漏，本轮一并修好）。 |
| `.gitignore` | 忽略 `native/rust-anything/target/` 与 `native/rust-anything/bin/`。 |
| `.github/actions/setup-rust/action.yml`（新） | 组合动作：`dtolnay/rust-toolchain@v1` + `toolchain: 1.97.0`（该 action 的 `1.97.0` 是分支 ref 而非 tag，不能写成 `@1.97.0`），再 `Swatinem/rust-cache@v2` 缓存 `native/rust-anything`。 |
| 12 个 workflow（19 处） | 在会跑构建/测试的 job 里、checkout 步骤**之后**插入 `uses: ./.github/actions/setup-rust`：`ci.yml`（coverage、consumers、node-compat）、`ci-windows.yml`（build、coverage）、`ci-master.yml`（4 个 job）、`e2e.yml`、`release.yml`、`release-publish.yml`、`sandbox.yml`（2 个）、`build-preview-cloudflare.yml`、`macos-package.yml`、`macos-intel-package.yml`、`windows-package.yml`、`build-exe-for-python-sdk.yml`。 |
| `scripts/run-gates.ts` | `coverageGates`、`ciUnitGates` 各加一个 `build:rust-anything` gate（CI 这两个聚合直接跑 `vitest`，不经过 `test:coverage` 脚本），并让用例 gate 依赖它。 |
| `scripts/run-gates.spec.ts` | 按三段式 `test` 脚本与新增 gate 更新断言（含 Windows ready/complete 的 `completeOnly` 集合）。 |
| `pnpm-workspace.yaml` / `pnpm-lock.yaml` | 删除 `minimumReleaseAgeExclude` 的 `rust-anything@2.1.0` 条目；锁文件移除外部分发包。 |
| 门禁例外 | `translation-pairing.manifest.json` 加 `native/rust-anything/`；`verify-concrete-terms`、`verify-repository-references`、`baseline-inventory` 把该子树排除（上游措辞、基线 commit 引用与 fixture 原样保留）。 |
| 生成物 | `pnpm run gen-tsconfig-paths`（条目移到排序位置）、`gen-config-catalog`（补该插件条目）、`gen-plugin-packages`（补技能包清单）。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 全新 clone + `pnpm install && pnpm run build` | `rust-anything CLI` gate 用 cargo 现场编译（首次约 1 分钟以内、`target/` 约 430 MB，之后增量）并暂存 `native/rust-anything/bin/rust-anything`（26 MB）；随后 `pnpm dsh web` 的 `standard` agent 拿到 16 个 `mcp__rust-anything__*` 工具 + 3 个共享资源工具。 |
| 没有 Rust 工具链 | `rust-toolchain.toml` 让 rustup 自动安装 1.97.0；cargo 本身缺失时构建以可操作报错失败（不会静默降级）。 |
| `RUST_ANYTHING_BIN` / `command` | 仍然优先于仓库内构建产物，便于用自编二进制或调试版本。 |
| `pnpm run test` / `test:coverage` | 先建二进制再跑用例；`tests/mount.spec.ts` 连接真实可执行文件。 |
| 改动 vendored 源码 | 下次 `pnpm run build`（或 `build:rust-anything`）重新编译并覆盖暂存文件。 |
| 发布安装 | 不再适用：包不在 npm 上自带二进制，`bin/` 是仓库构建产物。 |
| CI | 会构建/测试的 job 通过组合动作装 1.97.0 并缓存 cargo 产物。 |

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm exec tsx scripts/build-rust-anything.ts` | 通过；冷编译 32.6 s，暂存 26068 KiB；`rust-anything --version` → 2.1.0 |
| `pnpm vitest run packages/mcp/rust-anything` | 5/5 通过 |
| 覆盖率（本包 `src`） | 语句/分支/函数/行 100% |
| `pnpm run build:check` | 2/2（host、client 聚合检查） |
| `pnpm run test:docs` | 21/21 |
| `pnpm run hygiene` | 26/26 通过（publint、包依赖、NodeNext consumer、可选依赖、入口导出等） |
| 全量单测 | `vitest run --maxWorkers=8` | 60,724 个用例、29 个文件失败/150 个用例失败；其中**只有 `scripts/build.spec.ts` 与本改动相关**（缺 `rust-anything` gate 断言，已修，现 20/20）。其余为本机/既有环境问题：无 bubblewrap 沙箱、worktree git、缺 ripgrep、客户端用例取不到 `@xmanrui/dsh-im/client`（该包是 private workspace）、`verify-third-party-notices` 的 qqbot 阻断、临时目录类用例 |
| `pnpm run check:ci:static` | **70/70 通过**（`DSH_GATE_CONCURRENCY=4`，250.1 s）。默认并发下本机会有两个 gate 因资源问题失败（137=OOM、CLI help 子进程起不来），单独重跑与降并发重跑均通过——是 15 GB 机器的负载问题，不是改动 |
| `pnpm run verify-cordis-config` / `verify-module-graph` / `verify-locale-metadata` / `verify-package-dependencies` / `constraints` / `verify-product-use` / `verify-default-product-isolation` | 通过（377 个配置、3 个生成产物、8 语言元数据、103 个包、299/53 个包、363 个 Web 组合插件） |
| `pnpm run build`（全量） | **13/13 通过**（31.4 s 增量），含新增的 `rust-anything CLI` gate。首轮曾在 `Superagent plugin` gate 失败，原因是工作区里并发的 `apps/web/native/dsh-superagent` 未提交改动（删了 GitHub source 而 `tsdown.config.ts` 仍引用）；那部分改动收敛后全绿，与本改动无关。 |
| 组合检查 | `pnpm dsh --profile web --dump-config` | `standard` preset 中出现 1 处 `mcp-rust-anything`，`minimal` 不含 |
| oxlint（改动文件） | 0 warning / 0 error |
| 门禁图 spec | `pnpm vitest run scripts/run-gates.spec.ts` 168/168；workflow 相关 spec 共 279 通过 |
| workflow 结构 | 12 个 workflow + 组合动作全部通过 YAML 解析；19 处 `setup-rust` 都是独立 step（无 `with`），且 checkout 的 `with:` 与改动前逐项一致 |

## 4. 设计取舍

- **为什么是 `native/`**：仓库的 `native/` 已经是"导入的原生源码子树"（`.oxlintrc.json` 排除 `native/**`，README 说明其自带门禁）；上游是 Rust 原生 CLI，放这里语义最贴，且不需要把 Rust crate 塞进 pnpm workspace 或 `vendor/` 的 tsdown 机制。
- **为什么进 `pnpm run build`**：用户选择方案 A。与 dsh-im/superagent 的差别只在语言：它们用 tsdown，这里用 cargo；代价是构建前置条件多一条 Rust 1.97（CI 用组合动作补齐），换掉的是"构建产物来自网络下载"这件事。
- **为什么固定 `rust-toolchain.toml`**：本机 `cargo` 默认 1.96，`Cargo.toml` 的 `rust-version` 只会在编译时报错；固定 channel 后 rustup 负责选择与安装，`pnpm run build` 不再依赖调用者把默认工具链换到 1.97。
- **为什么二进制暂存在 `native/rust-anything/bin/`**：构建产物跟着源码树走（与 `native/system` 把产物写进平台包 `bin/` 一致），消费方 `packages/mcp/rust-anything` 按固定相对路径解析；`target/` 与 `bin/` 都不进 git。
- **为什么把解析逻辑移出入口**：`verify-plugin-entry-exports` 只允许函数插件入口导出 `name`/`inject`/`Config`/`apply`；把它们放进 `src/binary.ts` 后入口导出面回到协议允许的集合。
- **`test` 先建二进制**：沿用 `pnpm run test` 已先跑 `build:native-system` 的先例；否则 mount 用例会因缺少可执行文件失败。

## 5. 已知限制与后续

- **CI 覆盖面**：组合动作已接入会构建/测试的 12 个 workflow 的目标 job，但未在真实 CI 上跑过（本机 `gh` 无账号）。若新增构建类 job，需要同样插入该 step。
- **打包运行时（Desktop、Python SDK 可执行文件）不含该 CLI**：二进制在包目录之外，打包只带包内文件；这两处挂载后不会出现 rust-anything 工具（`fail-soft`）。上一版用 npm 依赖时它们能拿到二进制，因此这是源码 vendored 的已知回退，后续需要按平台暂存。
- **`python/sdk-runtime` 的二元性**：清单里现在包含本插件（部署闭包要求），但 python SDK 可执行文件不会带上 `native/rust-anything/bin/` 下的产物；在该运行时里 MCP server 会因找不到可执行文件而不提供工具（`fail-soft`，agent 创建不受影响）。若要让它真正可用，需要把 Rust 构建接进 `build-exe-for-python-sdk.yml` 或在该运行时禁用这行 preset。
- **本机静态门禁默认并发偏大**：15 GB 机器上 12 个并发 gate 会出现 OOM（137）与 CLI 子进程失败；本地复跑请用 `DSH_GATE_CONCURRENCY=4`。CI 的 runner 不受此影响。
- **上游 `cargo test` 未接入 `pnpm test`**：`native/rust-anything/tests/` 原样保留，可按需手动运行；仓库门禁只跑本包的 mount 用例。
- **`verify-third-party-notices` 仍被 fork 自带的 `@tencent-connect/qqbot-connector`（缺 repository 字段）阻断**，与本改动无关；修好后 `rust-anything`（MIT）应进入 notices。
- **本机完整 `pnpm run build` 暂不能全绿**：`apps/web/native/dsh-superagent` 有并发未提交改动，与本改动无关；`rust-anything CLI` gate 本身在本次运行中通过。
- **构建成本**：冷编译约 30–60 s、`target/` 约 430 MB（`.gitignore` 排除），CI 用 rust-cache 缓存 `native/rust-anything`。

## 6. 复查记录

| 轮次 | 发现 | 处理 |
|---|---|---|
| 1 | `pnpm run build` 在本机用默认 cargo 1.96 直接失败（oxc 要求 1.97） | 加 `native/rust-anything/rust-toolchain.toml` 固定 1.97.0，构建脚本以 crate 目录为 cwd 运行 |
| 1 | `verify-plugin-entry-exports` 报入口导出了 `vendoredBinary`/`resolveCommand` | 抽出 `src/binary.ts`，入口只留协议导出 |
| 1 | `verify-runtime-closure` 报 `standard preset -> @deepseek-ai/dsh-rust-anything` 不在部署清单（00097 遗留） | `python/sdk-runtime/package.json` 补 workspace 依赖 |
| 1 | `gen-tsconfig-paths --check`、`gen-config-catalog --check`、`gen-plugin-packages --check` 报生成物过期（均自 00097 起就未重新生成） | 重新生成三个产物（tsconfig 条目回到排序位置） |
| 1 | `check:ci:static` 首轮里 `verify-cli-help` 在并行负载下子进程失败 | 单独重跑通过，判定为并发负载下的偶发，不属于本次改动 |
| 1 | 全量 `pnpm run build` 的 `Superagent plugin` gate 失败 | 确认为工作区中并发的 dsh-superagent 未提交改动（删除 GitHub source）所致，未触碰他人改动；其改动收敛后重跑 13/13 通过 |
| 2 | **CI step 插入位置错误**：把 `setup-rust` 插在 `- uses: actions/checkout` 与其 `with:` 之间，`persist-credentials`/`fetch-depth` 等被挂到 setup 步骤上（8 个文件受影响） | 按"checkout 步骤块结束后再插入"的规则重排所有插入点，并用 YAML 解析 + 结构断言复核：19 处均为独立 step，checkout 的 `with` 与 HEAD 逐项一致 |
| 2 | **coverage/unit 聚合不经过 `test:coverage` 脚本**：`coverageGates` 直接 `vitest run --coverage`、`ciUnitGates` 直接 `vitest run`，所以只改根脚本 CI 仍拿不到二进制 | 在 `run-gates.ts` 的两个聚合里各加 `build:rust-anything` gate 并让用例依赖它，同步更新 `run-gates.spec.ts` |
| 2 | **`dtolnay/rust-toolchain@1.97.0` 引用的不是 tag**：该 action 的版本 ref 是分支，`1.97.0` 在 refs 里不存在 | 改为 `@v1` + `with: toolchain: 1.97.0` |
| 2 | **漏接 node-compat 与 python SDK 可执行文件**：前者在 Node 24.9 上跑 `build:artifacts`，后者默认跑 `pnpm run build` | 两个 job 都补 `setup-rust` |
| 2 | **`scripts/build.spec.ts` 没有跟上构建图**：新增 `rust-anything` gate 后该 spec 仍然只断言旧图（顺带发现它也没跟上本 fork 的 `dsh-im`/`superagent` gate 与 `web` 依赖，属于既有的红） | 把 `artifactGraph` 更新为实际图（含三个 fork gate 与 `web` 的依赖），`scripts/build.spec.ts` 20/20 通过 |
| 2 | **vendored 树保真度未验证** | 用 `git archive <tag>` 重新导出上游树并 `diff -r`：除新增 `NATIVE.md`、`rust-toolchain.toml` 与构建产物 `bin/`、`target/` 外与上游一致；提交时又按仓库空白门禁去掉了两个上游文件末尾的多余空行（已记入 `NATIVE.md` 的本地改动） |
| 2 | **技能里还写着 npm 安装路径**：`SKILL.md` 称 CLI 在 `node_modules/.bin/rust-anything`，并建议 `cargo install rust-anything` + `rust-anything setup` | 改为"由本检出编译到 `native/rust-anything/bin/`"，删掉全局安装与 `setup` 建议 |
| 2 | **`packages/mcp/README*` 行描述仍是 npm 依赖** | 中英文改为 vendored 源码编译，并重记 i18n 哈希 |
| 2 | **构建脚本在 `CARGO_TARGET_DIR` 被设置时给出难以理解的 ENOENT** | 先检查产物是否存在，报错指明该环境变量 |
| 3 | **`dsh web` 正在运行时重跑构建会 `ETXTBSY` 失败**：`copyFileSync` 直接覆盖正在执行的可执行文件 | 改为"先在旁边写 `.tmp`，再 `renameSync` 覆盖"；POSIX 下运行中的进程保留旧 inode。实测：两个 `rust-anything mcp` 进程存活时重跑，构建成功、服务不受影响。Windows 覆盖运行中的 exe 仍会失败，报错改为提示先停掉运行中的会话 |
| 3 | **打包运行时失去该工具（相对上一版是回退）**：可执行文件在所有包目录之外，Desktop 应用与 Python SDK 可执行文件只装插件、没有 CLI | 在包 README（中英）、`NATIVE.md`、集成报告里明确写出该限制与后续做法（像 `native/system` 那样按平台暂存） |

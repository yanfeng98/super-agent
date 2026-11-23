# 00119 — 内置 rust-anything 升级到 3.3.0

- **日期**：2026-11-23
- **背景**：上游 [erkamyaman/rust-anything](https://github.com/erkamyaman/rust-anything) 在本仓库基线 `v2.1.0` 之后发布了 2.2–3.3 系列，累计 81 个提交。内置的源码副本（`native/rust-anything/`）需要按 [NATIVE.md](../native/rust-anything/NATIVE.md) 的同步流程升级，跟随它的是 MCP 包、随包技能与所有描述工具面/成本的文档。
- **结论**：vendored 基线升到 `v3.3.0`（`c0862b8`，2026-11-22）；MCP 工具面 16 → 22，schema 从约 21 KB 增到 29 KB（约 8k tokens）；PHP 与 C# 单元可以 `bridge`；`cargo test --release` 在本机只余 3 处宿主环境限制。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `native/rust-anything/` | 按 `v3.3.0` 重新落地（346 个文件、8.7 MB，vendored 路径内 152 个文件 +18,464/−1,337 行）。新增 `plugin/` 与 `.mcp.json`：上游 `tests/plugin.rs` 校验插件副本的技能与 server 声明，缺了就红 |
| `native/rust-anything/NATIVE.md` | 新基线、复制清单（含 `plugin/`、`.mcp.json` 的来由）、22 个工具、共享 cargo target 目录、本地改动三条、宿主限制、同步步骤 |
| `native/rust-anything/.gitignore`（新增） | `!tests/fixtures/**`：抵消仓库根 `lib/` 忽略规则对上游 fixture 的影响，见 §2 |
| `packages/skill/skill-rust-anything/assets/dsh-rust-anything/SKILL.md` | 用 `v3.3.0` 文本重新同步，重放四处本地改写（`name`、语言覆盖含 Kotlin/PHP/C#、PHP 与 C# 段落、CLI 句、末尾 installation 小节）；新增上游的 `go`、`enable`/`disable`、C 指针类型等段落 |
| `packages/skill/skill-rust-anything/README{,.zh}.md` | 本地改写「三处」→「四处」，补共享缓存与 `enable`/`disable` 的提醒 |
| `packages/mcp/rust-anything/README{,.zh}.md`、`src/index.ts` | 16 → 22 个工具、21 KB/6k → 29 KB/8k tokens；语言清单补 PHP/C#；删掉「2.1.0 相对路径失败」的限制，改为共享 target 目录与语言运行时要求 |
| `packages/mcp/rust-anything/tests/mount.spec.ts` | eager 用例断言完整的 22 个工具名（新增工具或改名会让它红，逼着 README 的数字跟着动） |
| `packages/bundle/web-app/presets/standard.patch.yml` | 注释里的 16 → 22、7k → 8k tokens |
| `docs/user/guide/rust-anything{,.zh}.md` | 工具表补 `go`/`fuzz`/`report`/`enable`/`disable`；相对路径那句换成共享缓存；token 与 `enable` 提醒 |
| `README.md` | 差异表：22 个工具、约 8k tokens、语言清单、`go` 与 `app_*` 的分工 |
| `rust-anything-integration.zh.md` | 基线、证据表（编译、工具面、上游测试）、限制（共享缓存、`cargo test` 跳过项） |
| `rust-anything-analysis.zh.md` | 头部补一句：本文分析的是 `v2.1.0`，当前基线已到 `v3.3.0` |
| `docs/config-catalog{,.zh}.md` | 重新生成（生成的 Config 片段引用了 `16 tool schemas` 这条 JSDoc） |
| `docs/user/guide/rust-anything.i18n.yaml`、两个包 README 的 `.i18n.yaml` | 两侧同步后重新记录配对 |
| `logs/00119-…`（本文件） | 记录本次升级 |

## 2. 原因与行为

- **上游这 81 个提交带来什么**：`go`（一个慢命令的端到端：profile + bridge + record + 写 `TASK.md`，回来再跑一次就 build/diff/gate 并在全部通过后启用）、`enable`/`disable`（在 TS/JS、Python 源码里插一段标记块，把函数就地绑到 Rust）、`report`（`report/REPORT.md` + `report/card.svg`）、`fuzz`、`package`（把整个 Node CLI 打成可安装的 npm 包）；PHP 与 C# 单元可以 `bridge`（`ext-ffi` JSON dispatcher / P/Invoke），两者都不支持 `shadow` 与 `--batch`；端口与被桥接 crate 共用一个 cargo target 目录（`CARGO_TARGET_DIR`，否则平台缓存目录下的 `rust-anything/target`）；`profile` 把 workload 的输出改走 stderr，MCP 协议流不再被污染。工具名 16 → 22 就是这么来的。
- **上游技能文本滞后**：`v3.3.0` 的 `skills/rust-anything/SKILL.md` 仍然写着 PHP 与 C# 单元不能 `bridge`，与它自己的 CHANGELOG、`analyze` 输出（`PHP FFI (C ABI)`）以及本机实测（`bridge` 会以「static method … stay in PHP」逐条说明）矛盾。vendored 子树照旧逐字节保留上游文本；随包技能是模型实际看到的面，因此按 3.3.0 的真实行为改写，并在技能 README 里记为第四处本地改写，下次同步时一并重放。
- **`.gitignore` 是必需的**：仓库根 `.gitignore` 用 `lib/` 忽略所有同名目录，正好命中上游 10 个 fixture 文件（`tests/fixtures/ruby/lib/`、`tests/fixtures/lib/`、`tests/fixtures/aliases/src/lib/`）。`analyze` 与整应用流程按 gitignore 语义遍历，于是这些单元直接消失：`cargo test` 在 `tests/analyze.rs`、`tests/app.rs`、`tests/loop.rs` 里各有失败（`ruby: []`、`no unit or source file lib/textkit/`、`src/lib/` 少一个）。在子树里加 `!tests/fixtures/**` 后四个用例立刻恢复，`target/`、`bin/` 仍被根规则忽略。
- **工具数与成本**：`tools/list` 22 个工具，序列化 29,146 字节；按仓库此前的换算（约 3.5 字节/token）约 8k tokens。`standard` preset 保持 `eager`，未改产品行为，只把数字更新到实际值。
- **共享编译缓存**：迁移生成的 crate 首次编译会写到 `~/.cache/rust-anything/target`（遵循 `CARGO_TARGET_DIR`/`XDG_CACHE_HOME`），不再落在被迁移仓库的 `rust/*/target`；这是用户可见的行为变化，写进了 NATIVE.md、包 README 与用户指南。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm run build:rust-anything` | 通过；暂存 27389 KiB，`bin/rust-anything --version` → `3.3.0` |
| `rust-anything mcp` 的 `tools/list` | 22 个工具、29,146 字节；`initialize` 的 instructions 未变 |
| `pnpm vitest run packages/mcp/rust-anything packages/skill/skill-rust-anything` | 14/14 |
| `cd native/rust-anything && cargo test --release --no-fail-fast` | 34 个套件 426 通过 / 4 失败，4 个全是 §4 的宿主环境限制 |
| `pnpm run typecheck` | 5/5 |
| `pnpm run gen-config-catalog` + `verify-config-catalog` | 生成 3 个产物、写入 2 个；复查通过 |
| `pnpm run verify-translation-pairing` | 3 个配对重新记录后，1409 对全部一致 |
| `pnpm run doc-sync`（`DSH_GATE_CONCURRENCY=2`） | 43/43（首轮 41 通过 / 2 失败，均为上面两项；修复后复跑通过） |
| `pnpm run hygiene`（`DSH_GATE_CONCURRENCY=2`） | 27/27 |

## 4. 备注

- **本机环境限制（非本次改动引入）**：`cargo test` 有 3 个用例要宿主先装东西——`tests/native.rs` 的 Java 与 Ruby 运行时期望需要 `javac`，Ruby 那条还要 libclang（rb-sys 经 bindgen），`tests/loop.rs` 的 `ruby_modules_bridge_record_and_diff` 同样卡在 libclang；`tests/http.rs` 的 `c_server` 依赖 `cc` 的 `<string.h>` 在不定义 `_GNU_SOURCE` 时声明 `strcasestr`，本机 gcc 11 + glibc 2.35 下 fixture 会记到空的 POST body。三者都已写进 NATIVE.md 的检查小节。
- **下次同步**：照 NATIVE.md 的四步走，注意新复制清单多了 `.mcp.json` 与 `plugin/`，并且要保住 `rust-toolchain.toml`、`.gitignore` 与两处行尾空行裁剪；工具数变化时，`mount.spec.ts` 会先红，再同步 README、指南、preset 注释与 `gen-config-catalog`。
- **未做**：`go --agent` 与 `package` 在 DSH 里没有专门的默认用法说明；技能沿用上游「不要把 `--agent` 传给别的 agent」的口径，`package` 只在工具表里出现。

## 5. 复查修正

本轮自查改掉七处不准确或不稳的表述：

| 位置 | 修正 |
|---|---|
| `docs/user/guide/rust-anything{,.zh}.md` | 工具表按用途重排，`go` 从「规划」行独立成「只搬一个热点」；正文的循环那句补上 `go`；`toolCallTimeoutMs` 的说明补 `go`（它是最长的一次调用） |
| `packages/mcp/rust-anything/README{,.zh}.md` | 超时字段补 `go`；执行注意事项补一句 `enable` 会改写源码里被迁移函数的调用点 |
| `packages/mcp/rust-anything/tests/mount.spec.ts` | 工具集断言改成排序后比较：新增或改名会红，顺序变化不会 |
| `native/rust-anything/NATIVE.md` | `c_server` 的宿主条件改成实际观察到的现象：POST 拿到空 body，随后其余用例超时（手工 `curl` 复现了空 body，套件里从 POST 起全部 `timed_out`） |
| `native/rust-anything/NATIVE.md`（同步步骤） | 「三处本地文件」改成「本地新增与裁剪」：三条本地改动里只有两条是文件（`rust-toolchain.toml`、`.gitignore`），第三条是对两个上游文件的裁剪 |
| `native/rust-anything/.gitignore` | 反选原来写成 `!tests/fixtures/**`，会连带取消 `tests/fixtures/**/node_modules` 的忽略（上游测试会在 fixture 里装依赖）；收窄成 `**/lib/` 与 `**/lib/**`，只放行 `lib` 目录，`analyze`、`app`、`loop` 三个受影响的用例复跑仍通过 |
| 包 README 与用户指南（中英） | 共享 target 目录那句原来写成「工作树里只多出 `rust/` crate」，夸大了：`enable` 会改原始源码，录制、`map --write` 与 shadow 模块也落在 crate 旁。改成「编译一个端口只在被迁移仓库里留下它自己的 `rust/` crate」 |

自查同时确认了几条容易被当成「改坏了」的判断：`git archive <tag> .mcp.json` 可用（同步步骤写法成立）；`native/rust-anything/` 确实在 `translation-pairing.manifest.json` 的排除项里；仓库的 skill 提供方只扫 `<project>/.agents/skills`，新增的 `plugin/skills/` 不会冒充第二个技能。

另外把这次踩到的坑写成标准指令：根 `AGENTS.md` 的「Run relevant checks locally」新增一条「共享主机压低并发」（`DSH_GATE_CONCURRENCY=2`、`pnpm run test -- --maxWorkers=2`、重活逐个跑），`verify-doc-budgets` 的根 `AGENTS.md` 上限从 2,080 提到 2,220（2,101 词，留出 5% 余量），`docs/AGENTS.md` 的 target 同步更新。

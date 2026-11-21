# 00106 — README 补 rust-anything 在本仓库中的用法

- **日期**：2026-11-21
- **背景**：用户澄清上一轮的「目前这个项目如何用」问的是 rust-anything，不是整个项目的通用用法。上一轮落地的 [`## 使用`](00105-README补充使用章节.md) 只把 rust-anything 压成表格里的一行，读不出「在**这个仓库**里它已经装好了、怎么触发、产物落在哪、什么时候不该用」。
- **范围**：根 `README.md` 新增 `### 把代码迁移到 Rust`，并把 rust-anything 从「内置能力」表中移出，避免同一件事写两遍。

## 1. 变更

| 位置 | 内容 |
|---|---|
| `### 把代码迁移到 Rust` | 触发方式（直接说任务，或 `/` 点名 `dsh-rust-anything`）、单元循环八个步骤、`gate` 为完成标准、整应用走 `app_*` |
| 同上第二段 | CLI 由 `pnpm run build` 编译到 `native/rust-anything/bin/rust-anything`；生成的 crate 落在**目标仓库**的 `rust/<unit>-rs/`；回放会执行被录制的代码，只在副本上做、不放生产凭据；不需要时用 `--patch` 禁用 `mcp-rust-anything` 行，或改 `toolLoading: on-demand` |
| 同上第三段 | 指向单元级指南、整应用技能正文、`NATIVE.md` |
| 内置能力表 | 删除 rust-anything 行（内容并入上面的小节），其余四行不变 |

## 2. 事实核对

`native/rust-anything/bin/rust-anything doctor`：

- `ok  mcp server` — 「…/bin/rust-anything mcp answered with 16 tools」，即本仓库的挂载路径与工具数量与实际一致；README 里「已经挂载、不需要安装」由此得到验证。
- `outdated  cargo  cargo 1.96.0 …; rust-anything needs 1.97+` — 这是对 `PATH` 上默认工具链的检查；构建本身走 crate 的 `rust-toolchain.toml`（1.97.0），因此 `pnpm run build` 不受影响。README 不写这条警告，避免把「构建能用」和「doctor 报警」混为一谈。
- `missing  Claude Code mcp / skill` — 那些是 `rust-anything setup` 面向外部 agent 的注册项；本仓库通过 MCP 插件挂载，不需要跑 `setup`，README 也不提。

「禁用 `mcp-rust-anything` 行」沿用既有文档化的写法：profile patch 里 `- id: <行> / disabled: true`，与[工程技能指南](../docs/user/guide/engineering-skills.zh.md)的「覆盖或移除」小节一致。

## 3. 验证

| 检查 | 结果 |
|---|---|
| `pnpm run test:docs` | 21/21 |
| `DSH_GATE_CONCURRENCY=4 pnpm run doc-sync` | 43/43 |

编辑过程中曾把「启动片头包」的链接路径误写成 `packages/client/ui-boot-animation` 之外的名字，随即改正；最终的 `markdown links` 门禁覆盖了该路径。

## 4. 取舍

- **不写 `rust-anything setup`**：它是给 Claude Code、Codex 这类外部 agent 注册用的；本仓库走 `mcp-rust-anything` 插件挂载，写了反而让人以为要额外配置。
- **不展开 16 个工具名**：工具清单由 MCP client 的 `toolLoading` 与 server 决定，README 列举会漂移；这里只给数量与命名前缀。
- **保留「产物写进目标仓库」这条警告**：这是在本仓库里用它会踩到的唯一真实风险（在 `super-agent` 自己的树上跑迁移会生成 `rust/<unit>-rs/` 并执行被录制代码），属于 README 该承担的提醒，细节仍留在使用指南。

## 5. 复查：把风险句写清楚

用户读不懂原句「生成的 crate 写在目标仓库的 `rust/<unit>-rs/`，回放时会执行被录制的代码」。原文三处含糊：没说明「目标仓库」就是被迁移的那个仓库（不是本仓库）、没说明哪几步真的会运行目标代码、也没说清楚具体后果。已改写为一句自足的说明：crate 落在**你指定的目标仓库**并出现在它的工作树里；`record`、`bench`、`shadow`、`app record` 会真的运行目标代码（`record --test-command` 还会跑它的测试套件），这些代码以当前用户身份读写文件、访问网络；因此迁移在副本或临时 worktree 上做，生产凭据不留在环境里。

实测证据（临时演示仓库 `/tmp/ra-demo2`，2 行 JS）：

```
$ rust-anything bridge /tmp/ra-demo2 src/math.mjs --kind napi
  → 生成 /tmp/ra-demo2/rust/math-rs/{Cargo.toml,rust-anything.json,index.cjs,src/port.rs,…}
$ rust-anything record /tmp/ra-demo2/rust/math-rs
Recorded 24 cases (20 threw) in …/cases.jsonl
  slug: 12   sum: 12   Coverage: 2 of 2 original lines (100%)
```

`record` 的产物里包含 `{"fn":"sum","args":[null],"error":"Cannot read properties of null (reading 'reduce')"}` 这类真实运行结果，说明它确实把目标函数跑了起来，而不是静态分析。bridge 只写文件、不编译，也不会执行目标代码。

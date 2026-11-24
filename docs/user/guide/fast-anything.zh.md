# 用 fast-anything 让慢的部分变快

[English](fast-anything.md) | 中文

Web 的 `standard` agent preset 挂载了 [fast-anything](https://github.com/EL4CTEO/fast-anything) MCP server，因此默认 Session 可以把 Python、Node.js/TypeScript、Ruby、Java、Kotlin、Go、PHP、C、C++、C# 或 R 应用里最热的纯函数搬到 Rust，从而提速。每个移植都对照原代码在真实调用中的行为校验，只有在全部录制调用、派生边界用例与收缩输入都匹配之后才会启用。不需要全局安装：CLI 由本仓库里 vendored 的 crate 现场编译。

## 自带什么

| 阶段 | 工具 |
|---|---|
| 找出热点函数 | `fast_scan` |
| 移植单个函数 | `fast_port`、`fast_verify` |
| 启用移植 | `fast_enable`、`fast_disable` |
| 度量与报告 | `fast_bench`、`fast_report`、`fast_status` |

## 第一次 Session 之前

`pnpm install && pnpm run build` 会编译 CLI 并暂存为 `native/fast-anything/bin/fast-anything`。构建需要 `PATH` 上有 Rust 1.88 或更新版本；更新版本（例如仓库另一个 Rust 构建安装的 1.97.0）也可以，因此这个 crate 没有固定的 toolchain 文件。`pnpm dsh web` 不会重新构建，所以构建一次即可（vendored crate 变更后再构建），然后重启 server；已经打开的 Session 保留它启动时拿到的工具。

## 请求提速

描述任务，匹配的[技能](../../../packages/skill/skill-fast-anything/assets/dsh-fast-anything/SKILL.md)会加载；也可以在输入框输入 `/`，从 **内置 · EL4CTEO/fast-anything** 分组里选 `dsh-fast-anything` 先加载。例如：

- "这个脚本要跑五秒，找出热点函数并让它变快。"
- "剖析这条命令，把最慢的纯函数移植到 Rust。"
- "去重那一步占了大部分时间：把它搬到 Rust，并证明行为一致。"
- "撤销 `fuzzy.levenshtein` 的 Rust 移植，并测量差异。"

流程是 `fast_scan`、`fast_port`、按返回的任务写 Rust、`fast_verify`、`fast_enable`、`fast_bench` 与 `fast_report`；`fast_disable` 会把函数切回原实现，`fast_status` 列出扫描与移植过的内容。想确认挂载时，让 Agent 列出以 `mcp__fast-anything__` 开头的工具。

## 会留下什么

扫描与移植会在项目里写一个 `.fast/` 目录：每个移植的录制调用与生成的单函数 crate、描述要写什么的 `TASK.md`，`fast_report` 之后还有 `report.md` 与 `report.svg`。生成的 crate 都编译进共享的 `.fast/target` 缓存；`fast-anything clean` 会删除它，`clean --all` 还会把所有移植切回原实现。已启用的移植就是函数调用点上的一段带标记代码块，`fast_disable` 会按字节原样移除。`FAST_ANYTHING_DISABLE=1` 可以在不改动项目的情况下，让一次运行使用原代码。

## 直接使用 CLI

```sh
./native/fast-anything/bin/fast-anything scan -- python3 app.py
./native/fast-anything/bin/fast-anything port fuzzy.levenshtein
./native/fast-anything/bin/fast-anything verify fuzzy.levenshtein
./native/fast-anything/bin/fast-anything enable fuzzy.levenshtein
./native/fast-anything/bin/fast-anything report
```

MCP 工具可以传可选的 `path` 指定项目目录，否则使用 server 的工作目录；CLI 使用当前目录。

## 成本与限制

- 移植需要 Rust 1.88 或更新版本，以及被测量应用自己的运行时。`fast-anything doctor` 会报告哪些部分已就绪：`scan` 只需要应用的运行时，`verify` 与 `bench` 还要用 Rust 构建。
- 工具会以 Session 的环境执行应用代码并向项目写入内容，`fast_enable` 还会改动项目源码。代码不是一次性的时候请在副本上运行，审查 `enable` 留下的 diff，并避免把生产凭据放进环境。
- fast-anything 只移植它能搬动的最外层循环：辅助函数涉及 I/O、随机性、全局变量，或者函数就在入口脚本里时，会被报告为暂不可移植并给出原因。把纯逻辑挪进独立模块通常就能让它可移植。
- 这不是整应用重写：应用的其余部分保持原样。要在 Rust 里重写整个应用，请用 [rust-anything 迁移指南](./rust-anything.zh.md)。
- 退路：`fast-anything clean --all`、`fast_disable` 或 `FAST_ANYTHING_DISABLE=1` 都能让应用回到原代码。

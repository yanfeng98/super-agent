# 00104 — 把 Rust 前置条件写进 README 与开发指南

- **日期**：2026-11-21
- **背景**：用户问「环境配置是否已写进 README，比如新 clone 的项目」。核对后确认：`pnpm run build` 已把 rust-anything CLI 的编译设为必过 gate（见 [日志 00099](00099-源码vendored集成rust-anything.md)），但新 clone 之后最先读到的两份入口文档都没提 Rust —— 根 `README.md` 的「从源码运行」只要求 Node 与 pnpm，`docs/development.md` 的「前置条件」同样只有 Node；Rust 1.97 只写在特性页 [`docs/user/guide/rust-anything.md`](../docs/user/guide/rust-anything.md) 里。
- **后果**：照 README 操作的机器上没有 `cargo` 时，`pnpm run build` 在 `rust-anything` gate 处整体失败，报错只出现在构建输出里，读者没有任何前置提示。

## 1. 变更

| 位置 | 变更 |
|---|---|
| 根 `README.md`「从源码运行」 | 前置条件补 `Rust`（`rustup` 安装；构建按 crate 固定的 1.97.0 拉工具链） |
| 根 `README.md` 构建说明段 | 点明 `pnpm run build` 包含从 `native/rust-anything` 现场编译的 CLI、首次几分钟、中间产物在 `target/` 不入库、缺少 `cargo` 时整个构建失败 |
| `docs/development.md` / `.zh.md`「前置条件」 | 新增一条 Rust 条目：指向 vendored 目录，说明 crate 用 `rust-toolchain.toml` 固定 1.97.0、rustup 按需安装、缺 `cargo` 时构建失败 |
| `docs/development.i18n.yaml` | 「前置条件」小节中英哈希重录 |

## 2. 验证

| 检查 | 结果 |
|---|---|
| `pnpm run verify-translation-pairing --write docs/development.md` | 写入 1 条记录，`docs/development.i18n.yaml` 仅「前置条件」段两处哈希变化 |
| `pnpm run test:docs` | 21/21 |
| `DSH_GATE_CONCURRENCY=4 pnpm run doc-sync` | 43/43 |

新增的中英互链目标 [`native/rust-anything/NATIVE.md`](../native/rust-anything/NATIVE.md) 存在；`docs/**` 在 `verify-md-links` 扫描范围内，这两处链接由门禁校验。

## 3. 取舍

- **README 与开发指南各写一句**：Node 前置条件本来就在这两处各有一份，Rust 保持同样布局；细节（1.97 报错怎么修）仍只留在特性页的排障清单里，不复制。
- **不在 README 展开 `rustup toolchain install` 步骤**：`rust-toolchain.toml` 让 rustup 自动拉取，读者只需装 rustup；手动装工具链是失败后的排障路径，属于特性页。
- **没有把 Rust 检查提前到 `pnpm install` 阶段**：`postinstall` 只装 Git 钩子，提前探测会让「装依赖」变成另一种失败面；gate 在构建时报错已经足够显式。
- **遗留缺口**：`pnpm run build` 还会编译 native addon（需要 C/C++ 工具链），这一点在上游文档里同样没有写进新 clone 的入口；本轮只补 Rust，避免把范围扩散到既有的原生工具链描述。

# 00100 — 文档与日志同步：rust-anything 使用指南

- **日期**：2026-11-21
- **背景**：用户问"用户目前如何用这个项目"。此前 rust-anything 只在包 README、根 README 与集成报告里有零散说明，站点上没有面向用户的入口；同时第 2–4 轮复查（CI 接线、`ETXTBSY`、打包运行时回退、全量单测归因）的结论需要写进变更记录。
- **范围**：`docs/user/guide/rust-anything.{md,zh.md,i18n.yaml}`（新）、`docs/user/guide/index.{md,zh.md}`、`website/docs.ts`、根 `README.md`、`logs/00099`。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `docs/user/guide/rust-anything.md` / `.zh.md` / `.i18n.yaml`（新） | 面向用户的使用指南：内置的 16 个工具按阶段分组；构建前置（`pnpm run build` 编译 vendored crate、Rust 1.97 由 `rust-toolchain.toml` 固定、`dsh web` 不重新构建）；四类典型话术；产物落点（`rust/<unit>-rs`、`cases.jsonl`、`rust/PORT_MAP.md`、`shadow`）；直接使用 CLI 的三条命令与绝对路径要求；`toolLoading`/`command`/`toolCallTimeoutMs`/`env` 四个字段；成本与限制（约 7k tokens、每会话一个进程、只挂 Web/Desktop 的 `standard`、打包运行时无 CLI、生成 crate 需 1.97、会执行被录制代码）；排障（工具缺失、MSRV、cargo 缺失、`doctor`）。 |
| `docs/user/guide/index.md` / `.zh.md` | 「继续使用」列表首条之后加入该指南，中英同位置、同顺序。 |
| `website/docs.ts` | 注册页面：路由 `guide/rust-anything.md`，节点标签「迁移到 Rust / Port to Rust」，归入「集成 / Integrations」小节，`order: 3`。 |
| 根 `README.md` | rust-anything 行补「使用指南」链接。 |
| `logs/00099` | 补第 3、4 轮复查：`ETXTBSY` 覆盖运行中二进制、打包运行时不含 CLI 的回退、全量单测的失败归因、`scripts/build.spec.ts` 期望更新；同步替换为最新验证数字。 |

## 2. 行为

- 站点「集成」小节新增「迁移到 Rust」页，中英文成对发布，与「记忆 MCP」「插件市场」同级。
- Web UI 指南的「继续使用」列表可直接进入该页。
- 根 README 的「与上游的差异」表为该能力同时给出包契约、preset、使用指南、技能与变更记录五个入口。
- 页面内的事实来自可执行来源：工具清单取自 MCP `tools/list`（16 个）、CLI 参数取自 `rust-anything --help` / `map --help` / `bridge --help`、成本与限制取自包 README、路径取自 `native/rust-anything/NATIVE.md`。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm run test:docs` | 21/21（含 `verify-md-links`、`verify-md-wrap`、`verify-translation-pairing`、`verify-doc-budgets`、`doc-standard`）；首轮曾因中英代码块内容不一致被配对门禁拒绝，改成同一份命令行后通过 |
| `pnpm vitest run scripts/project-doc-site.spec.ts` | 通过（站点投影，含缺失链接的失败路径） |
| `pnpm run website:build` | 28.69 s 完成；`verify-doc-site-fragments` 校验 5448 个站内锚点、导出 211 个 raw Markdown 文件 |
| 站点产物 | `website/.dist/guide/rust-anything.html` 与 `.dist/en/guide/rust-anything.html`；侧栏「集成」小节出现「迁移到 Rust / Port to Rust」 |

## 4. 取舍

- **放 `docs/user/guide/` 而不是包 README**：包 README 是对使用者的契约（配置字段、token 影响、限制），读者是集成者；「打开 Web UI 后怎么说、产物在哪、坏了怎么修」是产品用户视角，属于 `docs/user/`。两者互相链接，不重复整段内容。
- **不写进 `docs/subsystems/mcp.md`**：子系统页面向维护者，说明挂载方式与责任边界；逐步操作属于用户层。
- **链接保持仓库相对路径**：与既有用户指南一致（`../../../packages/...`、`../../../.agents/...`），由站点投影统一改写，`verify-md-links` 校验目标存在。
- **不新增预算条目**：`verify-doc-budgets` 只对常设文档设上限，用户指南不在清单内，按 review 把握篇幅。

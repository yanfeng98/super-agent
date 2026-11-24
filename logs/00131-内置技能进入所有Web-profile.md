# 00131 — 内置技能进入所有 Web profile

- **日期**：2026-11-24
- **背景**：用户报告 `/` 技能菜单里只有 `mattpocock/skills` 的 eng-* 系列，`dsh-rea`、`dsh-rust-anything`、`dsh-fast-anything` 都不在菜单里，并怀疑 bundled-provider 抽取是原因。
- **根因**：不是共享提供方的问题。`skill-engineering` 挂在 `dsh-base` 的全局层，`skill-office` 挂在 `dsh-web-app` 的全局层，所以它们对每个会话可见；这三个技能只挂在 `standard` preset 的 scope 层。dsh-superagent 的执行器与主 Session 分别使用 `superagent-worker` 与 `superagent-router` preset，两者都没有这三行，于是技能目录里完全没有它们。
- **范围**：`packages/bundle/web-app/cordis.patch.yml`（三个技能行从 preset 移到宿主补丁）、`packages/bundle/web-app/presets/standard.patch.yml`（只保留对应的 MCP 工具行）、`packages/bundle/web-app/tests/bundled-skills.spec.ts`（新）、`README.md` 与 `docs/subsystems/mcp{,.zh}.md`。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `cordis.patch.yml` | 在 `skill-office` 旁新增 `skill-rust-anything`、`skill-rea`、`skill-fast-anything` 三行，与 office、工程技能一样进入 Web bundle 的全局层；不配置 `mcp-*`，不增加任何工具目录。 |
| `standard.patch.yml` | 删除三个技能行，保留 `mcp-rust-anything`、`mcp-rea`、`mcp-fast-anything` 及其 `toolLoading` 配置；注释改为说明技能随 Web bundle 全局挂载、工具仍按 preset 选择。 |
| `bundled-skills.spec.ts`（新） | 断言三个技能行在宿主补丁中按固定顺序出现，且不在任何 preset 里重复；断言三个 MCP 行仍在 `standard` preset 且顺序不变。 |
| 文档 | README 能力表、菜单分组说明与 fast-anything 段落，以及 `docs/subsystems/mcp{,.zh}.md`，改为“技能随 Web bundle 全局挂载、工具仍随 `standard` preset”。 |

## 2. 校验

| 命令 | 结果 |
|---|---|
| `vitest run packages/bundle/web-app/tests/bundled-skills.spec.ts` | 3/3 通过；新增运行时用例把三个提供方挂到宿主层，再从无 preset 行的 agent scope 列表，确认 `/` 菜单可见 |
| `vitest run packages/skill/{skill,skill-rea,skill-rust-anything,skill-fast-anything,skill-engineering}/tests` 与 bundled-skills | 6 文件、78 用例通过 |
| `vitest run packages/api/session-controller/tests` | `session-skills.host.spec.ts` 通过；同批其他文件因工作树缺 `@babel/code-frame`、`@shikijs/langs` 等依赖符号链接失败，与本次改动无关 |
| `tsx scripts/verify-runtime-closure.ts` | 通过 |
| `tsx scripts/gen-plugin-packages.ts --check` / `gen-module-graph.ts --check` / `gen-config-catalog.ts --check` | 全部 up to date |
| `tsx scripts/verify-md-links.ts` / `verify-doc-budgets.ts` / `verify-md-wrap.ts` | 通过 |

## 3. 已知限制

- 技能正文在非 `standard` preset 中可见，但没有对应的 MCP 工具；这与 `skill-office` 在 PTC/minimal preset 中可见而 `tool-office` 未挂载的行为一致。需要工具的会话应选择 `standard` preset。
- 工作树用符号链接复用主 checkout 的 `node_modules`，部分深层依赖无顶层链接，少数与本次改动无关的测试因此未能启动；完整 CI 在具备完整依赖树的环境中运行。
- 分支相对基点 `1bcd578257` 只保留 fast-anything 特性、bundled-provider 抽取与本工单修复三个提交；原先顺带带入的 rea scripts 补齐与 session-controller 导出修复属于其他工单，已移出本分支，相关文档中的悬空引用也已清掉。

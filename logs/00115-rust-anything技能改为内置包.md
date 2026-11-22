# 00115 — `dsh-rust-anything` 技能改为内置包

- **日期**：2026-11-22
- **背景**：用户在输入框敲 `/` 时，`dsh-rust-anything` 归在「项目 · .agents/skills」下，而它驱动的是随包能力。同类技能应当像 `@deepseek-ai/dsh-skill-engineering` 一样随包提供，而不是作为本仓库的项目技能存在。
- **结论**：新建 `packages/skill/skill-rust-anything`（`@deepseek-ai/dsh-skill-rust-anything`），把 `.agents/skills/dsh-rust-anything/` 的两份文件 `git mv` 进它的 `assets/dsh-rust-anything/`，由 Web `standard` preset 与 `mcp-rust-anything` 一并挂载。

## 1. 变更

| 位置 | 变更 |
|---|---|
| `packages/skill/skill-rust-anything/` | 新包：`src/index.ts`（提供方 `dsh-rust-anything`，`source: 'bundled'`、`upstream: 'erkamyaman/rust-anything'`、`shipped: true`、`BUNDLED_SKILL_RANK`）、`assets/dsh-rust-anything/{SKILL.md,whole-app.md}`、`tests/`、`locale/*.json`、双语 README 与 `package.json`、`tsconfig.json` |
| `.agents/skills/dsh-rust-anything/` | 删除；内容以 `git mv` 进入上面的 `assets/`，工作树里只剩一份 |
| `packages/bundle/web-app/presets/standard.patch.yml` | 在 `mcp-rust-anything` 前插入 `skill-rust-anything` 行，让技能与工具同进同出 |
| `packages/bundle/web-app/package.json` | 增加 `@deepseek-ai/dsh-skill-rust-anything` 依赖 |
| `packages/skill/skill-rust-anything/assets/dsh-rust-anything/SKILL.md` | 末节 `In this checkout` → `This installation`；CLI 那一句改为「本仓库把可执行文件编到 `native/rust-anything/bin/rust-anything`」，并补一句「两者都没有时告诉用户，不要装全局副本」 |
| `README.md` | 菜单示例补 `内置 · erkamyaman/rust-anything`；rust-anything 段与差异表改为「随包技能 `dsh-rust-anything` + MCP 工具」 |
| `docs/user/guide/rust-anything.md` / `.zh.md`、`packages/mcp/rust-anything/README*.md`、`native/rust-anything/NATIVE.md`、`rust-anything-integration.zh.md` | 技能链接改指新包资产 |
| `packages/skill/README*.md`、`docs/subsystems/skills*.md` | 技能族页面与提供方清单加入新包 |
| `packages/preset/agent-preset/skills/dsh-doc/references/packages.md` | `pnpm run gen-plugin-packages` 重新生成 |
| `tsconfig.base.json` | `pnpm run gen-tsconfig-paths` 写入新包的源码别名 |
| `tsconfig.host.json` | Host 聚合工程补上新包引用，`tsc -b` 才能编译到它的源码 |
| `scripts/gen-doc-graphs.ts` + `docs/capability-seams.md` | skills seam 的 implementations 增加 `skill-rust-anything` |
| `scripts/verify-package-readme-model-experience.ts` | `SENTENCE_MODEL_EXPERIENCE` 增加该包：模型文本由 `dsh-tool-skill` 渲染 |
| 上述双语对的 `.i18n.yaml` | 用 `pnpm run verify-translation-pairing --write` 重新录制分节哈希 |
| `logs/00115-…`（本文件） | 记录本次改动 |

## 2. 行为

- **菜单**：`standard` preset 的会话里，`/` 菜单多出 `内置 · erkamyaman/rust-anything` 组，内含 `dsh-rust-anything`；`内置`（office 4 个）、`内置 · mattpocock/skills`（27 个工程技能）不变。模型与用户都可调用，描述命中即自动加载。
- **来源与归因**：正文取自 vendored 上游 `native/rust-anything/skills/rust-anything/`，因此条目带上游项目名，按来源单独成组，而不是与只有本仓库自有的 office 技能混在一起。
- **范围**：`minimal`、`ptc` 与 headless、ACP、SDK profile 不挂这条技能，与它们拿不到 `mcp__rust-anything__*` 工具一致。
- **覆盖关系**：项目或用户技能目录里放一份同名 `dsh-rust-anything` 仍然覆盖随包版本；同一层内随包 rank 排在最后。
- **关闭**：把 `standard.patch.yml` 里的 `skill-rust-anything` 行换成 `disabled: true`。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `npx vitest run packages/skill/skill-rust-anything` | 9/9，含真实 `cordis.yml` 经 Loader 的搬迁资产组合测 |
| 该包 `src/index.ts` 覆盖率（v8，`--coverage.include`） | 100% 语句/分支/函数/行 |
| `pnpm run verify-cordis-config` | 377 个配置通过 |
| `pnpm run test:docs` | 21/21 |
| `pnpm run verify-doc-graphs`、`verify-tsconfig-paths`、`verify-plugin-packages`、`verify-locale-metadata`、`verify-package-meta`、`verify-skill-invocation-metadata`、`verify-package-readme-summaries`、`verify-package-readme-model-experience` | 全部通过 |
| `npx tsx scripts/run-oxlint.ts packages/skill/skill-rust-anything` | 0 error / 0 warning |
| `node --import tsx/esm -e "import('@deepseek-ai/dsh-skill-rust-anything')"` | 导出 `Config`、`apply`、`name`，源码启动路径可达 |

## 4. 取舍

- **挂在 preset 而不是 base 或 web-app bundle**：rust-anything 的 MCP server 本来就只在 `standard` preset 里；技能与工具挂在一起，就不会出现「有技能、没工具」的空壳。放到 base 会让 headless、ACP、SDK 也列一条它们用不上的技能。
- **单独建包而不是塞进 `packages/mcp/rust-anything`**：与 `tool-office`/`skill-office`、`skill-engineering` 的分工一致，技能提供方与工具挂载各自可禁用、可替换资产根。
- **保留上游归因**：正文确实是上游 `skills/rust-anything/` 的改编，`upstream` 让菜单分组与 README 的同步说明都有依据；本仓库自有的 office 技能则不带上游名。
- **删掉项目副本而不是保留**：保留会同时出现 `项目 · .agents/skills` 与随包两份同名技能，用户看到的仍是项目级那条（rank 更近的一层胜出），等于没修。
- **正文措辞从「本 checkout」放宽**：技能随包分发后不再是某一次 clone 的私有说明；同时保留本仓库的真实路径，避免编造全局安装步骤。

## 5. 复核后修正

第一次提交只跑了 `test:docs`、`doc-sync`、`typecheck` 与包测试，漏掉三类只在 `hygiene` / 打包校验里才走到的地方，复核时补齐：

| 问题 | 证据 | 修正 |
|---|---|---|
| 工作区约束不认新的 `files` 条目 | `npx tsx scripts/check-workspace-constraints.ts` 报 `files must be ["locale/*.json","lib/index.js",…,"src"]` | `scripts/check-workspace-constraints.ts` 的 `packageFileExtras` 登记 `@deepseek-ai/dsh-skill-rust-anything: ['assets']`，与 `skill-office`、`skill-engineering` 一致 |
| Python SDK 的运行时闭包缺依赖 | `pnpm run verify-runtime-closure` 报 `standard preset -> @deepseek-ai/dsh-skill-rust-anything … missing from python/sdk-runtime dependencies` | `python/sdk-runtime/package.json` 增加 `workspace:*` 依赖（SDK 可执行文件打包的就是 Web 的 `standard` preset） |
| 指南没说清打包运行时的表现 | 打包的 Desktop / SDK 里 MCP 工具不存在，但技能随 preset 一起挂载 | [使用指南](../docs/user/guide/rust-anything.md)「成本与限制」两条改写：技能与工具同挂同一 preset；打包环境里技能仍列出，并要求 Agent 停下来说明而不是自行安装 |

另外，本地 `pnpm run hygiene` 里的 publint 一度失败，原因是新包只有 `tsc -b` 产出的 `lib/types/`，没有 tsdown 的 `lib/index.js`；跑一次 `npx tsdown --config-loader native`（等价于 `build:lib`）后 `npx publint packages/skill/skill-rust-anything` 全部通过。这是产物状态，不是源码问题，但说明新包确实会被 tsdown 的 workspace 自动发现并构建。

复核后的校验：`verify-runtime-closure`、`check-workspace-constraints`（含 183 个 spec 用例）、`verify-package-paths`、`verify-dsh-package-licenses`、`verify-plugin-entry-exports`、`verify-default-product-isolation`、`publint` 全部通过；`pnpm run hygiene` 的 25 个门禁里，此前唯一失败的 publint 已在构建后通过。

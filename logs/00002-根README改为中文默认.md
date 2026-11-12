# 00002 — 根 README 改为中文默认

- **日期**：2026-11-12
- **背景**：用户只保留中文 README，并要求英文有、中文没有的内容先补进中文。
- **范围**：按确认结果，只处理仓库根 README；包级 README 保持现有中英成对结构不变。

## 1. 内容补全

- 已在上一步把英文根 README 独有的 Discord 邀请补进中文版（企微群二维码保留）。
- 复核确认：英文根 README 的章节、运行命令、社区入口、贡献、开发、引用、许可证在中文版中均有对应内容。

## 2. 文件变更

| 文件 | 操作 |
|---|---|
| `README.md` | 内容替换为原 `README.zh.md`，并删除文件开头的 `[English](README.md) \| 中文` 语言切换行 |
| `README.zh.md` | 删除 |
| `README.i18n.yaml` | 删除（配对记录不再需要） |
| `scripts/translation-pairing.manifest.json` | 在 `excluded` 中加入 `README.md`，告诉配对门禁根 README 只保留中文 |

## 3. 链接修正

删除 `README.zh.md` 后，以下中文文档中指向根 README 的链接由 `README.zh.md` 改为 `README.md`（仅改根 README 链接，包级 `README.zh.md` 链接保持不变）：

- `.agents/notes/proposed/process/2026-08-20-audience-first-documentation-quality.zh.md`
- `docs/user/develop/basic/index.zh.md`
- `docs/user/develop/basic/publish.zh.md`
- `docs/user/guide/index.zh.md`
- `docs/user/guide/providers.zh.md`

随后用 `pnpm run verify-translation-pairing --write <上述 5 对>` 重新记录这 5 对文档的 i18n 哈希。

## 4. 校验结果

| 命令 | 结果 |
|---|---|
| `pnpm run verify-translation-pairing` | 1261 对全部一致 |
| `pnpm run verify-md-links` | 2530 个文件，相对链接与锚点全部可解析 |
| `pnpm run verify-md-wrap` | 2565 个文件，无硬换行段落 |
| `pnpm run verify-doc-budgets` | 7 个受限文档均在预算内 |
| `pnpm run verify-doc-refs` | 4990 个文件，文档引用均可解析 |
| `pnpm exec vitest run scripts/translation-pairing.spec.ts` | 76 个测试通过 |

未运行 `verify-doc-site-fragments`：该门禁需要先执行 `docs:build` 生成 `website/.dist`，与本次改动无关。

## 5. 复查与修正

复查发现一处不一致：`docs/i18n/README.md` / `README.zh.md` 的「范围」仍写着「除 vendor 源码外的全部 README」，与新 manifest 中排除根 README 的事实矛盾。

- 中英两侧同步改为「除根 README 和 vendor 源码外的全部 README」，并在「排除」列表中增加根目录 README 条目，说明它只以中文维护、没有 `.zh.md` 对侧与一致性记录。
- 用 `pnpm run verify-translation-pairing --write docs/i18n/README.md` 重新记录该对文档的哈希。
- 复查后再次执行全量配对检查：1261 对全部一致。

补充校验：`pnpm exec vitest run scripts/doc-standard.spec.ts` 22 个测试通过；`verify-md-links`、`verify-md-wrap`、`verify-doc-budgets` 在最终状态重新执行并通过。

## 6. 第二轮复查

- 与删除前的英文根 README 逐节比对：章节、命令、社区入口、贡献、开发、引用、许可证均有对应中文内容；中文额外含企微群二维码。
- 与删除前的中文版逐行 diff：仅语言切换行被移除、社区列表补入 Discord 两项预期差异。
- 全量运行 `pnpm run test:docs`：21 个文档门禁全部通过（含 translation pairing、markdown links、markdown wrap、package README 系列、Agent Note 系列）。
- 修正 `README.md` 中「参与贡献」的链接文案：`[CONTRIBUTING.md](CONTRIBUTING.zh.md)` 改为 `[贡献指南](CONTRIBUTING.zh.md)`，避免默认中文 README 里出现英文文件名。
- 再次执行 `verify-md-links` 与 `git diff --check`，均通过。

## 7. 后续约定

- 根 `README.md` 为唯一版本，不再新增 `README.zh.md` 或 `README.i18n.yaml`。
- 包级 README 仍是中英成对；如需同样改为纯中文，需要逐个包处理，并改造 `verify-package-readme-model-experience` 与 `verify-package-readme-limitations` 两个英文标题门禁。

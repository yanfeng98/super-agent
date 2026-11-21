# 00105 — README 补「使用」章节

- **日期**：2026-11-21
- **背景**：用户问「目前这个项目如何用」，并在口头回答后要求「更新到 readme」。此前根 `README.md` 只覆盖到「怎么起」（从源码运行、日常开发），起起来之后的界面操作、本仓库内置的能力入口、以及非 Web 的启动模式都没有落地到入口文档，读者必须自己翻 `docs/user/` 和 `apps/`。
- **范围**：根 `README.md` 新增 `## 使用` 一节（28 行，位于「日常开发」与「查找插件」之间）。

## 1. 变更

| 位置 | 内容 |
|---|---|
| `## 使用` 引言 | 配置模型 → 选择工作区 → 新建会话的顺序前提；未选中工作区时输入框不可用 |
| 列表 | 直接描述任务；输入框 `/` 打开技能菜单（27 个工程技能：11 个自动加载、16 个点名，项目与用户目录同名技能优先覆盖）；空草稿 ↑ 召回最近 100 条；插件页 |
| `### 内置能力` 表 | IM 机器人、Superagent、rust-anything、工程工作流技能、启动片头，各带入口位置与归属文档链接 |
| `### 命令行模式` | `web`、`headless`、`acp`、`sdk` 的启动命令，`--patch`、`--dump-config`、`plugin add`，profile 首次使用从随附模板初始化，桌面端两条命令 |

## 2. 事实核对

写命令表时先验证再落笔，纠正了一处凭上游 `--help` 示例臆断的写法：

```
$ pnpm dsh --profile tui --dump-config
Error: dsh: profile "tui" does not exist; create it with 'dsh plugin --profile tui add <package>'
```

`dsh --help` 的示例用 `tui` 指代「自定义 profile」，随附模板实际只有 `web`、`headless`、`acp`、`sdk`、`sdk-minimal`（`desktop` 名称保留给 Electron）。初稿里的 `pnpm dsh tui # 终端交互界面` 会直接失败，已换成实际存在的 profile。

同时确认 `pnpm dsh web --dump-config` 可执行（launcher 选项放在 profile 名之后，与 `--help` 示例一致），因此命令表按该顺序书写。

其余数字取自现有文档：27 个技能（11 + 16）、召回上限 100 条、IM 渠道清单、Superagent 默认只用本地工单。

## 3. 验证

| 检查 | 结果 |
|---|---|
| `pnpm run test:docs` | 21/21 |
| `DSH_GATE_CONCURRENCY=4 pnpm run doc-sync` | 43/43 |

新增链接均指向已存在文件：`.agents/skills`、`apps/web/native/dsh-im/NATIVE.md`、`apps/web/native/dsh-superagent/docs/006-deployment.md#quick-start`、`packages/skill/skill-engineering/README.zh.md`、`packages/client/ui-boot-animation/README.zh.md`、`docs/user/guide/rust-anything.zh.md`、`docs/user/guide/engineering-skills.zh.md`、`docs/user/guide/python-sdk.zh.md`、`apps/cli/README.zh.md`。

## 4. 取舍

- **只放到「入口 + 链接」这一层**：界面细节留在 [`docs/user/guide/index.zh.md`](../docs/user/guide/index.zh.md)，各能力的契约留在各自包 README 与 `NATIVE.md`；README 不复制可执行步骤的正文。
- **不复述「与上游的差异」表**：那节记录「改了什么」，这节回答「怎么用」，两者对象不同；启动片头这类只在差异表里成立的条目，这里只写入口。
- **命令表不覆盖 desktop profile**：`desktop` 由 Electron 持有，CLI 拒绝为它启动，桌面端走 `pnpm run dev:desktop` / `start:desktop`，不列 `--profile desktop`。

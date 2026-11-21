# 00107 — README「使用」章节自查：三处事实错误

- **日期**：2026-11-21
- **背景**：用户要求复查最近几轮的改动。逐条对照源码与真实命令验证 [00105](00105-README补充使用章节.md)、[00106](00106-README补充rust-anything用法.md) 落地的 README 文本，查出三处写错、一处表述含糊。文档门禁只校验链接、换行与配对，查不出「命令或界面标签与实现不符」。

## 1. 错误与修正

| # | 原文 | 实际 | 修正 |
|---|---|---|---|
| 1 | `record --test-command` 会跑测试套件 | `record` 没有该选项；真实用法是把命令放在 `--` 之后，由 `--from-tests` 打开：`rust-anything record <crate> --from-tests -- npx jest` | 改为 `record --from-tests -- <测试命令>` |
| 2 | 「用 `--patch` 把 `mcp-rust-anything` 行禁用」 | profile 的 `--patch` 层级触达不到 preset 内部条目：`dsh: [/tmp/ra-off.yml] patch: entry "mcp-rust-anything" not found`，条目仍为 `toolLoading: eager` | 改为「删掉 `packages/bundle/web-app/presets/standard.patch.yml` 里的 `mcp-rust-anything` 行」，并保留「改成 `on-demand`」这条同样落在该行的做法 |
| 3 | 设置页「IM bots」 | 中文 locale 标题是 `IM 机器人 (dsh-im)`（`apps/web/native/dsh-im/locale/zh.json`） | 改为「IM 机器人」 |
| 4 | 「`.agents/skills`、项目与用户目录中的同名技能优先覆盖」 | 优先级方向没写清 | 改为文档原话：项目 `.dsh/skills`、`.agents/skills` 与用户技能目录里的同名技能会覆盖内置版本 |

## 2. 错误 2 的排查过程

`vendor/include/src/patch.ts` 的 `applyPatches` 只用 `mount()` 建立 id 索引，而 `mount` 仅在 `options.group && Array.isArray(options.config)` 时递归。`dsh --dump-config` 显示 `mcp-rust-anything` 位于 `preset-standard` 行的 `config.plugins` 下，不是 `config` 数组，因此不在索引里；补丁只能命中 profile 层直接声明的条目。preset 的定义归属 `dsh-web-app` bundle 的 YAML，改它才是改 preset 组成的方式。

## 3. 复查中确认无误的部分

| 断言 | 证据 |
|---|---|
| 27 个工程技能＝11 个模型加载 + 16 个 `/` 点名 | `find packages/skill/skill-engineering/assets -name SKILL.md \| wc -l` → 27；带 `disable-model-invocation: true` 的 16 个 |
| 侧栏「Superagent」→「执行」→「新建工单」 | `locales.ts`：`panel: 'Superagent'`、`'tab.execution': '执行'`、`newTicket: '新建工单'` |
| 「设置 → 模型」「选择工作区」 | `ui-settings-models/locales.ts` 的 `nav: '模型'`；`ui-conversation/locales.ts` 的 `'hero.chooseWorkspace': '选择工作区'` |
| `web`、`headless`、`acp`、`sdk`、`sdk-minimal` 是随附 profile | `ls ~/.dsh/profiles`；`apps/cli/README.zh.md` 的 profile 说明 |
| `pnpm dsh web --help`、`pnpm dsh web --dump-config` 可用 | 直接执行，均正常返回 |
| `--patch` 写在 profile 名之后 | 用 `/tmp/ra-off.yml`、`/tmp/ra-ondemand.yml` 实测被解析并应用（仅报上述「entry not found」警告） |
| 16 个 MCP 工具、`record` 会执行目标代码 | `rust-anything doctor` 的 `mcp server … answered with 16 tools`；`/tmp/ra-demo2` 的 `record` 产出 24 条真实运行结果 |

## 4. 验证

| 检查 | 结果 |
|---|---|
| `pnpm run test:docs` | 21/21 |
| `DSH_GATE_CONCURRENCY=4 pnpm run doc-sync` | 43/43 |

## 5. 取舍

- **不再在 README 提供 `--patch` 关停 preset 内插件的写法**：机制不支持，写出来只会让人照做失败。真要定制组成，按 preset registry 文档新建或覆盖 preset 行，那属于 bundle 补丁层面的操作，超出 README 一节该承担的深度。
- **保留「改 `standard.patch.yml`」这条**：本仓库自己维护这份 preset，删行 + 重启是直接可验证的做法；对比「新建 preset」成本更低。
- **门禁之外的检查靠人工**：界面标签、CLI 选项这类断言没有任何脚本校验，只能对着 locale 文件与 `--help` 核对；本轮把核对过的清单写进第 3 节，便于下次复用。

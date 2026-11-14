# 00011 — Composer 按文件夹的输入历史

- **日期**：2026-11-14
- **背景**：用户要求 composer 支持终端风格的 ↑/↓ 输入历史，历史范围由文件夹控制（不同文件夹历史不同），每个文件夹保留最新 100 条。
- **范围**：`packages/client/ui-conversation` 的 composer 输入链路、键盘映射、测试、双语 README；新增 Web 浏览器场景；`docs/user/guide` 用户指南；本日志。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/client/ui-conversation/src/client/input/history.ts` | 新增按文件夹的 localStorage 读写：key `dsh.conversation.input-history.v1.<cwd>`，值为最旧在前、最新在后的 JSON 字符串数组，上限 100；过滤空白与非字符串项，读取失败或存储不可用返回空，写入失败静默忽略；`atHistoryBoundary` 判断多行条目的首行/末行边界。 |
| `packages/client/ui-conversation/src/client/input/facade.ts` | `SessionInputDeps` 增加可选 `history` 源；新增 `historyStep` 浏览逻辑、`startHistory`/`recallHistory`/`cancelHistoryBrowse`；召回时先安装浏览状态再应用草稿（`applyResolvedDraft`），避免迟到的 Lexical 更新取消浏览；草稿不变的召回更新不重新 arm trigger 菜单；`commit-draft` 先记录被接受的文本再清空，记录失败不影响清空；`setDraft`、`requestDraftInitialization`、`submit` 结束浏览。 |
| `packages/client/ui-conversation/src/client/input/hub.ts` | 为每个 Session shell 注入历史源：文件夹取自 `sessions.list` 的 `cwd`，读写复用 `history.ts`。 |
| `packages/client/ui-conversation/src/client/input/editor/keymap.ts` | ↑/↓ 在 slash 菜单裁决为 `pass`、非 IME 组合、无 Shift/Ctrl/Meta/Alt 时调用 `historyStep`；历史消费则 `preventDefault`。 |
| `packages/client/ui-conversation/src/client/input/editor/view-binding.ts` | 把 `ComposerKeyboard.historyStep` 接入 keymap，锁定/提交中不浏览。 |
| `packages/client/ui-conversation/src/client/contract/draft-editor.ts` | `ComposerKeyboard` 增加 `historyStep(direction)`。 |
| `packages/client/ui-conversation/src/client/contract/input.ts` | `commit-draft` 效果增加 `sent`（提交面接受的 clipboard 文本）。 |
| `packages/client/ui-conversation/src/client/input/machine.ts` | 消息与命令成功结算的 `commit-draft` 都携带 `sent`。 |
| `packages/client/ui-conversation/tests/input-history.client.spec.ts` | 新增 22 个测试：存储解析/上限/隔离/失败、边界、shell 召回与取消、多行与 CRLF、斜杠命令不弹菜单、记录成功/失败、Hub 真实存储记录。 |
| `packages/client/ui-conversation/tests/keymap-routing.client.spec.tsx` | 新增 ↑/↓ 路由、菜单消费、修饰键/IME、历史不消费时回落的测试；补齐既有 handler 的 `historyStep`。 |
| `packages/client/ui-conversation/tests/submit-machine.client.spec.ts` | 更新 `commit-draft` 精确断言以包含 `sent`。 |
| `apps/web/tests/composer-input-history.e2e.ts` | 新增真实 Chromium 场景：两个文件夹各写入历史，验证只召回当前文件夹、↑/↑/↑/↓/↓ 顺序、召回 `/compact` 后再次 ↑ 不被菜单抢走。 |
| `apps/web/tsconfig.json`、`tsconfig.host.json` | 新 e2e 文件加入 host 类型检查，并从 Web client 工程排除。 |
| `packages/client/ui-conversation/README.md` / `.zh.md` / `.i18n.yaml` | Shell 章节新增输入历史行为；Known Limitations 增加浏览器本地、精确路径分桶的限制；重新记录双语哈希。 |
| `docs/user/guide/index.md` / `.zh.md` / `.i18n.yaml` | 新增「召回之前的输入」小节，说明 ↑/↓、100 条、浏览器本地与精确路径范围；重新记录双语哈希。 |

## 2. 行为

| 场景 | 结果 |
|---|---|
| 空草稿、无附件，当前文件夹有历史 | ↑ 召回最新一条；继续 ↑ 更旧；↓ 更新；↓ 越过最新一条清空草稿 |
| 草稿非空 / 有附件 / 文件夹未知 / 无历史 | 不进入历史，方向键保持原行为 |
| 多行条目 | ↑ 只在首行离开文本、↓ 只在末行离开文本；其余位置方向键移动光标 |
| 召回后编辑 | 立即结束历史导航，草稿保留编辑结果 |
| 斜杠菜单打开 / IME 组合 / 带修饰键的箭头 / 非折叠选区 | 不进入历史 |
| 成功发送普通消息或命令 | 以被接受的 clipboard 文本写入当前文件夹历史 |
| 发送失败/被拒绝、空白草稿、与最新一条相同的相邻重复 | 不记录 |
| 存储不可用/配额满、Session catalog 暂时不可用 | 历史读取为空或写入忽略；召回回退为普通方向键；已接受草稿仍正常清空 |
| 第 101 条 | 挤掉最旧一条，始终保留最新 100 条 |

**存储格式**：`localStorage['dsh.conversation.input-history.v1.' + cwd] = JSON.stringify(string[])`，数组最旧在前、最新在后，读取时过滤空白/非字符串并截取最后 100 条。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm run build` | 10/10 gates 通过（含 client/host aggregate check、Client bundle、Web build） |
| `pnpm vitest run packages/client/ui-conversation` | 57 个文件、954 个测试全部通过 |
| 聚焦测试（history/keymap/machine） | 3 个文件、97 个测试通过 |
| `pnpm vitest run --config vitest.web.config.ts apps/web/tests/composer-input-history.e2e.ts` | 通过（真实 Chromium） |
| 相关 composer 浏览器场景（placeholder/draft-scroll/paste-routing/shortcuts） | 4 个文件通过 |
| `pnpm run test:docs` | 21/21 通过 |
| `pnpm run doc-sync` | 42 passed、1 failed；唯一失败 `verify-package-paths` 指向旧版冻结升级指南 `docs/upgrade-guide/v0.2.1-alpha.1/python-ptc-sandbox` 中本 fork 已移除的 `packages/experimental/ptc-runtime-python/README.md` 外部 URL，属基线既有失败，本次未改动冻结指南 |
| 改动文件 `oxlint` | 0 warnings / 0 errors |
| `pnpm exec tsc -p tsconfig.client.json --noEmit`、`tsc -p tsconfig.host.json --noEmit` | 通过 |
| `verify-empty-catch` | 无新增静默 catch |
| `git diff --check` | 通过 |
| `pnpm vitest run packages/client` | 13,732 passed、1 skipped；唯一失败为不相关的 document-preview license 测试并行偶发，单跑通过 |
| `pnpm run test:gui` | 13,978 passed；唯一失败为不相关的 `open-in-app` 环境差异（本机存在 `/usr/bin/xdg-open`） |
| 完整 `DSH_SNAPSHOT=replay pnpm run test:web` | 未完成：串行全量约 55 分钟后手动停止；新增场景与相关 composer 场景已单独通过。抽查的 `composer-focus` 失败为 `llm-replay` fixture 未完全消费，与方向键无关 |

## 4. 设计取舍

- **浏览器 localStorage，按精确 cwd 分桶**：与现有草稿持久化一致，不引入 Host Remote 或新的持久化格式；代价是不跨浏览器/设备同步，重命名或移动文件夹后从空历史开始。
- **只存纯文本**：引用 chip、附件、quote 结构化内容不进入历史；召回后引用表现为其 clipboard 文本，附件-only 发送不记录。终端历史同样只保存键入文本。
- **空草稿才启动**：避免 ↑ 覆盖用户正在写的长草稿；多行召回后仍保留方向键编辑能力。
- **成功结算才记录**：`commit-draft` 是「Host/命令面已接受、即将清空」的提交点；失败保留草稿且不写历史。
- **相邻重复去重**：连续发送同一内容只保留一条，与最新一条相同的提交不重复占用 100 条配额。
- **召回期间不 arm trigger 菜单**：否则 `/命令` 的斜杠菜单会抢走下一次 ↑，终端式连按失效。
- **失败隔离**：历史是便利功能，folder 解析或存储异常不得阻断召回回退或已接受草稿的清空。
- **文档归属**：实现契约在包 README，用户操作说明在 Web UI 指南；作为本地 UI 交互改动，不新增 Agent Note。

## 5. 复查记录（2026-11-14）

复查方式：逐条核对计划行为、用 jsdom 真实 Lexical 编辑器复现边界、用真实 Chromium 场景验证键盘链路，并审计 `commit-draft` 的所有生产路径。

发现并修复 3 个问题：

| # | 问题 | 修复 |
|---|---|---|
| 1 | 召回时 `historyExpected` 取自尚未刷新的旧 projection；浏览器中迟到的 Lexical update 会把召回误判为用户编辑并取消浏览 | 召回先安装 index/expected 再应用草稿；`setDraft` 拆出内部 `applyResolvedDraft`，召回不经过取消浏览的公共路径 |
| 2 | 抑制 trigger `track` 只覆盖 `setDraft` 的同步更新；迟到更新仍会为召回 `/命令` 打开斜杠菜单，抢走下一次 ↑ | 草稿与最近召回内容一致时不 arm trigger 菜单；用户编辑后恢复正常 track |
| 3 | history folder 解析或记录抛错可能影响召回，甚至阻断已接受草稿的清空 | 召回范围解析失败回退为普通方向键；`commit-draft` 的记录包在独立 try 中，记录失败仍清空草稿 |

复查补充测试：多行/行尾换行/空行/CRLF 精确召回、召回 `/compact` 后第二次 ↑ 仍导航、folder/record 抛错时召回与提交清空、Hub 真实 localStorage 记录。

复查后重新执行：`pnpm run build` 10/10、`pnpm vitest run packages/client/ui-conversation` 954 个测试、聚焦测试 97 个、新增 Chromium 场景、`pnpm run test:docs` 21/21、改动文件 oxlint、client/host 聚合类型检查、`git diff --check`，全部通过。

## 6. 已知限制与后续

- 历史只存在当前浏览器、按精确路径分桶；不跨设备同步，清除站点数据即清空，重命名或移动文件夹从空历史开始。
- 不回填功能上线前的会话输入；历史只记录功能生效后的成功提交。
- 不提供清空历史或历史面板 UI；需要时通过清除站点数据实现。
- 附件-only 发送、纯空白草稿不进入历史。
- Session catalog 暂无 `cwd` 时（例如某些子代理会话）该输入不启用历史。
- 完整 Web e2e 串行 lane 尚未跑完；`test:gui`、`packages/client` 全量各有 1 个与本次无关的环境/偶发失败，已在第 3 节记录。
- `verify-no-unknown-casts` 在本基线报告中点名未改动的 `packages/api/session-controller/tests/agent.host.spec.ts`；`verify-client-domain-graph` 报告的是既有 ui-workspace、ui-conversation skeleton 跨域导入，均与本次改动无关。

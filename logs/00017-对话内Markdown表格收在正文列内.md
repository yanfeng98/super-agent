# 00017 — 对话内 Markdown 表格收在正文列内

- **日期**：2026-11-15
- **背景**：用户贴出对话截图反馈「有的表格是比页面宽的」。根因是两条规则叠加：渲染器按列数分流——四列及以上加 `md-table-wide` 保持自然宽度，少于四列（以及引用块内的表格）用 `tableFill` 撑满正文列；而聊天把 `md-table-wide` 用容器查询突破到整个 transcript 宽度，表格自然宽度又只受单元格 `max-width: min(30vw, 320px)` 约束。于是宽表要么越过正文列伸向页面边缘，要么在自然宽度超过可用宽度时被 `overflow` 裁掉、只有悬停才出现滚动条。用户选定「表格始终与正文列同宽，超出就换行」。
- **范围**：`packages/client/ui-primitives`（表格几何的唯一 owner）、`packages/client/ui-chat`（取消宽表突破）、`apps/web/tests` 宽表 e2e 与金标、`packages/client/ui-primitives` 双语 README、Agent Note、`logs/`。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `packages/client/ui-primitives/src/markdown/MarkdownText.module.css` | `.tableScroll table` 由 `width: max-content; max-width: max-content` 改为 `width: fit-content; max-width: 100%`：表格先取自然宽度，超过所在容器时收敛到容器并在其中换行；只有各列 100px 可读下限之和超过容器时，才由 wrapper 横向滚动。新增注释说明 `fit-content` 与 `max-width` 是同一上限的两重表述。 |
| `packages/client/ui-chat/src/client/chat/AssistantMarkdown.module.css` | 删除宽表突破：`--dsh-table-spare`、`--dsh-table-lead`、`width`、`max-width: none`、负 `margin-left` 与 `padding-left`，以及只为突破遮挡宽度手柄而存在的 `.md-table-wide > table { position: relative; z-index: 1 }`。文件头补一句：transcript 不再加宽表格。 |
| `packages/client/ui-chat/src/client/chat/ChatView.module.css` | `.scroll` 的 `container-type: inline-size` 保留（权限预设、任务行、子代理行、交付物等无自建容器的组件仍以它为最近容器），只改写注释：它不再是表格突破的基准。 |
| `packages/client/ui-primitives/src/markdown/render.tsx` | `renderTable` 注释同步（不再声称宿主可加宽表格），`md-table-wide`/`tableFill` 类名与 `tabindex` 行为不变。 |
| `apps/web/tests/markdown-wide-table.e2e.ts` | 播种内容新增「四列长文本」表格（`MWT_PROSE_C1`）作为本次修复的回归锚点；多列表格由 12 列扩到 14 列，使滚动回退在 1680px 档仍有 >290px 余量；删除突破与 lead padding 断言，改为「所有表格内容与正文列左对齐 + 该换行的换行 + 多列仍滚动」；gutter 用例改为只断言 `elementFromPoint` 命中手柄（多列表未被裁剪的矩形本就跨过手柄，矩形判定不成立）。 |
| `apps/web/tests/expected/markdown-wide-table/geometry.expected.md` | 金标重录：列改为 `viewport / table / fills the column / scrolls`，prose 行在 1680/1100/640 三档均为 `fills=true`，wrap-first 段纳入 prose。 |
| `packages/client/ui-primitives/README.md` / `.zh.md` / `.i18n.yaml` | `MarkdownText` 条目补表格排版契约：按容器宽度排版、100px 可读下限、四列及以上不拉伸、列下限装不下时在 wrapper 内滚动且滚动条悬停或键盘聚焦才显示；重新记录双语哈希。 |
| `.agents/notes/implemented/bug-fix/2026-11-15-markdown-tables-fit-the-column.md` / `.zh.md` / `.i18n.yaml` | 新增 Agent Note，记录撤销「宽表突破」这一既有决定的理由、备选方案与代价，并交叉链接被取代的归档笔记 `archived/feature/2026-08-19-web-markdown-wide-table-view.md`。 |

## 2. 行为

| 场景 | 修复前 | 修复后 |
|---|---|---|
| 四列长文本表格（装配页 1100px，合成语料） | 宽 1066px、正文列 680px：越出 386px，其中 240px 被裁掉 | 宽 680px，等于正文列，残余滚动 0 |
| 同一张表（1680px） | 宽 1066px、正文列 920px：越过正文列 146px 伸向 transcript 边缘 | 宽 920px，等于正文列 |
| 用户截图那条消息的 3 张表（1026px，真实内容） | 4 列表格越过正文列 | 三张表均 680px = 正文列，残余滚动 0，页面级横向溢出 0 |
| 同一消息（1300px / 1680px） | — | 三张表分别 796px / 920px，均等于正文列，页面级溢出 0 |
| 14 列表格 | 自然宽度，靠 hover 揭示横向滚动 | 不变：仍按最小列宽横向滚动（1680px 下残余 912px），hover 揭示与键盘聚焦路径保留 |
| 2–3 列表格与引用块内表格 | `tableFill` 撑满正文列 | 不变 |
| 思考行（compact 变体）、工具卡片、插件文档、侧栏文档预览 | 宽表按自然宽度被裁或溢出 | 一律收进各自容器并换行 |

**可访问性影响**：`md-table-wide` 仍带 `tabindex="0"` 与焦点环，滚动型表格的键盘滚动路径不变；能放下的宽表依旧在 Tab 序列中，这是列数静态判定的既有行为。

**布局连带影响**：表格不再越过正文列后，Conversation 两侧的宽度手柄不再被表格内容覆盖；e2e 的 gutter 用例因此从「表格覆盖手柄」反转为「手柄可命中」。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `pnpm run build` | 10 个构建门禁全部通过；构建产物中该规则为 `width:fit-content;max-width:100%`，且不再含 `--dsh-table-spare`/`--dsh-table-lead` |
| `pnpm run build:check` | host/client 两个聚合检查通过（改动后重跑两次） |
| `pnpm run test:gui` | 783/784 文件通过、13983 项通过；唯一失败为 `packages/host/open-in-app/tests/resolver.spec.ts`，在干净树上同样失败（本机 WSLg 带 `WAYLAND_DISPLAY`/`DISPLAY`），与本次无关 |
| `pnpm run lint` | 7251 个文件、0 warning、0 error |
| `pnpm run test:docs` | 21 个门禁全部通过（新增 Agent Note、README 与日志后各跑一次） |
| `pnpm run doc-sync` | 43 个门禁全部通过；首次运行有 1 个失败（`verify-package-paths`），根因是本机残留的未跟踪目录 `packages/experimental/ptc-runtime-python`（该包已从仓库删除，只剩 `lib/` 与 `node_modules/`），被门禁当成存活包叶从而把历史升级指南里一条指向上游 tag 的绝对 URL 判为漂移；删除该残留后 43/43 |
| 宽表 e2e（`DSH_SNAPSHOT=refresh` 后 `replay`） | 11/12 通过；唯一失败是 WebKit 触摸分支（宿主机缺 `libgtk-4.so.1` 等运行库），Chromium 分支与金标通过，金标重录后回放稳定 |
| `thinking-markdown.e2e.ts`（思考行 6 列表格） | 通过：6 列表仍需内部滚动，`wideContentBounded` 等断言不变 |
| `math-rendering.e2e.ts`、`markdown-cjk-strong.e2e.ts` | 3 项全部通过 |
| `window-drag-coverage.e2e.ts` | 17/18 通过；唯一失败（Plugins 按钮 30s 超时）单独复跑通过，且在干净树上同样只在完整用例顺序中出现，与本改动无关；含 `.md-table-wide` 的「滚到 chrome 之后」用例通过 |
| 装配页实测（真实消息，1026 / 1300 / 1680） | 三张表均等于正文列宽、残余滚动 0、页面级横向溢出 0 |
| 装配页实测（390px 响应式下限） | 页面级横向溢出 0；3–4 列表格因 100px 下限在自身 wrapper 内滚动（与 `tableFill` 既有行为一致） |
| 悬停 chrome 实测（能放下的宽表） | 前后高度均 91px（`overflow-x: hidden→scroll`、`padding-bottom: 5px→0`），截图确认不绘制空滚动条 |
| 线上服务核对 | 抓取 `127.0.0.1:3080` 正在提供的 shell 资源，与其 `apps/web/dist` 同名文件 SHA-256 一致、`cache-control: no-cache`，且含新规则；表格 CSS 属静态资源，刷新即生效 |
| 三个 CSS 写法对比（Chromium） | `max-content+max-width`、`fit-content`、两者并列在四种表格形态下几何完全一致（表格宽与每列宽像素级相同） |
| `git diff --check` | 通过 |

## 4. 设计取舍

- **取消突破而非「突破 + 封顶」**：保留突破时表格仍会单侧凸出正文列（截图里的 1950px vs 1845px），这正是用户指出的错位；统一到正文列后，表格与段落、标题、输入框共用同一条宽度轴。
- **上限用两个关键字表述**：`width: fit-content` 从宽度关键字侧给出「不超过可用宽度、装不下退到列下限」，`max-width: 100%` 从最大值侧给出同一边界；两者在 Chromium 下几何完全一致，宿主机无法运行 WebKit 时这是唯一可做的跨引擎加固。
- **不做测量**：不给每张表格加 `ResizeObserver` 或展开交互，宽度只由静态列数与容器决定，流式结算、虚拟化与重挂载都不需要恢复状态。
- **保留 100px 可读下限**：不为窄视口把列压到不可读；窄屏与极端多列继续在表格内部滚动，页面本身不横滚。
- **保留 `md-table-wide` 与 `.scroll` 的 `container-type`**：前者仍决定 hover 揭示滚动条与键盘聚焦，后者仍是其它组件无名 `@container` 的最近容器；只删掉确实只为突破存在的 `z-index` 规则。
- **文档落点**：包契约补进 `ui-primitives` 双语 README（消费者可见行为），决定与代价写进 Agent Note，实现与复查过程写进本日志。

## 5. 复查记录（2026-11-15）

复查方式：先按「修复是否正确」做两轮自查——第一轮在 Chromium 里对四种表格形态比对三种 CSS 写法，并扫描全仓 `md-table-wide` 消费者与所有 ≥4 列 e2e 语料；第二轮从 `~/.dsh/sessions` 取出用户截图那条消息的原文（3 张表：4 列×3 行、2 列×7 行、2 列×9 行），在装配页按 1026/1300/1680 三档实测，再逐字节核对线上服务提供的资源。

两轮共发现并修复 3 处问题：

| # | 问题 | 修复 |
|---|---|---|
| 1 | 上限只由 `max-width: 100%` 一个机制承担，宿主机无法运行 WebKit，缺少跨引擎冗余 | 改为 `width: fit-content; max-width: 100%`，先在 Chromium 证明三种写法几何一致，再更新 Agent Note 与 README |
| 2 | 改写后的 gutter 用例用「表格矩形是否跨过手柄」判定，但未被裁剪的多列表格矩形本来就跨过手柄，断言与真实几何不符 | 只断言 `elementFromPoint` 的命中结果（`hitTable=false`、`hitHandle=true`），两种表格都验证 |
| 3 | e2e 里 `SENTENCE` 的注释仍写「748px column」，与正文列可变（680/796/920）不符 | 改为「message column」 |

复核结论：

- 真实消息的三张表在三档视口下都等于正文列宽、无残余滚动，页面级横向溢出为 0；用户截图里被截断的 `修复` 列完整可读。
- 线上 `dsh web` 提供的 shell 资源与重建产物字节一致，浏览器刷新即可看到修复，不需要重启进程（与「模型提供商列表需要重启」不同，那是 host 侧启动期状态）。
- 能放下的宽表悬停时不会画出空滚动条：`overflow-x: scroll` 预留的 5px 被 `padding-bottom` 的 5px 抵消，高度恒定 91px。
- 390px 下 2 列长单元格表格仍无滚动，3–4 列表格在 wrapper 内滚动；这是 100px 下限的既有结论，不是本次引入。
- 唯一不可达的是 WebKit 几何：宿主机缺 GTK4/GStreamer 运行库且无 root，`playwright install webkit` 只完成下载、无法启动；Chromium 覆盖金标，WebKit 触摸分支由 CI 覆盖。
- 过程中为确认 `open-in-app` 的既有失败是否与本次相关，曾在工作树上做过一次 `git stash`/`pop`；期间发现同一工作树还有另一并发任务在改 `llm-pi-ai` 与相关文档，那批改动被原样恢复、未做任何修改，`git stash list` 为空。

## 6. 已知限制与后续

- `127.0.0.1:3080` 只需刷新即可看到修复（资源为 `no-cache` 且文件名哈希已更新）；若浏览器复用了旧 index，用强制刷新。
- 本机曾残留 `packages/experimental/ptc-runtime-python`（已删除包的未跟踪构建产物与 `node_modules`），会让 `verify-package-paths` 把历史指南里指向上游 tag 的绝对 URL 误判为漂移；该目录已删除，全新 clone 不会出现，`pnpm run clean` 也会清掉同类残渣。
- WebKit 下的表格几何未在本机验证；接续验证方式是在装有 WebKit 运行库的机器上给 `markdown-wide-table.e2e.ts` 增加一个 WebKit 视口扫描。
- ≤390px 时 3–4 列表格仍在表格内部横向滚动；若要在手机上让四列完全放下，需要下调 100px 可读下限或减小单元格内边距，属于独立的移动端密度决定。
- 若将来要恢复「宽表用满页面」，需要重新引入突破规则、恢复 e2e 的突破断言与金标列，并新增一条取代本决定的 Agent Note。

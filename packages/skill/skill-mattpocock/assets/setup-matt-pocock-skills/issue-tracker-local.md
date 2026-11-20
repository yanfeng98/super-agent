# Issue tracker: Local Markdown

这个仓库的 issue 和 spec 以 markdown 文件的形式存放在 `.scratch/` 里。

## Conventions

- 一个特性一个目录：`.scratch/<feature-slug>/`
- spec 是 `.scratch/<feature-slug>/spec.md`
- 实现用的 issue 一个工单一个文件，放在 `.scratch/<feature-slug>/issues/<NN>-<slug>.md`，从 `01` 开始编号，绝不是一个把所有工单合在一起的文件
- 分诊状态记录为每个 issue 文件靠近顶部的 `Status:` 行（角色字符串见 `triage-labels.md`）
- 评论与对话历史追加到文件底部，写在 `## Comments` 标题之下

## When a skill says "publish to the issue tracker"

在 `.scratch/<feature-slug>/` 下新建一个文件（需要时先把这个目录建出来）。

## When a skill says "fetch the relevant ticket"

读被引用路径上的那个文件。用户通常会直接给出路径或 issue 编号。

## Wayfinding operations

供 `/wayfinder` 使用。**地图**是一个文件，每张工单对应一个**子**文件。

- **地图**：`.scratch/<effort>/map.md`（Notes / Decisions-so-far / Fog 正文）。
- **子工单**：`.scratch/<effort>/issues/NN-<slug>.md`，从 `01` 开始编号，正文里写问题。一行 `Type:` 记录工单类型（`research`/`prototype`/`grilling`/`task`）；一行 `Status:` 记录 `claimed`/`resolved`。
- **阻塞**：靠近顶部的一行 `Blocked by: NN, NN`。当它列出的每个文件都是 `resolved` 时，这张工单就是未阻塞的。
- **前沿（frontier）**：扫描 `.scratch/<effort>/issues/`，找出处于开启、未阻塞、未被认领状态的文件；编号最小的胜出。
- **认领**：在做任何工作之前设置 `Status: claimed` 并保存。
- **解决**：把答案追加到 `## Answer` 标题之下，设置 `Status: resolved`，然后往 `map.md` 里地图的 Decisions-so-far 追加一条上下文指针（context pointer，要点 + 链接）。

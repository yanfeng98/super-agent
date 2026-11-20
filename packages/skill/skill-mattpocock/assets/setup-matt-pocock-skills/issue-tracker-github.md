# Issue tracker: GitHub

这个仓库的 issue 和 spec 以 GitHub issue 的形式存在。所有操作都用 `gh` CLI。

## Conventions

- **创建 issue**：`gh issue create --title "..." --body "..."`。多行正文用 heredoc。
- **读 issue**：`gh issue view <number> --json number,title,body,labels,comments`。
- **列出 issue**：`gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'`，配上合适的 `--label` 和 `--state` 过滤。
- **把 issue 做成父 issue 的子 issue**：`gh issue create --parent <parent> ...`，或者事后运行 `gh issue edit <parent> --add-sub-issue <child>`（`gh` 2.94+）。更老的 `gh`：`gh api --method POST repos/<owner>/<repo>/issues/<parent>/sub_issues -F sub_issue_id=<child-db-id>`（数据库 id，见下面的**阻塞**）。没有 sub-issues 时，在子 issue 正文顶部写 `Part of #<parent>`。
- **阻塞**：GitHub 的**原生 issue 依赖关系**，这也是规范的、在界面上看得见的表达方式。用 `gh issue create --blocked-by <n>,<n> ...` 或 `gh issue edit <child> --add-blocked-by <blocker>` 加一条边（`gh` 2.94+）。更老的 `gh`：`gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`，其中 `<blocker-db-id>` 是阻塞者的数字**数据库 id**（`gh api repos/<owner>/<repo>/issues/<n> --jq .id`，_不是_ `#number` 或 `node_id`）。依赖关系不可用时，退回到子 issue 正文顶部的一行 `Blocked by: #<n>, #<n>`。当每一个阻塞者都已关闭时，这张工单就是未阻塞的。
- **数未关闭的阻塞者**：`gh api repos/<owner>/<repo>/issues/<n> --jq .issue_dependencies_summary.blocked_by`（只算未关闭的阻塞者；0 表示可以开始）。`gh issue view <n> --json blockedBy` 也会列出已关闭的阻塞者，所以要过滤一下：`--jq '[.blockedBy.nodes[] | select(.state == "OPEN")] | length'`。
- **给 issue 评论**：`gh issue comment <number> --body "..."`
- **打标签 / 去掉标签**：`gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **关闭**：先用 `gh issue comment <number> --body "..."` 发出说明，再 `gh issue close <number>`。对于已经关闭的 issue，`gh issue close --comment` 会丢掉评论，而且仍然以 0 退出。

从 `git remote -v` 推断仓库；在克隆目录里运行时，`gh` 会自动这么做。

只把 issue 提交到用户自己的仓库里。除非用户拥有它，否则绝不提交到 `mattpocock/skills`（这些技能的来源仓库）。在 fork 里，`gh` 可能默认指向上游父仓库，所以每个会话第一次 `gh issue create` 之前，先用 `gh repo view --json nameWithOwner` 检查目标；如果不是用户的仓库，就和用户确认。

## Pull requests as a triage surface

**PRs as a request surface: no.** _（如果这个仓库把外部 PR 当作功能请求，就设为 `yes`；`/triage` 会读这个标志。）_

设为 `yes` 时，PR 走与 issue 相同的标签和状态，只是换成 `gh pr` 的对应命令：

- **读 PR**：`gh pr view <number>`，然后 `gh pr view <number> --comments`（不在 TTY 上时，`--comments` 只打印评论，不打印正文），再用 `gh pr diff <number>` 看 diff。
- **列出待分诊的外部 PR**：`gh api --paginate 'repos/{owner}/{repo}/pulls?state=open' --jq '.[] | select(.author_association | IN("OWNER","MEMBER","COLLABORATOR") | not) | {number, title, author: .user.login, author_association, labels: [.labels[].name]}'`。
- **评论 / 打标签 / 关闭**：`gh pr comment`、`gh pr edit --add-label`/`--remove-label`、`gh pr close`。

GitHub 的 issue 和 PR 共用一个编号空间，所以光秃秃的 `#42` 可能是两者之一：先用 `gh pr view 42` 解析，失败再退回到 `gh issue view 42`。

## When a skill says "publish to the issue tracker"

创建一个 GitHub issue。

## When a skill says "fetch the relevant ticket"

按上面**读 issue** 一节读取它。

## Wayfinding operations

供 `/wayfinder` 使用。**地图**是一个 issue，**子** issue 作为工单。

- **地图**：一个打了 `wayfinder:map` 标签的 issue，承载 Notes / Decisions-so-far / Fog 正文。`gh issue create --label wayfinder:map`。
- **子工单**：作为 GitHub sub-issue 链接到地图上的一个 issue（见**把 issue 做成父 issue 的子 issue**）。sub-issue 没启用时，把子工单加进地图正文里的任务清单，并在子工单正文顶部写上 `Part of #<map>`。标签：`wayfinder:<type>`（`research`/`prototype`/`grilling`/`task`）。一旦被认领，这张工单就指派给驱动这张地图的开发者。
- **阻塞**：见 Conventions 下的**阻塞**。
- **前沿查询（frontier query）**：列出地图未关闭的子工单（`gh issue list --state open`，范围限定在地图的 sub-issue / 任务清单上），丢掉任何有未关闭阻塞者（见**数未关闭的阻塞者**，或者 `Blocked by` 行里有一个未关闭的 issue）或者有指派人的；地图顺序里最靠前的胜出。
- **认领**：`gh issue edit <n> --add-assignee @me`，这是本会话的第一次写入。
- **解决**：`gh issue comment <n> --body "<answer>"`，然后 `gh issue close <n>`，再往地图的 Decisions-so-far 追加一条上下文指针（context pointer，要点 + 链接）。

# Issue tracker: GitLab

这个仓库的 issue 和 spec 以 GitLab issue 的形式存在。所有操作都用 [`glab`](https://gitlab.com/gitlab-org/cli) CLI。

## Conventions

- **创建 issue**：`glab issue create --title "..." --description "..."`。多行描述用 heredoc。传 `--description -` 可以打开编辑器。
- **读 issue**：`glab issue view <number> --comments`。要机器可读的输出就用 `-F json`。
- **列出 issue**：`glab issue list -O json`，配上合适的 `--label` 过滤。
- **给 issue 评论**：`glab issue note <number> --message "..."`。GitLab 把评论叫作 "notes"。
- **打标签 / 去掉标签**：`glab issue update <number> --label "..."` / `--unlabel "..."`。多个标签可以用逗号分隔，也可以重复这个 flag。
- **关闭**：`glab issue close <number>`。`glab issue close` 不接受关闭时的评论，所以先用 `glab issue note <number> --message "..."` 把解释发出去，再关闭。
- **合并请求**：GitLab 把 PR 叫作 "merge requests"。用 `glab mr create`、`glab mr view`、`glab mr note` 等，形状与 `gh pr ...` 相同，只是把 `pr` 换成 `mr`，把 `comment`/`--body` 换成 `note`/`--message`。

从 `git remote -v` 推断仓库；在克隆目录里运行时，`glab` 会自动这么做。

## Merge requests as a triage surface

**MRs as a request surface: no.** _（如果这个仓库把外部合并请求当作功能请求，就设为 `yes`；`/triage` 会读这个标志。）_

设为 `yes` 时，MR 走与 issue 相同的标签和状态，只是换成 `glab mr` 的对应命令：

- **读 MR**：`glab mr view <number> --comments`，以及用 `glab mr diff <number>` 看 diff。
- **列出待分诊的外部 MR**：`glab mr list -O json`，然后只保留作者不是项目成员/所有者的 MR（贡献者提交的 MR，而不是维护者手上正在做的活）。
- **评论 / 打标签 / 关闭**：`glab mr note`、`glab mr update --label`/`--unlabel`、`glab mr close`。

与 GitHub 不同，GitLab 给 issue 和 MR 分开编号，所以只要知道维护者指的是哪个界面，`#42` 就是明确的。

## When a skill says "publish to the issue tracker"

创建一个 GitLab issue。

## When a skill says "fetch the relevant ticket"

运行 `glab issue view <number> --comments`。

## Wayfinding operations

供 `/wayfinder` 使用。**地图**是一个 issue，**子** issue 作为工单。

- **地图**：一个打了 `wayfinder:map` 标签的 issue，承载 Notes / Decisions-so-far / Fog 正文。`glab issue create --label wayfinder:map`。（在有原生 epic 的 GitLab 套餐上，可以由一个 epic 承载地图；而打了标签的 issue 到处都能用。）
- **子工单**：一个在描述顶部写着 `Part of #<map>`、链接到地图（`glab issue create --linked-issues <map> --link-type relates_to`）、并带 `wayfinder:<type>` 标签（`research`/`prototype`/`grilling`/`task`）的 issue。一旦被认领，这张工单就指派给驱动这张地图的开发者。
- **阻塞**：GitLab 的**原生阻塞链接**，这也是规范的、在界面上看得见的表达方式。用 `/blocked_by #<n>` 快速操作来加，以 note 的形式发出去（`glab issue note <child> --message "/blocked_by #<blocker>"`）。原生阻塞链接是 Premium/Ultimate 功能；在免费套餐上（或者不可用时）退回到描述顶部的一行 `Blocked by: #<n>, #<n>`。当每一个阻塞者都已关闭时，这张工单就是未阻塞的。
- **前沿查询（frontier query）**：用 `glab api projects/:id/issues/<map>/links` 列出地图的子工单，保留带 `wayfinder:<type>` 标签的未关闭 issue；丢掉任何有未关闭阻塞者的：指向未关闭 issue 的原生 `blocked_by` 链接（`glab api projects/:id/issues/<child-iid>/links`）、`Blocked by` 行里有一个未关闭的 issue，或者有指派人；地图顺序里最靠前的胜出。
- **认领**：`glab issue update <n> --assignee @me`，这是本会话的第一次写入。
- **解决**：`glab issue note <n> --message "<answer>"`，然后 `glab issue close <n>`，再往地图的 Decisions-so-far 追加一条上下文指针（context pointer，要点 + 链接）。

---
name: setup-matt-pocock-skills
description: "为工程类技能配置这个仓库：设置它的问题跟踪器、分诊标签词汇和领域文档版式。在第一次使用其他工程技能之前运行一次。"
disable-model-invocation: true
---

# 配置 Matt Pocock 的技能

把工程类技能所假定的那套逐仓库配置搭起来：

- **问题跟踪器**：issue 住在哪里（默认 GitHub；本地 markdown 也开箱即用）
- **分诊标签**：五个规范分诊角色所用的标签字符串
- **领域文档**：`GLOSSARY.md` 和 ADR 住在哪里，以及读取它们的消费方规则

这是一个由提示词驱动的技能，不是一个确定性脚本。先探索，把你发现的东西摆出来，与用户确认，然后再写。

## 流程

### 1. 探索

看一下当前仓库，弄清它的起始状态。有什么就读什么，不要凭假设：

- `git remote -v` 和 `.git/config`：这是一个 GitHub 仓库吗？是哪一个？
- 仓库根目录下的 `AGENTS.md` 和 `CLAUDE.md`：两者中有任何一个存在吗？其中是否已经有 `## Agent skills` 一节？
- 仓库根目录下的 `GLOSSARY.md` 和 `GLOSSARY-MAP.md`，以及旧版的 `CONTEXT.md` / `CONTEXT-MAP.md`。如果存在旧版文件，提出用 `git mv` 把它改成 `GLOSSARY` 名字，并改写 `docs/agents/domain.md` 里的这些名字。
- `docs/adr/`，以及任何 `src/*/docs/adr/` 目录
- `docs/agents/`：这个技能上次的产出是否已经存在？
- `.scratch/`：这是本地 markdown 问题跟踪器约定已经在用的迹象
- `triage` 技能装了吗？（与这个技能并排的一个 `triage` 技能文件夹，或者你的可用技能里就有 `triage`。）这决定 B 节到底跑不跑。
- monorepo 的信号：一个 `pnpm-workspace.yaml`，`package.json` 里的 `workspaces` 字段，或者一个内容充实的 `packages/*` 且它自带 `src/`。只有真正大型的多包仓库里才有这些；没有它们就意味着单上下文（single-context），而这几乎是每一个仓库。

### 2. 摆出发现并提问

把已有的和缺的归纳一下。然后按顺序逐个过这些小节。一节，一个答复，再下一节。

每一节都先给出推荐的答案，用户一个字就能接受它。只有在这个选择真的分叉时才给一行解释；探索已经定了的，整节跳过（`triage` 没装时跳过 B 节，没有 monorepo 时跳过 C 节）。

**A 节：问题跟踪器。**

> 解释：「问题跟踪器」是这个仓库的 issue 住在哪里。`to-tickets`、`triage`、`to-spec` 这类技能会读写它。它们需要知道该调用 `gh issue create`、在 `.scratch/` 下写一个 markdown 文件，还是走你描述的其他某套工作流。挑你实际用来跟踪这个仓库工作的地方。

默认立场：这些技能是为 GitHub 设计的。如果某个 `git remote` 指向 GitHub，就提议它。如果某个 `git remote` 指向 GitLab（`gitlab.com` 或自建的 host），就提议 GitLab。否则（或者用户更想要别的），给出这些选项：

- **GitHub**：issue 住在这个仓库的 GitHub Issues 里（使用 `gh` CLI）
- **GitLab**：issue 住在这个仓库的 GitLab Issues 里（使用 [`glab`](https://gitlab.com/gitlab-org/cli) CLI）
- **本地 markdown**：issue 以文件形式住在这个仓库的 `.scratch/<feature>/` 下（适合单人项目或没有远程的仓库）
- **其他**（Jira、Linear 等）：让用户用一段话描述这套工作流；技能会把它记成自由格式的散文

把选择记录在 `docs/agents/issue-tracker.md` 里。GitHub 和 GitLab 模板带一个 "PRs as a request surface" 标志，默认**关闭**。让它关着，也不要提起它：想让外部 PR 进入分诊队列的用户，之后可以自己在文件里把它打开。

GitHub 或 GitLab 跟踪器需要一个可达的仓库。当用户选了其中之一，而没有任何 `git remote` 指向该主机时，向他们要仓库 URL 并接好（需要的话先 `git init`，再 `git remote add origin <url>`），然后再往下走。完成的判据是：在这个目录里运行 `gh repo view` / `glab repo view` 能打印出这个项目；如果打印不出来，就停下来，告诉用户缺什么。

**B 节：分诊标签词汇。**如果 `triage` 技能没装，就整节跳过（探索已经告诉你了），因为没装的技能不需要标签。

如果装了，就只问这一个问题：

> 你想保留默认的分诊标签吗？（推荐：**是**）

默认值就是那五个规范角色，每个标签字符串都与它的名字相同：`needs-triage`、`needs-info`、`ready-for-agent`、`ready-for-human`、`wontfix`。答**是**时，就照原样写下去。只有用户说不的时候（通常是因为他们的跟踪器已经在用别的名字，例如 `bug:triage` 对应 `needs-triage`），才收集这些覆盖项，好让 `triage` 应用已有的标签，而不是创建重复的标签。

**C 节：领域文档。**默认用**单上下文**（仓库根目录一个 `GLOSSARY.md` 加 `docs/adr/`）。这适合几乎每一个仓库；不用问，直接写。

只有当探索发现了 monorepo 信号时，才给出**多上下文**这个选项（根目录一个 `GLOSSARY-MAP.md`，指向每个上下文各自的 `GLOSSARY.md` 文件）。然后确认他们想要哪种版式。

### 3. 确认并编辑

给用户看这几样东西的草稿：

- 要加进 `CLAUDE.md` / `AGENTS.md` 中正在被编辑那一个里的 `## Agent skills` 块（选择规则见第 4 步）
- `docs/agents/issue-tracker.md`、`docs/agents/domain.md` 和 `docs/agents/triage-labels.md` 的内容（最后那个只在 `triage` 装了时才写）

写之前让他们先改。

### 4. 写入

**挑要编辑的文件：**

- 如果 `CLAUDE.md` 存在，就编辑它。
- 否则如果 `AGENTS.md` 存在，就编辑它。
- 如果两个都不存在，问用户要创建哪一个；不要替他挑。

`CLAUDE.md` 已经存在时，绝不创建 `AGENTS.md`（反过来也一样）；始终编辑已经在那里的那一个。

如果选中的文件里已经有 `## Agent skills` 块，就地更新它的内容，而不是再追加一个重复的。不要覆盖用户对周边各节的编辑。

这个块：

```markdown
## Agent skills

### Issue tracker

[issue 记在哪里的单行摘要]。见 `docs/agents/issue-tracker.md`。

### Triage labels

[标签词汇的单行摘要]。见 `docs/agents/triage-labels.md`。

### Domain docs

[版式的单行摘要："single-context" 或 "multi-context"]。见 `docs/agents/domain.md`。
```

只有当 `triage` 装了并且 B 节跑过时，才包含 `### Triage labels` 子块，并写出 `docs/agents/triage-labels.md`。没装时，两者都省略。

当 B 节在 GitHub 或 GitLab 上跑过时，为跟踪器缺少的每个已配置标签执行创建（`gh label create` / `glab label create`）。

然后用这个技能文件夹里的种子模板作起点，写出那些文档文件：

- [issue-tracker-github.md](./issue-tracker-github.md)：GitHub 问题跟踪器
- [issue-tracker-gitlab.md](./issue-tracker-gitlab.md)：GitLab 问题跟踪器
- [issue-tracker-local.md](./issue-tracker-local.md)：本地 markdown 问题跟踪器
- [triage-labels.md](./triage-labels.md)：标签映射（只在 `triage` 装了时才写）
- [domain.md](./domain.md)：领域文档的消费方规则 + 版式

对于 "other" 类问题跟踪器，用用户给的描述从零写出 `docs/agents/issue-tracker.md`。

### 5. 完成

告诉用户配置完成了，以及哪些工程类技能现在会读这些文件。提一句：之后他们可以直接编辑 `docs/agents/*.md`；只有想换问题跟踪器或者从头重来时，才需要重新跑这个技能。

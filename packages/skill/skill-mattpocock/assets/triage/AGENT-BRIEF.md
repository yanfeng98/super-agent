# 怎么写 agent 简报

agent 简报是一条结构化评论，在某条 GitHub issue 或 PR 移到 `ready-for-agent` 时贴上去。它是 AFK 的 agent 据以工作的权威 spec。原始正文与讨论是上下文：agent 简报才是契约。

简报写明 **agent 该做什么**，这在 issue 与 PR 两种场合都适用：对 issue，是从零把改动做出来；对 PR，是*在既有 diff 上*还剩什么要做：把它做完、补上缺口、回应评审意见。两边原则相同；下面的 PR 示例展示了其中的差别。

## 原则

### 耐用优先于精确

这条 issue 可能在 `ready-for-agent` 上待几天或几周。期间代码库会变。写简报时，要让它在文件被改名、移动或重构的过程中依然有用。

- **要**描述接口、类型与行为契约
- **要**点名 agent 应当去找或去改的具体类型、函数签名或配置形状
- **不要**引用文件路径：它们会过期
- **不要**引用行号
- **不要**假定当前的实现结构会保持不变

### 讲行为，不讲步骤

描述系统**该做什么**，而不是**怎么**实现。agent 会重新探索代码库，自己做实现决策。

- **好：**「`SkillConfig` 类型应当接受一个可选的 `schedule` 字段，类型为 `CronExpression`」
- **坏：**「打开 src/types/skill.ts，在第 42 行加一个 schedule 字段」
- **好：**「用户不带参数运行 `/triage` 时，应当看到一份需要注意的 issue 概要」
- **坏：**「在主处理函数里加一个 switch 语句」

### 完整的验收标准

agent 需要知道什么时候算做完。每份 agent 简报都必须有具体、可测的验收标准（acceptance criteria）。每条标准都应当可以独立验证。

- **好：**「运行 `gh issue list --label needs-triage` 返回的，是已经过初次归类的 issue」
- **坏：**「分诊应当正常工作」

### 明确的范围边界

写明什么在范围之外（out of scope）。这能防止 agent 镀金，或者对相邻功能自作假定。

## 模板

```markdown
## Agent Brief

**Category:** bug / enhancement
**Summary:** 用一行说明需要发生什么

**Current behavior:**
描述现在会发生什么。对 bug，这就是坏掉的行为。
对 enhancement，这是该功能所基于的现状。

**Desired behavior:**
描述 agent 的工作完成之后应当发生什么。
边界情况与错误条件要写具体。

**Key interfaces:**
- `TypeName`：要改什么、为什么改
- `functionName()` 的返回类型：它现在返回什么，应当返回什么
- 配置形状：需要的任何新配置项

**Acceptance criteria:**
- [ ] 具体、可测的标准 1
- [ ] 具体、可测的标准 2
- [ ] 具体、可测的标准 3

**Out of scope:**
- 本 issue 里**不**该改动或处理的东西
- 相邻的功能，看起来可能相关，但其实是独立的
```

## 示例

### 好的 agent 简报（bug）

```markdown
## Agent Brief

**Category:** bug
**Summary:** 技能描述截断会从词中间切断，产出坏掉的输出

**Current behavior:**
当一条技能描述超过 1024 个字符，它会正好在
1024 个字符处截断，完全不管词边界。这会产出在词中间结束的描述
（例如 "Use when the user wants to confi"）。

**Desired behavior:**
截断应当在 1024 个字符之前的最后一个词边界处断开，
并追加 "..." 表示已截断。

**Key interfaces:**
- `SkillMetadata` 类型的 `description` 字段：类型不用改，
  但填充它的校验与处理逻辑需要
  尊重词边界
- 任何读取 SKILL.md frontmatter 并取出 description 的函数

**Acceptance criteria:**
- [ ] 不到 1024 字符的描述保持不变
- [ ] 超过 1024 字符的描述会在最后一个词边界处截断
      也就是 1024 字符之前的那个词边界
- [ ] 被截断的描述以 "..." 结尾
- [ ] 含 "..." 在内的总长度不超过 1024 字符

**Out of scope:**
- 改动 1024 字符这个上限本身
- 支持多行描述
```

### 好的 agent 简报（enhancement）

```markdown
## Agent Brief

**Category:** enhancement
**Summary:** 加上对 `.out-of-scope/` 目录的支持，用来记录被否掉的功能请求

**Current behavior:**
功能请求被否掉时，issue 会带上一个 `wontfix` 标签被关闭，
并附一条评论。这个决定和它的理由没有任何长期记录。
以后再有类似的请求，就得靠维护者自己回忆或翻找
之前的那次讨论。

**Desired behavior:**
被否掉的功能请求应当记进 `.out-of-scope/<concept>.md`
文件里，这些文件要留住决定、理由，以及所有提出过
该功能的 issue 的链接。分诊新 issue 时，应当拿这些文件
去比对是否有匹配。

**Key interfaces:**
- `.out-of-scope/` 里的 Markdown 文件格式：每个文件应当有一个
  `# Concept Name` 标题、一行 `**Decision:**`、一行 `**Reason:**`，
  以及一份带 issue 链接的 `**Prior requests:**` 列表
- 分诊流程应当尽早读取全部 `.out-of-scope/*.md` 文件，
  并按概念相似度把进来的 issue 与它们比对

**Acceptance criteria:**
- [ ] 以 wontfix 关闭一个功能时，会创建/更新 `.out-of-scope/` 里的文件
- [ ] 该文件包含决定、理由，以及被关闭 issue 的链接
- [ ] 如果已有匹配的 `.out-of-scope/` 文件，新 issue 会被
      追加到它的 "Prior requests" 列表，而不是另建一份重复的文件
- [ ] 分诊期间会检查 `.out-of-scope/` 里已有的文件，并在
      新 issue 与先前某次否决匹配时把它摆出来

**Out of scope:**
- 自动匹配（由人来确认匹配）
- 重新打开先前被否掉的功能
- bug 报告（只有 enhancement 被否掉时才写进 `.out-of-scope/`）
```

### 好的 agent 简报（PR）

对 PR 来说，「Current behavior」描述的是这个 diff 的状态，简报要求 agent 把它做完或修好，而不是从零开始。

```markdown
## Agent Brief

**Category:** enhancement
**Summary:** 把贡献者那份给 `triage list` 用的 `--json` 输出标志做完

**Current behavior:**
这个 PR 加了一个 `--json` 标志，把 issue 列表序列化成 JSON。正常
路径能跑通，diff 也符合项目的命令结构。还差两处：
错误仍然按人读的文本打印（而不是 JSON），而且这个新标志没有
测试覆盖。

**Desired behavior:**
带上 `--json` 时，所有输出（包括错误）都是 stdout 上格式良好的 JSON，
命令的退出码保持不变。现有的人读输出
在不带这个标志时原样不动。

**Key interfaces:**
- 命令的错误路径在 `--json` 下应当输出 `{ "error": string }`，
  而不是纯文本错误
- 复用 PR 已经加的那个序列化器；不要引入第二个

**Acceptance criteria:**
- [ ] `triage list --json` 在成功和出错两种情况下都输出合法 JSON
- [ ] 退出码与不带 JSON 的命令一致
- [ ] 有一个测试覆盖 `--json` 的成功输出和一个错误场景
- [ ] 默认（非 JSON）输出逐字节不变

**Out of scope:**
- 给任何其他命令加 `--json`
- 改动 PR 已经定义好的成功载荷的 JSON 形状
```

### 坏的 agent 简报

```markdown
## Agent Brief

**Summary:** 修一下分诊这个 bug

**What to do:**
分诊这玩意儿坏了。看看主文件，把它修了。
大概第 150 行那个函数有问题。

**Files to change:**
- src/triage/handler.ts（第 150 行）
- src/types.ts（第 42 行）
```

这坏在哪：
- 没有类别
- 描述含糊（「分诊这玩意儿坏了」）
- 引用了会过期的文件路径和行号
- 没有验收标准
- 没有范围边界
- 没有描述当前行为与期望行为的差别

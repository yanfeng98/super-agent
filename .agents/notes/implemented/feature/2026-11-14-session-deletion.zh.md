# Agent Note：永久删除会话

状态：implemented

[English](2026-11-14-session-deletion.md) | 中文

## 问题

对话可以被隐藏，却永远无法移除：`WorkspaceRegistry.archiveSession` 只写入持久化的归档集合，日志、附件与工作区记账都保留，且包 README 明确记录会话删除是缺失能力。用户会无限累积对话，而一次误归档只需切换一次筛选就会重新出现。

持久化接缝没有删除操作，而安全地删除日志并不是一次文件系统调用。在 Web GUI 中打开会话会激活其 Agent（follow 流上的 `activate: true`），而 `ApiSessionAgentController` 丢弃了返回的 `AgentHandle`，因此每个被打开过的会话都持有其持久化写句柄——以及 `session.lock` 租约——直到 Host 退出。忽略该持有者的删除会 unlink 一份活动写方仍在追加的日志；而拒绝所有活动会话的删除会拒绝用户最常看到的大多数行。

## 决策

**持久化接缝增加一个独占且完整的删除操作。** `SessionPersistence.remove(id)` 删除一个会话的完整制品集合，而不是某一代际。每个后端都必须在本进程仍有打开的句柄或待创建项持有该 id 时拒绝（`SessionPersistenceBusyError`），在另一个 Host 持有写租约时跨进程拒绝，并对不存在的 id 拒绝（`SessionPersistenceNotFoundError`）。随产品交付的 JSONL 后端用其进程内追踪器、与 append 发布相同的 `SessionWriteLease`——在整个目录删除期间持有，因此锁文件随日志一起消失——以及清除其 header 与冷日志备忘录条目来实现它。只提供读取的测试替身也必须实现该方法。

**控制器现在可以停止它发布过的 Agent。** `ApiSessionAgentController` 保留它每次 `agents.create`/`agents.resume` 返回的句柄。`disposeAgent(sessionId)` 等待在途 resume，然后释放保留的句柄，从而停止循环、关闭写句柄并移除会话。`beginDeletion`/`endDeletion` 在删除进行期间挡住该身份的新 resume 与具名会话创建，因此重连的客户端、提醒或「新会话」复用都无法重新发布一个日志即将消失的会话。没有保留句柄的活动 Agent（仅在开发期重载替换本插件而 Host 仍保留其 Agent 时出现），或仍附着在 Session store 中却没有本控制器发布的 Agent 的会话，会报告为 `session/in-use`，而不是在其持有者之下被删除。

**`session/delete` 将准入置于任何破坏性步骤之前。** Host 从活动存储或 `stat` 解析身份（`session/not-found`）、拒绝子智能体对话（`session/subagent`，因为其行由父目录投影拥有）、询问既有的 `workspace/session-activity` waterfall 并以与归档相同的活动详情拒绝非空答案（`session/active`），之后才挡栅、释放 Agent、删除存储并调用 `workspaceRegistry.forgetSession`。删除在有工作运行时被拒绝而不是去停止它，因为删除不可逆：用户先停止回合，正如他们在不带 `stopActivity` 的归档之前所做的那样。`forgetSession` 以两次写入清除记账槽位、归档与置顶成员资格，以及缓存的 header 与路径事实；二者之间的崩溃只会留下成员投影与下一次修改本就能容忍的引用，因此没有新增 pending-mutation 标记。已释放的 Agent 已发出 `api-session/removed`；冷删除直接发出该边，因此每个对端都会移除该行。从未实体化的活动会话没有制品，其缺失的日志确认删除完成，而不是让删除失败。

**Client 只负责呈现。** `ISessions.delete` 调用 Remote 并刷新权威列表，失败时抛出携带 Host 失败的 `SessionDeleteError`。侧栏的垃圾桶悬停按钮与菜单中 danger 样式的“删除会话…”项升起同一个确认对话框；确认后在它指向被删除会话时清空选择，并升起成功提示；广播的移除也会让其他仍指向该会话的客户端复位其视图。不提供撤销，因为不再有副本；拒绝文案指出仍在运行的工作或持有者。子智能体行从不渲染该动作（侧栏已过滤 `origin === 'subagent'`），Host 的拒绝覆盖直接调用 API 的调用方。

## 备选方案

**把删除做成归档别名。** 代价：用户明确要求销毁的日志仍留在磁盘上，归档筛选会不断让它重新出现。产品已有归档；把第二次隐藏命名为删除会错误描述数据保证。

**只允许删除冷会话。** 代价：本次 Host 运行中打开过的每个对话都将无法删除，因为 Web 打开路径会激活 Agent，而写句柄存活到进程退出。可用的动作会退化成“重启 Host，然后删除”，这不是删除按钮所承诺的。

**增加 `AgentRegistry.dispose(id)` 作为第二释放所有者。** 代价：句柄契约刻意让创建方成为释放器的唯一所有者，为 GUI BFF 的需求改动核心生命周期会要求更新循环文档并扩大评审范围。在创建处保留句柄，能力仍留在其既有所有者手中。

**把删除级联到子智能体后代。** 代价：子会话是独立的持久化对话，通常有用户仍可能阅读的自己的记录；父会话的删除不得悄悄销毁它们。父会话消失后它们成为根行，`flattenLineage` 已处理该情况。

**回收投影检查点、附件与日程目标。** 代价：附件与检查点可能被恢复或分叉的对话共享，按会话回收并不安全；保留策略拥有它们。已结束或暂停的日程目标保留其 id，日程 UI 已把该对话报告为不可用。

## 后果

删除永久且不可逆；确认框会说明这一点，也不存在撤销提示。当工作或持有者只是暂时存在时 Host 可以拒绝，因此对话框把 `session/active`、`session/in-use`、`session/not-found` 与 `session/subagent` 映射为本地化文案，而不是隐藏动作。`SessionPersistence` 增加了一个必需抽象方法，自定义后端的迁移记录在升级指南中。在替换 Session Controller 而 Host 仍保留其 Agent 的开发期重载之后，活动会话在 Host 重启前无法删除；生产构建不会重载插件。该接缝只删除单个会话自身的制品：投影检查点行、内存中的阅读记录、非活动日程目标，以及成为孤儿的子智能体会话都会保留，其中没有任何一项会被当作被删除身份的权威数据读取。

覆盖：共享持久化契约新增删除用例（可见删除、busy 拒绝、未找到、跨实例重新打开），JSONL 测试覆盖目录与租约删除，工作区注册表测试覆盖遗忘幂等与重启，`delete.host.spec.ts` 覆盖拒绝顺序以及活动与冷态两条路径，Client 测试覆盖行按钮、对话框各分支、清空选择与提示，`apps/web/tests/session-delete.e2e.ts` 证明两个入口、活动会话释放其 Agent，以及跨重载的持久性。

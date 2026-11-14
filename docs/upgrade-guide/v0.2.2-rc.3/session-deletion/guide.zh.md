---
kind: upgrade-guide
description: 自定义会话持久化后端必须实现新增的必需方法 `remove` 后才能升级到下一版本。
---

# 会话持久化要求实现删除方法

[English](guide.md) | 中文

## 变更

`@deepseek-ai/dsh-session-persistence` 的 `SessionPersistence` 新增了一个必需抽象方法 `remove(id, options?)`，以及新的失败类型 `SessionPersistenceBusyError`。随产品交付的 JSONL 后端已实现两者；自定义后端子类在补上该方法之前无法实例化，而此前默默保留会话制品的既有后端也不再满足约定。

Web GUI 现在通过 `session/delete`、侧栏的行内按钮及其菜单提供永久删除会话。删除会移除单个会话的全部已提交代际，并清除其工作区记账、归档与置顶引用。该操作不可逆；归档操作仍然只隐藏而不删除。

## 迁移

1. 在你拥有的每个 `SessionPersistence` 子类中实现 `remove(id, options?)`。删除该会话完整的制品集合——全部已提交代际以及后端拥有的目录或记录——而不是某一个已观察代际。开始前先观察 `options.signal`。
2. 在本进程仍有打开的句柄或待创建项持有该 id 时，以及在另一个进程持有其写租约时，以 `SessionPersistenceBusyError` 拒绝。对不存在的身份以 `SessionPersistenceNotFoundError` 拒绝。完成后，共享该存储的所有实例的 `stat`、`list` 与 `open` 都不得再报告该身份。
3. 对你的后端运行 `packages/session/session-persistence/tests/contract.ts` 中共享的删除用例。
4. 用 `pnpm run typecheck` 与你的后端持久化测试套件确认迁移；随后验证删除一个已播种会话后 `stat` 返回 `undefined`，且 `open` 以未找到拒绝。

---
kind: upgrade-guide
description: 会话持久化声明后端能否删除会话；基类默认拒绝，随产品交付的 SQLite 后端把制品保持为仅追加。
---

# 会话持久化声明后端能否删除会话

[English](guide.md) | 中文

## 变更

`@deepseek-ai/dsh-session-persistence` 的 `SessionPersistence` 新增 `remove(id, options?)`，以及两个失败类型：`SessionPersistenceBusyError` 与新的 `SessionPersistenceRemoveUnsupportedError`。随产品交付的 JSONL 后端删除单个会话的全部已提交代际；随产品交付的 SQLite 后端把 Segment 与 frame 保持为仅追加，因此继承基类实现，以 `SessionPersistenceRemoveUnsupportedError` 拒绝，而不会声称完成一次它做不到的删除。未覆盖 `remove` 的子类同样继承该拒绝。

Web GUI 通过 `session/delete`、侧栏的行内按钮及其菜单提供永久删除会话。删除会移除该会话已存储的日志，并清除其工作区记账、归档与置顶引用。该操作不可逆；归档操作仍然只隐藏而不删除。

## 迁移

1. 只在存储能够删除会话的后端中覆盖 `remove(id, options?)`。删除完整的制品集合——全部已提交代际以及后端拥有的目录或记录——而不是某一个已观察代际，并在开始前先观察 `options.signal`。存储为仅追加时保留基类实现；有人针对这类 profile 请求删除时，侧栏会说明拒绝原因。
2. 在覆盖实现的后端中，本进程仍有打开的句柄或待创建项持有该 id 时，以及另一个进程持有其写租约时，以 `SessionPersistenceBusyError` 拒绝。对不存在的身份以 `SessionPersistenceNotFoundError` 拒绝。完成后，共享该存储的所有实例的 `stat`、`list` 与 `open` 都不得再报告该身份。
3. 运行 `packages/session/session-persistence/tests/contract.ts` 中共享的删除用例。在 `ContractBackend` 中传入 `removal: 'unsupported'` 即可改为接受拒绝行为的校验；后端能删除时省略该项。
4. 用 `pnpm run typecheck` 与你的后端持久化测试套件确认，然后验证删除一个已播种会话后 `stat` 返回 `undefined`，且 `open` 以未找到拒绝。

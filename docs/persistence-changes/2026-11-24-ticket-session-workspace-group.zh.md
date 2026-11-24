---
description: "记录持久化类型更改及其兼容性确认。"
kind: persistence-change
---

# 2026-11-24-ticket-session-workspace-group

[English](2026-11-24-ticket-session-workspace-group.md) | 中文

## 概述

为 session/workspace-group 提示新增 ticket role 和可选的 merged 标记。

## 目录

- [声明](#declaration)
- [兼容性](#compatibility)
- [验证](#verification)
- [开发备注](#dev-note)

<a id="declaration"></a>
## 声明

```yaml persistence-change
schemaVersion: 1
id: 2026-11-24-ticket-session-workspace-group
baseline: false
changes:
  - root: "event:session/workspace-group"
    previous: "2026-11-24-orchestrator-workspace-group"
    after: "6bfd95f8161011d9524585c00c132032ec74cb4838d066951b986f1873543c80"
    decision: same-version
```

<a id="compatibility"></a>
## 兼容性

旧记录仍可解析：两个新增字段都是可选的，缺少它们的提示保持原有含义。改动前的读取方会拒绝 ticket role（其状态 schema 只接受 orchestrator），因此会校验失败而不是误读为普通提示行，也不会把工单 Session 当作 orchestrator。新读取方接受两种取值。header 版本不变，因为 role 取值这个判别维度不会被误读成另一种既有状态。

<a id="verification"></a>
## 验证

pnpm exec vitest run packages/api/session-controller/tests/workspace-group.host.spec.ts：通过。pnpm exec vitest run packages/client/ui-workspace/tests：605 个通过。pnpm exec vitest run（dsh-superagent）tests/worktrees.spec.ts tests/panel-trigger.spec.tsx tests/panel-eval.spec.tsx：29 个通过。

<a id="dev-note"></a>
## 开发备注

无。

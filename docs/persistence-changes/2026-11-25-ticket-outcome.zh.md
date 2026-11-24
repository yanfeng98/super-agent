---
description: "记录持久化类型更改及其兼容性确认。"
kind: persistence-change
---

# 2026-11-25-ticket-outcome

[English](2026-11-25-ticket-outcome.md) | 中文

## 概述

为会话工作区分组显示提示增加可选的工单结束结果字段。

## 目录

- [声明](#declaration)
- [兼容性](#compatibility)
- [验证](#verification)
- [开发备注](#dev-note)

<a id="declaration"></a>
## 声明

```yaml persistence-change
schemaVersion: 1
id: 2026-11-25-ticket-outcome
baseline: false
changes:
  - root: "event:session/workspace-group"
    previous: "2026-11-24-ticket-session-workspace-group"
    after: "f0ff574af741acf7ce6dec754f4513e39196140d6f03a2435c9eaac7773fcfd3"
    decision: same-version
```

<a id="compatibility"></a>
## 兼容性

已有记录仍然有效：读取方忽略缺失的 outcome，并继续使用已有的 merged 标记。写入方只对未合并就结束的工单记录 outcome；缺失时侧边栏仍显示原有的工单或已合并标记。该字段只用于展示，不改变分组、归属或执行。

<a id="verification"></a>
## 验证

pnpm exec vitest run packages/api/session-controller/tests/workspace-group.host.spec.ts：4 个测试通过；pnpm run typecheck：5 个门禁通过。

<a id="dev-note"></a>
## 开发备注

无。

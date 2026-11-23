---
description: "记录持久化类型更改及其兼容性确认。"
kind: persistence-change
---

# 2026-11-24-orchestrator-workspace-group

[English](2026-11-24-orchestrator-workspace-group.md) | 中文

## 概述

新增仅写入日志的 session/workspace-group 事件及其 workspaceGroup 摘要投影：当会话存储的 cwd 指向另一个目录时（例如 IM bot 的 orchestrator 归入 bot 的工作区），记录持久的侧边栏分组提示。

## 目录

- [声明](#declaration)
- [兼容性](#compatibility)
- [验证](#verification)
- [开发备注](#dev-note)

<a id="declaration"></a>
## 声明

```yaml persistence-change
schemaVersion: 1
id: 2026-11-24-orchestrator-workspace-group
baseline: false
changes:
  - root: "event:session/workspace-group"
    previous: null
    after: "89f240c94d5d5d3a56241550814cf26f51b98d61d93c1977ba983b3ba7185311"
    decision: same-version
```

<a id="compatibility"></a>
## 兼容性

已有日志仍然有效：事件缺失时投影初始化为 null，且该事件不改变任何 header 字段、cwd、Workspace 成员关系、执行目录或模型可见面。不认识该事件的读者会拒绝携带它的日志，这与所有必需 Session 事件一致；该提示从不影响执行。写入方只对 orchestrator 会话追加，并且仅在记录值不同时追加。

<a id="verification"></a>
## 验证

pnpm exec vitest run packages/api/session-controller/tests/workspace-group.host.spec.ts packages/api/session-controller/tests/session-projections.host.spec.ts：29 个测试通过。pnpm exec vitest run packages/client/ui-workspace/tests/tree.client.spec.ts packages/client/ui-workspace/tests/rows.client.spec.tsx：172 个测试通过。apps/web/native/dsh-superagent 的 vitest tests/runtime.spec.ts：102 个测试通过。node --test apps/web/native/dsh-im/test/checked-delivery.test.mjs：13 个测试通过。pnpm run typecheck、pnpm run lint 与 pnpm run doc-sync 通过。Web e2e 侧边栏分组测试通过，过滤后的 v8 覆盖率运行显示 workspace-group.ts、tree.ts、Rows.tsx 与 locales.ts 的语句、分支、函数和行覆盖率均为 100%。聚合构建的 host、client 与 Web 门禁通过；与本次改动无关的 vendored rea 步骤要求 Node.js 24.22.3 或更新版本，在本机 24.21.0 上失败。

<a id="dev-note"></a>
## 开发备注

无。

---
description: "Records a persistence type transition and its compatibility acknowledgement."
kind: persistence-change
---

# 2026-11-24-orchestrator-workspace-group

English | [中文](2026-11-24-orchestrator-workspace-group.zh.md)

## Summary

Adds the log-only session/workspace-group event and its workspaceGroup summary projection: a durable sidebar grouping hint for a Session whose stored cwd names another directory, such as the IM bot orchestrator joining the bot's Workspace.

## Table of Contents

- [Declaration](#declaration)
- [Compatibility](#compatibility)
- [Verification](#verification)
- [Dev Note](#dev-note)

<a id="declaration"></a>
## Declaration

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
## Compatibility

Existing logs stay valid: an absent event initializes the projection to null, and the event changes no header field, cwd, Workspace membership, execution directory, or model-visible surface. A reader that does not know the event refuses a log carrying it, as with every required Session event, and the hint never alters execution. Writers append it only for the orchestrator Session and only when the recorded value differs.

<a id="verification"></a>
## Verification

pnpm exec vitest run packages/api/session-controller/tests/workspace-group.host.spec.ts packages/api/session-controller/tests/session-projections.host.spec.ts: 29 passed. pnpm exec vitest run packages/client/ui-workspace/tests/tree.client.spec.ts packages/client/ui-workspace/tests/rows.client.spec.tsx: 172 passed. apps/web/native/dsh-superagent vitest tests/runtime.spec.ts: 102 passed. node --test apps/web/native/dsh-im/test/checked-delivery.test.mjs: 13 passed. pnpm run typecheck, pnpm run lint, and pnpm run doc-sync passed. The web e2e sidebar grouping test passed, and a filtered v8 coverage run reports 100% statements, branches, functions, and lines for workspace-group.ts, tree.ts, Rows.tsx, and locales.ts. The aggregate build passed its host, client, and Web gates; its unrelated vendored rea step requires Node.js 24.22.3 or newer and fails on this host's 24.21.0.

<a id="dev-note"></a>
## Dev Note

None.

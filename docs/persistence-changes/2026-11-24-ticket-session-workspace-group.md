---
description: "Records a persistence type transition and its compatibility acknowledgement."
kind: persistence-change
---

# 2026-11-24-ticket-session-workspace-group

English | [中文](2026-11-24-ticket-session-workspace-group.zh.md)

## Summary

Widens the session/workspace-group hint with a ticket role and an optional merged marker.

## Table of Contents

- [Declaration](#declaration)
- [Compatibility](#compatibility)
- [Verification](#verification)
- [Dev Note](#dev-note)

<a id="declaration"></a>
## Declaration

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
## Compatibility

Older records parse unchanged: both additions are optional, and a hint without them keeps its previous meaning. A reader from before this change refuses the ticket role because its state schema accepts only orchestrator, so it fails validation instead of misreading the Session as an ordinary hinted row; it never treats a Ticket Session as an orchestrator. New readers accept both values. The header version is unchanged because the discriminator (role value) cannot be misread as a different established state.

<a id="verification"></a>
## Verification

pnpm exec vitest run packages/api/session-controller/tests/workspace-group.host.spec.ts: passed. pnpm exec vitest run packages/client/ui-workspace/tests: 605 passed. pnpm exec vitest run (dsh-superagent) tests/worktrees.spec.ts tests/panel-trigger.spec.tsx tests/panel-eval.spec.tsx: 29 passed.

<a id="dev-note"></a>
## Dev Note

None.

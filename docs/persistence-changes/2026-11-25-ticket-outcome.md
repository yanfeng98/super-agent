---
description: "Records a persistence type transition and its compatibility acknowledgement."
kind: persistence-change
---

# 2026-11-25-ticket-outcome

English | [中文](2026-11-25-ticket-outcome.zh.md)

## Summary

Adds an optional terminal ticket outcome to the session workspace-group display hint.

## Table of Contents

- [Declaration](#declaration)
- [Compatibility](#compatibility)
- [Verification](#verification)
- [Dev Note](#dev-note)

<a id="declaration"></a>
## Declaration

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
## Compatibility

Existing records remain valid: readers ignore an absent outcome and still use the existing merged flag. Writers add the outcome only for a ticket that ended without merging, and the sidebar shows the existing ticket or merged mark when it is absent. The field is display-only and never changes grouping, membership, or execution.

<a id="verification"></a>
## Verification

pnpm exec vitest run packages/api/session-controller/tests/workspace-group.host.spec.ts: 4 tests passed; pnpm run typecheck: 5 gates passed.

<a id="dev-note"></a>
## Dev Note

None.

---
kind: upgrade-guide
description: Session persistence declares whether a backend can remove a Session; the base class refuses, and the shipped SQLite backend keeps its artifacts append-only.
---

# Session persistence declares whether it can remove a Session

English | [中文](guide.zh.md)

## Change

`SessionPersistence` in `@deepseek-ai/dsh-session-persistence` gained `remove(id, options?)` with two failures: `SessionPersistenceBusyError` and the new `SessionPersistenceRemoveUnsupportedError`. The shipped JSONL backend deletes every committed generation of one Session; the shipped SQLite backend keeps its Segments and frames append-only, so it inherits the base class's implementation, reports `canRemove` as `false`, and refuses with `SessionPersistenceRemoveUnsupportedError` instead of claiming a deletion it cannot make. A subclass that does not override `remove` inherits that refusal and that `false`.

The Web GUI offers permanent Session deletion through `session/delete`, the sidebar's row button, and its menu. Deletion removes a Session's stored log and drops its Workspace account, archive, and pin references. It is irreversible, and the archive action still hides without deleting.

## Migration

1. Override `remove(id, options?)` and set `canRemove` to `true` only in a backend whose storage can delete a Session. Host callers read `canRemove` before releasing a Session, so the deletion command refuses while the conversation is still open when it is `false`. Delete the complete artifact set — every committed generation and backend-owned directory or record — not one observed generation, and observe `options.signal` before starting. Leave the base implementation in place when the storage is append-only; the sidebar reports the refusal when someone asks for a deletion such a profile cannot make.
2. In an overriding backend, refuse with `SessionPersistenceBusyError` while any open handle or pending creation holds the id in this process, and while another process holds its write lease. Reject an absent identity with `SessionPersistenceNotFoundError`. After resolution, `stat`, `list`, and `open` must not report the identity on any instance sharing the storage.
3. Run the shared removal cases in `packages/session/session-persistence/tests/contract.ts`. Pass `removal: 'unsupported'` in your `ContractBackend` to be held to the refusal instead; omit it when your backend deletes.
4. Confirm with `pnpm run typecheck` and your backend's persistence suite, then verify that removing a seeded Session makes `stat` return `undefined` and `open` reject as not found.

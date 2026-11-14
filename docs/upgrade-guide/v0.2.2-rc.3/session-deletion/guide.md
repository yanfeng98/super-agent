---
kind: upgrade-guide
description: Custom Session persistence backends must implement the new required `remove` method before the next release.
---

# Session persistence requires a removal method

English | [中文](guide.zh.md)

## Change

`SessionPersistence` in `@deepseek-ai/dsh-session-persistence` gained a required abstract method, `remove(id, options?)`, and a new failure, `SessionPersistenceBusyError`. The shipped JSONL backend already implements both; a custom backend subclass no longer instantiates until it adds the method, and existing backends that silently kept session artifacts can no longer satisfy the contract.

The Web GUI now offers permanent Session deletion through `session/delete`, the sidebar's row button, and its menu. Deletion removes every committed generation of one Session and drops its Workspace account, archive, and pin references. It is irreversible, and the archive action still hides without deleting.

## Migration

1. Implement `remove(id, options?)` in every `SessionPersistence` subclass you own. Delete the Session's complete artifact set — every committed generation and backend-owned directory or record — not one observed generation. Observe `options.signal` before starting.
2. Refuse with `SessionPersistenceBusyError` while any open handle or pending creation holds the id in this process, and while another process holds its write lease. Reject an absent identity with `SessionPersistenceNotFoundError`. After resolution, `stat`, `list`, and `open` must not report the identity on any instance sharing the storage.
3. Run the shared removal cases in `packages/session/session-persistence/tests/contract.ts` against your backend.
4. Confirm the migration with `pnpm run typecheck` and your backend's persistence suite; then verify that removing a seeded Session makes `stat` return `undefined` and `open` reject as not found.

# ADR-0005: Generic trigger-based undo/redo with stale check

- **Status:** Accepted (implemented)
- **Date:** 2026-09-30
- **Related:** FSD §51 (undo across major workspaces), §52 (Recently Deleted); Autosave/Undo/Recovery spec;
  `crates/openframe-persistence/src/undo.rs`, `crates/openframe-application/src/store.rs`;
  [17-autosave-undo-recovery.md](../engineering/17-autosave-undo-recovery.md)

## Context

FSD §51.1 requires undo for text edits, card create/delete/move, board reorder, breakdown association, shot reorder,
schedule moves, note changes and form edits: essentially every mutation in every module. Hand-written inverse
commands per operation are error-prone, and a forgotten inverse silently loses data. With applied Change Sets (AI,
import, package responses) and the Global Idea Vault shared across projects, an undo must never overwrite work done
afterwards by something else.

## Decision

1. **Capture, don't invert.** After migrations, each writer connection installs **TEMP triggers** (`AFTER
   INSERT/UPDATE/DELETE`) on every *tracked* table. They write JSON before/after row images into
   `temp._undo_capture`. Tracked = every table except the prefixes `_`, `sqlite_`, `sys_` and `search_`, and FTS
   virtual tables.
2. **Schema rules make this safe.** Tracked tables must have an `id TEXT PRIMARY KEY`, **no BLOB columns** (checked
   at install; the app refuses to start otherwise) and **no `ON DELETE CASCADE`** (convention, reviewed), so every
   row change is visible to the triggers.
3. **Per mutation** (`Store::mutate`): the capture is cleared at `BEGIN`, drained after the module logic, and folded
   into **net per-row changes** (earliest `old`, latest `new`; no-op rows dropped). The result is stored as one
   `sys_undo` entry in the **same transaction** with the human label from `MutationMeta` (e.g. `Moved Scene Card
   “…”`).
4. **Stacks** are per store (project / Global Idea Vault) and **per actor** (`actor_id`), capped at **300 entries**.
   A new mutation clears that actor's redo entries. Consecutive mutations with the same **coalesce key** within
   **4 s** merge into one step (typing).
5. **Stale check.** Before applying, undo verifies that every affected row still equals the entry's *after* image
   (redo: the *before* image). On any mismatch nothing is written, `conflict.undo` is returned ("…the same content
   was changed afterwards. Your latest work was kept."), and the conflicting entry is dropped together with all
   older undo entries of that actor (or all redo entries), keeping the remaining history consistent.
6. **Apply** runs in one `IMMEDIATE` transaction with `defer_foreign_keys=ON`. Undo writes rows in reverse order and
   redo in forward order. The applied changes are re-indexed for search, recorded as activity (`Undid: …`/`Redid: …`)
   and emitted as `DataChanged`.
7. **Persistence.** Undo history lives in the store database, so it survives restart and crash (FSD §51.2 "restore
   recent saved state changes"). Duplicate Project clears it.

## Consequences

- Every module gets correct undo/redo, including recoverable delete and restore, without writing inverse logic
  (AGENT_BRIEF §4 forbids it).
- Undo never overwrites later work. After a schema migration, old entries whose row shapes no longer match fail the
  stale check and are refused safely rather than applied wrongly.
- Storage cost: JSON row images, at most 300 entries per actor. Very large operations (e.g. importing a 120-page
  script) produce one large entry. This is acceptable and bounded.
- Infrastructure writes (`sys_activity`, `search_doc`, view state) are deliberately not undoable.
- Files created in a mutation are removed on rollback. Files are deleted only on *permanent* purge, after commit, so
  undo never needs to restore bytes.
- Editor-local undo inside the screenplay editor is a separate, UI-level layer (ADR-0010).

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Command pattern with hand-written inverses | Must be written and tested per op, and one forgotten inverse means data loss |
| SQLite session extension (changesets) | Not exposed safely by rusqlite's bundled build without extra features. Conflict handling is lower-level. Changesets are binary and opaque to debugging. |
| Snapshot-based undo (copy DB per step) | Too expensive per keystroke-coalesced edit |
| In-memory undo only | Lost on crash and restart, contradicting FSD §51.2 |

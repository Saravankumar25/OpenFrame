# ADR-0011: UUIDv7 identities and no stored display numbers

- **Status:** Accepted (implemented: `crates/openframe-domain/src/ids.rs`, all migrations, `persistence::rows`)
- **Date:** 2026-09-30
- **Related:** Domain spec §2 (stable identity); invariant 5; PRD N-1, N-3, N-10; FSD-STORY-003

## Context

Objects move between containers (cards between sequences), stores (Global Idea Vault → project), projects
(exchange packages, duplicate) and machines (packages returned by collaborators). Screenplay scene numbers, shot numbers and act
letters are *derived from order* and renumber when things move (PRD N-1, FSD-STORY-003 "no manual numbering").
Using numbers or database rowids as identity would break references on every reorder and collide across packages.

## Decision

1. **Every canonical row's primary key is a UUIDv7 string** (`openframe_domain::new_id()`), stored as `TEXT` in the
   canonical hyphenated form. v7 is globally unique (safe across packages and machines) and **time-sortable**,
   giving a stable tie-breaker and index locality.
2. Ids are **opaque**. Nothing parses meaning from them except `is_valid_id`, which validates ids arriving over IPC or
   inside packages before use.
3. **Order is `position INTEGER`** among siblings (`rows::{sibling_ids, next_position, place_at, renumber}`).
   **Display numbers are never stored**: scene numbers, shot numbers, act letters and strip numbers are computed at
   read time from order.
4. **Identity across versions** uses explicit lineage columns, not numbers. Example: `screenplay_scene.lineage_id` is
   stable across drafts, while each draft's scene rows have their own ids.
5. **Copies get new identities.** Vault → Story, Duplicate Project (new `project.id` and manifest), and importing a
   package "as copy" all create new ids. Provenance may be recorded, but a copy is never a live link (PRD N-3).
6. Timestamps are `INTEGER` milliseconds since the Unix epoch, UTC (`now_ms()`).

## Consequences

- Reordering touches only `position` values, and references stay valid, so undo of a move is a simple row restore.
- Search results, comments (`target_type` / `target_id`), breakdown elements and AI provenance refer to objects that
  survive renumbering.
- Ids are 36 characters as TEXT. The storage cost is accepted for readability in the human-inspectable database.
- **Open product decision (not decided here):** the industry convention of *frozen* scene numbers after script lock
  (12A, "OMITTED") contradicts PRD N-1. If adopted, it would be a stored *label* on the locked draft, never an
  identity. It is flagged for the product owner.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| SQLite `INTEGER PRIMARY KEY` rowids | Collide across packages and peers, and are reused after deletes |
| UUIDv4 | Random order causes poor index locality and no natural tie-break by creation time |
| ULID | Equivalent properties, but UUIDv7 is standardized (RFC 9562) and supported by the `uuid` crate |
| Human-readable ids (e.g. `scene-12`) | Encodes display order in identity, which is exactly what must not happen |

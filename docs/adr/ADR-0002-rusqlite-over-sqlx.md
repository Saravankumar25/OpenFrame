# ADR-0002: rusqlite (bundled SQLite with FTS5 + online backup API) instead of SQLx

- **Status:** Accepted (implemented). Supersedes the "DB access: SQLx" row of ESD §2.
- **Date:** 2026-09-30
- **Related:** ESD §2, §6; ADR-0003, ADR-0005; `crates/openframe-persistence`

## Context

ESD §2 proposed SQLx ("explicit SQL, migrations and typed mapping"). The persistence requirements that emerged
from the FSD, the Autosave/Recovery spec and the Release/Migration spec are:

1. **One writer, synchronous, transactional** mutation pipeline per store (ESD §6). Every mutation is a short
   `BEGIN IMMEDIATE … COMMIT` on a dedicated writer connection, and reads use a separate read-only connection.
2. **FTS5** full-text search available everywhere, with the same SQLite version on every user machine.
3. **Consistent online copies of a live database** for crash checkpoints, pre-migration safety backups, duplicate
   project and backup packages, taken without closing the project.
4. **Connection-scoped TEMP triggers** that capture before/after row images for generic undo (ADR-0005).
5. **Custom migration semantics:** verified safety backup first, a single atomic transaction for all pending
   migrations, refusal of databases written by newer versions (`too_new`), an FK and integrity check before
   commit, and checksum drift detection.
6. Tests that create hundreds of throwaway databases quickly and headless.

## Decision

Use **`rusqlite` 0.37** with the features `bundled` (SQLite compiled in, FTS5 enabled, identical on every machine),
`backup` (online backup API), `functions` and `serde_json`. Implement our own small migration runner
(`openframe-persistence::migrate`).

- Connection policy (`open_connection`): `journal_mode=WAL`, `synchronous=FULL`, `foreign_keys=ON`,
  `busy_timeout=5s`, `temp_store=MEMORY`, `recursive_triggers=OFF`, 16 MB page cache.
- `backup_to()` uses `rusqlite::backup::Backup`. It writes `<dest>.partial`, verifies it with `quick_check`, then
  renames it, so a crash never leaves a half-written backup under the final name.
- Blocking calls are kept off the UI thread by the Tauri adapter (`spawn_blocking` per `of_invoke`) and by
  `AppCore::spawn_task` for long work.

## Consequences

- There is no compile-time SQL checking (SQLx's `query!` needs a live `DATABASE_URL` or offline metadata). This is
  mitigated by integration tests that execute every op (`tests/*.rs`), a migration test that applies every set and
  installs undo capture (`schema.rs`), and allow-listed column updates (`rows::update_fields`).
- The API is synchronous and simple, and matches the single-writer model with no async runtime inside the
  persistence layer.
- The bundled SQLite adds about 1 MB to the binary and a C compile step (MSVC is already required by Tauri).
- We own the migration runner (about 200 lines, unit-tested: fresh, upgrade, rollback on failure, too-new,
  non-contiguous lists).

## Alternatives rejected

| Alternative | Reason |
|---|---|
| SQLx (sqlite) | Async-first. Its SQLite driver serializes onto a worker thread anyway, adding complexity with no concurrency gain for a single writer. It has no safe access to the online backup API. Compile-time checks need a database or offline metadata in CI. Its migrator lacks the safety-backup and too-new semantics we need. |
| Diesel / SeaORM | ORM abstractions hide the SQL we must reason about for undo capture, FTS5 and migrations. They add heavy generated code. |
| System SQLite (non-bundled) | The version and compile options (FTS5) vary per machine, so behavior could differ between users. |
| An embedded KV store (sled, redb) | No relational integrity, no FTS, and no human-inspectable format. The locked decision is SQLite. |

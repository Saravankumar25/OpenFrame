# 11 — Physical Database (SQLite) Specification

> **Status legend:** ✅ implemented · 🚧 in development (module branch) · 📋 planned.
> This document describes the code as it is. Source: `crates/openframe-persistence`, `migrations/`,
> `crates/openframe-application/src/{schema.rs,store.rs}`. Decisions: ADR-0002, ADR-0005, ADR-0011.

## 1. Databases

| Database | Path | Migrations | Contents | Undo-tracked |
|---|---|---|---|---|
| Project ✅ | `<Title>.openframe/project.sqlite` | `migrations/project/0001…0009` | All canonical project data | Yes (canonical tables) |
| Global Idea Vault ✅ | `Documents/OpenFrame/Global Idea Vault/vault.sqlite` | `migrations/global/0001…0002` | Global vault items, their assets, trash, search | Yes |
| Application ✅ | `%LOCALAPPDATA%/OpenFrame/app.sqlite` | `migrations/app/0001` | Local profile, recent projects registry, settings, global templates. **Never project content.** | No |

Migrations are embedded in the binary (`include_str!` in `schema.rs`). The migration number is fixed per module so
modules can be developed independently:

| # | File | Owner | Status |
|---|---|---|---|
| 1 | `0001_core.sql` | foundation (project, files, history, search) | ✅ |
| 2 | `0002_idea_vault.sql` | `modules/idea_vault` (+ `global/0002`) | 🚧 placeholder |
| 3 | `0003_story.sql` | `modules/story` | ✅ hub DDL / 🚧 module |
| 4 | `0004_screenplay.sql` | `modules/screenplay` | ✅ hub DDL / 🚧 module |
| 5 | `0005_production.sql` | `modules/production` | ✅ hub DDL / 🚧 module |
| 6 | `0006_visual.sql` | `modules/visual` | 🚧 placeholder |
| 7 | `0007_schedule.sql` | `modules/schedule` | 🚧 placeholder |
| 8 | `0008_ai.sql` | `modules/ai` | 🚧 placeholder |
| 9 | `0009_collaboration.sql` | package exchange (Exchange/Review/Response package log, import sessions) | 🚧 placeholder. The file comment's "LAN sessions" is obsolete (ADR-0007). |

## 2. Connection policy ✅ (`open_connection`)

| Setting | Value | Why |
|---|---|---|
| `journal_mode` | `WAL` | Readers never block the writer, and it is crash-safe. A warning is logged if the file system refuses WAL. |
| `synchronous` | `FULL` | A committed autosave survives power loss |
| `foreign_keys` | `ON` | Referential integrity is enforced by SQLite |
| `busy_timeout` | 5 s | Tolerates brief checkpoint/backup contention |
| `temp_store` | `MEMORY` | TEMP undo capture table and triggers |
| `recursive_triggers` | `OFF` | Capture triggers must not cascade |
| `cache_size` | −16000 (≈ 16 MB) | Keeps the working set of a large project in memory |
| Open flags | `READ_WRITE \| NO_MUTEX \| URI` (+ `CREATE` only on create) | Opening a missing database never creates an empty one silently |

Each open store has **one writer connection** (behind a mutex, used only by `Store::mutate`, undo/redo and
infrastructure work) and **one reader connection** with `PRAGMA query_only=ON` for all queries.

## 3. Table conventions (canonical tables) ✅

```sql
id          TEXT PRIMARY KEY,           -- UUIDv7 (ADR-0011); never a display number
…domain columns…
position    INTEGER NOT NULL,           -- only when siblings have user-controlled order
created_at  INTEGER NOT NULL,           -- epoch ms UTC
updated_at  INTEGER NOT NULL,
rev         INTEGER NOT NULL DEFAULT 1, -- optimistic concurrency; bumped on every update
deleted_at  INTEGER                     -- recoverable delete (NULL = live)
```

Rules enforced by code or review:

1. **`id TEXT PRIMARY KEY` is mandatory** for every tracked table. `undo::install_capture` refuses to start otherwise.
2. **No `BLOB` columns** in canonical tables. Binary data lives in files referenced by an `asset` row. This is
   enforced at capture install.
3. **No `ON DELETE CASCADE`** and no triggers that delete other canonical rows. Generic undo must see every row change
   (review rule).
4. Booleans are `INTEGER` 0/1. Enumerations are `TEXT` with a `CHECK` constraint when the set is closed. Values match
   the `openframe_domain::enums` text exactly (e.g. `"Extras / Background"`).
5. Structured side data is `*_json TEXT` (e.g. `settings_json`, `anchor_json`). It is validated in Rust before writing
   and is never queried for business rules.
6. Text length limits are enforced in Rust (`util::required_text` / `optional_text` / `body_text`), not in DDL, so the
   messages stay human.
7. Updates go through `rows::update_fields(conn, table, id, fields, ALLOWED_COLS, expected_rev, what)`: an
   allow-listed column set, `rev = rev + 1`, `updated_at = now`, and `conflict.stale` when `expected_rev` differs.
8. Ordering helpers: `rows::{sibling_ids, next_position, place_at, renumber}`. Positions are dense integers
   renumbered on move.
9. Display numbers (scene, shot, act letter, strip) are **never stored**.
10. Deletion of user content is **recoverable**: `store::soft_delete` sets `deleted_at`, bumps `rev`, and records a
    `deleted_item` row with type, table, title and previous parent/position. Queries filter `deleted_at IS NULL`.
    Permanent purge is the module's `TrashHandler.purge`, which removes rows it exclusively owns and schedules file
    deletion after commit.

## 4. Infrastructure tables (not undo-tracked) ✅

Names starting with `sys_`, `search_`, `_` or `sqlite_` are excluded from undo capture.

| Table | Purpose |
|---|---|
| `_schema_migrations` | version, name, SHA-256 checksum, applied_at (drift is logged, never fatal) |
| `sys_activity` | Activity history: actor, action code, human summary, target, origin `local`/`ai` (`lan` exists in code but is unused, ADR-0007). Consecutive coalesced edits within 60 s merge. |
| `sys_undo` | Undo/redo entries: `seq`, label, `actor_id`, `coalesce_key`, `changes_json` (net row images), state `done`/`undone`. Max 300 per actor. |
| `sys_view_state` | Per-user UI state (e.g. last location for "Continue"). Never content. |
| `search_doc` + `search_fts` | Search projection and FTS5 index (see 16-search.md). Rebuildable, never canonical. |
| `temp._undo_capture` | Per-connection TEMP capture buffer, cleared per mutation |

## 5. Table ownership and hub contracts

**Ownership:** each table belongs to the module whose migration creates it. Only that module writes it. Other modules
read through small `pub(crate)` query functions or by reading *contract columns* directly.

**Hub tables** are cross-module contracts. The owning module may add columns and tables, but **must never rename or
remove a contract column** or change its meaning.

| Migration | Hub tables (contract) | Read by |
|---|---|---|
| 0001 core | `project`, `project_member` (role), `season`, `episode`, `deleted_item`, `asset`, `project_file(_folder)`, `snapshot` (immutable), `project_note`, `task`, `comment` (`target_type`/`target_id`/`scene_id`/`anchor_json`, `parent_id` replies, status `Open`/`In Discussion`/`Resolved`, `review_round_id`), `private_note` (**owner-filtered always**), `template` | all modules |
| 0003 story | `story_act`, `story_sequence`, `story_beat`, `story_scene_card` (`parent_type` ∈ act/sequence/parking/unassigned; `screenplay_scene_id` is an informational link with no sync), `story_character` | screenplay, production, AI |
| 0004 screenplay | `screenplay` (`current_draft_id`), `screenplay_draft` (status Draft/Review/Locked/Revision, `created_from_draft_id`), `screenplay_scene` (`draft_id`, **`lineage_id`** = stable cross-draft identity, `position` → display number, `heading`, `synopsis`, `story_day`, `omitted`), `screenplay_element` (`scene_id`, `position`, `element_type`, `text`) | production, schedule, visual, AI, export |
| 0005 production | `production_source` (`draft_id`, `active`), `catalog_item` (canonical `category`), `breakdown_element` (`source_id`, `scene_id`, `scene_lineage_id`, `category`, `catalog_item_id`, `confirmation_state`; only Confirmed/Manual are production truth), `location`, `cast_member` (`character_id`), `crew_member` | schedule, call sheets, visual, AI |

Source-of-truth rules expressed in the schema: production references screenplay scenes of the selected
`production_source` and never writes screenplay tables. Story cards hold only an informational
`screenplay_scene_id`. Copies (vault → story, draft → draft) create new ids and record their `source_*_id`.

## 6. Foreign keys and indexes

- FKs reference parent ids **without** cascade. Deleting a parent with live children is prevented by the module (or
  children are moved to "unassigned" on restore, FSD §52.5).
- Undo/redo sets `PRAGMA defer_foreign_keys=ON` within its transaction, so intermediate row orders are allowed.
  Commit still validates.
- Every `(parent, position)` ordering has a composite index (`idx_story_card_parent`, `idx_scene_draft`,
  `idx_element_scene`, …). Lookups by target use `(target_type, target_id)` indexes.
- `deleted_item` has a unique `(table_name, object_id)`, so re-deleting after a restore updates the same row.

## 7. Migration policy ✅ (`migrate.rs`, `Store::open`)

1. The migration list must be contiguous from 1 (validated). The version is stored in `PRAGMA user_version`.
2. Opening an existing store:
   - `quick_check` first. A corrupt database is refused with **no modification** (`project_format.corrupt`).
   - Version **newer** than supported → refused (`project_format.too_new`). The project is untouched.
   - Upgrade → **verified safety backup first** (`backups/pre-migration-v<from>-<ms>.sqlite` via the online backup
     API + `quick_check`), then **all pending migrations in one transaction** with FKs off, followed by
     `foreign_key_check` + `quick_check`, then `user_version`, then commit. Any failure rolls back completely and
     restores `foreign_keys=ON`.
   - After an upgrade the search index is rebuilt.
3. **Frozen once shipped.** After a version ships, its migration file must never change. Changes are new appended
   files. Checksums of applied migrations are compared at open and drift is logged (never fatal, so a developer
   mistake can never lock a user out). Before the first release, module migration files may still be edited.
4. Table rebuilds (SQLite `ALTER` limits) follow the 12-step pattern inside the migration transaction. Contract
   columns keep their names and meaning.
5. Every migration set is tested to apply cleanly and to be undo-trackable (`schema.rs` test).

See also 23-release-migration.md.

## 8. Durability operations ✅

| Operation | Implementation |
|---|---|
| Autosave | Every command commits (WAL, `synchronous=FULL`) |
| Explicit Save / clean close | `wal_checkpoint(TRUNCATE)` + online backup to `recovery/checkpoint.sqlite` |
| Safety backups | `backup_to()`: `.partial` → `quick_check` → rename |
| Integrity | `integrity_problems(full)` (quick/full check), `foreign_key_problems()` |
| Search index | `Store::rebuild_search_index()` from canonical rows |

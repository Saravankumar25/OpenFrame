# 17: Autosave, Undo, Recovery & Backup

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned.
> Source: `store.rs` (`Store::mutate`, `SaveTracker`, undo/redo), `persistence/{lib.rs,undo.rs}`,
> `modules/{project.rs,history.rs}`, `project-format` (crash marker). Decisions: ADR-0003, ADR-0005, ADR-0010.
> Tests: `crates/openframe-application/tests/core_pipeline.rs`.

## 1. Guarantees

| Guarantee | What it promises | Mechanism |
|---|---|---|
| Saved means durable | A change the UI reports as saved survives a crash or power loss | Every command is one SQLite transaction committed with WAL + `synchronous=FULL` before the op returns ✅ |
| No partial writes | A failed write leaves no partial state | Transaction rollback; files created in the transaction are removed; purged files are deleted only after commit ✅ |
| Recovery choice after a crash | After an abnormal exit, the user chooses: latest autosave or last confirmed save | Crash marker + `recovery/checkpoint.sqlite` ✅ |
| Safe undo | Undo never overwrites later work | Stale check against full row images (ADR-0005) ✅ |
| Safe upgrade | Upgrading OpenFrame never destroys a project | Verified safety backup before migration; atomic migration; too-new refused ✅ |
| Recoverable deletion | Deletion is recoverable until deliberately purged | `deleted_at` + `deleted_item`; purge is a separate Owner action ✅ |

## 2. Autosave ✅

There is no "unsaved document". **Continuous autosave = every mutation is a commit.** Typing is persisted by the
editor through coalesced commands (ADR-0010), so a crash loses at most the in-flight keystrokes of the current debounce
interval in the webview. The editor's debounce is ≤ 500 ms (screenplay module contract).

**Save state** (FSD §3.3, `SaveTracker`, event `saveState`):

| Status | Meaning | Shown as |
|---|---|---|
| `Saving` | A mutation is in progress | "Saving…" |
| `Saved` | Last mutation committed. `lastSavedAt` is set. | "Saved" + time |
| `SavedPendingExternal` | Saved locally; an export/package/external operation is still running | "Saved · finishing export" |
| `Error` | A **storage** failure (`storage.*`: disk full, access denied, drive unavailable, busy) | Save-error banner with the human message. The failed change was rolled back and earlier work is intact. |

Validation, permission and conflict errors do not change the save state to `Error` (`SaveTracker::settle`).

**Explicit Save (Ctrl+S, `project.save`)** ✅: everything is already persisted. Save folds the WAL
(`wal_checkpoint(TRUNCATE)`) and writes a verified online backup to `recovery/checkpoint.sqlite`, the
"last confirmed saved state". The toast reads "Saved. Everything is stored on this computer."

## 3. Crash detection and recovery ✅

1. On open, `recovery/session.json` (`{pid, startedAt, appVersion}`) is written atomically. On clean close it is
   removed after checkpoint + backup. That work runs while the project is still the open one; only then is the
   project detached (which can wait a few seconds for its AI index worker), so by the time the app shows
   Application Home the close is already clean. One close runs at a time, and quitting waits for a close under
   way (`core_pipeline.rs::when_no_project_is_open_any_more_the_close_is_already_clean`,
   `::quitting_while_a_close_is_under_way_waits_for_it_and_offers_no_recovery`).
2. If the marker already exists when opening, the previous session ended abnormally. `project.open` returns a
   `RecoveryOffer {previousSessionStartedAt, lastAutosaveAt, hasCheckpoint, checkpointAt}` and the UI shows the
   recovery dialog (mock-up 016).
3. `project.resolve_recovery`:
   - `latest` (default): keep the current database, which contains every committed autosave. Nothing is lost.
   - `checkpoint`: close the session, save the current database as `backups/before-recovery-<ms>.sqlite` (verified),
     remove `-wal`/`-shm`, copy the checkpoint in via temp + rename, clear the marker, and reopen. This requires `Edit`.
4. SQLite itself recovers the WAL on the next open, so committed transactions are never lost by a crash.

Tested: `crash_offers_recovery_and_keeps_latest_autosave`, `corrupt_database_is_refused_without_modification`.

## 4. Undo/redo ✅

Specified in ADR-0005. Summary:

| Parameter | Value |
|---|---|
| Scope | Per store (project / Global Idea Vault) × per actor |
| Depth | 300 entries per actor per store |
| Typing coalescing | Same `coalesce_key` + actor within **4 s** → one step |
| Activity merge | Same action + target + actor within **60 s** (coalesced edits only) |
| Persistence | `sys_undo` in the store database, which survives restart and crash |
| Conflict | `conflict.undo`; the entry (and older undo entries, or all redo entries) is dropped and later work is kept |
| Not undoable | Permanent purge (`trash.purge`), project-level file operations (duplicate, delete to Recycle Bin, recovery) |

UI: toolbar Undo/Redo with human labels (`history.info`), Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z outside editable fields, and
toasts with an Undo link (`toast.undoable`). Inside editors, see ADR-0010.

## 5. Recently Deleted ✅

`trash.list` / `trash.restore` (`SoftDelete`) / `trash.purge` (`PermanentDelete`, confirmation in the UI, not
undoable). Restore returns the item to its previous parent and position, or to "Unassigned" with an explanation when
the container is gone (FSD §52.5). ~~There is no auto-purge and no retention timer.~~ (SUPERSEDED 2026-10-07.)

**30-day removal** (owner decision 2026-10-07, ADR-0012 amendment). `modules/retention.rs` removes an item for good 30 days
after its delete (`deleted_item.deleted_at`), exactly at the 30-day mark. It runs, with no network, when a project opens (the
project's Recently Deleted, which also holds the project's Idea Vault items) and when the Global Idea Vault first opens in a
session, which the desktop shell does at app start (after the team password on the internal builds). Each item is removed in
its own `Store::mutate` (quiet, not undoable), so a crash, a full drive or a kill leaves every item whole or gone and the next
open carries on; at most 500 items per run. Inside the transaction the record is read again and the item is removed only when
the registered table matches and the object there still has `deleted_at`: a live object behind a stale or crafted record is
never touched. A delete time later than now (clock turned back) counts as now: nothing is removed early and the screens show
30 days. A clock turned forward removes what is 30 days old by that clock. Both Recently Deleted screens show the whole days
left ("Removed in": 30 to 1 day left, `daysLeft` on `DeletedItemView` and `VaultTrashRow`); the Owner can still delete
permanently earlier. Private notes of another member are left in place (their owner's own app removes them). Tests:
`crates/openframe-application/tests/recently_deleted_purge.rs`.

## 6. Backups

| Backup | Status | Trigger | Location |
|---|---|---|---|
| Confirmed checkpoint | ✅ | Explicit Save, clean close | `recovery/checkpoint.sqlite` |
| Pre-migration safety copy | ✅ | Opening a project that needs a schema upgrade | `backups/pre-migration-v<from>-<ms>.sqlite` |
| Before-recovery copy | ✅ | Choosing "last confirmed save" | `backups/before-recovery-<ms>.sqlite` |
| Before-replace copy | 🚧 | Import "replace after backup" (identity collision) | `backups/` |
| Backup Package (user) | 🚧 | User action. The location is chosen by the user (FSD: "user-selected"). | `.zip` package, verified after write |

All database copies use the online backup API → `.partial` → `quick_check` → rename (`backup_to`). Backups inside
`backups/` are not pruned automatically today. 📋 Retention policy: keep the latest 5 pre-migration copies and any from
the last 30 days (needs product sign-off before implementation).

## 7. Failure matrix

| Failure | Behavior |
|---|---|
| Disk full during a mutation | Rollback → `storage.disk_full`, save state `Error`, previous state intact. Retry after freeing space. |
| Drive removed (external disk) | `storage.drive_unavailable` (Windows errors 21/55/1167). Project stays open read-only in practice. Reconnect and retry. |
| Access denied (e.g. read-only folder, AV lock) | `storage.access_denied` |
| DB busy beyond 5 s | `storage.busy` (retryable) |
| Corrupt database on open | Refused with no modification (`project_format.corrupt`); the user restores from a checkpoint or backup |
| App crash / power loss | WAL recovery on next open + recovery offer |
| Crash during migration | The transaction never committed, so the old schema is intact, and a safety copy exists |
| Checkpoint write fails on close | Logged; the next open offers recovery (the marker remains) |

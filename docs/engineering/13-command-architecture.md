# 13 — Rust Domain & Command Architecture

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned.
> Source: `crates/openframe-domain`, `crates/openframe-application/src/{core,registry,store,util,events}.rs`,
> `modules/*`. Module conventions for implementers: `docs/engineering/AGENT_BRIEF.md`.

## 1. Layers ✅

| Crate | Responsibility | May depend on |
|---|---|---|
| `openframe-domain` | Ids (UUIDv7), `now_ms`, `AppError` taxonomy, `Role`/`Capability`/`Actor`, canonical enums, format version constants. **No I/O.** | serde, uuid, time, ts-rs (rusqlite only for the `From` impl behind the `sqlite` feature) |
| `openframe-persistence` | Connection policy, migrations, undo capture, row helpers, backup/checkpoint/integrity | domain, rusqlite |
| `openframe-project-format` | Folder layout, manifest, lock, crash marker, atomic writes, default dirs | domain, security |
| `openframe-security` | Path confinement, safe ZIP, hashing, Ed25519, redaction | domain |
| `openframe-application` | `AppCore`, `Registry`, `Store` pipeline, modules | all of the above |
| `openframe-desktop` (`src-tauri`) | Adapter: `of_invoke`, asset scope, events, logging | application, domain, security |

## 2. AppCore ✅

`AppCore` (one per process, in an `Arc`) owns:

- `config: AppConfig`: `app_data_dir`, `projects_dir`, `global_vault_dir`, `app_version` (`for_current_user` or
  `isolated(root)` for tests);
- the app database connection (`app.sqlite`);
- `project: RwLock<Option<Arc<ProjectSession>>>`: at most one open project (layout, `Store`, manifest, recovery offer,
  lock);
- the lazily opened Global Idea Vault `Store`;
- `registry: Arc<Registry>`, `events: Arc<dyn EventSink>`, `save: Arc<SaveTracker>`, the `TaskManager`, and a typed
  service map (`service::<T>()`) for long-lived module services (e.g. the AI runtime supervisor).

`local_actor()` builds the `Actor` for UI calls: the local profile (`user_id`, `display_name`), the role from
`project_member` (default Owner), and origin `Local`.

## 3. Operations ✅

```rust
// registration (modules/<m>/mod.rs)
pub fn register(r: &mut Registry) {
    r.query("story.list_board", list_board);          // OpKind::Query
    r.command("story.move_card", move_card);          // OpKind::Command
    r.indexer("story_scene_card", index_card);        // search projection
    r.trash_handler(TrashHandler { object_type: "scene_card", table: "story_scene_card",
                                   label: "Scene Card", restore: Some(restore_card), purge: purge_card });
}
// handler shape
fn move_card(core: &AppCore, actor: &Actor, args: MoveCardArgs) -> AppResult<CardDto>;
```

- Names are `module.action`, unique (checked at startup), and form the allow-list (ADR-0004).
- `Args` structs: `#[derive(Deserialize, TS)] #[ts(export)] #[serde(rename_all="camelCase", deny_unknown_fields)]`.
  DTOs: `#[derive(Serialize, TS)]`. `i64` timestamps are typed `number` for TS.
- Handlers validate input first (`util::required_text`, `require_id`, …). Validation errors are human
  (`validation.required`: "Title is required.").

Registered foundation ops ✅:
`app.{info,set_display_name,save_state,cancel_task}`,
`project.{list_recent,create,open,close,current,home,update_settings,set_status,save,resolve_recovery,set_archived,
duplicate,delete,remove_recent,locate,set_last_location,members}`,
`history.{undo,redo,info,activity}`, `trash.{list,restore,purge}`, `search.{query,rebuild}`,
`files.{list,add,rename,move,update_notes,delete,relink,create_folder,rename_folder,delete_folder,asset}`.

## 4. The mutation pipeline ✅ (`Store::mutate`)

```text
Store::mutate(actor, MutationMeta{action, summary, cap, coalesce_key?, target?, undoable, record_activity}, |tx| …)
  1. actor.require(cap)                         → permission.denied (nothing opened, nothing written)
  2. SaveTracker: Saving                         → SaveState event (status bar)
  3. writer.lock(); BEGIN IMMEDIATE
  4. undo::reset_capture
  5. module closure f(&Tx): validation + SQL via tx.conn()
        tx.reindex(table,id)             extra search re-index for derived documents
        tx.created_file(path)            removed if the transaction rolls back
        tx.delete_file_after_commit(p)   permanent purge; deleted only after COMMIT
  6. changes = undo::drain_capture       net per-row before/after images
  7. search re-index of every touched row (+ extra) via registered indexers
  8. if changes non-empty: activity entry (merge window 60 s for coalesced edits) and undo entry
     (coalesce window 4 s; redo stack cleared; cap 300)
  9. COMMIT  — failure → rollback, created files removed
 10. purge deletions; release writer
 11. DataChanged{store, tables, ids, origin:"local"}; SaveTracker: Saved (or Error for storage.* failures)
```

Rules:

- **Only `Store::mutate` (and undo/redo) writes canonical data.** `with_writer` exists for infrastructure only
  (checkpoint, backup, integrity, ensuring the local member).
- **Queries** use `store.read(|conn| …)` on the read-only connection after `actor.require(Capability::View, …)`.
- `MutationMeta::not_undoable()` is for actions that cannot be reverted (permanent purge). `.quiet()` skips activity
  (view-only bookkeeping).
- `.coalesce(key)` for typing: same key + same actor within 4 s → one undo step, and one activity entry within 60 s.
- Cross-module effects happen **inside one transaction** when they are one user action (e.g. soft-delete + parent
  bookkeeping). Modules never call each other's handlers through the registry.

## 5. Recoverable delete ✅

`store::soft_delete(tx, DeleteSpec{object_type, table, id, title, parent_type, parent_id, position})` →
`deleted_at` set + `deleted_item` row. `trash.restore` calls the module's `restore` (or clears `deleted_at`) and must
return the item to its prior parent and position, or to "unassigned" with a message (FSD §52.5). `trash.purge`
(`PermanentDelete`, not undoable) calls the module's `purge`.

## 6. Background tasks ✅ (`AppCore::spawn_task`)

`spawn_task(kind, label, |core, handle| -> AppResult<Value>)` runs on a named thread (`of-task-<kind>`) and emits
`Task` events: `queued` → `running` (progress 0..1 + message) → `completed` (result) / `failed` (AppError) /
`cancelled`. `handle.check()?` honours cancellation (`app.cancel_task`). Panics are caught and reported as
`internal.unexpected`. All running tasks are cancelled on shutdown. Long tasks must not hold the writer lock across
slow I/O. They do the slow work first, then apply the result in one short `mutate`.

## 7. Errors ✅ (`openframe_domain::error`)

`AppError { code, message, detail?, retryable }` serializes to the UI. Code families (ESD §12):
`validation.*`, `permission.*`, `not_found.*`, `conflict.*` (`stale`, `undo`, `state`), `storage.*`
(`write_failed`, `disk_full`, `busy`, `drive_unavailable`, `access_denied`, `delete_failed`), `project_format.*`
(`invalid`, `corrupt`, `too_new`, `in_use`, `network_location`), `migration.*`, `import.*`, `export.*`, `ai.*`,
`network.*`, `security.*` (`unknown_operation`, `path_rejected`, `path_escape`, `signature_invalid`), `internal.*`
(`unexpected`, `cancelled`, `open_failed`).

- `message` is filmmaker-friendly and final. `detail` is technical and must never contain project content.
- `std::io::Error` and `rusqlite::Error` map to specific codes: disk full, access denied, drive unavailable, busy,
  corrupt, not found.
- The UI branches on `code` only (`OpError.is(prefix)`).

## 8. Events ✅

`AppEvent` (tagged `type`, camelCase): `dataChanged {store, tables, ids, origin}`, `saveState`, `projectOpened`,
`projectClosed`, `task`, `module {module, name, payload}`. They are delivered on the `of://event` channel by the Tauri
sink, recorded by `RecordingSink` in tests, and dropped by `NullSink`.

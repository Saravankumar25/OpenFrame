//! A transactional store (a project, or the Global Idea Vault) and the single
//! mutation pipeline every command goes through (ESD §6):
//!
//! permission → transaction → module logic (domain validation + writes) →
//! undo capture → search reindex → activity → commit → events → save state.
//!
//! Nothing else in the codebase opens a write transaction on a store.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::migrate::{self, Migration, MigrationPlan};
use openframe_persistence::undo::{self, Direction, RowChange, TableCatalog};
use parking_lot::{Mutex, RwLock};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use ts_rs::TS;

use crate::events::{AppEvent, EventSink, SaveState, SaveStatus, StoreKind};
use crate::registry::Registry;

/// Undo history kept per store and actor.
const UNDO_LIMIT: i64 = 300;
/// Consecutive edits with the same coalesce key within this window become one undo step.
const COALESCE_WINDOW_MS: i64 = 4_000;
/// Activity entries for the same action/target within this window are merged.
const ACTIVITY_MERGE_WINDOW_MS: i64 = 60_000;

/// Describes a mutation for permission checks, activity and undo.
#[derive(Debug, Clone)]
pub struct MutationMeta {
    /// Stable action code, e.g. `story.create_act`.
    pub action: &'static str,
    /// Human summary for activity/undo, e.g. `Added act “Act One”`.
    pub summary: String,
    pub cap: Capability,
    /// Phrase used in permission errors: "You don't have permission to {phrase}".
    pub cap_phrase: &'static str,
    pub coalesce_key: Option<String>,
    pub target: Option<(String, String)>,
    pub undoable: bool,
    pub record_activity: bool,
}

impl MutationMeta {
    pub fn new(action: &'static str, summary: impl Into<String>, cap: Capability) -> Self {
        let cap_phrase = match cap {
            Capability::Edit => "edit this project",
            Capability::SoftDelete => "delete items in this project",
            Capability::PermanentDelete => "permanently delete items",
            Capability::Comment => "comment in this project",
            Capability::ResolveComments => "resolve comments",
            Capability::LockOrFinalize => "lock or finalize documents",
            Capability::ManageProject => "change project settings",
            Capability::ManagePermissions => "change collaborator permissions",
            Capability::Import => "import into this project",
            Capability::ApplyChangeSet => "apply proposed changes",
            Capability::Export | Capability::CreatePackage => "export from this project",
            Capability::UseAi => "use the assistant",
            Capability::View => "view this project",
        };
        Self {
            action,
            summary: summary.into(),
            cap,
            cap_phrase,
            coalesce_key: None,
            target: None,
            undoable: true,
            record_activity: true,
        }
    }
    pub fn coalesce(mut self, key: impl Into<String>) -> Self {
        self.coalesce_key = Some(key.into());
        self
    }
    pub fn target(mut self, entity_type: &str, id: &str) -> Self {
        self.target = Some((entity_type.to_string(), id.to_string()));
        self
    }
    pub fn not_undoable(mut self) -> Self {
        self.undoable = false;
        self
    }
    pub fn quiet(mut self) -> Self {
        self.record_activity = false;
        self
    }
}

/// Save-state tracker shared by the UI status indicator (FSD §3.3).
pub struct SaveTracker {
    state: Mutex<SaveState>,
    events: Arc<dyn EventSink>,
}

impl SaveTracker {
    pub fn new(events: Arc<dyn EventSink>) -> Self {
        Self {
            state: Mutex::new(SaveState {
                status: SaveStatus::Saved,
                last_saved_at: None,
                error: None,
                pending_operations: 0,
            }),
            events,
        }
    }
    fn set(&self, f: impl FnOnce(&mut SaveState)) {
        let snapshot = {
            let mut s = self.state.lock();
            f(&mut s);
            s.clone()
        };
        self.events.emit(&AppEvent::SaveState(snapshot));
    }
    pub fn snapshot(&self) -> SaveState {
        self.state.lock().clone()
    }
    pub fn saving(&self) {
        self.set(|s| s.status = SaveStatus::Saving);
    }
    pub fn saved(&self) {
        self.set(|s| {
            s.status = if s.pending_operations > 0 {
                SaveStatus::SavedPendingExternal
            } else {
                SaveStatus::Saved
            };
            s.last_saved_at = Some(now_ms());
            s.error = None;
        });
    }
    /// A mutation failed without a storage problem (validation etc.).
    pub fn settle(&self) {
        self.set(|s| {
            if s.status == SaveStatus::Saving {
                s.status = if s.error.is_some() {
                    SaveStatus::Error
                } else if s.pending_operations > 0 {
                    SaveStatus::SavedPendingExternal
                } else {
                    SaveStatus::Saved
                };
            }
        });
    }
    pub fn failed(&self, err: &AppError) {
        let err = err.clone();
        self.set(|s| {
            s.status = SaveStatus::Error;
            s.error = Some(err);
        });
    }
    pub fn external_started(&self) {
        self.set(|s| {
            s.pending_operations += 1;
            if s.status == SaveStatus::Saved {
                s.status = SaveStatus::SavedPendingExternal;
            }
        });
    }
    pub fn external_finished(&self) {
        self.set(|s| {
            s.pending_operations = s.pending_operations.saturating_sub(1);
            if s.pending_operations == 0 && s.status == SaveStatus::SavedPendingExternal {
                s.status = SaveStatus::Saved;
            }
        });
    }
}

struct Writer {
    conn: Connection,
    catalog: TableCatalog,
}

/// Told about every committed change of a store, AFTER commit and after the
/// writer lock is released (mutations, undo/redo, restore, purge, package
/// apply). For derived indexes only (AI intelligence index): implementations
/// must just enqueue work — never block, never touch the canonical writer.
pub trait CommitObserver: Send + Sync {
    fn committed(&self, changes: &[RowChange], extra: &[(String, String)]);
}

/// The per-transaction context handed to module code.
pub struct Tx<'a> {
    conn: &'a Connection,
    actor: &'a Actor,
    store: &'a Store,
    catalog: &'a TableCatalog,
    extra_reindex: RefCell<BTreeSet<(String, String)>>,
    rollback_files: RefCell<Vec<PathBuf>>,
    commit_deletes: RefCell<Vec<PathBuf>>,
}

impl<'a> Tx<'a> {
    pub fn conn(&self) -> &Connection {
        self.conn
    }
    pub fn actor(&self) -> &Actor {
        self.actor
    }
    pub fn store(&self) -> &Store {
        self.store
    }
    pub fn root(&self) -> &Path {
        &self.store.root
    }
    pub fn kind(&self) -> StoreKind {
        self.store.kind
    }
    pub fn registry(&self) -> &Registry {
        &self.store.registry
    }
    /// Column metadata for tracked tables (used by package import to write validated rows).
    pub fn catalog(&self) -> &TableCatalog {
        self.catalog
    }
    /// Ask for a search re-index of a row whose document depends on other rows.
    pub fn reindex(&self, table: &str, id: &str) {
        self.extra_reindex
            .borrow_mut()
            .insert((table.to_string(), id.to_string()));
    }
    /// A file created during this mutation; removed if the transaction rolls back.
    pub fn created_file(&self, path: PathBuf) {
        self.rollback_files.borrow_mut().push(path);
    }
    /// A file to delete only after the transaction commits (permanent purge).
    pub fn delete_file_after_commit(&self, path: PathBuf) {
        self.commit_deletes.borrow_mut().push(path);
    }
}

/// A row of `deleted_item`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DeletedItemRow {
    pub id: String,
    pub object_type: String,
    pub object_id: String,
    pub table_name: String,
    pub title: Option<String>,
    pub parent_type: Option<String>,
    pub parent_id: Option<String>,
    #[ts(type = "number | null")]
    pub position: Option<i64>,
    #[ts(type = "number")]
    pub deleted_at: i64,
    pub deleted_by: Option<String>,
    /// Set after a restore when the object could not return to its original place (FSD §52.5).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restored_note: Option<String>,
}

pub struct DeleteSpec<'s> {
    pub object_type: &'s str,
    pub table: &'s str,
    pub id: &'s str,
    pub title: Option<String>,
    pub parent_type: Option<&'s str>,
    pub parent_id: Option<String>,
    pub position: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct UndoInfo {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_label: Option<String>,
    pub redo_label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OpenReport {
    pub migrated_from: Option<u32>,
    pub schema_version: u32,
    pub safety_backup: Option<PathBuf>,
}

pub struct Store {
    pub kind: StoreKind,
    root: PathBuf,
    db_path: PathBuf,
    writer: Mutex<Writer>,
    reader: Mutex<Connection>,
    registry: Arc<Registry>,
    events: Arc<dyn EventSink>,
    save: Arc<SaveTracker>,
    observer: RwLock<Option<Arc<dyn CommitObserver>>>,
}

impl Store {
    /// Open an existing or new store database, migrating safely:
    /// too-new → refuse; upgrade → verified safety backup first, then one
    /// atomic migration transaction; corrupt → refuse without modification.
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        kind: StoreKind,
        root: &Path,
        db_path: &Path,
        migrations: &[Migration],
        create: bool,
        safety_backup_dir: &Path,
        registry: Arc<Registry>,
        events: Arc<dyn EventSink>,
        save: Arc<SaveTracker>,
    ) -> AppResult<(Store, OpenReport)> {
        if !create && !db_path.is_file() {
            return Err(AppError::project_format(
                "This project's data file is missing.",
            ));
        }
        let mut conn = openframe_persistence::open_connection(db_path, create)?;
        if !create {
            let problems = openframe_persistence::integrity_problems(&conn, false)?;
            if !problems.is_empty() {
                return Err(AppError::new(
                    "project_format.corrupt",
                    "This project's data file is damaged. OpenFrame did not change it. You can restore it from a backup or recovery point.",
                )
                .with_detail(problems.join("; ")));
            }
            // A project folder may come from anywhere: refuse schemas carrying code
            // (triggers/views) OpenFrame didn't create, or hostile object names (PKG-03).
            let sql: Vec<&str> = migrations.iter().map(|m| m.sql).collect();
            let problems = openframe_persistence::untrusted_schema_problems(&conn, &sql)?;
            if !problems.is_empty() {
                return Err(AppError::new(
                    "project_format.untrusted_schema",
                    "This project's data file contains content OpenFrame doesn't create, so it was not opened. OpenFrame did not change it.",
                )
                .with_detail(problems.join("; ")));
            }
        }
        let mut report = OpenReport {
            migrated_from: None,
            schema_version: 0,
            safety_backup: None,
        };
        match migrate::plan(&conn, migrations)? {
            MigrationPlan::TooNew { found, supported } => {
                return Err(migrate::too_new_error(found, supported));
            }
            MigrationPlan::Fresh { .. } if !create => {
                return Err(AppError::project_format(
                    "This project's data file is empty or is not an OpenFrame project.",
                ));
            }
            MigrationPlan::Upgrade { from, .. } => {
                std::fs::create_dir_all(safety_backup_dir)?;
                let dest =
                    safety_backup_dir.join(format!("pre-migration-v{from}-{}.sqlite", now_ms()));
                openframe_persistence::backup_to(&conn, &dest)?;
                report.safety_backup = Some(dest);
                report.migrated_from = Some(from);
                migrate::apply(&mut conn, migrations)?;
            }
            _ => {
                migrate::apply(&mut conn, migrations)?;
            }
        }
        report.schema_version = migrate::current_version(&conn)?;
        let catalog = undo::install_capture(&conn)?;
        let reader = openframe_persistence::open_connection(db_path, false)?;
        reader.execute_batch("PRAGMA query_only=ON;")?;
        Ok((
            Store {
                kind,
                root: root.to_path_buf(),
                db_path: db_path.to_path_buf(),
                writer: Mutex::new(Writer { conn, catalog }),
                reader: Mutex::new(reader),
                registry,
                events,
                save,
                observer: RwLock::new(None),
            },
            report,
        ))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
    pub fn save_tracker(&self) -> &SaveTracker {
        &self.save
    }
    pub fn events(&self) -> &Arc<dyn EventSink> {
        &self.events
    }
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Install (or remove) the post-commit observer.
    pub fn set_commit_observer(&self, observer: Option<Arc<dyn CommitObserver>>) {
        *self.observer.write() = observer;
    }

    /// Run a read-only query on the reader connection (never blocks the writer).
    pub fn read<R>(&self, f: impl FnOnce(&Connection) -> AppResult<R>) -> AppResult<R> {
        let conn = self.reader.lock();
        f(&conn)
    }

    /// Exclusive raw access to the writer connection for infrastructure work
    /// (backup, checkpoint, integrity checks). Never used for canonical mutations.
    pub fn with_writer<R>(&self, f: impl FnOnce(&Connection) -> AppResult<R>) -> AppResult<R> {
        let w = self.writer.lock();
        f(&w.conn)
    }

    /// THE mutation pipeline. See module docs.
    pub fn mutate<R>(
        &self,
        actor: &Actor,
        meta: MutationMeta,
        f: impl FnOnce(&Tx<'_>) -> AppResult<R>,
    ) -> AppResult<R> {
        actor.require(meta.cap, meta.cap_phrase)?;
        self.save.saving();
        let result = self.mutate_inner(actor, &meta, f);
        match &result {
            Ok(_) => self.save.saved(),
            Err(e) if e.is("storage") => {
                tracing::error!(
                    code = e.code_str(),
                    action = meta.action,
                    "mutation failed to persist"
                );
                self.save.failed(e)
            }
            Err(_) => self.save.settle(),
        }
        result
    }

    fn mutate_inner<R>(
        &self,
        actor: &Actor,
        meta: &MutationMeta,
        f: impl FnOnce(&Tx<'_>) -> AppResult<R>,
    ) -> AppResult<R> {
        let mut w = self.writer.lock();
        let Writer { conn, catalog } = &mut *w;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        undo::reset_capture(&tx)?;
        let ctx = Tx {
            conn: &tx,
            actor,
            store: self,
            catalog,
            extra_reindex: RefCell::new(BTreeSet::new()),
            rollback_files: RefCell::new(Vec::new()),
            commit_deletes: RefCell::new(Vec::new()),
        };
        let outcome = (|| -> AppResult<(R, Vec<RowChange>)> {
            let out = f(&ctx)?;
            let changes = undo::drain_capture(&tx)?;
            let mut rows: BTreeSet<(String, String)> = changes
                .iter()
                .map(|c| (c.tbl.clone(), c.row_id.clone()))
                .collect();
            rows.extend(ctx.extra_reindex.borrow().iter().cloned());
            reindex_rows(&tx, &self.registry, &rows)?;
            if !changes.is_empty() {
                if meta.record_activity {
                    record_activity(&tx, actor, meta)?;
                }
                if meta.undoable {
                    push_undo(&tx, actor, meta, &changes)?;
                }
            }
            Ok((out, changes))
        })();
        let rollback_files = ctx.rollback_files.take();
        let commit_deletes = ctx.commit_deletes.take();
        let extra: Vec<(String, String)> = ctx.extra_reindex.take().into_iter().collect();
        drop(ctx);
        match outcome {
            Ok((out, changes)) => {
                if let Err(e) = tx.commit() {
                    cleanup(&rollback_files);
                    return Err(e.into());
                }
                for p in commit_deletes {
                    if let Err(e) = remove_path(&p) {
                        // Log the OS error kind only: the path contains the project title.
                        tracing::warn!(kind = ?e.kind(), "could not remove purged file; it was left in place");
                    }
                }
                drop(w);
                self.emit_changed(&changes, &extra, "local");
                Ok(out)
            }
            Err(e) => {
                drop(tx);
                cleanup(&rollback_files);
                Err(e)
            }
        }
    }

    fn emit_changed(&self, changes: &[RowChange], extra: &[(String, String)], origin: &str) {
        let observer = self.observer.read().clone();
        if let Some(o) = observer
            && (!changes.is_empty() || !extra.is_empty())
        {
            o.committed(changes, extra);
        }
        let mut tables = undo::touched_tables(changes);
        let mut ids: Vec<String> = changes.iter().map(|c| c.row_id.clone()).collect();
        for (t, id) in extra {
            if !tables.contains(t) {
                tables.push(t.clone());
            }
            ids.push(id.clone());
        }
        if tables.is_empty() {
            return;
        }
        ids.sort();
        ids.dedup();
        self.events.emit(&AppEvent::DataChanged {
            store: self.kind,
            tables,
            ids,
            origin: origin.to_string(),
        });
    }

    // ---------------------------------------------------------------- undo/redo

    pub fn undo_info(&self, actor: &Actor) -> AppResult<UndoInfo> {
        self.read(|c| {
            let undo: Option<String> = c
                .query_row(
                    "SELECT label FROM sys_undo WHERE state='done' AND actor_id IS ?1 ORDER BY seq DESC LIMIT 1",
                    [&actor.user_id],
                    |r| r.get(0),
                )
                .optional()?;
            let redo: Option<String> = c
                .query_row(
                    "SELECT label FROM sys_undo WHERE state='undone' AND actor_id IS ?1 ORDER BY seq ASC LIMIT 1",
                    [&actor.user_id],
                    |r| r.get(0),
                )
                .optional()?;
            Ok(UndoInfo { can_undo: undo.is_some(), can_redo: redo.is_some(), undo_label: undo, redo_label: redo })
        })
    }

    /// Undo the actor's most recent step. Returns the step label, or None if nothing to undo.
    pub fn undo(&self, actor: &Actor) -> AppResult<Option<String>> {
        self.step(actor, Direction::Undo)
    }

    pub fn redo(&self, actor: &Actor) -> AppResult<Option<String>> {
        self.step(actor, Direction::Redo)
    }

    fn step(&self, actor: &Actor, dir: Direction) -> AppResult<Option<String>> {
        // Undo only replays the actor's own steps, but read-only roles never mutate at all.
        if !(actor.can(Capability::Edit) || actor.can(Capability::Comment)) {
            return Err(AppError::permission_denied("undo changes"));
        }
        self.save.saving();
        let result = self.step_inner(actor, dir);
        match &result {
            Ok(_) => self.save.saved(),
            Err(e) if e.is("storage") => self.save.failed(e),
            Err(_) => self.save.settle(),
        }
        result
    }

    fn step_inner(&self, actor: &Actor, dir: Direction) -> AppResult<Option<String>> {
        let mut w = self.writer.lock();
        let Writer { conn, catalog } = &mut *w;
        let (state, order) = match dir {
            Direction::Undo => ("done", "DESC"),
            Direction::Redo => ("undone", "ASC"),
        };
        let entry: Option<(i64, String, String)> = conn
            .query_row(
                &format!(
                    "SELECT seq, label, changes_json FROM sys_undo WHERE state=?1 AND actor_id IS ?2 ORDER BY seq {order} LIMIT 1"
                ),
                params![state, actor.user_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((seq, label, json)) = entry else {
            return Ok(None);
        };
        let changes: Vec<RowChange> =
            serde_json::from_str(&json).map_err(|e| AppError::internal(e.to_string()))?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch("PRAGMA defer_foreign_keys=ON;")?;
        undo::reset_capture(&tx)?;
        if let Err(e) = undo::apply(&tx, catalog, &changes, dir) {
            drop(tx);
            if e.is("conflict") {
                // Never overwrite later work. Drop this step (and, for undo, everything
                // older) so the remaining history stays consistent.
                let sql = match dir {
                    Direction::Undo => {
                        "DELETE FROM sys_undo WHERE state='done' AND actor_id IS ?2 AND seq <= ?1"
                    }
                    Direction::Redo => {
                        "DELETE FROM sys_undo WHERE state='undone' AND actor_id IS ?2 AND ?1 = ?1"
                    }
                };
                conn.execute(sql, params![seq, actor.user_id])?;
                return Err(AppError::new(
                    "conflict.undo",
                    "This step can't be undone because the same content was changed afterwards. Your latest work was kept.",
                ));
            }
            return Err(e);
        }
        let applied = undo::drain_capture(&tx)?;
        let new_state = match dir {
            Direction::Undo => "undone",
            Direction::Redo => "done",
        };
        tx.execute(
            "UPDATE sys_undo SET state=?1, updated_at=?2 WHERE seq=?3",
            params![new_state, now_ms(), seq],
        )?;
        let rows: BTreeSet<(String, String)> = applied
            .iter()
            .map(|c| (c.tbl.clone(), c.row_id.clone()))
            .collect();
        reindex_rows(&tx, &self.registry, &rows)?;
        let verb = if dir == Direction::Undo {
            "Undid"
        } else {
            "Redid"
        };
        let meta = MutationMeta::new(
            if dir == Direction::Undo {
                "history.undo"
            } else {
                "history.redo"
            },
            format!("{verb}: {label}"),
            Capability::View,
        );
        // Private notes never produce Activity (it is visible to every project member):
        // undoing/redoing one must not reveal that a note exists or when it changed (PN-01).
        if !rows.iter().any(|(t, _)| t == "private_note") {
            record_activity(&tx, actor, &meta)?;
        }
        tx.commit()?;
        drop(w);
        self.emit_changed(&applied, &[], "local");
        Ok(Some(label))
    }

    /// Rebuild the whole search index from canonical tables (the index is never canonical).
    pub fn rebuild_search_index(&self) -> AppResult<usize> {
        let mut w = self.writer.lock();
        let tx = w
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM search_doc", [])?;
        let mut rows = BTreeSet::new();
        for table in self.registry.indexed_tables() {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [table],
                |r| r.get(0),
            )?;
            if !exists {
                continue;
            }
            let mut stmt = tx.prepare(&format!("SELECT id FROM \"{table}\""))?;
            let ids: Vec<String> = stmt
                .query_map([], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            for id in ids {
                rows.insert((table.to_string(), id));
            }
        }
        undo::reset_capture(&tx)?;
        reindex_rows(&tx, &self.registry, &rows)?;
        let n: i64 = tx.query_row("SELECT count(*) FROM search_doc", [], |r| r.get(0))?;
        tx.commit()?;
        Ok(n as usize)
    }
}

fn cleanup(paths: &[PathBuf]) {
    for p in paths {
        let _ = remove_path(p);
    }
}

fn remove_path(p: &Path) -> std::io::Result<()> {
    if p.is_dir() {
        std::fs::remove_dir_all(p)
    } else if p.exists() {
        std::fs::remove_file(p)
    } else {
        Ok(())
    }
}

fn reindex_rows(
    conn: &Connection,
    registry: &Registry,
    rows: &BTreeSet<(String, String)>,
) -> AppResult<()> {
    for (table, id) in rows {
        let Some(indexer) = registry.indexer_for(table) else {
            continue;
        };
        match indexer(conn, id)? {
            Some(doc) => {
                conn.execute(
                    "INSERT INTO search_doc(source_table, entity_id, entity_type, title, body, context, nav_json, owner_user_id, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT(source_table, entity_id) DO UPDATE SET
                        entity_type=excluded.entity_type, title=excluded.title, body=excluded.body,
                        context=excluded.context, nav_json=excluded.nav_json,
                        owner_user_id=excluded.owner_user_id, updated_at=excluded.updated_at",
                    params![
                        table,
                        id,
                        doc.entity_type,
                        doc.title,
                        doc.body,
                        doc.context,
                        doc.nav.to_string(),
                        doc.owner_user_id,
                        now_ms()
                    ],
                )?;
            }
            None => {
                conn.execute(
                    "DELETE FROM search_doc WHERE source_table=?1 AND entity_id=?2",
                    params![table, id],
                )?;
            }
        }
    }
    Ok(())
}

fn record_activity(conn: &Connection, actor: &Actor, meta: &MutationMeta) -> AppResult<()> {
    let now = now_ms();
    let (tt, tid) = meta.target.clone().unzip();
    let origin = match &actor.origin {
        openframe_domain::auth::ActorOrigin::Local => "local",
        openframe_domain::auth::ActorOrigin::Exchange { .. } => "exchange",
        openframe_domain::auth::ActorOrigin::Ai { .. } => "ai",
    };
    let last: Option<(String, String, Option<String>, Option<String>, i64)> = conn
        .query_row(
            "SELECT id, action, target_id, actor_id, at FROM sys_activity ORDER BY at DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    if let Some((id, action, target_id, actor_id, at)) = last
        && meta.coalesce_key.is_some()
        && action == meta.action
        && target_id == tid
        && actor_id.as_deref() == Some(actor.user_id.as_str())
        && now - at < ACTIVITY_MERGE_WINDOW_MS
    {
        conn.execute(
            "UPDATE sys_activity SET at=?1, summary=?2 WHERE id=?3",
            params![now, meta.summary, id],
        )?;
        return Ok(());
    }
    conn.execute(
        "INSERT INTO sys_activity(id, at, actor_id, actor_name, action, summary, target_type, target_id, origin)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![new_id(), now, actor.user_id, actor.display_name, meta.action, meta.summary, tt, tid, origin],
    )?;
    Ok(())
}

fn push_undo(
    conn: &Connection,
    actor: &Actor,
    meta: &MutationMeta,
    changes: &[RowChange],
) -> AppResult<()> {
    let now = now_ms();
    conn.execute(
        "DELETE FROM sys_undo WHERE state='undone' AND actor_id IS ?1",
        [&actor.user_id],
    )?;
    if let Some(key) = &meta.coalesce_key {
        let top: Option<(i64, Option<String>, String, i64)> = conn
            .query_row(
                "SELECT seq, coalesce_key, changes_json, updated_at FROM sys_undo
                 WHERE state='done' AND actor_id IS ?1 ORDER BY seq DESC LIMIT 1",
                [&actor.user_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        if let Some((seq, Some(top_key), json, updated_at)) = top
            && &top_key == key
            && now - updated_at < COALESCE_WINDOW_MS
        {
            let prev: Vec<RowChange> =
                serde_json::from_str(&json).map_err(|e| AppError::internal(e.to_string()))?;
            let merged = undo::merge(prev, changes.to_vec());
            if merged.is_empty() {
                conn.execute("DELETE FROM sys_undo WHERE seq=?1", [seq])?;
            } else {
                let json = serde_json::to_string(&merged)
                    .map_err(|e| AppError::internal(e.to_string()))?;
                conn.execute(
                    "UPDATE sys_undo SET changes_json=?1, updated_at=?2 WHERE seq=?3",
                    params![json, now, seq],
                )?;
            }
            return Ok(());
        }
    }
    let json = serde_json::to_string(changes).map_err(|e| AppError::internal(e.to_string()))?;
    conn.execute(
        "INSERT INTO sys_undo(label, actor_id, coalesce_key, changes_json, state, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'done', ?5, ?5)",
        params![meta.summary, actor.user_id, meta.coalesce_key, json, now],
    )?;
    conn.execute(
        "DELETE FROM sys_undo WHERE actor_id IS ?1 AND seq NOT IN
            (SELECT seq FROM sys_undo WHERE actor_id IS ?1 ORDER BY seq DESC LIMIT ?2)",
        params![actor.user_id, UNDO_LIMIT],
    )?;
    Ok(())
}

// ------------------------------------------------------------ recoverable delete

/// Move an object into the recoverable deleted state (FSD §52.1). The row keeps
/// its identity; `deleted_item` remembers where it lived so restore can return it.
pub fn soft_delete(tx: &Tx<'_>, spec: DeleteSpec<'_>) -> AppResult<()> {
    let now = now_ms();
    let n = tx.conn().execute(
        &format!(
            "UPDATE \"{}\" SET deleted_at=?1, updated_at=?1, rev=rev+1 WHERE id=?2 AND deleted_at IS NULL",
            spec.table.replace('"', "")
        ),
        params![now, spec.id],
    )?;
    if n == 0 {
        return Err(AppError::not_found("item"));
    }
    tx.conn().execute(
        "INSERT INTO deleted_item(id, object_type, object_id, table_name, title, parent_type, parent_id, position,
                                  deleted_by, deleted_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?10)
         ON CONFLICT(table_name, object_id) DO UPDATE SET deleted_at=excluded.deleted_at, title=excluded.title,
             parent_type=excluded.parent_type, parent_id=excluded.parent_id, position=excluded.position,
             deleted_by=excluded.deleted_by, updated_at=excluded.updated_at, rev=rev+1",
        params![
            new_id(),
            spec.object_type,
            spec.id,
            spec.table,
            spec.title,
            spec.parent_type,
            spec.parent_id,
            spec.position,
            tx.actor().user_id,
            now
        ],
    )?;
    Ok(())
}

pub fn load_deleted(conn: &Connection, deleted_id: &str) -> AppResult<DeletedItemRow> {
    conn.query_row(
        "SELECT id, object_type, object_id, table_name, title, parent_type, parent_id, position, deleted_at, deleted_by
         FROM deleted_item WHERE id=?1",
        [deleted_id],
        |r| {
            Ok(DeletedItemRow {
                id: r.get(0)?,
                object_type: r.get(1)?,
                object_id: r.get(2)?,
                table_name: r.get(3)?,
                title: r.get(4)?,
                parent_type: r.get(5)?,
                parent_id: r.get(6)?,
                position: r.get(7)?,
                deleted_at: r.get(8)?,
                deleted_by: r.get(9)?,
                restored_note: None,
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("deleted item"))
}

/// Detect (generically) whether a restored object landed somewhere other than
/// where it was deleted from, so the user can be told (FSD §52.5).
fn relocation_note(conn: &Connection, row: &DeletedItemRow) -> AppResult<Option<String>> {
    let table = row.table_name.replace('"', "");
    let cols: Vec<String> = {
        let mut stmt = conn.prepare(&format!(
            "SELECT name FROM pragma_table_info('{}')",
            table.replace('\'', "''")
        ))?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    if cols.iter().any(|c| c == "parent_type") {
        let pt: Option<String> = conn
            .query_row(
                &format!("SELECT parent_type FROM \"{table}\" WHERE id=?1"),
                [&row.object_id],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        if pt.as_deref() == Some("unassigned") && row.parent_type.as_deref() != Some("unassigned") {
            return Ok(Some(
                "Its original place no longer exists, so it was restored to Unassigned.".into(),
            ));
        }
    }
    if row.parent_id.is_some() {
        for col in [
            "parent_id",
            "folder_id",
            "act_id",
            "sequence_id",
            "storyboard_id",
            "moodboard_id",
        ] {
            if cols.iter().any(|c| c == col) {
                let v: Option<String> = conn
                    .query_row(
                        &format!("SELECT \"{col}\" FROM \"{table}\" WHERE id=?1"),
                        [&row.object_id],
                        |r| r.get(0),
                    )
                    .optional()?
                    .flatten();
                if v.is_none() {
                    return Ok(Some(
                        "Its original place no longer exists, so it was restored to the top level."
                            .into(),
                    ));
                }
                break;
            }
        }
    }
    Ok(None)
}

/// Restore a recoverably deleted object (FSD §52.5).
pub fn restore_deleted(tx: &Tx<'_>, deleted_id: &str) -> AppResult<DeletedItemRow> {
    let row = load_deleted(tx.conn(), deleted_id)?;
    // The deleted_item row is data (a received project can carry any): the table comes
    // from the registered handler, and must match the row — otherwise a crafted row could
    // bypass a module's own restore checks (e.g. private-note ownership).
    let handler = tx
        .registry()
        .trash_for(&row.object_type)
        .filter(|h| h.table == row.table_name)
        .ok_or_else(|| AppError::not_found("deleted item"))?;
    match handler.restore {
        Some(restore) => restore(tx, &row)?,
        None => {
            tx.conn().execute(
                &format!(
                    "UPDATE \"{}\" SET deleted_at=NULL, updated_at=?1, rev=rev+1 WHERE id=?2",
                    handler.table.replace('"', "\"\"")
                ),
                params![now_ms(), row.object_id],
            )?;
        }
    }
    tx.conn()
        .execute("DELETE FROM deleted_item WHERE id=?1", [deleted_id])?;
    let mut row = row;
    row.restored_note = relocation_note(tx.conn(), &row)?;
    Ok(row)
}

/// Permanently remove a deleted object (FSD §52.2 — the deliberate second action).
pub fn purge_deleted(tx: &Tx<'_>, deleted_id: &str) -> AppResult<DeletedItemRow> {
    let row = load_deleted(tx.conn(), deleted_id)?;
    let handler = tx
        .registry()
        .trash_for(&row.object_type)
        .ok_or_else(|| AppError::internal(format!("no purge handler for {}", row.object_type)))?;
    (handler.purge)(tx, &row)?;
    tx.conn()
        .execute("DELETE FROM deleted_item WHERE id=?1", [deleted_id])?;
    Ok(row)
}

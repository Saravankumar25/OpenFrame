//! Incremental, background intelligence indexing (§15, §32).
//!
//! ```text
//! Store::mutate ─► commit ─► CommitObserver::committed (row images, O(changes), no I/O)
//!                                  │ enqueue (coalesced by document key)
//!                                  ▼
//!                    ProjectIndex queue ── debounce (default 750 ms, max 5 s)
//!                                  ▼
//!   worker thread: own read-only connection to project.sqlite ─► build documents/graph
//!                  own write connection to cache/intelligence.sqlite ─► upsert (hash-guarded)
//!                  then embeddings in small batches (never while holding any project lock)
//! ```
//!
//! The canonical writer transaction is never held or waited on: a save always
//! succeeds even when indexing fails, and indexing failures only degrade
//! retrieval (FTS + SQL fallbacks).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use openframe_domain::{AppError, AppResult, now_ms};
use openframe_persistence::undo::RowChange;
use openframe_search::graph::{self, node_id};
use openframe_search::{
    Embedder, IndexCounts, IndexIdentity, IntelligenceDb, SearchError, SqliteVecIndex, docs, embed,
};
use parking_lot::{Condvar, Mutex};
use rusqlite::Connection;

use super::documents::{self, DocKey, Invalidation};
use crate::core::AppCore;
use crate::modules::ai::retrieval::IndexState;
use crate::registry::Registry;
use crate::store::CommitObserver;

/// Tuning knobs (tests shorten the debounce).
#[derive(Debug, Clone)]
pub struct IndexSettings {
    /// Quiet period after the last edit before re-indexing (coalesces typing).
    pub debounce: Duration,
    /// Upper bound on how long a change may wait during continuous editing.
    pub max_latency: Duration,
    /// Documents per intelligence-index transaction.
    pub build_batch: usize,
    /// Chunks per embedding call.
    pub embed_batch: usize,
    /// Pause before retrying after the embedding runtime failed.
    pub embed_retry: Duration,
}

impl Default for IndexSettings {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(750),
            max_latency: Duration::from_secs(5),
            build_batch: 64,
            embed_batch: 16,
            embed_retry: Duration::from_secs(30),
        }
    }
}

pub fn index_error(e: SearchError) -> AppError {
    AppError::ai(
        "index_unavailable",
        "Project context for AI is being prepared. AI can still answer some questions meanwhile.",
    )
    .with_detail(e.to_string())
}

#[derive(Debug, Default)]
struct Shared {
    pending: Invalidation,
    first_change: Option<Instant>,
    last_change: Option<Instant>,
    full: bool,
    clear: bool,
    activated: bool,
    opened: bool,
    /// Built from scratch (fresh/recreated file or explicit rebuild) and not finished yet.
    rebuilding: bool,
    running: bool,
    failed: Option<String>,
    unavailable: Option<String>,
    counts: IndexCounts,
    pending_embeddings: u64,
    embedder_present: bool,
    embed_retry_at: Option<Instant>,
    embedding_model: Option<String>,
    progress_done: u64,
    progress_total: u64,
    cycles: u64,
    last_cycle_ms: Option<u64>,
}

/// Point-in-time view of a project's intelligence index.
#[derive(Debug, Clone)]
pub struct IndexSnapshot {
    pub state: IndexState,
    pub counts: IndexCounts,
    pub pending_changes: u64,
    pub pending_embeddings: u64,
    /// Vectors exist for the current embedding model and nothing waits for one.
    pub semantic_ready: bool,
    pub embedder_present: bool,
    pub embedding_model: Option<String>,
    pub progress: Option<f64>,
    pub cycles: u64,
    pub last_cycle_ms: Option<u64>,
    pub error: Option<String>,
}

/// The intelligence index of one open project.
pub struct ProjectIndex {
    pub project_id: String,
    project_db: PathBuf,
    index_path: PathBuf,
    shared: Mutex<Shared>,
    cond: Condvar,
    reader: Mutex<Option<Connection>>,
    stop: AtomicBool,
    worker: Mutex<Option<JoinHandle<()>>>,
}

struct Job {
    clear: bool,
    full: bool,
    inv: Invalidation,
}

impl ProjectIndex {
    pub fn new(project_id: String, project_db: PathBuf, index_path: PathBuf) -> Self {
        Self {
            project_id,
            project_db,
            index_path,
            shared: Mutex::new(Shared::default()),
            cond: Condvar::new(),
            reader: Mutex::new(None),
            stop: AtomicBool::new(false),
            worker: Mutex::new(None),
        }
    }

    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    pub fn is_activated(&self) -> bool {
        self.shared.lock().activated
    }

    /// Record invalidated documents (coalesced; processed after the debounce).
    pub fn enqueue(&self, inv: Invalidation) {
        if inv.is_empty() {
            return;
        }
        let mut s = self.shared.lock();
        if !s.activated {
            // Not started yet: activation reconciles everything against canonical
            // data anyway, so nothing needs to be remembered (no unbounded growth
            // for users who never use AI).
            return;
        }
        s.pending.merge(inv);
        let now = Instant::now();
        s.first_change.get_or_insert(now);
        s.last_change = Some(now);
        drop(s);
        self.cond.notify_all();
    }

    /// Re-check everything against canonical data (hash-guarded; cheap when current).
    pub fn request_full(&self) {
        self.shared.lock().full = true;
        self.cond.notify_all();
    }

    /// Discard every derived row and rebuild from scratch (explicit "rebuild").
    pub fn request_rebuild(&self) {
        let mut s = self.shared.lock();
        s.clear = true;
        s.full = true;
        s.rebuilding = true;
        s.failed = None;
        s.embed_retry_at = None;
        drop(s);
        self.cond.notify_all();
    }

    /// Start the background worker (idempotent).
    pub fn activate(self: &Arc<Self>, core: &AppCore) {
        {
            let mut s = self.shared.lock();
            if s.activated {
                return;
            }
            s.activated = true;
            s.full = true;
        }
        let me = self.clone();
        let weak_core = Arc::downgrade(&core.arc());
        let registry = core.registry.clone();
        let spawned = std::thread::Builder::new()
            .name("of-intelligence-index".into())
            .spawn(move || run(me, weak_core, registry));
        match spawned {
            Ok(h) => *self.worker.lock() = Some(h),
            Err(e) => {
                tracing::error!(error = %e, "could not start the intelligence indexer");
                self.shared.lock().unavailable = Some("worker could not start".into());
            }
        }
    }

    /// Stop the worker and wait (bounded) until it released every file handle.
    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.cond.notify_all();
        let handle = self.worker.lock().take();
        if let Some(h) = handle {
            let deadline = Instant::now() + Duration::from_secs(20);
            while !h.is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            if h.is_finished() {
                let _ = h.join();
            } else {
                tracing::warn!(
                    "intelligence indexer did not stop in time; it will exit on its own"
                );
            }
        }
        *self.reader.lock() = None;
    }

    /// Run `f` on the retrieval connection to `intelligence.sqlite`.
    pub fn with_reader<R>(
        &self,
        f: impl FnOnce(&Connection) -> Result<R, SearchError>,
    ) -> Result<R, SearchError> {
        let g = self.reader.lock();
        match g.as_ref() {
            Some(c) => f(c),
            None => Err(SearchError::Unavailable("index not open yet".into())),
        }
    }

    pub fn snapshot(&self) -> IndexSnapshot {
        let s = self.shared.lock();
        let pending_changes = (s.pending.keys.len() + s.pending.resync.len()) as u64;
        let busy = s.running || s.full || s.clear || pending_changes > 0;
        let embedding_backlog = s.embedder_present && s.pending_embeddings > 0;
        let state = if s.unavailable.is_some() {
            IndexState::Unavailable
        } else if !s.activated {
            IndexState::Stale
        } else if s.rebuilding || !s.opened {
            IndexState::Rebuilding
        } else if busy || (embedding_backlog && s.failed.is_none()) {
            IndexState::Updating
        } else if s.failed.is_some() {
            IndexState::Failed
        } else {
            IndexState::Current
        };
        let progress = (s.progress_total > 0 && s.running)
            .then(|| s.progress_done as f64 / s.progress_total as f64);
        IndexSnapshot {
            state,
            counts: s.counts,
            pending_changes,
            pending_embeddings: s.pending_embeddings,
            semantic_ready: s.opened
                && s.embedder_present
                && s.counts.embedded > 0
                && s.pending_embeddings == 0,
            embedder_present: s.embedder_present,
            embedding_model: s.embedding_model.clone(),
            progress,
            cycles: s.cycles,
            last_cycle_ms: s.last_cycle_ms,
            error: s.failed.clone().or_else(|| s.unavailable.clone()),
        }
    }

    /// Block until nothing is queued or running (or `timeout`). True when idle.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut s = self.shared.lock();
        loop {
            let embed_waiting = s.embedder_present
                && s.pending_embeddings > 0
                && s.failed.is_none()
                && s.embed_retry_at.is_none();
            let idle = s.activated
                && (s.opened || s.unavailable.is_some())
                && !s.running
                && !s.full
                && !s.clear
                && s.pending.is_empty()
                && !embed_waiting;
            if idle {
                return true;
            }
            if !s.activated || Instant::now() >= deadline {
                return false;
            }
            self.cond.wait_until(&mut s, deadline);
        }
    }

    fn take_job(&self, settings: &IndexSettings, alive: &dyn Fn() -> bool) -> Option<Job> {
        let mut s = self.shared.lock();
        loop {
            if self.stop.load(Ordering::SeqCst) {
                return None;
            }
            let now = Instant::now();
            if s.clear || s.full {
                let job = Job {
                    clear: std::mem::take(&mut s.clear),
                    full: std::mem::take(&mut s.full),
                    inv: std::mem::take(&mut s.pending),
                };
                s.first_change = None;
                s.last_change = None;
                s.running = true;
                return Some(job);
            }
            let mut wake_at = now + Duration::from_secs(1);
            if !s.pending.is_empty() {
                let quiet = s.last_change.map(|t| t + settings.debounce).unwrap_or(now);
                let latest = s
                    .first_change
                    .map(|t| t + settings.max_latency)
                    .unwrap_or(now);
                let due = quiet.min(latest);
                if due <= now {
                    let job = Job {
                        clear: false,
                        full: false,
                        inv: std::mem::take(&mut s.pending),
                    };
                    s.first_change = None;
                    s.last_change = None;
                    s.running = true;
                    return Some(job);
                }
                wake_at = wake_at.min(due);
            } else if s.embedder_present && s.pending_embeddings > 0 && s.failed.is_none() {
                match s.embed_retry_at {
                    Some(t) if t > now => wake_at = wake_at.min(t),
                    _ => {
                        s.embed_retry_at = None;
                        s.running = true;
                        return Some(Job {
                            clear: false,
                            full: false,
                            inv: Invalidation::default(),
                        });
                    }
                }
            }
            self.cond.wait_until(&mut s, wake_at);
            // Periodic wake-ups double as a liveness check of the owning core.
            if !alive() {
                return None;
            }
        }
    }

    fn set_progress(&self, done: u64, total: u64) {
        let mut s = self.shared.lock();
        s.progress_done = done;
        s.progress_total = total;
    }

    fn pending_due(&self, settings: &IndexSettings) -> bool {
        let s = self.shared.lock();
        if s.clear || s.full {
            return true;
        }
        if s.pending.is_empty() {
            return false;
        }
        let now = Instant::now();
        let quiet = s.last_change.map(|t| t + settings.debounce).unwrap_or(now);
        let latest = s
            .first_change
            .map(|t| t + settings.max_latency)
            .unwrap_or(now);
        quiet.min(latest) <= now
    }
}

/// Maps committed row changes to invalidated documents. Holds the index weakly:
/// a closed project's observer is inert.
pub struct Observer(pub Weak<ProjectIndex>);

impl CommitObserver for Observer {
    fn committed(&self, changes: &[RowChange], extra: &[(String, String)]) {
        let Some(ix) = self.0.upgrade() else { return };
        let mut inv = Invalidation::default();
        for c in changes {
            documents::invalidate(&mut inv, &c.tbl, &c.row_id, c.old.as_ref(), c.new.as_ref());
        }
        for (t, id) in extra {
            documents::invalidate(&mut inv, t, id, None, None);
        }
        ix.enqueue(inv);
    }
}

fn open_project_reader(path: &Path) -> AppResult<Connection> {
    let conn = openframe_persistence::open_connection(path, false)?;
    conn.execute_batch("PRAGMA query_only=ON;")?;
    Ok(conn)
}

fn identity(conn: &Connection, project_id: &str) -> AppResult<IndexIdentity> {
    Ok(IndexIdentity {
        project_id: project_id.to_string(),
        canonical_schema_version: openframe_persistence::migrate::current_version(conn)?,
        builder_version: documents::BUILDER_VERSION,
    })
}

fn run(ix: Arc<ProjectIndex>, core: Weak<AppCore>, registry: Arc<Registry>) {
    let opened = (|| -> AppResult<IntelligenceDb> {
        let conn = open_project_reader(&ix.project_db)?;
        let id = identity(&conn, &ix.project_id)?;
        drop(conn);
        let (db, outcome) = IntelligenceDb::open(&ix.index_path, &id).map_err(index_error)?;
        if let Some(reason) = &outcome.reason {
            tracing::info!(reason = %reason, "intelligence index discarded; rebuilding");
        }
        let reader = IntelligenceDb::open_reader(&ix.index_path).map_err(index_error)?;
        *ix.reader.lock() = Some(reader);
        let mut s = ix.shared.lock();
        s.opened = true;
        s.rebuilding = outcome.created;
        s.full = true;
        s.counts = docs::counts(db.conn(), None).unwrap_or_default();
        Ok(db)
    })();
    let mut db = match opened {
        Ok(db) => db,
        Err(e) => {
            tracing::warn!(
                code = e.code_str(),
                "intelligence index unavailable; AI uses search and tools only"
            );
            ix.shared.lock().unavailable =
                Some("The project context index could not be opened.".into());
            ix.cond.notify_all();
            return;
        }
    };
    ix.cond.notify_all();
    loop {
        let Some(core_arc) = core.upgrade() else {
            break;
        };
        let svc = super::service(&core_arc);
        let settings = svc.settings();
        drop(core_arc);
        let alive = || core.strong_count() > 0;
        let Some(job) = ix.take_job(&settings, &alive) else {
            break;
        };
        if ix.stop.load(Ordering::SeqCst) {
            break;
        }
        let embedder = core.upgrade().and_then(|c| svc.embedder(&c));
        let started = Instant::now();
        let result = process(&ix, &mut db, &registry, &settings, job, embedder.as_deref());
        let model = embedder.as_ref().map(|e| e.model_id());
        let counts = docs::counts(db.conn(), model.as_deref()).unwrap_or_default();
        let pending_embeddings = match &model {
            Some(m) => docs::pending_count(db.conn(), Some(m)).unwrap_or(0),
            None => 0,
        };
        {
            let mut s = ix.shared.lock();
            s.running = false;
            s.counts = counts;
            s.pending_embeddings = pending_embeddings;
            s.embedder_present = embedder.is_some();
            s.embedding_model = model;
            s.cycles += 1;
            s.last_cycle_ms = Some(started.elapsed().as_millis() as u64);
            s.progress_done = 0;
            s.progress_total = 0;
            match result {
                Ok(Outcome {
                    key_errors: 0,
                    embed_failed,
                }) => {
                    s.failed = None;
                    if !s.full && !s.clear {
                        s.rebuilding = false;
                    }
                    if embed_failed {
                        s.embed_retry_at = Some(Instant::now() + settings.embed_retry);
                    }
                }
                Ok(Outcome { key_errors, .. }) => {
                    s.failed = Some(format!("{key_errors} items could not be indexed"));
                    s.rebuilding = false;
                }
                Err(e) => {
                    tracing::warn!(
                        code = e.code_str(),
                        "intelligence indexing failed; retrieval falls back to search"
                    );
                    s.failed = Some("Indexing failed".into());
                    s.rebuilding = false;
                }
            }
        }
        ix.cond.notify_all();
    }
    *ix.reader.lock() = None;
    drop(db);
    let mut s = ix.shared.lock();
    s.running = false;
    drop(s);
    ix.cond.notify_all();
}

struct Outcome {
    key_errors: usize,
    embed_failed: bool,
}

fn process(
    ix: &ProjectIndex,
    db: &mut IntelligenceDb,
    registry: &Registry,
    settings: &IndexSettings,
    job: Job,
    embedder: Option<&dyn Embedder>,
) -> AppResult<Outcome> {
    if job.clear {
        db.clear_all().map_err(index_error)?;
    }
    let model = match embedder {
        Some(e) => {
            db.ensure_embedding_model(&e.model_id(), e.dim())
                .map_err(index_error)?;
            Some(e.model_id())
        }
        None => None,
    };
    let mut key_errors = 0usize;
    let keys = collect_keys(ix, db, &job)?;
    if !keys.is_empty() {
        let conn = open_project_reader(&ix.project_db)?;
        let total = keys.len() as u64;
        let mut done = 0u64;
        for batch in keys.chunks(settings.build_batch.max(1)) {
            if ix.stop.load(Ordering::SeqCst) {
                return Ok(Outcome {
                    key_errors,
                    embed_failed: false,
                });
            }
            let now = now_ms();
            let tx = db
                .conn_mut()
                .transaction()
                .map_err(|e| index_error(e.into()))?;
            for key in batch {
                if let Err(e) = apply_key(
                    &conn,
                    &tx,
                    registry,
                    &ix.project_id,
                    key,
                    model.as_deref(),
                    now,
                ) {
                    key_errors += 1;
                    tracing::warn!(
                        code = e.code_str(),
                        kind = key.kind,
                        "could not index one item"
                    );
                }
            }
            tx.commit().map_err(|e| index_error(e.into()))?;
            done += batch.len() as u64;
            ix.set_progress(done, total);
        }
        drop(conn);
        if job.full {
            graph::gc_nodes(db.conn()).map_err(index_error)?;
        }
    }
    let mut embed_failed = false;
    if let (Some(e), Some(model)) = (embedder, model.as_deref()) {
        embed_failed = embed_pending(ix, db, settings, e, model)?;
    }
    Ok(Outcome {
        key_errors,
        embed_failed,
    })
}

/// Every document key a job must (re)build.
fn collect_keys(ix: &ProjectIndex, db: &IntelligenceDb, job: &Job) -> AppResult<Vec<DocKey>> {
    let mut keys: BTreeSet<DocKey> = BTreeSet::new();
    let indexed_of = |kind: &str| -> AppResult<Vec<DocKey>> {
        Ok(docs::doc_ids_of_type(db.conn(), kind)
            .map_err(index_error)?
            .into_iter()
            .filter_map(|d| DocKey::parse(&d))
            .collect())
    };
    let needs_canonical = job.full || !job.inv.resync.is_empty();
    let conn = if needs_canonical {
        Some(open_project_reader(&ix.project_db)?)
    } else {
        None
    };
    if job.full {
        keys.extend(documents::all_keys(conn.as_ref().expect("opened"))?);
        for id in docs::all_doc_ids(db.conn()).map_err(index_error)? {
            if let Some(k) = DocKey::parse(&id) {
                keys.insert(k);
            }
        }
    } else {
        keys.extend(job.inv.keys.iter().cloned());
        for kind in &job.inv.resync {
            keys.extend(documents::keys_of_kind(
                conn.as_ref().expect("opened"),
                kind,
            )?);
            keys.extend(indexed_of(kind)?);
        }
        for (table, id) in &job.inv.rows {
            for d in docs::docs_depending_on(db.conn(), table, id).map_err(index_error)? {
                if let Some(k) = DocKey::parse(&d) {
                    keys.insert(k);
                }
            }
        }
    }
    Ok(keys.into_iter().collect())
}

fn apply_key(
    conn: &Connection,
    tx: &Connection,
    registry: &Registry,
    project_id: &str,
    key: &DocKey,
    model: Option<&str>,
    now: i64,
) -> AppResult<()> {
    let doc_id = key.doc_id();
    match documents::build(conn, registry, project_id, key)? {
        Some(b) => {
            docs::upsert_document(tx, &SqliteVecIndex, &b.doc, model, now).map_err(index_error)?;
            graph::replace_fragment(tx, &doc_id, &b.own_nodes, &b.referenced, &b.edges)
                .map_err(index_error)?;
        }
        None => {
            docs::delete_document(tx, &SqliteVecIndex, &doc_id).map_err(index_error)?;
            graph::remove_fragment(tx, &doc_id, &[node_id(key.kind, &key.id)])
                .map_err(index_error)?;
        }
    }
    Ok(())
}

/// Embed waiting chunks in small batches. Yields to newly due edits so fresh
/// changes are indexed first. Returns true when the embedding runtime failed.
fn embed_pending(
    ix: &ProjectIndex,
    db: &mut IntelligenceDb,
    settings: &IndexSettings,
    e: &dyn Embedder,
    model: &str,
) -> AppResult<bool> {
    let total = docs::pending_count(db.conn(), Some(model)).map_err(index_error)?;
    let mut done = 0u64;
    loop {
        if ix.stop.load(Ordering::SeqCst) || ix.pending_due(settings) {
            return Ok(false);
        }
        let batch = docs::pending_chunks(db.conn(), model, settings.embed_batch.max(1))
            .map_err(index_error)?;
        if batch.is_empty() {
            return Ok(false);
        }
        let texts: Vec<String> = batch.iter().map(|(_, t)| t.clone()).collect();
        let vectors = match e
            .embed(&texts)
            .and_then(|v| embed::validate(v, texts.len(), e.dim()))
        {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(error = %err, "embedding runtime failed; semantic search stays unavailable for now");
                return Ok(true);
            }
        };
        let now = now_ms();
        let tx = db
            .conn_mut()
            .transaction()
            .map_err(|e| index_error(e.into()))?;
        for ((rowid, text), v) in batch.iter().zip(&vectors) {
            docs::store_embedding(&tx, &SqliteVecIndex, *rowid, text, model, v, now)
                .map_err(index_error)?;
        }
        tx.commit().map_err(|e| index_error(e.into()))?;
        done += batch.len() as u64;
        ix.set_progress(done, total.max(done));
    }
}

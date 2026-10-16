//! soak tests soak suite (build plan item soak tests; docs/engineering/22-qa-test-strategy.md §7).
//!
//! One `AppCore` with an open project, the intelligence index active (deterministic test
//! embedder) and a local model attached, driven only through `AppCore::dispatch` (the IPC
//! path minus the WebView2 hop) by many threads at once. Four phases share the time budget:
//!
//! 1. **Concurrent IPC, rapid autosave, thousands of mutations**: writer threads create,
//!    update, delete and restore Story Board cards and project notes and type into their own
//!    screenplay element; reader threads run workspace queries and search; one thread saves
//!    (checkpoint + verified backup) every 50–150 ms. Every writer keeps a model of what it
//!    wrote; the final database must equal the model exactly, and search must agree.
//! 2. **Undo during indexing**: while an `ai.index_rebuild` is always running against a slow
//!    embedder, one thread performs mutations, multi-step undo and redo, and checks after
//!    every step that the content equals the model's state for that point in history.
//! 3. **Close and reopen during indexing**: an index rebuild is running and another thread
//!    keeps writing notes while the project is closed and reopened. Close is bounded, the
//!    rebuild task ends, and every write that returned success is in the reopened project.
//! 4. **Repeated AI cancel**: index rebuild tasks cancelled at random points through
//!    `app.cancel_task`, and assistant requests whose local model call is cancelled or
//!    unavailable. Tasks always end, nothing fails, and the project content never changes.
//!
//! At the end: SQLite `integrity_check` and `foreign_key_check`, a consistent undo history
//! (states, cap, parseable steps, undo/redo round trip), counts equal to the models, the
//! index current and complete, no save error, process working set bounded, and a clean
//! close followed by a restart that opens without a recovery offer. Every phase and every
//! single operation is bounded by a timeout, so a deadlock fails the suite instead of
//! hanging it.
//!
//! The sidecar crash and restart loop lives in `crates/openframe-ai/tests/soak_sidecar.rs`:
//! the fake `llama-server` there is the openframe-ai test binary itself.
//!
//! Profiles (`OPENFRAME_SOAK_SECS` sets the total time of either):
//!
//! ```text
//! cargo test -p openframe-application --test platform soak::                     # short profile, 60 s
//! OPENFRAME_SOAK_SECS=1800 cargo test -p openframe-application --test platform -- --ignored --nocapture soak::m1_08_soak_long
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use openframe_ai::{ChatRequest, InferenceProvider};
use openframe_application::events::{AppEvent, EventSink, SaveStatus};
use openframe_application::modules::ai as ai_module;
use openframe_application::modules::ai::intelligence::{self, IndexSettings};
use openframe_application::modules::ai::retrieval::IndexState;
use openframe_application::{AppConfig, AppCore, Store};
use openframe_domain::{AppError, AppResult, new_id};
use openframe_search::Embedder;
use openframe_test_support::embedder::ConceptEmbedder;
use openframe_test_support::stress::Rng;
use serde_json::{Value, json};

const SECS_ENV: &str = "OPENFRAME_SOAK_SECS";
/// No single operation may take longer than this (a stuck lock fails the run).
const OP_BOUND: Duration = Duration::from_secs(30);
/// Closing a project while the index is busy (the UI waits on it). The indexer stops
/// between small batches and `ProjectIndex::shutdown` never waits more than 5 s, so this
/// leaves margin for the concurrent writers of a loaded run and still catches a hang.
const CLOSE_BOUND: Duration = Duration::from_secs(10);
/// Background work (index catch-up, task ends) must settle within this.
const SETTLE: Duration = Duration::from_secs(180);
/// Extra time a phase may use beyond its budget for its final settle before the watchdog fires.
const PHASE_GRACE: Duration = Duration::from_secs(300);
const WRITERS: usize = 4;
const READERS: usize = 2;
/// Working-set growth allowed between the end of warm-up and the end of the run.
const MAX_WS_GROWTH_MB: f64 = 256.0;
const TERMINAL: [&str; 3] = ["completed", "failed", "cancelled"];

// --------------------------------------------------------------------------- event sink

/// Keeps only what the suite checks (task ends, save errors). A recording sink would grow
/// with every event and hide the product's own memory behaviour.
#[derive(Default)]
struct Sink {
    events: AtomicU64,
    tasks: Mutex<HashMap<String, (String, Option<String>)>>,
    save_errors: Mutex<Vec<String>>,
}

impl EventSink for Sink {
    fn emit(&self, e: &AppEvent) {
        self.events.fetch_add(1, Ordering::Relaxed);
        match e {
            AppEvent::Task(t) => {
                self.tasks.lock().unwrap().insert(
                    t.task_id.clone(),
                    (
                        t.state.clone(),
                        t.error.as_ref().map(|e| e.code_str().to_string()),
                    ),
                );
            }
            AppEvent::SaveState(s) if s.status == SaveStatus::Error => {
                let mut v = self.save_errors.lock().unwrap();
                if v.len() < 20 {
                    v.push(format!("{:?}", s.error));
                }
            }
            _ => {}
        }
    }
}

// --------------------------------------------------------------------------- local model

/// A local model whose every call ends abnormally: cancelled (even calls) or unavailable
/// (odd calls), after an optional delay that keeps the request in flight.
#[derive(Default)]
struct CancellingModel {
    calls: AtomicU64,
    delay_ms: AtomicU64,
}

impl InferenceProvider for CancellingModel {
    fn chat(&self, _req: &ChatRequest) -> AppResult<String> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(self.delay_ms.load(Ordering::SeqCst)));
        Err(if n.is_multiple_of(2) {
            AppError::cancelled()
        } else {
            AppError::ai(
                "unavailable",
                "AI is currently unavailable. OpenFrame's core workflows continue to work normally.",
            )
        })
    }
    fn model_reference(&self) -> String {
        "soak-local-profile".into()
    }
    fn capabilities(&self) -> openframe_ai::ProviderCapabilities {
        openframe_ai::ProviderCapabilities {
            context_tokens: 8192,
            languages: None,
            grammar: true,
        }
    }
}

// --------------------------------------------------------------------------- harness

struct Soak {
    dir: tempfile::TempDir,
    core: Arc<AppCore>,
    sink: Arc<Sink>,
    embedder: Arc<ConceptEmbedder>,
    model: Arc<CancellingModel>,
    path: String,
    draft: String,
    /// One screenplay element per writer thread (its typing target).
    elements: Vec<String>,
    ops: AtomicU64,
    mutations: AtomicU64,
    slowest: Mutex<(f64, String)>,
    ws_samples: Mutex<Vec<(String, f64)>>,
}

fn settings() -> IndexSettings {
    IndexSettings {
        debounce: Duration::from_millis(0),
        max_latency: Duration::from_millis(0),
        embed_retry: Duration::from_millis(50),
        embed_batch: 4,
        ..IndexSettings::default()
    }
}

impl Soak {
    fn new() -> Arc<Soak> {
        let dir = tempfile::tempdir().unwrap();
        let sink = Arc::new(Sink::default());
        let core = AppCore::new(AppConfig::isolated(dir.path()), sink.clone()).unwrap();
        let embedder = Arc::new(ConceptEmbedder::with_dim(64));
        intelligence::service(&core).set_settings(settings());
        intelligence::service(&core).set_embedder(Some(embedder.clone() as Arc<dyn Embedder>));
        let model = Arc::new(CancellingModel::default());
        ai_module::service(&core).attach_model(Some(model.clone() as Arc<dyn InferenceProvider>));
        let actor = core.local_actor();
        core.dispatch(
            &actor,
            "project.create",
            json!({ "title": "Soak", "projectType": "Feature Film" }),
        )
        .unwrap();
        let path = core
            .project()
            .unwrap()
            .layout
            .root()
            .to_string_lossy()
            .into_owned();
        let actor = core.local_actor();
        let sp = core
            .dispatch(&actor, "screenplay.create", json!({}))
            .unwrap();
        let draft = sp["draft"]["id"].as_str().unwrap().to_string();
        let mut elements = Vec::new();
        for t in 0..WRITERS {
            let r = core
                .dispatch(
                    &actor,
                    "screenplay.create_scene",
                    json!({ "draftId": draft, "heading": format!("INT. RAILWAY STATION {t} - NIGHT") }),
                )
                .unwrap();
            let scene = r["createdIds"][0].as_str().unwrap().to_string();
            let el = new_id();
            core.dispatch(
                &actor,
                "screenplay.apply_edits",
                json!({ "draftId": draft, "ops": [{ "op": "insertElement", "id": el, "sceneId": scene, "elementType": "action", "text": "Rain hammers the platform." }] }),
            )
            .unwrap();
            elements.push(el);
        }
        Arc::new(Soak {
            dir,
            core,
            sink,
            embedder,
            model,
            path,
            draft,
            elements,
            ops: AtomicU64::new(0),
            mutations: AtomicU64::new(0),
            slowest: Mutex::new((0.0, String::new())),
            ws_samples: Mutex::new(Vec::new()),
        })
    }

    /// One IPC call, bounded by `OP_BOUND`.
    fn call(&self, op: &str, args: Value) -> AppResult<Value> {
        let t = Instant::now();
        let r = self.core.dispatch(&self.core.local_actor(), op, args);
        let el = t.elapsed();
        self.ops.fetch_add(1, Ordering::Relaxed);
        assert!(
            el < OP_BOUND,
            "{op} took {el:?} (bound {OP_BOUND:?}): a stuck lock or an unbounded wait"
        );
        let ms = el.as_secs_f64() * 1000.0;
        let mut s = self.slowest.lock().unwrap();
        if ms > s.0 {
            *s = (ms, op.to_string());
        }
        r
    }

    #[track_caller]
    fn ok(&self, op: &str, args: Value) -> Value {
        self.call(op, args)
            .unwrap_or_else(|e| panic!("{op} failed: {} — {} ({:?})", e.code, e.message, e.detail))
    }

    #[track_caller]
    fn mutate(&self, op: &str, args: Value) -> Value {
        let v = self.ok(op, args);
        self.mutations.fetch_add(1, Ordering::Relaxed);
        v
    }

    fn read<R>(&self, f: impl FnOnce(&rusqlite::Connection) -> AppResult<R>) -> R {
        self.core.project().unwrap().store.read(f).unwrap()
    }

    fn scalar(&self, sql: &str) -> i64 {
        self.read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
    }

    fn strings(&self, sql: &str) -> Vec<String> {
        self.read(|c| {
            let mut st = c.prepare(sql)?;
            let v = st
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(v)
        })
    }

    fn content_hash(&self) -> u64 {
        content_hash(&self.core.project().unwrap().store)
    }

    fn task_end(&self, id: &str) -> Option<(String, Option<String>)> {
        self.sink
            .tasks
            .lock()
            .unwrap()
            .get(id)
            .filter(|(s, _)| TERMINAL.contains(&s.as_str()))
            .cloned()
    }

    /// Wait (bounded) until a background task ends; returns its final state and error code.
    fn wait_task(&self, id: &str, bound: Duration) -> (String, Option<String>) {
        let deadline = Instant::now() + bound;
        loop {
            if let Some(end) = self.task_end(id) {
                return end;
            }
            assert!(
                Instant::now() < deadline,
                "background task {id} did not end within {bound:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// The index has caught up, is current and holds exactly the project's documents.
    fn assert_index_complete(&self, phase: &str) {
        let ix = intelligence::activate(&self.core).expect("project open");
        assert!(
            ix.wait_idle(SETTLE),
            "{phase}: index did not settle: {:?}",
            ix.snapshot()
        );
        let snap = ix.snapshot();
        assert_eq!(snap.state, IndexState::Current, "{phase}: {snap:?}");
        assert!(snap.error.is_none(), "{phase}: {snap:?}");
        let expected = self.read(|c| Ok(intelligence::documents::all_keys(c)?.len() as u64));
        assert_eq!(
            snap.counts.documents, expected,
            "{phase}: the index holds every project document exactly once"
        );
        assert_eq!(
            snap.counts.embedded, snap.counts.chunks,
            "{phase}: every chunk embedded"
        );
    }

    fn sample_ws(&self, label: &str) {
        if let Some(mb) = working_set_mb() {
            self.ws_samples
                .lock()
                .unwrap()
                .push((label.to_string(), mb));
        }
    }
}

/// Hash of all user content that undo can touch (cards, notes, trash, screenplay text).
fn content_hash(store: &Store) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for sql in [
        "SELECT id||'|'||parent_type||'|'||coalesce(parent_id,'')||'|'||short_description||'|'||coalesce(deleted_at,'') FROM story_scene_card ORDER BY id",
        "SELECT id||'|'||coalesce(title,'')||'|'||body||'|'||coalesce(deleted_at,'') FROM project_note ORDER BY id",
        "SELECT table_name||'|'||object_id FROM deleted_item ORDER BY table_name, object_id",
        "SELECT id||'|'||text FROM screenplay_element ORDER BY id",
    ] {
        let rows: Vec<String> = store
            .read(|c| {
                let mut st = c.prepare(sql)?;
                let v = st
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(v)
            })
            .unwrap();
        for row in rows {
            for b in row.as_bytes().iter().chain(b"\n") {
                h ^= *b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        }
    }
    h
}

/// Private working set of this process in MB (Windows `tasklist`; None elsewhere).
fn working_set_mb() -> Option<f64> {
    let out = std::process::Command::new("tasklist")
        .args([
            "/FI",
            &format!("PID eq {}", std::process::id()),
            "/FO",
            "CSV",
            "/NH",
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let field = s.split("\",\"").last()?;
    let digits: String = field.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<f64>().ok().map(|kb| kb / 1024.0)
}

/// Run `f` on its own thread and fail if it does not finish within `limit` (deadlock guard).
fn bounded<T: Send + 'static>(
    name: &str,
    limit: Duration,
    f: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = mpsc::channel();
    let h = std::thread::Builder::new()
        .name(format!("soak-{name}"))
        .spawn(move || {
            let v = f();
            let _ = tx.send(());
            v
        })
        .unwrap();
    match rx.recv_timeout(limit) {
        Ok(()) => h.join().unwrap(),
        Err(mpsc::RecvTimeoutError::Disconnected) => match h.join() {
            Ok(v) => v,
            Err(p) => std::panic::resume_unwind(p),
        },
        Err(mpsc::RecvTimeoutError::Timeout) => panic!(
            "soak phase `{name}` did not finish within {limit:?}: a deadlock or an unbounded wait"
        ),
    }
}

fn id_of(v: &Value) -> String {
    v["id"]
        .as_str()
        .or_else(|| v.as_str())
        .unwrap_or_else(|| panic!("no id in {v}"))
        .to_string()
}

// --------------------------------------------------------------------------- phase 1

#[derive(Default)]
struct Card {
    id: String,
    desc: String,
    deleted: bool,
}

#[derive(Default)]
struct WriterModel {
    cards: Vec<Card>,
    notes: Vec<(String, String)>,
    element_text: String,
}

struct ConcurrentReport {
    models: Vec<WriterModel>,
    saves: u64,
    reads: u64,
}

fn writer(s: &Soak, t: usize, deadline: Instant) -> WriterModel {
    let mut rng = Rng::new(1_000 + t as u64);
    let mut m = WriterModel {
        element_text: "Rain hammers the platform.".into(),
        ..WriterModel::default()
    };
    let el = s.elements[t].clone();
    let mut n = 0usize;
    while Instant::now() < deadline || n < 40 {
        n += 1;
        let live: Vec<usize> = (0..m.cards.len())
            .filter(|i| !m.cards[*i].deleted)
            .collect();
        match rng.below(20) {
            0..=3 => {
                let desc = format!("Soak beat sk{t}x{n}q under the rain");
                let v = s.mutate("story.create_card", json!({ "shortDescription": desc }));
                m.cards.push(Card {
                    id: id_of(&v),
                    desc,
                    deleted: false,
                });
            }
            4..=5 if !live.is_empty() => {
                let c = &mut m.cards[live[rng.below(live.len())]];
                c.desc = format!("Soak beat sk{t}x{n}q rewritten");
                s.mutate(
                    "story.update_card",
                    json!({ "id": c.id, "shortDescription": c.desc }),
                );
            }
            6..=7 => {
                let body = format!("Night shoot note nt{t}x{n}q.");
                let v = s.mutate(
                    "notes.create",
                    json!({ "title": format!("Soak note {t}-{n}"), "body": body }),
                );
                m.notes.push((id_of(&v), body));
            }
            8 if !m.notes.is_empty() => {
                let i = rng.below(m.notes.len());
                let body = format!("Night shoot note nt{t}x{n}q revised.");
                s.mutate("notes.update", json!({ "id": m.notes[i].0, "body": body }));
                m.notes[i].1 = body;
            }
            9 if !live.is_empty() => {
                let c = &mut m.cards[live[rng.below(live.len())]];
                s.mutate(
                    "story.delete_items",
                    json!({ "items": [{ "kind": "card", "id": c.id }] }),
                );
                c.deleted = true;
            }
            10 => {
                if let Some(i) = (0..m.cards.len()).find(|i| m.cards[*i].deleted) {
                    let trash = s.ok("trash.list", json!({}));
                    let entry = trash
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|r| r["objectId"] == m.cards[i].id.as_str())
                        .unwrap_or_else(|| panic!("deleted card {} not in trash", m.cards[i].id));
                    s.mutate("trash.restore", json!({ "id": entry["id"] }));
                    m.cards[i].deleted = false;
                }
            }
            _ => {
                // Typing into this writer's own element (coalesced keystroke batches).
                if m.element_text.len() > 1_500 {
                    m.element_text = format!("Rain again {n}.");
                } else {
                    m.element_text
                        .push_str(if n.is_multiple_of(5) { " " } else { "ab" });
                }
                s.mutate(
                    "screenplay.apply_edits",
                    json!({ "draftId": s.draft, "ops": [{ "op": "updateElement", "id": el, "text": m.element_text }] }),
                );
            }
        }
    }
    m
}

fn reader(s: &Soak, t: usize, deadline: Instant) -> u64 {
    let mut rng = Rng::new(5_000 + t as u64);
    let mut n = 0u64;
    while Instant::now() < deadline || n < 20 {
        n += 1;
        match rng.below(8) {
            0 => {
                let q = format!("sk{}x{}q", rng.below(WRITERS), rng.below(400));
                s.ok("search.query", json!({ "text": q, "limit": 20 }));
            }
            1 => {
                s.ok("story.board", json!({}));
            }
            2 => {
                s.ok("screenplay.document", json!({ "draftId": s.draft }));
            }
            3 => {
                s.ok("history.info", json!({}));
            }
            4 => {
                s.ok("project.home", json!({}));
            }
            5 => {
                s.ok("notes.list", json!({}));
            }
            6 => {
                s.ok("app.save_state", json!({}));
            }
            _ => {
                s.ok("history.activity", json!({}));
            }
        }
    }
    n
}

fn phase_concurrent(s: &Arc<Soak>, budget: Duration) -> ConcurrentReport {
    let deadline = Instant::now() + budget;
    let sample_every = (budget / 6).max(Duration::from_secs(5));
    std::thread::scope(|sc| {
        let writers: Vec<_> = (0..WRITERS)
            .map(|t| sc.spawn(move || writer(s, t, deadline)))
            .collect();
        let readers: Vec<_> = (0..READERS)
            .map(|t| sc.spawn(move || reader(s, t, deadline)))
            .collect();
        // Rapid explicit saves (checkpoint + verified backup) racing the writers.
        let saver = sc.spawn(move || {
            let mut rng = Rng::new(9);
            let mut saves = 0u64;
            let mut next_sample = Instant::now() + sample_every;
            while Instant::now() < deadline || saves < 5 {
                s.ok("project.save", json!({}));
                saves += 1;
                if Instant::now() >= next_sample {
                    s.sample_ws("phase 1 (concurrent)");
                    next_sample += sample_every;
                }
                std::thread::sleep(Duration::from_millis(50 + rng.below(100) as u64));
            }
            saves
        });
        let models = writers.into_iter().map(|h| h.join().unwrap()).collect();
        let reads = readers.into_iter().map(|h| h.join().unwrap()).sum();
        let saves = saver.join().unwrap();
        ConcurrentReport {
            models,
            saves,
            reads,
        }
    })
}

/// The database equals what the writers were told succeeded, and search agrees.
fn verify_concurrent(s: &Soak, r: &ConcurrentReport) {
    let mut live_cards = 0i64;
    for (t, m) in r.models.iter().enumerate() {
        for c in &m.cards {
            let (desc, deleted): (String, Option<i64>) = s.read(|db| {
                Ok(db.query_row(
                    "SELECT short_description, deleted_at FROM story_scene_card WHERE id=?1",
                    [&c.id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?)
            });
            assert_eq!(desc, c.desc, "writer {t}: card {} text", c.id);
            assert_eq!(
                deleted.is_some(),
                c.deleted,
                "writer {t}: card {} deleted state",
                c.id
            );
            if !c.deleted {
                live_cards += 1;
            }
        }
        for (id, body) in &m.notes {
            let got: String = s.read(|db| {
                Ok(
                    db.query_row("SELECT body FROM project_note WHERE id=?1", [id], |row| {
                        row.get(0)
                    })?,
                )
            });
            assert_eq!(&got, body, "writer {t}: note {id}");
        }
        let text: String = s.read(|db| {
            Ok(db.query_row(
                "SELECT text FROM screenplay_element WHERE id=?1",
                [&s.elements[t]],
                |row| row.get(0),
            )?)
        });
        assert_eq!(text, m.element_text, "writer {t}: typed element text");
    }
    assert_eq!(
        s.scalar("SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL"),
        live_cards,
        "live card count equals the writers' model"
    );
    let notes: usize = r.models.iter().map(|m| m.notes.len()).sum();
    assert_eq!(
        s.scalar("SELECT count(*) FROM project_note WHERE deleted_at IS NULL"),
        notes as i64,
        "note count equals the writers' model"
    );
    // Search agrees with the model: live cards are found by their current token, deleted
    // cards are not found at all (sampled: one search per card would dominate the run).
    let mut rng = Rng::new(77);
    for m in &r.models {
        for _ in 0..m.cards.len().min(15) {
            let c = &m.cards[rng.below(m.cards.len())];
            let token = c.desc.split_whitespace().nth(2).expect("token").to_string();
            let hits = s.ok(
                "search.query",
                json!({ "text": token, "entityTypes": ["story_scene_card"], "limit": 10 }),
            );
            let found = hits
                .as_array()
                .unwrap()
                .iter()
                .any(|h| h["entityId"] == c.id.as_str());
            assert_eq!(
                found, !c.deleted,
                "search for {token} (card {}, deleted {}): {hits}",
                c.id, c.deleted
            );
        }
    }
}

// --------------------------------------------------------------------------- phase 2

struct UndoReport {
    steps: u64,
    undos: u64,
    redos: u64,
    rebuilds: u64,
}

fn phase_undo_during_indexing(s: &Arc<Soak>, budget: Duration) -> UndoReport {
    let deadline = Instant::now() + budget;
    s.embedder.set_delay(Some(Duration::from_millis(4)));
    let stop = AtomicBool::new(false);
    let report = std::thread::scope(|sc| {
        // Keep an index rebuild running for the whole phase.
        let driver = sc.spawn(|| {
            let mut rebuilds = 0u64;
            while !stop.load(Ordering::SeqCst) {
                let id = s.ok("ai.index_rebuild", json!({}))["taskId"]
                    .as_str()
                    .unwrap()
                    .to_string();
                let (state, err) = s.wait_task(&id, SETTLE);
                assert_eq!(state, "completed", "index rebuild during undo: {err:?}");
                rebuilds += 1;
            }
            rebuilds
        });
        let mut rng = Rng::new(42);
        // hist[i] = content after step i; keys[i] = coalesce key of step i (undo merges equal keys).
        let mut hist = vec![s.content_hash()];
        let mut keys = vec![String::new()];
        let mut pos = 0usize;
        let (mut steps, mut undos, mut redos) = (0u64, 0u64, 0u64);
        let live_cards = |s: &Soak| {
            s.strings(
                "SELECT id FROM story_scene_card WHERE deleted_at IS NULL AND short_description LIKE 'Undo walk %' ORDER BY id",
            )
        };
        let live_notes = |s: &Soak| {
            s.strings(
                "SELECT id FROM project_note WHERE deleted_at IS NULL AND title LIKE 'Undo walk %' ORDER BY id",
            )
        };
        while Instant::now() < deadline || steps < 30 {
            let n = steps + undos;
            let roll = rng.below(10);
            let key: Option<String> = match roll {
                0 | 1 => {
                    s.mutate(
                        "story.create_card",
                        json!({ "shortDescription": format!("Undo walk card {n}") }),
                    );
                    Some(format!("create:{n}"))
                }
                2 => {
                    let ids: Vec<String> = live_cards(s)
                        .into_iter()
                        .filter(|id| format!("story.card:{id}") != keys[pos])
                        .collect();
                    if ids.is_empty() {
                        None
                    } else {
                        let id = ids[rng.below(ids.len())].clone();
                        s.mutate(
                            "story.update_card",
                            json!({ "id": id, "shortDescription": format!("Undo walk card {n} edited") }),
                        );
                        Some(format!("story.card:{id}"))
                    }
                }
                3 => {
                    s.mutate(
                        "notes.create",
                        json!({ "title": format!("Undo walk note {n}"), "body": "Undo walk body." }),
                    );
                    Some(format!("note:{n}"))
                }
                4 => {
                    let ids: Vec<String> = live_notes(s)
                        .into_iter()
                        .filter(|id| format!("notes.update:{id}") != keys[pos])
                        .collect();
                    if ids.is_empty() {
                        None
                    } else {
                        let id = ids[rng.below(ids.len())].clone();
                        s.mutate(
                            "notes.update",
                            json!({ "id": id, "body": format!("Undo walk body {n}.") }),
                        );
                        Some(format!("notes.update:{id}"))
                    }
                }
                5 => {
                    let ids = live_cards(s);
                    if ids.is_empty() {
                        None
                    } else {
                        let id = ids[rng.below(ids.len())].clone();
                        s.mutate(
                            "story.delete_items",
                            json!({ "items": [{ "kind": "card", "id": id }] }),
                        );
                        Some(format!("delete:{n}"))
                    }
                }
                _ => {
                    if pos > 0 {
                        let depth = 1 + rng.below(pos.min(5));
                        for _ in 0..depth {
                            let label = s.ok("history.undo", json!({}));
                            assert!(
                                !label.is_null(),
                                "undo at history position {pos} had nothing to undo"
                            );
                            pos -= 1;
                            undos += 1;
                            assert_eq!(
                                s.content_hash(),
                                hist[pos],
                                "after undo the content equals the state at history position {pos}"
                            );
                        }
                        if rng.below(2) == 0 {
                            for _ in 0..depth {
                                s.ok("history.redo", json!({}));
                                pos += 1;
                                redos += 1;
                                assert_eq!(
                                    s.content_hash(),
                                    hist[pos],
                                    "after redo the content equals the state at history position {pos}"
                                );
                            }
                        }
                    }
                    None
                }
            };
            if let Some(k) = key {
                steps += 1;
                hist.truncate(pos + 1);
                keys.truncate(pos + 1);
                hist.push(s.content_hash());
                keys.push(k);
                pos += 1;
                // Undo depth is at most 5, so only the recent history needs keeping.
                if hist.len() > 64 {
                    let cut = hist.len() - 16;
                    hist.drain(..cut);
                    keys.drain(..cut);
                    pos -= cut;
                }
            }
        }
        stop.store(true, Ordering::SeqCst);
        let rebuilds = driver.join().unwrap();
        UndoReport {
            steps,
            undos,
            redos,
            rebuilds,
        }
    });
    s.embedder.set_delay(None);
    s.assert_index_complete("phase 2 (undo during indexing)");
    report
}

// --------------------------------------------------------------------------- phase 3

struct CloseReport {
    rounds: u64,
    acknowledged: u64,
    refused: u64,
    slowest_close: Duration,
}

fn phase_close_reopen(s: &Arc<Soak>, budget: Duration) -> CloseReport {
    let deadline = Instant::now() + budget;
    s.embedder.set_delay(Some(Duration::from_millis(3)));
    let stop = AtomicBool::new(false);
    let acknowledged = AtomicU64::new(0);
    let refused = AtomicU64::new(0);
    let before = s.scalar("SELECT count(*) FROM project_note WHERE title LIKE 'Close race %'");
    assert_eq!(before, 0);
    let (rounds, slowest_close) = std::thread::scope(|sc| {
        // Concurrent IPC while the project closes and reopens: a write either succeeds (and
        // must then survive) or is refused because no project is open.
        sc.spawn(|| {
            let mut n = 0u64;
            while !stop.load(Ordering::SeqCst) {
                n += 1;
                match s.call(
                    "notes.create",
                    json!({ "title": format!("Close race {n}"), "body": "Written while the project closes and reopens." }),
                ) {
                    Ok(_) => {
                        acknowledged.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(e) => {
                        assert_eq!(
                            e.code_str(),
                            "not_found.project_not_open",
                            "a write racing close fails only because no project is open: {}",
                            e.message
                        );
                        refused.fetch_add(1, Ordering::SeqCst);
                    }
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        });
        let mut rounds = 0u64;
        let mut slowest = Duration::ZERO;
        while Instant::now() < deadline || rounds < 3 {
            rounds += 1;
            let calls = s.embedder.calls();
            let task = s.ok("ai.index_rebuild", json!({}))["taskId"]
                .as_str()
                .unwrap()
                .to_string();
            let wait = Instant::now() + Duration::from_secs(10);
            while s.embedder.calls() == calls && Instant::now() < wait {
                std::thread::sleep(Duration::from_millis(2));
            }
            let t = Instant::now();
            s.ok("project.close", json!({}));
            let took = t.elapsed();
            slowest = slowest.max(took);
            assert!(
                took < CLOSE_BOUND,
                "round {rounds}: closing during indexing took {took:?}"
            );
            let (state, err) = s.wait_task(&task, SETTLE);
            assert!(
                state != "failed" || err.as_deref() == Some("not_found.project_not_open"),
                "round {rounds}: the rebuild of the closed project ended {state} {err:?}"
            );
            let t = Instant::now();
            s.ok("project.open", json!({ "path": s.path }));
            assert!(
                t.elapsed() < CLOSE_BOUND,
                "round {rounds}: reopening took {:?}",
                t.elapsed()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        stop.store(true, Ordering::SeqCst);
        (rounds, slowest)
    });
    s.embedder.set_delay(None);
    let ack = acknowledged.load(Ordering::SeqCst);
    assert_eq!(
        s.scalar("SELECT count(*) FROM project_note WHERE title LIKE 'Close race %' AND deleted_at IS NULL"),
        ack as i64,
        "every write acknowledged during close/reopen is in the project, and nothing else"
    );
    s.assert_index_complete("phase 3 (close and reopen during indexing)");
    CloseReport {
        rounds,
        acknowledged: ack,
        refused: refused.load(Ordering::SeqCst),
        slowest_close,
    }
}

// --------------------------------------------------------------------------- phase 4

struct CancelReport {
    rebuilds_cancelled: u64,
    rebuilds_completed: u64,
    asks: u64,
}

fn phase_ai_cancel(s: &Arc<Soak>, budget: Duration) -> CancelReport {
    let deadline = Instant::now() + budget;
    s.embedder.set_delay(Some(Duration::from_millis(2)));
    let mut rng = Rng::new(4242);
    let (mut cancelled, mut completed, mut asks) = (0u64, 0u64, 0u64);
    let content = s.content_hash();
    let undo_steps = s.scalar("SELECT count(*) FROM sys_undo");
    let mut rounds = 0u64;
    while Instant::now() < deadline || rounds < 8 {
        rounds += 1;
        let task = s.ok("ai.index_rebuild", json!({}))["taskId"]
            .as_str()
            .unwrap()
            .to_string();
        std::thread::sleep(Duration::from_millis(rng.below(40) as u64));
        s.ok("app.cancel_task", json!({ "taskId": task }));
        let (state, err) = s.wait_task(&task, SETTLE);
        match state.as_str() {
            "cancelled" => cancelled += 1,
            "completed" => completed += 1,
            other => panic!("a cancelled index rebuild ended {other} {err:?}"),
        }
        // An assistant request whose local model call is cancelled or unavailable mid-run.
        s.model
            .delay_ms
            .store(rng.below(30) as u64, Ordering::SeqCst);
        match s.call(
            "ai.ask",
            json!({ "text": "How many scenes are in the script?", "scope": { "kind": "WholeProject" } }),
        ) {
            Ok(v) => assert!(
                v["result"]["kind"].is_string(),
                "an AI result is returned: {v}"
            ),
            Err(e) => assert!(
                e.code_str().starts_with("ai.") || e.code_str() == "internal.cancelled",
                "AI request failure is an AI error, not {}: {}",
                e.code_str(),
                e.message
            ),
        }
        asks += 1;
    }
    s.model.delay_ms.store(0, Ordering::SeqCst);
    s.embedder.set_delay(None);
    assert_eq!(
        s.content_hash(),
        content,
        "cancelled AI work never changes project content"
    );
    assert_eq!(
        s.scalar("SELECT count(*) FROM sys_undo"),
        undo_steps,
        "cancelled AI work adds no undo step"
    );
    s.assert_index_complete("phase 4 (repeated AI cancel)");
    CancelReport {
        rebuilds_cancelled: cancelled,
        rebuilds_completed: completed,
        asks,
    }
}

// --------------------------------------------------------------------------- final checks

fn assert_database_sound(s: &Soak, when: &str) {
    let ok: String = s.read(|c| Ok(c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?));
    assert_eq!(ok, "ok", "{when}: integrity_check");
    let fk = s.read(|c| {
        let mut st = c.prepare("PRAGMA foreign_key_check")?;
        let n = st.query_map([], |_| Ok(()))?.count();
        Ok(n)
    });
    assert_eq!(fk, 0, "{when}: foreign_key_check reports violations");
}

/// Undo history: valid states, capped, every step parseable, and an undo/redo round trip of
/// the most recent steps restores the content exactly.
fn assert_undo_history_consistent(s: &Soak) -> usize {
    assert_eq!(
        s.scalar("SELECT count(*) FROM sys_undo WHERE state NOT IN ('done','undone')"),
        0
    );
    let per_actor = s.scalar(
        "SELECT COALESCE(MAX(n),0) FROM (SELECT count(*) AS n FROM sys_undo GROUP BY actor_id)",
    );
    assert!(per_actor <= 300, "undo history above its cap: {per_actor}");
    for json in s.strings("SELECT changes_json FROM sys_undo") {
        let v: Value = serde_json::from_str(&json).expect("undo step is valid JSON");
        assert!(
            v.is_array() && !v.as_array().unwrap().is_empty(),
            "empty undo step"
        );
    }
    // Drop any redo branch so the round trip starts from the top of the history.
    let top = s.content_hash();
    let depth = (s.scalar("SELECT count(*) FROM sys_undo WHERE state='done'") as usize).min(10);
    let mut seen = vec![top];
    for _ in 0..depth {
        assert!(!s.ok("history.undo", json!({})).is_null());
        seen.push(s.content_hash());
    }
    for i in (0..depth).rev() {
        s.ok("history.redo", json!({}));
        assert_eq!(s.content_hash(), seen[i], "redo restores step {i} exactly");
    }
    assert_eq!(s.content_hash(), top);
    depth
}

fn run_soak(total: Duration) {
    let started = Instant::now();
    let s = Soak::new();
    s.sample_ws("start");
    let budget = |share: u32| total * share / 100;

    let s1 = s.clone();
    let b = budget(45);
    let r1 = bounded("concurrent", b + PHASE_GRACE, move || {
        let r = phase_concurrent(&s1, b);
        verify_concurrent(&s1, &r);
        r
    });
    s.sample_ws("after phase 1");
    let s2 = s.clone();
    let b = budget(20);
    let r2 = bounded("undo-during-indexing", b + PHASE_GRACE, move || {
        phase_undo_during_indexing(&s2, b)
    });
    s.sample_ws("after phase 2");
    let s3 = s.clone();
    let b = budget(20);
    let r3 = bounded("close-reopen-during-indexing", b + PHASE_GRACE, move || {
        phase_close_reopen(&s3, b)
    });
    s.sample_ws("after phase 3");
    let s4 = s.clone();
    let b = budget(15);
    let r4 = bounded("ai-cancel", b + PHASE_GRACE, move || {
        phase_ai_cancel(&s4, b)
    });
    s.sample_ws("after phase 4");

    let s5 = s.clone();
    let round_trip = bounded("final-checks", PHASE_GRACE, move || {
        assert_database_sound(&s5, "end of run");
        let d = assert_undo_history_consistent(&s5);
        s5.assert_index_complete("end of run");
        d
    });
    let save_errors = s.sink.save_errors.lock().unwrap().clone();
    assert!(save_errors.is_empty(), "save errors: {save_errors:?}");
    let unfinished: Vec<String> = s
        .sink
        .tasks
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, (st, _))| !TERMINAL.contains(&st.as_str()))
        .map(|(id, (st, _))| format!("{id} {st}"))
        .collect();
    assert!(
        unfinished.is_empty(),
        "background tasks still running: {unfinished:?}"
    );

    let cards = s.scalar("SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL");
    let notes = s.scalar("SELECT count(*) FROM project_note WHERE deleted_at IS NULL");
    let content = s.content_hash();
    let mutations = s.mutations.load(Ordering::Relaxed);
    let ops = s.ops.load(Ordering::Relaxed);

    // Memory: the working set after the run stays within a fixed bound of the warmed-up value.
    let samples = s.ws_samples.lock().unwrap().clone();
    if samples.len() >= 3 {
        let warm = samples[1].1;
        let last = samples.last().unwrap().1;
        let peak = samples.iter().map(|x| x.1).fold(0.0, f64::max);
        assert!(
            last - warm < MAX_WS_GROWTH_MB,
            "working set grew from {warm:.0} MB to {last:.0} MB (peak {peak:.0} MB): {samples:?}"
        );
    }

    // Clean close, then a restart on the same data: no recovery offer, same content.
    s.ok("project.close", json!({}));
    let dir_path = s.dir.path().to_path_buf();
    s.core.shutdown();
    let sink = Arc::new(Sink::default());
    let core = AppCore::new(AppConfig::isolated(&dir_path), sink).unwrap();
    let reopened = core
        .dispatch(
            &core.local_actor(),
            "project.open",
            json!({ "path": s.path }),
        )
        .unwrap();
    assert!(
        reopened["recovery"].is_null(),
        "a clean close leaves no recovery offer: {reopened}"
    );
    let store = &core.project().unwrap().store;
    let count = |sql: &str| -> i64 {
        store
            .read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
            .unwrap()
    };
    assert_eq!(
        count("SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL"),
        cards
    );
    assert_eq!(
        count("SELECT count(*) FROM project_note WHERE deleted_at IS NULL"),
        notes
    );
    let ok: String = store
        .read(|c| Ok(c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(ok, "ok", "integrity after restart");
    assert_eq!(content_hash(store), content, "content after restart");
    core.dispatch(&core.local_actor(), "project.close", json!({}))
        .unwrap();

    let slowest = s.slowest.lock().unwrap().clone();
    println!(
        "\nM1.08 soak: {:.0} s total (budget {} s)\n\
         - ops dispatched: {ops}; mutations acknowledged: {mutations}; events: {}\n\
         - phase 1: {} writers, {} readers, {} explicit saves, {} reads; final cards {cards}, notes {notes}\n\
         - phase 2: {} mutations, {} undos, {} redos checked against the model during {} index rebuilds\n\
         - phase 3: {} close/reopen rounds during indexing; {} writes acknowledged, {} refused while closed; slowest close {:?}\n\
         - phase 4: {} rebuilds cancelled, {} completed before the cancel landed, {} assistant requests cancelled/unavailable\n\
         - final undo/redo round trip depth {round_trip}; slowest single op {:.0} ms ({})\n\
         - working set samples (MB): {}",
        started.elapsed().as_secs_f64(),
        total.as_secs(),
        s.sink.events.load(Ordering::Relaxed),
        WRITERS,
        READERS,
        r1.saves,
        r1.reads,
        r2.steps,
        r2.undos,
        r2.redos,
        r2.rebuilds,
        r3.rounds,
        r3.acknowledged,
        r3.refused,
        r3.slowest_close,
        r4.rebuilds_cancelled,
        r4.rebuilds_completed,
        r4.asks,
        slowest.0,
        slowest.1,
        samples
            .iter()
            .map(|(l, mb)| format!("{l}={mb:.0}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn secs(default: u64) -> Duration {
    Duration::from_secs(
        std::env::var(SECS_ENV)
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(default),
    )
}

/// Short profile (runs in `cargo test`): every phase with a 60 s total by default.
#[test]
fn m1_08_soak_short() {
    run_soak(secs(60));
}

/// Long profile for release evidence: 30 minutes by default.
#[test]
#[ignore = "long soak: OPENFRAME_SOAK_SECS=1800 cargo test -p openframe-application --test platform -- --ignored --nocapture soak::m1_08_soak_long"]
fn m1_08_soak_long() {
    run_soak(secs(1800));
}

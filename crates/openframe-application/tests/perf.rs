//! Performance budgets (docs/engineering/21-performance-budgets.md) measured on
//! the "Feature-120" stress project built by `openframe_test_support::stress`.
//!
//! Ignored by default (it takes a minute and must run optimised):
//!
//! ```text
//! npm run perf
//! # = cargo test --profile perf -p openframe-application --test perf -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Every timing goes through `AppCore::dispatch` with JSON in and JSON out and
//! includes serialising the result to a string, i.e. everything the desktop
//! IPC path does except the WebView2 message hop.

use std::path::Path;
use std::time::{Duration, Instant};

use openframe_application::events::AppEvent;
use openframe_test_support::TestEnv;
use openframe_test_support::stress::{self, StressProject, StressSpec};
use serde_json::{Value, json};

// ------------------------------------------------------------------ harness

struct Row {
    name: String,
    n: usize,
    p50: f64,
    p95: f64,
    max: f64,
    bytes: usize,
    budget_ms: Option<f64>,
}

#[derive(Default)]
struct Report {
    rows: Vec<Row>,
    notes: Vec<String>,
}

impl Report {
    fn add(&mut self, name: &str, mut ms: Vec<f64>, bytes: usize, budget_ms: Option<f64>) -> f64 {
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let pct = |p: f64| ms[((ms.len() as f64 - 1.0) * p).round() as usize];
        let row = Row {
            name: name.to_string(),
            n: ms.len(),
            p50: pct(0.5),
            p95: pct(0.95),
            max: *ms.last().unwrap(),
            bytes,
            budget_ms,
        };
        let p95 = row.p95;
        self.rows.push(row);
        p95
    }
    fn note(&mut self, s: String) {
        println!("{s}");
        self.notes.push(s);
    }
    fn print(&self) {
        println!("\n| Measurement | n | p50 ms | p95 ms | max ms | result bytes | budget ms | |");
        println!("|---|---:|---:|---:|---:|---:|---:|---|");
        for r in &self.rows {
            let verdict = match r.budget_ms {
                Some(b) if r.p95 <= b => "ok",
                Some(_) => "OVER",
                None => "",
            };
            println!(
                "| {} | {} | {:.1} | {:.1} | {:.1} | {} | {} | {} |",
                r.name,
                r.n,
                r.p50,
                r.p95,
                r.max,
                r.bytes,
                r.budget_ms.map(|b| format!("{b:.0}")).unwrap_or_default(),
                verdict
            );
        }
        println!();
        for n in &self.notes {
            println!("- {n}");
        }
    }
    fn breaches(&self) -> Vec<String> {
        self.rows
            .iter()
            .filter(|r| r.budget_ms.is_some_and(|b| r.p95 > b))
            .map(|r| {
                format!(
                    "{}: p95 {:.1} ms > {:.0} ms",
                    r.name,
                    r.p95,
                    r.budget_ms.unwrap()
                )
            })
            .collect()
    }
}

/// One op call as the IPC layer performs it (dispatch + result serialisation).
fn timed(env: &TestEnv, op: &str, args: &Value) -> (f64, usize, Value) {
    let t = Instant::now();
    let v = env
        .call(op, args.clone())
        .unwrap_or_else(|e| panic!("{op} failed: {} — {}", e.code, e.message));
    let s = serde_json::to_string(&v).unwrap();
    (t.elapsed().as_secs_f64() * 1000.0, s.len(), v)
}

/// Measure a query `n` times (after one warm-up call).
fn query(r: &mut Report, env: &TestEnv, label: &str, op: &str, args: Value, n: usize, budget: f64) {
    let _ = timed(env, op, &args);
    let mut ms = Vec::with_capacity(n);
    let mut bytes = 0;
    for _ in 0..n {
        let (t, b, _) = timed(env, op, &args);
        ms.push(t);
        bytes = b;
    }
    r.add(label, ms, bytes, Some(budget));
}

fn db_bytes(env: &TestEnv) -> u64 {
    let root = Path::new(&env.project_path()).to_path_buf();
    ["project.sqlite", "project.sqlite-wal"]
        .iter()
        .map(|f| {
            std::fs::metadata(root.join(f))
                .map(|m| m.len())
                .unwrap_or(0)
        })
        .sum()
}

fn scalar(env: &TestEnv, sql: &str) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
        .unwrap()
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

// ------------------------------------------------------------ SQL profile

/// Per-statement totals collected by `sqlite3_trace_v2(SQLITE_TRACE_PROFILE)`.
mod sqlprof {
    use std::collections::HashMap;
    use std::ffi::{CStr, c_int, c_uint, c_void};
    use std::sync::Mutex;

    use rusqlite::{Connection, ffi};

    pub static STATS: Mutex<Option<HashMap<String, (u64, u64)>>> = Mutex::new(None);

    unsafe extern "C" fn cb(
        kind: c_uint,
        _ctx: *mut c_void,
        p: *mut c_void,
        x: *mut c_void,
    ) -> c_int {
        if kind == ffi::SQLITE_TRACE_PROFILE as c_uint {
            // SAFETY: for SQLITE_TRACE_PROFILE, `p` is the statement and `x` points to an i64 of nanoseconds.
            let (sql, ns) = unsafe {
                let s = ffi::sqlite3_sql(p as *mut ffi::sqlite3_stmt);
                let sql = if s.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(s).to_string_lossy().into_owned()
                };
                (sql, *(x as *const i64) as u64)
            };
            if let Ok(mut g) = STATS.lock()
                && let Some(m) = g.as_mut()
            {
                let e = m.entry(sql).or_insert((0, 0));
                e.0 += 1;
                e.1 += ns;
            }
        }
        0
    }

    pub fn attach(c: &Connection) {
        // SAFETY: registering a callback on a live connection handle; the callback only reads its arguments.
        unsafe {
            ffi::sqlite3_trace_v2(
                c.handle(),
                ffi::SQLITE_TRACE_PROFILE as c_uint,
                Some(cb),
                std::ptr::null_mut(),
            );
        }
    }

    pub fn detach(c: &Connection) {
        // SAFETY: as above; a zero mask removes the callback.
        unsafe {
            ffi::sqlite3_trace_v2(c.handle(), 0, None, std::ptr::null_mut());
        }
    }
}

fn profile_on(env: &TestEnv) {
    *sqlprof::STATS.lock().unwrap() = Some(Default::default());
    let s = env.core.project().unwrap();
    s.store
        .read(|c| {
            sqlprof::attach(c);
            Ok(())
        })
        .unwrap();
    s.store
        .with_writer(|c| {
            sqlprof::attach(c);
            Ok(())
        })
        .unwrap();
}

/// Stop profiling; print the most expensive statements with their query plans
/// and return the ones that full-scan a table (without an index).
fn profile_off(env: &TestEnv, r: &mut Report) -> Vec<String> {
    let s = env.core.project().unwrap();
    s.store
        .read(|c| {
            sqlprof::detach(c);
            Ok(())
        })
        .unwrap();
    s.store
        .with_writer(|c| {
            sqlprof::detach(c);
            Ok(())
        })
        .unwrap();
    let stats = sqlprof::STATS.lock().unwrap().take().unwrap_or_default();
    let mut rows: Vec<(String, u64, u64)> =
        stats.into_iter().map(|(k, (n, ns))| (k, n, ns)).collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.2));
    let total: u64 = rows.iter().map(|r| r.2).sum();
    println!(
        "\nTop SQL statements by total time ({} distinct, {:.0} ms total):",
        rows.len(),
        total as f64 / 1e6
    );
    let mut scans = Vec::new();
    for (sql, n, ns) in rows.iter().take(30) {
        let one_line: String = sql.split_whitespace().collect::<Vec<_>>().join(" ");
        let plan = s
            .store
            .read(|c| {
                let mut st = match c.prepare(&format!("EXPLAIN QUERY PLAN {sql}")) {
                    Ok(st) => st,
                    Err(_) => return Ok(Vec::new()),
                };
                let n = st.parameter_count();
                let params: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Null; n];
                let v = st
                    .query_map(rusqlite::params_from_iter(params), |r| {
                        r.get::<_, String>(3)
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(v)
            })
            .unwrap_or_default();
        println!(
            "  {:>6.1} ms total, {:>5} calls, {:>7.3} ms/call: {}",
            *ns as f64 / 1e6,
            n,
            *ns as f64 / 1e6 / *n as f64,
            &one_line[..one_line.len().min(160)]
        );
        for p in &plan {
            println!("        plan: {p}");
            let is_scan =
                p.starts_with("SCAN ") && !p.contains("USING") && !p.contains("VIRTUAL TABLE");
            if is_scan && !p.starts_with("SCAN CONSTANT") {
                scans.push(format!("{p} ← {}", &one_line[..one_line.len().min(120)]));
            }
        }
    }
    r.note(format!(
        "SQL profile: {} distinct statements, {:.0} ms total during the query/mutation phase.",
        rows.len(),
        total as f64 / 1e6
    ));
    scans
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let target = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &target);
        } else {
            std::fs::copy(e.path(), target).unwrap();
        }
    }
}

fn wait_task(env: &TestEnv, id: &str) -> Value {
    let start = Instant::now();
    loop {
        {
            let events = env.sink.events.lock();
            for e in events.iter().rev() {
                if let AppEvent::Task(t) = e
                    && t.task_id == id
                    && matches!(t.state.as_str(), "completed" | "failed" | "cancelled")
                {
                    assert_eq!(t.state, "completed", "task {id}: {:?}", t.error);
                    return t.result.clone().unwrap_or(Value::Null);
                }
            }
        }
        assert!(
            start.elapsed() < Duration::from_secs(300),
            "task {id} did not finish"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

// ------------------------------------------------------------------ the fixture

#[test]
fn stress_fixture_small_builds_through_the_op_registry() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let spec = StressSpec::small();
    let p = stress::build(&env, &spec);
    assert_eq!(p.scene_ids.len(), spec.scenes);
    assert_eq!(p.draft_ids.len(), spec.drafts);
    assert_eq!(p.card_ids.len(), spec.cards);
    assert_eq!(p.vault_ids.len(), spec.vault_notes + spec.vault_images);
    assert_eq!(p.shot_ids.len(), spec.shots);
    assert_eq!(p.call_sheet_ids.len(), spec.call_sheets);
    assert_eq!(
        scalar(
            &env,
            "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL"
        ),
        spec.breakdown_elements as i64
    );
    let sv = env.ok("schedule.get", json!({}));
    assert!(
        sv["unscheduled"].as_array().unwrap().is_empty(),
        "every strip is on a day"
    );
    let img = stress::png(8, 8, 3);
    assert_eq!(&img[..8], b"\x89PNG\r\n\x1a\n");
}

// ------------------------------------------- regression tests for the perf fixes

/// visual.scenes / storyboard.list fingerprint many scenes in bulk; the result
/// must equal the single-scene fingerprint used by shot.get.
#[test]
fn bulk_scene_fingerprints_flag_exactly_the_changed_scene() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let spec = StressSpec::small();
    let p = stress::build(&env, &spec);
    let rows = env.ok("visual.scenes", json!({}));
    assert!(
        rows["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["needsReview"] == false),
        "nothing changed yet: {rows}"
    );
    // Change scene 3 after planning.
    env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": p.draft_id, "ops": [{ "op": "updateElement", "id": p.action_ids[2], "text": "A new action line." }] }),
    );
    let rows = env.ok("visual.scenes", json!({}));
    let flagged: Vec<&str> = rows["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["needsReview"] == true)
        .map(|s| s["sceneId"].as_str().unwrap())
        .collect();
    assert_eq!(flagged, vec![p.scene_ids[2].as_str()]);
    // The single-scene path agrees.
    let shots = env.ok("shot.list", json!({ "sceneId": p.scene_ids[2] }));
    assert!(
        shots
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["needsReview"] == true)
    );
    let shots = env.ok("shot.list", json!({ "sceneId": p.scene_ids[3] }));
    assert!(
        shots
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["needsReview"] == false)
    );
    let boards = env.ok("storyboard.list", json!({}));
    for b in boards.as_array().unwrap() {
        let changed = b["sceneId"].as_str() == Some(p.scene_ids[2].as_str());
        assert_eq!(b["needsReview"], changed, "{b}");
    }
}

/// Type-restricted search filters before the LIMIT, so hits of the requested
/// type are not crowded out by better-ranked hits of other types.
#[test]
fn search_type_filter_applies_before_the_limit() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    for i in 0..12 {
        env.ok(
            "vault.create",
            json!({ "store": "project", "itemType": "note", "title": format!("Zebra zebra zebra {i}"), "body": "zebra" }),
        );
    }
    env.ok(
        "story.create_card",
        json!({ "shortDescription": "A zebra crossing at night" }),
    );
    let hits = env.ok(
        "search.query",
        json!({ "text": "zebra", "entityTypes": ["story_scene_card"], "limit": 2 }),
    );
    let hits = hits.as_array().unwrap();
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0]["entityType"], "story_scene_card");
    // Unfiltered search still ranks and limits.
    let all = env.ok("search.query", json!({ "text": "zebra", "limit": 5 }));
    assert_eq!(all.as_array().unwrap().len(), 5);
    let ranks: Vec<f64> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["rank"].as_f64().unwrap())
        .collect();
    assert!(
        ranks.windows(2).all(|w| w[0] <= w[1]),
        "best first: {ranks:?}"
    );
}

/// Bulk asset loading keeps availability exact (missing files are reported).
#[test]
fn vault_list_reports_missing_image_files() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let p = stress::build(&env, &StressSpec::small());
    let list = env.ok("vault.list", json!({ "store": "project" }));
    let images: Vec<&Value> = list
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["asset"].is_object())
        .collect();
    assert_eq!(images.len(), StressSpec::small().vault_images);
    assert!(images.iter().all(|i| i["asset"]["available"] == true));
    let gone = images[0]["asset"]["path"].as_str().unwrap().to_string();
    let gone_id = images[0]["id"].clone();
    std::fs::remove_file(&gone).unwrap();
    let list = env.ok("vault.list", json!({ "store": "project" }));
    for i in list
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["asset"].is_object())
    {
        assert_eq!(i["asset"]["available"], i["id"] != gone_id, "{i}");
    }
    let _ = p;
}

/// Undo bookkeeping reads sys_undo through an index (no table scans per mutation).
#[test]
fn undo_history_queries_use_the_index() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let plan: Vec<String> = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            let mut st = c.prepare(
                "EXPLAIN QUERY PLAN SELECT label FROM sys_undo WHERE state='done' AND actor_id IS ?1 ORDER BY seq DESC LIMIT 1",
            )?;
            let v = st
                .query_map(["x"], |r| r.get::<_, String>(3))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(v)
        })
        .unwrap();
    assert!(
        plan.iter().any(|p| p.contains("idx_undo_actor_state")),
        "{plan:?}"
    );
    // Undo/redo still restore rows exactly (the restore no longer rewrites `id`).
    let act = env.ok("story.create_act", json!({ "title": "Act One" }));
    env.ok(
        "story.update_act",
        json!({ "id": act["id"], "title": "Act 1" }),
    );
    env.undo();
    assert_eq!(
        env.ok("story.board", json!({}))["acts"][0]["title"],
        "Act One"
    );
    env.redo();
    assert_eq!(
        env.ok("story.board", json!({}))["acts"][0]["title"],
        "Act 1"
    );
}

// ------------------------------------------------------------------ Feature-120

#[test]
#[ignore = "performance suite: run with `npm run perf` (optimised build)"]
fn perf_feature_120_budgets() {
    let mut r = Report::default();
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let t = Instant::now();
    let spec = StressSpec::feature_120();
    let p: StressProject = stress::build(&env, &spec);
    r.note(format!(
        "Fixture built in {:.1} s: {} scenes, {} elements per draft × {} drafts, {} cards, {} vault items, {} shots.",
        t.elapsed().as_secs_f64(),
        p.scene_ids.len(),
        p.element_count,
        p.draft_ids.len(),
        p.card_ids.len(),
        p.vault_ids.len(),
        p.shot_ids.len()
    ));
    for (name, secs) in &p.phases {
        r.note(format!("  fixture phase {name}: {secs:.2} s"));
    }
    env.ok("project.save", json!({}));
    r.note(format!(
        "Project database after the fixture: {:.1} MB ({} search docs, {} undo entries, {} activity rows, history points {:.1} MB).",
        db_bytes(&env) as f64 / 1e6,
        scalar(&env, "SELECT count(*) FROM search_doc"),
        scalar(&env, "SELECT count(*) FROM sys_undo"),
        scalar(&env, "SELECT count(*) FROM sys_activity"),
        scalar(&env, "SELECT COALESCE(sum(length(snapshot_json)),0) FROM screenplay_history_point") as f64 / 1e6
    ));
    if let Some(mb) = working_set_mb() {
        r.note(format!(
            "Test process working set after building the fixture: {mb:.0} MB"
        ));
    }
    // Where the bytes are (dbstat is compiled into the bundled SQLite when available).
    let sizes: Vec<(String, i64)> = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            let mut st = match c.prepare(
                "SELECT name, SUM(pgsize) FROM dbstat GROUP BY name ORDER BY 2 DESC LIMIT 8",
            ) {
                Ok(st) => st,
                Err(_) => return Ok(Vec::new()),
            };
            let v = st
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(v)
        })
        .unwrap_or_default();
    if !sizes.is_empty() {
        r.note(format!(
            "Largest tables/indexes: {}",
            sizes
                .iter()
                .map(|(n, b)| format!("{n} {:.1} MB", *b as f64 / 1e6))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // --- P2 project open (close → open → Project Home), repeated.
    let path = p.project_path.clone();
    let mut open_ms = Vec::new();
    for _ in 0..7 {
        env.ok("project.close", json!({}));
        let t = Instant::now();
        env.ok("project.open", json!({ "path": path }));
        let home = env.ok("project.home", json!({}));
        let _ = serde_json::to_string(&home).unwrap();
        open_ms.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    r.add("P2 project.open + project.home", open_ms, 0, Some(1500.0));

    profile_on(&env);

    // --- Workspace queries (P4/P7 class: < 150 ms p95).
    let draft = p.draft_id.clone();
    let scene55 = p.scene_ids[54.min(p.scene_ids.len() - 1)].clone();
    let n = 15;
    query(
        &mut r,
        &env,
        "project.home",
        "project.home",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "story.board",
        "story.board",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "screenplay.overview",
        "screenplay.overview",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "screenplay.document (180 scenes)",
        "screenplay.document",
        json!({ "draftId": draft }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "screenplay.characters",
        "screenplay.characters",
        json!({ "draftId": draft }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "screenplay.lock_summary",
        "screenplay.lock_summary",
        json!({ "draftId": draft }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "screenplay.history_points",
        "screenplay.history_points",
        json!({ "draftId": draft }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "screenplay.scene_hub",
        "screenplay.scene_hub",
        json!({ "sceneId": scene55 }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "comment.list (draft)",
        "comment.list",
        json!({ "draftId": draft }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "vault.overview",
        "vault.overview",
        json!({ "store": "project" }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "vault.list (all, 1,500 items)",
        "vault.list",
        json!({ "store": "project" }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "vault.list (search)",
        "vault.list",
        json!({ "store": "project", "search": "rain" }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "production.overview",
        "production.overview",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "breakdown.scenes",
        "breakdown.scenes",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "breakdown.scene",
        "breakdown.scene",
        json!({ "sceneId": scene55 }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "catalog.list",
        "catalog.list",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "locations.list",
        "locations.list",
        json!({}),
        n,
        150.0,
    );
    query(&mut r, &env, "cast.list", "cast.list", json!({}), n, 150.0);
    query(&mut r, &env, "crew.list", "crew.list", json!({}), n, 150.0);
    query(
        &mut r,
        &env,
        "visual.scenes",
        "visual.scenes",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "shot.list (scene)",
        "shot.list",
        json!({ "sceneId": scene55 }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "storyboard.list",
        "storyboard.list",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "storyboard.get",
        "storyboard.get",
        json!({ "id": p.storyboard_ids[0] }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "schedule.get (stripboard)",
        "schedule.get",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "callsheets.list",
        "callsheets.list",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "callsheets.get",
        "callsheets.get",
        json!({ "id": p.call_sheet_ids[0] }),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "history.activity",
        "history.activity",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "history.info",
        "history.info",
        json!({}),
        n,
        150.0,
    );
    query(
        &mut r,
        &env,
        "trash.list",
        "trash.list",
        json!({}),
        n,
        150.0,
    );

    // --- P4 search: a mix of typical terms, project and project + global.
    let mut ms = Vec::new();
    let mut bytes = 0;
    for text in [
        "rain",
        "platform",
        "ar",
        "arj",
        "meera station",
        "idea 42",
        "the",
        "zzqx",
        "police station night",
        "sequence 3",
    ] {
        for global in [false, true] {
            let args = json!({ "text": text, "includeGlobal": global, "limit": 60 });
            let _ = timed(&env, "search.query", &args);
            for _ in 0..3 {
                let (t, b, _) = timed(&env, "search.query", &args);
                ms.push(t);
                bytes = bytes.max(b);
            }
        }
    }
    r.add(
        "P4 search.query (10 terms × project/global)",
        ms,
        bytes,
        Some(150.0),
    );

    // --- P3 (Rust side) typing: keystroke batches on one element of scene 55.
    let el = p.action_ids[54.min(p.action_ids.len() - 1)].clone();
    let mut text = String::from("Rain hammers the tin roof.");
    let mut ms = Vec::new();
    let mut with_refetch = Vec::new();
    for i in 0..200 {
        text.push_str(if i % 5 == 4 { " " } else { "ab" });
        let args =
            json!({ "draftId": draft, "ops": [{ "op": "updateElement", "id": el, "text": text }] });
        let (t, _, _) = timed(&env, "screenplay.apply_edits", &args);
        ms.push(t);
        if i % 10 == 0 {
            // What the UI does after the dataChanged event of a batch.
            let (t2, _, _) = timed(&env, "screenplay.document", &json!({ "draftId": draft }));
            with_refetch.push(t + t2);
        }
    }
    r.add(
        "P3 screenplay.apply_edits keystroke batch (coalesced)",
        ms,
        0,
        Some(16.0),
    );
    r.add(
        "   … plus the screenplay.document refetch it triggers",
        with_refetch,
        0,
        None,
    );

    // --- P7 ordinary mutations.
    let mut ms = Vec::new();
    for i in 0..40 {
        let (t, _, _) = timed(
            &env,
            "story.update_card",
            &json!({ "id": p.card_ids[i * 5 % p.card_ids.len()], "shortDescription": format!("Rewritten beat {i}") }),
        );
        ms.push(t);
    }
    r.add("P7 story.update_card", ms, 0, Some(50.0));
    let mut ms = Vec::new();
    for i in 0..40 {
        let (t, _, _) = timed(
            &env,
            "vault.update",
            &json!({ "store": "project", "id": p.vault_ids[i * 17 % p.vault_ids.len()], "title": format!("Idea retitled {i}") }),
        );
        ms.push(t);
    }
    r.add("P7 vault.update", ms, 0, Some(50.0));
    let sv = env.ok("schedule.get", json!({}));
    let strip_on_day0 = sv["days"][0]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|i| i["strip"]["id"].as_str().map(str::to_string))
        .expect("a strip on day 1");
    let mut ms = Vec::new();
    for i in 0..20 {
        let day = &p.day_ids[(i % 2) + 1];
        let (t, _, _) = timed(
            &env,
            "schedule.move_strip",
            &json!({ "stripId": strip_on_day0, "dayId": day, "index": 0 }),
        );
        ms.push(t);
    }
    r.add("P7 schedule.move_strip", ms, 0, Some(50.0));

    // --- P8 undo / redo.
    let mut undo_ms = Vec::new();
    let mut redo_ms = Vec::new();
    for _ in 0..20 {
        let (t, _, _) = timed(&env, "history.undo", &json!({}));
        undo_ms.push(t);
    }
    for _ in 0..20 {
        let (t, _, _) = timed(&env, "history.redo", &json!({}));
        redo_ms.push(t);
    }
    r.add("P8 history.undo", undo_ms, 0, Some(100.0));
    r.add("P8 history.redo", redo_ms, 0, Some(100.0));

    let scans = profile_off(&env, &mut r);
    for sc in &scans {
        r.note(format!("Full scan in a hot statement: {sc}"));
    }

    // --- Autosave throughput + DB growth after 1,000 edits (worst case:
    // alternating elements so no two consecutive batches coalesce).
    env.ok("project.save", json!({}));
    let before = db_bytes(&env);
    let undo_before = scalar(
        &env,
        "SELECT COALESCE(sum(length(changes_json)),0) FROM sys_undo",
    );
    let act_before = scalar(&env, "SELECT count(*) FROM sys_activity");
    let t = Instant::now();
    for i in 0..1000 {
        let e = &p.action_ids[(i % 40) * 4 % p.action_ids.len()];
        env.ok(
            "screenplay.apply_edits",
            json!({ "draftId": draft, "ops": [{ "op": "updateElement", "id": e, "text": format!("Edit number {i} of the scene action.") }] }),
        );
    }
    let secs = t.elapsed().as_secs_f64();
    env.ok("project.save", json!({}));
    let after = db_bytes(&env);
    r.note(format!(
        "Autosave throughput: 1,000 non-coalescing element edits in {secs:.2} s = {:.0} commits/s.",
        1000.0 / secs
    ));
    r.note(format!(
        "DB growth after 1,000 edits: {:+.2} MB (undo entries {} [cap 300], undo JSON {:+.0} KB, activity rows {:+}).",
        (after as f64 - before as f64) / 1e6,
        scalar(&env, "SELECT count(*) FROM sys_undo"),
        (scalar(&env, "SELECT COALESCE(sum(length(changes_json)),0) FROM sys_undo") - undo_before) as f64 / 1e3,
        scalar(&env, "SELECT count(*) FROM sys_activity") - act_before
    ));
    assert!(
        after < before + 8_000_000,
        "1,000 edits grew the project database by more than 8 MB"
    );

    // --- P12 explicit save.
    let mut ms = Vec::new();
    for _ in 0..5 {
        let (t, _, _) = timed(&env, "project.save", &json!({}));
        ms.push(t);
    }
    r.add(
        "P12 project.save (checkpoint + backup)",
        ms,
        0,
        Some(1000.0),
    );

    // --- Export: 120-page screenplay PDF.
    let out = tempfile::tempdir().unwrap();
    let mut ms = Vec::new();
    let mut pages = 0;
    for i in 0..3 {
        let dest = out.path().join(format!("script-{i}.pdf"));
        let (t, _, v) = timed(
            &env,
            "screenplay.export",
            &json!({ "draftId": draft, "format": "pdf", "path": dest.to_string_lossy() }),
        );
        pages = v["pages"].as_u64().unwrap_or(0);
        ms.push(t);
    }
    r.add(
        &format!("screenplay.export PDF ({pages} pages)"),
        ms,
        0,
        Some(3000.0),
    );

    // --- Backup of the whole project (background task).
    let mut ms = Vec::new();
    let mut size = 0;
    for i in 0..2 {
        let dest = out.path().join(format!("backup-{i}.ofbackup"));
        let t = Instant::now();
        let started = env.ok(
            "packages.create_backup",
            json!({ "path": dest.to_string_lossy() }),
        );
        wait_task(&env, started["taskId"].as_str().unwrap());
        ms.push(t.elapsed().as_secs_f64() * 1000.0);
        size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    }
    r.add(
        &format!("packages.create_backup ({:.1} MB)", size as f64 / 1e6),
        ms,
        0,
        Some(10_000.0),
    );

    if let Some(mb) = working_set_mb() {
        r.note(format!("Test process working set at the end: {mb:.0} MB"));
    }
    r.print();
    // Keep a copy of the project for UI-level profiling of a real build
    // (open it in the app: WebView typing latency, idle CPU/RAM).
    if let Ok(dest) = std::env::var("OPENFRAME_PERF_EXPORT") {
        env.ok("project.close", json!({}));
        let to = Path::new(&dest).join("Feature-120.openframe");
        copy_dir(Path::new(&p.project_path), &to);
        println!("Exported the stress project to {}", to.display());
    }
    let breaches = r.breaches();
    assert!(
        breaches.is_empty(),
        "budget breaches:\n{}",
        breaches.join("\n")
    );
}

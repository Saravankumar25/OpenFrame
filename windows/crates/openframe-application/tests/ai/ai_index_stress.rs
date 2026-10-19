//! index shutdown stress: intelligence index shutdown stress (AI/RAG spec §32–§33, CONTRIBUTING §2
//! invariant 11). Repeated open, close, switch and rebuild while the embedding runtime
//! has stopped answering: closing or switching never hangs, nothing crashes, background
//! work ends, and the reopened index is consistent and valid (or rebuilt).

use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_application::events::AppEvent;
use openframe_application::modules::ai::intelligence::{self, IndexSettings, ProjectIndex};
use openframe_application::modules::ai::retrieval::IndexState;
use openframe_search::Embedder;
use openframe_test_support::TestEnv;
use openframe_test_support::embedder::ConceptEmbedder;
use serde_json::json;

/// Closing or switching a project while the embedding runtime is stalled must return
/// within this bound (the UI waits on it). `ProjectIndex::shutdown` detaches a worker
/// blocked in the embedding runtime after 150 ms, so a close takes well under 500 ms on
/// an idle machine; the bound leaves room for a loaded CI runner and still fails on any
/// wait for the runtime.
const CLOSE_BOUND: Duration = Duration::from_secs(2);
/// Everything else that waits (background work ending, the index settling) is bounded too.
const SETTLE: Duration = Duration::from_secs(60);
const ROUNDS: usize = 4;

fn settings() -> IndexSettings {
    IndexSettings {
        debounce: Duration::from_millis(0),
        max_latency: Duration::from_millis(0),
        embed_retry: Duration::from_millis(50),
        // Small batches: a rebuild needs many embedding calls, so it is mid-way when stalled.
        embed_batch: 2,
        ..IndexSettings::default()
    }
}

/// A project with enough text for many embedding batches, built through registry operations.
fn seed(env: &TestEnv) {
    let sp = env.ok(
        "screenplay.create",
        json!({ "title": "Railway", "draftName": "Draft 1" }),
    );
    let draft = sp["draft"]["id"].as_str().unwrap().to_string();
    for i in 0..8 {
        let r = env.ok(
            "screenplay.create_scene",
            json!({ "draftId": draft, "heading": format!("INT. RAILWAY STATION {i} - NIGHT") }),
        );
        let scene = r["createdIds"][0].as_str().unwrap().to_string();
        for (t, text) in [
            ("action", "Rain hammers the platform. Ravi waits alone."),
            ("character", "RAVI"),
            ("dialogue", "She said the train would come at midnight."),
        ] {
            env.ok(
                "screenplay.insert_element",
                json!({ "sceneId": scene, "elementType": t, "text": text }),
            );
        }
    }
    for name in ["Ravi", "Anjali", "Meera", "Arjun"] {
        env.ok(
            "story.create_character",
            json!({ "name": name, "description": format!("{name} distrusts the station master.") }),
        );
    }
    for i in 0..4 {
        env.ok(
            "notes.create",
            json!({ "title": format!("Note {i}"), "body": "Night shoots at the station are lonely and hard." }),
        );
    }
}

fn current(env: &TestEnv) -> Arc<ProjectIndex> {
    intelligence::service(&env.core)
        .current()
        .expect("an open project has an index")
}

fn settle(env: &TestEnv) {
    intelligence::activate(&env.core).expect("project open");
    assert!(
        intelligence::wait_idle(&env.core, SETTLE),
        "index did not settle: {:?}",
        intelligence::service(&env.core)
            .current()
            .map(|i| i.snapshot())
    );
}

fn wait_until(what: &str, mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + SETTLE;
    while !f() {
        assert!(Instant::now() < deadline, "timed out waiting: {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn task_state(env: &TestEnv, task_id: &str) -> Option<String> {
    env.sink.events.lock().iter().rev().find_map(|e| match e {
        AppEvent::Task(t) if t.task_id == task_id => Some(t.state.clone()),
        _ => None,
    })
}

/// The index file passes SQLite's own integrity check.
fn assert_index_file_valid(path: &std::path::Path) {
    let c = rusqlite::Connection::open(path).unwrap();
    openframe_search::vector::register(&c).unwrap();
    let ok: String = c
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ok, "ok", "intelligence.sqlite is damaged");
}

#[test]
fn close_switch_and_rebuild_while_the_embedding_runtime_is_stalled() {
    let env = TestEnv::with_project("Railway", "Feature Film");
    intelligence::service(&env.core).set_settings(settings());
    let e = Arc::new(ConceptEmbedder::new());
    intelligence::service(&env.core).set_embedder(Some(e.clone() as Arc<dyn Embedder>));
    seed(&env);
    settle(&env);
    let path = env.project_path();
    let index_file = std::path::PathBuf::from(&path)
        .join("cache")
        .join("intelligence.sqlite");
    let baseline = current(&env).snapshot();
    assert_eq!(baseline.state, IndexState::Current);
    assert!(baseline.counts.chunks > 8, "{baseline:?}");
    assert_eq!(baseline.counts.embedded, baseline.counts.chunks);

    // A second project to switch to (creating it switches away from the first).
    env.ok(
        "project.create",
        json!({ "title": "Other", "projectType": "Feature Film" }),
    );
    let other = env.project_path();
    env.reopen_project(&path);
    settle(&env);

    let mut slowest = Duration::ZERO;
    for round in 0..ROUNDS {
        // The embedding runtime stops answering in the middle of an explicit rebuild.
        e.set_stalled(true);
        let calls = e.calls();
        let ix = current(&env);
        let task = env.ok("ai.index_rebuild", json!({}))["taskId"]
            .as_str()
            .unwrap()
            .to_string();
        wait_until("the rebuild reaches the stalled embedding runtime", || {
            e.calls() > calls
        });
        assert_eq!(ix.snapshot().state, IndexState::Rebuilding);

        // Close (even rounds) or switch to the other project (odd rounds): bounded, no hang.
        let started = Instant::now();
        if round % 2 == 0 {
            env.ok("project.close", json!({}));
            assert!(intelligence::service(&env.core).current().is_none());
        } else {
            env.reopen_project(&other);
        }
        let took = started.elapsed();
        slowest = slowest.max(took);
        assert!(
            took < CLOSE_BOUND,
            "round {round}: closing took {took:?} while the embedding runtime was stalled"
        );

        // Reopen while the runtime is still stalled: also bounded, and editing works.
        let started = Instant::now();
        env.reopen_project(&path);
        assert!(
            started.elapsed() < CLOSE_BOUND,
            "round {round}: reopening hung"
        );
        env.ok("notes.create", json!({ "title": format!("Round {round}"), "body": "Edited while the runtime is stalled." }));

        // The abandoned rebuild ends while the runtime is still stalled: it does not wait
        // for the retired index.
        wait_until(
            "the `ai.index_rebuild` task of the closed project ends",
            || {
                matches!(
                    task_state(&env, &task).as_deref(),
                    Some("completed" | "failed" | "cancelled")
                )
            },
        );
        assert_eq!(
            task_state(&env, &task).as_deref(),
            Some("cancelled"),
            "round {round}: the rebuild of a closed project ends as cancelled, not failed"
        );
        // The runtime recovers: the detached worker exits and releases the old index.
        e.set_stalled(false);
        wait_until(
            "no background work holds the closed project's index",
            || Arc::strong_count(&ix) == 1,
        );

        // The reopened index is consistent: every document of the project, fully embedded.
        settle(&env);
        let snap = current(&env).snapshot();
        assert_eq!(snap.state, IndexState::Current, "round {round}: {snap:?}");
        assert!(snap.error.is_none(), "round {round}: {snap:?}");
        assert_eq!(
            snap.counts.documents,
            baseline.counts.documents + round as u64 + 1,
            "round {round}: one document per note added since the baseline"
        );
        assert_eq!(snap.counts.embedded, snap.counts.chunks, "round {round}");
        assert!(snap.semantic_ready, "round {round}");
        assert_eq!(
            env.ok("ai.index_status", json!({}))["state"],
            "Current",
            "round {round}"
        );
    }
    eprintln!("slowest close/switch while stalled: {slowest:?}");

    // The derived file is valid after the stress, and a final close releases it completely.
    env.ok("project.close", json!({}));
    assert_index_file_valid(&index_file);
    std::fs::remove_file(&index_file).expect("the index file is released on close");
    env.reopen_project(&path);
    settle(&env);
    let rebuilt = current(&env).snapshot();
    assert_eq!(rebuilt.state, IndexState::Current);
    assert_eq!(
        rebuilt.counts.documents,
        baseline.counts.documents + ROUNDS as u64
    );
}

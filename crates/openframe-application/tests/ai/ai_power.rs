//! Sleep and wake (`AppCore::on_power_event`, the Mac `power_event(sleeping)` and the Windows
//! power broadcast): background AI work stops before the computer sleeps and goes on by itself
//! after the wake, and a request a person is waiting for is never lost.
//!
//! No model is downloaded. A deterministic test embedder stands in for the search engine (it can
//! hold a call until the test lets it go), and a scripted chat model answers `ai.ask`.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use openframe_ai::{ChatRequest, InferenceProvider};
use openframe_application::modules::ai::intelligence::{IndexSettings, ProjectIndex};
use openframe_application::modules::ai::{self, intelligence};
use openframe_domain::{AppError, AppResult};
use openframe_platform::power::PowerEvent;
use openframe_search::Embedder;
use openframe_test_support::TestEnv;
use openframe_test_support::embedder::ConceptEmbedder;
use serde_json::json;

const SHORT: &[u8] = include_bytes!("../../../../templates/samples/Example-Short.fountain");

/// The Example Short sample, one embedding text per call so a pass has many steps.
fn project(title: &str) -> TestEnv {
    let env = TestEnv::with_project(title, "Short Film");
    let path = env.write_file("short.fountain", SHORT);
    let p = env.ok("screenplay.import_preview", json!({ "path": path }));
    env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay" }),
    );
    intelligence::service(&env.core).set_settings(IndexSettings {
        debounce: Duration::from_millis(0),
        max_latency: Duration::from_millis(0),
        embed_batch: 1,
        embed_retry: Duration::from_millis(50),
        ..IndexSettings::default()
    });
    env
}

fn attach(env: &TestEnv) -> Arc<ConceptEmbedder> {
    let e = Arc::new(ConceptEmbedder::new());
    intelligence::service(&env.core).set_embedder(Some(e.clone() as Arc<dyn Embedder>));
    e
}

fn index(env: &TestEnv) -> Arc<ProjectIndex> {
    intelligence::service(&env.core)
        .current()
        .expect("project open")
}

fn wait_for(what: &str, mut ok: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(60);
    while !ok() {
        assert!(Instant::now() < until, "timed out waiting: {what}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Long enough for a worker that was not held to make several more embedding calls.
const QUIET: Duration = Duration::from_millis(400);

#[test]
fn sleep_stops_a_running_embedding_pass_and_holds_new_passes_until_the_wake() {
    let env = project("Night Train");
    let e = attach(&env);
    // The search engine is busy with the first text of the pass when the computer sleeps.
    e.set_stalled(true);
    intelligence::activate(&env.core).unwrap();
    wait_for("the indexer reached the embedder", || e.calls() == 1);
    env.core.on_power_event(PowerEvent::Suspending);
    // The call in progress finishes and is kept; the pass stops before the next one.
    e.set_stalled(false);
    wait_for("the call in progress ended", || e.texts() == 1);
    std::thread::sleep(QUIET);
    assert_eq!(e.calls(), 1, "no embedding call after the sleep");
    let ix = index(&env);
    assert!(ix.is_paused());
    let held = ix.snapshot();
    assert!(held.pending_embeddings > 0, "the rest of the pass waits");
    assert!(
        !ix.wait_idle(Duration::from_millis(100)),
        "asleep is not idle"
    );

    // An edit made while asleep is queued, not indexed. A second sleep changes nothing.
    env.ok(
        "notes.create",
        json!({ "body": "Platform lamps going out one by one." }),
    );
    env.core.on_power_event(PowerEvent::Suspending);
    std::thread::sleep(QUIET);
    assert_eq!(e.calls(), 1, "no new pass while asleep");
    assert!(
        ix.snapshot().pending_changes > 0,
        "the edit waits for the wake"
    );

    // After the wake the index goes on by itself and catches up with everything.
    env.core.on_power_event(PowerEvent::Resumed);
    assert!(!ix.is_paused());
    assert!(ix.wait_idle(Duration::from_secs(60)), "{:?}", ix.snapshot());
    let done = ix.snapshot();
    assert_eq!(done.pending_embeddings, 0);
    assert_eq!(done.pending_changes, 0);
    assert!(done.semantic_ready);
    assert!(
        done.counts.documents > held.counts.documents,
        "the note was indexed"
    );
    assert!(e.calls() > 1);

    // A wake without a sleep is harmless.
    let calls = e.calls();
    env.core.on_power_event(PowerEvent::Resumed);
    assert!(ix.wait_idle(Duration::from_secs(60)));
    assert_eq!(e.calls(), calls, "nothing to redo");
}

#[test]
fn an_index_started_while_asleep_builds_nothing_until_the_wake() {
    let env = project("Night Ferry");
    let e = attach(&env);
    env.core.on_power_event(PowerEvent::Suspending);
    // A question in the night activates the index: it opens its file but starts no pass.
    let ix = intelligence::activate(&env.core).unwrap();
    std::thread::sleep(QUIET);
    assert_eq!(e.calls(), 0);
    assert_eq!(ix.snapshot().counts.documents, 0, "no pass while asleep");
    env.core.on_power_event(PowerEvent::Resumed);
    assert!(ix.wait_idle(Duration::from_secs(60)), "{:?}", ix.snapshot());
    let s = ix.snapshot();
    assert!(s.counts.documents > 0 && s.semantic_ready, "{s:?}");
}

#[test]
fn a_project_opened_while_asleep_starts_held() {
    let env = project("Night Bus");
    env.core.on_power_event(PowerEvent::Suspending);
    env.ok(
        "project.create",
        json!({ "title": "Opened In The Night", "projectType": "Feature Film" }),
    );
    let ix = index(&env);
    assert!(ix.is_paused(), "the new project's index inherits the sleep");
    env.core.on_power_event(PowerEvent::Resumed);
    assert!(!ix.is_paused());
}

// ------------------------------------------------------------- a request in flight

#[derive(Default)]
struct HeldModel {
    replies: Mutex<VecDeque<String>>,
    /// (entered, released)
    gate: Mutex<(bool, bool)>,
    changed: Condvar,
    calls: Mutex<usize>,
}

impl InferenceProvider for HeldModel {
    fn chat(&self, _req: &ChatRequest) -> AppResult<String> {
        let first = {
            let mut n = self.calls.lock().unwrap();
            *n += 1;
            *n == 1
        };
        if first {
            let mut g = self.gate.lock().unwrap();
            g.0 = true;
            self.changed.notify_all();
            let (_g, _) = self
                .changed
                .wait_timeout_while(g, Duration::from_secs(60), |g| !g.1)
                .unwrap();
        }
        self.replies.lock().unwrap().pop_front().ok_or_else(|| {
            AppError::ai(
                "unavailable",
                "AI is currently unavailable. OpenFrame's core workflows continue to work normally.",
            )
        })
    }
    fn model_reference(&self) -> String {
        "test-local-profile".into()
    }
    fn capabilities(&self) -> openframe_ai::ProviderCapabilities {
        openframe_ai::ProviderCapabilities {
            context_tokens: 8192,
            languages: None,
            grammar: true,
        }
    }
}

#[test]
fn an_answer_in_flight_when_the_computer_sleeps_is_finished_not_lost() {
    let env = project("Night Shift");
    let e = attach(&env);
    let m = Arc::new(HeldModel::default());
    m.replies.lock().unwrap().push_back(
        "Devi studies the register while Kiran waits on the other side of the bars (Scene 4)."
            .into(),
    );
    ai::service(&env.core).attach_model(Some(m.clone() as Arc<dyn InferenceProvider>));
    // The index is busy in the background too.
    e.set_stalled(true);
    intelligence::activate(&env.core).unwrap();
    wait_for("the indexer reached the embedder", || e.calls() == 1);
    let x = std::thread::scope(|s| {
        let asker = s.spawn(|| {
            env.ok(
                "ai.ask",
                json!({
                    "text": "Explain what happens in scene 4",
                    "scope": { "kind": "WholeProject" },
                    "conversationId": "conv-sleep",
                }),
            )
        });
        {
            let g = m.gate.lock().unwrap();
            let (g, t) = m
                .changed
                .wait_timeout_while(g, Duration::from_secs(60), |g| !g.0)
                .unwrap();
            assert!(g.0 && !t.timed_out(), "the model was never called");
        }
        // The computer sleeps while the person waits for the answer, and wakes again.
        env.core.on_power_event(PowerEvent::Suspending);
        env.core.on_power_event(PowerEvent::Suspending);
        e.set_stalled(false);
        m.gate.lock().unwrap().1 = true;
        m.changed.notify_all();
        let x = asker.join().unwrap();
        // The answer finished while the index stayed held.
        std::thread::sleep(QUIET);
        assert_eq!(e.calls(), 1, "no embedding call while asleep");
        env.core.on_power_event(PowerEvent::Resumed);
        x
    });
    let r = &x["result"];
    assert_eq!(r["kind"], "Answer", "{r}");
    assert!(
        r["content"]
            .as_str()
            .unwrap()
            .contains("Devi studies the register"),
        "{r}"
    );
    let id = x["requestId"].as_str().unwrap().to_string();
    let status: String = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(
                c.query_row("SELECT status FROM ai_request WHERE id = ?1", [&id], |r| {
                    r.get(0)
                })?,
            )
        })
        .unwrap();
    assert_eq!(status, "Completed");
    // Background work went on after the wake.
    let ix = index(&env);
    assert!(ix.wait_idle(Duration::from_secs(60)), "{:?}", ix.snapshot());
    assert!(ix.snapshot().semantic_ready);
    assert!(e.calls() > 1);
}

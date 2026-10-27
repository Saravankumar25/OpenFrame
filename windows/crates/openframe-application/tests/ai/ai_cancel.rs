//! Stopping a request through `ai.ask` and `ai.cancel` (spec `ai-conversational-harness`, section 5;
//! harness plan item 14).
//!
//! No model is downloaded. A scripted `InferenceProvider` replays one reply per call; it can also
//! hold its first call until the test lets it go, so `ai.cancel` can arrive from another thread
//! while a model call is running. The project is the Example Short sample (ten scenes, Devi and
//! Kiran).

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use openframe_ai::{ChatRequest, InferenceProvider};
use openframe_application::modules::ai;
use openframe_domain::{AppError, AppResult};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const SHORT: &[u8] = include_bytes!("../../../../templates/samples/Example-Short.fountain");

#[derive(Default)]
struct Gate {
    hold_first: bool,
    entered: bool,
    released: bool,
}

#[derive(Default)]
struct ScriptedModel {
    replies: Mutex<VecDeque<String>>,
    seen: Mutex<Vec<ChatRequest>>,
    gate: Mutex<Gate>,
    changed: Condvar,
}

impl ScriptedModel {
    fn reply(&self, r: &str) {
        self.replies.lock().unwrap().push_back(r.to_string());
    }
    fn step(&self, tool: &str, args: Value, done: bool) {
        self.reply(&json!({ "action": tool, "arguments": args, "done": done }).to_string());
    }
    fn calls(&self) -> usize {
        self.seen.lock().unwrap().len()
    }
    /// The first model call blocks until `release`.
    fn hold_first_call(&self) {
        self.gate.lock().unwrap().hold_first = true;
    }
    fn wait_until_entered(&self) {
        let g = self.gate.lock().unwrap();
        let (g, timeout) = self
            .changed
            .wait_timeout_while(g, Duration::from_secs(60), |g| !g.entered)
            .unwrap();
        assert!(
            !timeout.timed_out() && g.entered,
            "the model was never called"
        );
    }
    fn release(&self) {
        self.gate.lock().unwrap().released = true;
        self.changed.notify_all();
    }
}

impl InferenceProvider for ScriptedModel {
    fn chat(&self, req: &ChatRequest) -> AppResult<String> {
        self.seen.lock().unwrap().push(req.clone());
        {
            let mut g = self.gate.lock().unwrap();
            if g.hold_first {
                g.hold_first = false;
                g.entered = true;
                self.changed.notify_all();
                let (_g, _) = self
                    .changed
                    .wait_timeout_while(g, Duration::from_secs(60), |g| !g.released)
                    .unwrap();
            }
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

fn project() -> (TestEnv, Arc<ScriptedModel>) {
    let env = TestEnv::with_project("Lost Property", "Short Film");
    let path = env.write_file("short.fountain", SHORT);
    let p = env.ok("screenplay.import_preview", json!({ "path": path }));
    env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay" }),
    );
    let m = Arc::new(ScriptedModel::default());
    ai::service(&env.core).attach_model(Some(m.clone() as Arc<dyn InferenceProvider>));
    (env, m)
}

fn ask_in(env: &TestEnv, text: &str, conversation: Option<&str>) -> Value {
    let mut args = json!({ "text": text, "scope": { "kind": "WholeProject" } });
    if let Some(c) = conversation {
        args["conversationId"] = json!(c);
    }
    env.ok("ai.ask", args)
}

fn trace_cancelled(env: &TestEnv, x: &Value) -> bool {
    let id = x["requestId"].as_str().unwrap().to_string();
    env.core
        .with_app_db(|c| {
            Ok(c.query_row(
                "SELECT cancelled FROM sys_ai_trace WHERE request_id = ?1",
                [&id],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

fn request_status(env: &TestEnv, x: &Value) -> String {
    let id = x["requestId"].as_str().unwrap().to_string();
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(
                c.query_row("SELECT status FROM ai_request WHERE id = ?1", [&id], |r| {
                    r.get::<_, String>(0)
                })?,
            )
        })
        .unwrap()
}

fn running_rows(env: &TestEnv) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM ai_request WHERE status = 'Running'",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .unwrap()
}

#[test]
fn a_stop_from_another_thread_ends_a_written_answer_while_the_model_is_working() {
    let (env, m) = project();
    m.hold_first_call();
    m.reply("Devi studies the register while Kiran waits on the other side of the bars (Scene 4).");
    let conversation = "conv-stop-write";
    let x = std::thread::scope(|s| {
        let asker = s.spawn(|| ask_in(&env, "Explain what happens in scene 4", Some(conversation)));
        m.wait_until_entered();
        // The request is inside the model call; the stop must not wait for it.
        let r = env.ok("ai.cancel", json!({ "conversationId": conversation }));
        assert_eq!(r["stopping"], true);
        m.release();
        asker.join().unwrap()
    });
    let r = &x["result"];
    assert_eq!(r["content"], "Stopped. Nothing was changed.", "{r}");
    assert_eq!(r["kind"], "Answer");
    assert_eq!(m.calls(), 1, "no second pass after the stop");
    assert_eq!(request_status(&env, &x), "Canceled");
    assert_eq!(running_rows(&env), 0);
    assert!(trace_cancelled(&env, &x), "the trace records the stop");
    // Nothing is left to stop afterwards.
    let again = env.ok("ai.cancel", json!({ "conversationId": conversation }));
    assert_eq!(again["stopping"], false);
}

#[test]
fn a_stop_ends_a_change_request_at_the_next_step_and_keeps_what_was_found() {
    let (env, m) = project();
    m.hold_first_call();
    m.step("search_project", json!({ "text": "lantern" }), false);
    let conversation = "conv-stop-change";
    let x = std::thread::scope(|s| {
        let asker = s.spawn(|| {
            ask_in(
                &env,
                "Move the lantern scene to the end of the script",
                Some(conversation),
            )
        });
        m.wait_until_entered();
        assert_eq!(
            env.ok("ai.cancel", json!({ "conversationId": conversation }))["stopping"],
            true
        );
        m.release();
        asker.join().unwrap()
    });
    let r = &x["result"];
    let content = r["content"].as_str().unwrap();
    assert!(
        content.starts_with("Stopped. Nothing was changed."),
        "{content}"
    );
    assert_eq!(m.calls(), 1, "no second model call after the stop");
    assert_eq!(request_status(&env, &x), "Canceled");
    assert_eq!(running_rows(&env), 0);
    assert_ne!(r["kind"], "Proposal");
}

#[test]
fn a_stop_names_a_running_conversation_of_the_person_and_nothing_else() {
    let (env, _m) = project();
    let r = env.ok(
        "ai.cancel",
        json!({ "conversationId": "no-such-conversation" }),
    );
    assert_eq!(r["stopping"], false);
    assert!(
        env.call("ai.cancel", json!({})).is_err(),
        "the conversation is required"
    );
}

#[test]
fn a_finished_request_leaves_no_stop_flag_behind() {
    let (env, m) = project();
    m.reply("You are welcome. Ask me anything about the project.");
    let x = ask_in(&env, "thanks, that helps", Some("conv-done"));
    assert_eq!(x["result"]["kind"], "Answer");
    assert_eq!(
        env.ok("ai.cancel", json!({ "conversationId": "conv-done" }))["stopping"],
        false
    );
    assert_eq!(request_status(&env, &x), "Completed");
}

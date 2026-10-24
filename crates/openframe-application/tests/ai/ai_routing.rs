//! Typo-tolerant routing, intent prototypes and Cancel, through `ai.ask` and `ai.cancel` (AI harness
//! rebuild plan, section 5.2 and items 5 and 14 of section 10).
//!
//! No model is downloaded. A scripted `InferenceProvider` replays one reply per call; it can also
//! hold its first call until the test lets it go, so `ai.cancel` can arrive from another thread
//! while a model call is running. The search model is a stand-in that puts word overlap into the
//! band E5 models use. The project is the Example Short sample (ten scenes, Devi and Kiran).

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use openframe_ai::{ChatRequest, InferenceProvider};
use openframe_application::modules::ai::{self, intelligence};
use openframe_domain::{AppError, AppResult};
use openframe_search::Embedder;
use openframe_test_support::TestEnv;
use openframe_test_support::embedder::BandEmbedder;
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
    fn finish(&self, text: &str) {
        self.step("final_answer", json!({ "text": text }), true);
    }
    fn calls(&self) -> usize {
        self.seen.lock().unwrap().len()
    }
    fn request(&self, i: usize) -> ChatRequest {
        self.seen.lock().unwrap()[i].clone()
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

fn ask(env: &TestEnv, text: &str) -> Value {
    ask_in(env, text, None)
}

fn trace_of(env: &TestEnv, x: &Value) -> (String, String, f64, f64, bool, bool) {
    let id = x["requestId"].as_str().unwrap().to_string();
    env.core
        .with_app_db(|c| {
            Ok(c.query_row(
                "SELECT lane, intent, intent_score, intent_margin, intent_used, cancelled
                 FROM sys_ai_trace WHERE request_id = ?1",
                [&id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
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

fn attach_search(env: &TestEnv, failing: bool) {
    let e = BandEmbedder::new("routing");
    e.set_failing(failing);
    intelligence::service(&env.core).set_embedder(Some(Arc::new(e) as Arc<dyn Embedder>));
}

// ------------------------------------------------------------------ typo-tolerant routing

#[test]
fn a_misspelt_count_question_is_answered_by_the_count_route_with_no_model_call() {
    let (env, m) = project();
    let x = ask(&env, "how mny seens are there");
    assert_eq!(m.calls(), 0, "the route answered; the model was not asked");
    let r = &x["result"];
    assert_eq!(r["kind"], "Answer", "{r}");
    assert!(r["content"].as_str().unwrap().contains("10"), "{r}");
    assert_eq!(r["lane"], "direct");
    // The record keeps the person's own words.
    assert_eq!(x["requestText"], "how mny seens are there");
    assert_eq!(r["steps"][0]["tool"], "count_scenes");
}

#[test]
fn a_misspelt_search_question_reaches_the_read_route() {
    let (env, m) = project();
    let x = ask(&env, "Whihc scenes mention the kite?");
    assert_eq!(m.calls(), 0);
    assert_eq!(
        x["result"]["steps"][0]["tool"], "find_scenes_mentioning",
        "{x}"
    );
    assert_eq!(x["requestText"], "Whihc scenes mention the kite?");
}

#[test]
fn the_model_reads_the_persons_words_and_a_typo_never_becomes_a_proposal() {
    let (env, m) = project();
    m.finish("I can prepare that for you to review.");
    let x = ask(&env, "Add a tsak to buy tape");
    assert_eq!(m.calls(), 1, "the corrected copy is not a proposal route");
    let seen = m.request(0).messages;
    let user = &seen[seen.len() - 1].content;
    assert!(user.contains("Add a tsak to buy tape"), "{user}");
    assert_ne!(x["result"]["kind"], "Proposal");
    assert_eq!(x["requestText"], "Add a tsak to buy tape");
}

// ------------------------------------------------------------------ intent prototypes

#[test]
fn a_reworded_question_with_a_clear_intent_becomes_a_direct_call_and_is_traced() {
    let (env, m) = project();
    attach_search(&env, false);
    let x = ask(&env, "Who is the most talkative person in this film?");
    assert_eq!(m.calls(), 0, "{x}");
    let r = &x["result"];
    assert_eq!(r["steps"][0]["tool"], "dialogue_fact", "{r}");
    assert_eq!(r["lane"], "direct");
    let (lane, intent, score, margin, used, _) = trace_of(&env, &x);
    assert_eq!(
        (lane.as_str(), intent.as_str(), used),
        ("direct", "dialogue_most", true)
    );
    assert!(score >= 0.88 && margin >= 0.03, "{score} {margin}");
}

#[test]
fn an_intent_whose_arguments_code_cannot_fill_limits_the_first_step_to_its_tool() {
    let (env, m) = project();
    attach_search(&env, false);
    m.finish("Nothing mentions a bicycle.");
    let x = ask(&env, "Which scenes have a red bicycle in them?");
    assert_eq!(m.calls(), 1, "{x}");
    let (_, intent, score, margin, used, _) = trace_of(&env, &x);
    let seen = format!("intent {intent} score {score:.3} margin {margin:.3} used {used}");
    let system = &m.request(0).messages[0].content;
    assert!(system.contains("find_scenes_mentioning("), "{seen}");
    assert!(
        !system.contains("retrieve_context("),
        "the first step offers one tool ({seen})"
    );
    assert_eq!((intent.as_str(), used), ("find_scenes_mentioning", true));
}

#[test]
fn without_a_search_model_the_rules_decide_as_before() {
    let (env, m) = project();
    attach_search(&env, true);
    m.finish("Kiran talks the most.");
    let x = ask(&env, "Who is the most talkative person in this film?");
    assert!(m.calls() >= 1, "the model was asked");
    let (_, intent, _, _, used, _) = trace_of(&env, &x);
    assert_eq!((intent.as_str(), used), ("", false));
    assert_eq!(x["result"]["kind"], "Answer");
}

#[test]
fn an_unrelated_question_is_not_forced_into_an_intent() {
    let (env, m) = project();
    attach_search(&env, false);
    m.finish("I can only help with your project.");
    let x = ask(&env, "What is the weather like today?");
    let (_, _, _, _, used, _) = trace_of(&env, &x);
    assert!(!used);
    assert!(m.calls() >= 1);
}

// ------------------------------------------------------------------ Cancel

#[test]
fn a_stop_from_another_thread_ends_a_chat_request_while_the_model_is_working() {
    let (env, m) = project();
    m.hold_first_call();
    m.reply("Devi studies the register while Kiran waits on the other side of the bars (Scene 4).");
    let conversation = "conv-stop-chat";
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
    assert_eq!(request_status(&env, &x), "Canceled");
    assert_eq!(running_rows(&env), 0);
    let (_, _, _, _, _, cancelled) = trace_of(&env, &x);
    assert!(cancelled, "the trace records the stop");
    // Nothing is left to stop afterwards.
    let again = env.ok("ai.cancel", json!({ "conversationId": conversation }));
    assert_eq!(again["stopping"], false);
}

#[test]
fn a_stop_ends_an_agent_request_at_the_next_step_and_keeps_what_was_found() {
    let (env, m) = project();
    m.hold_first_call();
    m.step("count_scenes", json!({}), false);
    let conversation = "conv-stop-agent";
    let x = std::thread::scope(|s| {
        let asker = s.spawn(|| {
            ask_in(
                &env,
                "Tell me something surprising about the lanterns",
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
    assert!(
        content.contains("10"),
        "what the first step found stays: {content}"
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
    m.finish("Hello.");
    let x = ask_in(
        &env,
        "Tell me something surprising about the lanterns",
        Some("conv-done"),
    );
    assert_eq!(x["result"]["kind"], "Answer");
    assert_eq!(
        env.ok("ai.cancel", json!({ "conversationId": "conv-done" }))["stopping"],
        false
    );
    assert_eq!(request_status(&env, &x), "Completed");
}

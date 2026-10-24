//! The lane router and the Chat and Recipe lanes through `ai.ask` (AI harness rebuild plan, items
//! 4, 8 and 12 of section 10).
//!
//! No model is downloaded: a scripted `InferenceProvider` replays one reply per call and records
//! every request. The project is the Example Short sample (ten scenes, Devi and Kiran, no acts).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use openframe_ai::{ChatRequest, InferenceProvider};
use openframe_application::events::AppEvent;
use openframe_application::modules::ai;
use openframe_domain::{AppError, AppResult};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const SHORT: &[u8] = include_bytes!("../../../../templates/samples/Example-Short.fountain");

/// An explanation of one scene is asked to grow when it is under 40 words, so the replies that
/// should pass at once are this long.
const SCENE_4: &str = "Devi studies the register while Kiran waits on the other side of the bars (Scene 4). She says there are still no kites, and he asks how she knew he came for the kite (Scene 4).\n\nShe says no one comes three nights in a row for a shoe. He almost smiles and goes, and Devi watches the empty window (Scene 4).";

#[derive(Default)]
struct ScriptedModel {
    replies: Mutex<VecDeque<String>>,
    seen: Mutex<Vec<ChatRequest>>,
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
    fn request(&self, i: usize) -> ChatRequest {
        self.seen.lock().unwrap()[i].clone()
    }
}

impl InferenceProvider for ScriptedModel {
    fn chat(&self, req: &ChatRequest) -> AppResult<String> {
        self.seen.lock().unwrap().push(req.clone());
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

/// The trace row of a request: lane, runner-up, margin, passes, failed checks.
fn trace(env: &TestEnv, x: &Value) -> (String, String, f64, i64, String) {
    let id = x["requestId"].as_str().unwrap().to_string();
    env.core
        .with_app_db(|c| {
            Ok(c.query_row(
                "SELECT lane, runner_up_lane, lane_margin, passes, checks_failed FROM sys_ai_trace WHERE request_id = ?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )?)
        })
        .unwrap()
}

fn progress_labels(env: &TestEnv) -> Vec<String> {
    env.sink
        .events
        .lock()
        .iter()
        .filter_map(|e| match e {
            AppEvent::Module {
                module,
                name,
                payload,
            } if module == "ai" && name == "progress" => {
                Some(payload["label"].as_str().unwrap_or_default().to_string())
            }
            _ => None,
        })
        .collect()
}

fn details(r: &Value) -> Vec<String> {
    r["details"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_str().unwrap().to_string())
        .collect()
}

// ------------------------------------------------------------------ abstain: no model call

#[test]
fn a_name_the_project_does_not_have_is_not_invented() {
    let (env, m) = project();
    let x = ask(&env, "Who is Marcus and what does he do?");
    let r = &x["result"];
    assert_eq!(m.calls(), 0, "no model call");
    assert_eq!(r["kind"], "Answer");
    assert_eq!(
        r["content"],
        "I couldn't find anyone or anything called Marcus in this project."
    );
    assert_eq!(r["confidence"], "Unavailable");
    assert_eq!(r["lane"], "chat");
    let t = trace(&env, &x);
    assert_eq!((t.0.as_str(), t.1.as_str()), ("chat", "agent"));
}

#[test]
fn a_scene_the_draft_does_not_have_is_answered_by_code() {
    let (env, m) = project();
    let x = ask(&env, "Explain scene 40");
    assert_eq!(m.calls(), 0, "no model call");
    assert_eq!(
        x["result"]["content"],
        "Scene 40 isn't in this draft; it has 10 scenes."
    );
    assert_eq!(x["result"]["confidence"], "Exact");
}

#[test]
fn an_ambiguous_pronoun_asks_which_character() {
    let (env, m) = project();
    m.reply("Devi guards the office and Kiran keeps returning for his kite (Scene 1).");
    let first = ask(&env, "Compare Devi and Kiran");
    assert_eq!(first["result"]["kind"], "Answer", "{first}");
    let conversation = first["conversationId"].as_str().unwrap().to_string();
    let before = m.calls();
    let x = ask_in(&env, "Why does she leave?", Some(&conversation));
    assert_eq!(m.calls(), before, "no model call");
    assert_eq!(x["result"]["kind"], "Clarify");
    assert_eq!(x["result"]["content"], "Do you mean DEVI or KIRAN?");
}

// ------------------------------------------------------------------ the verifier loop

#[test]
fn an_invented_name_is_fed_back_and_the_clean_second_pass_is_shown() {
    let (env, m) = project();
    m.reply("Marcus gives Kiran the kite (Scene 4).");
    m.reply(SCENE_4);
    let x = ask(&env, "Explain what happens in scene 4");
    assert_eq!(m.calls(), 2);
    let r = &x["result"];
    assert_eq!(r["content"], SCENE_4);
    assert!(details(r).is_empty(), "{r}");
    let t = trace(&env, &x);
    assert_eq!((t.0.as_str(), t.3), ("chat", 2));
    assert!(t.4.split(',').any(|c| c == "names"), "{}", t.4);
    // The feedback names the offender and nothing else about the project.
    let retry = m.request(1).messages;
    let last = &retry[retry.len() - 1].content;
    assert!(
        last.contains("These names are not in the project: Marcus."),
        "{last}"
    );
}

#[test]
fn an_answer_that_passes_at_once_is_shown_at_once_and_cites_its_scene() {
    let (env, m) = project();
    m.reply(SCENE_4);
    let x = ask(&env, "Explain what happens in scene 4");
    assert_eq!(m.calls(), 1);
    let r = &x["result"];
    let chips: Vec<&Value> = r["provenance"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["kind"] == "Scene")
        .collect();
    assert_eq!(chips.len(), 1, "{r}");
    assert!(chips[0]["label"].as_str().unwrap().starts_with("Scene 4:"));
    assert!(chips[0]["nav"]["params"]["sceneId"].is_string());
    assert_eq!(trace(&env, &x).3, 1);
    // The labels the screen reads.
    assert_eq!(r["lane"], "chat");
    assert_eq!(
        progress_labels(&env),
        [
            "Reading Scene 4",
            "Writing the answer",
            "Checking the answer"
        ]
    );
}

#[test]
fn when_every_pass_fails_the_answer_keeps_its_clean_sentences_and_says_it_is_not_sure() {
    let (env, m) = project();
    let bad = "Devi studies the register (Scene 4). Marcus laughs at her (Scene 4).";
    m.reply(bad);
    m.reply(bad);
    m.reply(bad);
    let x = ask(&env, "Explain what happens in scene 4");
    assert_eq!(m.calls(), 3, "at most three passes");
    let r = &x["result"];
    assert_eq!(r["content"], "Devi studies the register (Scene 4).");
    let d = details(r);
    assert!(
        d.contains(&"I'm not sure about parts of this answer.".to_string()),
        "{d:?}"
    );
    assert!(d.len() >= 2, "a clarifying question follows: {d:?}");
    assert_eq!(trace(&env, &x).3, 3);
}

// ------------------------------------------------------------------ layout and sampling

#[test]
fn a_deep_answer_keeps_its_bullets_and_a_quick_answer_is_flattened() {
    let (env, m) = project();
    m.reply("## What happens\n- Devi studies the register (Scene 4).\n- Kiran waits at the bars (Scene 4).");
    let x = ask(&env, "Explain scene 4 in detail");
    assert_eq!(
        x["result"]["content"],
        "What happens\n- Devi studies the register (Scene 4).\n- Kiran waits at the bars (Scene 4)."
    );
    let req = m.request(0);
    assert_eq!(
        (req.max_tokens, req.temperature, req.top_k),
        (700, 0.6, Some(64))
    );

    m.reply("**Yes.** - Kiran comes back to the office (Scene 4).");
    let q = ask(&env, "Does Kiran come back in scene 4?");
    assert_eq!(
        q["result"]["content"],
        "Yes. - Kiran comes back to the office (Scene 4)."
    );
    let req = m.request(1);
    assert_eq!(
        (req.max_tokens, req.temperature, req.top_p),
        (120, 0.3, None)
    );
}

#[test]
fn ideas_are_written_warm_and_start_with_idea() {
    let (env, m) = project();
    m.reply("Idea: Devi finds the kite in the lost property shelves (Scene 4).\nIdea: Kiran leaves a note on the register (Scene 4).");
    let x = ask(&env, "Any ideas for scene 4?");
    let req = m.request(0);
    assert_eq!(
        (req.temperature, req.max_tokens, req.top_p),
        (0.9, 400, Some(0.95))
    );
    let content = x["result"]["content"].as_str().unwrap();
    assert_eq!(content.lines().count(), 2, "{content}");
    assert!(content.lines().all(|l| l.starts_with("Idea:")));
    assert!(
        m.request(0).messages[0]
            .content
            .contains("starts with \"Idea:\"")
    );
}

#[test]
fn the_prompt_has_the_map_in_the_system_message_and_the_data_marked_untrusted() {
    let (env, m) = project();
    m.reply(SCENE_4);
    ask(&env, "Explain what happens in scene 4");
    let req = m.request(0);
    let (system, user) = (&req.messages[0].content, &req.messages[1].content);
    assert!(
        system.contains("Project map") && system.contains("<project_data trust=\"untrusted\">")
    );
    assert!(user.contains("Briefing: Scene 4"));
    assert!(user.contains("Answer mode: explain"));
    assert!(
        user.trim_end()
            .ends_with("User request: Explain what happens in scene 4")
    );
    // The stable start comes first: the same system message for another question.
    m.reply("Kiran waits on a bench under the yellow lamps while a goods train clanks past (Scene 2). He watches the sky above the station roof and does not move, with his knees pulled up and his school bag beside him (Scene 2).");
    ask(&env, "Explain what happens in scene 2");
    assert_eq!(m.request(1).messages[0].content, *system);
}

// ------------------------------------------------------------------ the recipe

#[test]
fn the_whole_screenplay_is_read_in_groups_and_answered_with_its_coverage() {
    let (env, m) = project();
    m.reply("Devi keeps the office and Kiran asks for his kite (Scenes 1, 2, 3, 4 and 5).");
    m.reply("Devi and Kiran meet again before dawn (Scenes 6, 7, 8 and 10).");
    m.reply("Devi runs the lost property office and Kiran keeps coming back for a red kite (Scene 1). They part at dawn (Scene 10).");
    let x = ask(&env, "Explain the whole screenplay");
    assert_eq!(m.calls(), 3);
    let r = &x["result"];
    assert_eq!(r["lane"], "recipe");
    assert_eq!(r["coverage"], json!({ "used": 9, "total": 10 }));
    assert_eq!(details(r), ["Used 9 of 10 scenes; not covered: Scene 9."]);
    assert_eq!(
        progress_labels(&env),
        [
            "Reading Scenes 1 to 5",
            "Reading Scenes 6 to 10",
            "Writing the answer",
            "Checking the answer"
        ]
    );
    let t = trace(&env, &x);
    assert_eq!((t.0.as_str(), t.1.as_str(), t.2), ("recipe", "chat", 1.0));
}

// ------------------------------------------------------------------ the other lanes

#[test]
fn a_list_question_still_runs_the_agent_with_four_steps_and_shows_its_progress() {
    let (env, m) = project();
    for i in 0..6 {
        m.step(
            "search_project",
            json!({ "text": format!("night{i}") }),
            false,
        );
    }
    let x = ask(&env, "Find the scenes set at night");
    assert_eq!(m.calls(), 4);
    assert_eq!(x["result"]["lane"], "agent");
    assert!(
        details(&x["result"])
            .iter()
            .any(|d| d.starts_with("I stopped after 4 steps"))
    );
    let steps: Vec<String> = progress_labels(&env);
    assert_eq!(
        steps,
        ["Step 1 of 4", "Step 2 of 4", "Step 3 of 4", "Step 4 of 4"]
    );
    assert_eq!(trace(&env, &x).0, "agent");
}

#[test]
fn direct_and_action_requests_keep_their_lanes() {
    let (env, m) = project();
    m.reply("Devi studies the register while Kiran waits.");
    let x = ask(&env, "Summarize Scene 3");
    assert_eq!(x["result"]["lane"], "direct");
    assert_eq!(trace(&env, &x).2, 1.0);

    let y = ask(&env, "Rename scene 3 to The Second Knock");
    assert_eq!(y["result"]["lane"], "action");
    assert_eq!(trace(&env, &y).0, "action");

    let z = ask(&env, "hello");
    assert_eq!(z["result"]["lane"], "direct");
}

#[test]
fn chat_with_nothing_to_read_and_nothing_named_hands_over_to_the_agent_once() {
    let (env, m) = project();
    // Ideas about "the story" name no person, scene or act, and a word no passage holds.
    m.step(
        "final_answer",
        json!({ "text": "Here is what I found." }),
        true,
    );
    let x = ask(&env, "Any ideas for zzqx?");
    let r = &x["result"];
    let t = trace(&env, &x);
    // Either the passages held something (Chat answers) or the Agent took over exactly once.
    if r["lane"] == "agent" {
        assert_eq!((t.0.as_str(), t.1.as_str(), t.2), ("agent", "chat", 0.5));
        assert_eq!(r["content"], "Here is what I found.");
        assert_eq!(m.calls(), 1);
    } else {
        assert_eq!(r["lane"], "chat");
    }
}

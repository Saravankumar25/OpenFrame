//! The reference resolver and the conversation focus through `ai.ask` (spec
//! `ai-conversational-harness`, slice S3): a project with two screenplays no longer asks "which
//! one do you mean" when the title, the focus or the scene number settles it, and the answer to the
//! question is kept as the focus.
//!
//! No model is downloaded and none is called: the questions below are answered by code.

use std::sync::Arc;

use openframe_ai::{ChatRequest, InferenceProvider};
use openframe_application::modules::ai;
use openframe_domain::AppResult;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const SHORT: &[u8] = include_bytes!("../../../../templates/samples/Example-Short.fountain");

/// A model that always writes the same sentence: the number is the project's own, so it passes the
/// checks for a count, and for every other message it is a reply the checks refuse.
struct Fixed;

impl InferenceProvider for Fixed {
    fn chat(&self, _: &ChatRequest) -> AppResult<String> {
        Ok("There are 10 scenes.".into())
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

/// Two screenplays with the same ten scenes: "Lost Property" and "Lost Property teaser".
fn two_screenplays() -> TestEnv {
    let env = TestEnv::with_project("Lost Property", "Short Film");
    for _ in 0..2 {
        let path = env.write_file("short.fountain", SHORT);
        let p = env.ok("screenplay.import_preview", json!({ "path": path }));
        env.ok(
            "screenplay.import_apply",
            json!({ "previewId": p["previewId"], "mode": "new_screenplay" }),
        );
    }
    let drafts = env.ok("production.drafts", json!({}));
    let ids: Vec<(String, String)> = drafts["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["screenplayId"].as_str().unwrap().to_string(),
                d["screenplayTitle"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(ids.len(), 2, "{drafts}");
    env.ok(
        "screenplay.rename",
        json!({ "screenplayId": ids[0].0, "title": "Lost Property" }),
    );
    env.ok(
        "screenplay.rename",
        json!({ "screenplayId": ids[1].0, "title": "Night Shift" }),
    );
    ai::service(&env.core).attach_model(Some(Arc::new(Fixed) as Arc<dyn InferenceProvider>));
    env
}

fn ask_in(env: &TestEnv, text: &str, conversation: &str) -> Value {
    env.ok(
        "ai.ask",
        json!({ "text": text, "scope": { "kind": "WholeProject" }, "conversationId": conversation }),
    )
}

fn content(x: &Value) -> String {
    x["result"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn focus_of(env: &TestEnv, x: &Value) -> Value {
    let id = x["requestId"].as_str().unwrap().to_string();
    let json: String = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT scope_json FROM ai_request WHERE id = ?1",
                [&id],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    serde_json::from_str::<Value>(&json).unwrap()["focus"].clone()
}

/// A title in a longer question goes to the model, which this test does not have: the request
/// fails, but its record already holds the focus the message gave.
#[test]
fn a_title_in_the_message_becomes_the_focus_of_the_conversation() {
    let env = two_screenplays();
    let _ = env.core.dispatch(
        &env.core.local_actor(),
        "ai.ask",
        json!({ "text": "How many scenes does Night Shift have?", "scope": { "kind": "WholeProject" }, "conversationId": "c-title" }),
    );
    let (scope_json, night): (String, String) = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok((
                c.query_row(
                    "SELECT scope_json FROM ai_request WHERE session_id = 'c-title'",
                    [],
                    |r| r.get(0),
                )?,
                c.query_row(
                    "SELECT id FROM screenplay WHERE title = 'Night Shift'",
                    [],
                    |r| r.get(0),
                )?,
            ))
        })
        .unwrap();
    let focus = serde_json::from_str::<Value>(&scope_json).unwrap()["focus"].clone();
    assert_eq!(focus["screenplay"], night, "{focus}");
}

#[test]
fn the_answer_to_the_question_is_kept_and_not_asked_again() {
    let env = two_screenplays();
    // A count of scenes reads every screenplay and is not asked about (S3 fix round), so this uses a
    // question that needs exactly one screenplay.
    let first = ask_in(&env, "Describe the scenes.", "c-ask");
    assert!(
        content(&first).contains("several screenplays") || content(&first).contains("Which"),
        "nothing settles the first question, so it is asked: {}",
        content(&first)
    );
    assert!(focus_of(&env, &first)["screenplay"].is_null());

    // The person answers with a title.
    let reply = ask_in(&env, "Night Shift", "c-ask");
    let focus = focus_of(&env, &reply);
    assert!(focus["screenplay"].is_string(), "{focus}");

    // The same question now has its answer, in the same conversation.
    let again = ask_in(&env, "Describe the scenes.", "c-ask");
    assert!(
        !content(&again).contains("several screenplays"),
        "asked again: {}",
        content(&again)
    );
    assert_ne!(again["result"]["kind"], "Clarify", "{again}");
    assert_eq!(focus_of(&env, &again)["screenplay"], focus["screenplay"]);

    // Another conversation starts with no focus.
    let other = ask_in(&env, "Describe the scenes.", "c-other");
    assert!(content(&other).contains("several screenplays") || content(&other).contains("Which"));
}

/// Ask, and when the model-less test run fails the request, still return its recorded id.
fn ask_lenient(env: &TestEnv, text: &str, conversation: &str) -> Value {
    let asked = env.core.dispatch(
        &env.core.local_actor(),
        "ai.ask",
        json!({ "text": text, "scope": { "kind": "WholeProject" }, "conversationId": conversation }),
    );
    if let Ok(x) = asked {
        return x;
    }
    let id: String = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT id FROM ai_request WHERE session_id = ?1 AND request_text = ?2",
                [conversation, text],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    json!({ "requestId": id })
}

/// The numbers the AI trace kept for a request about the answer-type stage (slice S4).
fn trace_turn(env: &TestEnv, x: &Value) -> (String, String, String, String, String) {
    let id = x["requestId"].as_str().unwrap().to_string();
    env.core
        .with_app_db(|c| {
            Ok(c.query_row(
                "SELECT answer_type, answer_group, turn_domains, length_class, turn_how
                 FROM sys_ai_trace WHERE request_id = ?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )?)
        })
        .unwrap()
}

#[test]
fn the_answer_type_is_read_stored_in_the_focus_and_kept_in_the_trace() {
    let env = two_screenplays();
    let first = ask_lenient(&env, "How many scenes are there?", "c-turn");
    assert_eq!(focus_of(&env, &first)["answerType"], "count");
    let (t, g, domains, class, how) = trace_turn(&env, &first);
    assert_eq!(
        (t.as_str(), g.as_str(), class.as_str(), how.as_str()),
        ("count", "facts", "one_sentence", "rule")
    );
    assert!(domains.contains("screenplay"), "{domains}");

    // A correction that asks for nothing new takes the type of the message before it.
    let again = ask_lenient(&env, "I said Night Shift, not the other one", "c-turn");
    assert_eq!(focus_of(&env, &again)["answerType"], "count");
    let (t, _, _, _, how) = trace_turn(&env, &again);
    assert_eq!((t.as_str(), how.as_str()), ("count", "repair"));

    // The length class moves with the words, and another conversation starts with no type.
    let brief = ask_lenient(&env, "Explain the first act briefly", "c-turn-2");
    let (t, g, _, class, _) = trace_turn(&env, &brief);
    assert_eq!(
        (t.as_str(), g.as_str(), class.as_str()),
        ("explain", "narrative", "short")
    );
    let hello = ask_lenient(&env, "thanks, that is all", "c-turn-3");
    let (t, g, domains, class, _) = trace_turn(&env, &hello);
    assert_eq!(
        (t.as_str(), g.as_str(), domains.as_str(), class.as_str()),
        ("chat", "chat", "", "brief")
    );
}

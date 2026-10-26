//! The reference resolver and the conversation focus through `ai.ask` (spec
//! `ai-conversational-harness`, slice S3): a project with two screenplays no longer asks "which
//! one do you mean" when the title, the focus or the scene number settles it, and the answer to the
//! question is kept as the focus.
//!
//! No model is downloaded and none is called: the questions below are answered by code.

use std::sync::Arc;

use openframe_ai::{ChatRequest, InferenceProvider};
use openframe_application::modules::ai;
use openframe_domain::{AppError, AppResult};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const SHORT: &[u8] = include_bytes!("../../../../templates/samples/Example-Short.fountain");

struct NoModel;

impl InferenceProvider for NoModel {
    fn chat(&self, _: &ChatRequest) -> AppResult<String> {
        Err(AppError::ai("unavailable", "no model in this test"))
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
    ai::service(&env.core).attach_model(Some(Arc::new(NoModel) as Arc<dyn InferenceProvider>));
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
    let first = ask_in(&env, "How many scenes are there?", "c-ask");
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
    let again = ask_in(&env, "How many scenes are there?", "c-ask");
    assert!(
        !content(&again).contains("several screenplays"),
        "asked again: {}",
        content(&again)
    );
    assert!(content(&again).contains("10"), "{}", content(&again));

    // Another conversation starts with no focus.
    let other = ask_in(&env, "How many scenes are there?", "c-other");
    assert!(content(&other).contains("several screenplays") || content(&other).contains("Which"));
}

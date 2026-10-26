//! The assistant's memory through `ai.ask`, `ai.memory_list` and `ai.memory_delete` (spec
//! `ai-conversational-harness`, slice S6): what the person states outright or confirms is kept,
//! listed and deletable; questions are not kept; the counts go into the request record.
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

fn project() -> TestEnv {
    let env = TestEnv::with_project("Lost Property", "Short Film");
    let path = env.write_file("short.fountain", SHORT);
    let p = env.ok("screenplay.import_preview", json!({ "path": path }));
    env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay" }),
    );
    ai::service(&env.core).attach_model(Some(Arc::new(Fixed) as Arc<dyn InferenceProvider>));
    env
}

fn say(env: &TestEnv, text: &str, conversation: &str) -> Value {
    // A statement may go to the model, which this test does not have; the request is still recorded.
    let asked = env.core.dispatch(
        &env.core.local_actor(),
        "ai.ask",
        json!({ "text": text, "scope": { "kind": "WholeProject" }, "conversationId": conversation }),
    );
    asked.unwrap_or_else(|_| json!({}))
}

fn remembered(env: &TestEnv) -> Vec<String> {
    env.ok("ai.memory_list", json!({}))["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["text"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_fact_or_preference_the_person_states_is_remembered_and_a_question_is_not() {
    let env = project();
    say(&env, "I'm the assistant director", "c1");
    say(&env, "Always answer in short sentences", "c1");
    say(&env, "Who is the assistant director?", "c1");
    say(&env, "Remember that my email is raj@example.com", "c1");
    let list = env.ok("ai.memory_list", json!({}));
    let entries = list["entries"].as_array().unwrap();
    let texts: Vec<&str> = entries
        .iter()
        .map(|e| e["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        texts.len(),
        2,
        "{texts:?}: the question and the contact data are not kept"
    );
    assert!(texts.contains(&"I'm the assistant director"));
    assert!(texts.contains(&"Always answer in short sentences"));
    let pref = entries
        .iter()
        .find(|e| e["text"] == "Always answer in short sentences")
        .unwrap();
    assert_eq!(pref["kind"], "preference");
}

#[test]
fn confirming_an_answer_remembers_it_and_deleting_forgets_it() {
    let env = project();
    let answer = say(&env, "How many scenes are there?", "c2");
    let shown = answer["result"]["content"].as_str().unwrap().to_string();
    assert!(shown.contains("10"), "{shown}");
    say(&env, "yes that's right", "c2");
    let texts = remembered(&env);
    assert_eq!(texts, vec![shown.clone()]);

    // Saying it again does not add a copy.
    say(&env, "How many scenes are there?", "c2");
    say(&env, "correct, remember that", "c2");
    assert_eq!(remembered(&env).len(), 1);

    let id = env.ok("ai.memory_list", json!({}))["entries"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let gone = env.ok("ai.memory_delete", json!({ "id": id }));
    assert_eq!(gone["deleted"], true);
    assert!(remembered(&env).is_empty());
    assert_eq!(
        env.ok("ai.memory_delete", json!({ "id": id }))["deleted"],
        false,
        "already gone"
    );
}

#[test]
fn a_remembered_entry_is_counted_in_the_next_request_record() {
    let env = project();
    say(&env, "Call me Saravan", "c3");
    let x = say(&env, "How many scenes are there?", "c3");
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
    let memory = serde_json::from_str::<Value>(&json).unwrap()["memory"].clone();
    assert_eq!(memory["remembered"], 1, "{memory}");
    assert_eq!(memory["recent"], 1, "{memory}");
}

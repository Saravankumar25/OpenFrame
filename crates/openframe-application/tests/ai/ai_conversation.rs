//! The conversational turn through `ai.ask` (spec `ai-conversational-harness`, slice S7): resolve,
//! read the answer type, fetch the facts, write, check, fall back, show sources.
//!
//! No model is downloaded. A scripted `InferenceProvider` replays one reply per call and records every
//! request, so these tests pin down what the APPLICATION puts in the prompt and does with any reply.
//! The project is two screenplays with the same ten scenes ("Lost Property" and "Night Shift"), a
//! story, a cast and crew with contact data, a budget and private notes.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use openframe_ai::{ChatRequest, InferenceProvider};
use openframe_application::modules::ai;
use openframe_domain::{AppError, AppResult, Role};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const SHORT: &[u8] = include_bytes!("../../../../templates/samples/Example-Short.fountain");

#[derive(Default)]
struct Scripted {
    replies: Mutex<VecDeque<String>>,
    seen: Mutex<Vec<ChatRequest>>,
}

impl Scripted {
    fn reply(&self, r: &str) {
        self.replies.lock().unwrap().push_back(r.to_string());
    }
    fn times(&self, n: usize, r: &str) {
        for _ in 0..n {
            self.reply(r);
        }
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
    /// Every message of every request, as one text.
    fn all_text(&self) -> String {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .flat_map(|r| r.messages.iter().map(|m| m.content.clone()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl InferenceProvider for Scripted {
    fn chat(&self, req: &ChatRequest) -> AppResult<String> {
        self.seen.lock().unwrap().push(req.clone());
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::ai("unavailable", "the scripted model has no more replies"))
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

fn id_of(v: &Value) -> String {
    v["id"]
        .as_str()
        .or_else(|| v.as_str())
        .unwrap_or_else(|| panic!("no id in {v}"))
        .to_string()
}

fn world() -> (TestEnv, Arc<Scripted>) {
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
    let ids: Vec<String> = drafts["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["screenplayId"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids.len(), 2, "{drafts}");
    env.ok(
        "screenplay.rename",
        json!({ "screenplayId": ids[0], "title": "Lost Property" }),
    );
    env.ok(
        "screenplay.rename",
        json!({ "screenplayId": ids[1], "title": "Night Shift" }),
    );
    let anna = id_of(&env.ok(
        "story.create_character",
        json!({ "name": "Anna", "roleLabel": "Lead", "description": "A courier who finds a bag." }),
    ));
    let ben = id_of(&env.ok(
        "story.create_character",
        json!({ "name": "Ben", "roleLabel": "Supporting", "description": "The porter." }),
    ));
    env.ok(
        "story.create_relationship",
        json!({ "fromCharacterId": ben, "toCharacterId": anna, "relationshipType": "brother of", "note": "Ben is Anna's older brother." }),
    );
    env.ok(
        "cast.create",
        json!({ "personName": "Maya Rao", "characterName": "Anna", "primary": true,
                "contact": "Phone +91 90000 11111; email maya.rao@lostproperty.example",
                "availabilityNotes": "Whole shoot" }),
    );
    let budget = id_of(&env.ok("budget.create", json!({ "currency": "INR" })));
    for (cat, name, planned, actual) in [
        ("Cast", "Lead cast", 400_000_000_i64, Some(385_000_000_i64)),
        ("Equipment", "Camera", 180_000_000, Some(192_500_000)),
    ] {
        env.ok(
            "budget.add_line",
            json!({ "budgetId": budget, "category": cat, "description": name, "planned": planned, "actual": actual }),
        );
    }
    let m = Arc::new(Scripted::default());
    ai::service(&env.core).attach_model(Some(m.clone() as Arc<dyn InferenceProvider>));
    (env, m)
}

fn ask_in(env: &TestEnv, text: &str, conversation: &str) -> Value {
    env.ok(
        "ai.ask",
        json!({ "text": text, "scope": { "kind": "WholeProject" }, "conversationId": conversation }),
    )
}

fn ask(env: &TestEnv, text: &str) -> Value {
    env.ok(
        "ai.ask",
        json!({ "text": text, "scope": { "kind": "WholeProject" } }),
    )
}

fn content(x: &Value) -> String {
    x["result"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// The row the AI trace kept for a request: (path, fallback, passes, checks failed).
fn trace(env: &TestEnv, x: &Value) -> (String, String, i64, String) {
    let id = x["requestId"].as_str().unwrap().to_string();
    env.core
        .with_app_db(|c| {
            Ok(c.query_row(
                "SELECT lane, runner_up_lane, passes, checks_failed FROM sys_ai_trace WHERE request_id = ?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?)
        })
        .unwrap()
}

fn stages(env: &TestEnv, x: &Value) -> Value {
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
    serde_json::from_str::<Value>(&json).unwrap()["stages"].clone()
}

fn scene_id_of(env: &TestEnv, title: &str, position: i64) -> String {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT s.id FROM screenplay_scene s
                 JOIN screenplay_draft d ON d.id = s.draft_id
                 JOIN screenplay p ON p.id = d.screenplay_id
                 WHERE p.title = ?1 AND s.position = ?2 AND s.deleted_at IS NULL",
                rusqlite::params![title, position],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

// ------------------------------------------------------------------ the five behaviours

#[test]
fn several_screenplays_are_answered_without_asking_which_one() {
    let (env, m) = world();
    m.reply("Night Shift has 10 scenes.");
    let first = ask_in(&env, "How many scenes does Night Shift have?", "c-two");
    assert_eq!(first["result"]["kind"], "Answer", "{first}");
    assert_eq!(content(&first), "Night Shift has 10 scenes.");
    assert!(
        m.request(0).messages[1].content.contains("Night Shift"),
        "the facts are about the screenplay the message named"
    );
    // The screenplay is now the focus of the conversation: no question is asked again.
    m.reply("There are 10 scenes.");
    let again = ask_in(&env, "And how many scenes are there?", "c-two");
    assert_eq!(again["result"]["kind"], "Answer", "{again}");
    assert_eq!(m.calls(), 2);
    // Another conversation has no focus and is asked one specific question, once, by code. (A plain
    // count of scenes reads every screenplay and is not asked about, so this needs exactly one.)
    let other = ask_in(&env, "Describe the scenes.", "c-other");
    assert_eq!(other["result"]["kind"], "Clarify", "{other}");
    assert!(content(&other).contains("Which screenplay"), "{other}");
    assert_eq!(
        m.calls(),
        2,
        "the question is asked by code, not by the model"
    );
    assert_eq!(trace(&env, &other).0, "fixed");
}

#[test]
fn an_angry_message_gets_a_brief_calm_reply_and_no_project_data() {
    let (env, m) = world();
    m.reply("I am sorry about that. Tell me what you need and I will try again.");
    let x = ask(&env, "this is useless");
    assert_eq!(x["result"]["kind"], "Answer");
    let c = content(&x);
    assert!(c.starts_with("I am sorry"), "{c}");
    assert!(c.split_whitespace().count() <= 40);
    let prompt = m.request(0).messages[1].content.clone();
    assert!(
        !prompt.contains("<facts>"),
        "no data reaches a chat reply: {prompt}"
    );
    assert!(!prompt.contains("Lead cast") && !prompt.contains("Maya Rao"));
    assert!(
        m.request(0).messages[0]
            .content
            .contains("not a question about the project")
    );
    assert_eq!(m.calls(), 1);
    // A reply that lists project data is refused, three times, and code answers briefly.
    m.times(3, "Sorry. You have 10 scenes and Maya Rao is cast.");
    let x = ask(&env, "this is useless");
    let c = content(&x);
    assert!(c.starts_with("Sorry about that"), "{c}");
    assert!(!c.contains("Maya"), "{c}");
}

#[test]
fn a_missing_wardrobe_says_none_is_recorded_and_a_made_up_one_is_never_shown() {
    let (env, m) = world();
    m.reply("The project has none recorded for the wardrobe of scene 3.");
    let x = ask(&env, "What is the wardrobe for scene 3 of Lost Property?");
    assert_eq!(
        content(&x),
        "The project has none recorded for the wardrobe of scene 3."
    );
    let prompt = m.request(0).messages[1].content.clone();
    assert!(
        prompt.contains(
            "Not found:\n- I could not find any wardrobe for scene 3. I looked in the Breakdown."
        ),
        "code decided it is missing and said so: {prompt}"
    );
    assert_eq!(m.calls(), 1);

    m.times(3, "The wardrobe is a blue raincoat.");
    let x = ask(&env, "What is the wardrobe for scene 3 of Lost Property?");
    assert_eq!(
        content(&x),
        "I could not find any wardrobe for scene 3. I looked in the Breakdown."
    );
    let (path, fallback, passes, failed) = trace(&env, &x);
    assert_eq!((path.as_str(), passes), ("write", 3));
    assert!(
        fallback == "last_resort" || fallback == "abstained",
        "{fallback}"
    );
    assert!(failed.contains("abstain"), "{failed}");
}

#[test]
fn a_navigate_message_opens_the_scene_of_the_screenplay_it_names() {
    let (env, m) = world();
    let x = ask_in(&env, "Open scene 4 of Night Shift", "c-nav");
    assert_eq!(x["result"]["kind"], "Navigate", "{x}");
    assert_eq!(
        x["result"]["nav"]["params"]["sceneId"],
        scene_id_of(&env, "Night Shift", 4).as_str()
    );
    assert_eq!(x["operationClass"], "Navigate");
    assert_eq!(m.calls(), 0, "navigation is code: the target is named");
    assert_eq!(trace(&env, &x).0, "navigate");
    // A correction names the other screenplay and still opens a scene, not an explanation.
    let x = ask_in(&env, "scene 4 of Lost Property", "c-nav");
    assert_eq!(x["result"]["kind"], "Navigate", "{x}");
    assert_eq!(
        x["result"]["nav"]["params"]["sceneId"],
        scene_id_of(&env, "Lost Property", 4).as_str()
    );
    // A scene the screenplay does not have is told, not guessed.
    let x = ask(&env, "Open scene 46 of Lost Property");
    assert!(
        content(&x).contains("46") || content(&x).contains("no Scene"),
        "{x}"
    );
    assert!(x["result"]["nav"].is_null());
    assert_eq!(m.calls(), 0);
}

#[test]
fn a_change_message_gives_a_proposal_that_changes_nothing_until_applied() {
    let (env, m) = world();
    let tasks = |env: &TestEnv| -> i64 {
        env.core
            .project()
            .unwrap()
            .store
            .read(|c| {
                Ok(c.query_row(
                    "SELECT count(*) FROM task WHERE deleted_at IS NULL",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap()
    };
    let x = ask(&env, "Create a task to scout Central Station.");
    assert_eq!(x["result"]["kind"], "Proposal", "{x}");
    assert_eq!(m.calls(), 0, "a plain command is its tool call");
    assert_eq!(trace(&env, &x).0, "change");
    assert_eq!(tasks(&env), 0);
    // Anything else runs the step loop, restricted to proposal tools.
    m.step(
        "propose_task",
        json!({ "title": "Confirm the station permit" }),
        true,
    );
    let x = ask(&env, "Set up a task about the station permit");
    assert_eq!(x["result"]["kind"], "Proposal", "{x}");
    assert_eq!(x["operationClass"], "Mutate");
    let offered = m.request(0).messages[0].content.clone();
    assert!(offered.contains("propose_task"), "{offered}");
    assert!(
        !offered.contains("count_scenes") && !offered.contains("scene_fact"),
        "the loop offers proposals and two lookups, not the read catalogue: {offered}"
    );
    assert_eq!(tasks(&env), 0, "planning never mutates");
    env.ok(
        "ai.change_set.accept",
        json!({ "id": id_of(&x["result"]["changeSet"]) }),
    );
    assert_eq!(tasks(&env), 1);
}

// ------------------------------------------------------------------ privacy, end to end

#[test]
fn another_persons_private_note_never_reaches_the_prompt_but_your_own_does() {
    let (env, m) = world();
    let scene = scene_id_of(&env, "Lost Property", 2);
    let other = env.other_user(Role::Editor);
    env.call_as(
        &other,
        "private_note.create",
        json!({ "body": "SECRET-OTHER the director will cut this scene", "targetType": "screenplay_scene", "targetId": scene }),
    )
    .unwrap();
    env.ok(
        "private_note.create",
        json!({ "body": "MINE remember the rain", "targetType": "screenplay_scene", "targetId": scene }),
    );
    m.times(3, "One option is that Kiran waits in the rain.");
    ask(&env, "Suggest improvements for scene 2 of Lost Property");
    let all = m.all_text();
    assert!(!all.contains("SECRET-OTHER"), "{all}");
    assert!(
        all.contains("MINE remember the rain"),
        "your own note is yours to use"
    );
    // Nor does it come back through the history.
    assert!(
        !env.ok("ai.history", json!({}))
            .to_string()
            .contains("SECRET-OTHER")
    );
}

#[test]
fn a_viewer_gets_no_contact_in_the_prompt_or_the_answer_and_an_owner_does() {
    let (env, m) = world();
    let viewer = env.actor_with_role(Role::Viewer);
    // The model "remembers" a number it was never given: the check refuses it every time.
    m.times(3, "Maya Rao can be reached on +91 90000 11111.");
    let x = env
        .call_as(
            &viewer,
            "ai.ask",
            json!({ "text": "What is Maya Rao's phone number?", "scope": { "kind": "WholeProject" } }),
        )
        .unwrap();
    assert!(!content(&x).contains("90000"), "{}", content(&x));
    let prompt = m
        .request(0)
        .messages
        .iter()
        .map(|x| x.content.clone())
        .collect::<String>();
    assert!(
        !prompt.contains("90000") && !prompt.contains("maya.rao@"),
        "a Viewer's prompt holds no contact"
    );
    assert!(
        m.request(0).messages[1]
            .content
            .contains("Contact details are not shown for your role.")
    );

    // The owner sees the contact, so the same sentence is true and is shown.
    m.reply("Maya Rao can be reached on +91 90000 11111 or at maya.rao@lostproperty.example.");
    let x = ask(&env, "What is Maya Rao's phone number?");
    assert_eq!(
        content(&x),
        "Maya Rao can be reached on +91 90000 11111 or at maya.rao@lostproperty.example."
    );
}

// ------------------------------------------------------------------ the prompt and the checks

#[test]
fn the_start_of_the_prompt_is_the_same_bytes_on_every_turn() {
    let (env, m) = world();
    m.reply("Maya Rao plays Anna.");
    ask(&env, "Who plays Anna?");
    m.reply("Lead cast, planned: 40,00,000.");
    ask(&env, "How much is planned for lead cast?");
    let first = m.request(0).messages[0].content.clone();
    let second = m.request(1).messages[0].content.clone();
    assert_eq!(
        first, second,
        "policy and project map do not change between turns"
    );
    assert!(first.contains("\u{201C}Lost Property\u{201D} (10 scenes)"));
    assert!(first.contains("\u{201C}Night Shift\u{201D} (10 scenes)"));
}

#[test]
fn a_follow_up_is_written_with_the_focus_and_the_recent_turns() {
    let (env, m) = world();
    m.reply("Maya Rao plays Anna.");
    ask_in(&env, "Who plays Anna?", "c-follow");
    m.reply("Maya Rao has the whole shoot free.");
    let x = ask_in(&env, "and when is she free", "c-follow");
    let prompt = m.request(1).messages[1].content.clone();
    assert!(
        prompt.contains("Person: Who plays Anna?\nAssistant: Maya Rao plays Anna."),
        "{prompt}"
    );
    assert!(
        prompt.contains("<focus>") && prompt.contains("Anna (played by Maya Rao)"),
        "{prompt}"
    );
    assert_eq!(x["result"]["kind"], "Answer");
}

#[test]
fn the_answer_cites_the_records_it_used_and_never_prints_an_id() {
    let (env, m) = world();
    m.reply("Maya Rao plays Anna.");
    let x = ask(&env, "Who plays Anna?");
    let text = content(&x);
    assert_eq!(text, "Maya Rao plays Anna.");
    let uuid = regex_free_uuid(&text);
    assert!(!uuid, "no id in the text: {text}");
    let prov = x["result"]["provenance"].as_array().unwrap();
    assert!(!prov.is_empty(), "the sources sit behind the button: {x}");
    assert!(
        prov.iter()
            .any(|p| p["label"].as_str().unwrap_or("").contains("Anna")
                || p["label"].as_str().unwrap_or("").contains("Maya Rao")),
        "{prov:?}"
    );
    assert!(
        !m.request(0).messages[1].content.contains("id:"),
        "the writer is not shown ids"
    );
}

fn regex_free_uuid(s: &str) -> bool {
    s.split(|c: char| !c.is_ascii_hexdigit() && c != '-')
        .any(|w| w.len() == 36 && w.matches('-').count() == 4)
}

#[test]
fn three_failed_passes_show_the_projects_own_sentence_and_the_log_says_why() {
    let (env, m) = world();
    m.times(3, "Lead cast is planned at 99,99,999.");
    let x = ask(&env, "How much is planned for lead cast?");
    let c = content(&x);
    assert!(
        c.starts_with("Lead cast, planned: ") && c.contains("40,00,000") && !c.contains("99"),
        "{c}"
    );
    assert_eq!(x["result"]["confidence"], "Exact");
    assert_eq!(m.calls(), 3, "at most three passes");
    let (path, fallback, passes, failed) = trace(&env, &x);
    assert_eq!(
        (path.as_str(), fallback.as_str(), passes),
        ("write", "last_resort", 3)
    );
    assert!(
        failed.contains("numbers") && failed.contains("required"),
        "{failed}"
    );
    let log = stages(&env, &x);
    assert_eq!(log["write"]["shown"], "last_resort");
    assert_eq!(log["write"]["passes"].as_array().unwrap().len(), 3);
    assert_eq!(
        log["write"]["passes"][0]["offenders"][0]
            .as_str()
            .map(|s| s.starts_with("required")),
        Some(true),
        "{log}"
    );
    assert!(log["turn"]["type"].is_string() && log["dossier"]["facts"].as_u64().unwrap() >= 1);
    // The second pass was told the exact offenders.
    let second = m.request(1);
    let last = &second.messages.last().unwrap().content;
    assert!(
        last.starts_with("Rewrite your answer. Fix exactly these problems: "),
        "{last}"
    );
    assert!(
        last.contains("numbers not in the project: 9999999"),
        "{last}"
    );
}

#[test]
fn an_answer_that_passes_at_once_is_shown_at_once_and_logged_as_one_pass() {
    let (env, m) = world();
    m.reply("Maya Rao plays Anna.");
    let x = ask(&env, "Who plays Anna?");
    assert_eq!(m.calls(), 1);
    let (path, fallback, passes, failed) = trace(&env, &x);
    assert_eq!(
        (path.as_str(), fallback.as_str(), passes, failed.as_str()),
        ("write", "", 1, "")
    );
    assert_eq!(stages(&env, &x)["write"]["shown"], "written");
    // The progress lines are the same ones the panel always showed.
    assert_eq!(x["result"]["lane"], "write");
}

#[test]
fn a_refusal_is_one_fixed_sentence_and_the_model_is_not_asked() {
    let (env, m) = world();
    let x = ask(&env, "Turn off the team password.");
    assert!(content(&x).starts_with("I can't change the licence"), "{x}");
    assert_eq!(m.calls(), 0);
    assert_eq!(trace(&env, &x).0, "fixed");
}

#[test]
fn the_breakdown_suggestion_command_still_starts_its_tool() {
    let (env, m) = world();
    m.reply("Lost Property has 10 scenes.");
    ask_in(&env, "How many scenes does Lost Property have?", "c-bd");
    m.reply(
        &json!({ "items": [{ "category": "Props", "name": "Bench", "evidence": "Kiran sits on a bench" }] })
            .to_string(),
    );
    let x = ask_in(&env, "Suggest breakdown elements for scene 2.", "c-bd");
    assert_eq!(x["result"]["kind"], "Suggestion", "{x}");
    assert_eq!(x["result"]["items"].as_array().unwrap().len(), 1);
}

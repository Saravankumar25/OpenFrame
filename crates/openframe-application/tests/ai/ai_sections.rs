//! The section-scoped assistant (ADR-0041, slices R1 to R3) on a small real project: the rules of the
//! picks, the leaf registry's fetchers, the split budget, text search held to the picked leaves, the
//! truthful label and the trace. No real model: a scripted one records what the writer is shown.
//! Also what stays out of the prompt when the picks change, and the out-of-section pointer (R4).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use openframe_ai::{ChatRequest, InferenceProvider};
use openframe_application::MutationMeta;
use openframe_application::modules::ai::dossier::{self, Dossier, LeafRead};
use openframe_application::modules::ai::index::{self, Focus, Kind};
use openframe_application::modules::ai::retrieval::{self, RetrievalRoute, RetrieveRequest};
use openframe_application::modules::ai::sections::{self, LEAVES, SectionPick};
use openframe_application::modules::ai::service;
use openframe_application::modules::ai::turn::{self, Aids, AnswerType, TurnInput};
use openframe_domain::{Actor, AppResult, Capability, Role};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

// ------------------------------------------------------------------ fixtures

#[derive(Default)]
struct ScriptedModel {
    replies: Mutex<VecDeque<String>>,
    seen: Mutex<Vec<ChatRequest>>,
}

impl ScriptedModel {
    fn calls(&self) -> usize {
        self.seen.lock().unwrap().len()
    }
    fn seen_text(&self) -> String {
        let seen = self.seen.lock().unwrap();
        seen.iter()
            .flat_map(|r| r.messages.iter().map(|m| m.content.clone()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl InferenceProvider for ScriptedModel {
    fn chat(&self, req: &ChatRequest) -> AppResult<String> {
        self.seen.lock().unwrap().push(req.clone());
        Ok(self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| "The project holds that.".to_string()))
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

fn attach(env: &TestEnv) -> Arc<ScriptedModel> {
    let m = Arc::new(ScriptedModel::default());
    service(&env.core).attach_model(Some(m.clone() as Arc<dyn InferenceProvider>));
    m
}

fn exec(env: &TestEnv, sql: &str) {
    let s = env.core.project().unwrap();
    let sql = sql.to_string();
    s.store
        .mutate(
            &env.actor(),
            MutationMeta::new("test.seed", "Seed", Capability::Edit),
            move |tx| {
                tx.conn().execute_batch(&sql)?;
                Ok(())
            },
        )
        .unwrap();
}

fn id_of(v: &Value) -> String {
    v["id"]
        .as_str()
        .or_else(|| v.as_str())
        .unwrap_or_else(|| panic!("no id in {v}"))
        .to_string()
}

const SEED: &str = "
    INSERT INTO screenplay(id,title,format,current_draft_id,created_at,updated_at) VALUES ('sp','Black Rain','Feature','d2',1,1);
    INSERT INTO screenplay_draft(id,screenplay_id,name,status,created_at,updated_at) VALUES
        ('d1','sp','Draft 1','Draft',1,1),
        ('d2','sp','Draft 2','Locked',2,2);
    INSERT INTO screenplay_scene(id,draft_id,lineage_id,position,heading,synopsis,created_at,updated_at) VALUES
        ('old1','d1','L1',1,'INT. OLD DRAFT HOUSE - DAY','An early idea',1,1),
        ('s1','d2','L1',1,'INT. POLICE STATION - NIGHT','Meera hands the file',1,1),
        ('s2','d2','L2',2,'EXT. OLD RAILWAY STATION - DAY',NULL,1,1),
        ('s3','d2','L3',3,'INT. MORGUE - DAY','Time of death',1,1);
    INSERT INTO screenplay_element(id,scene_id,position,element_type,text,created_at,updated_at) VALUES
        ('e0','old1',1,'action','A sentence only the first draft holds about the lighthouse.',1,1),
        ('e1','s1',1,'action','Rain lashes the windows. MEERA slides a red folder across the desk.',1,1),
        ('e2','s1',2,'character','MEERA',1,1),
        ('e3','s1',3,'dialogue','You will want to see this before anyone else does.',1,1),
        ('e4','s2',1,'action','ARJUN walks the empty platform, photograph in hand.',1,1),
        ('e5','s2',2,'character','ARJUN',1,1),
        ('e6','s2',3,'dialogue','The train never came.',1,1),
        ('e7','s3',1,'action','A sheet is pulled back.',1,1);
    INSERT INTO story_character(id,name,description,position,created_at,updated_at) VALUES
        ('arjun','Arjun','A tired detective.',1,1,1),
        ('meera','Meera','A clerk who keeps files.',2,1,1);
    INSERT INTO cast_member(id,person_name,character_id,created_at,updated_at) VALUES ('cm1','Karthik Menon','arjun',1,1);
    INSERT INTO location(id,name,address,status,created_at,updated_at) VALUES
        ('loc_police','Police Station','District Rd, Gudur','Confirmed',1,1),
        ('loc_rail','Old Railway Station',NULL,'Idea',1,1);
    INSERT INTO catalog_item(id,category,name,character_id,location_id,created_at,updated_at) VALUES
        ('ci_arjun','Cast','Arjun','arjun',NULL,1,1),
        ('ci_meera','Cast','Meera','meera',NULL,1,1),
        ('ci_police','Location / Set','Police Station',NULL,'loc_police',1,1),
        ('ci_folder','Props','Red Folder',NULL,NULL,1,1);
    INSERT INTO production_source(id,draft_id,active,selected_at,created_at,updated_at) VALUES ('ps1','d2',1,1,1,1);
    INSERT INTO breakdown_element(id,source_id,scene_id,scene_lineage_id,category,catalog_item_id,display_name,confirmation_state,created_at,updated_at) VALUES
        ('b1','ps1','s1','L1','Cast','ci_meera','Meera','Confirmed',1,1),
        ('b2','ps1','s2','L2','Cast','ci_arjun','Arjun','Confirmed',1,1),
        ('b3','ps1','s1','L1','Location / Set','ci_police','Police Station','Confirmed',1,1),
        ('b4','ps1','s1','L1','Props','ci_folder','Red Folder','Manual',1,1);
";

struct Film {
    env: TestEnv,
    day1: String,
    day2: String,
    sheet: String,
}

fn strip_of(env: &TestEnv, number: i64) -> String {
    let v = env.ok("schedule.get", json!({}));
    let mut all: Vec<Value> = v["unscheduled"].as_array().unwrap().clone();
    for d in v["days"].as_array().unwrap() {
        for it in d["items"].as_array().unwrap() {
            if it["kind"] == "strip" {
                all.push(it["strip"].clone());
            }
        }
    }
    all.iter()
        .find(|s| s["number"] == number)
        .unwrap_or_else(|| panic!("scene {number} not on the board"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn film_in(env: TestEnv) -> Film {
    env.ok(
        "project.update_settings",
        json!({ "logline": "A detective follows a file." }),
    );
    exec(&env, SEED);
    let schedule = id_of(&env.ok("schedule.create", json!({})));
    let day1 = env
        .ok(
            "schedule.create_day",
            json!({ "scheduleId": schedule, "date": "2026-12-01" }),
        )
        .as_str()
        .unwrap()
        .to_string();
    let day2 = env
        .ok(
            "schedule.create_day",
            json!({ "scheduleId": schedule, "date": "2026-12-02" }),
        )
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": strip_of(&env, 1), "dayId": day1 }),
    );
    env.ok(
        "schedule.move_strip",
        json!({ "stripId": strip_of(&env, 2), "dayId": day2 }),
    );
    let sheet = id_of(&env.ok("callsheets.create", json!({ "dayId": day1 })));
    env.ok("sides.create", json!({ "dayId": day1 }));
    env.ok(
        "tasks.create",
        json!({ "title": "Confirm the station permit" }),
    );
    env.ok(
        "notes.create",
        json!({ "title": "Permits", "body": "The harbour office wants the marigold form.", "pinned": false }),
    );
    let owner = env.actor().user_id;
    exec(
        &env,
        &format!(
            "INSERT INTO private_note(id, owner_user_id, target_type, target_id, body, created_at, updated_at)
             VALUES ('pn1', '{owner}', 'screenplay_scene', 's1', 'My own worry about the saffron scarf.', 1, 1)"
        ),
    );
    let budget = id_of(&env.ok("budget.create", json!({ "currency": "INR" })));
    env.ok(
        "budget.add_line",
        json!({ "budgetId": budget, "category": "Equipment", "description": "Camera", "planned": 180_000_000_i64, "actual": null }),
    );
    let act = id_of(&env.ok(
        "story.create_act",
        json!({ "title": "Act One", "note": "The file" }),
    ));
    env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "act", "parentId": act }, "shortDescription": "Meera finds the folder." }),
    );
    env.ok(
        "vault.create",
        json!({ "store": "project", "itemType": "note", "title": "Platform song", "body": "A tune the porter hums." }),
    );
    env.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "title": "Kept across films", "body": "Always shoot the rain." }),
    );
    env.ok(
        "story.page.save",
        json!({ "text": "The ending stays open.\nThe sea takes the file.", "expectedRev": 0 }),
    );
    let board = id_of(&env.ok("moodboard.create", json!({ "name": "Monsoon" })));
    env.ok(
        "shot.create",
        json!({ "sceneId": "s1", "description": "Wide of the desk" }),
    );
    env.ok("storyboard.create", json!({ "sceneId": "s1" }));
    let _ = board;
    let path = env.write_file("contract.txt", b"terms");
    env.ok("files.add", json!({ "paths": [path] }));
    Film {
        env,
        day1,
        day2,
        sheet,
    }
}

fn film() -> Film {
    film_in(TestEnv::with_project("Black Rain", "Feature Film"))
}

fn pick(leaf: &str) -> SectionPick {
    SectionPick {
        leaf: leaf.to_string(),
        ..SectionPick::default()
    }
}

/// The dossier of a message for the given picks, by rules, with the text search results given.
fn dossier_with(
    env: &TestEnv,
    actor: &Actor,
    message: &str,
    picks: &[SectionPick],
    t: Option<AnswerType>,
    passages: &[Vec<dossier::Fact>],
) -> Dossier {
    let s = env.core.project().expect("project");
    let ix = index::project_index(&s).expect("index");
    let resolved = index::resolve(&ix, &Focus::default(), message);
    let mut read = turn::read(
        &TurnInput {
            message,
            previous_message: None,
            previous_type: None,
            resolved: &resolved,
        },
        &Aids::none(),
    );
    if let Some(t) = t {
        read.answer_type = t;
        read.group = t.group();
        read.length_class = t.length_class();
    }
    let selection = s
        .store
        .read(|c| sections::resolve_picks(c, picks))
        .expect("picks");
    let leaf_read = LeafRead {
        selection: &selection,
        passages,
    };
    s.store
        .read(|c| {
            Ok(dossier::build(&dossier::Input {
                conn: c,
                actor,
                index: &ix,
                core: Some(&env.core),
                message,
                read: &read,
                resolved: &resolved,
                selected_scene: None,
                focus: None,
                previous_message: None,
                sections: Some(&leaf_read),
            }))
        })
        .expect("dossier")
}

fn facts_for(env: &TestEnv, message: &str, picks: &[SectionPick], t: AnswerType) -> Dossier {
    dossier_with(env, &env.actor(), message, picks, Some(t), &[])
}

fn text_of(d: &Dossier) -> String {
    format!("{}\n{}", d.render(), d.missing.join("\n"))
}

fn ask(env: &TestEnv, text: &str, picks: Value) -> Value {
    env.ok(
        "ai.ask",
        json!({ "text": text, "scope": { "kind": "WholeProject" }, "sections": picks }),
    )
}

fn trace_sections(env: &TestEnv, request: &str) -> Value {
    let raw: String = env
        .core
        .with_app_db(|c| {
            Ok(c.query_row(
                "SELECT sections_json FROM sys_ai_trace WHERE request_id = ?1",
                [request],
                |r| r.get(0),
            )?)
        })
        .expect("trace row");
    serde_json::from_str(&raw).expect("json")
}

// ------------------------------------------------------------------ the operation

#[test]
fn the_tree_lists_every_leaf_under_its_branch_and_what_its_pickers_list() {
    let f = film();
    let v = f.env.ok("ai.sections", json!({}));
    let branches = v["branches"].as_array().unwrap();
    let labels: Vec<&str> = branches
        .iter()
        .map(|b| b["label"].as_str().unwrap())
        .collect();
    assert_eq!(
        labels,
        vec![
            "Idea Vault",
            "Story",
            "Screenplay",
            "Breakdown",
            "Production",
            "Call Sheets",
            "Files",
            "Notes & Tasks",
            "App Help & Settings",
        ]
    );
    let leaves: usize = branches
        .iter()
        .map(|b| b["leaves"].as_array().unwrap().len())
        .sum();
    assert_eq!(leaves, LEAVES.len());
    assert_eq!(v["maxPicks"], 3);
    let scenes = v["scenes"].as_array().unwrap();
    assert_eq!(scenes.len(), 3, "the current draft's scenes: {scenes:?}");
    assert_eq!(v["days"].as_array().unwrap().len(), 2);
    assert_eq!(v["callSheets"].as_array().unwrap().len(), 1);
    assert_eq!(v["drafts"].as_array().unwrap().len(), 2);
    let older = f.env.ok("ai.sections", json!({ "draftId": "d1" }));
    assert_eq!(older["scenes"].as_array().unwrap().len(), 1);
    assert_eq!(
        f.env.err("ai.sections", json!({ "draftId": "nope" })),
        "validation.invalid_input"
    );
    let story = branches.iter().find(|b| b["label"] == "Story").unwrap();
    let episodes = story["leaves"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"] == "story/episodes")
        .unwrap();
    assert_eq!(
        episodes["available"], false,
        "a feature film has no episodes"
    );
    let scenes_leaf = branches
        .iter()
        .find(|b| b["label"] == "Screenplay")
        .unwrap()["leaves"][0]
        .clone();
    assert_eq!(scenes_leaf["picker"], "Screenplay");
}

#[test]
fn episodes_are_available_in_a_series_and_refused_elsewhere() {
    let series = TestEnv::with_project("The Tide", "Series");
    let v = series.ok("ai.sections", json!({}));
    let story = v["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["label"] == "Story")
        .unwrap()
        .clone();
    let episodes = story["leaves"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"] == "story/episodes")
        .unwrap()
        .clone();
    assert_eq!(episodes["available"], true);
    series.ok("story.create_episode", json!({ "title": "Pilot" }));
    let d = facts_for(
        &series,
        "Which episodes are there?",
        &[pick("story/episodes")],
        AnswerType::List,
    );
    assert!(text_of(&d).contains("Pilot"), "{}", text_of(&d));

    let f = film();
    attach(&f.env);
    let e = f.env.call(
        "ai.ask",
        json!({ "text": "Episodes?", "scope": { "kind": "WholeProject" }, "sections": [{ "leaf": "story/episodes" }] }),
    );
    assert!(e.unwrap_err().is("validation"));
}

// ------------------------------------------------------------------ the rules of the picks

#[test]
fn a_request_with_no_section_gets_a_fixed_sentence_and_runs_no_model() {
    let f = film_in(TestEnv::picking_sections_with_project(
        "Black Rain",
        "Feature Film",
    ));
    let model = attach(&f.env);
    let x = f.env.ok(
        "ai.ask",
        json!({ "text": "How many scenes are there?", "scope": { "kind": "WholeProject" } }),
    );
    assert_eq!(x["result"]["content"], sections::PICK_A_SECTION);
    assert_eq!(model.calls(), 0, "no model call");
    assert_eq!(x["scopeLabel"], "Using: no section");
    let t = trace_sections(&f.env, x["requestId"].as_str().unwrap());
    assert_eq!(t["leaves"], json!([]));
    assert_eq!(t["retrieval"], "none");
}

#[test]
fn a_request_with_no_section_answers_even_without_the_model() {
    let f = film_in(TestEnv::picking_sections_with_project(
        "Black Rain",
        "Feature Film",
    ));
    let x = f.env.ok(
        "ai.ask",
        json!({ "text": "Anything?", "scope": { "kind": "WholeProject" }, "sections": [] }),
    );
    assert_eq!(x["result"]["content"], sections::PICK_A_SECTION);
}

#[test]
fn the_evaluation_switch_reads_every_area_when_nothing_is_picked() {
    let f = film();
    let model = attach(&f.env);
    let x = f.env.ok(
        "ai.ask",
        json!({ "text": "What is the planned budget total?", "scope": { "kind": "WholeProject" } }),
    );
    assert_ne!(x["result"]["content"], sections::PICK_A_SECTION);
    assert!(model.calls() > 0);
    assert_ne!(x["scopeLabel"], "Using: no section");
}

#[test]
fn bad_picks_are_invalid_input_and_nothing_runs() {
    let f = film();
    let model = attach(&f.env);
    let call = |sections: Value| {
        f.env
            .call(
                "ai.ask",
                json!({ "text": "How many scenes?", "scope": { "kind": "WholeProject" }, "sections": sections }),
            )
            .expect_err("refused")
    };
    let invalid = [
        json!([{ "leaf": "nowhere/nothing" }]),
        json!([{ "leaf": "production/budget" }, { "leaf": "production/budget" }]),
        json!([{ "leaf": "production/budget" }, { "leaf": "files/files" }, { "leaf": "notes/notes" }, { "leaf": "notes/tasks" }]),
        json!([{ "leaf": "screenplay/scenes", "sceneId": "not-a-scene" }]),
        json!([{ "leaf": "screenplay/scenes", "draftId": "not-a-draft" }]),
        json!([{ "leaf": "screenplay/scenes", "screenplayId": "not-a-screenplay" }]),
        json!([{ "leaf": "production/schedule", "shootingDayId": "not-a-day" }]),
        json!([{ "leaf": "callsheets/callsheets", "callSheetId": "not-a-sheet" }]),
        // A choice the leaf does not have.
        json!([{ "leaf": "production/budget", "sceneId": "s1" }]),
        json!([{ "leaf": "breakdown/breakdown", "shootingDayId": f.day1 }]),
        // A scene of another draft.
        json!([{ "leaf": "screenplay/scenes", "draftId": "d2", "sceneId": "old1" }]),
    ];
    for bad in invalid {
        let e = call(bad.clone());
        assert!(e.is("validation"), "{bad}: {e:?}");
    }
    assert_eq!(model.calls(), 0);
}

#[test]
fn the_label_is_built_from_the_real_selection_and_the_trace_holds_codes_only() {
    let f = film();
    let _model = attach(&f.env);
    let x = ask(
        &f.env,
        "How many scenes does Arjun speak in?",
        json!([{ "leaf": "screenplay/scenes", "draftId": "d2" }, { "leaf": "production/cast-crew" }]),
    );
    assert_eq!(
        x["scopeLabel"],
        "Using: Screenplay (Draft 2, all scenes) + Cast & Crew"
    );
    let t = trace_sections(&f.env, x["requestId"].as_str().unwrap());
    assert_eq!(
        t["leaves"],
        json!(["screenplay/scenes", "production/cast-crew"])
    );
    assert_eq!(t["tokens"].as_array().unwrap().len(), 2);
    assert_eq!(t["retrieval"], "full_text", "no search model in this test");
    assert_eq!(t["degraded"], true);
    assert_eq!(t["instances"], 0);
    let raw = t.to_string();
    assert!(!raw.contains("Arjun") && !raw.contains("Draft 2"), "{raw}");
    let scene = ask(
        &f.env,
        "What happens here?",
        json!([{ "leaf": "breakdown/breakdown", "sceneId": "s2" }, { "leaf": "production/schedule", "shootingDayId": f.day2 }]),
    );
    assert_eq!(
        scene["scopeLabel"],
        "Using: Breakdown (Scene 2) + Schedule (Day 2)"
    );
    let t = trace_sections(&f.env, scene["requestId"].as_str().unwrap());
    assert_eq!(t["instances"], 2);
}

// ------------------------------------------------------------------ only the picked leaves

#[test]
fn only_the_picked_leaf_is_read_whatever_the_question_says() {
    let f = film();
    let d = facts_for(
        &f.env,
        "Who plays Arjun in scene 2, and what does the Camera cost?",
        &[pick("production/budget")],
        AnswerType::Lookup,
    );
    let text = text_of(&d);
    assert!(text.contains("Camera"), "{text}");
    for outside in [
        "Karthik Menon",
        "platform",
        "POLICE STATION",
        "Red Folder",
        "permit",
    ] {
        assert!(!text.contains(outside), "{outside} in: {text}");
    }
    // The names that live elsewhere are kept for the pointer, and only as references.
    let homes: Vec<&str> = d.outside.iter().map(|o| o.home).collect();
    assert!(homes.contains(&"story/characters"), "{homes:?}");
    assert!(homes.contains(&"screenplay/scenes"), "{homes:?}");
    assert!(
        d.outside
            .iter()
            .any(|o| o.kind == Kind::Character && o.name == "Arjun")
    );
}

#[test]
fn the_good_pick_of_the_design_reads_both_rooms() {
    let f = film();
    let d = facts_for(
        &f.env,
        "How many scenes does Arjun play in, and who plays him?",
        &[pick("screenplay/scenes"), pick("production/cast-crew")],
        AnswerType::Lookup,
    );
    let text = text_of(&d);
    assert!(text.contains("Karthik Menon"), "{text}");
    assert!(
        text.contains("Appearances") || text.contains("scenes"),
        "{text}"
    );
    assert!(d.outside.is_empty(), "{:?}", d.outside);
}

#[test]
fn no_extra_domain_is_added_for_a_scene_or_a_character() {
    let f = film();
    let d = facts_for(
        &f.env,
        "Explain scene 1",
        &[pick("screenplay/scenes")],
        AnswerType::Explain,
    );
    let text = text_of(&d);
    assert!(text.contains("POLICE STATION"), "{text}");
    assert!(!text.contains("Red Folder"), "no breakdown: {text}");
    assert!(!text.contains("Day 1"), "no schedule: {text}");
}

#[test]
fn every_leaf_reads_its_own_area_with_a_plain_question() {
    let f = film();
    let expected = [
        ("vault/project", "Platform song"),
        ("vault/global", "Kept across films"),
        ("story/page", "sea takes the file"),
        ("story/beat-board", "Meera finds the folder"),
        ("story/outline", "Act One"),
        ("story/characters", "Arjun"),
        ("story/timeline", "Unassigned"),
        ("screenplay/scenes", "Black Rain"),
        ("breakdown/breakdown", "elements in the breakdown"),
        ("production/overview", "Production overview"),
        ("production/catalog", "Red Folder"),
        ("production/locations", "Police Station"),
        ("production/cast-crew", "Karthik Menon"),
        ("production/moodboards", "Monsoon"),
        ("production/storyboards", "Storyboard"),
        ("production/shots", "shots"),
        ("production/schedule", "shoot days"),
        ("production/day-out-of-days", "Day out of days"),
        ("production/daily-view", "Daily view"),
        ("production/sides", "Sides"),
        ("production/reports", "Prop Report"),
        ("production/budget", "planned total"),
        ("callsheets/callsheets", "Call sheet"),
        ("files/files", "contract"),
        ("notes/notes", "Permits"),
        ("notes/tasks", "Confirm the station permit"),
        ("help/settings", "Black Rain"),
    ];
    for (leaf, word) in expected {
        let message = if leaf == "screenplay/scenes" {
            "How many scenes are there?"
        } else {
            "What is here?"
        };
        let t = if leaf == "screenplay/scenes" {
            AnswerType::Count
        } else {
            AnswerType::List
        };
        let d = facts_for(&f.env, message, &[pick(leaf)], t);
        let text = text_of(&d);
        assert!(text.contains(word), "{leaf} did not read {word}: {text}");
    }
    let guide = facts_for(
        &f.env,
        "How do I export a screenplay?",
        &[pick("help/guide")],
        AnswerType::Help,
    );
    assert!(guide.render().contains("Guide"), "{}", guide.render());
}

#[test]
fn a_kind_from_another_leaf_never_passes_even_when_its_fetcher_returns_it() {
    let f = film();
    // The production fetcher reads tasks and locations; the Locations leaf admits only its own.
    let d = facts_for(
        &f.env,
        "Which tasks are open and which locations are confirmed?",
        &[pick("production/locations")],
        AnswerType::List,
    );
    let text = text_of(&d);
    assert!(text.contains("Police Station"), "{text}");
    assert!(!text.contains("Confirm the station permit"), "{text}");
    let d = facts_for(
        &f.env,
        "Which tasks are open and which locations are confirmed?",
        &[pick("notes/tasks")],
        AnswerType::List,
    );
    let text = text_of(&d);
    assert!(text.contains("Confirm the station permit"), "{text}");
    assert!(!text.contains("Police Station"), "{text}");
}

// ------------------------------------------------------------------ instance picks

#[test]
fn a_picked_scene_narrows_the_leaf_and_a_named_scene_cannot_widen_it() {
    let f = film();
    let one = SectionPick {
        scene_id: Some("s2".into()),
        ..pick("screenplay/scenes")
    };
    let d = facts_for(
        &f.env,
        "What happens in scene 1?",
        &[one],
        AnswerType::Explain,
    );
    let text = text_of(&d);
    assert!(text.contains("RAILWAY"), "{text}");
    assert!(!text.contains("POLICE STATION"), "{text}");
    let breakdown = SectionPick {
        scene_id: Some("s1".into()),
        ..pick("breakdown/breakdown")
    };
    let d = facts_for(&f.env, "What is needed?", &[breakdown], AnswerType::List);
    assert!(text_of(&d).contains("Red Folder"), "{}", text_of(&d));
}

#[test]
fn a_picked_draft_is_the_draft_that_is_read() {
    let f = film();
    let old = SectionPick {
        draft_id: Some("d1".into()),
        ..pick("screenplay/scenes")
    };
    let d = facts_for(
        &f.env,
        "What happens in scene 1?",
        &[old],
        AnswerType::Explain,
    );
    let text = text_of(&d);
    assert!(text.contains("OLD DRAFT HOUSE"), "{text}");
    assert!(!text.contains("POLICE STATION"), "{text}");
}

#[test]
fn a_picked_day_narrows_the_schedule_and_the_daily_view_and_the_day_out_of_days() {
    let f = film();
    let day = |leaf: &str| SectionPick {
        shooting_day_id: Some(f.day2.clone()),
        ..pick(leaf)
    };
    let d = facts_for(
        &f.env,
        "What is shot that day?",
        &[day("production/daily-view")],
        AnswerType::Lookup,
    );
    let text = text_of(&d);
    assert!(text.contains("Day 2"), "{text}");
    let d = facts_for(
        &f.env,
        "What is shot that day?",
        &[day("production/schedule")],
        AnswerType::Lookup,
    );
    let text = text_of(&d);
    assert!(
        text.contains("2026-12-02") || text.contains("Day 2"),
        "{text}"
    );
    assert!(!text.contains("POLICE"), "{text}");
    let d = facts_for(
        &f.env,
        "Who works?",
        &[day("production/day-out-of-days")],
        AnswerType::List,
    );
    assert!(text_of(&d).contains("code on this day"), "{}", text_of(&d));
}

#[test]
fn a_picked_call_sheet_or_day_narrows_the_call_sheets() {
    let f = film();
    let sheet = SectionPick {
        call_sheet_id: Some(f.sheet.clone()),
        ..pick("callsheets/callsheets")
    };
    let d = facts_for(&f.env, "What is the status?", &[sheet], AnswerType::Lookup);
    assert!(d.render().contains(&f.sheet), "{}", d.render());
    let other_day = SectionPick {
        shooting_day_id: Some(f.day2.clone()),
        ..pick("callsheets/callsheets")
    };
    let d = facts_for(
        &f.env,
        "What is the status?",
        &[other_day],
        AnswerType::Lookup,
    );
    assert!(!d.render().contains(&f.sheet), "{}", d.render());
}

// ------------------------------------------------------------------ privacy

#[test]
fn a_private_note_reaches_its_owner_only_in_every_leaf() {
    let f = film();
    let owner_text = text_of(&facts_for(
        &f.env,
        "What are my notes?",
        &[pick("notes/notes")],
        AnswerType::List,
    ));
    assert!(owner_text.contains("saffron scarf"), "{owner_text}");
    let other = Actor {
        user_id: "someone-else".into(),
        ..f.env.actor()
    };
    let other_text = text_of(&dossier_with(
        &f.env,
        &other,
        "What are my notes?",
        &[pick("notes/notes")],
        Some(AnswerType::List),
        &[],
    ));
    assert!(!other_text.contains("saffron scarf"), "{other_text}");
    // The text search keeps the owner rule too.
    let req = RetrieveRequest {
        query: "saffron scarf".into(),
        scope: None,
        limit: 8,
        route: Some(RetrievalRoute::Lexical),
        nav: Some(sections::leaf("screenplay/scenes").unwrap().search),
    };
    let mine = retrieval::retrieve(&f.env.core, &f.env.actor(), &req).unwrap();
    assert!(
        mine.items.iter().any(|i| i.text.contains("saffron")),
        "the owner finds it under the scene it is attached to"
    );
    let theirs = retrieval::retrieve(&f.env.core, &other, &req).unwrap();
    assert!(!theirs.items.iter().any(|i| i.text.contains("saffron")));
}

#[test]
fn a_viewer_gets_no_contact_details_in_a_contact_leaf() {
    let f = film();
    f.env.ok(
        "cast.create",
        json!({ "personName": "Maya Rao", "characterName": "Meera", "primary": true, "contact": "email maya.rao@example.test" }),
    );
    let owner = facts_for(
        &f.env,
        "What is the contact email of Maya Rao?",
        &[pick("production/cast-crew")],
        AnswerType::Lookup,
    );
    assert!(
        text_of(&owner).contains("maya.rao@example.test"),
        "{}",
        text_of(&owner)
    );
    let viewer = f.env.actor_with_role(Role::Viewer);
    let d = dossier_with(
        &f.env,
        &viewer,
        "What is the contact email of Maya Rao?",
        &[pick("production/cast-crew")],
        Some(AnswerType::Lookup),
        &[],
    );
    let text = text_of(&d);
    assert!(!text.contains("maya.rao@example.test"), "{text}");
    assert!(text.contains("not shown for your role"), "{text}");
}

// ------------------------------------------------------------------ text inside the picks

#[test]
fn text_search_is_held_to_the_picked_leaf_by_its_navigation_rules() {
    let f = film();
    let search = |leaf: &str, q: &str| {
        retrieval::retrieve(
            &f.env.core,
            &f.env.actor(),
            &RetrieveRequest {
                query: q.into(),
                scope: None,
                limit: 8,
                route: Some(RetrievalRoute::HybridGraph),
                nav: Some(sections::leaf(leaf).unwrap().search),
            },
        )
        .unwrap()
    };
    let notes = search("notes/notes", "marigold form");
    assert!(notes.items.iter().any(|i| i.text.contains("marigold")));
    assert!(notes.degraded, "no search model: full text stood in");
    for other in ["production/budget", "screenplay/scenes", "notes/tasks"] {
        let got = search(other, "marigold form");
        assert!(
            !got.items.iter().any(|i| i.text.contains("marigold")),
            "{other} read a note"
        );
    }
    let screenplay = search("screenplay/scenes", "photograph platform");
    assert!(
        screenplay
            .items
            .iter()
            .any(|i| i.text.contains("photograph"))
    );
    let scene_chunk = screenplay
        .items
        .iter()
        .find(|i| i.text.contains("photograph"))
        .unwrap();
    assert!(
        scene_chunk.text.contains("RAILWAY"),
        "a scene chunk keeps its contextual header: {}",
        scene_chunk.text
    );
}

#[test]
fn an_answer_on_full_text_says_the_context_is_being_prepared_and_reaches_the_writer() {
    let f = film();
    let model = attach(&f.env);
    let x = ask(
        &f.env,
        "What does the harbour office want?",
        json!([{ "leaf": "notes/notes" }]),
    );
    let seen = model.seen_text();
    assert!(
        seen.contains("marigold"),
        "the writer read the note: {seen}"
    );
    assert!(
        !seen.contains("Rain lashes"),
        "the writer read only the pick"
    );
    let details = x["result"]["details"].to_string();
    assert!(details.contains(retrieval::PREPARING_NOTICE), "{details}");
    let t = trace_sections(&f.env, x["requestId"].as_str().unwrap());
    assert_eq!(t["retrieval"], "full_text");
    assert_eq!(t["degraded"], true);
}

#[test]
fn a_leaf_narrowed_to_one_instance_runs_no_text_search() {
    let f = film();
    let _model = attach(&f.env);
    let x = ask(
        &f.env,
        "What happens at the morgue?",
        json!([{ "leaf": "screenplay/scenes", "sceneId": "s1" }]),
    );
    let t = trace_sections(&f.env, x["requestId"].as_str().unwrap());
    assert_eq!(t["retrieval"], "none");
}

// ------------------------------------------------------------------ the budget

fn passage(i: usize, chars: usize) -> dossier::Fact {
    dossier::Fact::new("Found", format!("p{i}"))
        .f("text", "w ".repeat(chars / 2))
        .prio(dossier::P_LOW)
}

#[test]
fn one_large_leaf_cannot_starve_the_others_and_required_facts_stay() {
    let f = film();
    let picks = [pick("production/budget"), pick("notes/notes")];
    // Far more text than the cap, all from the notes leaf.
    let notes: Vec<dossier::Fact> = (0..40).map(|i| passage(i, 800)).collect();
    let d = dossier_with(
        &f.env,
        &f.env.actor(),
        "What is the planned total and what is in my notes?",
        &picks,
        Some(AnswerType::Lookup),
        &[Vec::new(), notes],
    );
    assert!(d.render().contains("Budget total"), "{}", d.render());
    let budget = d
        .leaf_tokens
        .iter()
        .find(|l| l.leaf == "production/budget")
        .unwrap();
    assert!(budget.tokens > 0);
    let cap = dossier::cap_tokens(turn::LengthClass::OneSentence);
    assert!(d.tokens_estimate <= cap, "{} over {cap}", d.tokens_estimate);
    let found = d.facts.iter().filter(|x| x.kind == "Found").count();
    assert!(found > 0, "spare share flows to the text");
    assert!(found < 40, "the text was cut");
}

// ------------------------------------------------------------------ nothing from outside the picks

fn prompt_of(model: &ScriptedModel, call: usize) -> String {
    let seen = model.seen.lock().unwrap();
    seen[call]
        .messages
        .iter()
        .map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

fn ask_in(env: &TestEnv, conversation: &str, text: &str, picks: Value) -> Value {
    env.ok(
        "ai.ask",
        json!({ "text": text, "scope": { "kind": "WholeProject" }, "conversationId": conversation, "sections": picks }),
    )
}

#[test]
fn no_fact_from_an_unpicked_section_reaches_the_prompt_when_the_picks_change() {
    let f = film();
    let model = attach(&f.env);
    let conversation = "c-picks";
    model
        .replies
        .lock()
        .unwrap()
        .push_back("Karthik Menon plays Arjun.".to_string());
    // The person told the assistant two things earlier: a fact about a person and a preference.
    let cast = json!([{ "leaf": "production/cast-crew" }]);
    ask_in(&f.env, conversation, "Who plays Arjun?", cast.clone());
    ask_in(
        &f.env,
        conversation,
        "Remember that Karthik Menon has a bad back",
        cast.clone(),
    );
    ask_in(
        &f.env,
        conversation,
        "Always answer in short sentences",
        cast.clone(),
    );
    let first_prompt = prompt_of(&model, 0);
    assert!(
        first_prompt.contains("the person chose to read Cast & Crew"),
        "the map lists the picks: {first_prompt}"
    );
    assert!(!first_prompt.contains("Black Rain"), "{first_prompt}");

    let before = model.calls();
    ask_in(
        &f.env,
        conversation,
        "What does the Camera cost?",
        json!([{ "leaf": "production/budget" }]),
    );
    assert!(model.calls() > before, "the writer ran");
    let prompt = prompt_of(&model, before);
    assert!(prompt.contains("Camera"), "the pick is read: {prompt}");
    assert!(
        prompt.contains("the person chose to read Budget"),
        "{prompt}"
    );
    for outside in [
        "Karthik Menon",
        "bad back",
        "Arjun",
        "Black Rain",
        "3 scenes",
        "played by",
    ] {
        assert!(!prompt.contains(outside), "{outside} reached: {prompt}");
    }
    assert!(
        prompt.contains("short sentences"),
        "a preference holds no fact of the project and stays: {prompt}"
    );

    // Back on the section the fact belongs to, the earlier turn and the memory are there again.
    let before = model.calls();
    ask_in(&f.env, conversation, "And who is the lead?", cast);
    let prompt = prompt_of(&model, before);
    assert!(prompt.contains("Karthik Menon plays Arjun"), "{prompt}");
    assert!(prompt.contains("bad back"), "{prompt}");
}

#[test]
fn the_focus_keeps_its_ids_for_a_follow_up_but_gives_no_text_outside_the_picks() {
    let f = film();
    let model = attach(&f.env);
    let conversation = "c-focus";
    ask_in(
        &f.env,
        conversation,
        "Who plays Arjun in scene 2?",
        json!([{ "leaf": "production/cast-crew" }, { "leaf": "screenplay/scenes" }]),
    );
    let before = model.calls();
    ask_in(
        &f.env,
        conversation,
        "What is the planned total?",
        json!([{ "leaf": "production/budget" }]),
    );
    let prompt = prompt_of(&model, before);
    assert!(!prompt.contains("<focus>"), "{prompt}");
    assert!(!prompt.contains("Arjun"), "{prompt}");
    // The ids stayed: with the people picked again, "he" finds him.
    let before = model.calls();
    ask_in(
        &f.env,
        conversation,
        "Who plays him?",
        json!([{ "leaf": "production/cast-crew" }]),
    );
    let prompt = prompt_of(&model, before);
    assert!(prompt.contains("Karthik Menon"), "{prompt}");
}

#[test]
fn the_summary_of_older_turns_names_only_what_the_picks_can_answer_about() {
    let f = film();
    let model = attach(&f.env);
    let conversation = "c-summary";
    let both = json!([{ "leaf": "production/cast-crew" }, { "leaf": "production/budget" }]);
    // More turns than are kept word for word, each naming a person.
    for _ in 0..10 {
        ask_in(&f.env, conversation, "Who plays Arjun?", both.clone());
    }
    let before = model.calls();
    ask_in(
        &f.env,
        conversation,
        "What is the planned total?",
        json!([{ "leaf": "production/budget" }]),
    );
    let prompt = prompt_of(&model, before);
    assert!(!prompt.contains("Arjun"), "{prompt}");
    assert!(!prompt.contains("Karthik"), "{prompt}");
}

// ------------------------------------------------------------------ the pointer

const POINT_AT_CAST: &str =
    "That is in Production > Cast & Crew. Add it to your selection and ask again.";

#[test]
fn a_name_that_lives_elsewhere_gets_the_fixed_sentence_with_no_model() {
    let f = film();
    let model = attach(&f.env);
    let x = ask(
        &f.env,
        "Who plays Arjun?",
        json!([{ "leaf": "production/budget" }]),
    );
    assert_eq!(x["result"]["content"], POINT_AT_CAST);
    assert_eq!(model.calls(), 0, "no model call");
    assert_eq!(x["result"]["pointer"]["leaf"], "production/cast-crew");
    assert_eq!(x["result"]["pointer"]["label"], "Production > Cast & Crew");
    assert_eq!(x["result"]["lane"], "fixed");
    let raw = x.to_string();
    assert!(!raw.contains("Karthik"), "no fact of the other leaf: {raw}");
    let t = trace_sections(&f.env, x["requestId"].as_str().unwrap());
    assert_eq!(t["pointer"], "production/cast-crew");
    assert_eq!(t["pointer_trigger"], "name");
    assert!(!t.to_string().contains("Arjun"), "codes only: {t}");
}

#[test]
fn a_name_goes_to_its_home_when_no_word_says_otherwise() {
    let f = film();
    let model = attach(&f.env);
    let x = ask(
        &f.env,
        "Tell me about Meera",
        json!([{ "leaf": "notes/tasks" }]),
    );
    assert_eq!(x["result"]["pointer"]["leaf"], "story/characters");
    assert_eq!(model.calls(), 0);
}

#[test]
fn a_topic_word_points_only_when_the_picks_returned_nothing_that_answers() {
    let f = film();
    let model = attach(&f.env);
    let x = ask(
        &f.env,
        "Who is the director?",
        json!([{ "leaf": "production/budget" }]),
    );
    assert_eq!(x["result"]["pointer"]["leaf"], "production/cast-crew");
    assert_eq!(model.calls(), 0);
    let t = trace_sections(&f.env, x["requestId"].as_str().unwrap());
    assert_eq!(t["pointer_trigger"], "topic");
    // A budget line holds the word: the topic word does not override what the pick found.
    let v = f.env.ok("ai.sections", json!({}));
    assert!(v["branches"].is_array());
    let budget = f
        .env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT id FROM budget_snapshot WHERE deleted_at IS NULL LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )?)
        })
        .unwrap();
    f.env.ok(
        "budget.add_line",
        json!({ "budgetId": budget, "category": "Crew", "description": "Director fee", "planned": 5_000_000_i64, "actual": null }),
    );
    let x = ask(
        &f.env,
        "Who is the director?",
        json!([{ "leaf": "production/budget" }]),
    );
    assert!(x["result"].get("pointer").is_none(), "{x}");
    assert!(model.calls() >= 1, "the writer answers from the pick");
}

#[test]
fn a_word_that_points_at_a_pick_keeps_every_other_word_from_pointing() {
    let f = film();
    let _model = attach(&f.env);
    for (text, leaf) in [
        ("What was planned for Day 1?", "production/schedule"),
        (
            "What is the planned cost of the camera?",
            "production/budget",
        ),
        ("Which task is about the call sheet?", "notes/tasks"),
        ("Is there a timeline for the story?", "story/timeline"),
        (
            "How many shots are planned for scene 1?",
            "production/shots",
        ),
    ] {
        let x = ask(&f.env, text, json!([{ "leaf": leaf }]));
        assert!(x["result"].get("pointer").is_none(), "{text}: {x}");
    }
}

#[test]
fn a_word_with_no_topic_is_looked_for_in_the_index_of_the_other_sections() {
    let f = film();
    let model = attach(&f.env);
    // The word is in one project note and nowhere near a topic word.
    f.env.ok(
        "notes.create",
        json!({ "title": "Quarry", "body": "Marigold quarry access stays closed.", "pinned": false }),
    );
    let x = ask(
        &f.env,
        "Marigold quarry access?",
        json!([{ "leaf": "production/shots" }]),
    );
    assert_eq!(x["result"]["pointer"]["leaf"], "notes/notes");
    assert_eq!(model.calls(), 0);
    let t = trace_sections(&f.env, x["requestId"].as_str().unwrap());
    assert_eq!(t["pointer_trigger"], "exists");
    assert!(!x.to_string().contains("stays closed"), "{x}");
}

#[test]
fn part_of_a_question_that_fits_is_written_and_the_rest_gets_the_sentence() {
    let f = film();
    let model = attach(&f.env);
    model.replies.lock().unwrap().push_back(
        "The Camera is planned at INR 18,00,000, and no actual amount is recorded.".to_string(),
    );
    let x = ask(
        &f.env,
        "Who plays Arjun, and what does the Camera cost?",
        json!([{ "leaf": "production/budget" }]),
    );
    let content = x["result"]["content"].as_str().unwrap();
    assert!(content.starts_with("The Camera"), "{content}");
    assert!(content.ends_with(POINT_AT_CAST), "{content}");
    assert_eq!(x["result"]["pointer"]["leaf"], "production/cast-crew");
    assert!(model.calls() >= 1, "the writer answered the part that fits");
    let prompt = prompt_of(&model, 0);
    assert!(
        prompt.contains("Part of the request is about a section"),
        "{prompt}"
    );
    assert!(prompt.contains("Camera"), "{prompt}");
    assert!(!prompt.contains("Karthik"), "{prompt}");
    assert_eq!(x["result"]["lane"], "write");
}

#[test]
fn a_clause_on_a_topic_of_the_pick_is_written_and_a_clause_on_another_gets_the_sentence() {
    let f = film();
    let model = attach(&f.env);
    model
        .replies
        .lock()
        .unwrap()
        .push_back("The planned total is what the project records.".to_string());
    let x = ask(
        &f.env,
        "What does the budget total, and who is the director?",
        json!([{ "leaf": "production/budget" }]),
    );
    let content = x["result"]["content"].as_str().unwrap();
    assert!(content.ends_with(POINT_AT_CAST), "{content}");
    assert!(model.calls() >= 1);
    let t = trace_sections(&f.env, x["requestId"].as_str().unwrap());
    assert_eq!(t["pointer_trigger"], "topic");
}

#[test]
fn the_pointer_never_tells_a_viewer_where_contact_details_are() {
    let f = film();
    let _model = attach(&f.env);
    f.env.ok(
        "cast.create",
        json!({ "personName": "Maya Rao", "characterName": "Meera", "primary": true, "contact": "email maya.rao@example.test" }),
    );
    let sections = json!({
        "text": "What is the email of Maya Rao?",
        "scope": { "kind": "WholeProject" },
        "sections": [{ "leaf": "production/budget" }],
    });
    let owner = f.env.ok("ai.ask", sections.clone());
    assert_eq!(owner["result"]["pointer"]["leaf"], "production/cast-crew");
    let viewer = f.env.actor_with_role(Role::Viewer);
    let x = f
        .env
        .core
        .dispatch(&viewer, "ai.ask", sections)
        .expect("a viewer may ask");
    let content = x["result"]["content"].as_str().unwrap();
    assert!(content.starts_with("I could not find"), "{content}");
    assert!(content.ends_with("I looked in Budget."), "{content}");
    assert!(x["result"].get("pointer").is_none(), "{x}");
    assert!(!content.contains("Cast"), "{content}");
}

#[test]
fn another_persons_private_note_is_never_pointed_at_and_its_owner_may_be() {
    let question = json!({
        "text": "saffron scarf worry?",
        "scope": { "kind": "WholeProject" },
        "sections": [{ "leaf": "production/shots" }],
    });
    let f = film();
    let _model = attach(&f.env);
    let other = Actor {
        user_id: "someone-else".into(),
        ..f.env.actor()
    };
    let with_note = f
        .env
        .core
        .dispatch(&other, "ai.ask", question.clone())
        .unwrap();
    assert!(with_note["result"].get("pointer").is_none(), "{with_note}");
    let mine = f.env.ok("ai.ask", question.clone());
    assert_eq!(mine["result"]["pointer"]["leaf"], "screenplay/scenes");

    // What the other person reads does not depend on whether the note exists.
    exec(
        &f.env,
        "UPDATE private_note SET deleted_at = 1 WHERE id = 'pn1'",
    );
    f.env.ok("search.rebuild", json!({}));
    let without_note = f.env.core.dispatch(&other, "ai.ask", question).unwrap();
    assert_eq!(
        with_note["result"]["content"], without_note["result"]["content"],
        "the reply gives the note away"
    );
}

#[test]
fn nothing_found_anywhere_gives_the_sentence_with_the_picks_and_no_model() {
    let f = film();
    let model = attach(&f.env);
    let x = ask(
        &f.env,
        "Who is Marcus?",
        json!([{ "leaf": "story/characters" }, { "leaf": "production/cast-crew" }]),
    );
    let content = x["result"]["content"].as_str().unwrap();
    assert_eq!(
        content,
        "I could not find a character called Marcus. I looked in Characters and Cast & Crew."
    );
    assert_eq!(model.calls(), 0);
    assert!(x["result"].get("pointer").is_none(), "{x}");
}

#[test]
fn an_empty_section_is_not_pointed_at() {
    let env = TestEnv::with_project("Empty", "Feature Film");
    let model = attach(&env);
    let x = ask(
        &env,
        "What is the total planned budget?",
        json!([{ "leaf": "notes/tasks" }]),
    );
    let content = x["result"]["content"].as_str().unwrap();
    assert!(content.starts_with("I could not find"), "{content}");
    assert!(content.ends_with("I looked in Tasks."), "{content}");
    assert!(x["result"].get("pointer").is_none(), "{x}");
    assert_eq!(model.calls(), 0);
}

#[test]
fn a_pointer_reply_keeps_its_section_when_the_conversation_is_read_again() {
    let f = film();
    let _model = attach(&f.env);
    let x = ask_in(
        &f.env,
        "c-again",
        "Who plays Arjun?",
        json!([{ "leaf": "production/budget" }]),
    );
    let again = f
        .env
        .ok("ai.history", json!({ "conversationId": "c-again" }));
    let raw = again.to_string();
    assert!(raw.contains("production/cast-crew"), "{raw}");
    assert_eq!(x["result"]["pointer"]["label"], "Production > Cast & Crew");
}

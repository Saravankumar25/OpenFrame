//! `ai.sections`: the section tree Straft AI panel shows (slice R1, spec
//! `ai-section-scoped-assistant` section 12.2). It lists only the leaves the person may read,
//! Episodes only on an episodic project, and the screenplays, drafts and scenes to choose from.

use serde_json::{Value, json};
use straft_domain::Role;
use straft_test_support::TestEnv;

const SHORT: &[u8] = include_bytes!("../../../../templates/samples/Example-Short.fountain");

fn leaves(tree: &Value) -> Vec<String> {
    tree["branches"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|b| b["leaves"].as_array().unwrap().iter())
        .map(|l| l["id"].as_str().unwrap().to_string())
        .collect()
}

fn with_script(project_type: &str) -> TestEnv {
    let env = TestEnv::with_project("Lost Property", project_type);
    let path = env.write_file("short.fountain", SHORT);
    let p = env.ok("screenplay.import_preview", json!({ "path": path }));
    env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay" }),
    );
    env
}

#[test]
fn the_tree_lists_the_sections_with_the_screenplays_and_scenes_to_choose_from() {
    let env = with_script("Short Film");
    let tree = env.ok("ai.sections", json!({}));
    let ids = leaves(&tree);
    assert!(ids.contains(&"screenplay/scenes".to_string()), "{ids:?}");
    assert!(ids.contains(&"production/cast-crew".to_string()));
    assert!(ids.contains(&"help/settings".to_string()));
    // Episodes belong to a series.
    assert!(!ids.contains(&"story/episodes".to_string()), "{ids:?}");
    let branches: Vec<&str> = tree["branches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["label"].as_str().unwrap())
        .collect();
    assert_eq!(branches.first(), Some(&"Idea Vault"));
    assert!(branches.contains(&"Notes & Tasks"));
    let scenes_leaf = tree["branches"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|b| b["leaves"].as_array().unwrap())
        .find(|l| l["id"] == "screenplay/scenes")
        .unwrap();
    assert_eq!(scenes_leaf["picker"], "screenplay");
    assert_eq!(scenes_leaf["pathLabel"], "Screenplay > Scenes");
    let sps = tree["screenplays"].as_array().unwrap();
    assert_eq!(sps.len(), 1);
    assert_eq!(sps[0]["title"], "LOST PROPERTY");
    assert_eq!(sps[0]["drafts"].as_array().unwrap().len(), 1);
    assert_eq!(sps[0]["drafts"][0]["current"], true);
    assert_eq!(sps[0]["scenes"].as_array().unwrap().len(), 10);
    assert_eq!(sps[0]["scenes"][0]["number"], 1);
    // The Story page is in the tree with the reason it cannot answer.
    let page = tree["branches"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|b| b["leaves"].as_array().unwrap())
        .find(|l| l["id"] == "story/story-page")
        .unwrap();
    assert!(page["note"].as_str().unwrap().contains("not offered"));
}

#[test]
fn episodes_appear_on_a_series() {
    let env = TestEnv::with_project("Night Shift", "Series");
    let ids = leaves(&env.ok("ai.sections", json!({})));
    assert!(ids.contains(&"story/episodes".to_string()), "{ids:?}");
}

#[test]
fn a_viewer_may_look_but_export_only_may_not_use_the_assistant() {
    let env = with_script("Short Film");
    let viewer = env.actor_with_role(Role::Viewer);
    let tree = env.call_as(&viewer, "ai.sections", json!({})).unwrap();
    assert!(leaves(&tree).len() >= 28);
    let export_only = env.actor_with_role(Role::ExportOnly);
    let denied = env
        .call_as(&export_only, "ai.sections", json!({}))
        .unwrap_err();
    assert_eq!(denied.code_str(), "permission.denied");
}

#[test]
fn a_draft_asked_for_lists_its_scenes() {
    let env = with_script("Short Film");
    let tree = env.ok("ai.sections", json!({}));
    let draft = tree["screenplays"][0]["drafts"][0]["id"].as_str().unwrap();
    let again = env.ok("ai.sections", json!({ "draftId": draft }));
    assert_eq!(again["screenplays"][0]["scenesDraftId"], draft);
    assert_eq!(
        again["screenplays"][0]["scenes"].as_array().unwrap().len(),
        10
    );
}

// ---------------------------------------------------------------- the payload of ai.ask (R2)

fn ask(env: &TestEnv, sections: Value) -> Result<Value, String> {
    env.call(
        "ai.ask",
        json!({ "text": "How many scenes are there?", "sections": sections }),
    )
    .map_err(|e| format!("{} {}", e.code_str(), e.message))
}

#[test]
fn a_question_with_no_picks_or_too_many_is_refused_in_plain_words() {
    let env = with_script("Short Film");
    let none = ask(&env, json!([])).unwrap_err();
    assert!(none.contains("Pick a section to ask about."), "{none}");
    let four = ask(
        &env,
        json!([
            { "leaf": "screenplay/scenes" },
            { "leaf": "production/cast-crew" },
            { "leaf": "production/budget" },
            { "leaf": "files/files" }
        ]),
    )
    .unwrap_err();
    assert!(
        four.contains("Three sections at most. Remove one to add another."),
        "{four}"
    );
}

#[test]
fn an_unknown_or_repeated_section_is_refused() {
    let env = with_script("Short Film");
    let unknown = ask(&env, json!([{ "leaf": "production/nothing" }])).unwrap_err();
    assert!(
        unknown.contains("is not a section Straft AI can read"),
        "{unknown}"
    );
    let twice = ask(
        &env,
        json!([{ "leaf": "files/files" }, { "leaf": "files/files" }]),
    )
    .unwrap_err();
    assert!(twice.contains("was picked twice"), "{twice}");
    // The old payload is gone: a request with a scope and no sections does not run.
    let old = env
        .call(
            "ai.ask",
            json!({ "text": "x", "scope": { "kind": "WholeProject" } }),
        )
        .unwrap_err();
    assert_eq!(old.code_str(), "validation.invalid_input");
}

#[test]
fn the_using_line_is_built_from_the_picks() {
    use straft_application::modules::ai::sections::{SectionPick, using_label};
    let env = with_script("Short Film");
    let tree = env.ok("ai.sections", json!({}));
    let scene = tree["screenplays"][0]["scenes"][1]["id"].as_str().unwrap();
    let pick = |leaf: &str| SectionPick {
        leaf: leaf.into(),
        screenplay_id: None,
        draft_id: None,
        scene_id: None,
        shooting_day_id: None,
        call_sheet_id: None,
    };
    let label = |picks: Vec<SectionPick>| {
        let s = env.core.project().unwrap();
        s.store.read(|c| using_label(c, &picks)).unwrap()
    };
    let draft = tree["screenplays"][0]["drafts"][0]["name"]
        .as_str()
        .unwrap();
    assert_eq!(
        label(vec![
            pick("screenplay/scenes"),
            pick("production/cast-crew")
        ]),
        format!("Using: Screenplay ({draft}, all scenes) + Cast & Crew")
    );
    let mut one = pick("screenplay/scenes");
    one.scene_id = Some(scene.into());
    assert_eq!(
        label(vec![one]),
        format!("Using: Screenplay ({draft}, Scene 2)")
    );
    assert_eq!(
        label(vec![
            pick("idea-vault/this-project"),
            pick("call-sheets/call-sheets")
        ]),
        "Using: Idea Vault > This Project + Call Sheets"
    );
}

// ---------------------------------------------------------------- the dossier by leaves (R3)

mod by_leaves {
    use serde_json::json;
    use straft_application::modules::ai::dossier::{self, Dossier};
    use straft_application::modules::ai::index::{self, Focus};
    use straft_application::modules::ai::sections::SectionPick;
    use straft_application::modules::ai::turn::{self, Aids, AnswerType, TurnInput};
    use straft_domain::{Actor, Role};
    use straft_test_support::TestEnv;

    use super::super::ai_dossier::project;

    fn pick(leaf: &str) -> SectionPick {
        SectionPick {
            leaf: leaf.into(),
            screenplay_id: None,
            draft_id: None,
            scene_id: None,
            shooting_day_id: None,
            call_sheet_id: None,
        }
    }

    fn build(
        env: &TestEnv,
        actor: &Actor,
        message: &str,
        t: AnswerType,
        picks: &[SectionPick],
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
        read.answer_type = t;
        read.group = t.group();
        read.length_class = t.length_class();
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
                    sections: picks,
                }))
            })
            .expect("dossier")
    }

    fn kinds(d: &Dossier) -> Vec<&'static str> {
        d.facts.iter().map(|f| f.kind).collect()
    }

    #[test]
    fn the_picks_bound_the_facts() {
        let env = project();
        let me = env.actor();
        let ask = "how many locations do we have and what is the lead cast budget";
        let loc = build(
            &env,
            &me,
            ask,
            AnswerType::Lookup,
            &[pick("production/locations")],
        );
        assert!(
            kinds(&loc).contains(&"Location") || loc.render().contains("locations"),
            "{}",
            loc.render()
        );
        assert!(
            !kinds(&loc).iter().any(|k| k.starts_with("Budget")),
            "a budget fact reached a dossier that picked Locations: {}",
            loc.render()
        );
        let bud = build(
            &env,
            &me,
            ask,
            AnswerType::Lookup,
            &[pick("production/budget")],
        );
        assert!(
            kinds(&bud).iter().any(|k| k.starts_with("Budget")),
            "{}",
            bud.render()
        );
        assert!(!kinds(&bud).contains(&"Location"), "{}", bud.render());
        // Explaining a scene brings nothing from the leaves that were not picked.
        let scene = build(
            &env,
            &me,
            "explain scene 2",
            AnswerType::Explain,
            &[pick("screenplay/scenes")],
        );
        for k in kinds(&scene) {
            let home = straft_application::modules::ai::sections::home_of_kind(k).map(|l| l.id);
            assert!(
                home == Some("screenplay/scenes")
                    || straft_application::modules::ai::sections::SHARED_KINDS.contains(&k),
                "{k} came from a leaf that was not picked: {}",
                scene.render()
            );
        }
    }

    #[test]
    fn a_name_homed_in_a_leaf_that_was_not_picked_is_kept_aside_and_not_fetched() {
        let env = project();
        let d = build(
            &env,
            &env.actor(),
            "who plays Anna",
            AnswerType::Lookup,
            &[pick("production/budget")],
        );
        assert!(
            !kinds(&d).contains(&"Character") && !kinds(&d).contains(&"Cast"),
            "{}",
            d.render()
        );
        assert!(
            d.elsewhere
                .iter()
                .any(|e| e.leaf == "story/characters" && e.name == "Anna"),
            "{:?}",
            d.elsewhere
        );
        assert!(d.resolved.characters.is_empty());
        // With the leaf picked, the same name is fetched.
        let d = build(
            &env,
            &env.actor(),
            "who plays Anna",
            AnswerType::Lookup,
            &[pick("story/characters"), pick("production/cast-crew")],
        );
        assert!(kinds(&d).contains(&"Cast"), "{}", d.render());
        assert!(d.elsewhere.is_empty());
    }

    #[test]
    fn a_scene_pick_stands_for_the_scene_the_message_did_not_name() {
        let env = project();
        let tree = env.ok("ai.sections", json!({}));
        let scene = tree["screenplays"][0]["scenes"][1]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let mut p = pick("breakdown/breakdown");
        p.scene_id = Some(scene);
        let d = build(
            &env,
            &env.actor(),
            "what does it need",
            AnswerType::Lookup,
            &[p],
        );
        // The short film holds no breakdown yet: the dossier says so for the picked scene.
        assert!(
            d.missing.iter().any(|m| m.contains("scene 2")),
            "{:?} / {}",
            d.missing,
            d.render()
        );
    }

    #[test]
    fn one_huge_leaf_cannot_starve_the_others_and_the_cap_holds() {
        let env = project();
        for i in 0..60 {
            env.ok(
                "notes.create",
                json!({ "title": format!("Long note {i}"), "body": format!("{} {i}", "the porter hums a tune ".repeat(20)), "pinned": false }),
            );
        }
        let d = build(
            &env,
            &env.actor(),
            "list the notes and who plays Anna",
            AnswerType::List,
            &[
                pick("notes-tasks/notes"),
                pick("story/characters"),
                pick("production/cast-crew"),
            ],
        );
        let tokens = |leaf: &str| {
            d.leaves
                .iter()
                .find(|l| l.leaf == leaf)
                .map_or(0, |l| l.tokens)
        };
        assert!(
            tokens("production/cast-crew") > 0,
            "the small leaf was starved: {:?}",
            d.leaves
        );
        assert!(tokens("story/characters") > 0, "{:?}", d.leaves);
        let required: usize = d
            .facts
            .iter()
            .filter(|f| !f.requires.is_empty())
            .map(|f| f.tokens())
            .sum();
        assert!(
            d.tokens_estimate <= 1_500 + required,
            "{} tokens, cap 1500, required {required}: {:?}",
            d.tokens_estimate,
            d.leaves
        );
        assert!(
            d.leaf_tokens().contains("notes-tasks/notes="),
            "{}",
            d.leaf_tokens()
        );
    }

    #[test]
    fn a_required_value_stays_whatever_the_share() {
        let env = project();
        let d = build(
            &env,
            &env.actor(),
            "lead cast planned vs actual and the difference",
            AnswerType::Compare,
            &[
                pick("production/budget"),
                pick("screenplay/scenes"),
                pick("story/characters"),
            ],
        );
        assert!(
            d.required.iter().any(|r| r.contains("40,00,000")
                || r.contains("4000000")
                || r.contains("4,000,000")),
            "{:?} / {}",
            d.required,
            d.render()
        );
    }

    #[test]
    fn retrieval_stays_inside_the_pick_and_never_returns_another_persons_private_note() {
        use straft_application::modules::ai::intelligence::retriever::gather_in;
        let env = project();
        env.ok("notes.create", json!({ "title": "Zebra plan", "body": "The zebra crossing needs a permit.", "pinned": false }));
        env.ok(
            "private_note.create",
            json!({ "body": "my own zebra thought" }),
        );
        let other = env.other_user(Role::Editor);
        env.call_as(
            &other,
            "private_note.create",
            json!({ "body": "SECRETZEBRA the other persons plan" }),
        )
        .unwrap();
        let me = env.actor();
        let s = env.core.project().unwrap();
        let find = |word: &str, types: &[&str]| {
            s.store
                .read(|c| Ok(gather_in(c, &env.core, &me, None, None, word, types, 20)))
                .unwrap()
        };
        // Notes picked: the project note and my own private note, never the other person's.
        let notes = find("zebra", &["project_note", "private_note"]);
        assert!(!notes.hits.is_empty());
        for h in &notes.hits {
            assert!(
                matches!(h.key.kind, "project_note" | "private_note"),
                "{h:?}"
            );
        }
        let mut private_bodies = Vec::new();
        for h in &notes.hits {
            if h.key.kind == "private_note" {
                let body: String = s
                    .store
                    .read(|c| {
                        Ok(c.query_row(
                            "SELECT body FROM private_note WHERE id = ?1",
                            [&h.key.id],
                            |r| r.get(0),
                        )?)
                    })
                    .unwrap();
                private_bodies.push(body);
            }
        }
        assert_eq!(private_bodies, vec!["my own zebra thought".to_string()]);
        // The same word under a leaf that does not hold it finds nothing, and a word of the script
        // found under Notes finds nothing either.
        assert!(find("zebra", &["screenplay_scene"]).hits.is_empty());
        assert!(
            find("clanks", &["project_note", "private_note"])
                .hits
                .is_empty()
        );
        // The script word is found under Scenes: the filter is the pick, not a blind spot.
        let scenes = find("clanks", &["screenplay_scene"]);
        assert!(scenes.hits.iter().all(|h| h.key.kind == "screenplay_scene"));
        assert!(!scenes.hits.is_empty());
        // With no project index running, only the words were searched, and it says so.
        assert!(!notes.hybrid && notes.degraded);
    }

    #[test]
    fn passages_in_a_dossier_are_homed_in_the_picked_leaf_and_the_dossier_says_how_they_were_found()
    {
        let env = project();
        for i in 0..9 {
            env.ok(
                "notes.create",
                json!({ "title": format!("Crossing {i}"), "body": format!("The zebra crossing number {i} needs a permit."), "pinned": false }),
            );
        }
        // Words in the body only, so no title is spelled and no value is required: the facts alone do
        // not settle the question, and passages fill what is left.
        let d = build(
            &env,
            &env.actor(),
            "zebra crossing permit",
            AnswerType::Lookup,
            &[pick("notes-tasks/notes")],
        );
        let how = d.retrieval.expect("passages were looked for");
        assert!(!how.hybrid && how.degraded, "{how:?}");
        for f in d.facts.iter().filter(|f| f.kind == "Found") {
            assert_eq!(f.home, Some("notes-tasks/notes"), "{}", f.render());
            assert!(
                matches!(f.field("type"), Some("project_note" | "private_note")),
                "{}",
                f.render()
            );
        }
        assert!(!d.render().contains("SECRETZEBRA"));
    }

    #[test]
    fn nothing_found_names_the_picked_sections() {
        let env = project();
        let d = build(
            &env,
            &env.actor(),
            "quokka",
            AnswerType::Lookup,
            &[pick("screenplay/scenes")],
        );
        assert!(
            d.missing.iter().any(|m| m == "I could not find anything that matches \u{201C}quokka\u{201D}. I looked in Screenplay > Scenes."),
            "{:?}",
            d.missing
        );
    }
}

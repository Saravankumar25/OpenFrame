//! The project index and the reference resolver on the real evaluation fixtures (spec
//! `ai-conversational-harness`, slice S3). Names and aliases come from
//! `gold/fixtures/low-tide-two-screenplays/FACTS.md`.

use std::sync::Arc;
use std::time::Instant;

use openframe_ai_eval::fixture;
use openframe_application::core::{AppConfig, AppCore};
use openframe_application::events::RecordingSink;
use openframe_application::modules::ai::index::{
    EntityIndex, Focus, How, Kind, Resolved, project_index, resolve,
};
use serde_json::{Value, json};

fn core_with_project() -> (tempfile::TempDir, Arc<AppCore>) {
    let dir = tempfile::tempdir().unwrap();
    let core = AppCore::new(
        AppConfig::isolated(dir.path()),
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    q(
        &core,
        "project.create",
        json!({ "title": "fixture", "projectType": "Feature Film" }),
    );
    (dir, core)
}

#[track_caller]
fn q(core: &AppCore, op: &str, args: Value) -> Value {
    match core.dispatch(&core.local_actor(), op, args) {
        Ok(v) => v,
        Err(e) => panic!("{op}: {} {}", e.code_str(), e.message),
    }
}

fn index_of(core: &AppCore) -> Arc<EntityIndex> {
    project_index(&core.project().unwrap()).unwrap()
}

fn count(ix: &EntityIndex, kind: Kind) -> usize {
    ix.entities.iter().filter(|e| e.kind == kind).count()
}

fn names(hits: &[openframe_application::modules::ai::index::Hit]) -> Vec<&str> {
    hits.iter().map(|h| h.name.as_str()).collect()
}

fn low_tide() -> (tempfile::TempDir, Arc<AppCore>, Arc<EntityIndex>) {
    let (dir, core) = core_with_project();
    fixture::build(&core, fixture::LOW_TIDE).unwrap();
    let ix = index_of(&core);
    (dir, core, ix)
}

#[test]
fn the_index_holds_what_the_facts_file_states() {
    let (_dir, _core, ix) = low_tide();
    let sp: Vec<_> = ix
        .screenplays
        .iter()
        .map(|s| (s.title.as_str(), s.scene_count))
        .collect();
    assert_eq!(
        sp,
        vec![("LOW TIDE RADIO", 45), ("Low Tide Radio teaser trailer", 3)]
    );
    assert_eq!(count(&ix, Kind::Scene), 48);
    assert_eq!(count(&ix, Kind::Character), 13);
    // 12 cast and 14 crew entries; Prasad Venugopalan is in both lists.
    assert_eq!(count(&ix, Kind::Person), 26);
    assert_eq!(count(&ix, Kind::Location), 16);
    assert_eq!(count(&ix, Kind::Act), 3);
    assert_eq!(count(&ix, Kind::Card), 9);
    assert_eq!(count(&ix, Kind::Day), 14);
    assert_eq!(count(&ix, Kind::File), 4);
    assert_eq!(count(&ix, Kind::BudgetLine), 11);
    assert_eq!(count(&ix, Kind::VaultItem), 4);
    assert_eq!(count(&ix, Kind::CallSheet), 6);

    // 12 Story relationships are kept as relations between characters.
    let story = ix
        .relations
        .iter()
        .filter(|r| !["plays", "appears_in", "in_act", "for_day"].contains(&r.kind.as_str()))
        .count();
    assert_eq!(story, 12);

    // Alternate cues linked with screenplay.set_character_link, and "Also called" names.
    let alias = |character: &str, alias: &str| {
        let e = ix
            .entities
            .iter()
            .find(|e| e.kind == Kind::Character && e.name == character)
            .unwrap_or_else(|| panic!("no character {character}"));
        assert!(
            e.aliases.iter().any(|a| a.eq_ignore_ascii_case(alias)),
            "{character} has aliases {:?}, not {alias}",
            e.aliases
        );
    };
    for (character, a) in [
        ("Anjali", "ANJU"),
        ("Anjali", "Anju"),
        ("Mariamma", "AMMACHI"),
        ("Varghese", "VARKEY"),
        ("Sudheesh", "SUDHI"),
        ("Rosily", "ROSY"),
        ("Fr. Thomas", "ACHAN"),
        ("Fr. Thomas", "Thomas"),
        ("Nandan", "NANDU"),
        ("Salim", "SALIM BHAI"),
    ] {
        alias(character, a);
    }

    // Who plays whom, from the cast list.
    let young = ix
        .entities
        .iter()
        .find(|e| e.kind == Kind::Character && e.name == "Young Anjali")
        .unwrap();
    let players: Vec<_> = ix
        .played_by(&young.id)
        .iter()
        .map(|e| e.name.clone())
        .collect();
    assert_eq!(players, vec!["Aadhya Varma"]);
    // Jose is not cast.
    let jose = ix
        .entities
        .iter()
        .find(|e| e.kind == Kind::Character && e.name == "Jose")
        .unwrap();
    assert!(ix.played_by(&jose.id).is_empty());
    // Cues seen in each screenplay: Young Anjali only speaks in the feature.
    let appears = |character: &str, sp: &str| {
        let c = ix.entities.iter().find(|e| e.name == character).unwrap();
        let s = ix.screenplays.iter().find(|s| s.title == sp).unwrap();
        ix.relations
            .iter()
            .any(|r| r.kind == "appears_in" && r.from == c.id && r.to == s.id)
    };
    assert!(appears("Young Anjali", "LOW TIDE RADIO"));
    assert!(!appears("Young Anjali", "Low Tide Radio teaser trailer"));
    assert!(appears("Anjali", "Low Tide Radio teaser trailer"));
}

fn say(ix: &EntityIndex, focus: &Focus, message: &str) -> Resolved {
    resolve(ix, focus, message)
}

#[test]
fn open_scene_42_is_the_feature_without_a_question() {
    let (_dir, _core, ix) = low_tide();
    let r = say(&ix, &Focus::default(), "open scene 42");
    assert!(r.ambiguous.is_none(), "{:?}", r.ambiguous);
    assert_eq!(names(&r.screenplays), vec!["LOW TIDE RADIO"]);
    assert_eq!(r.screenplays[0].how, How::SceneCount);
    assert_eq!(r.scenes.len(), 1);
    assert_eq!(r.scenes[0].ordinal, 42);
    assert_eq!(
        r.scenes[0].heading.as_deref(),
        Some("EXT. COAST RADIO STATION - DAY")
    );
}

#[test]
fn scene_46_does_not_exist_and_the_answer_says_so() {
    let (_dir, _core, ix) = low_tide();
    let r = say(&ix, &Focus::default(), "what happens in scene 46");
    assert!(r.scenes.is_empty() && r.ambiguous.is_none());
    assert!(
        r.not_found[0].contains("45") && r.not_found[0].contains("3"),
        "{:?}",
        r.not_found
    );
}

/// "How many characters in this story" with no focus. The Story characters belong to the project,
/// not to a screenplay, so the message needs no screenplay: the resolver picks none and asks
/// nothing. Counting scenes reads every screenplay (fix round after the Low Tide run), so it asks
/// nothing either. "Describe the scenes" does need one screenplay, and with no focus and a current
/// draft in both screenplays (rule d finds two) it asks, with both titles.
#[test]
fn a_project_wide_question_needs_no_screenplay_and_a_scene_question_asks_which() {
    let (_dir, _core, ix) = low_tide();
    let r = say(&ix, &Focus::default(), "how many characters in this story");
    assert!(r.screenplays.is_empty() && r.ambiguous.is_none());

    let r = say(&ix, &Focus::default(), "how many scenes are there");
    assert!(
        r.ambiguous.is_none(),
        "a count across screenplays asks nothing"
    );

    let r = say(&ix, &Focus::default(), "describe the scenes");
    let q = r.ambiguous.expect("a question");
    assert!(
        q.options.len() == 2 && q.text.starts_with("Which screenplay do you mean:"),
        "{}",
        q.text
    );
}

#[test]
fn who_plays_young_anjali_is_the_character_young_anjali() {
    let (_dir, _core, ix) = low_tide();
    let r = say(&ix, &Focus::default(), "who plays young anjali");
    assert_eq!(names(&r.characters), vec!["Young Anjali"]);
    assert!(r.ambiguous.is_none() && r.screenplays.is_empty());
    let typo = say(&ix, &Focus::default(), "who plays yung anjali");
    assert_eq!(names(&typo.characters), vec!["Young Anjali"]);
    let id = &r.characters[0].id;
    let players: Vec<_> = ix.played_by(id).iter().map(|e| e.name.as_str()).collect();
    assert_eq!(players, vec!["Aadhya Varma"]);
}

#[test]
fn mariama_and_varghese_are_both_found() {
    let (_dir, _core, ix) = low_tide();
    let r = say(&ix, &Focus::default(), "mariama and varghese");
    let mut found = names(&r.characters);
    found.sort();
    assert_eq!(found, vec!["Mariamma", "Varghese"]);
    assert!(r.ambiguous.is_none());
}

#[test]
fn aliases_cues_and_the_teaser_resolve() {
    let (_dir, _core, ix) = low_tide();
    let r = say(
        &ix,
        &Focus::default(),
        "what does Ammachi say to Anju in the teaser",
    );
    let mut found = names(&r.characters);
    found.sort();
    assert_eq!(found, vec!["Anjali", "Mariamma"]);
    assert_eq!(names(&r.screenplays), vec!["Low Tide Radio teaser trailer"]);
    let r = say(&ix, &Focus::default(), "LOW TIDE RADIO");
    assert_eq!(names(&r.screenplays), vec!["LOW TIDE RADIO"]);
    let r = say(
        &ix,
        &Focus::default(),
        "who is at the coast radio station roof on day 4",
    );
    assert_eq!(names(&r.locations), vec!["Coast Radio Station Roof"]);
    assert_eq!(names(&r.days), vec!["Shoot Day 4"]);
}

#[test]
fn the_answer_to_which_screenplay_settles_the_next_question() {
    let (_dir, _core, ix) = low_tide();
    let asked = Focus::default().after(&say(&ix, &Focus::default(), "how many scenes are there"));
    let answered = asked.after(&say(&ix, &asked, "LOW TIDE RADIO"));
    let again = say(&ix, &answered, "how many scenes are there");
    assert!(again.ambiguous.is_none());
    assert_eq!(names(&again.screenplays), vec!["LOW TIDE RADIO"]);
    assert_eq!(again.screenplays[0].how, How::Focus);
}

#[test]
fn the_index_is_kept_until_a_write_and_then_rebuilt() {
    let (_dir, core, first) = low_tide();
    let same = index_of(&core);
    assert!(Arc::ptr_eq(&first, &same), "no write, so no rebuild");
    assert!(
        say(&first, &Focus::default(), "tell me about the old quay")
            .locations
            .is_empty()
    );

    q(&core, "locations.create", json!({ "name": "Old Quay" }));
    let after = index_of(&core);
    assert!(
        !Arc::ptr_eq(&first, &after),
        "a write makes the index stale"
    );
    let r = say(&after, &Focus::default(), "tell me about the old quay");
    assert_eq!(names(&r.locations), vec!["Old Quay"]);
    assert!(Arc::ptr_eq(&after, &index_of(&core)));
}

/// The index of the 500-scene project builds fast. A release build must stay under 200 ms; a debug
/// build is several times slower, so it only has to stay under two seconds.
#[test]
fn the_big_project_index_builds_within_the_budget() {
    let (_dir, core) = core_with_project();
    fixture::build(&core, fixture::BIG).unwrap();
    let s = core.project().unwrap();
    let mut best = std::time::Duration::MAX;
    let mut ix = None;
    for _ in 0..5 {
        let t = Instant::now();
        let built = s.store.read(EntityIndex::from_database).unwrap();
        best = best.min(t.elapsed());
        ix = Some(built);
    }
    let ix = ix.unwrap();
    eprintln!(
        "big-project index: {} entities, {} scenes, best of 5 builds {:?} ({})",
        ix.entities.len(),
        count(&ix, Kind::Scene),
        best,
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    assert_eq!(ix.screenplays[0].scene_count, 500);
    assert_eq!(count(&ix, Kind::Scene), 500);
    let limit = if cfg!(debug_assertions) { 2_000 } else { 200 };
    assert!(best.as_millis() < limit, "index build took {best:?}");
    // Resolving against it is just as fast.
    let t = Instant::now();
    let r = resolve(&ix, &Focus::default(), "open scene 321");
    eprintln!("resolve on big-project: {:?}", t.elapsed());
    assert_eq!(r.scenes[0].ordinal, 321);
}

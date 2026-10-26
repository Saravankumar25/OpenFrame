//! The resolver on small hand-built indexes: no database, no model.

use super::*;

const FEATURE: &str = "sp-feature";
const TEASER: &str = "sp-teaser";

fn screenplay(id: &str, title: &str, scenes: u32, current: bool) -> ScreenplayInfo {
    ScreenplayInfo {
        id: id.into(),
        title: title.into(),
        format: "Feature".into(),
        scene_count: scenes,
        has_current_draft: current,
        aliases: Vec::new(),
    }
}

fn character(id: &str, name: &str, aliases: &[&str]) -> Entity {
    let mut e = Entity::new(Kind::Character, id, name);
    e.aliases = aliases.iter().map(|a| a.to_string()).collect();
    e
}

fn person(id: &str, name: &str) -> Entity {
    let mut e = Entity::new(Kind::Person, id, name);
    e.tokens = name.split(' ').map(str::to_string).collect();
    e
}

/// "Low Tide Radio" (45 scenes) and its teaser (3 scenes), both with a current draft.
fn build(teaser_has_current_draft: bool) -> EntityIndex {
    let mut entities = Vec::new();
    for (sp, count) in [(FEATURE, 45), (TEASER, 3)] {
        for n in 1..=count {
            let mut s = Entity::new(
                Kind::Scene,
                format!("{sp}-scene-{n}"),
                format!("INT. PLACE {n} - DAY"),
            );
            s.screenplay_id = Some(sp.into());
            s.ordinal = Some(n);
            if sp == FEATURE && n == 12 {
                s.aliases.push("12A".into());
            }
            entities.push(s);
        }
    }
    entities.push(character("ch-anjali", "Anjali", &["ANJU"]));
    entities.push(character("ch-young", "Young Anjali", &[]));
    entities.push(character("ch-mariamma", "Mariamma", &["AMMACHI"]));
    entities.push(character("ch-varghese", "Varghese", &["VARKEY"]));
    entities.push(person("p-devika", "Devika Ramanathan"));
    entities.push(person("p-aadhya", "Aadhya Varma"));
    entities.push(person("p-anil-k", "Anil Kumar"));
    entities.push(person("p-anil-m", "Anil Menon"));
    let mut loc = Entity::new(Kind::Location, "loc-station", "Coast Radio Station");
    loc.detail = "Confirmed".into();
    entities.push(loc);
    let mut day = Entity::new(Kind::Day, "day-4", "Shoot Day 4");
    day.ordinal = Some(4);
    day.aliases.push("day 4".into());
    entities.push(day);
    let relations = vec![Relation {
        from: "p-aadhya".into(),
        to: "ch-young".into(),
        kind: "plays".into(),
    }];
    EntityIndex::new(
        vec![
            screenplay(FEATURE, "LOW TIDE RADIO", 45, true),
            screenplay(
                TEASER,
                "Low Tide Radio teaser trailer",
                3,
                teaser_has_current_draft,
            ),
        ],
        entities,
        relations,
    )
}

fn names(hits: &[Hit]) -> Vec<&str> {
    hits.iter().map(|h| h.name.as_str()).collect()
}

fn on(screenplay: &str) -> Focus {
    Focus {
        screenplay: Some(screenplay.into()),
        ..Focus::default()
    }
}

// ---- screenplay rule (a): named in the message

#[test]
fn a_title_in_any_case_names_the_screenplay() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "LOW TIDE RADIO");
    assert_eq!(r.screenplays.len(), 1);
    assert_eq!(r.screenplays[0].id, FEATURE);
    assert_eq!(r.screenplays[0].how, How::Exact);
    assert!(r.ambiguous.is_none());
    let r = resolve(&ix, &Focus::default(), "how long is low tide radio?");
    assert_eq!(r.screenplay_id(), Some(FEATURE));
}

#[test]
fn the_longer_title_wins_over_the_title_inside_it() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "open low tide radio teaser trailer");
    assert_eq!(r.screenplay_id(), Some(TEASER));
    // The extra word alone is enough: it belongs to the teaser, not to the feature.
    let r = resolve(
        &ix,
        &Focus::default(),
        "how many scenes in low tide radio teaser",
    );
    assert_eq!(names(&r.screenplays), vec!["Low Tide Radio teaser trailer"]);
}

#[test]
fn the_teaser_the_trailer_and_the_feature_are_aliases() {
    let ix = build(true);
    for (msg, want) in [
        ("show me the teaser", TEASER),
        ("what is in the trailer", TEASER),
        ("list the scenes of the feature", FEATURE),
    ] {
        let r = resolve(&ix, &Focus::default(), msg);
        assert_eq!(r.screenplay_id(), Some(want), "{msg}");
        assert_eq!(r.screenplays[0].how, How::Alias, "{msg}");
    }
}

#[test]
fn two_screenplays_in_one_message_are_both_found() {
    let ix = build(true);
    let r = resolve(
        &ix,
        &Focus::default(),
        "compare low tide radio and the teaser",
    );
    assert_eq!(r.screenplays.len(), 2);
}

#[test]
fn a_typo_in_the_title_still_finds_the_screenplay() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "open low tied radio");
    assert_eq!(r.screenplay_id(), Some(FEATURE));
    assert!(matches!(r.screenplays[0].how, How::Fuzzy(s) if s > 0.8));
}

#[test]
fn a_title_in_the_message_beats_the_focus() {
    let ix = build(true);
    let r = resolve(&ix, &on(FEATURE), "how many scenes in the teaser");
    assert_eq!(r.screenplay_id(), Some(TEASER));
}

// ---- rule (b): the focus

#[test]
fn the_focus_screenplay_settles_a_message_that_names_none() {
    let ix = build(true);
    let r = resolve(&ix, &on(TEASER), "how many scenes are there");
    assert_eq!(r.screenplay_id(), Some(TEASER));
    assert_eq!(r.screenplays[0].how, How::Focus);
    assert!(r.ambiguous.is_none());
}

#[test]
fn a_scene_the_focus_screenplay_lacks_goes_to_the_one_that_has_it() {
    let ix = build(true);
    let r = resolve(&ix, &on(TEASER), "open scene 42");
    assert_eq!(r.screenplay_id(), Some(FEATURE));
    assert_eq!(r.screenplays[0].how, How::SceneCount);
    assert_eq!(r.scenes[0].ordinal, 42);
}

// ---- rule (c): the scene number

#[test]
fn a_scene_number_only_one_screenplay_reaches_picks_that_screenplay() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "open scene 42");
    assert_eq!(r.screenplay_id(), Some(FEATURE));
    assert_eq!(r.screenplays[0].how, How::SceneCount);
    assert_eq!(r.scenes.len(), 1);
    assert_eq!(r.scenes[0].ordinal, 42);
    assert_eq!(r.scenes[0].scene_id.as_deref(), Some("sp-feature-scene-42"));
    assert!(r.ambiguous.is_none() && r.not_found.is_empty());
}

#[test]
fn scenes_listed_with_and_are_all_found() {
    let ix = build(true);
    let r = resolve(&ix, &on(FEATURE), "compare scenes 3 and 41");
    assert_eq!(
        r.scenes.iter().map(|s| s.ordinal).collect::<Vec<_>>(),
        vec![3, 41]
    );
}

#[test]
fn a_production_scene_number_with_a_letter_is_found() {
    let ix = build(true);
    let r = resolve(&ix, &on(FEATURE), "read scene 12a");
    assert_eq!(r.scenes[0].ordinal, 12);
    let r = resolve(&ix, &on(FEATURE), "read scene 13b");
    assert!(r.scenes.is_empty());
    assert!(
        r.not_found[0].contains("no scene numbered 13B"),
        "{:?}",
        r.not_found
    );
}

#[test]
fn scene_46_of_45_is_not_a_scene_and_says_so() {
    let ix = build(true);
    let r = resolve(&ix, &on(FEATURE), "what happens in scene 46");
    assert!(r.scenes.is_empty());
    assert_eq!(r.not_found.len(), 1);
    assert!(
        r.not_found[0].contains("has 45 scenes") && r.not_found[0].contains("no scene 46"),
        "{:?}",
        r.not_found
    );
    // No focus and no screenplay reaches it: nothing to ask, the answer is the same for both.
    let r = resolve(&ix, &Focus::default(), "what happens in scene 46");
    assert!(r.scenes.is_empty() && r.ambiguous.is_none());
    assert!(r.not_found[0].contains("has 45") && r.not_found[0].contains("has 3"));
}

// ---- rules (d) and (e)

#[test]
fn the_only_screenplay_with_a_current_draft_is_the_one() {
    let ix = build(false);
    let r = resolve(&ix, &Focus::default(), "how many scenes are there");
    assert_eq!(r.screenplay_id(), Some(FEATURE));
    assert_eq!(r.screenplays[0].how, How::CurrentDraft);
}

#[test]
fn otherwise_one_specific_question_with_both_titles() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "how many scenes are there");
    assert!(r.screenplays.is_empty());
    let q = r.ambiguous.expect("a question");
    assert_eq!(
        q.text,
        "Which screenplay do you mean: \u{201C}LOW TIDE RADIO\u{201D} or \u{201C}Low Tide Radio teaser trailer\u{201D}?"
    );
    assert_eq!(q.options.len(), 2);
    // Scene 2 exists in both, so the scene number does not decide either.
    let r = resolve(&ix, &Focus::default(), "open scene 2");
    assert!(r.ambiguous.is_some() && r.scenes.is_empty());
}

#[test]
fn a_question_about_the_whole_story_needs_no_screenplay() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "how many characters in this story");
    assert!(r.screenplays.is_empty() && r.ambiguous.is_none() && r.not_found.is_empty());
}

#[test]
fn a_single_screenplay_project_never_asks() {
    let ix = EntityIndex::new(
        vec![screenplay("only", "The Short", 5, true)],
        Vec::new(),
        Vec::new(),
    );
    let r = resolve(&ix, &Focus::default(), "how many scenes are there");
    assert_eq!(r.screenplay_id(), Some("only"));
    assert_eq!(r.screenplays[0].how, How::OnlyOne);
}

// ---- names, aliases, typos

#[test]
fn a_linked_cue_is_an_alias_of_the_character() {
    let ix = build(true);
    for (msg, id) in [
        ("where does ANJU appear", "ch-anjali"),
        ("what does Ammachi say", "ch-mariamma"),
        ("is varkey in scene 5", "ch-varghese"),
    ] {
        let r = resolve(&ix, &on(FEATURE), msg);
        assert_eq!(r.characters.len(), 1, "{msg}");
        assert_eq!(r.characters[0].id, id, "{msg}");
        assert_eq!(r.characters[0].how, How::Alias, "{msg}");
    }
}

#[test]
fn young_anjali_is_not_read_as_anjali_and_a_typo_still_finds_her() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "who plays young anjali");
    assert_eq!(names(&r.characters), vec!["Young Anjali"]);
    assert_eq!(r.characters[0].how, How::Exact);
    let r = resolve(&ix, &Focus::default(), "who plays yung anjali");
    assert_eq!(names(&r.characters), vec!["Young Anjali"]);
    assert!(matches!(r.characters[0].how, How::Fuzzy(_)));
    // The relation answers "who plays": Aadhya Varma.
    let who: Vec<_> = ix
        .played_by("ch-young")
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(who, vec!["Aadhya Varma"]);
}

#[test]
fn two_names_with_one_typo_are_both_found() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "mariama and varghese");
    let mut found = names(&r.characters);
    found.sort();
    assert_eq!(found, vec!["Mariamma", "Varghese"]);
    assert!(r.ambiguous.is_none());
}

#[test]
fn a_short_word_is_never_corrected_into_a_name() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "anj is here");
    assert!(r.characters.is_empty());
}

#[test]
fn places_and_days_are_found_by_name_and_by_number() {
    let ix = build(true);
    let r = resolve(
        &ix,
        &Focus::default(),
        "who is at the coast radio station on day 4",
    );
    assert_eq!(names(&r.locations), vec!["Coast Radio Station"]);
    assert_eq!(names(&r.days), vec!["Shoot Day 4"]);
}

// ---- two people with similar names

#[test]
fn a_first_name_two_people_share_asks_which_one() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "what is anil's phone number");
    assert!(r.persons.is_empty());
    let q = r.ambiguous.expect("a question");
    assert_eq!(
        q.text,
        "Which person do you mean: Anil Kumar or Anil Menon?"
    );
}

#[test]
fn the_full_name_or_the_focus_settles_a_shared_first_name() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "what is anil menon's role");
    assert_eq!(names(&r.persons), vec!["Anil Menon"]);
    assert!(r.ambiguous.is_none());
    let focus = Focus {
        persons: vec!["p-anil-k".into()],
        ..Focus::default()
    };
    let r = resolve(&ix, &focus, "what is anil's phone number");
    assert_eq!(names(&r.persons), vec!["Anil Kumar"]);
    assert_eq!(r.persons[0].how, How::Focus);
}

#[test]
fn a_last_name_is_a_weaker_match_than_a_cue_alias() {
    let ix = build(true);
    let r = resolve(&ix, &Focus::default(), "tell me about devika");
    assert_eq!(names(&r.persons), vec!["Devika Ramanathan"]);
    assert_eq!(r.persons[0].how, How::Alias);
}

// ---- pronouns and the focus

#[test]
fn she_and_him_take_the_focus() {
    let ix = build(true);
    let focus = Focus {
        characters: vec!["ch-mariamma".into()],
        ..Focus::default()
    };
    let r = resolve(&ix, &focus, "what does she say in scene 3");
    assert_eq!(names(&r.characters), vec!["Mariamma"]);
    assert_eq!(r.characters[0].how, How::Pronoun);
    // A name in the message wins over the pronoun.
    let r = resolve(&ix, &focus, "does she meet varghese");
    assert_eq!(names(&r.characters), vec!["Varghese"]);
    // Nothing in focus: nothing to take.
    let r = resolve(&ix, &Focus::default(), "what does she say");
    assert!(r.characters.is_empty() && r.persons.is_empty());
}

#[test]
fn that_scene_takes_the_focus_scene() {
    let ix = build(true);
    let focus = Focus {
        screenplay: Some(FEATURE.into()),
        scene: Some(7),
        ..Focus::default()
    };
    let r = resolve(&ix, &focus, "who is in that scene");
    assert_eq!(r.scenes.len(), 1);
    assert_eq!((r.scenes[0].ordinal, r.scenes[0].how), (7, How::Pronoun));
    assert_eq!(r.screenplay_id(), Some(FEATURE));
}

#[test]
fn the_focus_follows_the_message_and_keeps_what_it_did_not_name() {
    let ix = build(true);
    let f1 = Focus::default().after(&resolve(
        &ix,
        &Focus::default(),
        "open scene 42 of the feature",
    ));
    assert_eq!(
        (f1.screenplay.as_deref(), f1.scene),
        (Some(FEATURE), Some(42))
    );
    let f2 = f1.after(&resolve(&ix, &f1, "what does mariamma say"));
    assert_eq!(f2.characters, vec!["ch-mariamma".to_string()]);
    assert_eq!(
        (f2.screenplay.as_deref(), f2.scene),
        (Some(FEATURE), Some(42))
    );
    // "she" keeps the same person; a new screenplay clears the scene.
    let f3 = f2.after(&resolve(&ix, &f2, "and what does she want"));
    assert_eq!(f3.characters, f2.characters);
    let f4 = f3.after(&resolve(&ix, &f3, "now the teaser"));
    assert_eq!((f4.screenplay.as_deref(), f4.scene), (Some(TEASER), None));
}

#[test]
fn the_answer_to_the_question_settles_the_next_message() {
    let ix = build(true);
    let ask = resolve(&ix, &Focus::default(), "how many scenes are there");
    assert!(ask.ambiguous.is_some());
    let asked = Focus::default().after(&ask);
    assert!(asked.is_empty());
    let answered = asked.after(&resolve(&ix, &asked, "LOW TIDE RADIO"));
    assert_eq!(answered.screenplay.as_deref(), Some(FEATURE));
    let again = resolve(&ix, &answered, "how many scenes are there");
    assert_eq!(again.screenplay_id(), Some(FEATURE));
    assert!(again.ambiguous.is_none());
}

#[test]
fn the_focus_round_trips_through_json() {
    let f = Focus {
        screenplay: Some("a".into()),
        scene: Some(3),
        characters: vec!["c".into()],
        persons: Vec::new(),
        answer_type: None,
    };
    let v = focus::with_focus(serde_json::json!({"kind": "WholeProject"}), &f);
    assert_eq!(v["kind"], "WholeProject");
    let back: Focus = serde_json::from_value(v["focus"].clone()).unwrap();
    assert_eq!(back, f);
    // An older record without a focus, or with extra keys, still loads.
    let old: Focus = serde_json::from_str(r#"{"screenplay":"x","later":1}"#).unwrap();
    assert_eq!(old.screenplay.as_deref(), Some("x"));
}

#[test]
fn a_message_with_nothing_in_it_resolves_to_nothing() {
    let ix = build(true);
    for msg in ["", "   ", "hello", "thanks!"] {
        assert_eq!(
            resolve(&ix, &Focus::default(), msg),
            Resolved::default(),
            "{msg:?}"
        );
    }
}

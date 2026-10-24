use rusqlite::params;

use super::super::scope::{AiScopeKind, ResolvedScope};
use super::super::test_script::{add, project, sample, scene, talk};
use super::*;

fn w(s: &str) -> Vec<String> {
    words(s)
}

#[test]
fn one_letter_off_is_found_and_equal_is_not() {
    assert!(within_one("KIRANN", "KIRAN"));
    assert!(within_one("KIRAN", "KIRANN"));
    assert!(within_one("KIREN", "KIRAN"));
    assert!(within_one("KIRRAN", "KIRRUN"));
    assert!(!within_one("KIRAN", "KIRAN"));
    assert!(!within_one("KIRRAN", "KORRUN"));
    assert!(!within_one("KIRAN", "KIR"));
}

#[test]
fn act_words_map_to_numbers() {
    assert_eq!(act_refs(&w("what happens in act 2")), vec![2]);
    assert_eq!(act_refs(&w("Act two and the second act")), vec![2, 2]);
    assert_eq!(act_refs(&w("explain Act II")), vec![2]);
    assert_eq!(act_refs(&w("acts 1 and 3")), vec![1, 3]);
    assert_eq!(act_refs(&w("the last act")), vec![usize::MAX]);
    assert!(act_refs(&w("act of kindness")).is_empty());
}

#[test]
fn intents_follow_the_keyword_rules() {
    let i = |t: &str, chars: usize| intent_of(&plain(t), chars);
    assert_eq!(i("what's between Devi and Kiran", 2), Intent::Relationship);
    assert_eq!(i("compare Devi and Kiran", 2), Intent::Compare);
    assert_eq!(i("Devi vs Kiran", 2), Intent::Compare);
    assert_eq!(i("what is the theme of the film", 0), Intent::Theme);
    assert_eq!(i("give me notes on scene 3", 0), Intent::Critique);
    assert_eq!(i("brainstorm a better ending", 0), Intent::Ideas);
    assert_eq!(i("how many scenes has Devi", 1), Intent::ListOrCount);
    assert_eq!(i("summarise the whole script", 0), Intent::Whole);
    assert_eq!(i("who's Devi", 1), Intent::Character);
    assert_eq!(i("where is the kite found", 0), Intent::Find);
    assert_eq!(i("what happens in scene 5", 0), Intent::Explain);
    assert_eq!(i("hello there", 0), Intent::Other);
}

#[test]
fn depth_reads_length_words_then_deep_words_then_the_shape_of_the_question() {
    let d = |t: &str, intent: Intent, whole: bool| depth_of(&plain(t), intent, whole);
    assert_eq!(
        d("who is Devi, briefly", Intent::Character, false),
        Depth::Quick
    );
    assert_eq!(
        d("Does Devi meet Kiran in scene 4?", Intent::Other, false),
        Depth::Quick
    );
    assert_eq!(
        d("Can you explain scene 4", Intent::Explain, false),
        Depth::Explain
    );
    assert_eq!(
        d("Is scene 4 sad? Explain why", Intent::Other, false),
        Depth::Explain
    );
    assert_eq!(d("where is the kite", Intent::Find, false), Depth::Quick);
    assert_eq!(d("describe Devi", Intent::Character, false), Depth::Explain);
    assert_eq!(d("break down scene 4", Intent::Other, false), Depth::Deep);
    assert_eq!(d("compare them", Intent::Compare, false), Depth::Deep);
    assert_eq!(d("summarise it all", Intent::Whole, true), Depth::Deep);
}

fn scope_of(draft: &DraftRef, scene: Option<queries::SceneRow>) -> ResolvedScope {
    ResolvedScope {
        kind: AiScopeKind::CurrentScreenplay,
        label: String::new(),
        draft: Some(draft.clone()),
        scene,
        selection: Vec::new(),
        context: Vec::new(),
    }
}

fn ask(c: &Connection, d: &DraftRef, text: &str, previous: &[Entity]) -> Understood {
    understand(c, d, &scope_of(d, None), text, previous).unwrap()
}

fn who(u: &Understood) -> Vec<(&str, &str)> {
    u.entities
        .iter()
        .map(|e| {
            let k = match e.kind {
                EntityKind::Character => "character",
                EntityKind::Location => "location",
                EntityKind::Scene => "scene",
                EntityKind::Act => "act",
            };
            (k, e.label.as_str())
        })
        .collect()
}

fn character(name: &str) -> Entity {
    Entity {
        kind: EntityKind::Character,
        id: name.to_uppercase(),
        label: name.to_uppercase(),
    }
}

#[test]
fn two_characters_in_the_order_they_are_named_and_a_relationship_intent() {
    let c = project();
    let d = sample(&c, "example-short");
    assert_eq!(queries::scenes(&c, &d.id).unwrap().len(), 10);
    let u = ask(&c, &d, "what's between Devi and kiran", &[]);
    assert_eq!(who(&u), vec![("character", "DEVI"), ("character", "KIRAN")]);
    assert_eq!(u.intent, Intent::Relationship);
    assert!(u.fuzzy.is_empty() && !u.pronoun_unresolved);
    // Possessive, other order, repeated mention counted once.
    let u = ask(&c, &d, "Kiran's kite and Devi's register, Kiran again", &[]);
    assert_eq!(who(&u), vec![("character", "KIRAN"), ("character", "DEVI")]);
}

#[test]
fn a_name_one_letter_off_is_matched_and_reported() {
    let c = project();
    let d = sample(&c, "example-short");
    let u = ask(&c, &d, "What does Kirann want in scene 1?", &[]);
    assert_eq!(who(&u), vec![("character", "KIRAN")]);
    assert_eq!(u.fuzzy, vec![("Kirann".to_string(), "KIRAN".to_string())]);
    assert_eq!(u.scenes, vec![1]);
    // A four-letter name is too short to guess at, and a word far from any name is left alone.
    let u = ask(&c, &d, "Is Devis kite red? Kiren is a kid, not a goat", &[]);
    assert!(
        u.fuzzy.len() == 1 && u.fuzzy[0].0 == "Kiren",
        "{:?}",
        u.fuzzy
    );
}

#[test]
fn scene_references_special_phrases_and_missing_scenes() {
    let c = project();
    let d = sample(&c, "example-short");
    assert_eq!(
        ask(&c, &d, "What happens in the last scene?", &[]).scenes,
        vec![10]
    );
    assert_eq!(
        ask(&c, &d, "the final scene and the opening", &[]).scenes,
        vec![1, 10]
    );
    let u = ask(&c, &d, "Explain scene 40", &[]);
    assert!(u.scenes.is_empty());
    assert_eq!(u.missing_scenes, vec![40]);
    let u = ask(&c, &d, "Compare scenes 3-5 with scene 11", &[]);
    assert_eq!((u.scenes, u.missing_scenes), (vec![3, 4, 5], vec![11]));
    // "and in scene 6?" only adds scenes.
    let u = ask(&c, &d, "and in scene 6?", &[character("Devi")]);
    assert_eq!((u.scenes, u.entities.len()), (vec![6], 0));
    // "this scene" is the scene in view.
    let rows = queries::scenes(&c, &d.id).unwrap();
    let sc = scope_of(&d, Some(rows[3].clone()));
    let u = understand(&c, &d, &sc, "Explain this scene", &[]).unwrap();
    assert_eq!(u.scenes, vec![4]);
}

#[test]
fn depth_and_intent_from_whole_questions() {
    let c = project();
    let d = sample(&c, "example-short");
    let u = ask(&c, &d, "Who is Devi, briefly?", &[]);
    assert_eq!((u.depth, u.intent), (Depth::Quick, Intent::Character));
    let u = ask(&c, &d, "Analyse the whole script in detail", &[]);
    assert_eq!(
        (u.depth, u.intent, u.whole_range),
        (Depth::Deep, Intent::Whole, true)
    );
    let u = ask(&c, &d, "Where does Devi first find the kite?", &[]);
    assert_eq!(u.intent, Intent::Find);
    let u = ask(&c, &d, "How many scenes is Kiran in?", &[]);
    assert_eq!((u.intent, u.depth), (Intent::ListOrCount, Depth::Explain));
    let u = ask(&c, &d, "Compare Devi and Kiran", &[]);
    assert_eq!((u.intent, u.depth), (Intent::Compare, Depth::Deep));
    let u = ask(&c, &d, "Tell me about Devi", &[]);
    assert_eq!((u.intent, u.depth), (Intent::Character, Depth::Explain));
}

#[test]
fn a_pronoun_follows_one_character_and_asks_when_there_are_two() {
    let c = project();
    let d = sample(&c, "example-short");
    let both = [character("Devi"), character("Kiran")];
    let u = ask(&c, &d, "What does she say in scene 4?", &both);
    assert!(u.pronoun_unresolved && u.entities.is_empty());
    let u = ask(
        &c,
        &d,
        "What does she say in scene 4?",
        &[character("Devi")],
    );
    assert!(!u.pronoun_unresolved);
    assert_eq!(who(&u), vec![("character", "DEVI")]);
    // The same character twice in the earlier answer is still one candidate.
    let twice = [character("Devi"), character("Devi")];
    assert_eq!(
        ask(&c, &d, "and what about them?", &twice).entities.len(),
        1
    );
    // A named character wins over the earlier ones; no gender is read from any name.
    let u = ask(&c, &d, "Does Kiran trust her?", &both);
    assert_eq!(who(&u), vec![("character", "KIRAN")]);
    assert!(!u.pronoun_unresolved);
}

#[test]
fn locations_come_from_headings_whole_or_by_their_last_part() {
    let c = project();
    let d = sample(&c, "example-short");
    let u = ask(&c, &d, "What happens in the lost property office?", &[]);
    assert_eq!(
        who(&u),
        vec![("location", "KESARGANJ JUNCTION - LOST PROPERTY OFFICE")]
    );
    // "Kesarganj Junction" starts several locations, so it names none of them alone.
    let u = ask(
        &c,
        &d,
        "Is the footbridge scene sad at Kesarganj Junction",
        &[],
    );
    assert_eq!(
        who(&u),
        vec![("location", "KESARGANJ JUNCTION - FOOTBRIDGE")]
    );
    // Under four letters is not a location.
    assert!(ask(&c, &d, "Is the car there", &[]).entities.is_empty());
    // The Location record's spelling becomes the label.
    c.execute(
        "INSERT INTO location(id, name, created_at, updated_at) VALUES ('l1', 'Store Room', 1, 1)",
        [],
    )
    .unwrap();
    let u = ask(&c, &d, "who is in the store room", &[]);
    assert_eq!(who(&u), vec![("location", "Store Room")]);
}

#[test]
fn long_names_win_and_a_first_word_works_only_when_it_is_unique() {
    let c = project();
    let d = add(
        &c,
        "Names",
        &[
            talk(
                "INT. HALL - DAY",
                &[("VENKAT RAO", "Hello there."), ("ANITA ROY", "Hi.")],
            ),
            talk(
                "INT. HALL - NIGHT",
                &[("ANITA SEN", "Bye."), ("RAO", "Wait.")],
            ),
        ],
    );
    let u = ask(
        &c,
        &d,
        "What does Venkat say, and Venkat Rao's friend?",
        &[],
    );
    assert_eq!(who(&u), vec![("character", "VENKAT RAO")]);
    // ANITA starts two names: neither matches alone, both match whole.
    assert!(ask(&c, &d, "tell me about Anita", &[]).entities.is_empty());
    let u = ask(&c, &d, "Anita Sen and Anita Roy", &[]);
    assert_eq!(
        who(&u),
        vec![("character", "ANITA SEN"), ("character", "ANITA ROY")]
    );
    // RAO is a character of its own, so "Rao" is RAO and "Venkat Rao" is the long name.
    let u = ask(&c, &d, "Rao and Venkat Rao", &[]);
    assert_eq!(
        who(&u),
        vec![("character", "RAO"), ("character", "VENKAT RAO")]
    );
}

#[test]
fn characters_from_the_story_section_count_and_cue_extensions_are_ignored() {
    let c = project();
    let d = add(
        &c,
        "Cues",
        &[talk(
            "INT. HALL - DAY",
            &[("MEERA (V.O.)", "Hello."), ("MEERA (CONT'D)", "Again.")],
        )],
    );
    c.execute(
        "INSERT INTO story_character(id, name, position, created_at, updated_at)
         VALUES ('c1', 'Arjun', 1, 1, 1)",
        [],
    )
    .unwrap();
    let u = ask(&c, &d, "Is Arjun with Meera?", &[]);
    assert_eq!(
        who(&u),
        vec![("character", "Arjun"), ("character", "MEERA")]
    );
}

#[test]
fn acts_map_to_the_beat_board_order_and_nothing_without_acts() {
    let c = project();
    let d = sample(&c, "example-short");
    assert!(ask(&c, &d, "What happens in act 2?", &[]).acts.is_empty());
    for (i, t) in ["Setup", "Confrontation", "Resolution"].iter().enumerate() {
        c.execute(
            "INSERT INTO story_act(id, title, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, 1, 1)",
            params![format!("a{i}"), t, i as i64 + 1],
        )
        .unwrap();
    }
    let u = ask(&c, &d, "Act two, then the third act, then act 9", &[]);
    let labels: Vec<&str> = u.acts.iter().map(|a| a.label.as_str()).collect();
    assert_eq!(labels, vec!["Confrontation", "Resolution"]);
    assert_eq!(u.acts[0].id, "a1");
    assert_eq!(ask(&c, &d, "the last act", &[]).acts[0].label, "Resolution");
}

#[test]
fn omitted_scenes_are_reported_as_missing() {
    let c = project();
    let mut cut = scene("INT. CELLAR - NIGHT", &[("action", "Dark.")]);
    cut.omitted = true;
    let d = add(
        &c,
        "Omit",
        &[
            scene("INT. HALL - DAY", &[("action", "Bright.")]),
            cut,
            scene("EXT. ROAD - DAY", &[("action", "Long.")]),
        ],
    );
    let u = ask(&c, &d, "Scenes 1 to 3, and the last scene", &[]);
    assert_eq!((u.scenes, u.missing_scenes), (vec![1, 3], vec![2]));
}

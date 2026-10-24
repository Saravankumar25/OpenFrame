use super::*;

#[test]
fn delimiter_tags_never_reach_the_person() {
    let a = "<project_data trust=\"untrusted\"> [Project map] Venkat repairs radios.</project_data>\n<OBSERVATION step=\"1\" tool=\"x\">Lakshmi waits.</observation> <conversation_history> <product_guide> </product_guide";
    let c = clean(a);
    assert!(!c.contains('<'), "{c}");
    assert!(!c.contains("Project map"), "{c}");
    assert!(c.contains("Venkat repairs radios."), "{c}");
    assert!(c.contains("Lakshmi waits."), "{c}");
}

#[test]
fn briefing_metadata_labels_go_with_their_values() {
    let a = "Scene 4: INT. VENKAT RAO'S HOUSE - KITCHEN - NIGHT Scene 4: INT. VENKAT RAO'S HOUSE - KITCHEN - NIGHT Characters: NEWSREADER, LAKSHMI, VENKAT RAO Location: VENKAT RAO'S HOUSE Time: NIGHT";
    let c = clean(a);
    assert_eq!(c, "Scene 4: INT. VENKAT RAO'S HOUSE - KITCHEN - NIGHT");
}

#[test]
fn source_labels_and_draft_prefixes_go() {
    let a = "[Screenplay \u{b7} Scene 2: INT. VENKAT RADIO WORKS - DAY]\nScene: Scene 2: ...\nCharacters: VENKAT RAO, LAKSHMI Location: the shop\nScene 5 \u{b7} Example-Short (imported) Characters: DEVI, KIRAN Time: DAY\n[Briefing: Scene 4] [Act 2 of 3: The Chase]\nLakshmi hands Venkat the radio.";
    let c = clean(a);
    assert_eq!(c, "Lakshmi hands Venkat the radio.");
}

#[test]
fn record_and_note_lines_go() {
    let a = "Record: relationship A -> B: \"friend\"\nNote: 3 more lines not shown.\nDevi gives Kiran the kite.";
    assert_eq!(clean(a), "Devi gives Kiran the kite.");
}

#[test]
fn briefing_labels_in_front_of_a_value_are_dropped_and_the_value_kept() {
    let a = "Scene 4: synopsis: Devi gives Kiran the kite.\nScene 4: first action: Kiran waits.\nScene 4: the scene before is Scene 3: the market.";
    let c = clean(a);
    assert!(c.starts_with("Devi gives Kiran the kite."), "{c}");
    assert!(!c.contains("synopsis"), "{c}");
    assert!(!c.contains("first action"), "{c}");
    assert!(c.contains("Kiran waits."), "{c}");
}

#[test]
fn briefing_headings_are_dropped_before_hashes_are_stripped() {
    let t = super::super::orchestrator::plain_text(
        "## Scene 4 (Example-Short (imported))\n## Project map (Draft 1)\nDevi gives Kiran the kite.",
    );
    assert_eq!(t, "Devi gives Kiran the kite.");
}

#[test]
fn a_repeated_scene_heading_is_collapsed() {
    let a = "INT. SHOP - DAY INT. SHOP - DAY Venkat opens the shutter.";
    assert_eq!(clean(a), "INT. SHOP - DAY Venkat opens the shutter.");
    // Plain repetition in the story is not touched.
    let b = "He knocks and knocks and knocks and knocks.";
    assert_eq!(clean(b), b);
}

#[test]
fn sentences_about_the_reply_go_and_facts_stay() {
    let a = "Okay, here's the revised response, removing the relationship records and repeating the idea suggestion once:\nIdea: Let Lakshmi finish the call.";
    assert_eq!(clean(a), "Idea: Let Lakshmi finish the call.");
    assert_eq!(
        clean("Here is the corrected answer: Devi gives Kiran the kite."),
        "Devi gives Kiran the kite."
    );
    assert_eq!(
        clean(
            "As requested, I have shortened it. Devi gives Kiran the kite. I have removed the extra notes."
        ),
        "Devi gives Kiran the kite."
    );
    // A story sentence that starts with the same word is a fact.
    let story = "Removing the mask, Venkat laughs. He has removed his coat.";
    assert_eq!(clean(story), story);
}

#[test]
fn a_clean_answer_is_unchanged_and_layout_stays() {
    let a = "Devi gives Kiran the red kite (Scene 4). The time is short.\n\n- Kiran waits.\n- Note the pause.";
    assert_eq!(clean(a), a);
    assert_eq!(clean("Idea: one\nIdea: two"), "Idea: one\nIdea: two");
    assert_eq!(clean(""), "");
}

#[test]
fn a_label_inside_a_word_is_not_a_label() {
    let a = "Meantime: Devi waits. Footnote: none.";
    assert_eq!(clean(a), a);
}

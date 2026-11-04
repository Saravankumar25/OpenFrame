use super::*;

#[test]
fn delimiter_tags_never_reach_the_person() {
    let a = "<project_data trust=\"untrusted\"> Venkat repairs radios.</project_data>\n<OBSERVATION step=\"1\" tool=\"x\">Lakshmi waits.</observation> <conversation_history> <product_guide> </product_guide <facts> </facts> <memory> <focus>";
    let c = clean(a);
    assert!(!c.contains('<'), "{c}");
    assert!(c.contains("Venkat repairs radios."), "{c}");
    assert!(c.contains("Lakshmi waits."), "{c}");
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
fn a_label_that_is_part_of_the_story_is_not_removed() {
    let a = "Location: the harbour at night. Characters: none are named. Note: the radio is off.";
    assert_eq!(clean(a), a);
}

use std::time::Instant;

use super::super::orchestrator::{Style, plain_text_with};
use super::super::test_script::Scripted;
use super::*;

const EVIDENCE: &str = "Briefing: Scene 4\nScene 4: Devi gives Kiran the red kite.\nKIRAN: \"I will come tomorrow.\"\nScene 5: Kiran waits on the platform.";

fn facts(style: Style) -> Facts {
    Facts {
        request: "What happens in scene 4?".into(),
        evidence: vec![EVIDENCE.into()],
        names: "DEVI KIRAN".into(),
        live_scenes: [4, 5].into_iter().collect(),
        language: None,
        limits: Limits {
            max_sentences: None,
            max_paragraphs: None,
            max_words: 200,
        },
        style,
        min_words: None,
    }
}

fn codes(o: &[Offence]) -> Vec<&'static str> {
    o.iter().map(|o| o.check.code()).collect()
}

#[test]
fn a_grounded_answer_passes_every_check() {
    let f = facts(Style::Flat);
    let a = "Devi gives Kiran the red kite (Scene 4). He says \"I will come tomorrow.\"";
    assert!(check(a, &f).is_empty(), "{:?}", check(a, &f));
}

#[test]
fn each_check_names_its_offenders() {
    let f = facts(Style::Flat);
    let o = check("Marcus takes the kite (Scene 4).", &f);
    assert_eq!(codes(&o), ["names"]);
    assert_eq!(o[0].items, ["Marcus"]);

    // Scene 40 is not in the draft; Scene 9 is not in the evidence although the draft has it.
    let mut g = facts(Style::Flat);
    g.live_scenes.insert(9);
    let o = check("Devi waits (Scene 40). Kiran leaves (Scene 9).", &g);
    assert_eq!(codes(&o), ["scene_numbers"]);
    assert_eq!(o[0].items, ["Scene 9", "Scene 40"]);

    let o = check("Kiran says \"I will never come back again.\" (Scene 4)", &f);
    assert_eq!(codes(&o), ["quotes"]);

    let o = check("Devi has 7 kites in Scene 4.", &f);
    assert_eq!(codes(&o), ["numbers"]);
    assert_eq!(o[0].items, ["7"]);

    let o = check("Kiran is Devi's grandson (Scene 4). Her husband waits.", &f);
    assert_eq!(codes(&o), ["relationship_words"]);
    assert_eq!(o[0].items, ["husband"]);

    let o = check(
        "Devi waits. Devi waits. Devi waits. Devi waits. Devi waits (Scene 4).",
        &f,
    );
    assert_eq!(codes(&o), ["repetition"]);

    assert_eq!(codes(&check("   ", &f)), ["empty"]);
    let o = check("I renamed the scene for you (Scene 4).", &f);
    assert_eq!(codes(&o), ["change_claim"]);
}

#[test]
fn list_numbers_and_the_asked_scene_are_not_invented_numbers() {
    let f = facts(Style::Rich);
    let a =
        "Two things happen.\n1. Devi gives Kiran the kite (Scene 4).\n2. Kiran waits (Scene 5).";
    assert!(check(a, &f).is_empty(), "{:?}", check(a, &f));
}

#[test]
fn a_heading_of_a_deep_answer_is_not_a_name() {
    let f = facts(Style::Rich);
    let a =
        "Key Moments\nDevi gives Kiran the kite (Scene 4).\nHow It Ends\nKiran waits (Scene 5).";
    assert!(check(a, &f).is_empty(), "{:?}", check(a, &f));
    // The same lines in a quick answer: a capitalised run is checked.
    let q = facts(Style::Flat);
    assert_eq!(codes(&check("Devi meets Marcus Webb.", &q)), ["names"]);
}

#[test]
fn a_reply_in_the_wrong_language_fails_the_language_check() {
    let mut f = facts(Style::Flat);
    f.request = "\u{0938}\u{0940}\u{0928} 4 \u{092e}\u{0947}\u{0902} \u{0915}\u{094d}\u{092f}\u{093e} \u{0939}\u{094b}\u{0924}\u{093e} \u{0939}\u{0948}".into();
    f.language = expected_language(&f.request);
    assert_eq!(f.language, Some("hi"));
    let o = check("Devi gives Kiran the kite (Scene 4).", &f);
    assert_eq!(codes(&o), ["language"]);
    assert!(feedback(&o, &f).contains("Write the whole answer in Hindi."));
}

#[test]
fn feedback_is_one_short_machine_written_message() {
    let f = facts(Style::Flat);
    let o = check("Marcus takes the kite (Scene 40).", &f);
    let m = feedback(&o, &f);
    assert!(m.contains(
        "These names are not in the project: Marcus. Remove them or say you could not find them."
    ));
    assert!(m.contains("Scene 40"));
    assert!(m.ends_with("Reply with the corrected answer only."));
}

#[test]
fn the_length_is_trimmed_to_sentences_before_it_can_fail() {
    let l = Limits {
        max_sentences: Some(2),
        max_paragraphs: None,
        max_words: 40,
    };
    assert_eq!(fit("One. Two. Three.".into(), &l), "One. Two.");
    let l = Limits {
        max_sentences: None,
        max_paragraphs: Some(1),
        max_words: 40,
    };
    assert_eq!(fit("First para.\n\nSecond para.".into(), &l), "First para.");
    let l = Limits {
        max_sentences: None,
        max_paragraphs: None,
        max_words: 6,
    };
    assert_eq!(
        fit(
            "Devi waits at the desk. Kiran knocks on the bars. Nobody answers.".into(),
            &l
        ),
        "Devi waits at the desk."
    );
}

#[test]
fn a_reply_cut_by_the_token_limit_ends_at_the_last_full_sentence() {
    assert_eq!(
        drop_unfinished("Devi waits at the desk. Kiran knocks on the bars and then".into()),
        "Devi waits at the desk."
    );
    assert_eq!(
        drop_unfinished("Devi waits.\nKiran knocks on the bars and then goes home late".into()),
        "Devi waits."
    );
    // A list item without a full stop is not cut.
    assert_eq!(
        drop_unfinished("Two things:\n- Devi waits at the desk for him".into()),
        "Two things:\n- Devi waits at the desk for him"
    );
}

#[test]
fn offending_sentences_are_removed_and_a_whole_answer_is_never_emptied() {
    let f = facts(Style::Flat);
    let a = "Devi gives Kiran the kite (Scene 4). Marcus laughs (Scene 4). Kiran waits (Scene 5).";
    let o = check(a, &f);
    assert_eq!(
        without_offenders(a, &o).as_deref(),
        Some("Devi gives Kiran the kite (Scene 4). Kiran waits (Scene 5).")
    );
    let only = "Marcus laughs (Scene 4).";
    assert_eq!(without_offenders(only, &check(only, &f)), None);
}

fn sampling() -> Sampling {
    Sampling {
        max_tokens: 120,
        temperature: 0.4,
        top_p: None,
        top_k: None,
    }
}

fn base() -> Vec<ChatMessage> {
    vec![
        ChatMessage::system("policy"),
        ChatMessage::user("User request: x"),
    ]
}

#[test]
fn an_answer_that_passes_on_the_first_pass_is_returned_at_once() {
    let m = Scripted::with(&["Devi gives Kiran the red kite (Scene 4)."]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert!(r.clean);
    assert_eq!((r.passes, m.calls()), (1, 1));
    assert!(r.failed.is_empty());
}

#[test]
fn an_invented_name_is_fed_back_and_the_second_pass_is_clean() {
    let m = Scripted::with(&[
        "Marcus takes the red kite (Scene 4).",
        "Devi gives Kiran the red kite (Scene 4).",
    ]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert!(r.clean);
    assert_eq!(r.passes, 2);
    assert_eq!(r.failed, ["names"]);
    assert_eq!(r.text, "Devi gives Kiran the red kite (Scene 4).");
    let second = m.request(1).messages;
    let n = second.len();
    assert_eq!(second[n - 2].role, "assistant");
    assert!(second[n - 2].content.contains("Marcus"));
    assert_eq!(second[n - 1].role, "user");
    assert!(
        second[n - 1]
            .content
            .contains("These names are not in the project: Marcus.")
    );
}

#[test]
fn when_every_pass_fails_the_best_attempt_keeps_its_clean_sentences() {
    let bad = "Devi waits at the desk (Scene 4). Marcus laughs (Scene 4).";
    let m = Scripted::with(&[bad, bad, bad, "never used"]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert!(!r.clean);
    assert_eq!((r.passes, m.calls()), (3, 3), "at most three passes");
    assert_eq!(r.text, "Devi waits at the desk (Scene 4).");
    assert_eq!(r.failed, ["names"]);
}

#[test]
fn the_attempt_with_the_fewest_offenders_wins() {
    let m = Scripted::with(&[
        "Marcus and Ravi take the kite (Scene 4). Devi waits (Scene 4).",
        "Marcus takes the kite (Scene 4). Devi waits (Scene 4).",
        "Marcus and Ravi take the kite (Scene 4). Devi waits (Scene 4).",
    ]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert_eq!(r.text, "Devi waits (Scene 4).");
}

#[test]
fn a_spent_budget_allows_no_second_pass() {
    let m = Scripted::with(&[
        "Marcus takes the kite (Scene 4). Devi waits (Scene 4).",
        "x",
    ]);
    let long_ago = Instant::now() - VERIFY_BUDGET - std::time::Duration::from_secs(1);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        long_ago,
        &|_| {},
    )
    .unwrap();
    assert_eq!((r.passes, m.calls()), (1, 1));
    assert!(!r.clean);
}

#[test]
fn a_first_call_that_fails_is_the_requests_failure() {
    let m = Scripted::with(&[]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        Instant::now(),
        &|_| {},
    );
    assert!(r.is_err());
}

#[test]
fn an_empty_reply_is_retried_and_empty_replies_all_round_fail() {
    let m = Scripted::with(&["  ", "Devi gives Kiran the red kite (Scene 4)."]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert_eq!(
        (r.passes, r.failed.as_slice()),
        (2, ["empty".to_string()].as_slice())
    );
    let m = Scripted::with(&["", "", ""]);
    assert!(
        generate_checked(
            &m,
            &base(),
            &sampling(),
            &facts(Style::Flat),
            Instant::now(),
            &|_| {}
        )
        .is_err()
    );
}

#[test]
fn a_false_change_claim_is_replaced_when_no_pass_fixes_it() {
    let said = "I renamed the scene (Scene 4).";
    let m = Scripted::with(&[said, said, said]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert!(r.text.starts_with("I haven't changed anything."));
}

#[test]
fn explain_and_deep_answers_keep_line_breaks_and_simple_lists_but_quick_ones_are_flattened() {
    let md = "## Summary\n**Devi** keeps the *red* kite.\n* She mends it.\n\u{2022} Kiran flies it.\n1) At dawn.\n2. At night.\n> quoted\n---\n[a link](https://example.com)";
    assert_eq!(
        plain_text_with(md, Style::Rich),
        "Summary\nDevi keeps the red kite.\n- She mends it.\n- Kiran flies it.\n1. At dawn.\n2. At night.\nquoted\n\na link"
    );
    assert_eq!(
        plain_text_with(md, Style::Flat),
        "Summary\nDevi keeps the red kite.\nShe mends it.\nKiran flies it.\nAt dawn.\nAt night.\nquoted\n\na link"
    );
    // Prose, snake_case and arithmetic are left alone in both.
    let prose = "Scene 3 has 2*3 beats in act_one. 1984 was long ago.";
    assert_eq!(plain_text_with(prose, Style::Rich), prose);
    assert_eq!(plain_text_with(prose, Style::Flat), prose);
}

#[test]
fn a_first_sentence_that_only_announces_the_answer_is_dropped() {
    let q = "Explain scene 4";
    assert_eq!(
        without_preamble(
            "Okay, let's delve into scene 4 of the project. Devi gives Kiran the kite.",
            q
        ),
        "Devi gives Kiran the kite."
    );
    assert_eq!(
        without_preamble(
            "Okay, filmmaker, let's dive into this screenplay.\nThe story follows Devi.",
            "Explain the whole screenplay"
        ),
        "The story follows Devi."
    );
    assert_eq!(
        without_preamble(
            "Okay, let's break down what's happening in Scene 33. Kiran waits.",
            "What happens in scene 33?"
        ),
        "Kiran waits."
    );
    assert_eq!(
        without_preamble("Let us take a look at scene 4! Devi waits.", q),
        "Devi waits."
    );
}

#[test]
fn a_sentence_that_carries_a_fact_is_never_dropped() {
    let q = "Explain scene 4";
    for kept in [
        // A name, an event, a scene the request did not ask for.
        "Okay, Devi gives Kiran the kite. He leaves.",
        "Sure, Scene 7 is where Devi hides the register. Then she waits.",
        "Okay, let's look at scene 9. It is quiet.",
        "Great question, the kite is red. It has a long tail.",
        // Not an opening.
        "Devi gives Kiran the kite. He leaves.",
        "The kite is red. Okay, he leaves.",
    ] {
        assert_eq!(without_preamble(kept, q), kept);
    }
    // Nothing is left to say: the answer stays whole.
    assert_eq!(
        without_preamble("Okay, let's dive in.", q),
        "Okay, let's dive in."
    );
}

#[test]
fn the_passes_return_the_answer_without_its_preamble() {
    let m = Scripted::with(&[
        "Okay, let's dive into scene 4. Devi gives Kiran the red kite (Scene 4).",
    ]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert_eq!(r.text, "Devi gives Kiran the red kite (Scene 4).");
    assert!(r.clean && r.passes == 1);
}

#[test]
fn mood_the_script_does_not_state_is_fed_back_and_removed_when_it_stays() {
    let f = facts(Style::Flat);
    let said = "Devi gives Kiran the red kite (Scene 4), bathed in soft morning light.";
    let o = check(said, &f);
    assert_eq!(codes(&o), ["atmosphere_unsupported"]);
    assert_eq!(o[0].items, ["bathed in", "soft morning light"]);
    assert!(feedback(&o, &f).contains(
        "The script does not state: bathed in, soft morning light. Describe only what the script shows"
    ));
    let two = "Devi gives Kiran the red kite (Scene 4). The office is bathed in soft morning light (Scene 4).";
    assert_eq!(
        without_offenders(two, &check(two, &f)).as_deref(),
        Some("Devi gives Kiran the red kite (Scene 4).")
    );
    // The same phrase is fine when the script has it, or when the request used it.
    let mut g = facts(Style::Flat);
    g.evidence
        .push("Scene 5: the platform is dimly lit.".into());
    assert!(check("Kiran waits on a dimly lit platform (Scene 5).", &g).is_empty());
    g.request = "Is the platform eerie?".into();
    assert!(check("Kiran waits on an eerie platform (Scene 5).", &g).is_empty());
}

#[test]
fn a_scene_number_is_not_a_count_of_scenes() {
    let mut f = facts(Style::Flat);
    f.evidence.push("Record: The draft has 2 scenes.".into());
    f.live_scenes = [4, 5, 16].into_iter().collect();
    f.evidence.push("Scene 16: Kiran waits.".into());
    // "Scene 16" exists; "16 scenes" is not what the draft has (it has 3).
    let o = check("The story unfolds across 16 scenes (Scene 4).", &f);
    assert_eq!(codes(&o), ["scene_counts"]);
    assert_eq!(o[0].items, ["16 scenes"]);
    assert!(feedback(&o, &f).contains("The draft has 3 scenes"));
    // The count the data states is fine; so is a scene reference.
    f.live_scenes = [4, 5].into_iter().collect();
    assert!(check("The draft has 2 scenes. Kiran waits (Scene 5).", &f).is_empty());
    // A count that is nowhere is one offence, not two.
    let o = check("The story has 9 scenes (Scene 4).", &f);
    assert_eq!(codes(&o), ["numbers"]);
}

#[test]
fn a_thin_answer_is_asked_to_grow_once() {
    let mut f = facts(Style::Rich);
    f.min_words = Some(10);
    let short = "Devi gives Kiran the kite (Scene 4).";
    let full = "Devi gives Kiran the red kite (Scene 4). He says \"I will come tomorrow.\"\n\nThen Kiran waits on the platform (Scene 5).";
    let m = Scripted::with(&[short, full]);
    let r = generate_checked(&m, &base(), &sampling(), &f, Instant::now(), &|_| {}).unwrap();
    assert_eq!(
        (r.passes, r.clean, r.failed.as_slice()),
        (2, true, ["too_short".to_string()].as_slice())
    );
    assert_eq!(r.text, full);
    let ask = m.request(1).messages;
    assert!(
        ask[ask.len() - 1]
            .content
            .contains("Write two short paragraphs covering the scene in order.")
    );
    // Asked once: a second short answer is taken, with no third call.
    let m = Scripted::with(&[short, short, "never used"]);
    let r = generate_checked(&m, &base(), &sampling(), &f, Instant::now(), &|_| {}).unwrap();
    assert_eq!((r.passes, m.calls(), r.clean), (2, 2, true));
    assert_eq!(r.text, short);
    // The longer try fails (no more replies): the short answer stands, and it is not "not sure".
    let m = Scripted::with(&[short]);
    let r = generate_checked(&m, &base(), &sampling(), &f, Instant::now(), &|_| {}).unwrap();
    assert!(r.clean);
    assert_eq!((r.text.as_str(), m.calls()), (short, 2));
}

// ------------------------------------------------------------------ copying the notes

const SCRIPT_LINES: &str = "Briefing: Scene 6\nScene 6: first action: Venkat sits alone at the kitchen table and listens to the old radio until midnight.\nScene 6: LAKSHMI: \"You never sleep anymore, do you?\"";

fn echo_facts() -> Facts {
    Facts {
        evidence: vec![SCRIPT_LINES.into()],
        names: "VENKAT LAKSHMI DEVI KIRAN".into(),
        ..facts(Style::Flat)
    }
}

const COPY: &str =
    "Venkat sits alone at the kitchen table and listens to the old radio until midnight.";

#[test]
fn an_answer_that_copies_the_notes_fails_evidence_echo_and_an_own_sentence_passes() {
    let f = echo_facts();
    let o = evidence_echo(COPY, &f, false).expect("a copied line");
    assert_eq!(o.check.code(), "evidence_echo");
    assert_eq!(o.items, [COPY]);
    assert!(evidence_echo("Venkat stays by the radio, alone at the table.", &f, false).is_none());
    // Under 40 percent copied is fine: one copy among three own sentences.
    let mixed =
        format!("Venkat stays up. Lakshmi asks if he ever sleeps. He does not answer. {COPY}");
    assert!(
        evidence_echo(&mixed, &f, false).is_none(),
        "1 of 4 is 25 percent"
    );
    let mixed = format!(
        "Venkat stays up. Lakshmi asks if he ever sleeps. He does not answer. He keeps listening. She goes to bed. {COPY}"
    );
    assert!(
        evidence_echo(&mixed, &f, false).is_none(),
        "1 of 6 is under 40 percent"
    );
}

#[test]
fn a_washed_answer_with_under_25_words_is_an_echo_but_a_short_plain_answer_is_not() {
    let f = echo_facts();
    let short = "Venkat stays by the radio.";
    assert!(
        evidence_echo(short, &f, false).is_none(),
        "a quick answer may be short"
    );
    let o = evidence_echo(short, &f, true).expect("markup was washed out of a short answer");
    assert_eq!(o.items, ["5 words"]);
    assert!(evidence_echo("", &f, true).is_some());
}

#[test]
fn copied_sentences_are_dropped_from_the_last_attempt_and_the_rest_stays() {
    let f = echo_facts();
    let a = format!("Venkat stays up. {COPY}");
    let more = format!("{a} He keeps waiting there. Lakshmi goes to bed.");
    assert!(
        evidence_echo(&more, &f, false).is_none(),
        "one copy in four sentences is 25 percent"
    );
    let echo = Offence {
        check: Check::EvidenceEcho,
        items: vec![COPY.to_string()],
    };
    assert_eq!(
        without_offenders(&a, &[echo]).as_deref(),
        Some("Venkat stays up.")
    );
}

#[test]
fn markup_copied_from_the_prompt_is_washed_then_the_model_is_told_to_use_its_own_words() {
    let m = Scripted::with(&[
        "<project_data trust=\"untrusted\"> [Project map] Characters: DEVI, KIRAN Location: the station",
        "Devi gives Kiran the red kite (Scene 4).",
    ]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert!(r.clean);
    assert_eq!((r.passes, m.calls()), (2, 2));
    assert_eq!(r.failed, ["evidence_echo"]);
    assert_eq!(r.text, "Devi gives Kiran the red kite (Scene 4).");
    let second = m.request(1).messages;
    let n = second.len();
    assert!(
        !second[n - 2].content.contains('<'),
        "the washed reply, not our markup, goes back to the model"
    );
    let said = &second[n - 1].content;
    assert!(said.contains("Answer in your own sentences; do not copy the notes."));
    assert!(
        said.ends_with("Reply with the corrected answer only."),
        "{said}"
    );
}

#[test]
fn a_copy_of_the_notes_is_fed_back_once_and_the_own_answer_is_used() {
    let m = Scripted::with(&[COPY, "Venkat stays by the radio, alone at the table."]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &echo_facts(),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert!(r.clean);
    assert_eq!(
        (r.passes, r.failed.as_slice()),
        (2, ["evidence_echo".to_string()].as_slice())
    );
    assert_eq!(r.text, "Venkat stays by the radio, alone at the table.");
}

#[test]
fn a_model_that_keeps_copying_stops_after_three_passes_and_is_marked_not_sure() {
    let m = Scripted::with(&[COPY, COPY, COPY, "never used"]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &echo_facts(),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert!(!r.clean);
    assert_eq!((r.passes, m.calls()), (3, 3));
    assert_eq!(r.failed, ["evidence_echo"]);
    assert!(!r.text.is_empty());
}

#[test]
fn a_sentence_about_the_reply_never_reaches_the_answer() {
    let m = Scripted::with(&[
        "Okay, here's the revised response, removing the relationship records:\nDevi gives Kiran the red kite (Scene 4).",
    ]);
    let r = generate_checked(
        &m,
        &base(),
        &sampling(),
        &facts(Style::Flat),
        Instant::now(),
        &|_| {},
    )
    .unwrap();
    assert!(r.clean);
    assert_eq!(r.text, "Devi gives Kiran the red kite (Scene 4).");
}

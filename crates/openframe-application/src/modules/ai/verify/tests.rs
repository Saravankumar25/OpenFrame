use std::time::Instant;

use super::super::orchestrator::Style;
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
    assert!(m.ends_with("Write the answer again."));
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

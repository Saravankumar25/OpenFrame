use std::cell::RefCell;

use super::super::test_script::{Scripted, project, sample};
use super::*;

fn short() -> (Connection, DraftRef) {
    let c = project();
    let d = sample(&c, "example-short");
    (c, d)
}

const NOTE_ONE: &str =
    "Devi keeps the lost property office and Kiran asks for his kite (Scenes 1, 2, 3, 4 and 5).";
const NOTE_TWO: &str = "Devi and Kiran meet again before dawn (Scenes 6, 7, 8, 9 and 10).";
const ANSWER: &str = "Devi runs the lost property office and Kiran keeps coming back for a red kite (Scene 1). They part at dawn (Scene 10).";

struct Run {
    answer: LaneAnswer,
    model: Scripted,
    progress: Vec<(u32, u32, String)>,
}

fn run_with(replies: &[&str], cap: Duration, cancelled: &dyn Fn() -> bool) -> Run {
    let (c, d) = short();
    let material = gather(&c, &d).unwrap();
    let model = Scripted::with(replies);
    let seen = RefCell::new(Vec::new());
    let progress = |step: u32, total: u32, label: &str| {
        seen.borrow_mut().push((step, total, label.to_string()))
    };
    let cx = Cx {
        draft: &d,
        model: &model,
        text: "Explain the whole screenplay",
        started: Instant::now(),
        cap,
        progress: &progress,
        cancelled,
    };
    let answer = write(&cx, &material).unwrap();
    let progress = seen.into_inner();
    Run {
        answer,
        model,
        progress,
    }
}

fn used_and_total(o: &ToolOutput) -> (u64, u64) {
    let c = &o.structured.as_ref().unwrap()["coverage"];
    (c["used"].as_u64().unwrap(), c["total"].as_u64().unwrap())
}

#[test]
fn groups_of_about_eight_scenes_without_acts() {
    let live: Vec<usize> = (1..=10).collect();
    let g = plan(&[], &live);
    assert_eq!(g.iter().map(|g| g.scenes.len()).collect::<Vec<_>>(), [5, 5]);
    assert_eq!(g[0].short, "Scenes 1 to 5");
    assert_eq!(plan(&[], &(1..=8).collect::<Vec<_>>()).len(), 1);
    assert_eq!(
        plan(&[], &(1..=36).collect::<Vec<_>>()).len(),
        5,
        "36 scenes, five groups"
    );
    assert!(plan(&[], &[]).is_empty());
}

#[test]
fn groups_follow_the_acts_and_split_a_long_act() {
    let acts = vec![
        ("Act 1: Setup".to_string(), (1..=4).collect::<Vec<_>>()),
        ("Act 2: Chase".to_string(), (5..=20).collect()),
        ("Scenes not placed in an act".to_string(), vec![21]),
    ];
    let g = plan(&acts, &(1..=21).collect::<Vec<_>>());
    assert_eq!(g[0].short, "Act 1 of 3");
    assert_eq!(g[0].label, "Act 1: Setup");
    assert_eq!(g[1].short, "Scenes 5 to 12");
    assert_eq!(g[2].short, "Scenes 13 to 20");
    assert_eq!(g[3].short, "Scenes not placed in an act");
    assert_eq!(g.iter().map(|g| g.scenes.len()).sum::<usize>(), 21);
}

#[test]
fn the_whole_of_example_short_is_read_in_two_groups_and_reported_as_covered() {
    let r = run_with(&[NOTE_ONE, NOTE_TWO, ANSWER], RECIPE_CAP, &|| false);
    assert_eq!(r.model.calls(), 3, "two group notes and one compose");
    assert_eq!(used_and_total(&r.answer.output), (10, 10));
    assert!(
        r.answer.output.details.is_empty(),
        "{:?}",
        r.answer.output.details
    );
    assert_eq!(r.answer.output.content, ANSWER);
    assert_eq!(r.answer.passes, 1);
    assert!(r.answer.failed.is_empty());

    // The notes are written cold and short; the answer is warmer and longer.
    let (n, a) = (r.model.request(0), r.model.request(2));
    assert_eq!((n.temperature, n.max_tokens), (0.2, 150));
    assert_eq!(
        (a.temperature, a.max_tokens, a.top_p),
        (0.6, 700, Some(0.95))
    );
    // The first group's prompt holds its own scenes only.
    let first = &n.messages[1].content;
    assert!(
        first.contains("Scene 5") && !first.contains("Scene 6"),
        "{first}"
    );
    // The compose prompt holds the project map and both notes.
    let compose = &a.messages;
    assert!(compose[0].content.contains("Project map"));
    assert!(compose[1].content.contains("Scenes 1, 2, 3, 4 and 5"));
    assert!(compose[1].content.contains("Scenes 6, 7, 8, 9 and 10"));
}

#[test]
fn a_scene_no_note_names_is_reported_as_not_covered() {
    let note_two = "Devi and Kiran meet again before dawn (Scenes 6, 7, 8 and 10).";
    let r = run_with(&[NOTE_ONE, note_two, ANSWER], RECIPE_CAP, &|| false);
    assert_eq!(used_and_total(&r.answer.output), (9, 10));
    assert_eq!(
        r.answer.output.details,
        ["Used 9 of 10 scenes; not covered: Scene 9."]
    );
}

#[test]
fn progress_lines_come_in_order_and_end_with_checking() {
    let r = run_with(&[NOTE_ONE, NOTE_TWO, ANSWER], RECIPE_CAP, &|| false);
    let labels: Vec<&str> = r.progress.iter().map(|p| p.2.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Reading Scenes 1 to 5",
            "Reading Scenes 6 to 10",
            "Writing the answer",
            "Checking the answer"
        ]
    );
    let steps: Vec<(u32, u32)> = r.progress.iter().map(|p| (p.0, p.1)).collect();
    assert_eq!(steps, [(1, 3), (2, 3), (3, 3), (3, 3)]);
}

#[test]
fn a_cancel_between_calls_stops_the_recipe_with_an_honest_line() {
    let calls = std::cell::Cell::new(0);
    let stop_after_first_note = || {
        calls.set(calls.get() + 1);
        calls.get() > 1
    };
    let r = run_with(
        &[NOTE_ONE, NOTE_TWO, ANSWER],
        RECIPE_CAP,
        &stop_after_first_note,
    );
    assert_eq!(r.model.calls(), 1);
    assert_eq!(used_and_total(&r.answer.output), (5, 10));
    assert!(
        r.answer
            .output
            .content
            .starts_with("Stopped. I had read 5 of 10 scenes.")
    );
}

#[test]
fn a_spent_time_cap_answers_from_the_notes_so_far() {
    // The first group is always read; the second would not leave time for the answer.
    let r = run_with(&[NOTE_ONE, ANSWER], Duration::ZERO, &|| false);
    assert_eq!(r.model.calls(), 2, "one note and the answer");
    assert_eq!(used_and_total(&r.answer.output), (5, 10));
    assert_eq!(
        r.answer.output.details,
        ["Used 5 of 10 scenes; not covered: Scenes 6 to 10."]
    );
}

#[test]
fn a_note_that_invents_a_name_loses_that_sentence() {
    let bad = "Devi keeps the office (Scenes 1, 2, 3 and 4). Marcus Webb helps her (Scene 5).";
    let r = run_with(&[bad, NOTE_TWO, ANSWER], RECIPE_CAP, &|| false);
    let compose = &r.model.request(2).messages[1].content;
    assert!(compose.contains("Devi keeps the office"), "{compose}");
    assert!(!compose.contains("Marcus"), "{compose}");
    assert_eq!(
        used_and_total(&r.answer.output),
        (9, 10),
        "scene 5 was in the cut sentence"
    );
}

#[test]
fn the_answer_goes_through_the_verifiers_like_a_chat_answer() {
    let invented = "Devi and Marcus run the office (Scene 1).";
    let r = run_with(&[NOTE_ONE, NOTE_TWO, invented, ANSWER], RECIPE_CAP, &|| {
        false
    });
    assert_eq!(r.answer.passes, 2);
    assert_eq!(r.answer.failed, ["names"]);
    assert_eq!(r.answer.output.content, ANSWER);
    // A cited scene becomes a source chip that opens it.
    let chips: Vec<&str> = r
        .answer
        .output
        .provenance
        .iter()
        .map(|p| p.label.as_str())
        .collect();
    assert!(chips.iter().any(|l| l.starts_with("Scene 1:")), "{chips:?}");
    assert!(r.answer.output.provenance.iter().all(|p| p.nav.is_some()));
}

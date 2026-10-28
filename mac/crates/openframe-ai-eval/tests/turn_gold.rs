//! The answer-type stage (spec `ai-conversational-harness`, slice S4) on the real fixtures, with the
//! rules alone: no embedding model, no engine. The measured run with nearest neighbours and the
//! model is `ai-eval --turn-only`; this file keeps what must hold without them.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use openframe_ai_eval::turn::{Conv, from_cases, from_sets, index_of, read_conv};
use openframe_ai_eval::{fixture, load_cases};
use openframe_application::core::{AppConfig, AppCore};
use openframe_application::events::RecordingSink;
use openframe_application::modules::ai::turn::{Aids, AnswerType, ReadHow};

const EXAMPLES: &str =
    include_str!("../../openframe-application/src/modules/ai/turn_examples.jsonl");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn core() -> (tempfile::TempDir, Arc<AppCore>) {
    let dir = tempfile::tempdir().unwrap();
    let core = AppCore::new(
        AppConfig::isolated(dir.path()),
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    (dir, core)
}

fn convs() -> Vec<Conv> {
    let cases = load_cases(&root().join("gold")).unwrap();
    let mut all = from_cases(&cases);
    all.extend(from_sets(&root().join("turn-sets")).unwrap());
    all.retain(|c| c.project == fixture::LOW_TIDE);
    all
}

fn words(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn no_example_is_a_gold_request_or_a_spike_message() {
    let mut seen = BTreeSet::new();
    for c in load_cases(&root().join("gold")).unwrap() {
        seen.insert(words(&c.request));
        seen.extend(c.before.iter().map(|b| words(b)));
    }
    for conv in from_sets(&root().join("turn-sets")).unwrap() {
        seen.extend(conv.turns.iter().map(|t| words(t)));
    }
    assert!(
        seen.len() > 300,
        "the gold sets did not load: {}",
        seen.len()
    );
    let mut n = 0;
    for line in EXAMPLES.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        let text = v["m"].as_str().unwrap();
        assert!(
            !seen.contains(&words(text)),
            "example copied from a gold or spike message: {text}"
        );
        n += 1;
    }
    assert_eq!(n, 275);
}

/// Read every measured turn with the rules alone: (labelled type, read type, how, message).
fn rule_reads() -> Vec<(AnswerType, AnswerType, ReadHow, String)> {
    let (_dir, core) = core();
    let ix = index_of(&core, fixture::LOW_TIDE).unwrap();
    let mut out = Vec::new();
    for conv in convs() {
        let reads = read_conv(&ix, &conv, &Aids::none());
        for m in &conv.measured {
            if let Some(want) = m.want {
                let r = &reads[m.turn];
                out.push((want, r.answer_type, r.how, conv.turns[m.turn].clone()));
            }
        }
    }
    out
}

#[test]
fn rules_alone_never_read_anger_or_small_talk_as_a_change_a_count_or_a_fetch() {
    let reads = rule_reads();
    assert!(reads.len() > 250, "{}", reads.len());
    let chat: Vec<_> = reads.iter().filter(|r| r.0 == AnswerType::Chat).collect();
    assert!(chat.len() >= 20, "{}", chat.len());
    for (_, got, how, msg) in &chat {
        let decided = matches!(how, ReadHow::Rule | ReadHow::Repair);
        let bad = matches!(
            got,
            AnswerType::Change
                | AnswerType::Count
                | AnswerType::Lookup
                | AnswerType::List
                | AnswerType::Compare
        );
        assert!(!(decided && bad), "{got:?} by {how:?}: {msg}");
    }
}

#[test]
fn what_the_rules_decide_they_decide_right_most_of_the_time() {
    let reads = rule_reads();
    let decided: Vec<_> = reads
        .iter()
        .filter(|r| matches!(r.2, ReadHow::Rule | ReadHow::Repair))
        .collect();
    let right = decided.iter().filter(|r| r.0 == r.1).count();
    let coverage = decided.len() as f64 / reads.len() as f64;
    let precision = right as f64 / decided.len() as f64;
    eprintln!(
        "rules decided {} of {} ({:.0}%), right {} ({:.1}%)",
        decided.len(),
        reads.len(),
        coverage * 100.0,
        right,
        precision * 100.0
    );
    assert!(precision >= 0.85, "precision {precision:.3}");
    assert!(coverage >= 0.35, "coverage {coverage:.3}");
}

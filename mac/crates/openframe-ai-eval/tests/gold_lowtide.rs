//! The whole gold folder loads, and the low-tide conversation bench is well formed: every case
//! has the fields the scorer needs, and no `required_facts` entry is invented (each one appears
//! in `FACTS.md`, a Fountain file or `setup.jsonl` of the fixture).

use std::path::PathBuf;

use openframe_ai_eval::{fixture, load_cases};

const SAMPLES: &[&str] = &[
    "example-feature",
    "example-short",
    "example-scene",
    "fdx-interchange-test",
];
const LENGTH_CLASSES: &[&str] = &["one_sentence", "short", "paragraph", "brief"];
const ANSWER_TYPES: &[&str] = &[
    "lookup",
    "list",
    "count",
    "explain",
    "relation",
    "compare",
    "brainstorm",
    "navigate",
    "change",
    "help",
    "chat",
];

fn gold() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gold")
}

#[test]
fn gold_folder_loads_and_the_lowtide_bench_is_well_formed() {
    // load_cases fails on a malformed line and on a duplicate id.
    let cases = load_cases(&gold()).unwrap();

    for c in &cases {
        assert!(
            SAMPLES.contains(&c.project.as_str()) || fixture::is_fixture(&c.project),
            "{}: unknown project {}",
            c.id,
            c.project
        );
    }

    let lt: Vec<_> = cases
        .iter()
        .filter(|c| c.id.starts_with("conv-lt-"))
        .collect();
    assert!(lt.len() >= 200, "only {} conv-lt cases", lt.len());
    assert_eq!(
        lt.iter()
            .filter(|c| c.id.starts_with("conv-lt-export-"))
            .count(),
        40
    );

    for c in &lt {
        let e = &c.expect;
        assert_eq!(c.kind, "conversation", "{}", c.id);
        assert!(fixture::is_fixture(&c.project), "{}", c.id);
        let lc = e
            .length_class
            .as_deref()
            .unwrap_or_else(|| panic!("{}: no length_class", c.id));
        assert!(LENGTH_CLASSES.contains(&lc), "{}: length_class {lc}", c.id);
        let at = e
            .answer_type
            .as_deref()
            .unwrap_or_else(|| panic!("{}: no answer_type", c.id));
        assert!(ANSWER_TYPES.contains(&at), "{}: answer_type {at}", c.id);
        assert!(e.no_internal_words, "{}: no_internal_words", c.id);
        assert!(c.before.len() <= 6, "{}: before is capped at 6", c.id);
        if e.abstain || !e.abstain_names.is_empty() {
            assert!(
                e.abstain && !e.abstain_names.is_empty(),
                "{}: abstain needs abstain_names",
                c.id
            );
            assert!(
                e.answer_has.is_empty(),
                "{}: abstain case has answer_has",
                c.id
            );
        }
        if !e.no_dump.is_empty() {
            assert!(
                (2..=3).contains(&e.no_dump.len()),
                "{}: no_dump wants 2 or 3 strings",
                c.id
            );
            assert!(e.only_read_tools && e.no_proposal, "{}", c.id);
        }
        if c.lang != "en" {
            assert!(
                c.review.is_some(),
                "{}: non-English needs a review note",
                c.id
            );
        }
    }

    let tools: Vec<_> = cases
        .iter()
        .filter(|c| c.id.starts_with("tool-lt-"))
        .collect();
    assert!(tools.len() >= 20, "only {} tool-lt cases", tools.len());
    for c in &tools {
        assert!(c.expect.tool.is_some(), "{}: no tool", c.id);
    }
}

#[test]
fn lowtide_required_facts_come_from_the_fixture_files() {
    let dir = fixture::fixtures_dir().join(fixture::LOW_TIDE);
    let corpus: String = [
        "FACTS.md",
        "feature.fountain",
        "teaser.fountain",
        "setup.jsonl",
    ]
    .iter()
    .map(|f| std::fs::read_to_string(dir.join(f)).unwrap())
    .collect::<Vec<_>>()
    .join("\n")
    .to_lowercase();

    let mut checked = 0;
    for c in load_cases(&gold()).unwrap() {
        if !(c.id.contains("-lt-") && fixture::is_fixture(&c.project)) {
            continue;
        }
        for f in &c.expect.required_facts {
            checked += 1;
            assert!(
                corpus.contains(&f.to_lowercase()),
                "{}: required fact {f:?} is not in the fixture",
                c.id
            );
        }
    }
    assert!(checked > 150, "only {checked} required facts were checked");
}

#[test]
fn new_gold_files_use_no_em_dash() {
    for f in [
        "conversation.lowtide.en.jsonl",
        "tools.lowtide.en.jsonl",
        "README.md",
    ] {
        let text = std::fs::read_to_string(gold().join(f)).unwrap();
        assert!(!text.contains('\u{2014}'), "{f} holds an em dash");
    }
}

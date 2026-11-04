//! The section-scoped gold set (`gold/sections.lowtide.en.jsonl`, spec
//! `docs/specs/ai-section-scoped-assistant.md` sections 6 and 12) is well formed and its ground
//! truth is sound: ids, leaves, picks, pointers, types, the sample, and every expected value is in
//! the fixture it runs on (or in the product guide for the help leaves).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde_json::Value;
use straft_ai_eval::{fixture, load_cases};

/// The leaf ids of spec section 3.1, as the registry will spell them.
const LEAVES: &[&str] = &[
    "idea-vault/this-project",
    "idea-vault/global",
    "story/story-page",
    "story/beat-board",
    "story/outline",
    "story/characters",
    "story/timeline",
    "story/episodes",
    "screenplay/scenes",
    "breakdown/breakdown",
    "production/overview",
    "production/catalog",
    "production/locations",
    "production/cast-crew",
    "production/moodboards",
    "production/storyboards",
    "production/shot-lists",
    "production/schedule",
    "production/day-out-of-days",
    "production/daily-view",
    "production/sides",
    "production/reports",
    "production/budget",
    "call-sheets/call-sheets",
    "files/files",
    "notes-tasks/notes",
    "notes-tasks/tasks",
    "help/product-guide",
    "help/settings",
];
/// Leaves whose data only `low-tide-sections` holds.
const NEW_DATA: &[&str] = &["story/timeline", "idea-vault/global"];
/// Leaves with no cases of their own, and why (README).
const NO_OWN_CASES: &[&str] = &["story/story-page", "story/episodes"];
const FAMILIES: &[&str] = &["pair", "triple", "neg", "conv", "turn"];
const OLD_TYPES: &[&str] = &[
    "lookup",
    "list",
    "count",
    "explain",
    "relation",
    "compare",
    "brainstorm",
    "navigate",
    "help",
];
const OLD_CLASSES: &[&str] = &["one_sentence", "short", "paragraph", "brief"];
const NEW_TYPES: &[&str] = &["describe", "brief", "story_explanation", "relation"];
const NEW_CLASSES: &[(&str, u64)] = &[("long250", 250), ("long350", 350)];

fn gold() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gold")
}

fn slug(leaf: &str) -> String {
    leaf.replace('/', "-")
}

fn read_cases() -> Vec<Value> {
    let text = std::fs::read_to_string(gold().join("sections.lowtide.en.jsonl")).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, l)| serde_json::from_str(l).unwrap_or_else(|e| panic!("line {}: {e}", i + 1)))
        .collect()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn leaves_of(picks: &Value) -> Vec<String> {
    picks
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| p["leaf"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The text a case's expected values must come from: the documents and files of the fixture it
/// runs on, and the product guide when a help leaf is picked.
fn corpus(project: &str, help: bool) -> String {
    let dir = fixture::fixtures_dir();
    let mut files = vec![
        dir.join(fixture::LOW_TIDE).join("FACTS.md"),
        dir.join(fixture::LOW_TIDE).join("feature.fountain"),
        dir.join(fixture::LOW_TIDE).join("teaser.fountain"),
        dir.join(fixture::LOW_TIDE).join("setup.jsonl"),
    ];
    if project == fixture::LOW_TIDE_SECTIONS {
        files.push(dir.join(fixture::LOW_TIDE_SECTIONS).join("FACTS.md"));
        files.push(dir.join(fixture::LOW_TIDE_SECTIONS).join("extra.jsonl"));
    }
    if help {
        files.push(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../straft-application/src/modules/ai/knowledge.md"),
        );
    }
    files
        .iter()
        .map(|f| std::fs::read_to_string(f).unwrap_or_else(|e| panic!("{}: {e}", f.display())))
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase()
}

#[test]
fn sections_gold_is_sound() {
    // The whole gold folder still loads through the first harness, ids unique across all files.
    let loaded = load_cases(&gold()).unwrap();
    let cases = read_cases();
    assert!(cases.len() >= 600, "only {} section cases", cases.len());
    assert_eq!(
        loaded.iter().filter(|c| c.id.starts_with("sec-")).count(),
        cases.len()
    );

    let slugs: BTreeMap<String, &str> = LEAVES.iter().map(|l| (slug(l), *l)).collect();
    let mut ids = BTreeSet::new();
    let mut own: BTreeMap<&str, usize> = LEAVES.iter().map(|l| (*l, 0)).collect();
    let mut in_sample: BTreeMap<&str, usize> = LEAVES.iter().map(|l| (*l, 0)).collect();
    let mut negs: BTreeMap<&str, usize> = LEAVES.iter().map(|l| (*l, 0)).collect();
    let mut by_family: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut sample = 0;
    let mut checked = 0;

    for c in &cases {
        let id = c["id"].as_str().expect("id");
        assert!(ids.insert(id.to_string()), "duplicate id {id}");

        // ---- id: sec-<s|x>-<leaf slug or family>-NNN
        let rest = id
            .strip_prefix("sec-")
            .unwrap_or_else(|| panic!("{id}: not sec-"));
        let (mark, rest) = rest
            .split_once('-')
            .unwrap_or_else(|| panic!("{id}: no mark"));
        assert!(mark == "s" || mark == "x", "{id}: mark {mark}");
        let (key, number) = rest
            .rsplit_once('-')
            .unwrap_or_else(|| panic!("{id}: no number"));
        assert!(
            number.len() == 3 && number.bytes().all(|b| b.is_ascii_digit()),
            "{id}: number {number}"
        );
        let family = if FAMILIES.contains(&key) {
            key.to_string()
        } else {
            assert!(
                slugs.contains_key(key),
                "{id}: {key} is neither a family nor a leaf"
            );
            "leaf".to_string()
        };
        let sampled = mark == "s";
        let f = by_family.entry(family.clone()).or_default();
        f.0 += 1;
        f.1 += usize::from(sampled);
        sample += usize::from(sampled);

        // ---- shape
        assert_eq!(c["kind"], "conversation", "{id}");
        assert_eq!(c["lang"], "en", "{id}");
        let project = c["project"].as_str().unwrap();
        assert!(
            project == fixture::LOW_TIDE || project == fixture::LOW_TIDE_SECTIONS,
            "{id}: project {project}"
        );
        assert!(
            c["request"].as_str().is_some_and(|r| !r.trim().is_empty()),
            "{id}"
        );

        // ---- picks: 1 to 3 known leaves, once each; instance choices of the right kind
        let picks = c["sections"]
            .as_array()
            .unwrap_or_else(|| panic!("{id}: no sections"));
        assert!(
            (1..=3).contains(&picks.len()),
            "{id}: {} picks",
            picks.len()
        );
        let picked = leaves_of(&c["sections"]);
        assert_eq!(picked.len(), picks.len(), "{id}");
        let distinct: BTreeSet<_> = picked.iter().collect();
        assert_eq!(distinct.len(), picked.len(), "{id}: a leaf twice");
        for p in picks {
            let leaf = p["leaf"].as_str().unwrap();
            assert!(LEAVES.contains(&leaf), "{id}: unknown leaf {leaf}");
            assert!(
                leaf != "story/episodes",
                "{id}: Episodes cannot be picked on a feature film"
            );
            for (k, v) in p.as_object().unwrap() {
                match k.as_str() {
                    "leaf" => {}
                    "screenplay" => assert!(v.is_string(), "{id}"),
                    "scene" | "day" | "draft" => assert!(v.is_u64() || k == "draft", "{id}: {k}"),
                    other => panic!("{id}: unknown pick field {other}"),
                }
            }
            if leaf == "screenplay/scenes" {
                assert!(
                    p["screenplay"].is_string(),
                    "{id}: scenes needs a screenplay"
                );
            }
        }
        if let Some(before) = c.get("sections_before") {
            assert!(
                !strings(&c["before"]).is_empty(),
                "{id}: sections_before without before"
            );
            for l in leaves_of(before) {
                assert!(LEAVES.contains(&l.as_str()), "{id}: sections_before {l}");
            }
        }
        let before = strings(&c["before"]);
        assert!(before.len() <= 6, "{id}: before is capped at 6");
        if family == "turn" && !before.is_empty() {
            assert_eq!(
                c["expect"]["no_reask"], true,
                "{id}: a follow-up sets no_reask"
            );
        }

        // ---- the fixture the case runs on
        let pointer = c["expect"]["pointer"].as_str();
        let before_leaves = leaves_of(c.get("sections_before").unwrap_or(&Value::Null));
        let touches_new = picked
            .iter()
            .chain(before_leaves.iter())
            .any(|l| NEW_DATA.contains(&l.as_str()))
            || pointer.is_some_and(|p| NEW_DATA.contains(&p));
        let want = if touches_new {
            fixture::LOW_TIDE_SECTIONS
        } else {
            fixture::LOW_TIDE
        };
        assert_eq!(project, want, "{id}: wrong fixture");

        // ---- family rules
        if family == "leaf" {
            let leaf = slugs[key];
            assert_eq!(
                picked,
                vec![leaf.to_string()],
                "{id}: a leaf case picks only its leaf"
            );
            *own.get_mut(leaf).unwrap() += 1;
            *in_sample.get_mut(leaf).unwrap() += usize::from(sampled);
            assert!(
                !NO_OWN_CASES.contains(&leaf),
                "{id}: {leaf} has no data to ask about"
            );
            assert!(pointer.is_none(), "{id}: a leaf case expects an answer");
        }
        if family == "triple" {
            assert_eq!(picks.len(), 3, "{id}");
            assert!(sampled, "{id}: all triples are in the sample");
        }
        if family == "pair" {
            assert_eq!(picks.len(), 2, "{id}");
        }
        let e = &c["expect"];
        if let Some(p) = pointer {
            assert!(LEAVES.contains(&p), "{id}: pointer {p}");
            assert!(
                !picked.iter().any(|l| l == p),
                "{id}: the pointer {p} is among the picks"
            );
        }
        if family == "neg" {
            assert!(
                pointer.is_some() || e["abstain"] == true,
                "{id}: a negative expects a pointer or a could-not-find"
            );
            if picks.len() == 1 && pointer.is_some() {
                *negs.get_mut(picked[0].as_str()).unwrap() += 1;
            }
            if pointer.is_some() {
                assert!(
                    !strings(&e["answer_lacks"]).is_empty() || !e["answer_has"].is_null(),
                    "{id}: a pointer case names facts that must not leak"
                );
            }
        }
        if e["abstain"] == true {
            assert!(
                !strings(&e["abstain_names"]).is_empty(),
                "{id}: abstain needs names"
            );
            assert!(
                e["answer_has"].is_null(),
                "{id}: abstain case has answer_has"
            );
        }
        assert_eq!(e["no_internal_words"], true, "{id}");

        // ---- types and classes: old values load in the first harness, new ones sit in new_ fields
        let at = e["answer_type"].as_str();
        let nat = e["new_answer_type"].as_str();
        let lc = e["length_class"].as_str();
        let nlc = e["new_length_class"].as_str();
        assert!(at.is_some() || nat.is_some(), "{id}: no answer type");
        assert!(lc.is_some() || nlc.is_some(), "{id}: no length class");
        if let Some(t) = at {
            assert!(OLD_TYPES.contains(&t), "{id}: answer_type {t}");
        }
        if let Some(t) = nat {
            assert!(NEW_TYPES.contains(&t), "{id}: new_answer_type {t}");
        }
        if let Some(l) = lc {
            assert!(OLD_CLASSES.contains(&l), "{id}: length_class {l}");
        }
        if let Some(l) = nlc {
            let cap = NEW_CLASSES
                .iter()
                .find(|(n, _)| *n == l)
                .unwrap_or_else(|| panic!("{id}: new_length_class {l}"))
                .1;
            assert_eq!(e["max_words"], cap, "{id}: max_words");
            assert!(e["min_words"].is_u64(), "{id}: min_words");
            assert!(lc.is_none(), "{id}: both a long and an old length class");
            assert_eq!(
                e["no_navigate"], true,
                "{id}: narrative answers do not navigate"
            );
        }
        if nat.is_some_and(|t| t != "relation") {
            assert!(at.is_none(), "{id}: a new type has no old type");
        }

        // ---- ground truth: every expected value is in the fixture text
        let help = picked.iter().any(|l| l.starts_with("help/"));
        let text = corpus(project, help);
        let derived: BTreeSet<String> = strings(&e["derived"])
            .iter()
            .map(|d| d.to_lowercase())
            .collect();
        let mut checks = 0;
        for fact in strings(&e["required_facts"]) {
            checks += 1;
            assert!(
                text.contains(&fact.to_lowercase()) || derived.contains(&fact.to_lowercase()),
                "{id}: required fact {fact:?} is not in the fixture"
            );
        }
        for group in e["answer_has"].as_array().into_iter().flatten() {
            checks += 1;
            let alts = strings(group);
            assert!(!alts.is_empty(), "{id}: an empty answer_has group");
            assert!(
                alts.iter().any(|a| {
                    let a = a.to_lowercase();
                    text.contains(&a) || derived.contains(&a)
                }),
                "{id}: none of {alts:?} is in the fixture"
            );
        }
        // The case must check something: a value, an absence, a pointer or an abstention.
        assert!(
            checks > 0
                || e["abstain"] == true
                || pointer.is_some()
                || !strings(&e["answer_lacks"]).is_empty(),
            "{id}: checks nothing"
        );
        checked += checks;
    }

    // ---- the whole set
    assert!(checked > 700, "only {checked} expected values were checked");
    assert!(
        (280..=320).contains(&sample),
        "the sample is {sample} cases"
    );
    for (leaf, n) in &in_sample {
        assert!(*n <= 6, "{leaf}: {n} cases in the sample, at most 6");
    }
    for leaf in LEAVES {
        if *leaf == "story/episodes" {
            assert_eq!(negs[leaf], 0, "{leaf} cannot be picked");
            continue;
        }
        assert!(
            negs[leaf] >= 3,
            "{leaf}: only {} single-leaf negatives",
            negs[leaf]
        );
        if NO_OWN_CASES.contains(leaf) {
            assert_eq!(own[leaf], 0, "{leaf}");
        } else {
            assert!(
                own[leaf] >= 7,
                "{leaf}: only {} cases of its own",
                own[leaf]
            );
            assert!(
                in_sample[leaf] >= 6,
                "{leaf}: only {} in the sample",
                in_sample[leaf]
            );
        }
    }
    assert_eq!(by_family["triple"].0, 5);
    assert!(by_family["pair"].0 >= 45, "15 pairs, three cases each");
    assert!(
        by_family["conv"].0 >= 24,
        "four conversational types, six each"
    );

    // The example case of spec section 6 is in the sample.
    assert!(
        cases.iter().any(|c| {
            let l = leaves_of(&c["sections"]);
            c["id"].as_str().unwrap().starts_with("sec-s-pair-")
                && l.contains(&"screenplay/scenes".to_string())
                && l.contains(&"production/cast-crew".to_string())
                && c["request"]
                    .as_str()
                    .unwrap()
                    .contains("describe his character")
        }),
        "the Screenplay + Cast & Crew example is missing from the sample"
    );

    // ---- plain words
    for f in ["sections.lowtide.en.jsonl", "README.md"] {
        let text = std::fs::read_to_string(gold().join(f)).unwrap();
        assert!(!text.contains('\u{2014}'), "{f} holds an em dash");
    }
    let text = std::fs::read_to_string(gold().join("sections.lowtide.en.jsonl")).unwrap();
    assert!(!text.contains("sec-sample-"));

    eprintln!(
        "section cases: {} (sample {sample}), expected values checked: {checked}",
        cases.len()
    );
    for (f, (n, s)) in &by_family {
        eprintln!("  {f}: {n} ({s} in the sample)");
    }
    for leaf in LEAVES {
        eprintln!(
            "  {leaf}: {} own, {} in the sample, {} negatives",
            own[leaf], in_sample[leaf], negs[leaf]
        );
    }
}

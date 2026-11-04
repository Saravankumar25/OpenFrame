//! The leaf registry on a project full of data (slice R1, spec `ai-section-scoped-assistant`
//! section 12.1). Every leaf's sources run against `low-tide-sections`; then the gate, with one leaf
//! picked, lets through nothing that another leaf read.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde_json::json;
use straft_ai_eval::fixture;
use straft_application::core::{AppConfig, AppCore};
use straft_application::events::RecordingSink;
use straft_application::modules::ai::dossier::{self, Fact};
use straft_application::modules::ai::index::{self, Focus};
use straft_application::modules::ai::sections::{self, LEAVES};
use straft_application::modules::ai::turn::{self, Aids, TurnInput};

/// Leaves that can answer nothing on this fixture, and why.
const SILENT: &[(&str, &str)] = &[
    (
        "story/story-page",
        "the Story page is not offered to the assistant",
    ),
    (
        "story/episodes",
        "the fixture is a feature film, not a series",
    ),
];

fn core_with_fixture() -> (tempfile::TempDir, Arc<AppCore>) {
    let dir = tempfile::tempdir().unwrap();
    let core = AppCore::new(
        AppConfig::isolated(dir.path()),
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    core.dispatch(
        &core.local_actor(),
        "project.create",
        json!({ "title": "fixture", "projectType": "Feature Film" }),
    )
    .unwrap();
    fixture::build(&core, fixture::LOW_TIDE_SECTIONS).unwrap();
    (dir, core)
}

/// What every leaf reads for a message, in registry order.
fn every_leaf(core: &AppCore, message: &str) -> Vec<(&'static str, Vec<Fact>)> {
    let s = core.project().unwrap();
    let ix = index::project_index(&s).unwrap();
    let resolved = index::resolve(&ix, &Focus::default(), message);
    let read = turn::read(
        &TurnInput {
            message,
            previous_message: None,
            previous_type: None,
            resolved: &resolved,
        },
        &Aids::none(),
    );
    let actor = core.local_actor();
    s.store
        .read(|c| {
            let input = dossier::Input {
                conn: c,
                actor: &actor,
                index: &ix,
                core: Some(core),
                message,
                read: &read,
                resolved: &resolved,
                selected_scene: None,
                focus: None,
                previous_message: None,
                sections: &[],
            };
            Ok(LEAVES
                .iter()
                .map(|l| (l.id, sections::read_leaf(l, &input)))
                .collect())
        })
        .unwrap()
}

fn messages() -> Vec<String> {
    let mut m: Vec<String> = LEAVES.iter().map(|l| l.example.to_string()).collect();
    m.push("list everything and tell me how many there are".into());
    m.push("explain scene 3 and who is in it on day 2".into());
    m
}

#[test]
fn the_registry_has_the_leaves_of_the_gold_set_in_its_order() {
    let ids: Vec<&str> = LEAVES.iter().map(|l| l.id).collect();
    assert_eq!(ids.len(), 29);
    // The same list, in the same order, that the gold set was written against.
    let gold = include_str!("sections_gold.rs");
    let listed = gold
        .split("const LEAVES: &[&str] = &[")
        .nth(1)
        .and_then(|s| s.split("];").next())
        .unwrap();
    let want: Vec<&str> = listed.split('"').skip(1).step_by(2).collect();
    assert_eq!(ids, want);
}

#[test]
fn a_fact_homed_elsewhere_never_passes_the_gate() {
    let (_dir, core) = core_with_fixture();
    for message in messages() {
        let read = every_leaf(&core, &message);
        let all: Vec<Fact> = read.iter().flat_map(|(_, f)| f.clone()).collect();
        assert!(all.len() > 20, "{message}: only {} facts", all.len());
        for pick in LEAVES {
            let mut gated = all.clone();
            sections::gate(&[pick], &mut gated);
            for f in &gated {
                let ok = if sections::SHARED_KINDS.contains(&f.kind) {
                    f.home == Some(pick.id) || f.home.is_none()
                } else {
                    sections::home_of_kind(f.kind).map(|l| l.id) == Some(pick.id)
                };
                assert!(
                    ok,
                    "{message}: with {} picked, a {} fact (read by {:?}) passed: {}",
                    pick.id,
                    f.kind,
                    f.home,
                    f.render()
                );
            }
        }
    }
}

#[test]
fn every_kind_a_leaf_really_writes_has_a_home() {
    let (_dir, core) = core_with_fixture();
    let mut seen = BTreeSet::new();
    for message in messages() {
        for (leaf, facts) in every_leaf(&core, &message) {
            for f in facts {
                seen.insert((leaf, f.kind));
                if !sections::SHARED_KINDS.contains(&f.kind) {
                    assert!(
                        sections::home_of_kind(f.kind).is_some(),
                        "{leaf} wrote a {} fact that no leaf homes",
                        f.kind
                    );
                }
            }
        }
    }
    assert!(seen.len() > 30, "{seen:?}");
}

#[test]
fn each_leaf_reads_something_of_its_own_on_a_full_project() {
    let (_dir, core) = core_with_fixture();
    let mut own: BTreeSet<&str> = BTreeSet::new();
    for message in messages() {
        for (leaf, facts) in every_leaf(&core, &message) {
            let l = sections::leaf(leaf).unwrap();
            let mut mine = facts.clone();
            sections::gate(&[l], &mut mine);
            if mine.iter().any(|f| !f.is_missing()) {
                own.insert(leaf);
            }
        }
    }
    for l in LEAVES {
        if let Some((_, why)) = SILENT.iter().find(|(id, _)| *id == l.id) {
            assert!(!own.contains(l.id), "{} should be silent: {why}", l.id);
        } else {
            assert!(own.contains(l.id), "{} read nothing of its own", l.id);
        }
    }
}

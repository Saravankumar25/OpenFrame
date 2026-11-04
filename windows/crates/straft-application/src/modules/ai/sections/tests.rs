//! The coverage tests of the leaf table (spec section 12.1): a screen, an entity type, a fact kind or
//! an agent tool that maps to zero leaves or to two fails the build.

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::*;
use crate::modules::ai::intelligence::documents::KINDS;
use crate::modules::ai::toolbox;

fn desktop_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src")
}

fn homes<'a>(pred: impl Fn(&Leaf) -> bool + 'a) -> Vec<&'static str> {
    LEAVES.iter().filter(|l| pred(l)).map(|l| l.id).collect()
}

#[test]
fn leaf_ids_are_unique_and_sit_in_their_branch() {
    let mut seen = BTreeSet::new();
    for l in LEAVES {
        assert!(seen.insert(l.id), "{} appears twice", l.id);
        let (branch, name) = l.id.split_once('/').expect("branch/leaf");
        assert_eq!(branch, l.branch, "{}", l.id);
        assert!(
            !name.is_empty() && !l.label.is_empty() && !l.example.is_empty(),
            "{}",
            l.id
        );
        assert!(
            super::branch(l.branch).is_some(),
            "{}: unknown branch",
            l.id
        );
        assert!((1..=5).contains(&l.weight), "{}: weight", l.id);
    }
    for b in BRANCHES {
        assert!(
            LEAVES.iter().any(|l| l.branch == b.id),
            "{} has no leaf",
            b.id
        );
    }
}

#[test]
fn every_search_entity_type_maps_to_exactly_one_leaf() {
    // The types the intelligence index covers, plus the two the project search holds besides.
    let mut types: Vec<&str> = KINDS.iter().map(|k| k.kind).collect();
    types.extend(["template", "product_guide"]);
    for t in types {
        if let Some((_, why)) = NOT_RETRIEVED.iter().find(|(n, _)| *n == t) {
            assert!(
                homes(|l| l.entity_types.contains(&t)).is_empty(),
                "{t}: {why}"
            );
            continue;
        }
        let h = homes(|l| l.entity_types.contains(&t));
        assert_eq!(h.len(), 1, "entity type {t} is homed in {h:?}");
    }
    // A type listed on a leaf must be a real one.
    let known: BTreeSet<&str> = KINDS
        .iter()
        .map(|k| k.kind)
        .chain(["template", "product_guide"])
        .collect();
    for l in LEAVES {
        for t in l.entity_types {
            assert!(known.contains(t), "{}: unknown entity type {t}", l.id);
        }
    }
}

#[test]
fn every_name_kind_the_resolver_finds_is_served_by_a_leaf_and_has_one_home() {
    for k in [
        Kind::Scene,
        Kind::Character,
        Kind::Person,
        Kind::Location,
        Kind::Act,
        Kind::Card,
        Kind::VaultItem,
        Kind::File,
        Kind::BudgetLine,
        Kind::Day,
        Kind::CallSheet,
    ] {
        let h = homes(|l| l.name_kinds.contains(&k));
        assert!(!h.is_empty(), "name kind {k:?} is served by no leaf");
        // The home is the first leaf that serves it: the leaf the pointer names.
        assert_eq!(home_of_name_kind(k).map(|l| l.id), h.first().copied());
    }
}

/// The kinds the fetchers write, read from their source: `Fact::new("Kind", ...)` and the sheet
/// facts' `base("Kind")`. A new kind added to a fetcher fails here until a leaf homes it.
fn kinds_in_sources() -> BTreeSet<String> {
    let sources = [
        include_str!("../dossier/fetch/breakdown.rs"),
        include_str!("../dossier/fetch/budget.rs"),
        include_str!("../dossier/fetch/dayviews.rs"),
        include_str!("../dossier/fetch/files.rs"),
        include_str!("../dossier/fetch/notes.rs"),
        include_str!("../dossier/fetch/overview.rs"),
        include_str!("../dossier/fetch/people.rs"),
        include_str!("../dossier/fetch/product.rs"),
        include_str!("../dossier/fetch/production.rs"),
        include_str!("../dossier/fetch/reports.rs"),
        include_str!("../dossier/fetch/schedule.rs"),
        include_str!("../dossier/fetch/screenplay.rs"),
        include_str!("../dossier/fetch/settings.rs"),
        include_str!("../dossier/fetch/story.rs"),
        include_str!("../dossier/fetch/timeline.rs"),
        include_str!("../dossier/fetch/vault.rs"),
        include_str!("../dossier/fetch/visual.rs"),
    ];
    let mut out = BTreeSet::new();
    for text in sources {
        let text = text.replace("\r\n", "\n");
        for marker in ["Fact::new(", "base("] {
            for part in text.split(marker).skip(1) {
                if let Some(rest) = part.trim_start().strip_prefix('"')
                    && let Some(end) = rest.find('"')
                {
                    out.insert(rest[..end].to_string());
                }
            }
        }
    }
    // Written by relabelling a fact, or through a parameter, not by `Fact::new("...")`.
    out.extend(
        [
            "Outline act",
            "Outline card",
            "Global vault item",
            "Idea Vault item",
        ]
        .into_iter()
        .map(String::from),
    );
    out
}

#[test]
fn every_fact_kind_a_fetcher_can_produce_maps_to_exactly_one_leaf() {
    let kinds = kinds_in_sources();
    assert!(kinds.len() > 40, "the scan found only {kinds:?}");
    for k in &kinds {
        if SHARED_KINDS.contains(&k.as_str()) {
            assert!(
                homes(|l| l.fact_kinds.contains(&k.as_str())).is_empty(),
                "{k} is shared and must not be homed"
            );
            continue;
        }
        let h = homes(|l| l.fact_kinds.contains(&k.as_str()));
        assert_eq!(h.len(), 1, "fact kind {k:?} is homed in {h:?}");
    }
    // And nothing homed that no fetcher writes: a dead entry would hide a typo.
    for l in LEAVES {
        for k in l.fact_kinds {
            assert!(kinds.contains(*k), "{}: no fetcher writes {k:?}", l.id);
        }
    }
}

/// The screens of the app and the leaf each one is. A screen that is not a section says why.
const SCREENS: &[(&str, &str)] = &[
    ("vault", "idea-vault/this-project"),
    ("story/page", "story/story-page"),
    ("story/board", "story/beat-board"),
    ("story/outline", "story/outline"),
    ("story/characters", "story/characters"),
    ("story/timeline", "story/timeline"),
    ("story/episodes", "story/episodes"),
    ("screenplay", "screenplay/scenes"),
    ("breakdown", "breakdown/breakdown"),
    ("production/overview", "production/overview"),
    ("production/catalog", "production/catalog"),
    ("production/locations", "production/locations"),
    ("production/cast-crew", "production/cast-crew"),
    ("production/moodboards", "production/moodboards"),
    ("production/storyboards", "production/storyboards"),
    ("production/shots", "production/shot-lists"),
    ("production/schedule", "production/schedule"),
    ("production/dood", "production/day-out-of-days"),
    ("production/daily", "production/daily-view"),
    ("production/sides", "production/sides"),
    ("production/reports", "production/reports"),
    ("production/budget", "production/budget"),
    ("callsheets", "call-sheets/call-sheets"),
    ("files", "files/files"),
    ("notes", "notes-tasks/notes"),
    ("settings", "help/settings"),
    // Pages with no content of their own to ask about.
    ("home", "-"),
    ("trash", "-"),
    ("activity", "-"),
];

/// Leaves that are no screen of their own: the Global vault is a switch inside the Idea Vault,
/// Tasks sit beside Notes on one page, and the Product Guide is the manual, not a page of the app.
const NOT_A_SCREEN: &[&str] = &[
    "idea-vault/global",
    "notes-tasks/tasks",
    "help/product-guide",
];

#[test]
fn every_navigation_screen_maps_to_exactly_one_leaf() {
    // The table above names every screen; the app's routes must agree with it.
    let stores = std::fs::read_to_string(desktop_src().join("app/stores.ts")).expect("stores.ts");
    let union = stores
        .split("export type WorkspaceId =")
        .nth(1)
        .and_then(|s| s.split(';').next())
        .expect("WorkspaceId");
    let workspaces: BTreeSet<String> = union
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect();
    let table: BTreeSet<&str> = SCREENS
        .iter()
        .map(|(s, _)| s.split('/').next().unwrap())
        .collect();
    for w in &workspaces {
        assert!(
            table.contains(w.as_str()),
            "workspace {w} has no entry in SCREENS"
        );
    }
    for w in &table {
        assert!(
            workspaces.contains(*w),
            "SCREENS names {w}, which is no workspace"
        );
    }
    // Production tabs, from the files that declare them.
    let tabs_dir = desktop_src().join("workspaces/production/tabs");
    let mut tabs = BTreeSet::new();
    for e in std::fs::read_dir(tabs_dir).expect("tabs") {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "tsx") && !p.to_string_lossy().contains(".test.") {
            let text = std::fs::read_to_string(&p).unwrap();
            if let Some(rest) = text.split("export const tab = { id: \"").nth(1) {
                tabs.insert(rest.split('"').next().unwrap().to_string());
            }
        }
    }
    let table_tabs: BTreeSet<String> = SCREENS
        .iter()
        .filter_map(|(s, _)| s.strip_prefix("production/").map(str::to_string))
        .collect();
    assert_eq!(tabs, table_tabs, "Production tabs and SCREENS disagree");
    // Story views.
    let story = std::fs::read_to_string(desktop_src().join("workspaces/story/index.tsx")).unwrap();
    let views = story
        .split("type Tab = ")
        .nth(1)
        .and_then(|s| s.split(';').next())
        .expect("story Tab");
    let story_views: BTreeSet<String> = views
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect();
    let table_views: BTreeSet<String> = SCREENS
        .iter()
        .filter_map(|(s, _)| s.strip_prefix("story/").map(str::to_string))
        .collect();
    assert_eq!(story_views, table_views, "Story views and SCREENS disagree");
    // Each screen is exactly one leaf; each leaf is a screen or listed as not being one.
    let mut screen_leaves = BTreeSet::new();
    for (screen, leaf) in SCREENS {
        if *leaf == "-" {
            continue;
        }
        assert!(
            super::leaf(leaf).is_some(),
            "{screen} -> unknown leaf {leaf}"
        );
        assert!(
            screen_leaves.insert(*leaf),
            "{leaf} is the home of two screens"
        );
    }
    for l in LEAVES {
        assert!(
            screen_leaves.contains(l.id) || NOT_A_SCREEN.contains(&l.id),
            "{} is no screen and is not listed as one that is not",
            l.id
        );
    }
}

#[test]
fn every_agent_tool_is_homed_in_a_leaf_or_listed_as_cross_section_or_loop_control() {
    let mut tools: BTreeSet<&str> = toolbox::coverage().into_iter().map(|(t, _, _)| t).collect();
    tools.extend(LOOP_TOOLS.iter().copied());
    for t in &tools {
        let homes = tools::homes_of_tool(t);
        let cross =
            CROSS_TOOLS.contains(t) || LOOP_TOOLS.contains(t) || UNSECTIONED_TOOLS.contains(t);
        match (homes, cross) {
            (1, false) | (0, true) => {}
            _ => panic!("tool {t} is homed in {homes} leaves and is cross-section: {cross}"),
        }
    }
    for l in LEAVES {
        for t in tools::tools_of(l) {
            assert!(tools.contains(t.as_str()), "{}: unknown tool {t}", l.id);
        }
    }
    for (id, _) in tools::PROPOSALS {
        assert!(
            super::leaf(id).is_some(),
            "proposal table names unknown leaf {id}"
        );
    }
}

fn fact(kind: &'static str) -> Fact {
    Fact::new(kind, "x")
}

#[test]
fn the_gate_drops_a_fact_whose_kind_is_not_homed_in_a_picked_leaf() {
    let cast = leaf("production/cast-crew").unwrap();
    let budget = leaf("production/budget").unwrap();
    let mut facts = vec![
        fact("Cast"),
        fact("Budget line"),
        fact("Scene"),
        fact("Character"),
        fact("Unknown kind"),
    ];
    gate(&[cast], &mut facts);
    assert_eq!(
        facts.iter().map(|f| f.kind).collect::<Vec<_>>(),
        vec!["Cast"]
    );
    let mut facts = vec![fact("Cast"), fact("Budget line")];
    gate(&[cast, budget], &mut facts);
    assert_eq!(facts.len(), 2);
}

#[test]
fn a_shared_kind_is_homed_by_the_leaf_that_read_it() {
    let cast = leaf("production/cast-crew").unwrap();
    let budget = leaf("production/budget").unwrap();
    let mut from_budget = fact("Count");
    from_budget.home = Some(budget.id);
    let mut from_cast = fact("Count");
    from_cast.home = Some(cast.id);
    assert!(!passes(&[cast], &from_budget));
    assert!(passes(&[cast], &from_cast));
    // A count nobody read is dropped; the orientation code adds itself stays.
    assert!(!passes(&[cast], &fact("Count")));
    assert!(passes(&[cast], &fact("Assumed")));
    assert!(passes(&[cast], &fact("Missing")));
}

#[test]
fn the_story_page_leaf_says_why_it_cannot_answer() {
    let l = leaf("story/story-page").unwrap();
    assert!(l.note.is_some());
    assert!(l.fact_kinds.is_empty() && l.entity_types.is_empty());
}

#[test]
fn a_leaf_reads_as_branch_then_leaf_and_a_lone_leaf_as_its_branch() {
    assert_eq!(
        leaf("production/cast-crew").unwrap().path_label(),
        "Production > Cast & Crew"
    );
    assert_eq!(
        leaf("breakdown/breakdown").unwrap().path_label(),
        "Breakdown"
    );
    assert_eq!(
        leaf("call-sheets/call-sheets").unwrap().path_label(),
        "Call Sheets"
    );
    assert_eq!(leaf("files/files").unwrap().path_label(), "Files");
}

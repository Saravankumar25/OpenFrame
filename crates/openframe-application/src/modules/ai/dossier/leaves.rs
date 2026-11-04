//! The dossier by picked sections (ADR-0041, slice R3). The person chose where to look, so nothing is
//! added: only the fetchers of the picked leaves run, only the kinds of fact each leaf admits pass,
//! and only names a picked leaf can answer about are read. A name that lives elsewhere is listed for
//! the pointer and nothing is said or fetched about it.
//!
//! The budget is split across the leaves by weight, each with a minimum share, and what one leaf does
//! not need flows to the others. Text passages have the lowest priority, so they fill what the facts
//! leave. A fact that carries a required value is never cut.

use super::ctx::FetchCtx;
use super::fetch::story;
use super::{Dossier, Fact, Input, cap_tokens, dedupe, guard, names, trim, wants_facts, words};
use crate::modules::ai::index::{self, Hit, How, Kind, Resolved, SceneHit};
use crate::modules::ai::sections::{Leaf, PickedLeaf, SectionPicker, Selection, home_of};
use crate::modules::ai::{pointer, queries};

/// The least share of the budget a picked leaf gets, in percent, when more than one is picked.
const MIN_SHARE_PERCENT: usize = 20;

/// What the picked sections are, and what the text search found in them (one list per pick). The
/// search runs before the stage: it opens the store itself, and the stage runs inside a read.
pub struct LeafRead<'a> {
    pub selection: &'a Selection,
    pub passages: &'a [Vec<Fact>],
}

/// A name the message holds that no picked leaf can answer about.
#[derive(Debug, Clone, PartialEq)]
pub struct OutsideName {
    pub kind: Kind,
    pub id: String,
    pub name: String,
    /// The leaf where it lives.
    pub home: &'static str,
}

/// How many tokens of the dossier one leaf took.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafTokens {
    pub leaf: &'static str,
    pub tokens: usize,
}

/// Split `cap` tokens across leaves. Each entry is (weight, tokens wanted). Every leaf is promised
/// its share by weight and at least the minimum share; a leaf that wants less gives the rest to the
/// others, by weight.
pub fn split_budget(cap: usize, leaves: &[(u32, usize)]) -> Vec<usize> {
    let n = leaves.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![cap];
    }
    let total: usize = leaves
        .iter()
        .map(|(w, _)| *w as usize)
        .sum::<usize>()
        .max(1);
    let floor = (cap * MIN_SHARE_PERCENT / 100).min(cap / n);
    let mut share: Vec<usize> = leaves
        .iter()
        .map(|(w, _)| (cap * *w as usize / total).max(floor))
        .collect();
    // The floors may have promised more than the cap: take it back from the largest shares.
    let mut over = share.iter().sum::<usize>().saturating_sub(cap);
    while over > 0 {
        let Some((i, room)) = share
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.saturating_sub(floor)))
            .max_by_key(|(_, r)| *r)
        else {
            break;
        };
        if room == 0 {
            break;
        }
        let take = room.min(over);
        share[i] -= take;
        over -= take;
    }
    let mut got: Vec<usize> = share
        .iter()
        .zip(leaves)
        .map(|(s, (_, want))| (*s).min(*want))
        .collect();
    // What was not needed goes to the leaves that still want more, by weight.
    for _ in 0..n {
        let spare = cap.saturating_sub(got.iter().sum());
        if spare == 0 {
            break;
        }
        let hungry: Vec<usize> = (0..n).filter(|i| got[*i] < leaves[*i].1).collect();
        if hungry.is_empty() {
            break;
        }
        let weight: usize = hungry
            .iter()
            .map(|i| leaves[*i].0 as usize)
            .sum::<usize>()
            .max(1);
        let mut given = 0;
        for i in &hungry {
            let part = (spare * leaves[*i].0 as usize / weight).max(1);
            let take = part.min(leaves[*i].1 - got[*i]).min(spare - given);
            got[*i] += take;
            given += take;
        }
        if given == 0 {
            break;
        }
    }
    got
}

/// The names of a message, held to what the picked leaves can answer about. The rest goes to the
/// list for the pointer; a name the conversation only carried along is not listed.
fn restrict(all: &Resolved, sel: &Selection) -> (Resolved, Vec<OutsideName>) {
    let mut r = Resolved {
        not_found: Vec::new(),
        ambiguous: None,
        ..Resolved::default()
    };
    let mut outside = Vec::new();
    let named = |h: &Hit| matches!(h.how, How::Exact | How::Alias | How::Fuzzy(_));
    let mut sort = |kind: Kind, hits: &[Hit], into: &mut Vec<Hit>| {
        if sel.takes(kind) {
            into.extend(hits.iter().cloned());
        } else {
            for h in hits.iter().filter(|h| named(h)) {
                outside.push(OutsideName {
                    kind,
                    id: h.id.clone(),
                    name: h.name.clone(),
                    home: home_of(kind),
                });
            }
        }
    };
    sort(Kind::Person, &all.persons, &mut r.persons);
    sort(Kind::Character, &all.characters, &mut r.characters);
    sort(Kind::Location, &all.locations, &mut r.locations);
    sort(Kind::Act, &all.acts, &mut r.acts);
    sort(Kind::Day, &all.days, &mut r.days);
    sort(Kind::File, &all.files, &mut r.files);
    sort(Kind::BudgetLine, &all.budget_lines, &mut r.budget_lines);
    for (kind, hit) in &all.others {
        let mut one = Vec::new();
        sort(*kind, std::slice::from_ref(hit), &mut one);
        r.others.extend(one.into_iter().map(|h| (*kind, h)));
    }
    if sel.takes(Kind::Scene) {
        r.screenplays = all.screenplays.clone();
        r.scenes = all.scenes.clone();
        r.not_found = all.not_found.clone();
    } else {
        for h in all.screenplays.iter().filter(|h| named(h)) {
            outside.push(OutsideName {
                kind: Kind::Scene,
                id: h.id.clone(),
                name: h.name.clone(),
                home: home_of(Kind::Scene),
            });
        }
        for s in all
            .scenes
            .iter()
            .filter(|s| matches!(s.how, How::Exact | How::Alias | How::Fuzzy(_)))
        {
            outside.push(OutsideName {
                kind: Kind::Scene,
                id: s.scene_id.clone().unwrap_or_default(),
                name: format!("Scene {}", s.ordinal),
                home: home_of(Kind::Scene),
            });
        }
    }
    r.ambiguous = all
        .ambiguous
        .clone()
        .filter(|q| question_applies(&q.text, sel));
    (r, outside)
}

/// The one question the names left open, when a picked leaf can read what it asks about.
fn question_applies(text: &str, sel: &Selection) -> bool {
    // A section with no screenplay to choose reads the one with the production data. The Scenes section
    // has that choice: chosen, the question was settled before this; not chosen, it is asked.
    if text.starts_with("Which screenplay") {
        return sel
            .picks
            .iter()
            .any(|p| p.leaf.picker == Some(SectionPicker::Screenplay));
    }
    for kind in [
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
        if text.starts_with(&format!("Which {} ", kind.label())) {
            return sel.takes(kind);
        }
    }
    false
}

/// The names one leaf reads: the kinds it can answer about, then the instance the person chose.
fn for_leaf(all: &Resolved, p: &PickedLeaf, ix: &index::EntityIndex) -> Resolved {
    let l = p.leaf;
    let mut r = Resolved {
        screenplays: if l.takes(Kind::Scene) {
            all.screenplays.clone()
        } else {
            Vec::new()
        },
        scenes: if l.takes(Kind::Scene) {
            all.scenes.clone()
        } else {
            Vec::new()
        },
        persons: if l.takes(Kind::Person) {
            all.persons.clone()
        } else {
            Vec::new()
        },
        characters: if l.takes(Kind::Character) {
            all.characters.clone()
        } else {
            Vec::new()
        },
        locations: if l.takes(Kind::Location) {
            all.locations.clone()
        } else {
            Vec::new()
        },
        acts: if l.takes(Kind::Act) {
            all.acts.clone()
        } else {
            Vec::new()
        },
        days: if l.takes(Kind::Day) {
            all.days.clone()
        } else {
            Vec::new()
        },
        files: if l.takes(Kind::File) {
            all.files.clone()
        } else {
            Vec::new()
        },
        budget_lines: if l.takes(Kind::BudgetLine) {
            all.budget_lines.clone()
        } else {
            Vec::new()
        },
        others: all
            .others
            .iter()
            .filter(|(k, _)| l.takes(*k))
            .cloned()
            .collect(),
        not_found: Vec::new(),
        ambiguous: None,
    };
    // The draft the person chose: scene numbers are read in that draft, not in the current one.
    if let Some(d) = &p.draft {
        if ix.screenplay(&d.screenplay_id).is_some() && l.takes(Kind::Scene) {
            r.screenplays = vec![Hit {
                id: d.screenplay_id.clone(),
                name: d.screenplay_title.clone(),
                how: How::Focus,
            }];
        }
        if !d.is_current {
            for s in &mut r.scenes {
                if s.screenplay_id == d.screenplay_id {
                    s.scene_id = None;
                }
            }
        }
    }
    if let Some((d, s)) = &p.scene {
        r.screenplays = vec![Hit {
            id: d.screenplay_id.clone(),
            name: d.screenplay_title.clone(),
            how: How::Focus,
        }];
        r.scenes = vec![SceneHit {
            screenplay_id: d.screenplay_id.clone(),
            ordinal: s.number as u32,
            scene_id: Some(s.id.clone()),
            heading: Some(s.heading.clone()),
            how: How::Focus,
        }];
    }
    if let Some(day) = &p.day {
        r.days = vec![Hit {
            id: day.id.clone(),
            name: day
                .number
                .map_or("Off day".to_string(), |n| format!("Shoot Day {n}")),
            how: How::Focus,
        }];
    }
    r
}

/// With several sections picked, each reads its area whatever the question says. What the question did
/// not ask for is reading around it, not something the answer must state: an answer about the scenes
/// need not give the number of characters. A value stays demanded when the words the person typed open
/// the fact, not the section's topic, and the fact speaks of a word of the question.
fn only_the_asked_is_demanded(
    got: &mut [Fact],
    input: &Input<'_>,
    ctx: &FetchCtx<'_>,
    p: &PickedLeaf,
    r: &Resolved,
) {
    let asked = pointer::asked_words(&ctx.message);
    let typed: Vec<String> = if p.leaf.topic.is_empty() {
        Vec::new()
    } else {
        let typed = FetchCtx::typed_for_leaf(input, p);
        p.leaf
            .fetchers
            .iter()
            .flat_map(|fetcher| (fetcher.run)(&typed, input.read, r))
            .map(|f| f.render())
            .collect()
    };
    for f in got.iter_mut() {
        let opened = p.leaf.topic.is_empty() || typed.contains(&f.render());
        if !opened || (!asked.is_empty() && !pointer::speaks_of(f, &asked)) {
            f.requires.clear();
        }
    }
}

/// Build the dossier from the picked sections. Never fails: a fetcher that cannot read adds nothing.
pub fn build(input: &Input<'_>, read: &LeafRead<'_>) -> Dossier {
    let sel = read.selection;
    let base = FetchCtx::new(input);
    let settled = settle_screenplay(input, sel);
    let (all, assumed) = names::effective(&base, input, settled.as_ref().unwrap_or(input.resolved));
    let (resolved, outside) = restrict(&all, sel);
    let mut missing: Vec<String> = resolved.not_found.clone();
    let mut notes_on_guard = 0;
    let mut per_leaf: Vec<Vec<Fact>> = Vec::with_capacity(sel.picks.len());
    let mut leaf_gaps: Vec<Vec<String>> = Vec::with_capacity(sel.picks.len());
    let wants = wants_facts(input.read.group);
    for (i, p) in sel.picks.iter().enumerate() {
        let ctx = FetchCtx::for_leaf(input, p);
        let r = for_leaf(&all, p, input.index);
        let mut got: Vec<Fact> = Vec::new();
        if wants {
            for fetcher in p.leaf.fetchers {
                got.extend((fetcher.run)(&ctx, input.read, &r));
            }
            got.retain(|f| p.leaf.admits(f));
            if sel.picks.len() > 1 {
                only_the_asked_is_demanded(&mut got, input, &ctx, p, &r);
            }
            got.extend(read.passages.get(i).cloned().unwrap_or_default());
        }
        if i == 0 {
            got.extend(
                assumed
                    .iter()
                    .filter(|f| sel.takes(Kind::Scene) && p.leaf.admits(f))
                    .cloned(),
            );
        }
        notes_on_guard += guard::apply(&ctx.guard, &mut got, &mut missing);
        let (gaps, mut facts): (Vec<Fact>, Vec<Fact>) = got.into_iter().partition(Fact::is_missing);
        leaf_gaps.push(gaps.into_iter().filter_map(|g| g.sentence).collect());
        dedupe(&mut facts);
        per_leaf.push(facts);
    }
    // What one pick does not hold is said when the question is about it, or when no other pick has
    // anything: a good answer from Overview is not wrong for leaving out an empty Catalog.
    let asked = pointer::content_words(input.message);
    for (i, gaps) in leaf_gaps.into_iter().enumerate() {
        let others_hold = per_leaf
            .iter()
            .enumerate()
            .any(|(j, facts)| j != i && facts.iter().any(pointer::real));
        for gap in gaps {
            let says_why = gap == guard::NOT_FOR_YOUR_ROLE;
            if (says_why || !others_hold || pointer::gap_concerns(&gap, &asked))
                && !missing.contains(&gap)
            {
                missing.push(gap);
            }
        }
    }
    // A name the picked leaves cannot answer about says nothing here; the pointer uses the list. A name
    // the project does not hold at all is the Story fetcher's to state.
    if wants
        && let Some(who) = story::unknown_name(&base.message, &resolved)
        && resolved.characters.is_empty()
        && sel.takes(Kind::Character)
    {
        let line = queries::could_not_find(&format!("a character called {who}"), &sel.looked_in());
        if !missing.contains(&line) {
            missing.push(line);
        }
    }
    let cap = cap_tokens(input.read.length_class);
    let wanted: Vec<(u32, usize)> = per_leaf
        .iter()
        .zip(&sel.picks)
        .map(|(facts, p)| (p.leaf.weight, facts.iter().map(Fact::tokens).sum()))
        .collect();
    let shares = split_budget(cap, &wanted);
    let mut leaf_tokens = Vec::with_capacity(per_leaf.len());
    let mut facts: Vec<Fact> = Vec::new();
    for ((mut got, p), share) in per_leaf.into_iter().zip(&sel.picks).zip(&shares) {
        trim(&mut got, *share);
        leaf_tokens.push(LeafTokens {
            leaf: p.leaf.id,
            tokens: got.iter().map(Fact::tokens).sum(),
        });
        facts.extend(got);
    }
    dedupe(&mut facts);
    // The same line in two leaves came once; the whole stays within the existing cap unless the
    // required facts alone are larger.
    trim(&mut facts, cap);
    if facts.is_empty() && missing.is_empty() && wants {
        missing.push(queries::could_not_find(
            &format!(
                "anything that matches \u{201C}{}\u{201D}",
                words::clip(base.message.trim(), 80)
            ),
            &sel.looked_in(),
        ));
    }
    let mut required: Vec<String> = Vec::new();
    for f in &facts {
        for v in &f.requires {
            if !required.contains(v) {
                required.push(v.clone());
            }
        }
    }
    let tokens_estimate = facts.iter().map(Fact::tokens).sum();
    Dossier {
        facts,
        required,
        missing,
        notes_on_guard,
        tokens_estimate,
        resolved,
        outside,
        leaf_tokens,
    }
}

/// What one section holds for this message, read under the same guards as an answer. Only for the
/// pointer to count: nothing it returns may reach a reply or a prompt.
pub fn probe(input: &Input<'_>, leaf: &'static Leaf) -> Dossier {
    let selection = Selection {
        picks: vec![PickedLeaf {
            leaf,
            screenplay_id: None,
            draft: None,
            scene: None,
            day: None,
            call_sheet: None,
            label: String::new(),
        }],
        label: String::new(),
    };
    build(
        input,
        &LeafRead {
            selection: &selection,
            passages: &[],
        },
    )
}

/// With a screenplay chosen, the question of which screenplay is settled and a scene number is read
/// in that one.
fn settle_screenplay(input: &Input<'_>, sel: &Selection) -> Option<Resolved> {
    let d = sel.draft()?;
    let info = input.index.screenplay(&d.screenplay_id)?;
    let mut r = input.resolved.clone();
    if r.ambiguous
        .as_ref()
        .is_some_and(|q| q.text.starts_with("Which screenplay"))
    {
        r.ambiguous = None;
    }
    r.screenplays = vec![Hit {
        id: info.id.clone(),
        name: info.title.clone(),
        how: How::Focus,
    }];
    r.scenes = names::scenes_in_message(input.index, input.message, &info.id);
    if r.scenes.is_empty() {
        r.scenes = input
            .resolved
            .scenes
            .iter()
            .filter(|s| s.screenplay_id == info.id)
            .cloned()
            .collect();
    }
    Some(r)
}

//! The out-of-section pointer (spec `ai-section-scoped-assistant`, sections 3.6 and 12.4, round R4).
//!
//! Code only: no model call decides where to look. The person picked the sections; when the
//! question lives in one that was not picked, the reply says which one, in a fixed sentence, and
//! never carries a fact from it. Triggers, in this order:
//!
//! 1. A name the message holds (a character, a person, a place, a note ...) whose home leaf is not
//!    picked. The dossier kept those aside (`Dossier::elsewhere`).
//! 2. The picked leaves returned nothing for the thing asked, and a cheap existence check (words in
//!    the project's search documents) finds it in a leaf that was not picked.
//! 3. The name a person uses for a section ("the tasks"), again only when the picked leaves returned
//!    nothing. A bare topic word (`turn::Domain`) points nowhere: it named the wrong leaf too often.
//!
//! A pick that returned facts about the asked thing almost never points: only trigger 1 may add a
//! partial pointer. Each thing has one home: a Story Day in the Timeline, a shooting day in the
//! Schedule, a call time in the Call Sheets, a scene's props in the Breakdown, a line a character
//! says in the Scenes.
//!
//! Permission is checked before pointing: a leaf the person may not read is never named, and the
//! reply is the plain "I could not find ..." sentence. The pointer names a leaf, never content. When
//! part of the question fits the pick the answer is written from the picked facts as usual and the
//! pointer sentence is appended by code after the checks (it never reaches the model).

use rusqlite::{Connection, params_from_iter};

use super::{Leaf, MAX_PICKS, Picker, SectionPick, leaf};
use crate::core::AppCore;
use crate::modules::ai::dossier::words::Msg;
use crate::modules::ai::dossier::{Dossier, Elsewhere};
use crate::modules::ai::index::{How, Kind, Resolved};
use crate::modules::ai::queries;
use crate::modules::ai::turn::{Domain, Group, TurnRead};
use crate::modules::idea_vault::read_vault;
use crate::util::StoreSel;
use straft_domain::Actor;

/// Where the pointer sentence goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// The sentence is the whole reply: nothing the picked leaves hold answers the question.
    Only,
    /// Part of the question fits the pick: the answer comes first, the sentence follows it.
    After,
}

/// A leaf to point to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Point {
    pub leaf: &'static str,
    /// The leaf as the panel's button names it: `Budget`, `Cast & Crew`.
    pub label: String,
    /// The fixed sentence.
    pub sentence: String,
    pub place: Place,
}

/// What the stage decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Point(Point),
    /// The thing lives in a leaf the person may not read: the plain not-found sentence, no name.
    NotFound(String),
}

pub struct Input<'a> {
    pub conn: &'a Connection,
    /// The person asking: another person's private notes are not searched.
    pub user_id: &'a str,
    pub message: &'a str,
    pub read: &'a TurnRead,
    pub dossier: &'a Dossier,
    pub picks: &'a [SectionPick],
    /// Whether the person may read a leaf (checked before any leaf is named).
    pub may: &'a dyn Fn(&Leaf) -> bool,
    /// The core and the person, for the Global Idea Vault: it lives in another database than the
    /// project's search index. None in unit tests.
    pub vault: Option<(&'a AppCore, &'a Actor)>,
}

/// The sentence for a leaf. At the most leaves a question may read, the person is told to remove one.
pub fn sentence(leaf: &Leaf, picked: usize) -> String {
    let path = leaf.path_label();
    if picked >= MAX_PICKS {
        format!("That is in {path}. Remove a section and add this one, then ask again.")
    } else {
        format!("That is in {path}. Add it to your selection and ask again.")
    }
}

/// Answers that read the project and so can point. A navigation is pointed in `action` (round R6).
pub fn applies(group: Group) -> bool {
    matches!(
        group,
        Group::Facts | Group::Narrative | Group::Brainstorm | Group::Help
    )
}

fn point(leaf: &'static Leaf, picked: usize, place: Place) -> Decision {
    Decision::Point(Point {
        leaf: leaf.id,
        label: leaf.short_label(),
        sentence: sentence(leaf, picked),
        place,
    })
}

/// A leaf the assistant offers: Story Page is the writer's own free writing, kept apart on purpose.
fn offered(l: &Leaf) -> bool {
    l.note.is_none()
}

/// The message as lower case words with one space around each, to look whole words up in.
fn said(message: &str) -> String {
    format!(" {} ", crate::modules::ai::index::normalize(message))
}

fn says(said: &str, words: &[&str]) -> bool {
    words.iter().any(|w| said.contains(&format!(" {w} ")))
}

/// The message names a scene by number ("scene 3"), whether or not the picked leaves resolved it.
fn says_scene_number(said: &str) -> bool {
    let w: Vec<&str> = said.split_whitespace().collect();
    w.windows(2).any(|p| {
        matches!(p[0], "scene" | "scenes")
            && !p[1].is_empty()
            && p[1].chars().all(|c| c.is_ascii_digit())
    })
}

/// The message points back at the conversation ("those", "the second one"): the thing asked is in
/// the answer before, never a new topic to look up.
fn refers_back(said: &str) -> bool {
    says(said, &["those", "these", "them"])
        || (says(said, &["one", "ones"])
            && says(
                said,
                &["first", "second", "third", "last", "next", "previous"],
            ))
}

/// The picks as leaves, in the order picked.
fn picked_of(picks: &[SectionPick]) -> Vec<&'static Leaf> {
    picks.iter().filter_map(|p| leaf(&p.leaf)).collect()
}

/// Trigger 1: a name homed in a leaf that was not picked.
fn names_elsewhere(i: &Input<'_>, picked: &[&'static Leaf]) -> Option<Decision> {
    let looked = || queries::join_and(&picked.iter().map(|l| l.path_label()).collect::<Vec<_>>());
    let mut hidden: Option<String> = None;
    let hay = facts_text(i.dossier);
    for e in &i.dossier.elsewhere {
        // A picked leaf whose facts say the name (a card that mentions Babu) holds the question.
        if hay.contains(&e.name.to_lowercase()) {
            continue;
        }
        let Some(l) = home_for(e, &i.read.domains, i.message)
            .filter(|l| offered(l) && !picked.iter().any(|p| p.id == l.id))
        else {
            continue;
        };
        if (i.may)(l) {
            let place = if names_a_subject(&i.dossier.resolved) {
                Place::After
            } else {
                Place::Only
            };
            return Some(point(l, picked.len(), place));
        }
        hidden.get_or_insert_with(|| queries::could_not_find(&e.name, &looked()));
    }
    hidden.map(Decision::NotFound)
}

/// A pointer for a navigation or a command (round R6): "Open Scene 12" resolves inside the picked
/// leaves, otherwise the pointer names the leaf the thing lives in. Names homed elsewhere count
/// first; then a scene the message names while Screenplay > Scenes was not picked.
pub fn decide_open(i: &Input<'_>) -> Option<Decision> {
    let picked = picked_of(i.picks);
    if let Some(d) = names_elsewhere(i, &picked) {
        return Some(d);
    }
    let scenes = leaf("screenplay/scenes").filter(|l| offered(l))?;
    let named = i
        .dossier
        .resolved
        .scenes
        .iter()
        .any(|h| matches!(h.how, How::Exact | How::Alias | How::Fuzzy(_)));
    if named && !picked.iter().any(|p| p.id == scenes.id) {
        return Some(if (i.may)(scenes) {
            point(scenes, picked.len(), Place::Only)
        } else {
            Decision::NotFound(queries::could_not_find(
                "that scene",
                &queries::join_and(&picked.iter().map(|l| l.path_label()).collect::<Vec<_>>()),
            ))
        });
    }
    None
}

/// The pointer to a leaf that a command needs (the agent gate, round R6).
pub fn point_for(leaf: &'static Leaf, picked: usize) -> Point {
    match point(leaf, picked, Place::Only) {
        Decision::Point(p) => p,
        Decision::NotFound(_) => unreachable!("point builds a pointer"),
    }
}

/// Record the leaf the answer points to in the result's structured data, where the screen and the
/// trace read it.
pub fn attach(out: &mut crate::modules::ai::tools::ToolOutput, p: &Point) {
    let mut data = out
        .structured
        .take()
        .unwrap_or_else(|| serde_json::json!({}));
    if let serde_json::Value::Object(m) = &mut data {
        m.insert(
            "pointer".into(),
            serde_json::json!({ "leaf": p.leaf, "label": p.label }),
        );
    }
    out.structured = Some(data);
}

pub fn decide(i: &Input<'_>) -> Option<Decision> {
    let picked = picked_of(i.picks);
    let n = picked.len();
    let is_picked = |l: &Leaf| picked.iter().any(|p| p.id == l.id);
    let looked = || queries::join_and(&picked.iter().map(|l| l.path_label()).collect::<Vec<_>>());

    // 1. A name homed in a leaf that was not picked.
    if let Some(d) = names_elsewhere(i, &picked) {
        return Some(d);
    }

    // A scene or a day the message names, asked under leaves that do not read it.
    if let Some(d) = address_elsewhere(i, &picked) {
        return Some(d);
    }

    // Triggers 2 and 3 only when the picked leaves returned nothing for what was asked. A name they
    // hold (a character, a person, a place ... named in the message or carried over from the
    // conversation) was returned by them: the question is about it.
    if holds_a_name(&i.dossier.resolved) && i.dossier.facts.iter().any(|f| !f.is_missing()) {
        return None;
    }
    // A scene or a day the message names is an address the picked leaves read by.
    let r = &i.dossier.resolved;
    let named = |how: &How| matches!(how, How::Exact | How::Alias | How::Fuzzy(_));
    if (r.scenes.iter().any(|h| named(&h.how)) || r.days.iter().any(|h| named(&h.how)))
        && i.dossier.facts.iter().any(|f| !f.is_missing())
    {
        return None;
    }
    let said = said(i.message);
    // A scene the message numbers is an address the picked leaves read by, and a question that points
    // back at the conversation has no new topic to look up.
    if (says_scene_number(&said) && i.dossier.facts.iter().any(|f| !f.is_missing()))
        || refers_back(&said)
    {
        return None;
    }
    let terms = terms_of(i.message);
    if terms.is_empty() || mentioned(i.dossier, &terms[..terms.len().min(2)]) {
        return None;
    }
    // 2. The existence check.
    if let Some(l) = exists_elsewhere(i, &terms, &is_picked) {
        return Some(if (i.may)(l) {
            point(l, n, Place::Only)
        } else {
            Decision::NotFound(queries::could_not_find(
                &format!("anything that matches \u{201C}{}\u{201D}", i.message.trim()),
                &looked(),
            ))
        });
    }
    // 3. The name a person uses for a section: its label and aliases.
    let l = alias_hint(i.message, &is_picked)?;
    // "How do I change the theme" is a settings question, not the product guide's.
    let settings = leaf("help/settings").filter(|s| offered(s) && !is_picked(s));
    let l = match settings {
        Some(s) if l.id == "help/product-guide" && i.read.domains.contains(&Domain::Settings) => s,
        _ => l,
    };
    (i.may)(l).then(|| point(l, n, Place::Only))
}

/// Leaves that list scenes, days and the things in them themselves: asked under one of these, a scene
/// or a day in the message is read there and never pointed away.
const READS_ADDRESSES: &[&str] = &[
    "story/timeline",
    "production/reports",
    "production/catalog",
    "breakdown/breakdown",
    "notes-tasks/tasks",
];

/// A scene or a day the message names while no picked leaf reads that address. A scene's props live in
/// the Breakdown (the Catalog holds the same words but not the scene); a scene alone in the
/// Screenplay; a shooting day in the Schedule, but only when the message says shoot, shooting, shot
/// or schedule (a Story Day is the Timeline's, and "Day 2" alone is no shooting day); a call time in
/// the Call Sheets.
fn address_elsewhere(i: &Input<'_>, picked: &[&'static Leaf]) -> Option<Decision> {
    let r = &i.dossier.resolved;
    let named = |how: &How| matches!(how, How::Exact | How::Alias | How::Fuzzy(_));
    let said = said(i.message);
    let scene = r.scenes.iter().any(|h| named(&h.how)) || says_scene_number(&said);
    let day = r.days.iter().any(|h| named(&h.how));
    let has_id = |id: &str| picked.iter().any(|l| l.id == id);
    let has = |f: fn(&Picker) -> bool| picked.iter().any(|l| f(&l.picker));
    let story_day = said.contains(" story day ") || said.contains(" story days ");
    let shoot = says(
        &said,
        &[
            "shoot",
            "shoots",
            "shooting",
            "shot",
            "schedule",
            "scheduled",
        ],
    );
    let reads_day = has(|p| {
        matches!(
            p,
            Picker::ShootingDay | Picker::DayAndScene | Picker::CallSheet
        )
    });
    let target = if story_day {
        if has_id("story/timeline") {
            return None;
        }
        "story/timeline"
    } else if picked.iter().any(|l| READS_ADDRESSES.contains(&l.id)) {
        return None;
    } else if scene && i.read.domains.contains(&Domain::Breakdown) {
        "breakdown/breakdown"
    } else if day && says(&said, &["called", "call"]) && !reads_day {
        "call-sheets/call-sheets"
    } else if (day || scene) && shoot && !reads_day {
        "production/schedule"
    } else if scene
        && !day
        && !shoot
        // A leaf with a picker reads by scene or day (the Daily View, the Sides ...); the rest do not.
        && !has(|p| !matches!(p, Picker::None))
    {
        "screenplay/scenes"
    } else {
        return None;
    };
    let l = leaf(target).filter(|l| offered(l) && !picked.iter().any(|p| p.id == l.id))?;
    (i.may)(l).then(|| point(l, picked.len(), Place::Only))
}

/// The leaf to name for a name homed elsewhere: the leaf a topic word of the question points at when
/// it serves the name's kind ("who plays Anjali" names Cast & Crew, not the Story), else the first
/// leaf in the table that serves the kind.
fn home_for(e: &Elsewhere, domains: &[Domain], message: &str) -> Option<&'static Leaf> {
    // What a character says is written in the scenes, not in the character's card.
    if e.kind == Kind::Character
        && says(
            &said(message),
            &[
                "say", "says", "said", "speak", "speaks", "line", "lines", "dialogue", "reply",
                "replies",
            ],
        )
        && let Some(l) = leaf("screenplay/scenes").filter(|l| offered(l))
    {
        return Some(l);
    }
    domains
        .iter()
        .filter_map(|d| hint_leaf(*d))
        .find(|l| l.name_kinds.contains(&e.kind))
        .or_else(|| leaf(e.leaf))
}

const AMBIGUOUS: &[&str] = &["scenes", "scene", "to do"];

/// The one section that was not picked whose label or alias the message says, the longest name
/// winning ("shoot days" over "days"); None when none, or two with the same length, say it.
fn alias_hint(message: &str, is_picked: &dyn Fn(&Leaf) -> bool) -> Option<&'static Leaf> {
    use crate::modules::ai::index::normalize;
    let said = format!(" {} ", normalize(message));
    let mut best: Option<(usize, &'static Leaf)> = None;
    let mut tie = false;
    for l in super::LEAVES.iter().filter(|l| offered(l) && !is_picked(l)) {
        let longest = std::iter::once(&l.label)
            .chain(l.aliases.iter())
            // "scenes" names a thing in a question, "to do" a verb: neither names a section.
            .map(|n| normalize(n))
            .filter(|n| !AMBIGUOUS.contains(&n.as_str()))
            .filter(|n| !n.is_empty() && said.contains(&format!(" {n} ")))
            .map(|n| n.len())
            .max();
        let Some(len) = longest else { continue };
        match best {
            Some((b, _)) if len < b => {}
            Some((b, _)) if len == b => tie = true,
            _ => {
                best = Some((len, l));
                tie = false;
            }
        }
    }
    best.filter(|_| !tie).map(|(_, l)| l)
}

/// The facts the picked leaves gave, as one lower case text.
fn facts_text(d: &Dossier) -> String {
    d.facts
        .iter()
        .filter(|f| !f.is_missing() && f.kind != "Assumed")
        .map(|f| f.render_plain().to_lowercase())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The leaf a topic word points at, when it is one leaf alone. Story, Production and Visual are
/// several leaves each, so a word of theirs points nowhere.
fn hint_leaf(d: Domain) -> Option<&'static Leaf> {
    let id = match d {
        Domain::Budget => "production/budget",
        Domain::People => "production/cast-crew",
        Domain::Breakdown => "breakdown/breakdown",
        Domain::Schedule => "production/schedule",
        Domain::Screenplay => "screenplay/scenes",
        Domain::Files => "files/files",
        Domain::Settings => "help/settings",
        Domain::Product => "help/product-guide",
        Domain::Story | Domain::Production | Domain::Visual => return None,
    };
    leaf(id).filter(|l| offered(l))
}

/// The message names something the picked leaves hold (not just a screenplay or a pick's own scene):
/// the question has a part that fits, so the answer is written and the pointer follows it.
fn names_a_subject(r: &Resolved) -> bool {
    let named = |how: &How| matches!(how, How::Exact | How::Alias | How::Fuzzy(_));
    r.scenes.iter().any(|h| named(&h.how))
        || r.characters.iter().any(|h| named(&h.how))
        || r.persons.iter().any(|h| named(&h.how))
        || r.locations.iter().any(|h| named(&h.how))
        || r.acts.iter().any(|h| named(&h.how))
        || r.days.iter().any(|h| named(&h.how))
        || r.files.iter().any(|h| named(&h.how))
        || r.budget_lines.iter().any(|h| named(&h.how))
        || r.others.iter().any(|(_, h)| named(&h.how))
}

/// The resolved names the picked leaves serve, however they were found (the message, a pronoun, the
/// conversation focus). A scene or a day is an address, and a pick may supply it, so it does not count.
fn holds_a_name(r: &Resolved) -> bool {
    !(r.characters.is_empty()
        && r.persons.is_empty()
        && r.locations.is_empty()
        && r.acts.is_empty()
        && r.files.is_empty()
        && r.budget_lines.is_empty()
        && r.others.is_empty())
}

/// Verbs a question is asked with, which name no topic.
const ASKING: &[&str] = &[
    "brief",
    "briefing",
    "describe",
    "explain",
    "walk",
    "recap",
    "summarise",
    "summarize",
    "tell",
    "says",
    "mean",
    "means",
    "include",
    "includes",
    "contain",
    "contains",
    "hold",
    "holds",
    "write",
    "wrote",
    "written",
    "mention",
    "mentions",
    "mentioned",
    "talk",
    "talks",
    "called",
    "named",
];

/// The words of the message that name the thing asked: a topic word of four letters or more that
/// is no common word, at most five, the longest first.
fn terms_of(message: &str) -> Vec<String> {
    let mut t: Vec<String> = Msg::new(message)
        .keywords()
        .into_iter()
        .filter(|w| w.chars().count() >= 4 && !ASKING.contains(&w.as_str()))
        .collect();
    t.sort_by_key(|w| std::cmp::Reverse(w.chars().count()));
    t.dedup();
    t.truncate(5);
    t
}

/// How many of the words one document must hold: all of up to two, three fifths of more (rounded up).
fn need(n: usize) -> usize {
    if n <= 2 { n } else { (n * 3).div_ceil(5) }
}

/// A query that scores each document by the number of words it holds and returns, per `key`, the best
/// score and the number of documents that count: they hold `need` words, or both of the two longest.
fn scored(
    key: &str,
    doc: &str,
    from_where: &str,
    terms: &[String],
    args: &mut Vec<String>,
) -> String {
    let mut all = Vec::new();
    for t in terms {
        args.push(format!("%{}%", stem(t).replace(['%', '_'], "")));
        all.push(format!("({doc} LIKE ?{})", args.len()));
    }
    let top = all[..all.len().min(2)].join("+");
    format!(
        "SELECT {key}, MAX(s), COUNT(*) FROM (SELECT {key}, ({}) AS s, ({top}) AS top {from_where}) WHERE s >= {} OR top = {} GROUP BY {key}",
        all.join("+"),
        need(terms.len()),
        // Two long words together are evidence only in a short question; in a longer one a rare word
        // is the point and the long ones are common.
        if terms.len() <= 3 {
            terms.len().min(2)
        } else {
            terms.len() + 1
        }
    )
}

/// The stem a fact line is searched for: a plural or a past tense still matches.
fn stem(w: &str) -> &str {
    let w = w.strip_suffix('s').unwrap_or(w);
    w.strip_suffix("ed").filter(|s| s.len() >= 4).unwrap_or(w)
}

/// Whether any fact the picked leaves gave holds a word of the question. The leaves' own labels and
/// field names count: asking about the budget in the Budget leaf is answered by it.
fn mentioned(d: &Dossier, terms: &[String]) -> bool {
    let hay = d
        .facts
        .iter()
        .filter(|f| !f.is_missing() && f.kind != "Assumed")
        .map(|f| f.render_plain().to_lowercase())
        .collect::<Vec<_>>()
        .join("\n");
    terms.iter().any(|t| hay.contains(stem(t)))
}

/// The leaf that was not picked and holds a search document with all the words, or None. Another
/// person's private notes are not searched (`owner_user_id`).
fn exists_elsewhere(
    i: &Input<'_>,
    terms: &[String],
    is_picked: &dyn Fn(&Leaf) -> bool,
) -> Option<&'static Leaf> {
    // The Breakdown and the Catalog hold the things a question names (props, wardrobe, vehicles ...) by
    // scene or by name: asked under either, the item is theirs to answer.
    if is_picked(leaf("breakdown/breakdown")?) || is_picked(leaf("production/catalog")?) {
        return None;
    }
    let mut args: Vec<String> = vec![i.user_id.to_string()];
    let sql = scored(
        "entity_type",
        "lower(COALESCE(title,'') || ' ' || COALESCE(body,''))",
        "FROM search_doc WHERE (owner_user_id IS NULL OR owner_user_id = ?1)",
        terms,
        &mut args,
    );
    let mut st = i.conn.prepare(&sql).ok()?;
    let rows: Vec<(String, i64, i64)> = st
        .query_map(params_from_iter(args.iter()), |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .ok()?
        .collect::<Result<_, _>>()
        .ok()?;
    // The best leaf: the document that holds most of the words, then the most documents.
    let mut best: Option<(&'static Leaf, (i64, i64))> = None;
    for (etype, score, n) in rows {
        let Some(l) = super::home_of_entity_type(&etype).filter(|l| offered(l)) else {
            continue;
        };
        if is_picked(l) {
            // The picked leaves hold it, so nothing is elsewhere.
            return None;
        }
        // Call sheets repeat the words of every day ("called", "call off"): the message names them.
        if l.id == "call-sheets/call-sheets" {
            continue;
        }
        if best.is_none_or(|(_, b)| (score, n) > b) {
            best = Some((l, (score, n)));
        }
    }
    // The Global Idea Vault is another database: its items are not in the project's search index.
    if let Some((core, actor)) = i.vault
        && let Some(g) = leaf("idea-vault/global").filter(|l| offered(l) && !is_picked(l))
        && (i.may)(g)
    {
        let mut args: Vec<String> = Vec::new();
        let sql = scored(
            "item_type",
            "lower(COALESCE(title,'') || ' ' || COALESCE(body,'') || ' ' || COALESCE(caption,'') || ' ' || COALESCE(source_text,'') || ' ' || COALESCE(url,''))",
            "FROM vault_item WHERE deleted_at IS NULL",
            terms,
            &mut args,
        );
        let found = read_vault(core, actor, StoreSel::Global, |c, _| {
            let mut st = c.prepare(&sql)?;
            let rows: Vec<(i64, i64)> = st
                .query_map(params_from_iter(args.iter()), |r| {
                    Ok((r.get(1)?, r.get(2)?))
                })?
                .collect::<Result<_, _>>()?;
            Ok(rows)
        })
        .unwrap_or_default();
        let score = found.iter().map(|r| r.0).max();
        if let Some(score) = score {
            let n = found.iter().map(|r| r.1).sum::<i64>();
            if best.is_none_or(|(_, b)| (score, n) > b) {
                best = Some((g, (score, n)));
            }
        }
    }
    best.map(|(l, _)| l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sentence_names_the_leaf_and_changes_at_three_sections() {
        let l = leaf("production/budget").unwrap();
        assert_eq!(
            sentence(l, 1),
            "That is in Production > Budget. Add it to your selection and ask again."
        );
        assert_eq!(
            sentence(l, 3),
            "That is in Production > Budget. Remove a section and add this one, then ask again."
        );
        assert_eq!(
            sentence(leaf("breakdown/breakdown").unwrap(), 2),
            "That is in Breakdown. Add it to your selection and ask again."
        );
    }

    #[test]
    fn only_answers_that_read_the_project_can_point() {
        assert!(applies(Group::Facts) && applies(Group::Narrative) && applies(Group::Help));
        assert!(!applies(Group::Chat) && !applies(Group::Change) && !applies(Group::Navigate));
    }

    #[test]
    fn the_story_page_is_never_offered_as_a_target() {
        assert!(!offered(leaf("story/story-page").unwrap()));
        assert!(offered(leaf("production/budget").unwrap()));
    }

    #[test]
    fn terms_are_the_two_longest_topic_words_and_plurals_match() {
        let t = terms_of("How many wardrobe changes does the helicopter pilot have?");
        assert_eq!(t[..2], ["helicopter".to_string(), "wardrobe".to_string()]);
        assert_eq!(t.len(), 4);
        assert_eq!(
            (need(1), need(2), need(3), need(4), need(5)),
            (1, 2, 2, 3, 3)
        );
        assert_eq!(stem("props"), "prop");
        assert_eq!(stem("scenes"), "scene");
        assert!(terms_of("What is it?").is_empty());
    }

    use crate::modules::ai::dossier::Fact;
    use crate::modules::ai::index::{Hit, Kind};
    use crate::modules::ai::turn::{AnswerType, ReadHow};

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE search_doc(entity_type TEXT, entity_id TEXT, title TEXT, body TEXT, owner_user_id TEXT);
             INSERT INTO search_doc VALUES ('project_note','n1','Sea safety plan','Harbour wind limit is 20 knots.',NULL);
             INSERT INTO search_doc VALUES ('private_note','n2','Mine','The contingency is secret.','someone-else');",
        )
        .unwrap();
        c
    }

    fn read(domains: &[Domain]) -> TurnRead {
        TurnRead {
            answer_type: AnswerType::Lookup,
            group: Group::Facts,
            domains: domains.to_vec(),
            length_class: AnswerType::Lookup.length_class(),
            how: ReadHow::Rule,
            vote: 0.0,
        }
    }

    fn picks(leaves: &[&str]) -> Vec<SectionPick> {
        leaves
            .iter()
            .map(|l| SectionPick {
                leaf: (*l).to_string(),
                screenplay_id: None,
                draft_id: None,
                scene_id: None,
                shooting_day_id: None,
                call_sheet_id: None,
            })
            .collect()
    }

    fn decide_with(
        message: &str,
        domains: &[Domain],
        leaves: &[&str],
        d: &Dossier,
        may: &dyn Fn(&Leaf) -> bool,
    ) -> Option<Decision> {
        let c = db();
        decide(&Input {
            conn: &c,
            user_id: "me",
            message,
            read: &read(domains),
            dossier: d,
            picks: &picks(leaves),
            may,
            vault: None,
        })
    }

    fn yes(_: &Leaf) -> bool {
        true
    }

    fn away(leaf: &'static str, name: &str) -> Dossier {
        Dossier {
            elsewhere: vec![Elsewhere {
                leaf,
                kind: if leaf == "production/budget" {
                    Kind::BudgetLine
                } else {
                    Kind::Character
                },
                name: name.into(),
            }],
            ..Dossier::default()
        }
    }

    fn pointed(d: Option<Decision>) -> Point {
        match d {
            Some(Decision::Point(p)) => p,
            other => panic!("expected a pointer, got {other:?}"),
        }
    }

    #[test]
    fn a_name_homed_elsewhere_points_to_its_leaf_and_nothing_else() {
        let d = away("production/cast-crew", "Anjali");
        let p = pointed(decide_with(
            "Who plays Anjali?",
            &[],
            &["production/budget"],
            &d,
            &yes,
        ));
        assert_eq!(p.leaf, "production/cast-crew");
        assert_eq!(p.label, "Cast & Crew");
        assert_eq!(p.place, Place::Only);
        assert_eq!(
            p.sentence,
            "That is in Production > Cast & Crew. Add it to your selection and ask again."
        );
    }

    #[test]
    fn a_topic_word_picks_the_serving_leaf_that_fits_the_question() {
        // A character is served by the Story, the Cast & Crew and others; "plays" is a People word.
        let d = Dossier {
            elsewhere: vec![Elsewhere {
                leaf: "story/characters",
                kind: Kind::Character,
                name: "Anjali".into(),
            }],
            ..Dossier::default()
        };
        let with = pointed(decide_with(
            "Who plays Anjali?",
            &[Domain::People],
            &["production/budget"],
            &d,
            &yes,
        ));
        assert_eq!(with.leaf, "production/cast-crew");
        let without = pointed(decide_with(
            "Tell me about Anjali",
            &[],
            &["production/budget"],
            &d,
            &yes,
        ));
        assert_eq!(without.leaf, "story/characters");
    }

    #[test]
    fn at_three_sections_the_sentence_says_to_remove_one() {
        let d = away("production/budget", "Camera");
        let p = pointed(decide_with(
            "What is Camera planned at?",
            &[],
            &["files/files", "notes-tasks/tasks", "notes-tasks/notes"],
            &d,
            &yes,
        ));
        assert!(p.sentence.contains("Remove a section and add this one"));
    }

    #[test]
    fn part_of_the_question_fitting_the_pick_gets_an_answer_then_the_pointer() {
        let mut d = away("production/cast-crew", "Anjali");
        d.resolved.locations.push(Hit {
            id: "l1".into(),
            name: "Harbour".into(),
            how: How::Exact,
        });
        let p = pointed(decide_with(
            "Who plays Anjali and is the Harbour confirmed?",
            &[],
            &["production/locations"],
            &d,
            &yes,
        ));
        assert_eq!(p.place, Place::After);
        // A location the pick itself supplied is not the message naming something.
        let mut d = away("production/cast-crew", "Anjali");
        d.resolved.locations.push(Hit {
            id: "l1".into(),
            name: "Harbour".into(),
            how: How::Focus,
        });
        let p = pointed(decide_with(
            "Who plays Anjali?",
            &[],
            &["production/locations"],
            &d,
            &yes,
        ));
        assert_eq!(p.place, Place::Only);
    }

    #[test]
    fn a_leaf_the_person_may_not_read_is_never_named() {
        let d = away("production/budget", "Camera");
        let no_budget = |l: &Leaf| l.id != "production/budget";
        let got = decide_with(
            "What is Camera planned at?",
            &[],
            &["files/files"],
            &d,
            &no_budget,
        );
        let Some(Decision::NotFound(s)) = got else {
            panic!("expected the not-found sentence, got {got:?}");
        };
        assert_eq!(s, "I could not find Camera. I looked in Files.");
        assert!(!s.contains("Budget"));
        // Another leaf the person may read is still named.
        let mut d = d;
        d.elsewhere.push(Elsewhere {
            leaf: "production/cast-crew",
            kind: Kind::Character,
            name: "Anjali".into(),
        });
        let p = pointed(decide_with(
            "Camera and Anjali",
            &[],
            &["files/files"],
            &d,
            &no_budget,
        ));
        assert_eq!(p.leaf, "production/cast-crew");
    }

    #[test]
    fn the_existence_check_points_only_when_the_picked_leaves_returned_nothing() {
        let none = Dossier::default();
        let p = pointed(decide_with(
            "What does the harbour wind limit say?",
            &[],
            &["production/budget"],
            &none,
            &yes,
        ));
        assert_eq!(p.leaf, "notes-tasks/notes");
        // The picked leaf already mentions a word of the question: no pointer.
        let have = Dossier {
            facts: vec![Fact::new("Budget line", "b1").f("description", "Harbour permit")],
            ..Dossier::default()
        };
        assert_eq!(
            decide_with(
                "What does the harbour wind limit say?",
                &[],
                &["production/budget"],
                &have,
                &yes
            ),
            None
        );
        // The notes are picked: it is there, nothing is elsewhere.
        assert_eq!(
            decide_with(
                "What does the harbour wind limit say?",
                &[],
                &["notes-tasks/notes"],
                &none,
                &yes
            ),
            None
        );
        // Nothing anywhere: the dossier's own could-not-find stands.
        assert_eq!(
            decide_with(
                "What about the zeppelin hangar?",
                &[],
                &["production/budget"],
                &none,
                &yes
            ),
            None
        );
        // Another person's private note is not searched.
        assert_eq!(
            decide_with(
                "Is the contingency secret?",
                &[],
                &["production/budget"],
                &none,
                &yes
            ),
            None
        );
    }

    #[test]
    fn a_bare_topic_word_points_nowhere() {
        // "rupees" is a Budget word, but a word is no proof: the Budget is not named.
        assert_eq!(
            decide_with(
                "What was spent in rupees?",
                &[Domain::Budget],
                &["files/files"],
                &Dossier::default(),
                &yes
            ),
            None
        );
    }

    #[test]
    fn a_name_the_picked_leaves_facts_say_is_held_by_them() {
        // The Beat Board cards say Babu: the question is about the cards, not about the character.
        let mut d = away("story/characters", "Babu");
        d.facts
            .push(Fact::new("Card", "c1").f("description", "Babu cuts the power at the station"));
        assert_eq!(
            decide_with(
                "Which cards are about Babu?",
                &[],
                &["story/beat-board"],
                &d,
                &yes
            ),
            None
        );
    }

    #[test]
    fn the_name_a_person_uses_for_a_section_points_to_it_when_the_picked_ones_gave_nothing() {
        let p = pointed(decide_with(
            "Which tasks are still open?",
            &[],
            &["files/files"],
            &Dossier::default(),
            &yes,
        ));
        assert_eq!(p.leaf, "notes-tasks/tasks");
        // Two sections said by name with the same length: no guess.
        assert_eq!(
            decide_with(
                "Compare the notes and the tasks",
                &[],
                &["files/files"],
                &Dossier::default(),
                &yes
            )
            .map(|d| matches!(d, Decision::Point(_))),
            None
        );
    }

    #[test]
    fn a_scene_or_day_the_message_names_is_read_by_the_picked_leaves_not_pointed_away() {
        use crate::modules::ai::index::SceneHit;
        let mut d = Dossier {
            facts: vec![Fact::new("Shoot day", "d1").f("scenes", "3, 12")],
            ..Dossier::default()
        };
        d.resolved.scenes.push(SceneHit {
            screenplay_id: "sp".into(),
            ordinal: 12,
            scene_id: Some("s12".into()),
            heading: None,
            how: How::Exact,
        });
        assert_eq!(
            decide_with(
                "Which day do we shoot scene 12?",
                &[Domain::Schedule],
                &["production/daily-view"],
                &d,
                &yes
            ),
            None
        );
    }

    #[test]
    fn words_spread_over_one_document_are_enough_when_three_fifths_match() {
        // Five words, the document holds three (the two longest are not both in it).
        let p = pointed(decide_with(
            "What is the harbour wind limit at the zeppelin hangar?",
            &[],
            &["production/budget"],
            &Dossier::default(),
            &yes,
        ));
        assert_eq!(p.leaf, "notes-tasks/notes");
        // One word of five is no evidence.
        assert_eq!(
            decide_with(
                "What is the harbour permit zeppelin hangar rule?",
                &[],
                &["production/budget"],
                &Dossier::default(),
                &yes
            ),
            None
        );
    }

    fn scene_dossier(how: How) -> Dossier {
        use crate::modules::ai::index::SceneHit;
        let mut d = Dossier {
            facts: vec![Fact::new("Scene", "s3").f("heading", "INT. STATION")],
            ..Dossier::default()
        };
        d.resolved.scenes.push(SceneHit {
            screenplay_id: "sp".into(),
            ordinal: 3,
            scene_id: Some("s3".into()),
            heading: None,
            how,
        });
        d
    }

    #[test]
    fn a_named_scene_asked_for_props_points_to_the_breakdown_and_not_the_catalog() {
        let d = scene_dossier(How::Exact);
        for picks in [&["production/budget"][..], &["screenplay/scenes"][..]] {
            let p = pointed(decide_with(
                "What props does scene 3 need?",
                &[Domain::Breakdown],
                picks,
                &d,
                &yes,
            ));
            assert_eq!(p.leaf, "breakdown/breakdown");
        }
        // The Breakdown is picked: the question is read there.
        assert_eq!(
            decide_with(
                "What props does scene 3 need?",
                &[Domain::Breakdown],
                &["breakdown/breakdown"],
                &d,
                &yes
            ),
            None
        );
    }

    #[test]
    fn a_scene_or_day_asked_under_a_leaf_that_does_not_read_it_points_to_screenplay_or_schedule() {
        let scene = scene_dossier(How::Exact);
        let p = pointed(decide_with(
            "What happens in scene 3?",
            &[],
            &["production/budget"],
            &scene,
            &yes,
        ));
        assert_eq!(p.leaf, "screenplay/scenes");
        // A leaf that reads scenes keeps the question.
        assert_eq!(
            decide_with(
                "What happens in scene 3?",
                &[],
                &["production/daily-view"],
                &scene,
                &yes
            ),
            None
        );
        let mut day = Dossier::default();
        day.resolved.days.push(Hit {
            id: "d2".into(),
            name: "Day 2".into(),
            how: How::Exact,
        });
        let p = pointed(decide_with(
            "What is shot on day 2?",
            &[],
            &["production/budget"],
            &day,
            &yes,
        ));
        assert_eq!(p.leaf, "production/schedule");
        // A leaf the person may not read is never named.
        let no = |l: &Leaf| l.id != "production/schedule";
        assert_eq!(
            decide_with(
                "What is shot on day 2?",
                &[],
                &["production/budget"],
                &day,
                &no
            ),
            None
        );
    }

    fn day(name: &str) -> Dossier {
        let mut d = Dossier::default();
        d.resolved.days.push(Hit {
            id: "d2".into(),
            name: name.into(),
            how: How::Exact,
        });
        d
    }

    #[test]
    fn a_story_day_belongs_to_the_timeline_and_never_to_the_schedule() {
        let d = day("DAY 2");
        // Asked under the Timeline itself, "Day 2" is read there.
        for q in [
            "Which scenes are on DAY 2?",
            "What Story Day is scene 11 on?",
        ] {
            assert_eq!(
                decide_with(q, &[Domain::Schedule], &["story/timeline"], &d, &yes),
                None,
                "{q}"
            );
        }
        // Asked elsewhere, the words Story Day name the Timeline.
        let p = pointed(decide_with(
            "What Story Day is scene 11 on?",
            &[Domain::Story],
            &["help/product-guide"],
            &d,
            &yes,
        ));
        assert_eq!(p.leaf, "story/timeline");
    }

    #[test]
    fn a_day_is_a_shooting_day_only_when_the_message_says_so() {
        let d = day("Shoot Day 1");
        // A date or a bare "Day 1" is no shooting day.
        assert_eq!(
            decide_with(
                "Who is unavailable on 2027-03-07, Day 1?",
                &[Domain::Schedule],
                &["production/cast-crew"],
                &d,
                &yes
            ),
            None
        );
        let p = pointed(decide_with(
            "What is shot on day 1?",
            &[],
            &["production/cast-crew"],
            &d,
            &yes,
        ));
        assert_eq!(p.leaf, "production/schedule");
        // A call time on a shooting day is the Call Sheets.
        let p = pointed(decide_with(
            "What time is the makeup artist called on Shoot Day 1?",
            &[Domain::Schedule],
            &["files/files"],
            &d,
            &yes,
        ));
        assert_eq!(p.leaf, "call-sheets/call-sheets");
    }

    #[test]
    fn leaves_that_list_scenes_and_days_read_them_without_a_pointer() {
        let scene = scene_dossier(How::Exact);
        for leaf_id in ["production/reports", "story/timeline", "notes-tasks/tasks"] {
            assert_eq!(
                decide_with(
                    "What is the heading of scene 3?",
                    &[],
                    &[leaf_id],
                    &scene,
                    &yes
                ),
                None,
                "{leaf_id}"
            );
        }
        assert_eq!(
            decide_with(
                "Which day is scene 3 shot?",
                &[Domain::Schedule],
                &["production/schedule"],
                &scene,
                &yes
            ),
            None
        );
    }

    #[test]
    fn a_scene_numbered_in_the_message_asked_for_props_points_to_the_breakdown_unresolved() {
        // The picked leaves did not resolve the scene: the words still say it.
        let p = pointed(decide_with(
            "What props does scene 3 need?",
            &[Domain::Screenplay, Domain::Breakdown],
            &["production/budget"],
            &Dossier::default(),
            &yes,
        ));
        assert_eq!(p.leaf, "breakdown/breakdown");
        // Which scenes use a prop is the Breakdown or the Catalog own question.
        assert_eq!(
            decide_with(
                "Which scenes use the rubber stamp?",
                &[Domain::Screenplay],
                &["production/catalog"],
                &Dossier::default(),
                &yes
            ),
            None
        );
    }

    #[test]
    fn the_existence_check_leaves_items_to_the_breakdown_and_catalog_and_the_conversation() {
        for picks in [&["breakdown/breakdown"][..], &["production/catalog"][..]] {
            assert_eq!(
                decide_with(
                    "What does the harbour wind limit say?",
                    &[],
                    picks,
                    &Dossier::default(),
                    &yes
                ),
                None
            );
        }
        // "those" points back at the answer before: no new topic.
        assert_eq!(
            decide_with(
                "Which of those is due before the harbour wind limit?",
                &[],
                &["production/budget"],
                &Dossier::default(),
                &yes
            ),
            None
        );
        // "called" is a way of asking, not a topic.
        assert!(terms_of("What is Act Two called?").is_empty());
    }

    #[test]
    fn a_long_question_needs_more_than_its_two_longest_words() {
        // station and coast are in a note; mast and tall are not: two of four is not enough.
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE search_doc(entity_type TEXT, entity_id TEXT, title TEXT, body TEXT, owner_user_id TEXT);
             INSERT INTO search_doc VALUES ('project_note','n1','Coast station','The coast station keeps a log.',NULL);",
        )
        .unwrap();
        let none = Dossier::default();
        let ask = |m: &str| {
            decide(&Input {
                conn: &c,
                user_id: "me",
                message: m,
                read: &read(&[]),
                dossier: &none,
                picks: &picks(&["production/budget"]),
                may: &yes,
                vault: None,
            })
        };
        assert_eq!(ask("How tall is the coast station mast?"), None);
        let p = pointed(ask("What does the coast station keep?"));
        assert_eq!(p.leaf, "notes-tasks/notes");
    }

    #[test]
    fn what_a_character_says_is_in_the_scenes_and_a_section_name_is_not_a_scene_word() {
        let d = away("story/characters", "Harbour Clerk");
        let p = pointed(decide_with(
            "What does the Harbour Clerk say about the licence fee?",
            &[],
            &["production/budget"],
            &d,
            &yes,
        ));
        assert_eq!(p.leaf, "screenplay/scenes");
        // "scenes" in a question and "to do" as a verb name no section.
        for q in ["Which scenes use the lamp?", "What do I need to do next?"] {
            assert_eq!(
                decide_with(q, &[], &["files/files"], &Dossier::default(), &yes),
                None,
                "{q}"
            );
        }
        // How do I change a setting belongs to the Settings, not the product guide.
        let p = pointed(decide_with(
            "How do I pick another colour theme?",
            &[Domain::Settings, Domain::Product],
            &["files/files"],
            &Dossier::default(),
            &yes,
        ));
        assert_eq!(p.leaf, "help/settings");
    }

    #[test]
    fn every_topic_hint_is_one_offered_leaf() {
        for d in [
            Domain::Budget,
            Domain::People,
            Domain::Breakdown,
            Domain::Schedule,
            Domain::Screenplay,
            Domain::Files,
            Domain::Settings,
            Domain::Product,
        ] {
            assert!(hint_leaf(d).is_some(), "{d:?}");
        }
        assert!(hint_leaf(Domain::Story).is_none());
    }
}

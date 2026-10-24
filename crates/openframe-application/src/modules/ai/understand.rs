//! Query understanding in code (AI harness rebuild plan, section 5.3).
//!
//! "Code researches, the model writes": this module reads a person's question and the draft
//! and says which characters, locations, scenes and acts the question names, how deep the
//! answer should go and what kind of question it is. No model is involved and nothing is
//! written. The Chat lane (a later slice) calls it before it builds a briefing.
//!
//! Scene numbers are the display numbers of `queries::scenes`: position order, omitted scenes
//! keep their number. An omitted or missing scene is reported in `missing_scenes`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use openframe_domain::AppResult;
use rusqlite::Connection;

use super::queries::{self, DraftRef};
use super::scope::ResolvedScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EntityKind {
    Character,
    Location,
    Act,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entity {
    pub kind: EntityKind,
    /// Characters and locations: the normalized (upper case) name. Acts: the act id.
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Depth {
    Quick,
    Explain,
    Deep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intent {
    Explain,
    Character,
    Relationship,
    Compare,
    Theme,
    Critique,
    Ideas,
    Whole,
    ListOrCount,
    Find,
    Other,
}

#[derive(Debug, Clone)]
pub(crate) struct Understood {
    /// Characters and locations in order of first mention.
    pub entities: Vec<Entity>,
    /// Scene numbers that exist in the draft (not omitted), sorted, no repeats.
    pub scenes: Vec<usize>,
    /// Scene numbers the person asked for that the draft does not have.
    pub missing_scenes: Vec<usize>,
    pub acts: Vec<Entity>,
    pub depth: Depth,
    pub intent: Intent,
    /// (word as asked, name it was matched to).
    pub fuzzy: Vec<(String, String)>,
    /// A pronoun with more than one possible character before it: the caller asks which one.
    pub pronoun_unresolved: bool,
    pub whole_range: bool,
}

// ------------------------------------------------------------------ names

/// A character the draft or the Story section knows by name.
#[derive(Debug, Clone)]
pub(super) struct CharName {
    /// Upper case cue without extensions: "DEVI", "VENKAT RAO".
    pub key: String,
    pub label: String,
    pub words: Vec<String>,
}

/// A phrase that names one entry of a name list.
#[derive(Debug, Clone)]
pub(super) struct Alias {
    pub words: Vec<String>,
    pub idx: usize,
}

/// Upper case alphanumeric words. An apostrophe splits, so "Devi's" is DEVI and S.
pub(super) fn words(text: &str) -> Vec<String> {
    text.to_uppercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Speaking cue names of the draft plus the Story section's character names, by key.
pub(super) fn character_names(c: &Connection, draft_id: &str) -> AppResult<Vec<CharName>> {
    let mut by_key: BTreeMap<String, String> = BTreeMap::new();
    for key in queries::character_cues(c, draft_id)?.into_keys() {
        by_key.insert(key.clone(), key);
    }
    for (_, name, _) in queries::character_records(c)? {
        let key = queries::normalize_cue(&name);
        if !key.is_empty() {
            by_key.entry(key).or_insert_with(|| name.trim().to_string());
        }
    }
    Ok(by_key
        .into_iter()
        .filter_map(|(key, label)| {
            let words = words(&key);
            (!words.is_empty() && key.chars().count() >= 2).then_some(CharName {
                key,
                label,
                words,
            })
        })
        .collect())
}

/// The full name of each character, and the first word of a longer name when no other
/// character starts with that word ("VENKAT RAO" is also "VENKAT").
pub(super) fn aliases(names: &[CharName]) -> Vec<Alias> {
    let mut first: HashMap<&str, usize> = HashMap::new();
    for n in names {
        *first.entry(n.words[0].as_str()).or_default() += 1;
    }
    let mut out = Vec::new();
    for (idx, n) in names.iter().enumerate() {
        out.push(Alias {
            words: n.words.clone(),
            idx,
        });
        if n.words.len() > 1 && first[n.words[0].as_str()] == 1 && n.words[0].chars().count() >= 2 {
            out.push(Alias {
                words: vec![n.words[0].clone()],
                idx,
            });
        }
    }
    out
}

fn letters(phrase: &[String]) -> usize {
    phrase.iter().map(|w| w.chars().count()).sum()
}

/// Equal but for one inserted, deleted or changed letter.
fn within_one(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a == b || a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let skip = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let (a, b) = (&a[skip..], &b[skip..]);
    match a.len().cmp(&b.len()) {
        std::cmp::Ordering::Equal => a[1..] == b[1..],
        std::cmp::Ordering::Less => a == &b[1..],
        std::cmp::Ordering::Greater => &a[1..] == b,
    }
}

// -------------------------------------------------------------- locations

struct LocationName {
    key: String,
    label: String,
    phrases: Vec<Vec<String>>,
}

/// Location parts of the scene headings ("LOST PROPERTY OFFICE" from "INT. KESARGANJ JUNCTION -
/// LOST PROPERTY OFFICE - NIGHT") plus the production Location records. The whole location is a
/// phrase; so is its last part after " - " when no other location ends the same way.
fn location_names(c: &Connection, scenes: &[queries::SceneRow]) -> AppResult<Vec<LocationName>> {
    let mut heading: BTreeMap<String, String> = BTreeMap::new();
    for s in scenes {
        let loc = queries::parse_heading(&s.heading).location;
        if !loc.is_empty() {
            heading.entry(words(&loc).join(" ")).or_insert(loc);
        }
    }
    let last_part = |loc: &str| -> Option<Vec<String>> {
        let (_, tail) = ["\u{2014}", "\u{2013}", " - "]
            .iter()
            .find_map(|d| loc.rsplit_once(d))?;
        let w = words(tail);
        (!w.is_empty()).then_some(w)
    };
    let mut tails: HashMap<String, usize> = HashMap::new();
    for loc in heading.values() {
        if let Some(t) = last_part(loc) {
            *tails.entry(t.join(" ")).or_default() += 1;
        }
    }
    let mut out: Vec<LocationName> = Vec::new();
    for (key, loc) in &heading {
        let mut phrases = vec![words(loc)];
        if let Some(t) = last_part(loc)
            && tails[&t.join(" ")] == 1
            && !heading.contains_key(&t.join(" "))
        {
            phrases.push(t);
        }
        out.push(LocationName {
            key: key.clone(),
            label: loc.clone(),
            phrases,
        });
    }
    if queries::table_exists(c, "location")? {
        let mut stmt = c.prepare(
            "SELECT name FROM location WHERE deleted_at IS NULL AND archived=0 ORDER BY name",
        )?;
        let names: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        for name in names {
            let w = words(&name);
            if w.is_empty() {
                continue;
            }
            // Already known by a heading: the record's own spelling becomes the label.
            if let Some(known) = out.iter_mut().find(|l| l.phrases.contains(&w)) {
                known.label = name.trim().to_string();
            } else {
                out.push(LocationName {
                    key: w.join(" "),
                    label: name.trim().to_string(),
                    phrases: vec![w],
                });
            }
        }
    }
    Ok(out)
}

/// The words of the project's own names (characters and places), lower case, that the spelling
/// tolerant routing copy of a request may correct toward.
pub(super) fn name_words(c: &Connection, draft: &DraftRef) -> AppResult<Vec<String>> {
    let rows = queries::scenes(c, &draft.id)?;
    let mut out: BTreeSet<String> = BTreeSet::new();
    for n in character_names(c, &draft.id)? {
        out.extend(n.words.iter().map(|w| w.to_lowercase()));
    }
    for l in location_names(c, &rows)? {
        for p in &l.phrases {
            out.extend(p.iter().map(|w| w.to_lowercase()));
        }
    }
    Ok(out.into_iter().collect())
}

// ------------------------------------------------------------------- acts

fn number_word(w: &str) -> Option<usize> {
    const NUMBERS: [&[&str]; 10] = [
        &["1", "ONE", "I", "FIRST"],
        &["2", "TWO", "II", "SECOND"],
        &["3", "THREE", "III", "THIRD"],
        &["4", "FOUR", "IV", "FOURTH"],
        &["5", "FIVE", "V", "FIFTH"],
        &["6", "SIX", "VI", "SIXTH"],
        &["7", "SEVEN", "VII", "SEVENTH"],
        &["8", "EIGHT", "VIII", "EIGHTH"],
        &["9", "NINE", "IX", "NINTH"],
        &["10", "TEN", "X", "TENTH"],
    ];
    NUMBERS.iter().position(|l| l.contains(&w)).map(|i| i + 1)
}

/// "act 2", "act two", "act II", "acts 1 and 2", "second act", "last act" as act numbers
/// (`usize::MAX` stands for the last act).
fn act_refs(tokens: &[String]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if (t == "ACT" || t == "ACTS")
            && let Some(n) = tokens.get(i + 1).and_then(|w| number_word(w))
        {
            out.push(n);
            i += 2;
            while tokens.get(i).map(String::as_str) == Some("AND")
                && let Some(n) = tokens.get(i + 1).and_then(|w| number_word(w))
            {
                out.push(n);
                i += 2;
            }
            continue;
        }
        if tokens.get(i + 1).map(String::as_str) == Some("ACT") {
            if let Some(n) =
                number_word(t).filter(|_| t.len() > 2 && t.chars().all(char::is_alphabetic))
            {
                out.push(n);
            } else if t == "LAST" || t == "FINAL" {
                out.push(usize::MAX);
            }
        }
        i += 1;
    }
    out
}

pub(super) fn acts_of(c: &Connection, draft: &DraftRef) -> AppResult<Vec<(String, String)>> {
    let mut stmt = c.prepare(
        "SELECT id, title FROM story_act WHERE deleted_at IS NULL AND episode_id IS ?1
         ORDER BY position, id",
    )?;
    Ok(stmt
        .query_map([&draft.episode_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?)
}

// --------------------------------------------------------------- keywords

/// The question as lower case words with apostrophes removed, padded with spaces so a phrase can
/// be looked up as " how many ".
fn plain(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .filter(|c| !matches!(c, '\'' | '\u{2019}' | '\u{2018}'))
        .collect();
    format!(" {} ", words(&cleaned).join(" ").to_lowercase())
}

fn has_any(plain: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|p| plain.contains(&format!(" {p} ")))
}

const COMPARE: &[&str] = &[
    "compare",
    "comparing",
    "comparison",
    "contrast",
    "difference",
    "differences",
    "versus",
    "vs",
    "changed between",
    "change between",
];
const RELATIONSHIP: &[&str] = &[
    "dynamic",
    "dynamics",
    "relationship",
    "relationships",
    "between",
    "feel about",
    "feels about",
    "get along",
    "gets along",
    "with each other",
];
const THEME: &[&str] = &[
    "theme",
    "themes",
    "meaning",
    "about at heart",
    "symbol",
    "symbols",
    "symbolism",
    "motif",
    "motifs",
];
const CRITIQUE: &[&str] = &[
    "critique",
    "notes on",
    "what works",
    "weak",
    "weaker",
    "weakest",
    "stronger",
    "improve",
    "feedback",
];
const IDEAS: &[&str] = &[
    "idea",
    "ideas",
    "suggest",
    "suggestions",
    "brainstorm",
    "what if",
    "alternative",
    "alternatives",
];
const LIST_COUNT: &[&str] = &[
    "how many",
    "count",
    "counts",
    "list",
    "which scenes",
    "name all",
    "number of",
];
const WHOLE: &[&str] = &[
    "whole",
    "entire",
    "overall",
    "all the scenes",
    "each act",
    "every scene",
    "start to end",
    "start to finish",
    "from beginning",
    "from the beginning",
    "beginning to end",
];
const CHARACTER: &[&str] = &["who is", "whos", "describe", "tell me about", "who are"];
const FIND: &[&str] = &["where", "which scene", "find"];
const EXPLAIN: &[&str] = &[
    "explain",
    "what happens",
    "summarise",
    "summarize",
    "summary",
    "walk me through",
];
const DEEP: &[&str] = &[
    "in detail",
    "thoroughly",
    "analyse",
    "analyze",
    "analysis",
    "deep",
    "deeply",
    "in depth",
    "break down",
    "breakdown",
];
const QUICK: &[&str] = &[
    "briefly",
    "in a line",
    "in one sentence",
    "quick",
    "quickly",
];
const YES_NO: &[&str] = &[
    "is", "does", "did", "was", "are", "can", "has", "do", "were", "have",
];
const NOT_QUICK: &[&str] = &[
    "explain",
    "why",
    "how",
    "describe",
    "summarise",
    "summarize",
];
const PRONOUNS: &[&str] = &[
    "HE", "SHE", "THEY", "HIM", "HER", "THEM", "HIS", "HERS", "THEIR", "THEIRS",
];

fn intent_of(plain: &str, characters: usize) -> Intent {
    if has_any(plain, COMPARE) {
        Intent::Compare
    } else if characters >= 2 && has_any(plain, RELATIONSHIP) {
        Intent::Relationship
    } else if has_any(plain, THEME) {
        Intent::Theme
    } else if has_any(plain, CRITIQUE) {
        Intent::Critique
    } else if has_any(plain, IDEAS) {
        Intent::Ideas
    } else if has_any(plain, LIST_COUNT) {
        Intent::ListOrCount
    } else if has_any(plain, WHOLE) {
        Intent::Whole
    } else if characters == 1 && has_any(plain, CHARACTER) {
        Intent::Character
    } else if has_any(plain, FIND) {
        Intent::Find
    } else if has_any(plain, EXPLAIN) {
        Intent::Explain
    } else {
        Intent::Other
    }
}

/// An explicit word about length wins; then deep requests; then short factual questions.
fn depth_of(plain: &str, intent: Intent, whole_range: bool) -> Depth {
    if has_any(plain, QUICK) {
        return Depth::Quick;
    }
    if has_any(plain, DEEP) || intent == Intent::Compare || whole_range {
        return Depth::Deep;
    }
    let ws: Vec<&str> = plain.split_whitespace().collect();
    let first = ws.first().copied().unwrap_or("");
    // "Can you explain ..." is a request, not a yes or no question.
    let yes_no = YES_NO.contains(&first) && !matches!(ws.get(1), Some(&"you" | &"u"));
    let short_wh = matches!(
        first,
        "who" | "whos" | "where" | "wheres" | "when" | "whens"
    ) && ws.len() < 10;
    if (yes_no || short_wh) && !has_any(plain, NOT_QUICK) {
        Depth::Quick
    } else {
        Depth::Explain
    }
}

// ------------------------------------------------------------------ understand

struct Candidate {
    words: Vec<String>,
    kind: EntityKind,
    id: String,
    label: String,
}

pub(crate) fn understand(
    c: &Connection,
    draft: &DraftRef,
    scope: &ResolvedScope,
    text: &str,
    previous: &[Entity],
) -> AppResult<Understood> {
    let raw: Vec<&str> = text
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let tokens = words(text);
    let rows = queries::scenes(c, &draft.id)?;
    let names = character_names(c, &draft.id)?;
    let char_aliases = aliases(&names);
    let locations = location_names(c, &rows)?;

    let mut candidates: Vec<Candidate> = Vec::new();
    for a in &char_aliases {
        candidates.push(Candidate {
            words: a.words.clone(),
            kind: EntityKind::Character,
            id: names[a.idx].key.clone(),
            label: names[a.idx].label.clone(),
        });
    }
    for l in &locations {
        for p in l.phrases.iter().filter(|p| letters(p) >= 4) {
            candidates.push(Candidate {
                words: p.clone(),
                kind: EntityKind::Location,
                id: l.key.clone(),
                label: l.label.clone(),
            });
        }
    }
    // Longest names first, so "VENKAT RAO" is taken whole before "RAO" could match alone.
    candidates.sort_by_key(|k| std::cmp::Reverse((k.words.len(), letters(&k.words))));

    let mut consumed = vec![false; tokens.len()];
    let mut hits: Vec<(usize, EntityKind, String, String)> = Vec::new();
    for cand in &candidates {
        let m = cand.words.len();
        let mut i = 0;
        while i + m <= tokens.len() {
            if tokens[i..i + m] == cand.words[..] && !consumed[i..i + m].iter().any(|x| *x) {
                consumed[i..i + m].iter_mut().for_each(|x| *x = true);
                hits.push((i, cand.kind, cand.id.clone(), cand.label.clone()));
                i += m;
            } else {
                i += 1;
            }
        }
    }

    // A word that is one letter off exactly one character name word.
    let all_name_words: HashSet<&str> = names
        .iter()
        .flat_map(|n| n.words.iter().map(String::as_str))
        .collect();
    let mut fuzzy = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        if consumed[i] || t.chars().count() < 5 || all_name_words.contains(t.as_str()) {
            continue;
        }
        let near: BTreeSet<usize> = char_aliases
            .iter()
            .filter(|a| a.words.len() == 1 && a.words[0].chars().count() >= 5)
            .filter(|a| within_one(t, &a.words[0]))
            .map(|a| a.idx)
            .collect();
        if let [idx] = near.into_iter().collect::<Vec<_>>()[..] {
            consumed[i] = true;
            fuzzy.push((raw[i].to_string(), names[idx].label.clone()));
            hits.push((
                i,
                EntityKind::Character,
                names[idx].key.clone(),
                names[idx].label.clone(),
            ));
        }
    }
    hits.sort_by_key(|h| h.0);
    let mut entities: Vec<Entity> = Vec::new();
    for (_, kind, id, label) in hits {
        if !entities.iter().any(|e| e.kind == kind && e.id == id) {
            entities.push(Entity { kind, id, label });
        }
    }

    // Pronouns: only when no character is named, and only when one character came before.
    let mut pronoun_unresolved = false;
    let named = entities.iter().any(|e| e.kind == EntityKind::Character);
    if !named && tokens.iter().any(|t| PRONOUNS.contains(&t.as_str())) {
        let mut before: Vec<&Entity> = Vec::new();
        for e in previous.iter().filter(|e| e.kind == EntityKind::Character) {
            if !before.iter().any(|b| b.id == e.id) {
                before.push(e);
            }
        }
        match before[..] {
            [one] => entities.push(one.clone()),
            [] => {}
            _ => pronoun_unresolved = true,
        }
    }

    // Scenes.
    let live: BTreeSet<usize> = rows
        .iter()
        .filter(|s| !s.omitted)
        .map(|s| s.number)
        .collect();
    let mut asked: BTreeSet<usize> = openframe_ai_checks::scene_refs(text)
        .into_iter()
        .map(|n| n as usize)
        .collect();
    let p = plain(text);
    if has_any(&p, &["this scene", "the current scene", "current scene"])
        && let Some(s) = scope.scene.as_ref()
        && scope.draft.as_ref().is_some_and(|d| d.id == draft.id)
    {
        asked.insert(s.number);
    }
    if has_any(&p, &["the opening", "opening scene", "first scene"])
        && let Some(n) = live.first()
    {
        asked.insert(*n);
    }
    if has_any(
        &p,
        &["last scene", "final scene", "the ending", "closing scene"],
    ) && let Some(n) = live.last()
    {
        asked.insert(*n);
    }
    let scenes: Vec<usize> = asked.iter().copied().filter(|n| live.contains(n)).collect();
    let missing_scenes: Vec<usize> = asked
        .iter()
        .copied()
        .filter(|n| !live.contains(n))
        .collect();

    // Acts, by their order on the Beat Board.
    let story_acts = acts_of(c, draft)?;
    let mut acts: Vec<Entity> = Vec::new();
    for n in act_refs(&tokens) {
        let n = if n == usize::MAX { story_acts.len() } else { n };
        if let Some((id, title)) = story_acts.get(n.wrapping_sub(1))
            && !acts.iter().any(|a| &a.id == id)
        {
            acts.push(Entity {
                kind: EntityKind::Act,
                id: id.clone(),
                label: title.clone(),
            });
        }
    }

    let characters = entities
        .iter()
        .filter(|e| e.kind == EntityKind::Character)
        .count();
    let intent = intent_of(&p, characters);
    let whole_range = intent == Intent::Whole || has_any(&p, WHOLE);
    let depth = depth_of(&p, intent, whole_range);
    Ok(Understood {
        entities,
        scenes,
        missing_scenes,
        acts,
        depth,
        intent,
        fuzzy,
        pronoun_unresolved,
        whole_range,
    })
}

#[cfg(test)]
mod tests;

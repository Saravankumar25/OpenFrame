//! Reference resolution (spec section 5.2): which things in the project does a message name?
//! A pure function of the index, the focus and the message. No database, no model, no clock.
//!
//! Order of work: scene numbers ("scene 42"), screenplay titles and aliases, other names (longest
//! phrase first, exact before typo-tolerant), then pronouns from the focus. When a message needs a
//! screenplay and does not name one, the screenplay rules run in this order: (a) named in the
//! message, (b) the screenplay of the conversation focus, (c) the only screenplay with that many
//! scenes, (d) the only screenplay with a current draft, (e) one specific question, asked only when
//! the answer needs exactly one screenplay: a search, a list or a count across scenes reads them all.

use std::collections::BTreeMap;

use super::{EntityIndex, Focus, Kind, Rank, ScreenplayInfo};
use crate::modules::ai::queries::{could_not_find, join_and, join_or};
use crate::modules::ai::typo::{COMMON, distance};

/// How a mention was matched.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum How {
    /// The full name or title as written.
    Exact,
    /// A linked cue, an "also called" name, a part of a name, "the teaser", "day 4".
    Alias,
    /// Written with a typo; the score is 0 to 1.
    Fuzzy(f32),
    /// Taken from the conversation focus (a tie broken by it, or the screenplay rule (b)).
    Focus,
    /// "she", "him", "that scene": taken from the focus.
    Pronoun,
    /// The only screenplay with that many scenes (rule c).
    SceneCount,
    /// The only screenplay with a current draft (rule d).
    CurrentDraft,
    /// The project has one screenplay.
    OnlyOne,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub id: String,
    pub name: String,
    pub how: How,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneHit {
    pub screenplay_id: String,
    /// 1-based place in the draft.
    pub ordinal: u32,
    pub scene_id: Option<String>,
    pub heading: Option<String>,
    pub how: How,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub text: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Resolved {
    /// Normally zero or one. More than one when the message names several.
    pub screenplays: Vec<Hit>,
    /// Scenes of the first screenplay.
    pub scenes: Vec<SceneHit>,
    pub persons: Vec<Hit>,
    pub characters: Vec<Hit>,
    pub locations: Vec<Hit>,
    pub acts: Vec<Hit>,
    pub days: Vec<Hit>,
    pub files: Vec<Hit>,
    pub budget_lines: Vec<Hit>,
    /// Cards, Idea Vault items and call sheets.
    pub others: Vec<(Kind, Hit)>,
    /// Things asked for that the project does not have ("scene 46": the screenplay has 45).
    pub not_found: Vec<String>,
    /// One specific question, when the message cannot be settled by the rules.
    pub ambiguous: Option<Question>,
}

impl Resolved {
    pub fn screenplay_id(&self) -> Option<&str> {
        self.screenplays.first().map(|h| h.id.as_str())
    }

    fn bucket(&mut self, kind: Kind, hit: Hit) {
        let list = match kind {
            Kind::Character => &mut self.characters,
            Kind::Person => &mut self.persons,
            Kind::Location => &mut self.locations,
            Kind::Act => &mut self.acts,
            Kind::Day => &mut self.days,
            Kind::File => &mut self.files,
            Kind::BudgetLine => &mut self.budget_lines,
            Kind::Scene => return,
            Kind::Card | Kind::VaultItem | Kind::CallSheet => {
                if !self.others.iter().any(|(_, h)| h.id == hit.id) {
                    self.others.push((kind, hit));
                }
                return;
            }
        };
        if !list.iter().any(|h| h.id == hit.id) {
            list.push(hit);
        }
    }

    fn any_entity(&self) -> bool {
        !(self.persons.is_empty()
            && self.characters.is_empty()
            && self.locations.is_empty()
            && self.acts.is_empty()
            && self.days.is_empty()
            && self.files.is_empty()
            && self.budget_lines.is_empty()
            && self.others.is_empty())
    }
}

/// Lower case, letters and digits only, one space between words. "Mariamma's House" is
/// "mariamma house": a possessive and an inner apostrophe disappear.
pub fn normalize(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' || c == '\u{2019}' {
            let after_letter = i > 0 && chars[i - 1].is_alphanumeric();
            let possessive = chars
                .get(i + 1)
                .is_some_and(|n| n.eq_ignore_ascii_case(&'s'))
                && chars.get(i + 2).is_none_or(|n| !n.is_alphanumeric());
            i += if after_letter && possessive { 2 } else { 1 };
            continue;
        }
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else {
            out.push(' ');
        }
        i += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn words(s: &str) -> Vec<String> {
    normalize(s)
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Edits allowed for a phrase of this many letters: none under five, one up to nine, two beyond.
fn allowed_edits(chars: usize) -> Option<usize> {
    match chars {
        0..=4 => None,
        5..=9 => Some(1),
        _ => Some(2),
    }
}

/// 0 to 1, higher is closer; `None` when the two are not a typo apart.
fn typo_score(typed: &str, name: &str) -> Option<f32> {
    let n = typed.chars().count();
    let max = allowed_edits(n)?;
    if typed.chars().next() != name.chars().next() {
        return None;
    }
    let d = distance(typed, name, max).filter(|d| *d > 0)?;
    Some(1.0 - d as f32 / n.max(name.chars().count()) as f32)
}

struct SceneRef {
    raw: String,
    /// Plain number: the place in the draft. A number with a letter ("12A") is a production number.
    number: Option<u32>,
}

fn scene_number(s: &str) -> bool {
    let digits = s.chars().take_while(char::is_ascii_digit).count();
    (1..=4).contains(&digits) && s.chars().count() - digits <= 1
}

/// "scene 42", "scenes 3 and 41", "scene 12A".
fn scene_mentions(w: &[String], used: &mut [bool]) -> Vec<SceneRef> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < w.len() {
        if matches!(w[i].as_str(), "scene" | "scenes" | "sc")
            && w.get(i + 1).is_some_and(|n| scene_number(n))
        {
            used[i] = true;
            let mut j = i + 1;
            while j < w.len() {
                if scene_number(&w[j]) {
                    used[j] = true;
                    let digits: String = w[j].chars().take_while(char::is_ascii_digit).collect();
                    out.push(SceneRef {
                        raw: w[j].to_uppercase(),
                        number: (digits.len() == w[j].len()).then(|| digits.parse().unwrap_or(0)),
                    });
                    j += 1;
                } else if w[j] == "and" && w.get(j + 1).is_some_and(|n| scene_number(n)) {
                    used[j] = true;
                    j += 1;
                } else {
                    break;
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

struct SpMention {
    index: usize,
    start: usize,
    end: usize,
    how: How,
}

/// Screenplays named by title or alias. Typos are tolerated in a phrase of six letters or more.
fn screenplay_mentions(ix: &EntityIndex, w: &[String], used: &mut [bool]) -> Vec<Hit> {
    let mut found: Vec<(SpMention, f32)> = Vec::new();
    for (index, sp) in ix.screenplays.iter().enumerate() {
        let mut best: Option<(SpMention, f32)> = None;
        let phrases = std::iter::once((&sp.title, Rank::Name))
            .chain(sp.aliases.iter().map(|a| (a, Rank::Alias)));
        for (phrase, rank) in phrases {
            let pw = words(phrase);
            let n = pw.len();
            if n == 0 || n > w.len() {
                continue;
            }
            for start in 0..=w.len() - n {
                if used[start..start + n].iter().any(|u| *u) {
                    continue;
                }
                let span = &w[start..start + n];
                let (score, how) = if span == pw.as_slice() {
                    (
                        rank.score(),
                        if rank == Rank::Name {
                            How::Exact
                        } else {
                            How::Alias
                        },
                    )
                } else if let Some(s) = typo_score(&span.join(" "), &pw.join(" "))
                    .filter(|_| pw.join(" ").chars().count() >= 6)
                {
                    (rank.score() * s, How::Fuzzy(rank.score() * s))
                } else {
                    continue;
                };
                let better = best
                    .as_ref()
                    .is_none_or(|(b, bs)| score > *bs || (score == *bs && n > b.end - b.start));
                if better {
                    best = Some((
                        SpMention {
                            index,
                            start,
                            end: start + n,
                            how,
                        },
                        score,
                    ));
                }
            }
        }
        found.extend(best);
    }
    // "low tide radio teaser": the feature's title sits inside the teaser's, and the extra word
    // belongs to the teaser. Drop a screenplay whose title words are inside another hit's title when
    // the two mentions touch.
    let title_words: Vec<Vec<String>> = ix.screenplays.iter().map(|s| words(&s.title)).collect();
    let touching = |a: &SpMention, b: &SpMention| a.start <= b.end && b.start <= a.end;
    let dropped: Vec<usize> = found
        .iter()
        .filter(|(s, _)| {
            found.iter().any(|(t, _)| {
                t.index != s.index
                    && title_words[s.index].len() < title_words[t.index].len()
                    && title_words[s.index]
                        .iter()
                        .all(|x| title_words[t.index].contains(x))
                    && touching(s, t)
            })
        })
        .map(|(s, _)| s.index)
        .collect();
    found.retain(|(s, _)| !dropped.contains(&s.index));
    found.sort_by_key(|(s, _)| s.start);
    found
        .into_iter()
        .map(|(m, _)| {
            used[m.start..m.end].iter_mut().for_each(|u| *u = true);
            let sp = &ix.screenplays[m.index];
            Hit {
                id: sp.id.clone(),
                name: sp.title.clone(),
                how: m.how,
            }
        })
        .collect()
}

/// A word that is never a name on its own.
fn filler(w: &str) -> bool {
    w.chars().count() < 3
        || matches!(
            w,
            "the"
                | "and"
                | "for"
                | "who"
                | "how"
                | "any"
                | "are"
                | "was"
                | "not"
                | "all"
                | "has"
                | "with"
                | "about"
                | "from"
                | "this"
                | "that"
                | "what"
                | "when"
                | "where"
                | "which"
                | "there"
        )
}

const MAX_PHRASE: usize = 6;

/// Names of everything except screenplays and scenes. Longest phrase first, so "young anjali" is
/// not read as "anjali"; at one length, an exact name before a typo.
fn entity_mentions(
    ix: &EntityIndex,
    focus: &Focus,
    w: &[String],
    used: &mut [bool],
    out: &mut Resolved,
) {
    for n in (1..=MAX_PHRASE.min(w.len())).rev() {
        for start in 0..=w.len() - n {
            if used[start..start + n].iter().any(|u| *u) {
                continue;
            }
            let span = &w[start..start + n];
            if n == 1 && filler(&span[0]) {
                continue;
            }
            let phrase = span.join(" ");
            // (entity position, score, rank, typo)
            let mut cands: Vec<(usize, f32, Rank, bool)> = ix
                .phrase(&phrase)
                .unwrap_or_default()
                .iter()
                .map(|(i, r)| (*i, r.score(), *r, false))
                .collect();
            let typed_digits = span.iter().any(|x| x.chars().all(|c| c.is_ascii_digit()));
            let all_common = span.iter().all(|x| COMMON.contains(&x.as_str()));
            if cands.is_empty() && n <= 4 && !typed_digits && !all_common {
                for (name, words_in, i, rank) in ix.fuzzy_pool() {
                    if *words_in == n
                        && let Some(s) = typo_score(&phrase, name)
                    {
                        cands.push((*i, rank.score() * s, *rank, true));
                    }
                }
            }
            if cands.is_empty() {
                continue;
            }
            used[start..start + n].iter_mut().for_each(|u| *u = true);
            let mut by_kind: BTreeMap<Kind, Vec<(usize, f32, Rank, bool)>> = BTreeMap::new();
            for c in cands {
                by_kind.entry(ix.entities[c.0].kind).or_default().push(c);
            }
            for (kind, list) in by_kind {
                let best = list.iter().map(|c| c.1).fold(0.0_f32, f32::max);
                let mut tied: Vec<_> = list.into_iter().filter(|c| c.1 >= best - 1e-6).collect();
                tied.dedup_by_key(|c| c.0);
                let pick = if tied.len() == 1 {
                    Some((tied[0], false))
                } else {
                    let in_focus: Vec<_> = tied
                        .iter()
                        .filter(|c| focus.contains(&ix.entities[c.0].id))
                        .collect();
                    (in_focus.len() == 1).then(|| (*in_focus[0], true))
                };
                match pick {
                    Some(((i, score, rank, typo), by_focus)) => {
                        let e = &ix.entities[i];
                        let how = if by_focus {
                            How::Focus
                        } else if typo {
                            How::Fuzzy(score)
                        } else if rank == Rank::Name {
                            How::Exact
                        } else {
                            How::Alias
                        };
                        out.bucket(
                            kind,
                            Hit {
                                id: e.id.clone(),
                                name: e.name.clone(),
                                how,
                            },
                        );
                    }
                    None if out.ambiguous.is_none() => {
                        let names: Vec<&str> = tied
                            .iter()
                            .map(|c| ix.entities[c.0].name.as_str())
                            .collect();
                        let same_name = names
                            .iter()
                            .any(|a| names.iter().filter(|b| *b == a).count() > 1);
                        let options: Vec<String> = tied
                            .iter()
                            .map(|c| {
                                let e = &ix.entities[c.0];
                                if same_name && !e.detail.is_empty() {
                                    format!("{} ({})", e.name, e.detail)
                                } else {
                                    e.name.clone()
                                }
                            })
                            .collect();
                        out.ambiguous = Some(Question {
                            text: format!(
                                "Which {} do you mean: {}?",
                                kind.label(),
                                join_or(&options)
                            ),
                            options,
                        });
                    }
                    None => {}
                }
            }
        }
    }
}

const PRONOUNS: [&str; 11] = [
    "she", "he", "her", "hers", "him", "his", "they", "them", "their", "herself", "himself",
];

/// Words that mean the message is about a screenplay.
const NEEDS_SCREENPLAY: &[&str] = &[
    "screenplay",
    "screenplays",
    "script",
    "scripts",
    "draft",
    "drafts",
    "scene",
    "scenes",
    "dialogue",
    "dialogues",
    "heading",
    "headings",
    "cue",
    "cues",
];

fn quoted(titles: impl Iterator<Item = String>) -> Vec<String> {
    titles.map(|t| format!("\u{201C}{t}\u{201D}")).collect()
}

fn hit(sp: &ScreenplayInfo, how: How) -> Hit {
    Hit {
        id: sp.id.clone(),
        name: sp.title.clone(),
        how,
    }
}

/// A search, a list or a count across scenes: it reads every screenplay and says which screenplay
/// each hit is in, so it never needs the one question of rule (e).
fn across_screenplays(w: &[String]) -> bool {
    let has = |any: &[&str]| w.iter().any(|t| any.contains(&t.as_str()));
    !has(&[
        "open",
        "explain",
        "describe",
        "summarise",
        "summarize",
        "recap",
        "read",
    ]) && (has(&[
        "mention",
        "mentions",
        "mentioning",
        "mentioned",
        "contain",
        "contains",
        "containing",
        "search",
        "find",
        "list",
        "count",
    ]) || (has(&["which", "many", "all", "every", "each", "total"])
        && has(&[
            "scenes",
            "drafts",
            "screenplays",
            "scripts",
            "dialogue",
            "headings",
        ])))
}

/// The screenplay rules (b) to (e), for a message that needs a screenplay and names none. `across`
/// is a message that reads every screenplay: rule (e) does not ask.
fn choose_screenplay(ix: &EntityIndex, focus: &Focus, most: u32, across: bool, out: &mut Resolved) {
    let all = &ix.screenplays;
    match all.as_slice() {
        [] => return,
        [only] => {
            out.screenplays.push(hit(only, How::OnlyOne));
            return;
        }
        _ => {}
    }
    let fitting: Vec<&ScreenplayInfo> = all.iter().filter(|s| s.scene_count >= most).collect();
    // (b) The conversation focus. When it lacks the scene asked for and exactly one other
    // screenplay has it, the scene number is the better evidence: fall through to (c).
    if let Some(sp) = focus.screenplay.as_deref().and_then(|id| ix.screenplay(id))
        && (sp.scene_count >= most || fitting.len() != 1)
    {
        out.screenplays.push(hit(sp, How::Focus));
        return;
    }
    // (c) Only one screenplay has that many scenes.
    if most > 0 {
        match fitting.as_slice() {
            [one] => {
                out.screenplays.push(hit(one, How::SceneCount));
                return;
            }
            [] => {
                let have = join_and(
                    &all.iter()
                        .map(|s| {
                            format!(
                                "\u{201C}{}\u{201D} ({})",
                                s.title,
                                scenes_word(s.scene_count)
                            )
                        })
                        .collect::<Vec<_>>(),
                );
                out.not_found
                    .push(could_not_find(&format!("scene {most}"), &have));
                return;
            }
            _ => {}
        }
    }
    // (d) Only one screenplay holds a current draft.
    let candidates: Vec<&ScreenplayInfo> = if most > 0 {
        fitting
    } else {
        all.iter().collect()
    };
    let current: Vec<&&ScreenplayInfo> =
        candidates.iter().filter(|s| s.has_current_draft).collect();
    if let [one] = current.as_slice() {
        out.screenplays.push(hit(one, How::CurrentDraft));
        return;
    }
    // (e) Ask, unless the message reads every screenplay.
    if across {
        return;
    }
    let options = quoted(candidates.iter().map(|s| s.title.clone()));
    out.ambiguous = Some(Question {
        text: format!("Which screenplay do you mean: {}?", join_or(&options)),
        options,
    });
}

/// "I could not find scene 46. I looked in “T”, which has 45 scenes."
pub fn scene_not_found(n: u32, title: &str, scene_count: u32) -> String {
    could_not_find(
        &format!("scene {n}"),
        &format!(
            "\u{201C}{title}\u{201D}, which has {}",
            scenes_word(scene_count)
        ),
    )
}

fn scenes_word(n: u32) -> String {
    if n == 1 {
        "1 scene".into()
    } else {
        format!("{n} scenes")
    }
}

/// Which things in the project does `message` name? See the module comment for the rules.
pub fn resolve(ix: &EntityIndex, focus: &Focus, message: &str) -> Resolved {
    let w = words(message);
    let mut used = vec![false; w.len()];
    let mut out = Resolved::default();
    let scene_refs = scene_mentions(&w, &mut used);
    let named = screenplay_mentions(ix, &w, &mut used);
    entity_mentions(ix, focus, &w, &mut used, &mut out);

    // Pronouns take the focus, unless the message names someone itself.
    let pronoun = (0..w.len()).any(|i| !used[i] && PRONOUNS.contains(&w[i].as_str()));
    if pronoun && out.characters.is_empty() && out.persons.is_empty() {
        let (ids, kind) = if focus.characters.is_empty() {
            (&focus.persons, Kind::Person)
        } else {
            (&focus.characters, Kind::Character)
        };
        for e in ids.iter().filter_map(|id| ix.entity(id)) {
            out.bucket(
                kind,
                Hit {
                    id: e.id.clone(),
                    name: e.name.clone(),
                    how: How::Pronoun,
                },
            );
        }
    }
    let that_scene = w
        .windows(2)
        .any(|p| matches!(p[0].as_str(), "that" | "this") && p[1] == "scene")
        || (scene_refs.is_empty()
            && !out.any_entity()
            && named.is_empty()
            && w.iter().any(|x| x == "it"));
    let scene_pronoun = that_scene && scene_refs.is_empty() && focus.scene.is_some();

    out.screenplays = named;
    if out.screenplays.is_empty() {
        let needs = !scene_refs.is_empty()
            || scene_pronoun
            || w.iter().any(|x| NEEDS_SCREENPLAY.contains(&x.as_str()));
        if needs {
            let most = scene_refs
                .iter()
                .filter_map(|r| r.number)
                .max()
                .unwrap_or(0);
            let across = scene_refs.is_empty() && !scene_pronoun && across_screenplays(&w);
            choose_screenplay(ix, focus, most, across, &mut out);
        }
    }

    let Some(sp) = out.screenplays.first().and_then(|h| ix.screenplay(&h.id)) else {
        return out;
    };
    let title = format!("\u{201C}{}\u{201D}", sp.title);
    for r in scene_refs {
        let (found, miss) = match r.number {
            Some(n) if (1..=sp.scene_count).contains(&n) => {
                (ix.scene(&sp.id, n).map(|e| (n, e)), String::new())
            }
            Some(n) => (None, scene_not_found(n, &sp.title, sp.scene_count)),
            None => {
                let by_number = ix.entities.iter().find(|e| {
                    e.kind == Kind::Scene
                        && e.screenplay_id.as_deref() == Some(sp.id.as_str())
                        && e.aliases.iter().any(|a| a.eq_ignore_ascii_case(&r.raw))
                });
                (
                    by_number.and_then(|e| e.ordinal.map(|n| (n, e))),
                    could_not_find(&format!("scene {}", r.raw), &title),
                )
            }
        };
        match found {
            Some((n, e)) => out.scenes.push(SceneHit {
                screenplay_id: sp.id.clone(),
                ordinal: n,
                scene_id: Some(e.id.clone()),
                heading: Some(e.name.clone()),
                how: How::Exact,
            }),
            None => out.not_found.push(miss),
        }
    }
    if scene_pronoun
        && focus.screenplay.as_deref() == Some(sp.id.as_str())
        && let Some(n) = focus.scene
        && let Some(e) = ix.scene(&sp.id, n)
    {
        out.scenes.push(SceneHit {
            screenplay_id: sp.id.clone(),
            ordinal: n,
            scene_id: Some(e.id.clone()),
            heading: Some(e.name.clone()),
            how: How::Pronoun,
        });
    }
    out
}

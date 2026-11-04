//! The out-of-section pointer (ADR-0041, decision 5). Code only, no model: the person picked where to
//! look, and a question that belongs elsewhere gets one sentence naming the section to add.
//!
//! Three things can fire it, strongest first:
//!
//! 1. **A name** the message holds that no picked section can answer about (`Dossier::outside`).
//! 2. **A topic word** that points at a section that is not picked (`turn::leaf_hints`). A word is only
//!    a hint: it fires when no hint points at a picked section, the picks' values do not hold the word
//!    and the picks returned nothing that answers. A topic word alone never overrides facts that were
//!    found in the picks.
//! 3. **A cheap existence check** over the search index, when the picks returned nothing that answers
//!    and no word pointed anywhere.
//!
//! Opening something and asking for a change go the same way (ADR-0041 decision 7): what is opened must
//! live in the picks, and a change needs a tool homed in them.
//!
//! The reply names a section, never content. Permission comes first, so the pointer never tells a
//! person that something they may not read exists: contact details below their role and another
//! person's private note give the same "I could not find X. I looked in A and B." as a thing the
//! project does not hold at all.

use std::sync::LazyLock;

use openframe_domain::Actor;
use regex::Regex;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

use super::answer::language_of;
use super::commands;
use super::dossier::{self, Dossier, Fact};
use super::index::{Hit, How, Kind};
use super::queries;
use super::scope::ResolvedScope;
use super::sections::{LEAVES, Leaf, Selection, leaf};
use super::toolbox::{self, homes};
use super::turn::{Group, LeafHint, leaf_hints};
use crate::core::AppCore;

/// What fired the pointer, written to the trace.
pub mod trigger {
    pub const NAME: &str = "name";
    pub const TOPIC: &str = "topic";
    pub const EXISTS: &str = "exists";
    pub const HELP: &str = "help";
    pub const TOOL: &str = "tool";
    pub const OPEN: &str = "open";
}

/// The section a reply points at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pointer {
    pub leaf: &'static Leaf,
    pub trigger: &'static str,
}

impl Pointer {
    pub fn of(leaf: &'static Leaf, trigger: &'static str) -> Self {
        Self { leaf, trigger }
    }

    /// The fixed sentence: "That is in Production > Budget. Add it to your selection and ask again."
    pub fn sentence(&self) -> String {
        format!(
            "That is in {}. Add it to your selection and ask again.",
            self.leaf.path()
        )
    }

    /// `{"leaf": "production/budget", "label": "Production > Budget"}`, read by the screens.
    pub fn structured(&self) -> Value {
        json!({ "leaf": self.leaf.id, "label": self.leaf.path() })
    }
}

/// What code decided before the writer runs.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Verdict {
    /// Nothing to say: the writer answers from the facts.
    #[default]
    Write,
    /// The whole question lives elsewhere: this sentence and no model.
    Point(Pointer),
    /// Part of the question fits the picks: the writer answers it, then the sentence follows.
    Partial(Pointer),
    /// The project holds nothing the person may read for this: this sentence and no model.
    NotFound(String),
}

impl Verdict {
    pub fn pointer(&self) -> Option<Pointer> {
        match self {
            Verdict::Point(p) | Verdict::Partial(p) => Some(*p),
            _ => None,
        }
    }
}

// ------------------------------------------------------------------ does a fact answer

/// Words that say nothing about what is asked.
const STOP: &[&str] = &[
    "a", "about", "after", "all", "also", "am", "an", "and", "any", "are", "as", "at", "be",
    "been", "before", "by", "can", "could", "did", "do", "does", "for", "from", "get", "give",
    "going", "had", "has", "have", "he", "her", "him", "his", "how", "i", "in", "is", "it", "its",
    "list", "many", "me", "much", "my", "need", "needs", "of", "on", "or", "our", "please", "say",
    "says", "scene", "scenes", "she", "should", "show", "some", "tell", "that", "the", "their",
    "them", "there", "these", "they", "this", "those", "to", "today", "us", "use", "used", "was",
    "we", "were", "what", "when", "where", "which", "who", "whose", "why", "will", "with", "would",
    "you", "your",
];

/// Field names that say what kind of value follows, not what the value is about.
const PLAIN_FIELDS: &[&str] = &[
    "detail",
    "heading",
    "name",
    "note",
    "notes",
    "role",
    "status",
    "summary",
    "text",
    "title",
    "type",
    "value",
    "what",
    "where",
    "writer's note",
];

fn stem(word: &str) -> String {
    let w = word.to_lowercase();
    match w.strip_suffix('s') {
        Some(s) if s.chars().count() >= 3 && !s.ends_with('s') => s.to_string(),
        _ => w,
    }
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(stem)
        .collect()
}

/// A statement of what a section does not hold speaks of what was asked: a word of the question is in
/// it, the words of the fixed sentence not counted.
pub(crate) fn gap_concerns(gap: &str, asked: &[String]) -> bool {
    const FRAME: &[&str] = &["could", "not", "find", "looked"];
    words(gap)
        .iter()
        .any(|w| asked.contains(w) && !FRAME.contains(&w.as_str()))
}

/// What the message asks about: its words without the filler, the numbers and the possessive "s".
pub(crate) fn content_words(message: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in message.split(|c: char| !c.is_alphanumeric()) {
        let lower = raw.to_lowercase();
        let keep = lower.chars().count() >= 2
            && !lower.chars().all(|c| c.is_ascii_digit())
            && !STOP.contains(&lower.as_str());
        let w = stem(&lower);
        if keep && !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

/// The words of a question to look for in a fact: `content_words`, with "scene" kept, since a fact about
/// scenes says so.
pub(crate) fn asked_words(message: &str) -> Vec<String> {
    let mut out = content_words(message);
    if words(message).iter().any(|w| w == "scene") {
        out.push("scene".to_string());
    }
    out
}

/// A word of the question is in the fact.
pub(crate) fn speaks_of(f: &Fact, asked: &[String]) -> bool {
    words_held(f, asked) > 0
}

/// A record of the project, not a note about what was assumed or the bare project title.
pub(crate) fn real(f: &Fact) -> bool {
    !f.is_missing() && f.kind != "Assumed" && (f.kind != "Project" || f.field("logline").is_some())
}

/// A passage of the text search: it was found by any one word of the message, so it says less.
fn passage(f: &Fact) -> bool {
    f.kind == "Found"
}

fn words_held(f: &Fact, asked: &[String]) -> usize {
    let line = words(&f.render_plain());
    asked.iter().filter(|w| line.contains(w)).count()
}

/// One fact answers when it holds more than half of what is asked. A passage has to hold more than
/// two thirds of it: any one word finds a passage.
fn fact_answers(f: &Fact, asked: &[String]) -> bool {
    let held = words_held(f, asked);
    if passage(f) {
        held * 3 > asked.len() * 2
    } else {
        held * 2 > asked.len()
    }
}

/// The picks returned something that answers. A question with nothing to look for is answered by any
/// fact, and a fact with no word of it is the leaf's general reading, which answers nothing.
fn answered(facts: &[Fact], message: &str) -> bool {
    let asked = content_words(message);
    let mut real_facts = facts.iter().filter(|f| real(f));
    if asked.is_empty() {
        return real_facts.next().is_some();
    }
    real_facts.any(|f| fact_answers(f, &asked))
}

/// Half of what is asked is somewhere in the picks, passages counted. Not enough to say the question
/// is answered, but enough not to send the person away on the strength of a word search.
fn partly_there(facts: &[Fact], message: &str) -> bool {
    let asked = content_words(message);
    !asked.is_empty()
        && facts
            .iter()
            .filter(|f| real(f))
            .any(|f| words_held(f, &asked) * 2 >= asked.len())
}

/// What the facts speak of: their values and sentences, and the field names that name a thing
/// ("shoot days", "locations"). Passages are left out: they were found by the words themselves.
fn values_of(facts: &[Fact]) -> Vec<String> {
    let mut out = Vec::new();
    for f in facts.iter().filter(|f| real(f) && !passage(f)) {
        if let Some(s) = &f.sentence {
            out.extend(words(s));
        }
        for (name, value) in &f.fields {
            out.extend(words(value));
            if !PLAIN_FIELDS.contains(&name.as_str()) {
                out.extend(words(name));
            }
        }
    }
    out
}

/// A question form asks for a record of another section whatever the picks' words are: the "day" of a
/// scene heading is not the day it is shot.
const FORMS: &[&str] = &["which day", "what day", "who plays", "how much"];

/// The picks speak of the hint's topic: its words are in their values.
fn hint_held(h: &LeafHint, values: &[String]) -> bool {
    if FORMS.contains(&h.word.as_str()) {
        return false;
    }
    const FORM_WORDS: &[&str] = &["how", "many", "which", "the"];
    let lower = h.word.to_lowercase();
    let wanted: Vec<String> = lower
        .split_whitespace()
        .filter(|w| !FORM_WORDS.contains(w))
        .map(stem)
        .collect();
    !wanted.is_empty() && wanted.iter().all(|w| values.contains(w))
}

/// "As the director, brief me" and "for the casting director" say who is asking, not what is asked.
fn without_roles(message: &str) -> String {
    static ROLE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(?:as|for) (?:the|a|an|our|my)(?: \w+)? (?:director|producer|editor|writer|gaffer|cinematographer|actors?|cast|crew)\b",
        )
        .expect("valid pattern")
    });
    ROLE.replace_all(message, " ").into_owned()
}

fn hint_picked(h: &LeafHint, sel: &Selection) -> bool {
    h.leaves.iter().any(|l| sel.has(l))
}

// ------------------------------------------------------------------ the clauses of a message

static CLAUSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\s*(?:[,;]?\s+and\s+(?:then\s+)?(?:also\s+)?(?:what|who|whose|how|which|where|when|why|describe|tell|list|show|give)\b|[,;]\s*(?:what|who|whose|how|which|where|when|why|describe|tell|list|show|give)\b|[,;]?\s+also\b|[?;]\s+)",
    )
    .expect("valid pattern")
});

/// A message that asks two things has them apart: "Who plays Arjun, and what does the Camera cost?"
fn clauses(message: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut from = 0;
    for m in CLAUSE.find_iter(message) {
        let piece = message[from..m.start()].trim();
        if !piece.is_empty() {
            out.push(piece.to_string());
        }
        // The question word stays with the clause it opens.
        let joint = m.as_str();
        let word_at = joint
            .rfind(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '?')
            .map_or(0, |i| i + 1);
        from = m.start() + word_at;
    }
    let last = message[from..].trim();
    if !last.is_empty() {
        out.push(last.to_string());
    }
    out
}

// ------------------------------------------------------------------ the decision

fn asks_contact(message: &str) -> bool {
    static CONTACT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:phone|e-?mail|contact|mobile|whatsapp|number for|reach)\b")
            .expect("valid pattern")
    });
    CONTACT.is_match(message)
}

/// The sentence for "nothing the person may read": the words asked for, and the picks.
fn not_found(message: &str, sel: &Selection) -> Verdict {
    Verdict::NotFound(queries::could_not_find(
        &format!(
            "anything that matches \u{201C}{}\u{201D}",
            queries::truncate_chars(message.trim(), 80)
        ),
        &sel.looked_in(),
    ))
}

/// The section of a name that lives outside the picks: a section a word of the message points at and
/// that can answer about that kind of name, else the name's home.
fn home_of_name(kind: Kind, home: &'static str, hints: &[LeafHint]) -> &'static Leaf {
    for h in hints {
        for id in h.leaves {
            if let Some(l) = leaf(id)
                && l.takes(kind)
            {
                return l;
            }
        }
    }
    leaf(home).unwrap_or(&LEAVES[0])
}

/// A word points elsewhere when no word points at a pick and the picks' values do not hold it.
fn topic_target(hints: &[LeafHint], facts: &[Fact], sel: &Selection) -> Option<&'static Leaf> {
    if hints.is_empty() || hints.iter().any(|h| hint_picked(h, sel)) {
        return None;
    }
    let values = values_of(facts);
    hints
        .iter()
        .find(|h| !hint_held(h, &values))
        .and_then(|h| h.leaves.first())
        .and_then(|id| leaf(id))
}

/// The one clause of a two-part question that points elsewhere, when another clause is answered.
fn partial_topic(
    message: &str,
    facts: &[Fact],
    sel: &Selection,
    inside: &[String],
) -> Option<&'static Leaf> {
    let parts = clauses(message);
    if parts.len() < 2 {
        return None;
    }
    let mut target = None;
    let mut one_answered = false;
    for part in &parts {
        let hints = leaf_hints(&without_roles(part));
        // A clause about a thing the picks hold is theirs, whatever else it says.
        let lower = part.to_lowercase();
        let theirs = inside.iter().any(|n| lower.contains(n.as_str()));
        match topic_target(&hints, facts, sel) {
            Some(l) if !theirs && !answered(facts, part) => target = target.or(Some(l)),
            _ => {
                one_answered |= answered(facts, part) || hints.iter().any(|h| hint_picked(h, sel));
            }
        }
    }
    target.filter(|_| one_answered)
}

/// The best hit of the search index for all the words asked about, when it is outside the picks.
/// The owner rule of Global Search holds, so another person's private note is never found. A short
/// word, or one word alone, finds too much to send anyone anywhere.
fn exists_elsewhere(
    c: &Connection,
    actor: &Actor,
    sel: &Selection,
    message: &str,
) -> Option<&'static Leaf> {
    let terms: Vec<String> = content_words(message)
        .iter()
        .filter(|t| t.chars().count() >= 4)
        .take(8)
        .map(|t| format!("\"{}\"*", t.replace('"', "")))
        .collect();
    if terms.len() < 2 {
        return None;
    }
    let mut st = c
        .prepare(
            "SELECT d.nav_json FROM search_fts f JOIN search_doc d ON d.rowid = f.rowid
             WHERE search_fts MATCH ?1 AND (d.owner_user_id IS NULL OR d.owner_user_id = ?2)
             ORDER BY bm25(search_fts) LIMIT 5",
        )
        .ok()?;
    let rows: Vec<String> = st
        .query_map(params![terms.join(" AND "), actor.user_id], |r| r.get(0))
        .ok()?
        .filter_map(Result::ok)
        .collect();
    for nav in rows {
        let Ok(nav) = serde_json::from_str::<Value>(&nav) else {
            continue;
        };
        let Some(l) = LEAVES
            .iter()
            .find(|l| l.search.iter().any(|r| r.matches(&nav)))
        else {
            continue;
        };
        if sel.has(l.id) {
            return None;
        }
        if l.min_role.allows_contacts(actor) {
            return Some(l);
        }
    }
    None
}

/// Point at this section, unless the person may not read what was asked or it holds nothing they may
/// read. The look at the section runs its fetchers under the same guards as an answer, and what it
/// finds is only counted: it never reaches the reply or the prompt.
fn point_if_allowed(
    input: &dossier::Input<'_>,
    sel: &Selection,
    target: &'static Leaf,
    trigger: &'static str,
    partial: bool,
) -> Verdict {
    let blocked = asks_contact(input.message) && !target.min_role.allows_contacts(input.actor);
    let holds = trigger == trigger::NAME
        || trigger == trigger::EXISTS
        || dossier::probe(input, target).facts.iter().any(real);
    if blocked || !holds {
        return if partial {
            Verdict::Write
        } else {
            not_found(input.message, sel)
        };
    }
    let p = Pointer::of(target, trigger);
    if partial {
        Verdict::Partial(p)
    } else {
        Verdict::Point(p)
    }
}

/// The section a screen or a scene is on. Opening a workspace takes any section of its branch.
fn screen_leaf(workspace: &str, sub: Option<&str>) -> Option<&'static Leaf> {
    let key = match (workspace, sub) {
        ("trash", _) => "recentlyDeleted".to_string(),
        (w, Some(s)) => format!("{w}/{s}"),
        (w, None) => w.to_string(),
    };
    LEAVES.iter().find(|l| l.screens.contains(&key.as_str()))
}

/// "Open Scene 12", "Go to the budget" and "Open the Harbour Office": what is opened must live in the
/// picks. A scene is opened in the Screenplay, whichever section can read its number.
fn for_open(input: &dossier::Input<'_>, sel: &Selection, d: &Dossier) -> Verdict {
    let open_elsewhere = |l: &'static Leaf| Verdict::Point(Pointer::of(l, trigger::OPEN));
    let screenplay = leaf("screenplay/scenes").unwrap_or(&LEAVES[0]);
    if let Some(call) = commands::navigation_call(input.message, |_| true) {
        if call.tool == "open_scene" {
            return if sel.has(screenplay.id) {
                Verdict::Write
            } else {
                open_elsewhere(screenplay)
            };
        }
        let workspace = call.arguments["workspace"].as_str().unwrap_or_default();
        let sub = call.arguments["sub"].as_str();
        let Some(target) = screen_leaf(workspace, sub) else {
            return Verdict::Write;
        };
        let inside = sel
            .picks
            .iter()
            .any(|p| p.leaf.id == target.id || (sub.is_none() && p.leaf.branch == target.branch));
        return if inside {
            Verdict::Write
        } else {
            open_elsewhere(target)
        };
    }
    if !d.resolved.scenes.is_empty() && !sel.has(screenplay.id) {
        return open_elsewhere(screenplay);
    }
    let r = &d.resolved;
    let named_inside = !r.persons.is_empty()
        || !r.characters.is_empty()
        || !r.locations.is_empty()
        || !r.acts.is_empty()
        || !r.files.is_empty()
        || !r.days.is_empty()
        || !r.budget_lines.is_empty()
        || !r.others.is_empty();
    if named_inside {
        return Verdict::Write;
    }
    match d
        .outside
        .iter()
        .find(|o| input.index.screenplay(&o.id).is_none())
    {
        Some(o) => open_elsewhere(leaf(o.home).unwrap_or(&LEAVES[0])),
        None => Verdict::Write,
    }
}

/// A change whose tool lives outside the picks. A plain command names its tool outright ("Create a
/// task to scout the pier"); anything else goes by the words of the tool names. A tool may need two
/// sections: a breakdown suggestion reads the scene's text too.
pub fn for_change(
    core: &AppCore,
    actor: &Actor,
    scope: &ResolvedScope,
    message: &str,
) -> Option<Pointer> {
    let id = match commands::change_command(message, |_| true) {
        Some(call) => homes::missing_leaf(&call.tool, &scope.sections)?,
        None => toolbox::home_of_change(core, actor, scope, message)?,
    };
    Some(Pointer::of(leaf(id)?, trigger::TOOL))
}

/// Decide before the writer runs. `d` is the dossier of the picks, `input` what it was built from.
pub fn decide(input: &dossier::Input<'_>, sel: &Selection, d: &Dossier) -> Verdict {
    let group = input.read.group;
    if group == Group::Navigate {
        return for_open(input, sel, d);
    }
    if !matches!(
        group,
        Group::Facts | Group::Narrative | Group::Brainstorm | Group::Help
    ) {
        return Verdict::Write;
    }
    // The help sections answer how-to questions whatever they name.
    if group == Group::Help && (sel.has("help/guide") || sel.has("help/settings")) {
        return Verdict::Write;
    }
    let message = input.message;
    let hints = leaf_hints(&without_roles(message));
    let two_parts = clauses(message).len() > 1;
    let topic_is_picked = hints.iter().any(|h| hint_picked(h, sel));
    // A question form ("who plays", "which day") asks for a record, and a passage that shares its words
    // is not one. With no such form, a passage can answer.
    let form_elsewhere = !topic_is_picked && hints.iter().any(|h| h.word.contains(' '));
    let found = if form_elsewhere {
        let records: Vec<Fact> = d.facts.iter().filter(|f| !passage(f)).cloned().collect();
        answered(&records, message)
    } else {
        answered(&d.facts, message)
    };
    let values = values_of(&d.facts);
    // The question names a person, a character, a place or a thing the picks can answer about.
    let named = |h: &Hit| matches!(h.how, How::Exact | How::Alias | How::Fuzzy(_));
    let r = &d.resolved;
    let inside: Vec<String> = [
        &r.persons,
        &r.characters,
        &r.locations,
        &r.acts,
        &r.files,
        &r.budget_lines,
    ]
    .iter()
    .flat_map(|list| list.iter())
    .chain(r.others.iter().map(|(_, h)| h))
    .filter(|h| named(h))
    .map(|h| h.name.to_lowercase())
    .collect();
    let names_inside = !inside.is_empty();

    // A name that lives elsewhere. A screenplay's title only says which film; it is not what is asked.
    // Beside a topic of the pick, a scene or a day only narrows it ("the sides for day 2"), and a name
    // the picks speak of ("which beat shows Salim's log") is theirs too.
    let outside = d.outside.iter().find(|o| {
        let narrows = topic_is_picked
            && (matches!(o.kind, Kind::Scene | Kind::Day)
                || words(&o.name).iter().all(|w| values.contains(w)));
        input.index.screenplay(&o.id).is_none() && !narrows
    });
    if let Some(o) = outside {
        let target = home_of_name(o.kind, o.home, &hints);
        // Two questions in one, and the picks hold a thing the other one names: write that part.
        if two_parts && (found || names_inside) {
            return point_if_allowed(input, sel, target, trigger::NAME, true);
        }
        if found {
            return Verdict::Write;
        }
        return point_if_allowed(input, sel, target, trigger::NAME, false);
    }
    if let Some(target) = partial_topic(message, &d.facts, sel, &inside) {
        return point_if_allowed(input, sel, target, trigger::TOPIC, true);
    }
    if found {
        return Verdict::Write;
    }
    // Code already knows the thing asked about is not in the project: "a character called Marcus".
    let asked = content_words(message);
    if language_of(message).is_none()
        && !partly_there(&d.facts, message)
        && let Some(gap) = d
            .missing
            .iter()
            .find(|m| gap_concerns(m, &asked) && !m.contains("your role"))
    {
        return Verdict::NotFound(gap.clone());
    }
    if (form_elsewhere || !names_inside)
        && let Some(target) = topic_target(&hints, &d.facts, sel)
    {
        // The word may have gone to a section that holds nothing about this. All the words of the
        // question together know better.
        if !answered(&dossier::probe(input, target).facts, message)
            && let Some(better) = exists_elsewhere(input.conn, input.actor, sel, message)
            && better.id != target.id
        {
            return point_if_allowed(input, sel, better, trigger::EXISTS, false);
        }
        return point_if_allowed(input, sel, target, trigger::TOPIC, false);
    }
    // A how-to question with the guide not picked.
    if group == Group::Help
        && let Some(guide) = leaf("help/guide")
    {
        return Verdict::Point(Pointer::of(guide, trigger::HELP));
    }
    // Nothing pointed anywhere. Words of the question in the picks, or a pick narrowed to one scene
    // or day, keep the search from sending the person away: the writer answers from the picks.
    if hints.is_empty()
        && !partly_there(&d.facts, message)
        && !sel.picks.iter().any(|p| p.narrowed())
        && let Some(target) = exists_elsewhere(input.conn, input.actor, sel, message)
    {
        return point_if_allowed(input, sel, target, trigger::EXISTS, false);
    }
    // The picks hold nothing at all. The statements are code's, so the model has nothing to add. In
    // another language the writer still says it, in the person's words.
    if !d.facts.iter().any(real) && language_of(message).is_none() {
        return if d.missing.is_empty() {
            not_found(message, sel)
        } else {
            Verdict::NotFound(d.missing.join(" "))
        };
    }
    Verdict::Write
}

/// [`decide`] for a request in progress: a fault here costs the pointer, never the request.
pub fn try_decide(input: &dossier::Input<'_>, sel: &Selection, d: &Dossier) -> Verdict {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decide(input, sel, d)))
        .map_err(|_| tracing::error!("the pointer stopped on a fault"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sentence_and_the_structured_part_name_the_leaf() {
        let p = Pointer::of(leaf("production/budget").unwrap(), trigger::NAME);
        assert_eq!(
            p.sentence(),
            "That is in Production > Budget. Add it to your selection and ask again."
        );
        assert_eq!(
            p.structured(),
            json!({ "leaf": "production/budget", "label": "Production > Budget" })
        );
    }

    #[test]
    fn content_words_leave_the_filler_and_the_numbers_out() {
        assert_eq!(
            content_words("Which day is scene 3 shot?"),
            vec!["day".to_string(), "shot".to_string()]
        );
        assert_eq!(
            content_words("What is Sanoj Kurup's email address?"),
            vec!["sanoj", "kurup", "email", "address"]
        );
        assert!(content_words("What is in scene 4?").is_empty());
    }

    #[test]
    fn a_fact_answers_when_it_holds_more_than_half_of_what_is_asked() {
        let f = Fact::new("Budget line", "b1")
            .f("line", "Director fee")
            .f("planned", "5,00,000");
        assert!(answered(std::slice::from_ref(&f), "Who is the director?"));
        assert!(answered(
            std::slice::from_ref(&f),
            "What is the director's fee?"
        ));
        assert!(!answered(
            std::slice::from_ref(&f),
            "Which day is the director fee tall mast"
        ));
        assert!(!answered(std::slice::from_ref(&f), "Who plays Arjun?"));
        assert!(!answered(&[], "Who plays Arjun?"));
        assert!(answered(std::slice::from_ref(&f), "What is in scene 4?"));
        // A passage must hold all of it: any one word finds a passage.
        let p = Fact::new("Found", "x").f("text", "the valve insert");
        assert!(!answered(
            std::slice::from_ref(&p),
            "What lens is used for the valve insert?"
        ));
        assert!(partly_there(
            std::slice::from_ref(&p),
            "What lens is used for the valve insert?"
        ));
        assert!(answered(std::slice::from_ref(&p), "the valve insert"));
        let longer = Fact::new("Found", "x").f("text", "Babu offers Anjali money at the jetty");
        assert!(answered(
            std::slice::from_ref(&longer),
            "Babu offering Anjali money"
        ));
    }

    #[test]
    fn who_is_asking_is_not_what_is_asked() {
        assert_eq!(
            without_roles("As the director, brief me on this scene"),
            " , brief me on this scene"
        );
        assert!(leaf_hints(&without_roles("Describe Thomas for the casting director")).is_empty());
    }

    #[test]
    fn a_statement_of_what_is_missing_answers_nothing() {
        let gap = Fact::absent("any director", "the Budget");
        assert!(!answered(
            std::slice::from_ref(&gap),
            "Who is the director?"
        ));
    }

    #[test]
    fn a_two_part_question_is_split_with_its_question_word_kept() {
        assert_eq!(
            clauses("Who plays Arjun in scene 2, and what does the Camera cost?"),
            vec!["Who plays Arjun in scene 2", "what does the Camera cost?"]
        );
        assert_eq!(
            clauses("Who plays Arjun? What does the Camera cost?"),
            vec!["Who plays Arjun", "What does the Camera cost?"]
        );
        assert_eq!(clauses("How are Mariamma and Anjali related?").len(), 1);
        assert_eq!(clauses("Is the file for Babu and Jose here?").len(), 1);
    }

    #[test]
    fn a_word_of_a_pick_or_in_its_values_keeps_a_topic_word_from_pointing() {
        let budget = crate::modules::ai::sections::SectionPick {
            leaf: "production/budget".into(),
            ..Default::default()
        };
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE project(project_type TEXT); INSERT INTO project VALUES ('Feature Film');",
        )
        .unwrap();
        let sel = crate::modules::ai::sections::resolve_picks(&c, &[budget]).unwrap();
        // "cost" points at the Budget, which is picked; "director" does not, so it points away.
        let both = leaf_hints("What does the director cost?");
        assert!(topic_target(&both, &[], &sel).is_none());
        let away = leaf_hints("Who is the director?");
        assert_eq!(
            topic_target(&away, &[], &sel).map(|l| l.id),
            Some("production/cast-crew")
        );
        // A value of the picks holds the word: the pick covers it.
        let f = Fact::new("Budget line", "b1").f("line", "Director fee");
        assert!(topic_target(&away, std::slice::from_ref(&f), &sel).is_none());
        // A field that names a thing counts; a plain label such as "note" does not.
        let g = Fact::new("Budget line", "b1").f("director", "Camera");
        assert!(topic_target(&away, std::slice::from_ref(&g), &sel).is_none());
        let notes = leaf_hints("What does the note say?");
        let h = Fact::new("Budget line", "b1").f("note", "Camera");
        assert_eq!(
            topic_target(&notes, std::slice::from_ref(&h), &sel).map(|l| l.id),
            Some("notes/notes")
        );
        // A passage was found by the words themselves, so it says nothing about the topic.
        let p = Fact::new("Found", "x").f("text", "the director spoke");
        assert!(topic_target(&away, std::slice::from_ref(&p), &sel).is_some());
    }
}

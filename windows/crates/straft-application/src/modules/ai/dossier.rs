//! Fetching the facts an answer is written from (spec `ai-conversational-harness`, section 5.4;
//! spec `ai-section-scoped-assistant`, sections 3.3 to 3.5 and 12.1; rounds S5 and R3).
//!
//! One stage with typed input and typed output: the message was read (`turn::TurnRead`) and its
//! names were resolved (`index::Resolved`); this stage reads the project and returns a
//! [`Dossier`], a list of facts, one per line, each with its field names and the id of the record
//! it came from. Code decides what exists: a thing the project does not hold is listed in
//! `missing` as a plain statement, never left for the model to guess.
//!
//! The person picked the sections to read (one to three leaves of `sections::LEAVES`); this stage
//! reads those and nothing else. Order of work:
//!
//! 1. **Names** (`names.rs`): the resolver's answer is completed (the scene on screen, the focus) and
//!    restricted to the entities homed in the picked leaves. A name homed in another leaf is not
//!    fetched; it is kept aside in `Dossier::elsewhere` for the pointer. The instance picks (a
//!    draft, a scene, a shooting day, a call sheet) narrow what the leaves fetch.
//! 2. **Fetch**: each picked leaf runs its sources (`sections::run_leaf`). Nothing is added
//!    automatically: explaining a scene does not bring its breakdown or its schedule.
//! 3. **The gate** (`sections::gate`): a fact whose kind is not homed in a picked leaf is dropped,
//!    whatever produced it. Then **guards** (`guard.rs`): other people's private notes are dropped,
//!    contact data only stays for people who may see it, text that carries an instruction is marked
//!    as data, and a file system path never reaches the facts. A guard cannot be undone later.
//! 4. **Budget** (`allot.rs`): about 1,500 tokens (2,500 for a paragraph or a narrative answer),
//!    split across the leaves by weight with a minimum share each; a leaf that needs less passes
//!    its spare on. The lowest priority facts of a leaf go first; a fact that carries a required
//!    value never goes.
//! 5. **Passages** (`passages.rs`): leaf-scoped hybrid retrieval fills what is left of the budget,
//!    and never displaces a fact.
//!
//! Nothing here writes, prompts or checks: the answer stage (`answer`) puts the dossier into the
//! writer and checks the written answer against it.

mod allot;
pub(crate) mod ctx;
pub(crate) mod fetch;
mod guard;
mod names;
mod passages;
mod registry;
#[cfg(test)]
mod tests;
pub(crate) mod words;

use std::fmt::Write as _;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use straft_domain::Actor;

use super::index::{EntityIndex, Focus, Kind, Resolved};
use super::queries;
use super::sections::{self, Leaf, SectionPick};
use super::turn::{Group, LengthClass, TurnRead};
use crate::core::AppCore;

pub use ctx::FetchCtx;

/// How much a fact matters when the dossier is over its cap. A fact with a required value is never
/// cut whatever its priority.
pub const P_HIGH: u8 = 80;
pub const P_MID: u8 = 50;
pub const P_LOW: u8 = 20;

/// The kind that carries a statement code has decided ("no character called Marcus"); `build` moves
/// these to [`Dossier::missing`].
const MISSING: &str = "Missing";

/// One fact: what a record says, with the names of its fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    pub kind: &'static str,
    pub fields: Vec<(String, String)>,
    /// The record it came from, for the sources button.
    pub record_id: String,
    /// A relationship is told as its plain sentence first ("Varghese is brother of Mariamma.").
    pub sentence: Option<String>,
    pub priority: u8,
    /// Values a correct answer must state; a fact that holds one is never cut.
    pub requires: Vec<String>,
    /// Set for a private note: only this person may be given it.
    pub private_to: Option<String>,
    /// The leaf whose source read it (`sections::run_leaf` sets it). A kind several leaves produce,
    /// such as a count, is homed by this stamp.
    pub home: Option<&'static str>,
}

fn tidy(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" | ", " / ")
        .replace('|', "/")
}

impl Fact {
    pub fn new(kind: &'static str, record_id: impl Into<String>) -> Self {
        Self {
            kind,
            fields: Vec::new(),
            record_id: record_id.into(),
            sentence: None,
            priority: P_MID,
            requires: Vec::new(),
            private_to: None,
            home: None,
        }
    }

    /// A field. An empty value is left out: no field is better than an empty one.
    pub fn f(mut self, name: &str, value: impl AsRef<str>) -> Self {
        let v = tidy(value.as_ref());
        if !v.is_empty() {
            self.fields.push((name.to_string(), v));
        }
        self
    }

    pub fn said(mut self, sentence: impl AsRef<str>) -> Self {
        self.sentence = Some(tidy(sentence.as_ref()));
        self
    }

    pub fn prio(mut self, p: u8) -> Self {
        self.priority = p;
        self
    }

    pub fn require(mut self, value: impl AsRef<str>) -> Self {
        let v = tidy(value.as_ref());
        if !v.is_empty() && !self.requires.contains(&v) {
            self.requires.push(v);
        }
        self
    }

    pub fn private_to(mut self, user_id: &str) -> Self {
        self.private_to = Some(user_id.to_string());
        self
    }

    /// The project does not hold `what` (a noun phrase): "I could not find X. I looked in A and B."
    pub fn absent(what: impl AsRef<str>, looked_in: impl AsRef<str>) -> Self {
        Self::missing(queries::could_not_find(what.as_ref(), looked_in.as_ref()))
    }

    /// A statement about something the project does not hold.
    pub fn missing(text: impl AsRef<str>) -> Self {
        Fact::new(MISSING, "").said(text)
    }

    pub fn is_missing(&self) -> bool {
        self.kind == MISSING
    }

    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// `Kind | field: value | field: value | id: <id>`. A sentence comes first and stands for the kind.
    pub fn render(&self) -> String {
        let mut line = self.render_plain();
        if !self.record_id.is_empty() {
            let _ = write!(line, " | id: {}", self.record_id);
        }
        line
    }

    /// The same line without the record id: what the writer is shown. An id costs about fifteen
    /// tokens and invites the model to echo it; the sources are found by code from the facts.
    pub fn render_plain(&self) -> String {
        let mut line = match &self.sentence {
            Some(s) => s.clone(),
            None => self.kind.to_string(),
        };
        for (n, v) in &self.fields {
            let _ = write!(line, " | {n}: {v}");
        }
        line
    }

    /// Four characters to the token, the same estimate the spec uses.
    pub fn tokens(&self) -> usize {
        (self.render().chars().count() + 1).div_ceil(4)
    }
}

/// The output of the stage.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dossier {
    pub facts: Vec<Fact>,
    /// Values a correct answer must state.
    pub required: Vec<String>,
    /// Plain statements about what the project does not hold.
    pub missing: Vec<String>,
    /// How many times a guard removed, hid or marked something.
    pub notes_on_guard: usize,
    /// The token cap cut facts away before the writer saw them.
    pub trimmed: bool,
    pub tokens_estimate: usize,
    /// What the message names once the scene on screen, the focus and the default screenplay are
    /// counted in, restricted to the picked leaves. `resolved.ambiguous` is the one question left
    /// when nothing settles it.
    pub resolved: Resolved,
    /// The tokens each picked leaf took, in the order of the picks.
    pub leaves: Vec<LeafUse>,
    /// How passages were found, when any were looked for.
    pub retrieval: Option<Retrieval>,
    /// Names the message holds that are homed in a leaf that was not picked. They are not fetched;
    /// the pointer (round R4) names the leaf.
    pub elsewhere: Vec<Elsewhere>,
}

/// The dossier tokens one picked leaf took.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeafUse {
    pub leaf: &'static str,
    pub tokens: usize,
}

/// How the passages were found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retrieval {
    /// Meaning search ran besides the words (hybrid).
    pub hybrid: bool,
    /// The project index was not ready, so only the words were searched.
    pub degraded: bool,
}

/// A name of the message whose home leaf was not picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Elsewhere {
    /// The home leaf: the first leaf in the table that serves the kind.
    pub leaf: &'static str,
    pub kind: Kind,
    pub name: String,
}

/// What the trace keeps: numbers only, never the text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub facts: usize,
    pub tokens: usize,
    pub missing: usize,
    pub required: usize,
}

impl Dossier {
    /// The facts as the writer will read them, one per line.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for f in &self.facts {
            out.push_str(&f.render());
            out.push('\n');
        }
        out
    }

    /// The facts as the writer reads them: one per line, no record ids.
    pub fn render_plain(&self) -> String {
        let mut out = String::new();
        for f in &self.facts {
            out.push_str(&f.render_plain());
            out.push('\n');
        }
        out
    }

    /// `leaf=tokens` for each picked leaf, comma separated, for the trace (codes and numbers only).
    pub fn leaf_tokens(&self) -> String {
        self.leaves
            .iter()
            .map(|l| format!("{}={}", l.leaf, l.tokens))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn summary(&self) -> Summary {
        Summary {
            facts: self.facts.len(),
            tokens: self.tokens_estimate,
            missing: self.missing.len(),
            required: self.required.len(),
        }
    }
}

/// The input of the stage.
pub struct Input<'a> {
    pub conn: &'a Connection,
    pub actor: &'a Actor,
    pub index: &'a EntityIndex,
    /// The application core, for the few facts that are app-level (appearance, spelling).
    pub core: Option<&'a AppCore>,
    pub message: &'a str,
    pub read: &'a TurnRead,
    pub resolved: &'a Resolved,
    /// The scene the person has selected in the app, when one is: the scene "it" and "this" mean when
    /// the message names no other.
    pub selected_scene: Option<&'a str>,
    /// What the conversation is about once this message is part of it. A correction that names
    /// nothing ("I asked you to explain, not open") is about this.
    pub focus: Option<&'a Focus>,
    /// The person's previous message, which a correction repeats the question of.
    pub previous_message: Option<&'a str>,
    /// The sections the person picked, with the instance choices under each. The dossier reads these
    /// leaves and no others.
    pub sections: &'a [SectionPick],
}

/// The most a dossier may hold, in tokens.
pub fn cap_tokens(class: LengthClass) -> usize {
    match class {
        LengthClass::Paragraph | LengthClass::Long250 | LengthClass::Long350 => 2_500,
        _ => 1_500,
    }
}

/// [`build`] for a request in progress: a fault in the stage costs the dossier, never the request.
pub fn try_build(input: &Input<'_>) -> Option<Dossier> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build(input)))
        .map_err(|_| tracing::error!("the dossier stage stopped on a fault"))
        .ok()
}

/// The leaves a question reads: the picks, each leaf once, in the order they were picked.
fn picked_leaves(sections: &[SectionPick]) -> Vec<&'static Leaf> {
    let mut out: Vec<&'static Leaf> = Vec::new();
    for p in sections {
        if let Some(l) = sections::leaf(&p.leaf)
            && !out.iter().any(|o| o.id == l.id)
        {
            out.push(l);
        }
    }
    out
}

/// Build the dossier of one message from the sections that were picked. Never fails and never
/// blocks: a fetcher that cannot read its tables adds nothing and logs a warning.
pub fn build(input: &Input<'_>) -> Dossier {
    let picked = picked_leaves(input.sections);
    let ctx = FetchCtx::new(input);
    names::pin_drafts(&ctx, input.sections);
    let (resolved, mut facts) = names::effective(&ctx, input);
    let (resolved, elsewhere) = names::restrict(resolved, &picked);
    let resolved = names::narrow(&ctx, resolved, input.sections);
    if registry::wants_facts(input.read.group) {
        for leaf in &picked {
            facts.extend(sections::run_leaf(leaf, &ctx, input.read, &resolved));
        }
    }
    finish(&ctx, input, &picked, resolved, elsewhere, facts)
}

fn finish(
    ctx: &FetchCtx<'_>,
    input: &Input<'_>,
    picked: &[&'static Leaf],
    resolved: Resolved,
    elsewhere: Vec<Elsewhere>,
    mut facts: Vec<Fact>,
) -> Dossier {
    let mut missing: Vec<String> = resolved.not_found.clone();
    // The one hard boundary: nothing homed in a leaf that was not picked goes on.
    sections::gate(picked, &mut facts);
    let mut notes_on_guard = guard::apply(&ctx.guard, &mut facts, &mut missing);
    let (gaps, mut facts): (Vec<Fact>, Vec<Fact>) = facts.into_iter().partition(Fact::is_missing);
    for g in gaps {
        if let Some(s) = g.sentence
            && !missing.contains(&s)
        {
            missing.push(s);
        }
    }
    dedupe(&mut facts);
    let cap = cap_tokens(input.read.length_class);
    let trimmed = split_and_trim(&mut facts, picked, cap);
    let wants = registry::wants_facts(input.read.group);
    // Passages fill what the leaves left. They are looked for when the facts cannot settle the
    // question alone (no value to state and nothing said to be missing) and for narrative answers.
    let settled = facts.iter().any(|f| !f.requires.is_empty()) || !missing.is_empty();
    let narrative = matches!(input.read.group, Group::Narrative | Group::Brainstorm);
    let mut retrieval = None;
    if wants && input.read.group != Group::Help && (!settled || narrative) {
        let used: usize = facts.iter().map(Fact::tokens).sum();
        let have: Vec<String> = facts.iter().map(|f| f.record_id.clone()).collect();
        let (mut found, how) = passages::find(
            ctx.conn,
            input,
            picked,
            &ctx.message,
            cap.saturating_sub(used),
            &have,
        );
        retrieval = how;
        // Passages pass the same guards as the leaves' facts: privacy, contact data, paths and
        // instruction-like text.
        notes_on_guard += guard::apply(&ctx.guard, &mut found, &mut missing);
        facts.extend(found);
    }
    if !facts.iter().any(|f| !f.is_missing()) && missing.is_empty() && wants {
        let looked: Vec<String> = picked.iter().map(|l| l.path_label()).collect();
        missing.push(queries::could_not_find(
            &format!(
                "anything that matches \u{201C}{}\u{201D}",
                words::clip(ctx.message.trim(), 80)
            ),
            &queries::join_and(&looked),
        ));
    }
    let mut required: Vec<String> = Vec::new();
    for f in &facts {
        for r in &f.requires {
            if !required.contains(r) {
                required.push(r.clone());
            }
        }
    }
    let tokens_estimate = facts.iter().map(Fact::tokens).sum();
    let leaves = picked
        .iter()
        .map(|l| LeafUse {
            leaf: l.id,
            tokens: facts
                .iter()
                .filter(|f| f.home == Some(l.id))
                .map(Fact::tokens)
                .sum(),
        })
        .collect();
    Dossier {
        facts,
        required,
        missing,
        notes_on_guard,
        trimmed,
        tokens_estimate,
        resolved,
        leaves,
        retrieval,
        elsewhere,
    }
}

/// Keep each picked leaf within its share of `cap` (spec 3.4). Facts no leaf read (the orientation
/// code adds itself) count against the cap first. A required fact is never cut, so a leaf can stay
/// over its share when its required facts alone are large.
fn split_and_trim(facts: &mut Vec<Fact>, picked: &[&'static Leaf], cap: usize) -> bool {
    let own: usize = facts
        .iter()
        .filter(|f| !picked.iter().any(|l| Some(l.id) == f.home))
        .map(Fact::tokens)
        .sum();
    let room = cap.saturating_sub(own);
    let wants: Vec<allot::Want> = picked
        .iter()
        .map(|l| allot::Want {
            weight: u32::from(l.weight),
            demand: facts
                .iter()
                .filter(|f| f.home == Some(l.id))
                .map(Fact::tokens)
                .sum(),
        })
        .collect();
    let shares = allot::shares(room, &wants);
    let mut cut = vec![false; facts.len()];
    for (leaf, share) in picked.iter().zip(shares) {
        let mine: Vec<usize> = (0..facts.len())
            .filter(|i| facts[*i].home == Some(leaf.id))
            .collect();
        for i in cuts(facts, &mine, share) {
            cut[i] = true;
        }
    }
    let any = cut.iter().any(|c| *c);
    let mut at = 0;
    facts.retain(|_| {
        at += 1;
        !cut[at - 1]
    });
    any
}

/// The same line twice adds nothing; its required values are kept.
fn dedupe(facts: &mut Vec<Fact>) {
    let mut seen: Vec<(String, usize)> = Vec::new();
    let mut out: Vec<Fact> = Vec::new();
    for f in facts.drain(..) {
        let line = f.render();
        if let Some((_, at)) = seen.iter().find(|(l, _)| *l == line) {
            for r in f.requires {
                if !out[*at].requires.contains(&r) {
                    out[*at].requires.push(r);
                }
            }
            continue;
        }
        seen.push((line, out.len()));
        out.push(f);
    }
    *facts = out;
}

/// Which of the facts at `idx` to cut so that they fit `cap` tokens: the lowest priority first, and
/// within one priority the later fact first. A fact that carries a required value is never cut.
fn cuts(facts: &[Fact], idx: &[usize], cap: usize) -> Vec<usize> {
    let mut total: usize = idx.iter().map(|i| facts[*i].tokens()).sum();
    if total <= cap {
        return Vec::new();
    }
    let mut order: Vec<usize> = idx
        .iter()
        .copied()
        .filter(|i| facts[*i].requires.is_empty() && !facts[*i].is_missing())
        .collect();
    order.sort_by_key(|i| (facts[*i].priority, std::cmp::Reverse(*i)));
    let mut out = Vec::new();
    for i in order {
        if total <= cap {
            break;
        }
        total -= facts[i].tokens();
        out.push(i);
    }
    out
}

/// Cut the lowest priority facts until the facts fit `cap` tokens. A fact that carries a required
/// value never goes, so the result can stay over the cap when the required facts alone are large.
#[cfg(test)]
fn trim(facts: &mut Vec<Fact>, cap: usize) {
    let all: Vec<usize> = (0..facts.len()).collect();
    let cut = cuts(facts, &all, cap);
    let mut at = 0;
    facts.retain(|_| {
        at += 1;
        !cut.contains(&(at - 1))
    });
}

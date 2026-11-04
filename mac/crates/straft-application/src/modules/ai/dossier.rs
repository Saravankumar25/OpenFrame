//! Fetching the facts an answer is written from (spec `ai-conversational-harness`, section 5.4,
//! slice S5).
//!
//! One stage with typed input and typed output: the message was read (`turn::TurnRead`) and its
//! names were resolved (`index::Resolved`); this stage reads the project and returns a
//! [`Dossier`], a list of facts, one per line, each with its field names and the id of the record
//! it came from. Code decides what exists: a thing the project does not hold is listed in
//! `missing` as a plain statement, never left for the model to guess.
//!
//! Only the sections the person picked are read (ADR-0041). Order of work, in `leaves.rs`:
//!
//! 1. **The fetchers of the picked leaves** run, and only the kinds of fact each leaf admits pass.
//!    Nothing is added on the strength of the message's words.
//! 2. **Guards** run on what a fetcher may ask for (`FetchCtx::contacts`) and again on the facts
//!    (`guard.rs`). A guard cannot be undone by a later step: other people's private notes are
//!    dropped, contact data only stays for people who may see it, text that carries an instruction is
//!    marked as data, and a file system path never reaches the facts.
//! 3. **Text** found by the search inside each leaf comes in at the lowest priority.
//! 4. **Cap**: about 1,500 tokens for one-sentence, short and brief answers and 2,500 for a paragraph
//!    or a long answer, split across the leaves by weight. The lowest priority facts go first; a fact
//!    that carries a required value never goes.
//!
//! Nothing here writes, prompts or checks: the answer stage (`answer`) puts the dossier into the
//! writer and checks the written answer against it.

mod asked;
mod ctx;
pub(super) mod fetch;
pub(crate) mod guard;
mod leaves;
mod names;
#[cfg(test)]
mod tests;
mod words;

use std::fmt::Write as _;

use straft_domain::Actor;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::index::{EntityIndex, Focus, Resolved};
use super::queries;
use super::turn::{Group, LengthClass, TurnRead};
use crate::core::AppCore;

pub use ctx::FetchCtx;
pub use leaves::{LeafRead, LeafTokens, OutsideName, probe, split_budget};

/// One way of reading the project.
pub type FetchFn = fn(&FetchCtx<'_>, &TurnRead, &Resolved) -> Vec<Fact>;

/// How much a fact matters when the dossier is over its cap. A fact with a required value is never
/// cut whatever its priority.
pub const P_HIGH: u8 = 80;
/// A row the words of the question picked: the project holds what is asked, so "could not find" is wrong.
pub const P_PICKED: u8 = P_HIGH + 5;
pub const P_MID: u8 = 50;
pub const P_LOW: u8 = 20;

/// Chat and change requests need no facts; the rest read the project.
pub fn wants_facts(group: Group) -> bool {
    !matches!(group, Group::Chat | Group::Change)
}

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
    pub tokens_estimate: usize,
    /// What the message names once the scene on screen, the focus and the default screenplay are
    /// counted in. `resolved.ambiguous` is the one question left when nothing settles it.
    pub resolved: Resolved,
    /// Names the message holds that no picked section can answer about, for the pointer.
    pub outside: Vec<OutsideName>,
    /// How many tokens each picked section took, for the trace.
    pub leaf_tokens: Vec<LeafTokens>,
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
#[derive(Clone, Copy)]
pub struct Input<'a> {
    pub conn: &'a Connection,
    pub actor: &'a Actor,
    pub index: &'a EntityIndex,
    /// The application core, for the few facts that are app-level (appearance, spelling).
    pub core: Option<&'a AppCore>,
    pub message: &'a str,
    pub read: &'a TurnRead,
    pub resolved: &'a Resolved,
    /// What the conversation is about once this message is part of it. A correction that names
    /// nothing ("I asked you to explain, not open") is about this.
    pub focus: Option<&'a Focus>,
    /// The person's previous message, which a correction repeats the question of.
    pub previous_message: Option<&'a str>,
    /// The sections the person picked, and what the text search found in them.
    pub sections: &'a LeafRead<'a>,
}

/// The most a dossier may hold, in tokens.
pub fn cap_tokens(class: LengthClass) -> usize {
    match class {
        LengthClass::Paragraph | LengthClass::Narrative | LengthClass::Story => 2_500,
        _ => 1_500,
    }
}

/// [`build`] for a request in progress: a fault in the stage costs the dossier, never the request.
pub fn try_build(input: &Input<'_>) -> Option<Dossier> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build(input)))
        .map_err(|_| tracing::error!("the dossier stage stopped on a fault"))
        .ok()
}

/// Build the dossier of one message. Never fails and never blocks: a fetcher that cannot read its
/// tables adds nothing and logs a warning.
pub fn build(input: &Input<'_>) -> Dossier {
    leaves::build(input, input.sections)
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

/// Cut the lowest priority facts until the facts fit `cap` tokens. A fact that carries a required
/// value never goes, so the result can stay over the cap when the required facts alone are large.
fn trim(facts: &mut Vec<Fact>, cap: usize) {
    let mut total: usize = facts.iter().map(Fact::tokens).sum();
    if total <= cap {
        return;
    }
    let mut order: Vec<usize> = (0..facts.len())
        .filter(|i| facts[*i].requires.is_empty() && !facts[*i].is_missing())
        .collect();
    // Lowest priority first, and within one priority the later fact first.
    order.sort_by_key(|i| (facts[*i].priority, std::cmp::Reverse(*i)));
    let mut cut = vec![false; facts.len()];
    for i in order {
        if total <= cap {
            break;
        }
        total -= facts[i].tokens();
        cut[i] = true;
    }
    let mut at = 0;
    facts.retain(|_| {
        at += 1;
        !cut[at - 1]
    });
}

//! Reading the answer type of a message (spec `ai-conversational-harness`, section 5.3, slice S4).
//!
//! One stage with typed input and typed output. The turn reads two separate things: the kind of
//! answer wanted (`AnswerType`) and where the facts live (`Domain`), and from the type it takes the
//! length class of the answer. Order of reading, cheapest first, each step only when the one before
//! it was not sure:
//!
//! 1. Strong rules (`rules.rs`): "how many", relation and compare words, "open scene 4", explain
//!    verbs, "how do I", a change verb at the start, brainstorm words. Names come from the resolver
//!    (`index::Resolved`), never from a word list.
//! 2. A correction or complaint that names no new request ("I asked you", "again, you opened scene
//!    41"), and a bare title that answers the last question, take the previous answer type.
//! 3. Chat, but only from social or angry words in a message with no request and no resolved name.
//! 4. Nearest neighbours (`neighbour.rs`): the message is embedded with the search model the app
//!    ships and the labelled example messages of `turn_examples.jsonl` vote.
//! 5. Only when that vote is weak, one short model call with the closed list.
//!
//! Without an embedder or an engine (tests, AI not installed) the stage ends with the rules and the
//! previous type. It never panics and never blocks a request: the embedding step waits at most
//! [`WAIT`], and a late result still fills the example cache for the next message.
//!
//! The stage decides nothing about the answer yet: the read is stored in the conversation focus and
//! in the AI trace, where the evaluation measures it.

mod domains;
mod model;
mod neighbour;
mod rules;
#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::time::Duration;

use openframe_ai::InferenceProvider;
use openframe_search::Embedder;
use serde::{Deserialize, Serialize};

use super::index::Resolved;

pub use domains::Domain;
pub use neighbour::{LooRead, leave_one_out};

/// How long a request waits for the embedding step. The first message after the app starts embeds
/// every example, which takes longer; the vector cache then makes later messages fast.
pub const WAIT: Duration = Duration::from_millis(1500);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerType {
    Lookup,
    List,
    Count,
    Explain,
    Relation,
    Compare,
    Brainstorm,
    Navigate,
    Change,
    Help,
    Chat,
}

pub const ANSWER_TYPES: [AnswerType; 11] = [
    AnswerType::Lookup,
    AnswerType::List,
    AnswerType::Count,
    AnswerType::Explain,
    AnswerType::Relation,
    AnswerType::Compare,
    AnswerType::Brainstorm,
    AnswerType::Navigate,
    AnswerType::Change,
    AnswerType::Help,
    AnswerType::Chat,
];

impl AnswerType {
    pub fn code(self) -> &'static str {
        match self {
            AnswerType::Lookup => "lookup",
            AnswerType::List => "list",
            AnswerType::Count => "count",
            AnswerType::Explain => "explain",
            AnswerType::Relation => "relation",
            AnswerType::Compare => "compare",
            AnswerType::Brainstorm => "brainstorm",
            AnswerType::Navigate => "navigate",
            AnswerType::Change => "change",
            AnswerType::Help => "help",
            AnswerType::Chat => "chat",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        ANSWER_TYPES.into_iter().find(|t| t.code() == s.trim())
    }

    /// The coarse group: it picks the fetch plan and the length class.
    pub fn group(self) -> Group {
        match self {
            AnswerType::Lookup | AnswerType::List | AnswerType::Count | AnswerType::Compare => {
                Group::Facts
            }
            AnswerType::Explain | AnswerType::Relation => Group::Narrative,
            AnswerType::Brainstorm => Group::Brainstorm,
            AnswerType::Navigate => Group::Navigate,
            AnswerType::Change => Group::Change,
            AnswerType::Help => Group::Help,
            AnswerType::Chat => Group::Chat,
        }
    }

    pub fn length_class(self) -> LengthClass {
        match self {
            AnswerType::Lookup | AnswerType::Count | AnswerType::Navigate | AnswerType::Change => {
                LengthClass::OneSentence
            }
            AnswerType::List | AnswerType::Compare => LengthClass::Short,
            AnswerType::Explain
            | AnswerType::Relation
            | AnswerType::Brainstorm
            | AnswerType::Help => LengthClass::Paragraph,
            AnswerType::Chat => LengthClass::Brief,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Facts,
    Narrative,
    Brainstorm,
    Navigate,
    Change,
    Help,
    Chat,
}

impl Group {
    pub fn code(self) -> &'static str {
        match self {
            Group::Facts => "facts",
            Group::Narrative => "narrative",
            Group::Brainstorm => "brainstorm",
            Group::Navigate => "navigate",
            Group::Change => "change",
            Group::Help => "help",
            Group::Chat => "chat",
        }
    }
}

/// The shape of the answer. The word ranges are the same as `LENGTH_CLASSES` in the evaluation crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LengthClass {
    OneSentence,
    Short,
    Paragraph,
    Brief,
}

impl LengthClass {
    pub fn code(self) -> &'static str {
        match self {
            LengthClass::OneSentence => "one_sentence",
            LengthClass::Short => "short",
            LengthClass::Paragraph => "paragraph",
            LengthClass::Brief => "brief",
        }
    }

    /// The inclusive word range of an answer of this class.
    pub fn words(self) -> (usize, usize) {
        match self {
            LengthClass::OneSentence => (1, 30),
            LengthClass::Short => (1, 80),
            LengthClass::Paragraph => (40, 200),
            LengthClass::Brief => (1, 40),
        }
    }

    /// One class shorter. Chat and the one-sentence class are already as short as they go.
    fn shorter(self) -> Self {
        match self {
            LengthClass::Paragraph => LengthClass::Short,
            LengthClass::Short => LengthClass::OneSentence,
            other => other,
        }
    }

    /// One class longer, up to a paragraph. A chat reply stays brief.
    fn longer(self) -> Self {
        match self {
            LengthClass::OneSentence => LengthClass::Short,
            LengthClass::Short => LengthClass::Paragraph,
            other => other,
        }
    }
}

/// What decided the answer type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadHow {
    /// A strong rule, or the chat gate.
    Rule,
    /// A correction, complaint or bare title: the type of the previous message.
    Repair,
    /// The nearest example messages voted.
    Neighbour,
    /// The vote was weak and one short model call chose from the closed list.
    Model,
    /// Nothing could decide (no embedder, no engine): the previous type, or a default.
    Fallback,
}

impl ReadHow {
    pub fn code(self) -> &'static str {
        match self {
            ReadHow::Rule => "rule",
            ReadHow::Repair => "repair",
            ReadHow::Neighbour => "neighbour",
            ReadHow::Model => "model",
            ReadHow::Fallback => "fallback",
        }
    }
}

/// The output of the stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnRead {
    pub answer_type: AnswerType,
    pub group: Group,
    pub domains: Vec<Domain>,
    pub length_class: LengthClass,
    pub how: ReadHow,
    /// Share of the nearest examples that voted for the type, 0 to 1; 0 when no vote ran.
    pub vote: f32,
}

impl TurnRead {
    /// The domains as one trace value: `budget,schedule`.
    pub fn domain_codes(&self) -> String {
        self.domains
            .iter()
            .map(|d| d.code())
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// The input of the stage.
pub struct TurnInput<'a> {
    pub message: &'a str,
    /// The previous message of the person in this conversation.
    pub previous_message: Option<&'a str>,
    /// The answer type read for that message (the focus keeps it).
    pub previous_type: Option<AnswerType>,
    pub resolved: &'a Resolved,
}

/// What the stage may use besides code. Each is optional; the stage works with none of them.
pub struct Aids<'a> {
    pub embedder: Option<Arc<dyn Embedder>>,
    pub model: Option<&'a dyn InferenceProvider>,
    /// True once the person pressed Stop: no model call is started.
    pub stopped: &'a dyn Fn() -> bool,
    /// How long the embedding step may take.
    pub wait: Duration,
}

impl Aids<'static> {
    /// Rules and the previous type only.
    pub fn none() -> Self {
        Aids {
            embedder: None,
            model: None,
            stopped: &|| false,
            wait: WAIT,
        }
    }
}

/// Longest message the embedding and the model see.
const MAX_CHARS: usize = 1000;

/// Read one message.
pub fn read(input: &TurnInput<'_>, aids: &Aids<'_>) -> TurnRead {
    let message: String = input.message.chars().take(MAX_CHARS).collect();
    let (answer_type, how, vote) = decide(&message, input, aids);
    TurnRead {
        answer_type,
        group: answer_type.group(),
        domains: domains::find(&message, input.resolved, answer_type),
        length_class: rules::length_class(&message, answer_type),
        how,
        vote,
    }
}

fn decide(message: &str, input: &TurnInput<'_>, aids: &Aids<'_>) -> (AnswerType, ReadHow, f32) {
    let previous = input.previous_type.filter(|t| *t != AnswerType::Chat);
    match rules::decide(message, input.resolved, previous) {
        rules::Verdict::Type(t) => (t, ReadHow::Rule, 0.0),
        rules::Verdict::Inherit(t) => (t, ReadHow::Repair, 0.0),
        rules::Verdict::Chat => (AnswerType::Chat, ReadHow::Rule, 0.0),
        rules::Verdict::Open { angry } => {
            let guard = |t: AnswerType| {
                // Anger is never a change or a count, whatever the neighbours say.
                if angry && matches!(t, AnswerType::Change | AnswerType::Count) {
                    AnswerType::Chat
                } else {
                    t
                }
            };
            let vote = aids
                .embedder
                .clone()
                .and_then(|e| neighbour::vote_within(e, message, aids.wait));
            if let Some(v) = vote {
                if v.clear() {
                    return (guard(v.winner), ReadHow::Neighbour, v.share);
                }
                if !(aids.stopped)()
                    && let Some(model) = aids.model
                    && let Some(t) = model::ask(model, message, input.previous_message)
                {
                    return (guard(t), ReadHow::Model, v.share);
                }
                return (guard(v.winner), ReadHow::Neighbour, v.share);
            }
            match previous {
                Some(t) => (t, ReadHow::Fallback, 0.0),
                None => (rules::default_type(message), ReadHow::Fallback, 0.0),
            }
        }
    }
}

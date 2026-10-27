//! Conversations for measuring the answer-type stage (spec slice S4): the gold cases and the
//! `turn-sets/` files turned into lists of messages with labels, and the way the assistant reads
//! them (the earlier messages first, each with the previous message and type, the resolver working
//! on the fixture project). Used by `ai-eval --turn-only` and by the tests.

use std::path::Path;
use std::sync::Arc;

use openframe_application::core::AppCore;
use openframe_application::modules::ai::index::{self, EntityIndex, Focus};
use openframe_application::modules::ai::turn;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{Case, fixture, is_proposal_tool};

pub struct Measured {
    pub turn: usize,
    pub set: String,
    pub family: String,
    pub id: String,
    pub want: Option<turn::AnswerType>,
}

pub struct Conv {
    pub project: String,
    pub turns: Vec<String>,
    pub measured: Vec<Measured>,
}

#[derive(Deserialize)]
struct SetLine {
    id: String,
    msg: String,
    prev: String,
    want: String,
}

fn parse_type(s: &str) -> Option<turn::AnswerType> {
    turn::AnswerType::parse(s)
}

/// The type a tool case implies, for cases that carry a tool and no `answer_type`.
fn tool_label(c: &Case) -> Option<turn::AnswerType> {
    let tools = c.expect.tools();
    if tools.is_empty() {
        return None;
    }
    if tools.iter().all(|t| is_proposal_tool(t)) {
        Some(turn::AnswerType::Change)
    } else if tools.iter().all(|t| t.starts_with("open_")) {
        Some(turn::AnswerType::Navigate)
    } else if tools.iter().all(|t| t.starts_with("count_")) {
        Some(turn::AnswerType::Count)
    } else {
        None
    }
}

pub fn from_cases(cases: &[Case]) -> Vec<Conv> {
    let mut out = Vec::new();
    for c in cases.iter().filter(|c| c.kind != "retrieval") {
        let want = c
            .expect
            .answer_type
            .as_deref()
            .and_then(parse_type)
            .or_else(|| tool_label(c));
        let (set, family) = if c.id.starts_with("conv-lt-") {
            let family = c.id.split('-').nth(2).unwrap_or("").to_string();
            ("lowtide conversation".to_string(), family)
        } else if c.id.starts_with("tool-lt-") {
            ("lowtide tools".to_string(), "tool".to_string())
        } else {
            (format!("{} {}", c.kind, c.lang), c.kind.clone())
        };
        let mut turns = c.before.clone();
        turns.push(c.request.clone());
        out.push(Conv {
            project: c.project.clone(),
            measured: vec![Measured {
                turn: turns.len() - 1,
                set,
                family,
                id: c.id.clone(),
                want,
            }],
            turns,
        });
    }
    out
}

/// The 40 export messages as one conversation, the 30 held-out messages one by one.
pub fn from_sets(dir: &Path) -> Result<Vec<Conv>, String> {
    let load = |name: &str| -> Result<Vec<SetLine>, String> {
        let p = dir.join(name);
        std::fs::read_to_string(&p)
            .map_err(|e| format!("{}: {e}", p.display()))?
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(|e| format!("{}: {e}", p.display())))
            .collect()
    };
    let want = |s: &str| parse_type(s).ok_or_else(|| format!("unknown type {s}"));
    let mut out = Vec::new();
    let export = load("export-40.jsonl")?;
    let mut conv = Conv {
        project: fixture::LOW_TIDE.into(),
        turns: Vec::new(),
        measured: Vec::new(),
    };
    for l in &export {
        conv.measured.push(Measured {
            turn: conv.turns.len(),
            set: "export 40".into(),
            family: "export".into(),
            id: l.id.clone(),
            want: Some(want(&l.want)?),
        });
        conv.turns.push(l.msg.clone());
    }
    out.push(conv);
    // fresh-40 was written after the S4 rules were tuned on the other sets, so it is the honest
    // estimate for new wording.
    for (file, set) in [
        ("heldout-30.jsonl", "held-out 30"),
        ("fresh-40.jsonl", "fresh 40"),
    ] {
        for l in load(file)? {
            let mut turns = Vec::new();
            if !l.prev.is_empty() {
                turns.push(l.prev.clone());
            }
            turns.push(l.msg.clone());
            out.push(Conv {
                project: fixture::LOW_TIDE.into(),
                measured: vec![Measured {
                    turn: turns.len() - 1,
                    set: set.into(),
                    family: "held-out".into(),
                    id: l.id.clone(),
                    want: Some(want(&l.want)?),
                }],
                turns,
            });
        }
    }
    Ok(out)
}

pub fn index_of(core: &AppCore, project: &str) -> Result<Arc<EntityIndex>, String> {
    let op = |name: &str, args: Value| {
        core.dispatch(&core.local_actor(), name, args)
            .map_err(|e| format!("{name}: {} {}", e.code_str(), e.message))
    };
    if core.project_opt().is_some() {
        op("project.close", json!({}))?;
    }
    op(
        "project.create",
        json!({ "title": project, "projectType": "Feature Film" }),
    )?;
    if fixture::is_fixture(project) {
        fixture::build(core, project)?;
    } else {
        op(
            "screenplay.import_apply",
            json!({ "sample": project, "mode": "new_screenplay" }),
        )?;
    }
    let session = core.project().map_err(|e| e.message)?;
    index::project_index(&session).map_err(|e| e.message)
}

/// Read every turn of one conversation in order.
pub fn read_conv(ix: &EntityIndex, conv: &Conv, aids: &turn::Aids<'_>) -> Vec<turn::TurnRead> {
    read_conv_resolved(ix, conv, aids)
        .into_iter()
        .map(|(_, r, _)| r)
        .collect()
}

/// Every turn of one conversation in order, with the names each message resolved to and the focus
/// once the message is part of the conversation.
pub fn read_conv_resolved(
    ix: &EntityIndex,
    conv: &Conv,
    aids: &turn::Aids<'_>,
) -> Vec<(index::Resolved, turn::TurnRead, Focus)> {
    let mut focus = Focus::default();
    let mut previous: Option<(&str, turn::AnswerType)> = None;
    let mut reads = Vec::new();
    for msg in &conv.turns {
        let resolved = index::resolve(ix, &focus, msg);
        let r = turn::read(
            &turn::TurnInput {
                message: msg,
                previous_message: previous.map(|p| p.0),
                previous_type: previous.map(|p| p.1),
                resolved: &resolved,
            },
            aids,
        );
        focus = focus.after(&resolved);
        previous = Some((msg, r.answer_type));
        reads.push((resolved, r, focus.clone()));
    }
    reads
}

//! Short-term memory (spec section 5.1): the last turns of the conversation word for word, and a
//! rolling summary of the older ones written by code from the focus each turn stored.
//!
//! The summary never comes from the model: it lists what each older turn was about (the scene, the
//! screenplay, the people) and what kind of answer it was, so a long conversation keeps its thread
//! without a second model call.

use openframe_domain::{Actor, AppResult};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use super::approx_tokens;
use super::rules;
use crate::modules::ai::index::{EntityIndex, Focus, Kind};
use crate::modules::ai::queries::truncate_chars;
use crate::modules::ai::sections::Selection;
use crate::modules::ai::types::{AiResultKind, AiResultStatus};

/// Turns kept word for word.
pub const RECENT_TURNS: usize = 8;
/// Tokens (about four characters each) all the kept turns may use together.
pub const RECENT_TOKENS: usize = 1_200;
/// One message or one answer is cut to this many tokens, so a single long answer cannot use the budget.
const TURN_PART_TOKENS: usize = RECENT_TOKENS / 2;
/// Older turns looked at for the summary.
const OLDER_LOOKED_AT: usize = 60;
/// Lines the summary keeps (the newest older turns), each at most [`LINE_CHARS`] characters.
const SUMMARY_LINES: usize = 10;
const LINE_CHARS: usize = 110;

/// One earlier exchange as the assistant saw it: the person's message and the answer as shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentTurn {
    pub asked: String,
    pub answered: String,
    /// The answer was a plain, short answer the person may confirm with "yes, that's right"
    /// (never a proposal, a refusal, an error, a private note or a doubtful reply).
    pub confirmable: bool,
}

/// Everything the assistant knows about this conversation before it answers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConversationMemory {
    /// Oldest first.
    pub recent: Vec<RecentTurn>,
    /// One line per older turn that named something, oldest first ("asked about scene 35 of LOW TIDE RADIO").
    pub summary_lines: Vec<String>,
    pub focus: Focus,
}

impl ConversationMemory {
    /// The summary as one sentence for the prompt, or empty when nothing older was about anything.
    pub fn summary_text(&self) -> String {
        if self.summary_lines.is_empty() {
            String::new()
        } else {
            format!("Earlier: {}", self.summary_lines.join("; "))
        }
    }
}

fn answer_phrase(code: &str) -> &'static str {
    match code {
        "lookup" => "looked something up",
        "list" => "asked for a list",
        "count" => "asked for a count",
        "explain" | "story" => "asked for an explanation",
        "describe" => "asked for a description",
        "brief" => "asked for a brief",
        "relation" => "asked how things relate",
        "compare" => "compared things",
        "brainstorm" => "brainstormed",
        "navigate" => "opened a screen",
        "change" => "asked for a change",
        "help" => "asked how the app works",
        _ => "asked",
    }
}

/// "asked for a count about scene 35 of LOW TIDE RADIO, Young Anjali (Aadhya Varma)" from the focus
/// stored with a turn. `None` when the focus names nothing the project still has. With sections
/// picked, a name is written only when a picked section can answer about that kind of thing.
fn line(focus: &Focus, names: Option<&EntityIndex>, within: Option<&Selection>) -> Option<String> {
    let ix = names?;
    let takes = |k: Kind| within.is_none_or(|sel| sel.takes(k));
    let mut about: Vec<String> = Vec::new();
    let title = focus
        .screenplay
        .as_deref()
        .and_then(|id| ix.screenplay(id))
        .map(|s| s.title.clone())
        .filter(|_| takes(Kind::Scene));
    match (focus.scene, &title) {
        (Some(n), Some(t)) => about.push(format!("scene {n} of {t}")),
        (None, Some(t)) => about.push(t.clone()),
        _ => {}
    }
    if takes(Kind::Character) {
        for id in focus.characters.iter().take(2) {
            if let Some(e) = ix.entity(id) {
                let players = ix.played_by(id);
                match players.as_slice() {
                    [one] if takes(Kind::Person) => {
                        about.push(format!("{} ({})", e.name, one.name));
                    }
                    _ => about.push(e.name.clone()),
                }
            }
        }
    }
    if takes(Kind::Person) {
        for id in focus.persons.iter().take(2) {
            if let Some(e) = ix.entity(id) {
                about.push(e.name.clone());
            }
        }
    }
    if about.is_empty() {
        return None;
    }
    let phrase = answer_phrase(focus.answer_type.as_deref().unwrap_or(""));
    Some(truncate_chars(
        &format!("{phrase} about {}", about.join(", ")),
        LINE_CHARS,
    ))
}

fn focus_of(scope_json: &str) -> Focus {
    serde_json::from_str::<Value>(scope_json)
        .ok()
        .and_then(|v| serde_json::from_value(v.get("focus")?.clone()).ok())
        .unwrap_or_default()
}

/// The sections a turn was asked under, or `None` for a turn with no record of them.
fn sections_of(scope_json: &str) -> Option<Vec<String>> {
    let v: Value = serde_json::from_str(scope_json).ok()?;
    serde_json::from_value(v.get("sections")?.clone()).ok()
}

/// A turn read other sections than these are picked now: its words stay out of the prompt.
fn turn_fits(scope_json: &str, within: Option<&Selection>) -> bool {
    let Some(sel) = within else {
        return true;
    };
    sections_of(scope_json).is_some_and(|read| read.iter().all(|leaf| sel.has(leaf)))
}

/// The newest result of a request: kind, content, status and whether it rests on a private note.
struct Shown {
    content: String,
    plain: bool,
}

fn shown(c: &Connection, request_id: &str) -> AppResult<Option<Shown>> {
    let row: Option<(String, String, String, String)> = c
        .query_row(
            "SELECT result_kind, content, status, provenance_json FROM ai_result
             WHERE request_id = ?1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1",
            [request_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    Ok(row.map(|(kind, content, status, provenance)| Shown {
        plain: AiResultKind::parse(&kind) == AiResultKind::Answer
            && AiResultStatus::parse(&status) == AiResultStatus::Informational
            && !provenance.to_lowercase().contains("private"),
        content,
    }))
}

/// Load the conversation memory of `conversation` for the actor, before the new request is stored.
pub fn load(
    c: &Connection,
    actor: &Actor,
    conversation: &str,
    focus: &Focus,
    names: Option<&EntityIndex>,
    within: Option<&Selection>,
) -> AppResult<ConversationMemory> {
    let mut st = c.prepare(
        "SELECT id, request_text, scope_json FROM ai_request
         WHERE user_id = ?1 AND session_id = ?2 AND deleted_at IS NULL
         ORDER BY created_at DESC, id DESC LIMIT ?3",
    )?;
    let rows = st
        .query_map(
            params![
                actor.user_id,
                conversation,
                (RECENT_TURNS + OLDER_LOOKED_AT) as i64
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;

    // Newest first: keep turns while there are fewer than eight and the budget holds. The newest
    // turn is always kept, cut if it alone is long.
    let mut recent: Vec<RecentTurn> = Vec::new();
    let mut used = 0;
    let mut older_from = rows.len();
    for (i, (id, asked, scope_json)) in rows.iter().enumerate() {
        if recent.len() >= RECENT_TURNS {
            older_from = i;
            break;
        }
        if !turn_fits(scope_json, within) {
            continue;
        }
        let Some(answer) = shown(c, id)? else {
            // Still running, or it ended before an answer was stored: nothing to remember of it.
            continue;
        };
        let asked_cut = truncate_chars(asked, TURN_PART_TOKENS * 4);
        let answered_cut = truncate_chars(&answer.content, TURN_PART_TOKENS * 4);
        let cost = approx_tokens(&asked_cut) + approx_tokens(&answered_cut);
        if !recent.is_empty() && used + cost > RECENT_TOKENS {
            older_from = i;
            break;
        }
        used += cost;
        recent.push(RecentTurn {
            confirmable: answer.plain && rules::usable_answer(&answer.content).is_some(),
            asked: asked_cut,
            answered: answered_cut,
        });
    }
    recent.reverse();

    // The older turns, oldest first, one line each; a line equal to the one before it adds nothing.
    let mut lines: Vec<String> = Vec::new();
    for (_, _, scope_json) in rows[older_from.min(rows.len())..].iter().rev() {
        if !turn_fits(scope_json, within) {
            continue;
        }
        if let Some(l) = line(&focus_of(scope_json), names, within)
            && lines.last() != Some(&l)
        {
            lines.push(l);
        }
    }
    let skip = lines.len().saturating_sub(SUMMARY_LINES);
    Ok(ConversationMemory {
        recent,
        summary_lines: lines.split_off(skip),
        focus: focus.clone(),
    })
}

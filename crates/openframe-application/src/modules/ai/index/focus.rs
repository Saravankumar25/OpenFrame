//! What a conversation is about right now: the screenplay, the scene, the people and the kind of
//! the last answer (spec section 5.2: "she", "him" and "that scene" take the focus).
//!
//! It is kept with the conversation, in the `focus` key of the `scope_json` of each request, because
//! the conversation has no table of its own and the focus after a message depends only on the focus
//! before it and the message, so it is written once, when the request is recorded. The newest request
//! of the conversation holds the current focus. References only (ids), never text.

use openframe_domain::AppResult;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::resolve::{How, Resolved};

/// Names kept per kind. "They" means the last few people, not everybody ever mentioned.
const KEPT: usize = 4;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Focus {
    pub screenplay: Option<String>,
    /// 1-based scene of `screenplay`.
    pub scene: Option<u32>,
    pub characters: Vec<String>,
    pub persons: Vec<String>,
    /// The answer type read for the newest message (`turn::AnswerType::code`); empty before the first.
    pub answer_type: Option<String>,
}

impl Focus {
    pub fn is_empty(&self) -> bool {
        *self == Focus::default()
    }

    /// The focus after a message resolved to `r`. Things the message names replace the old ones; a
    /// message that names nothing of a kind leaves that kind as it was.
    pub fn after(&self, r: &Resolved) -> Focus {
        let mut f = self.clone();
        let named = |h: &super::resolve::Hit| h.how != How::Pronoun;
        if let Some(sp) = r.screenplays.first() {
            if f.screenplay.as_deref() != Some(sp.id.as_str()) {
                f.scene = None;
            }
            f.screenplay = Some(sp.id.clone());
        }
        if let Some(s) = r.scenes.iter().rev().find(|s| s.scene_id.is_some()) {
            f.screenplay = Some(s.screenplay_id.clone());
            f.scene = Some(s.ordinal);
        }
        if r.characters.iter().any(named) {
            f.characters = r
                .characters
                .iter()
                .take(KEPT)
                .map(|h| h.id.clone())
                .collect();
        }
        if r.persons.iter().any(named) {
            f.persons = r.persons.iter().take(KEPT).map(|h| h.id.clone()).collect();
        }
        f
    }

    /// The same focus on another screenplay: the scene belonged to the old one.
    pub fn on_screenplay(mut self, id: &str) -> Focus {
        if self.screenplay.as_deref() != Some(id) {
            self.scene = None;
            self.screenplay = Some(id.to_string());
        }
        self
    }

    pub fn contains(&self, id: &str) -> bool {
        self.characters.iter().chain(&self.persons).any(|x| x == id)
    }
}

/// The focus the conversation had after its newest request that recorded one.
pub fn load(c: &Connection, user_id: &str, conversation: &str) -> AppResult<Focus> {
    let mut st = c.prepare(
        "SELECT scope_json FROM ai_request
         WHERE user_id = ?1 AND session_id = ?2 AND deleted_at IS NULL
         ORDER BY created_at DESC, rowid DESC LIMIT 8",
    )?;
    let rows = st
        .query_map(params![user_id, conversation], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .iter()
        .find_map(|s| {
            let v: Value = serde_json::from_str(s).ok()?;
            serde_json::from_value(v.get("focus")?.clone()).ok()
        })
        .unwrap_or_default())
}

/// The newest message the person sent in the conversation before the one being read (slice S4: a
/// correction such as "I asked you" is read against it).
pub fn last_message(
    c: &Connection,
    user_id: &str,
    conversation: &str,
) -> AppResult<Option<String>> {
    Ok(c.query_row(
        "SELECT request_text FROM ai_request
         WHERE user_id = ?1 AND session_id = ?2 AND deleted_at IS NULL
         ORDER BY created_at DESC, rowid DESC LIMIT 1",
        params![user_id, conversation],
        |r| r.get::<_, String>(0),
    )
    .optional()?)
}

/// `scope_json` with the focus added.
pub fn with_focus(mut scope_json: Value, focus: &Focus) -> Value {
    if let Value::Object(m) = &mut scope_json {
        m.insert("focus".into(), json!(focus));
    }
    scope_json
}

//! Character usage detection (Writing Room "Characters" panel).
//!
//! Character cue elements are normalized ("MEERA (V.O.)" → "MEERA") and matched
//! to Story characters by name; the writer can correct a match or mark a cue as
//! not being a character. This is a read-side lookup: nothing is synced between
//! the screenplay and the Story character directory (FSD §13, §63).

use std::collections::HashMap;

use openframe_domain::enums::ElementType;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{opt_text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{load_draft, load_scenes, load_screenplay};
use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::MutationMeta;

pub fn register(r: &mut Registry) {
    r.query("screenplay.characters", characters);
    r.command("screenplay.set_character_link", set_link);
    r.command("screenplay.clear_character_link", clear_link);
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplaySceneRef {
    pub scene_id: String,
    pub number: u32,
    pub heading: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayCharacterCueUsage {
    /// Normalized cue, e.g. "MEERA".
    pub cue_name: String,
    pub character_id: Option<String>,
    pub character_name: Option<String>,
    /// "manual" (writer's correction), "name" (exact), "first_name" (suggested), "ignored", or "none".
    pub match_kind: String,
    pub scenes: Vec<ScreenplaySceneRef>,
    /// Number of speeches (character cues) in the draft.
    pub speeches: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayCharacterRef {
    pub id: String,
    pub name: String,
    pub role_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayCharacterReport {
    pub screenplay_id: String,
    pub cues: Vec<ScreenplayCharacterCueUsage>,
    /// Story characters (for corrections), including ones that never speak in this draft.
    pub characters: Vec<ScreenplayCharacterRef>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayCharactersArgs {
    pub draft_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplaySetCharacterLinkArgs {
    pub screenplay_id: String,
    pub cue_name: String,
    /// The Story character this cue refers to; None with `ignored` = not a character.
    #[serde(default)]
    #[ts(optional)]
    pub character_id: Option<String>,
    #[serde(default)]
    pub ignored: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayClearCharacterLinkArgs {
    pub screenplay_id: String,
    pub cue_name: String,
}

/// "meera (V.O.) ^" → "MEERA". Strips trailing extensions and the dual-dialogue caret.
pub fn normalize_cue(raw: &str) -> String {
    let mut s = raw.trim().trim_end_matches('^').trim().to_uppercase();
    loop {
        let t = s.trim_end().to_string();
        if t.ends_with(')')
            && let Some(open) = t.rfind('(')
            && open > 0
        {
            s = t[..open].to_string();
            continue;
        }
        s = t;
        break;
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn story_characters(c: &Connection) -> AppResult<Vec<ScreenplayCharacterRef>> {
    let mut stmt = c.prepare(
        "SELECT id, name, role_label FROM story_character WHERE deleted_at IS NULL AND archived = 0 ORDER BY position, name",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ScreenplayCharacterRef {
                id: r.get(0)?,
                name: r.get(1)?,
                role_label: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn characters(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayCharactersArgs,
) -> AppResult<ScreenplayCharacterReport> {
    actor.require(Capability::View, "view the screenplay")?;
    core.project()?.store.read(|c| {
        let d = load_draft(c, &a.draft_id)?;
        let chars = story_characters(c)?;
        let by_name: HashMap<String, &ScreenplayCharacterRef> = chars.iter().map(|ch| (normalize_cue(&ch.name), ch)).collect();
        let mut by_first: HashMap<String, Vec<&ScreenplayCharacterRef>> = HashMap::new();
        for ch in &chars {
            if let Some(first) = normalize_cue(&ch.name).split(' ').next() {
                by_first.entry(first.to_string()).or_default().push(ch);
            }
        }
        let mut links: HashMap<String, (Option<String>, bool)> = HashMap::new();
        let mut stmt = c.prepare("SELECT cue_name, character_id, ignored FROM screenplay_character_link WHERE screenplay_id=?1")?;
        for r in stmt.query_map([&d.screenplay_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, bool>(2)?)))? {
            let (cue, ch, ig) = r?;
            links.insert(cue, (ch, ig));
        }
        let mut order: Vec<String> = Vec::new();
        let mut usage: HashMap<String, (Vec<ScreenplaySceneRef>, u32)> = HashMap::new();
        for scene in load_scenes(c, &d.id)? {
            for e in &scene.elements {
                if e.element_type != ElementType::Character {
                    continue;
                }
                let cue = normalize_cue(&e.text);
                if cue.is_empty() {
                    continue;
                }
                let entry = usage.entry(cue.clone()).or_insert_with(|| {
                    order.push(cue.clone());
                    (Vec::new(), 0)
                });
                entry.1 += 1;
                if entry.0.last().map(|s| s.scene_id != scene.id).unwrap_or(true) {
                    entry.0.push(ScreenplaySceneRef { scene_id: scene.id.clone(), number: scene.number, heading: scene.heading.clone() });
                }
            }
        }
        let name_of = |id: &str| chars.iter().find(|ch| ch.id == id).map(|ch| ch.name.clone());
        let cues = order
            .into_iter()
            .map(|cue| {
                let (scenes, speeches) = usage.remove(&cue).unwrap_or_default();
                let (character_id, match_kind) = match links.get(&cue) {
                    Some((_, true)) => (None, "ignored"),
                    Some((Some(id), false)) if name_of(id).is_some() => (Some(id.clone()), "manual"),
                    _ => match by_name.get(&cue) {
                        Some(ch) => (Some(ch.id.clone()), "name"),
                        None => match by_first.get(&cue) {
                            Some(v) if v.len() == 1 => (Some(v[0].id.clone()), "first_name"),
                            _ => (None, "none"),
                        },
                    },
                };
                ScreenplayCharacterCueUsage {
                    character_name: character_id.as_deref().and_then(name_of),
                    character_id,
                    match_kind: match_kind.to_string(),
                    cue_name: cue,
                    scenes,
                    speeches,
                }
            })
            .collect();
        Ok(ScreenplayCharacterReport { screenplay_id: d.screenplay_id, cues, characters: chars })
    })
}

fn set_link(core: &AppCore, actor: &Actor, a: ScreenplaySetCharacterLinkArgs) -> AppResult<()> {
    let cue = normalize_cue(&a.cue_name);
    if cue.is_empty() {
        return Err(AppError::required("Character cue"));
    }
    if a.character_id.is_none() && !a.ignored {
        return Err(AppError::invalid_input(
            "Choose a character, or mark the cue as not a character.",
        ));
    }
    let s = core.project()?;
    let summary = if a.ignored {
        format!("Marked “{cue}” as not a character")
    } else {
        format!("Linked “{cue}” to a character")
    };
    s.store.mutate(actor, MutationMeta::new("screenplay.set_character_link", summary, Capability::Edit), |tx| {
        let c = tx.conn();
        load_screenplay(c, &a.screenplay_id)?;
        if let Some(ch) = &a.character_id {
            let ok: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM story_character WHERE id=?1 AND deleted_at IS NULL)",
                [ch],
                |r| r.get(0),
            )?;
            if !ok {
                return Err(AppError::not_found("character"));
            }
        }
        let existing: Option<String> = c
            .query_row(
                "SELECT id FROM screenplay_character_link WHERE screenplay_id=?1 AND cue_name=?2",
                params![a.screenplay_id, cue],
                |r| r.get(0),
            )
            .optional()?;
        let character = if a.ignored { None } else { a.character_id.clone() };
        match existing {
            Some(id) => {
                update_fields(
                    c,
                    "screenplay_character_link",
                    &id,
                    &[("character_id", opt_text(character)), ("ignored", SqlValue::Integer(a.ignored as i64))],
                    &["character_id", "ignored"],
                    None,
                    "character link",
                )?;
            }
            None => {
                let now = now_ms();
                c.execute(
                    "INSERT INTO screenplay_character_link(id, screenplay_id, cue_name, character_id, ignored, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    params![new_id(), a.screenplay_id, cue, character, a.ignored, now],
                )?;
            }
        }
        Ok(())
    })
}

fn clear_link(core: &AppCore, actor: &Actor, a: ScreenplayClearCharacterLinkArgs) -> AppResult<()> {
    let cue = normalize_cue(&a.cue_name);
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.clear_character_link",
            format!("Reset the match for “{cue}”"),
            Capability::Edit,
        ),
        |tx| {
            tx.conn().execute(
                "DELETE FROM screenplay_character_link WHERE screenplay_id=?1 AND cue_name=?2",
                params![a.screenplay_id, cue],
            )?;
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cues_normalize() {
        assert_eq!(normalize_cue("Meera (V.O.)"), "MEERA");
        assert_eq!(normalize_cue("ARJUN (CONT'D) (O.S.)"), "ARJUN");
        assert_eq!(normalize_cue("  mrs.   rao ^"), "MRS. RAO");
        assert_eq!(normalize_cue("(beat)"), "(BEAT)");
    }
}

//! Read-only access to the screenplay hub tables (migration 0004 contract
//! columns). Production reads screenplay scenes of the selected Production
//! Source; it never writes screenplay tables (FSD §1.4, §53, §124).
//!
//! Display scene numbers are never stored: they are derived from `position`
//! within a draft (Domain §2.2).

use std::collections::HashMap;

use openframe_domain::AppResult;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

/// A screenplay draft as production sees it (source selection, banners).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionDraftInfo {
    pub id: String,
    pub name: String,
    /// Draft | Review | Locked | Revision
    pub status: String,
    pub screenplay_id: String,
    pub screenplay_title: String,
    pub episode_id: Option<String>,
    #[ts(type = "number")]
    pub scene_count: i64,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number | null")]
    pub locked_at: Option<i64>,
    /// The screenplay's current working draft.
    pub is_current: bool,
    /// The draft was deleted in the Screenplay workspace (historical sources only).
    pub deleted: bool,
}

const DRAFT_SQL: &str = "SELECT d.id, d.name, d.status, d.screenplay_id, s.title, s.episode_id,
        (SELECT count(*) FROM screenplay_scene sc WHERE sc.draft_id = d.id AND sc.deleted_at IS NULL),
        d.created_at, d.locked_at, (s.current_draft_id IS d.id), (d.deleted_at IS NOT NULL OR s.deleted_at IS NOT NULL)
     FROM screenplay_draft d JOIN screenplay s ON s.id = d.screenplay_id";

fn draft_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ProductionDraftInfo> {
    Ok(ProductionDraftInfo {
        id: r.get(0)?,
        name: r.get(1)?,
        status: r.get(2)?,
        screenplay_id: r.get(3)?,
        screenplay_title: r.get(4)?,
        episode_id: r.get(5)?,
        scene_count: r.get(6)?,
        created_at: r.get(7)?,
        locked_at: r.get(8)?,
        is_current: r.get(9)?,
        deleted: r.get(10)?,
    })
}

pub fn draft_info(c: &Connection, draft_id: &str) -> AppResult<Option<ProductionDraftInfo>> {
    Ok(c.query_row(
        &format!("{DRAFT_SQL} WHERE d.id = ?1"),
        [draft_id],
        draft_row,
    )
    .optional()?)
}

/// All live drafts, newest first within each screenplay.
pub fn list_drafts(c: &Connection) -> AppResult<Vec<ProductionDraftInfo>> {
    let mut stmt = c.prepare(&format!(
        "{DRAFT_SQL} WHERE d.deleted_at IS NULL AND s.deleted_at IS NULL ORDER BY s.created_at, d.created_at DESC, d.id"
    ))?;
    let rows = stmt
        .query_map([], draft_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// The newest live draft of the same screenplay created after `draft`, if any
/// (FSD §160 "Newer revision available").
pub fn newer_draft(
    c: &Connection,
    draft: &ProductionDraftInfo,
) -> AppResult<Option<ProductionDraftInfo>> {
    Ok(c.query_row(
        &format!(
            "{DRAFT_SQL} WHERE d.screenplay_id = ?1 AND d.id <> ?2 AND d.deleted_at IS NULL AND d.created_at > ?3
             ORDER BY d.created_at DESC, d.id DESC LIMIT 1"
        ),
        params![draft.screenplay_id, draft.id, draft.created_at],
        draft_row,
    )
    .optional()?)
}

#[derive(Debug, Clone)]
pub struct SceneRow {
    pub id: String,
    pub lineage_id: String,
    /// Derived display number (1-based order within the draft).
    pub number: String,
    pub heading: String,
    pub omitted: bool,
    pub index: usize,
    pub updated_at: i64,
}

/// Scenes of a draft in script order with derived numbers.
pub fn draft_scenes(c: &Connection, draft_id: &str) -> AppResult<Vec<SceneRow>> {
    let mut stmt = c.prepare(
        "SELECT id, lineage_id, heading, omitted, updated_at FROM screenplay_scene
         WHERE draft_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let rows = stmt
        .query_map([draft_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .enumerate()
        .map(
            |(i, (id, lineage_id, heading, omitted, updated_at))| SceneRow {
                id,
                lineage_id,
                number: (i + 1).to_string(),
                heading,
                omitted,
                index: i,
                updated_at,
            },
        )
        .collect())
}

/// Minimal scene lookup (any draft, including deleted scenes for historical display).
pub struct SceneLookup {
    pub id: String,
    pub draft_id: String,
    pub lineage_id: String,
    pub heading: String,
    pub deleted: bool,
}

pub fn scene_lookup(c: &Connection, scene_id: &str) -> AppResult<Option<SceneLookup>> {
    Ok(c.query_row(
        "SELECT id, draft_id, lineage_id, heading, deleted_at IS NOT NULL FROM screenplay_scene WHERE id = ?1",
        [scene_id],
        |r| Ok(SceneLookup { id: r.get(0)?, draft_id: r.get(1)?, lineage_id: r.get(2)?, heading: r.get(3)?, deleted: r.get(4)? }),
    )
    .optional()?)
}

#[derive(Debug, Clone)]
pub struct ElementRow {
    pub id: String,
    pub element_type: String,
    pub text: String,
    pub updated_at: i64,
}

pub fn scene_elements(c: &Connection, scene_id: &str) -> AppResult<Vec<ElementRow>> {
    let mut stmt = c.prepare(
        "SELECT id, element_type, text, updated_at FROM screenplay_element WHERE scene_id = ?1 ORDER BY position, id",
    )?;
    let rows = stmt
        .query_map([scene_id], |r| {
            Ok(ElementRow {
                id: r.get(0)?,
                element_type: r.get(1)?,
                text: r.get(2)?,
                updated_at: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Elements of every scene of a draft, keyed by scene id (one query).
pub fn draft_elements(
    c: &Connection,
    draft_id: &str,
) -> AppResult<HashMap<String, Vec<ElementRow>>> {
    let mut stmt = c.prepare(
        "SELECT e.scene_id, e.id, e.element_type, e.text, e.updated_at FROM screenplay_element e
         JOIN screenplay_scene s ON s.id = e.scene_id
         WHERE s.draft_id = ?1 AND s.deleted_at IS NULL ORDER BY e.scene_id, e.position, e.id",
    )?;
    let mut out: HashMap<String, Vec<ElementRow>> = HashMap::new();
    let rows = stmt.query_map([draft_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            ElementRow {
                id: r.get(1)?,
                element_type: r.get(2)?,
                text: r.get(3)?,
                updated_at: r.get(4)?,
            },
        ))
    })?;
    for row in rows {
        let (scene, el) = row?;
        out.entry(scene).or_default().push(el);
    }
    Ok(out)
}

/// The heading shown for a scene: the scene's heading, or its first heading element.
pub fn effective_heading(heading: &str, elements: &[ElementRow]) -> String {
    let h = heading.trim();
    if !h.is_empty() {
        return h.to_string();
    }
    elements
        .iter()
        .find(|e| e.element_type == "scene_heading")
        .map(|e| e.text.trim().to_string())
        .unwrap_or_default()
}

/// Serialized scene content used as the production baseline and for change detection.
/// Format: first line = heading; then one `type\ttext` line per element (text newlines escaped).
pub fn encode_scene(heading: &str, elements: &[ElementRow]) -> String {
    let mut s = String::with_capacity(256);
    s.push_str(heading.trim());
    for e in elements {
        if e.element_type == "scene_heading" || e.element_type == "note" {
            continue;
        }
        s.push('\n');
        s.push_str(&e.element_type);
        s.push('\t');
        s.push_str(&e.text.replace('\\', "\\\\").replace('\n', "\\n"));
    }
    s
}

/// Inverse of [`encode_scene`]: (heading, [(type, text)]).
pub fn decode_scene(encoded: &str) -> (String, Vec<(String, String)>) {
    let mut lines = encoded.split('\n');
    let heading = lines.next().unwrap_or_default().to_string();
    let els = lines
        .filter_map(|l| {
            let (t, x) = l.split_once('\t')?;
            Some((t.to_string(), unescape(x)))
        })
        .collect();
    (heading, els)
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some(o) => out.push(o),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn fingerprint(encoded: &str) -> String {
    openframe_security::sha256_bytes(encoded.as_bytes())
}

/// Character name from a character cue: extensions such as (V.O.), (O.S.),
/// (CONT'D) and the dual-dialogue caret are removed; returned upper-case.
pub fn cue_name(cue: &str) -> Option<String> {
    let mut out = String::with_capacity(cue.len());
    let mut depth = 0i32;
    for ch in cue.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            '^' => {}
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    let mut name = out.trim().to_uppercase();
    for suffix in ["CONT'D", "CONT’D", "CONTD"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            name = stripped.trim().to_string();
        }
    }
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() || !name.chars().any(|c| c.is_alphabetic()) {
        None
    } else {
        Some(name)
    }
}

/// True when the cue carries a voice-over extension.
pub fn cue_is_voice_over(cue: &str) -> bool {
    let u = cue.to_uppercase();
    u.contains("(V.O.)") || u.contains("(V.O)") || u.contains("(VO)")
}

/// Latest change time of a scene (its own row or any element).
pub fn scene_changed_at(scene_updated_at: i64, elements: &[ElementRow]) -> i64 {
    elements
        .iter()
        .map(|e| e.updated_at)
        .chain(std::iter::once(scene_updated_at))
        .max()
        .unwrap_or(scene_updated_at)
}

/// Story characters (read-only; owned by the story module): (id, name).
pub fn story_characters(c: &Connection) -> AppResult<Vec<(String, String)>> {
    let mut stmt = c.prepare(
        "SELECT id, name FROM story_character WHERE deleted_at IS NULL AND archived = 0 ORDER BY position, name",
    )?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn story_character_name(c: &Connection, id: &str) -> AppResult<Option<String>> {
    Ok(c.query_row(
        "SELECT name FROM story_character WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        |r| r.get(0),
    )
    .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cue_names_drop_extensions() {
        assert_eq!(cue_name("ARJUN (V.O.)").as_deref(), Some("ARJUN"));
        assert_eq!(cue_name("meera (CONT'D)").as_deref(), Some("MEERA"));
        assert_eq!(
            cue_name("THE STATION MASTER ^").as_deref(),
            Some("THE STATION MASTER")
        );
        assert_eq!(cue_name("RAVI CONT'D").as_deref(), Some("RAVI"));
        assert_eq!(cue_name("(beat)"), None);
        assert!(cue_is_voice_over("ARJUN (V.O.)"));
    }

    #[test]
    fn scene_encoding_round_trips() {
        let els = vec![
            ElementRow {
                id: "1".into(),
                element_type: "action".into(),
                text: "Line one\nline two \\ ok".into(),
                updated_at: 0,
            },
            ElementRow {
                id: "2".into(),
                element_type: "character".into(),
                text: "ARJUN".into(),
                updated_at: 0,
            },
        ];
        let enc = encode_scene("INT. HOUSE - DAY", &els);
        let (h, back) = decode_scene(&enc);
        assert_eq!(h, "INT. HOUSE - DAY");
        assert_eq!(
            back[0],
            ("action".to_string(), "Line one\nline two \\ ok".to_string())
        );
        assert_eq!(back[1].1, "ARJUN");
        assert_eq!(
            fingerprint(&enc),
            fingerprint(&encode_scene("INT. HOUSE - DAY", &els))
        );
    }
}

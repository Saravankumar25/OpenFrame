//! Project values for the screenplay editor's smart suggestion list (SCRIPT-018).
//!
//! The editor builds the list itself: scene heading prefixes and time words are fixed, and
//! character names, locations and parentheticals already typed in the script are read from the
//! live document. This query adds what lives elsewhere in the project: Story Board characters,
//! cast character names, production locations and the locations of Story Board scene headings.
//! The result is deterministic (fixed order, case-insensitive de-duplication) and bounded.

use std::collections::HashSet;

use openframe_domain::{Actor, AppResult, Capability};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::Registry;

/// At most this many values of each kind are returned.
pub const MAX_VALUES: usize = 500;

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.query("writing.suggestion_sources", sources).meta(
        M::read("Character names and locations from the Story Board, cast and production locations, for the editor's suggestion list.")
            .hidden(h::UI_FLOW),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum SuggestionSource {
    Story,
    Cast,
    Production,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionValue {
    /// As the writer types it in the script (character cues and locations in capitals).
    pub name: String,
    pub source: SuggestionSource,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WritingSuggestionSources {
    pub characters: Vec<SuggestionValue>,
    pub locations: Vec<SuggestionValue>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WritingSuggestionSourcesArgs {}

fn sources(
    core: &AppCore,
    actor: &Actor,
    _a: WritingSuggestionSourcesArgs,
) -> AppResult<WritingSuggestionSources> {
    actor.require(Capability::View, "view the screenplay")?;
    core.project()?.store.read(load_sources)
}

pub(crate) fn load_sources(c: &Connection) -> AppResult<WritingSuggestionSources> {
    let mut characters = Bounded::default();
    let mut st = c.prepare(
        "SELECT name FROM story_character WHERE deleted_at IS NULL AND archived = 0 ORDER BY position, name",
    )?;
    for name in st.query_map([], |r| r.get::<_, String>(0))? {
        characters.push(cue_name(&name?), SuggestionSource::Story);
    }
    let mut st = c.prepare(
        "SELECT COALESCE(ch.name, cm.character_name) FROM cast_member cm
         LEFT JOIN story_character ch ON ch.id = cm.character_id AND ch.deleted_at IS NULL
         WHERE cm.deleted_at IS NULL AND cm.archived = 0
         ORDER BY cm.created_at, cm.id",
    )?;
    for name in st.query_map([], |r| r.get::<_, Option<String>>(0))? {
        if let Some(n) = name? {
            characters.push(cue_name(&n), SuggestionSource::Cast);
        }
    }

    let mut locations = Bounded::default();
    let mut st = c.prepare(
        "SELECT name FROM location WHERE deleted_at IS NULL AND archived = 0 AND status <> 'Rejected' ORDER BY name, id",
    )?;
    for name in st.query_map([], |r| r.get::<_, String>(0))? {
        locations.push(cue_name(&name?), SuggestionSource::Production);
    }
    let mut st = c.prepare(
        "SELECT scene_heading FROM story_scene_card
         WHERE deleted_at IS NULL AND scene_heading IS NOT NULL AND TRIM(scene_heading) <> ''
         ORDER BY position, id",
    )?;
    for heading in st.query_map([], |r| r.get::<_, String>(0))? {
        let (_, loc, _) = heading_parts(&heading?);
        locations.push(loc, SuggestionSource::Story);
    }
    Ok(WritingSuggestionSources {
        characters: characters.out,
        locations: locations.out,
    })
}

#[derive(Default)]
struct Bounded {
    seen: HashSet<String>,
    out: Vec<SuggestionValue>,
}

impl Bounded {
    fn push(&mut self, name: String, source: SuggestionSource) {
        if name.is_empty() || name.chars().count() > 120 || self.out.len() >= MAX_VALUES {
            return;
        }
        if self.seen.insert(name.to_lowercase()) {
            self.out.push(SuggestionValue { name, source });
        }
    }
}

/// Names are written in capitals in character cues and scene headings.
fn cue_name(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

/// Scene heading prefixes the editor offers, longest first so `INT./EXT.` wins over `INT.`.
pub const HEADING_PREFIXES: &[&str] = &[
    "INT./EXT.",
    "EXT./INT.",
    "INT/EXT",
    "I/E.",
    "I/E",
    "INT.",
    "EXT.",
    "EST.",
    "INT ",
    "EXT ",
];

/// Split "INT. KITCHEN - NIGHT" into ("INT.", "KITCHEN", "NIGHT"). The location is in capitals;
/// any part may be empty. The time is whatever follows the last spaced dash.
pub fn heading_parts(heading: &str) -> (String, String, String) {
    let h = heading.trim();
    let upper = h.to_uppercase();
    let mut prefix = String::new();
    let mut rest = upper.as_str();
    for p in HEADING_PREFIXES {
        if let Some(r) = rest.strip_prefix(p) {
            prefix = p.trim().to_string();
            rest = r;
            break;
        }
    }
    let rest = rest.trim();
    let mut location = rest.to_string();
    let mut time = String::new();
    for sep in [" - ", " \u{2013} ", " \u{2014} "] {
        if let Some(i) = rest.rfind(sep) {
            location = rest[..i].trim().to_string();
            time = rest[i + sep.len()..].trim().to_string();
            break;
        }
    }
    (prefix, cue_name(&location), time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_parts_split_prefix_location_and_time() {
        assert_eq!(
            heading_parts("INT. KITCHEN - NIGHT"),
            ("INT.".into(), "KITCHEN".into(), "NIGHT".into())
        );
        assert_eq!(
            heading_parts("int./ext. old  bus depot \u{2013} moments later"),
            (
                "INT./EXT.".into(),
                "OLD BUS DEPOT".into(),
                "MOMENTS LATER".into()
            )
        );
        assert_eq!(
            heading_parts("I/E CAR - DAY"),
            ("I/E".into(), "CAR".into(), "DAY".into())
        );
        assert_eq!(
            heading_parts("EXT. RAILWAY STATION"),
            ("EXT.".into(), "RAILWAY STATION".into(), String::new())
        );
        assert_eq!(
            heading_parts(""),
            (String::new(), String::new(), String::new())
        );
        // Telugu location names keep their script.
        assert_eq!(heading_parts("INT. రైల్వే స్టేషన్ - DAY").1, "రైల్వే స్టేషన్");
    }

    #[test]
    fn values_are_deduplicated_case_insensitively_and_bounded() {
        let mut b = Bounded::default();
        b.push("MEERA".into(), SuggestionSource::Story);
        b.push("meera".into(), SuggestionSource::Cast);
        b.push(String::new(), SuggestionSource::Cast);
        for i in 0..(MAX_VALUES + 10) {
            b.push(format!("EXTRA {i}"), SuggestionSource::Cast);
        }
        assert_eq!(b.out.len(), MAX_VALUES);
        assert_eq!(b.out[0].source, SuggestionSource::Story);
    }
}

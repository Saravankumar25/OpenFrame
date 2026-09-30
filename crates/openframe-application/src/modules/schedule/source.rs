//! Read-only projections over hub tables (screenplay + production) used by the
//! schedule, call sheets, sides and reports. This module never writes: the
//! schedule references screenplay scenes; it does not own or rewrite them.

use std::collections::HashMap;

use openframe_domain::AppResult;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::fmt::fnv_hex;

/// The Production Source a schedule is (or would be) built from.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSourceInfo {
    pub id: String,
    pub draft_id: String,
    pub draft_name: String,
    pub draft_status: String,
    pub screenplay_title: String,
    /// e.g. "Shooting Draft 6 (Locked)".
    pub label: String,
    #[ts(type = "number")]
    pub scene_count: i64,
}

/// A screenplay scene as the schedule sees it (derived, never stored).
#[derive(Debug, Clone)]
pub struct SceneInfo {
    pub id: String,
    pub lineage_id: String,
    /// Display number derived from order within the draft.
    pub number: i64,
    pub heading: String,
    pub int_ext: String,
    pub day_night: String,
    pub time_label: String,
    pub heading_location: String,
    pub synopsis: String,
    pub page_eighths: i64,
    pub text_hash: String,
    pub omitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleCastRef {
    /// Stable key: `char:<character id>` or `name:<CHARACTER NAME>`.
    pub key: String,
    pub character: String,
    pub actor: Option<String>,
    pub initials: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleLocRef {
    /// `loc:<location id>`, `item:<catalog id>` or `heading:<TEXT>`.
    pub key: String,
    pub name: String,
    pub address: Option<String>,
    /// Location record status (Idea/Shortlisted/Confirmed/Rejected) when linked.
    pub status: Option<String>,
    pub location_id: Option<String>,
    /// False when only the scene heading names the place (no confirmed breakdown).
    pub from_breakdown: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleItemRef {
    pub category: String,
    pub name: String,
    pub catalog_item_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ElementText {
    pub element_type: String,
    pub text: String,
}

/// Everything the schedule derives from one Production Source.
pub struct SourceData {
    pub info: ScheduleSourceInfo,
    pub scenes: Vec<SceneInfo>,
    by_id: HashMap<String, usize>,
    by_lineage: HashMap<String, Vec<usize>>,
    cast: HashMap<String, Vec<ScheduleCastRef>>,
    locations: HashMap<String, Vec<ScheduleLocRef>>,
    items: HashMap<String, Vec<ScheduleItemRef>>,
    breakdown_counts: HashMap<String, (i64, i64)>,
}

impl SourceData {
    pub fn scene(&self, id: &str) -> Option<&SceneInfo> {
        self.by_id.get(id).map(|i| &self.scenes[*i])
    }
    pub fn scenes_for_lineage(&self, lineage: &str) -> Vec<&SceneInfo> {
        self.by_lineage
            .get(lineage)
            .map(|v| v.iter().map(|i| &self.scenes[*i]).collect())
            .unwrap_or_default()
    }
    pub fn cast(&self, scene_id: &str) -> &[ScheduleCastRef] {
        self.cast.get(scene_id).map(|v| v.as_slice()).unwrap_or(&[])
    }
    /// Locations from confirmed breakdown, else the place named in the heading.
    pub fn locations(&self, scene: &SceneInfo) -> Vec<ScheduleLocRef> {
        match self.locations.get(&scene.id) {
            Some(v) if !v.is_empty() => v.clone(),
            _ if scene.heading_location.trim().is_empty() => Vec::new(),
            _ => vec![ScheduleLocRef {
                key: format!("heading:{}", scene.heading_location.trim().to_uppercase()),
                name: title_case(&scene.heading_location),
                address: None,
                status: None,
                location_id: None,
                from_breakdown: false,
            }],
        }
    }
    /// Confirmed non-cast, non-location breakdown items (props, wardrobe…).
    pub fn items(&self, scene_id: &str) -> &[ScheduleItemRef] {
        self.items
            .get(scene_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
    /// (confirmed/manual, suggested-and-unreviewed) breakdown element counts.
    pub fn breakdown_counts(&self, scene_id: &str) -> (i64, i64) {
        self.breakdown_counts
            .get(scene_id)
            .copied()
            .unwrap_or((0, 0))
    }
}

/// The active Production Source (created by the production module).
pub fn active_source(c: &Connection) -> AppResult<Option<ScheduleSourceInfo>> {
    let id: Option<String> = c
        .query_row(
            "SELECT id FROM production_source WHERE active = 1 ORDER BY selected_at DESC, id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    match id {
        Some(id) => source_info(c, &id),
        None => Ok(None),
    }
}

pub fn source_info(c: &Connection, source_id: &str) -> AppResult<Option<ScheduleSourceInfo>> {
    let row: Option<(String, String, String, String, String)> = c
        .query_row(
            "SELECT ps.id, ps.draft_id, d.name, d.status, s.title
             FROM production_source ps
             JOIN screenplay_draft d ON d.id = ps.draft_id
             JOIN screenplay s ON s.id = d.screenplay_id
             WHERE ps.id = ?1",
            [source_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let Some((id, draft_id, draft_name, draft_status, screenplay_title)) = row else {
        return Ok(None);
    };
    let scene_count: i64 = c.query_row(
        "SELECT count(*) FROM screenplay_scene WHERE draft_id = ?1 AND deleted_at IS NULL AND omitted = 0",
        [&draft_id],
        |r| r.get(0),
    )?;
    let label = if draft_status == "Draft" {
        draft_name.clone()
    } else {
        format!("{draft_name} ({draft_status})")
    };
    Ok(Some(ScheduleSourceInfo {
        id,
        draft_id,
        draft_name,
        draft_status,
        screenplay_title,
        label,
        scene_count,
    }))
}

/// Load scenes, page estimates and confirmed breakdown data for a source.
pub fn load_source(c: &Connection, source_id: &str) -> AppResult<Option<SourceData>> {
    let Some(info) = source_info(c, source_id)? else {
        return Ok(None);
    };
    let draft = info.draft_id.clone();

    // Elements (for page estimate, text fingerprint and synopsis fallback).
    let mut elements: HashMap<String, Vec<ElementText>> = HashMap::new();
    {
        let mut st = c.prepare(
            "SELECT e.scene_id, e.element_type, e.text FROM screenplay_element e
             JOIN screenplay_scene s ON s.id = e.scene_id
             WHERE s.draft_id = ?1 AND s.deleted_at IS NULL ORDER BY e.scene_id, e.position, e.id",
        )?;
        let rows = st.query_map([&draft], |r| {
            Ok((
                r.get::<_, String>(0)?,
                ElementText {
                    element_type: r.get(1)?,
                    text: r.get(2)?,
                },
            ))
        })?;
        for row in rows {
            let (sid, el) = row?;
            elements.entry(sid).or_default().push(el);
        }
    }

    let mut scenes = Vec::new();
    {
        let mut st = c.prepare(
            "SELECT id, lineage_id, heading, synopsis, omitted FROM screenplay_scene
             WHERE draft_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
        )?;
        let rows = st.query_map([&draft], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, i64>(4)? != 0,
            ))
        })?;
        for (i, row) in rows.enumerate() {
            let (id, lineage_id, heading, synopsis, omitted) = row?;
            let els = elements.get(&id).map(|v| v.as_slice()).unwrap_or(&[]);
            let parsed = parse_heading(&heading);
            let synopsis = synopsis
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    els.iter()
                        .find(|e| e.element_type == "action" && !e.text.trim().is_empty())
                        .map(|e| e.text.clone())
                })
                .map(|s| truncate(&s, 90))
                .unwrap_or_default();
            scenes.push(SceneInfo {
                page_eighths: page_eighths(&heading, els),
                text_hash: text_hash(els),
                id,
                lineage_id,
                number: i as i64 + 1,
                heading: heading.trim().to_string(),
                int_ext: parsed.int_ext,
                day_night: parsed.day_night,
                time_label: parsed.time_label,
                heading_location: parsed.location,
                synopsis,
                omitted,
            });
        }
    }
    let mut by_id = HashMap::new();
    let mut by_lineage: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, s) in scenes.iter().enumerate() {
        by_id.insert(s.id.clone(), i);
        by_lineage.entry(s.lineage_id.clone()).or_default().push(i);
    }

    // Cast member (actor) per character, primary first.
    let mut actors: HashMap<String, String> = HashMap::new();
    {
        let mut st = c.prepare(
            "SELECT character_id, person_name FROM cast_member
             WHERE character_id IS NOT NULL AND deleted_at IS NULL AND archived = 0
             ORDER BY is_primary DESC, created_at ASC",
        )?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (ch, name) = row?;
            actors.entry(ch).or_insert(name);
        }
    }
    let mut locs_by_id: HashMap<String, (String, Option<String>, String)> = HashMap::new();
    {
        let mut st =
            c.prepare("SELECT id, name, address, status FROM location WHERE deleted_at IS NULL")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (id, name, addr, status) = row?;
            locs_by_id.insert(id, (name, addr.filter(|a| !a.trim().is_empty()), status));
        }
    }

    let mut cast: HashMap<String, Vec<ScheduleCastRef>> = HashMap::new();
    let mut locations: HashMap<String, Vec<ScheduleLocRef>> = HashMap::new();
    let mut items: HashMap<String, Vec<ScheduleItemRef>> = HashMap::new();
    let mut breakdown_counts: HashMap<String, (i64, i64)> = HashMap::new();
    {
        let mut st = c.prepare(
            "SELECT be.scene_id, be.category, be.display_name, be.catalog_item_id, ci.name, ci.character_id,
                    ci.location_id, ch.name, be.confirmation_state
             FROM breakdown_element be
             JOIN screenplay_scene s ON s.id = be.scene_id
             LEFT JOIN catalog_item ci ON ci.id = be.catalog_item_id AND ci.deleted_at IS NULL
             LEFT JOIN story_character ch ON ch.id = ci.character_id AND ch.deleted_at IS NULL
             WHERE s.draft_id = ?1 AND be.source_id = ?2 AND be.archived = 0 AND be.deleted_at IS NULL
             ORDER BY be.scene_id, be.category, be.created_at, be.id",
        )?;
        #[allow(clippy::type_complexity)]
        let rows = st.query_map([draft.as_str(), source_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, String>(8)?,
            ))
        })?;
        for row in rows {
            let (
                scene_id,
                category,
                display,
                item_id,
                item_name,
                character_id,
                location_id,
                character_name,
                state,
            ) = row?;
            let counts = breakdown_counts.entry(scene_id.clone()).or_default();
            match state.as_str() {
                "Confirmed" | "Manual" => counts.0 += 1,
                "Suggested" => {
                    counts.1 += 1;
                    continue;
                }
                _ => continue,
            }
            match category.as_str() {
                "Cast" => {
                    let character = character_name.or(item_name).unwrap_or(display);
                    let key = match &character_id {
                        Some(id) => format!("char:{id}"),
                        None => format!("name:{}", character.trim().to_uppercase()),
                    };
                    let list = cast.entry(scene_id).or_default();
                    if list.iter().any(|c| c.key == key) {
                        continue;
                    }
                    list.push(ScheduleCastRef {
                        actor: character_id.as_ref().and_then(|id| actors.get(id).cloned()),
                        initials: initials(&character),
                        character: character.trim().to_string(),
                        key,
                    });
                }
                "Location / Set" => {
                    let loc = match location_id
                        .as_ref()
                        .and_then(|id| locs_by_id.get(id).map(|l| (id, l)))
                    {
                        Some((id, (name, addr, status))) => ScheduleLocRef {
                            key: format!("loc:{id}"),
                            name: name.clone(),
                            address: addr.clone(),
                            status: Some(status.clone()),
                            location_id: Some(id.clone()),
                            from_breakdown: true,
                        },
                        None => {
                            let name = item_name.unwrap_or(display);
                            ScheduleLocRef {
                                key: match &item_id {
                                    Some(id) => format!("item:{id}"),
                                    None => format!("heading:{}", name.trim().to_uppercase()),
                                },
                                name: name.trim().to_string(),
                                address: None,
                                status: None,
                                location_id: None,
                                from_breakdown: true,
                            }
                        }
                    };
                    let list = locations.entry(scene_id).or_default();
                    if !list.iter().any(|l| l.key == loc.key) {
                        list.push(loc);
                    }
                }
                _ => {
                    let name = item_name.unwrap_or(display);
                    let list = items.entry(scene_id).or_default();
                    if !list
                        .iter()
                        .any(|i| i.category == category && i.name == name)
                    {
                        list.push(ScheduleItemRef {
                            category,
                            name,
                            catalog_item_id: item_id,
                        });
                    }
                }
            }
        }
    }

    Ok(Some(SourceData {
        info,
        scenes,
        by_id,
        by_lineage,
        cast,
        locations,
        items,
        breakdown_counts,
    }))
}

/// Screenplay text of a set of scenes (for Sides).
pub fn scene_elements(c: &Connection, scene_id: &str) -> AppResult<Vec<ElementText>> {
    let mut st = c.prepare(
        "SELECT element_type, text FROM screenplay_element WHERE scene_id = ?1 ORDER BY position, id",
    )?;
    let rows = st.query_map(params![scene_id], |r| {
        Ok(ElementText {
            element_type: r.get(0)?,
            text: r.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

// ------------------------------------------------------------------ parsing

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHeading {
    /// "INT", "EXT", "INT/EXT" or "" when the heading has no recognised prefix.
    pub int_ext: String,
    /// "D", "N" or "" (unknown).
    pub day_night: String,
    pub time_label: String,
    pub location: String,
}

/// Parse a scene heading such as `INT. POLICE STATION - NIGHT`.
pub fn parse_heading(heading: &str) -> ParsedHeading {
    let h = heading.trim();
    let upper = h.to_uppercase();
    const PREFIXES: &[(&str, &str)] = &[
        ("INT./EXT.", "INT/EXT"),
        ("EXT./INT.", "INT/EXT"),
        ("INT/EXT.", "INT/EXT"),
        ("EXT/INT.", "INT/EXT"),
        ("INT/EXT", "INT/EXT"),
        ("EXT/INT", "INT/EXT"),
        ("INT.EXT.", "INT/EXT"),
        ("I/E.", "INT/EXT"),
        ("I/E", "INT/EXT"),
        ("INT.", "INT"),
        ("EXT.", "EXT"),
        ("EST.", "EXT"),
        ("INT ", "INT"),
        ("EXT ", "EXT"),
    ];
    let mut int_ext = String::new();
    let mut rest = h;
    for (p, v) in PREFIXES {
        if upper.starts_with(p) {
            int_ext = (*v).to_string();
            rest = &h[p.len()..];
            break;
        }
    }
    let rest = rest.trim();
    // The time of day follows the last dash separator.
    let mut location = rest.to_string();
    let mut time_label = String::new();
    for sep in [" - ", " — ", " – ", " -- "] {
        if let Some(i) = rest.rfind(sep) {
            location = rest[..i].trim().to_string();
            time_label = rest[i + sep.len()..].trim().to_string();
            break;
        }
    }
    let location = location
        .trim_matches(|c: char| c == '.' || c == '-' || c.is_whitespace())
        .to_string();
    let t = time_label.to_uppercase();
    let day_night = if ["NIGHT", "EVENING", "DUSK", "MIDNIGHT", "NITE", "TWILIGHT"]
        .iter()
        .any(|w| t.contains(w))
    {
        "N"
    } else if [
        "DAY",
        "MORNING",
        "DAWN",
        "AFTERNOON",
        "NOON",
        "SUNRISE",
        "SUNSET",
    ]
    .iter()
    .any(|w| t.contains(w))
    {
        "D"
    } else {
        ""
    };
    ParsedHeading {
        int_ext,
        day_night: day_night.to_string(),
        time_label,
        location,
    }
}

/// Estimated page length in eighths: screenplay text lines / 55 lines per page.
pub fn page_eighths(heading: &str, els: &[ElementText]) -> i64 {
    let mut lines: i64 = 0;
    if !els.iter().any(|e| e.element_type == "scene_heading") && !heading.trim().is_empty() {
        lines += 2;
    }
    for e in els {
        let width: i64 = match e.element_type.as_str() {
            "dialogue" => 35,
            "parenthetical" => 25,
            "character" | "transition" => 40,
            _ => 60,
        };
        let body: i64 = e
            .text
            .split('\n')
            .map(|l| {
                let n = l.chars().count() as i64;
                ((n + width - 1) / width).max(1)
            })
            .sum::<i64>()
            .max(1);
        let spacing = if matches!(e.element_type.as_str(), "dialogue" | "parenthetical") {
            0
        } else {
            1
        };
        lines += body + spacing;
    }
    ((lines * 8 + 54) / 55).max(1)
}

pub fn text_hash(els: &[ElementText]) -> String {
    let mut s = String::new();
    for e in els {
        s.push_str(&e.element_type);
        s.push('\u{1}');
        s.push_str(&e.text);
        s.push('\u{2}');
    }
    fnv_hex(&s)
}

pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split_whitespace()
        .filter(|w| w.chars().next().is_some_and(|c| c.is_alphanumeric()))
        .collect();
    match words.as_slice() {
        [] => "?".into(),
        [one] => one
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(2)
            .collect::<String>()
            .to_uppercase(),
        [first, .., last] => {
            let mut s = String::new();
            s.extend(first.chars().next());
            s.extend(last.chars().next());
            s.to_uppercase()
        }
    }
}

pub fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let lower = w.to_lowercase();
            let mut ch = lower.chars();
            match ch.next() {
                Some(f) => f.to_uppercase().collect::<String>() + ch.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn truncate(s: &str, max: usize) -> String {
    let one_line = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let mut t: String = one_line.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_headings() {
        let p = parse_heading("INT. POLICE STATION - NIGHT");
        assert_eq!(
            (
                p.int_ext.as_str(),
                p.day_night.as_str(),
                p.location.as_str()
            ),
            ("INT", "N", "POLICE STATION")
        );
        let p = parse_heading("EXT. OLD RAILWAY STATION — DAY");
        assert_eq!(
            (
                p.int_ext.as_str(),
                p.day_night.as_str(),
                p.location.as_str()
            ),
            ("EXT", "D", "OLD RAILWAY STATION")
        );
        let p = parse_heading("INT./EXT. CAR - MOVING - DUSK");
        assert_eq!(
            (
                p.int_ext.as_str(),
                p.day_night.as_str(),
                p.location.as_str()
            ),
            ("INT/EXT", "N", "CAR - MOVING")
        );
        let p = parse_heading("MONTAGE");
        assert_eq!(
            (
                p.int_ext.as_str(),
                p.day_night.as_str(),
                p.location.as_str()
            ),
            ("", "", "MONTAGE")
        );
    }

    #[test]
    fn page_estimate_uses_55_lines_per_page() {
        let one = vec![ElementText {
            element_type: "action".into(),
            text: "x".repeat(60),
        }];
        assert_eq!(page_eighths("INT. A - DAY", &one), 1);
        let long: Vec<ElementText> = (0..27)
            .map(|_| ElementText {
                element_type: "action".into(),
                text: "word ".repeat(12),
            })
            .collect();
        // 27 × (1 line + 1 spacing) = 54 lines + 2 heading lines = 56 → 1 page (8/8), rounded up.
        assert_eq!(page_eighths("INT. A - DAY", &long), 9);
    }

    #[test]
    fn initials_are_short_and_upper() {
        assert_eq!(initials("Arjun"), "AR");
        assert_eq!(initials("meera nair"), "MN");
        assert_eq!(initials(""), "?");
    }
}

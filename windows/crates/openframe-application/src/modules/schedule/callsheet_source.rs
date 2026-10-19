//! What a call sheet is built from: a read-only snapshot of one shooting day (production depth
//! spec §29 and §31; doc 33 section 5).
//!
//! [`build`] is the one definition of the call sheet source. The schedule uses it to decide
//! whether a sheet needs a refresh (the fingerprint), the call sheet uses it to prefill and to
//! preview an update. It reads the schedule (days, strips, markers and the day timeline from
//! [`Board::day_timeline`]), the Production Source projection (scenes, cast, locations and
//! breakdown items) and location details. It never writes anything. The call sheet never
//! computes schedule times on its own terms: every start time and day time comes from the
//! schedule's timeline.
//!
//! Older call sheets (made before migration 0016) stored a smaller source with no `version`.
//! Every field added later has a serde default, so an old stored source still reads. Staleness
//! of such a sheet is judged on the data its source held ([`CallSheetSource::matches`]), so an
//! upgrade alone never marks a draft "Needs refresh"; the first refresh stores the full source.

use std::collections::HashMap;

use openframe_domain::AppResult;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::board::{Board, DayRow, Item};
use super::fmt::{fnv_hex, pages};

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceScene {
    /// Scene lineage id (stable across drafts).
    pub key: String,
    pub number: String,
    pub heading: String,
    pub description: String,
    pub pages: String,
    /// "INT", "EXT", "INT/EXT" or empty.
    #[serde(default)]
    pub int_ext: String,
    /// "DAY", "NIGHT" or empty.
    #[serde(default)]
    pub day_night: String,
    /// Location names joined for the scene table.
    #[serde(default)]
    pub location: String,
    /// Cast ID numbers in the scene ("1, 3").
    #[serde(default)]
    pub cast_ids: String,
    /// Estimated length in minutes.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceCast {
    pub key: String,
    pub character: String,
    pub actor: Option<String>,
    /// Cast ID: order of the character's first appearance in the script.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub cast_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceLocation {
    pub key: String,
    pub name: String,
    pub address: Option<String>,
    #[serde(default)]
    pub parking: Option<String>,
    #[serde(default)]
    pub access: Option<String>,
    #[serde(default)]
    pub contact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceBreak {
    pub marker_type: String,
    pub label: String,
    pub at_time: Option<String>,
    /// Marker id.
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub minutes: Option<i64>,
}

/// One row of the day's timeline, in schedule order: a scene (key = scene lineage id) or a
/// marker (key = marker id, kind = Meal, Travel, Company Move or Custom).
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceRow {
    pub key: String,
    /// "Scene" or the marker type.
    pub kind: String,
    /// "18:30" when the schedule implies a start time.
    pub start: Option<String>,
    #[ts(type = "number | null")]
    pub minutes: Option<i64>,
}

/// Day times the schedule implies ("18:00").
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct CallSheetSourceTimes {
    pub crew_call: Option<String>,
    /// The shooting start entered on the day (the shooting call falls back to the first row).
    pub shooting_start: Option<String>,
    pub shooting_call: Option<String>,
    pub target_wrap: Option<String>,
    pub estimated_wrap: Option<String>,
    /// Start of the first meal marker.
    pub lunch: Option<String>,
}

/// Breakdown items of the day for one department ("Props": "Sc 12: case file").
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceDepartment {
    pub department: String,
    pub items: Vec<String>,
}

/// Background groups of the day (breakdown "Extras / Background").
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceExtra {
    pub key: String,
    pub description: String,
    /// Scene numbers ("12, 17").
    pub scenes: String,
}

/// The next shooting day, for the Advance schedule section.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceAdvance {
    pub day_label: String,
    pub date: Option<String>,
    pub scenes: String,
    pub locations: String,
    pub cast: String,
    pub props: String,
    pub wardrobe: String,
}

/// Version of the source written by this build ([`build`]); sources stored before migration
/// 0016 have none and read as 0.
pub const SOURCE_VERSION: u32 = 1;

/// Snapshot of the shooting day a call sheet was generated from.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSource {
    /// [`SOURCE_VERSION`] for sources made by this build, 0 for older stored ones.
    #[serde(default)]
    pub version: u32,
    pub production_title: String,
    pub source_label: String,
    pub day_id: String,
    #[ts(type = "number | null")]
    pub day_number: Option<i64>,
    pub day_label: String,
    pub date: Option<String>,
    pub day_notes: Option<String>,
    pub scenes: Vec<CallSheetSourceScene>,
    pub cast: Vec<CallSheetSourceCast>,
    pub locations: Vec<CallSheetSourceLocation>,
    pub breaks: Vec<CallSheetSourceBreak>,
    /// Shooting days in the schedule (for "Day 4 of 12").
    #[serde(default)]
    #[ts(type = "number | null")]
    pub total_days: Option<i64>,
    #[serde(default)]
    pub times: CallSheetSourceTimes,
    #[serde(default)]
    pub rows: Vec<CallSheetSourceRow>,
    #[serde(default)]
    pub extras: Vec<CallSheetSourceExtra>,
    #[serde(default)]
    pub departments: Vec<CallSheetSourceDepartment>,
    #[serde(default)]
    pub advance: Option<CallSheetSourceAdvance>,
}

impl CallSheetSource {
    pub fn fingerprint(&self) -> String {
        fnv_hex(&serde_json::to_string(self).unwrap_or_default())
    }

    /// The fingerprint in the shape a source of `version` was stored with. Version 0 is the
    /// shape written before migration 0016 (scenes with their start, cast, locations and breaks,
    /// and the three day times), so an older sheet is only stale when that data changed.
    pub fn fingerprint_as(&self, version: u32) -> String {
        if version >= SOURCE_VERSION {
            return self.fingerprint();
        }
        let scenes = self
            .scenes
            .iter()
            .map(|s| LegacyScene {
                key: &s.key,
                number: &s.number,
                heading: &s.heading,
                description: &s.description,
                pages: &s.pages,
                start: self
                    .rows
                    .iter()
                    .find(|r| r.kind == "Scene" && r.key == s.key)
                    .and_then(|r| r.start.as_deref()),
            })
            .collect();
        let legacy = Legacy {
            production_title: &self.production_title,
            source_label: &self.source_label,
            day_id: &self.day_id,
            day_number: self.day_number,
            day_label: &self.day_label,
            date: self.date.as_deref(),
            day_notes: self.day_notes.as_deref(),
            scenes,
            cast: self
                .cast
                .iter()
                .map(|c| LegacyCast {
                    key: &c.key,
                    character: &c.character,
                    actor: c.actor.as_deref(),
                })
                .collect(),
            locations: self
                .locations
                .iter()
                .map(|l| LegacyLocation {
                    key: &l.key,
                    name: &l.name,
                    address: l.address.as_deref(),
                })
                .collect(),
            breaks: self
                .breaks
                .iter()
                .map(|b| LegacyBreak {
                    marker_type: &b.marker_type,
                    label: &b.label,
                    at_time: b.at_time.as_deref(),
                })
                .collect(),
            crew_call: self.times.crew_call.as_deref(),
            shooting_start: self.times.shooting_start.as_deref(),
            target_wrap: self.times.target_wrap.as_deref(),
        };
        fnv_hex(&serde_json::to_string(&legacy).unwrap_or_default())
    }

    /// Does this (current) source still match a sheet that stored `fingerprint` from a source
    /// of `version`?
    pub fn matches(&self, fingerprint: &str, version: u32) -> bool {
        self.fingerprint_as(version) == fingerprint
    }
}

// The source shape stored before migration 0016, field for field and in the same order, so its
// fingerprint can be recomputed from a current source.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Legacy<'a> {
    production_title: &'a str,
    source_label: &'a str,
    day_id: &'a str,
    day_number: Option<i64>,
    day_label: &'a str,
    date: Option<&'a str>,
    day_notes: Option<&'a str>,
    scenes: Vec<LegacyScene<'a>>,
    cast: Vec<LegacyCast<'a>>,
    locations: Vec<LegacyLocation<'a>>,
    breaks: Vec<LegacyBreak<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    crew_call: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shooting_start: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_wrap: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyScene<'a> {
    key: &'a str,
    number: &'a str,
    heading: &'a str,
    description: &'a str,
    pages: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyCast<'a> {
    key: &'a str,
    character: &'a str,
    actor: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyLocation<'a> {
    key: &'a str,
    name: &'a str,
    address: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyBreak<'a> {
    marker_type: &'a str,
    label: &'a str,
    at_time: Option<&'a str>,
}

/// Location record details a call sheet shows (Production, Locations).
#[derive(Debug, Clone, Default)]
pub struct LocationDetail {
    pub parking: Option<String>,
    pub access: Option<String>,
    pub contact: Option<String>,
}

/// Parking, access and contact of every location record, by location id.
pub fn load_location_details(c: &Connection) -> AppResult<HashMap<String, LocationDetail>> {
    let mut st =
        c.prepare("SELECT id, contact, notes_json FROM location WHERE deleted_at IS NULL")?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    let clean = |v: Option<&serde_json::Value>| {
        v.and_then(|x| x.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let mut out = HashMap::new();
    for row in rows {
        let (id, contact, notes) = row?;
        let notes: serde_json::Value = serde_json::from_str(&notes).unwrap_or_default();
        out.insert(
            id,
            LocationDetail {
                parking: clean(notes.get("parking")),
                access: clean(notes.get("access")),
                contact: contact
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
            },
        );
    }
    Ok(out)
}

/// Breakdown categories that become call sheet department notes, with the department name the
/// production depth spec §29.7 uses. Cast, locations and extras have their own sections.
const DEPARTMENTS: &[(&str, &str)] = &[
    ("Props", "Props"),
    ("Wardrobe", "Costumes"),
    ("Hair / Makeup", "Makeup / Hair"),
    ("Sound", "Sound"),
    ("Vehicles", "Transport"),
    ("Special Effects", "Special effects"),
    ("VFX", "VFX"),
    ("Animals", "Animals"),
];

const EXTRAS: &str = "Extras / Background";

/// Cast ID numbers: order of each character's first appearance in the Production Source.
fn cast_ids(board: &Board) -> HashMap<String, i64> {
    let mut ids = HashMap::new();
    if let Some(src) = &board.source {
        for sc in &src.scenes {
            for c in src.cast(&sc.id) {
                let n = ids.len() as i64 + 1;
                ids.entry(c.key.clone()).or_insert(n);
            }
        }
    }
    ids
}

fn join<I: IntoIterator<Item = String>>(it: I) -> String {
    let mut v: Vec<String> = Vec::new();
    for s in it {
        if !s.trim().is_empty() && !v.contains(&s) {
            v.push(s);
        }
    }
    v.join(", ")
}

/// The next shooting day after `day` (off days skipped).
fn next_shooting_day<'a>(board: &'a Board, day: &DayRow) -> Option<&'a DayRow> {
    board
        .days
        .iter()
        .filter(|d| !d.off && d.position > day.position)
        .min_by_key(|d| (d.position, d.id.clone()))
}

fn advance(board: &Board, day: &DayRow) -> Option<CallSheetSourceAdvance> {
    let next = next_shooting_day(board, day)?;
    let strips = board.day_strips(&next.id);
    let items_of = |category: &str| {
        join(strips.iter().flat_map(|s| {
            board
                .items(s)
                .into_iter()
                .filter(|i| i.category == category)
                .map(|i| i.name)
                .collect::<Vec<_>>()
        }))
    };
    Some(CallSheetSourceAdvance {
        day_label: next.label(),
        date: next.date.clone(),
        scenes: join(strips.iter().map(|s| {
            let n = board
                .scene_number(s)
                .map(|n| n.to_string())
                .unwrap_or_default();
            let heading = board
                .scene(s)
                .map(|x| x.heading.clone())
                .unwrap_or_else(|| s.source_heading.clone());
            format!("{n} {heading}").trim().to_string()
        })),
        locations: join(
            strips
                .iter()
                .flat_map(|s| board.locations(s).into_iter().map(|l| l.name)),
        ),
        cast: join(strips.iter().flat_map(|s| {
            board
                .cast(s)
                .into_iter()
                .map(|c| c.character.to_uppercase())
        })),
        props: items_of("Props"),
        wardrobe: items_of("Wardrobe"),
    })
}

/// The day data a call sheet is prefilled from, and compared against for staleness.
pub fn build(board: &Board, d: &DayRow) -> CallSheetSource {
    let ids = cast_ids(board);
    let timeline = board.day_timeline(d);
    let time_of: HashMap<&str, (Option<String>, Option<i64>)> = timeline
        .rows
        .iter()
        .filter_map(|r| {
            r.item_id
                .as_deref()
                .map(|id| (id, (r.start_label.clone(), r.duration_minutes)))
        })
        .collect();
    let mut scenes = Vec::new();
    let mut cast: Vec<CallSheetSourceCast> = Vec::new();
    let mut locations: Vec<CallSheetSourceLocation> = Vec::new();
    let mut breaks = Vec::new();
    let mut rows = Vec::new();
    let mut extras: Vec<CallSheetSourceExtra> = Vec::new();
    let mut departments: Vec<CallSheetSourceDepartment> = Vec::new();
    let mut lunch = None;
    for item in board.day_items(&d.id) {
        let (start, minutes) = time_of.get(item.id()).cloned().unwrap_or((None, None));
        match item {
            Item::Strip(s) => {
                let sc = board.scene(s);
                let number = sc.map(|x| x.number.to_string()).unwrap_or_default();
                let scene_cast = board.cast(s);
                let scene_locs = board.locations(s);
                scenes.push(CallSheetSourceScene {
                    key: s.lineage_id.clone(),
                    number: number.clone(),
                    heading: sc
                        .map(|x| x.heading.clone())
                        .unwrap_or_else(|| s.source_heading.clone()),
                    description: sc.map(|x| x.synopsis.clone()).unwrap_or_default(),
                    pages: pages(board.page_eighths(s)),
                    int_ext: sc.map(|x| x.int_ext.clone()).unwrap_or_default(),
                    day_night: sc.map(|x| x.day_night.clone()).unwrap_or_default(),
                    location: join(scene_locs.iter().map(|l| l.name.clone())),
                    cast_ids: join(
                        scene_cast
                            .iter()
                            .filter_map(|c| ids.get(&c.key).map(|n| n.to_string())),
                    ),
                    minutes,
                });
                rows.push(CallSheetSourceRow {
                    key: s.lineage_id.clone(),
                    kind: "Scene".into(),
                    start: start.clone(),
                    minutes,
                });
                for c in scene_cast {
                    if !cast.iter().any(|x| x.key == c.key) {
                        cast.push(CallSheetSourceCast {
                            cast_id: ids.get(&c.key).copied(),
                            key: c.key,
                            character: c.character.to_uppercase(),
                            actor: c.actor,
                        });
                    }
                }
                for l in scene_locs {
                    if !locations.iter().any(|x| x.key == l.key) {
                        let detail = l
                            .location_id
                            .as_ref()
                            .and_then(|id| board.location_details.get(id))
                            .cloned()
                            .unwrap_or_default();
                        locations.push(CallSheetSourceLocation {
                            key: l.key,
                            name: l.name,
                            address: l.address,
                            parking: detail.parking,
                            access: detail.access,
                            contact: detail.contact,
                        });
                    }
                }
                for i in board.items(s) {
                    if i.category == EXTRAS {
                        let key = format!("extra:{}", i.name.trim().to_uppercase());
                        match extras.iter_mut().find(|e| e.key == key) {
                            Some(e) => {
                                if !number.is_empty() {
                                    e.scenes = join(
                                        e.scenes
                                            .split(", ")
                                            .map(str::to_string)
                                            .chain([number.clone()]),
                                    );
                                }
                            }
                            None => extras.push(CallSheetSourceExtra {
                                key,
                                description: i.name.trim().to_string(),
                                scenes: number.clone(),
                            }),
                        }
                    } else if let Some((_, dept)) =
                        DEPARTMENTS.iter().find(|(cat, _)| *cat == i.category)
                    {
                        let line = if number.is_empty() {
                            i.name.trim().to_string()
                        } else {
                            format!("Sc {number}: {}", i.name.trim())
                        };
                        match departments.iter_mut().find(|x| x.department == *dept) {
                            Some(x) => {
                                if !x.items.contains(&line) {
                                    x.items.push(line);
                                }
                            }
                            None => departments.push(CallSheetSourceDepartment {
                                department: dept.to_string(),
                                items: vec![line],
                            }),
                        }
                    }
                }
            }
            Item::Marker(m) => {
                if m.marker_type == "Meal" && lunch.is_none() {
                    lunch = start.clone();
                }
                breaks.push(CallSheetSourceBreak {
                    marker_type: m.marker_type.clone(),
                    label: m.label.clone(),
                    at_time: m.at_time.clone(),
                    key: m.id.clone(),
                    minutes,
                });
                rows.push(CallSheetSourceRow {
                    key: m.id.clone(),
                    kind: m.marker_type.clone(),
                    start,
                    minutes,
                });
            }
        }
    }
    departments.sort_by_key(|x| {
        DEPARTMENTS
            .iter()
            .position(|(_, dept)| *dept == x.department)
            .unwrap_or(usize::MAX)
    });
    CallSheetSource {
        version: SOURCE_VERSION,
        production_title: board.project_title.clone(),
        source_label: board
            .source
            .as_ref()
            .map(|s| s.info.label.clone())
            .unwrap_or_default(),
        day_id: d.id.clone(),
        day_number: d.number,
        day_label: d.label(),
        date: d.date.clone(),
        day_notes: d.notes.clone(),
        scenes,
        cast,
        locations,
        breaks,
        total_days: Some(board.days.iter().filter(|x| !x.off).count() as i64),
        times: CallSheetSourceTimes {
            crew_call: timeline.crew_call_label.clone(),
            shooting_start: timeline.shooting_start_label.clone(),
            shooting_call: timeline.shooting_start_label.clone().or_else(|| {
                timeline
                    .rows
                    .iter()
                    .find(|r| r.item_id.is_some())
                    .and_then(|r| r.start_label.clone())
            }),
            target_wrap: timeline.target_wrap_label.clone(),
            estimated_wrap: timeline.estimated_wrap_label.clone(),
            lunch,
        },
        rows,
        extras,
        departments,
        advance: advance(board, d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source stored before migration 0016 keeps its fingerprint: the legacy shape is the
    /// exact JSON those builds wrote (times left out while unknown).
    #[test]
    fn legacy_fingerprint_matches_the_shape_older_builds_stored() {
        let mut s = CallSheetSource {
            version: SOURCE_VERSION,
            production_title: "Black Rain".into(),
            day_id: "d1".into(),
            day_number: Some(4),
            day_label: "Shoot Day 4".into(),
            scenes: vec![CallSheetSourceScene {
                key: "L1".into(),
                number: "12".into(),
                heading: "INT. STATION - NIGHT".into(),
                description: String::new(),
                pages: "1".into(),
                int_ext: "INT".into(),
                day_night: "NIGHT".into(),
                location: "Station".into(),
                cast_ids: "1".into(),
                minutes: Some(60),
            }],
            cast: vec![CallSheetSourceCast {
                key: "arjun".into(),
                character: "ARJUN".into(),
                actor: Some("Karthik".into()),
                cast_id: Some(1),
            }],
            rows: vec![CallSheetSourceRow {
                key: "L1".into(),
                kind: "Scene".into(),
                start: None,
                minutes: Some(60),
            }],
            ..Default::default()
        };
        let old = r#"{"productionTitle":"Black Rain","sourceLabel":"","dayId":"d1","dayNumber":4,"dayLabel":"Shoot Day 4","date":null,"dayNotes":null,"scenes":[{"key":"L1","number":"12","heading":"INT. STATION - NIGHT","description":"","pages":"1"}],"cast":[{"key":"arjun","character":"ARJUN","actor":"Karthik"}],"locations":[],"breaks":[]}"#;
        assert!(
            s.matches(&fnv_hex(old), 0),
            "an upgrade alone never marks a sheet stale"
        );
        assert!(!s.matches(&fnv_hex(old), SOURCE_VERSION));
        // Data the old shape held still counts: a new crew call makes the old sheet stale.
        s.times.crew_call = Some("06:30".into());
        assert!(!s.matches(&fnv_hex(old), 0));
        let with_times = r#"{"productionTitle":"Black Rain","sourceLabel":"","dayId":"d1","dayNumber":4,"dayLabel":"Shoot Day 4","date":null,"dayNotes":null,"scenes":[{"key":"L1","number":"12","heading":"INT. STATION - NIGHT","description":"","pages":"1"}],"cast":[{"key":"arjun","character":"ARJUN","actor":"Karthik"}],"locations":[],"breaks":[],"crewCall":"06:30"}"#;
        assert!(s.matches(&fnv_hex(with_times), 0));
        assert!(s.matches(&s.fingerprint(), SOURCE_VERSION));
    }

    #[test]
    fn stored_sources_without_a_version_read_as_version_zero() {
        let s: CallSheetSource = serde_json::from_str(r#"{"productionTitle":"T","sourceLabel":"","dayId":"d","dayNumber":null,"dayLabel":"Day","date":null,"dayNotes":null,"scenes":[],"cast":[],"locations":[],"breaks":[]}"#).unwrap();
        assert_eq!(s.version, 0);
        assert!(s.rows.is_empty() && s.advance.is_none());
    }
}

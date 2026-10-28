//! Call sheets (FSD §38, §105, §147 and §148; FSD-CALL-001…012; production depth spec §29 to §32;
//! doc 33 section 5).
//!
//! A call sheet is created from one shooting day and prefilled from the schedule and project
//! data. From then on it is its own document: edits never flow back into the screenplay, the
//! breakdown or the schedule (FSD-CALL-002 / §38.7). When the day changes the sheet reads
//! "Needs refresh"; the update preview lists the changed scenes, cast calls and locations and
//! applies nothing until the user asks. Finalize freezes an immutable snapshot; later changes
//! create a new revision (FSD §38.8, §126).
//!
//! Status words (owner decision 2026-10-07): Draft, Needs refresh, Final. The stored status is
//! Draft or Final; "Needs refresh" is derived from the source fingerprint.
//!
//! The document (`document_json`, version [`DOC_VERSION`]) holds the six default sections
//! (production header, day and times, locations, scene schedule, talent, practical notes) and
//! the optional sections the user adds with "Add Section". An optional section is `None` until
//! it is added or prefilled with data, so empty optional sections stay hidden. Values nobody
//! has entered yet read "Needs input"; OpenFrame never invents them (weather, sunrise and sunset
//! are typed by the user; there is no network lookup).

use std::collections::HashMap;
use std::path::PathBuf;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, text, update_fields};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::board::{Board, effective_status, load_days, load_schedule};
use super::callsheet_source::CallSheetSource;
use super::fmt::{clock, clock_minutes};
use super::fmt::{date_long_upper, date_short};
use super::ops::{read, write};
use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::ingest_file;

/// Version of the call sheet document schema written by this build.
pub const DOC_VERSION: u32 = 2;

/// The six sections every call sheet shows (production depth spec §30), named and ordered as in
/// the exported PDF (owner, 2026-10-27).
pub const DEFAULT_SECTIONS: [&str; 6] = [
    "Day and times",
    "Production",
    "Locations",
    "Scene schedule",
    "Talent / Cast",
    "Practical / Safety information",
];

/// Sections added with "Add Section" (key, label).
pub const OPTIONAL_SECTIONS: [(&str, &str); 9] = [
    ("weather", "Weather"),
    ("extras", "Extras"),
    ("departments", "Department notes"),
    ("advance", "Advance schedule"),
    ("crew", "Crew"),
    ("communication", "Walkie channels and contacts"),
    ("referenceImages", "Reference images"),
    ("attachments", "Attachments"),
    ("specialNotes", "Special notes"),
];

/// What a call sheet shows where nobody has entered a value yet.
pub const NEEDS_INPUT: &str = "Needs input";

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    use openframe_domain::Capability as Cap;
    r.module("Call Sheets");
    r.query("callsheets.list", list)
        .meta(M::read("Call sheets with day, revision and status."));
    r.query("callsheets.get", get).meta(M::read(
        "One call sheet document (header, day times, locations, scenes, talent, notes and the added sections).",
    ));
    r.query("callsheets.refresh_preview", refresh_preview)
        .meta(M::compute(
            "What refreshing a call sheet from the schedule would change (scenes, cast calls, locations, date).",
        ));
    r.command("callsheets.create", create)
        .meta(M::edit("Create a call sheet for a shooting day."));
    r.command("callsheets.update", update)
        .meta(M::edit("Save edits to a call sheet document."));
    r.command("callsheets.add_section", add_section)
        .meta(M::edit(
            "Add an optional section to a call sheet, prefilled where the project has the data.",
        ));
    r.command("callsheets.refresh", refresh)
        .meta(M::edit("Refresh a call sheet from the current schedule."));
    r.command("callsheets.finalize", finalize).meta(
        M::command(
            Cap::LockOrFinalize,
            "Finalize a call sheet (frozen snapshot).",
        )
        .confirm(),
    );
    r.command("callsheets.new_revision", new_revision)
        .meta(M::edit("Start a new revision of a finalized call sheet."));
    r.command("callsheets.attach", attach).meta(
        M::edit("Attach a file the user picked to a call sheet.")
            .fs(Fs::ReadsUserFile)
            .hidden(h::USER_PATH),
    );
    r.command("callsheets.delete", delete)
        .meta(M::soft_delete("Move a call sheet to Recently Deleted."));
    r.indexer("call_sheet", index_call_sheet);
    r.trash_handler(TrashHandler {
        object_type: "call_sheet",
        table: "call_sheet",
        label: "Call Sheet",
        restore: None,
        purge: purge_call_sheet,
    });
}

// ------------------------------------------------------------------ document

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetDocCast {
    pub key: String,
    pub actor: String,
    pub character: String,
    pub call_time: String,
    pub notes: String,
    /// Cast ID number ("1").
    pub cast_id: String,
    /// Work status such as SW, W, WF or H.
    pub status: String,
    pub pickup: String,
    /// Hair and makeup time.
    pub makeup: String,
    pub contact: String,
}

/// One row of the scene schedule table: a scene, or (when `kind` is set) a meal, travel,
/// company move or custom row from the schedule.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetDocScene {
    pub key: String,
    pub number: String,
    pub heading: String,
    pub description: String,
    pub pages: String,
    /// "" for a scene; "Meal", "Travel", "Company Move" or "Custom" for a schedule row.
    pub kind: String,
    /// Start time ("18:30"); empty = estimated from the row before it.
    pub start: String,
    pub int_ext: String,
    pub day_night: String,
    /// Cast ID numbers in the scene.
    pub cast_ids: String,
    pub location: String,
    #[ts(type = "number | null")]
    pub minutes: Option<i64>,
}

impl CallSheetDocScene {
    pub fn is_scene(&self) -> bool {
        self.kind.is_empty()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetDocLocation {
    pub key: String,
    pub name: String,
    pub address: String,
    pub meeting_point: String,
    pub parking: String,
    pub hospital: String,
    pub emergency_contact: String,
    /// Meeting and access instructions.
    pub access: String,
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetPractical {
    pub parking: String,
    pub meeting_point: String,
    /// Transport.
    pub travel_notes: String,
    /// Meals.
    pub meal_break: String,
    pub emergency_contact: String,
    pub production_notes: String,
    pub toilets: String,
    pub access: String,
    pub safety: String,
    pub special_instructions: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetAttachment {
    pub asset_id: String,
    pub name: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetExtra {
    pub key: String,
    pub description: String,
    pub quantity: String,
    pub call_time: String,
    pub on_set: String,
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetDepartmentNote {
    pub department: String,
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetAdvance {
    /// "Shoot Day 5 · Tue 15 Jun 2027".
    pub day: String,
    pub location: String,
    pub cast: String,
    pub props: String,
    pub wardrobe: String,
    pub preparation: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetCrewMember {
    pub key: String,
    pub name: String,
    pub role: String,
    pub department: String,
    pub call_time: String,
    pub contact: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetCommunication {
    pub walkie_channels: String,
    pub emergency_contact: String,
    pub production_contact: String,
    pub note: String,
}

/// Optional sections stay hidden (None) until the user adds them or the project has data for
/// them (FSD §38.4; production depth spec §30).
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetOptional {
    /// Weather forecast as the user typed it.
    pub weather: Option<String>,
    /// Shown inside the Weather section; typed by the user.
    pub sunrise: String,
    pub sunset: String,
    pub special_notes: Option<String>,
    pub attachments: Option<Vec<CallSheetAttachment>>,
    pub reference_images: Option<Vec<CallSheetAttachment>>,
    pub extras: Option<Vec<CallSheetExtra>>,
    pub departments: Option<Vec<CallSheetDepartmentNote>>,
    pub advance: Option<CallSheetAdvance>,
    pub crew: Option<Vec<CallSheetCrewMember>>,
    pub communication: Option<CallSheetCommunication>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetExtraField {
    pub label: String,
    pub value: String,
}

/// Production header (production depth spec §29.1).
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetHeader {
    pub company: String,
    pub address: String,
    pub producer: String,
    pub first_ad: String,
    pub director: String,
    /// Main production contact.
    pub contact: String,
}

/// Day times besides the crew call (production depth spec §29.2).
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetTimes {
    pub general_call: String,
    pub shooting_call: String,
    pub breakfast: String,
    pub lunch: String,
    pub estimated_wrap: String,
}

/// Editable call-sheet content.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CallSheetDocument {
    /// Document schema version (0 = written before version 2).
    pub version: u32,
    /// Production name.
    pub title: String,
    pub header: CallSheetHeader,
    /// "SHOOT DAY 4".
    pub day_label: String,
    /// "Day 4 of 12".
    pub day_of: String,
    pub date: Option<String>,
    pub crew_call: String,
    pub times: CallSheetTimes,
    pub day_notes: String,
    pub locations: Vec<CallSheetDocLocation>,
    pub cast: Vec<CallSheetDocCast>,
    /// The scene schedule table: scenes and schedule rows in day order.
    pub scenes: Vec<CallSheetDocScene>,
    pub practical: CallSheetPractical,
    pub optional: CallSheetOptional,
    pub extra_fields: Vec<CallSheetExtraField>,
}

/// Crew record used to fill the Crew section.
#[derive(Debug, Clone)]
pub struct CrewRef {
    pub id: String,
    pub name: String,
    pub role: String,
    pub department: String,
}

fn crew_refs(c: &Connection) -> AppResult<Vec<CrewRef>> {
    let mut st = c.prepare(
        "SELECT id, person_name, role, COALESCE(department, '') FROM crew_member
         WHERE deleted_at IS NULL AND archived = 0
         ORDER BY department COLLATE NOCASE, role COLLATE NOCASE, person_name COLLATE NOCASE, id",
    )?;
    let rows = st.query_map([], |r| {
        Ok(CrewRef {
            id: r.get(0)?,
            name: r.get(1)?,
            role: r.get(2)?,
            department: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn crew_rows(crew: &[CrewRef]) -> Vec<CallSheetCrewMember> {
    crew.iter()
        .map(|c| CallSheetCrewMember {
            key: c.id.clone(),
            name: c.name.clone(),
            role: c.role.clone(),
            department: if c.department.trim().is_empty() {
                "Other".into()
            } else {
                c.department.trim().to_string()
            },
            ..Default::default()
        })
        .collect()
}

fn day_of(src: &CallSheetSource) -> String {
    match (src.day_number, src.total_days) {
        (Some(n), Some(t)) => format!("Day {n} of {t}"),
        (Some(n), None) => format!("Day {n}"),
        _ => String::new(),
    }
}

fn breaks_text(src: &CallSheetSource, types: &[&str]) -> String {
    src.breaks
        .iter()
        .filter(|b| types.contains(&b.marker_type.as_str()))
        .map(|b| match &b.at_time {
            Some(t) => format!("{} {t}", b.label),
            None => b.label.clone(),
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn scene_rows(src: &CallSheetSource) -> Vec<CallSheetDocScene> {
    let scene = |key: &str| src.scenes.iter().find(|s| s.key == key);
    let brk = |key: &str| src.breaks.iter().find(|b| b.key == key);
    if src.rows.is_empty() {
        // A source stored before day timelines: scenes only.
        return src
            .scenes
            .iter()
            .map(|s| CallSheetDocScene {
                key: s.key.clone(),
                number: s.number.clone(),
                heading: s.heading.clone(),
                description: s.description.clone(),
                pages: s.pages.clone(),
                ..Default::default()
            })
            .collect();
    }
    src.rows
        .iter()
        .filter_map(|r| {
            let start = r.start.clone().unwrap_or_default();
            if r.kind == "Scene" {
                let s = scene(&r.key)?;
                Some(CallSheetDocScene {
                    key: s.key.clone(),
                    number: s.number.clone(),
                    heading: s.heading.clone(),
                    description: s.description.clone(),
                    pages: s.pages.clone(),
                    kind: String::new(),
                    start,
                    int_ext: s.int_ext.clone(),
                    day_night: s.day_night.clone(),
                    cast_ids: s.cast_ids.clone(),
                    location: s.location.clone(),
                    minutes: s.minutes.or(r.minutes),
                })
            } else {
                let b = brk(&r.key)?;
                Some(CallSheetDocScene {
                    key: r.key.clone(),
                    heading: b.label.clone(),
                    kind: r.kind.clone(),
                    start,
                    minutes: r.minutes,
                    ..Default::default()
                })
            }
        })
        .collect()
}

fn extras_rows(src: &CallSheetSource) -> Vec<CallSheetExtra> {
    src.extras
        .iter()
        .map(|e| CallSheetExtra {
            key: e.key.clone(),
            description: e.description.clone(),
            notes: if e.scenes.is_empty() {
                String::new()
            } else {
                format!("Sc {}", e.scenes)
            },
            ..Default::default()
        })
        .collect()
}

fn department_rows(src: &CallSheetSource) -> Vec<CallSheetDepartmentNote> {
    src.departments
        .iter()
        .map(|d| CallSheetDepartmentNote {
            department: d.department.clone(),
            notes: d.items.join("; "),
        })
        .collect()
}

fn advance_of(src: &CallSheetSource) -> Option<CallSheetAdvance> {
    src.advance.as_ref().map(|a| CallSheetAdvance {
        day: match &a.date {
            Some(d) => format!("{} · {}", a.day_label, date_short(d)),
            None => a.day_label.clone(),
        },
        location: a.locations.clone(),
        cast: a.cast.clone(),
        props: a.props.clone(),
        wardrobe: a.wardrobe.clone(),
        preparation: String::new(),
    })
}

/// FSD-CALL-001…004: prefill from the shooting day; nothing is re-entered. Optional sections
/// are filled only when the project has data for them; the rest stay hidden.
pub fn prefill(src: &CallSheetSource, crew: &[CrewRef]) -> CallSheetDocument {
    let extras = extras_rows(src);
    let departments = department_rows(src);
    CallSheetDocument {
        version: DOC_VERSION,
        title: src.production_title.clone(),
        header: CallSheetHeader::default(),
        day_label: src.day_label.to_uppercase(),
        day_of: day_of(src),
        date: src.date.clone(),
        crew_call: src.times.crew_call.clone().unwrap_or_default(),
        times: CallSheetTimes {
            shooting_call: src.times.shooting_call.clone().unwrap_or_default(),
            lunch: src.times.lunch.clone().unwrap_or_default(),
            estimated_wrap: src.times.estimated_wrap.clone().unwrap_or_default(),
            ..Default::default()
        },
        day_notes: src.day_notes.clone().unwrap_or_default(),
        locations: src
            .locations
            .iter()
            .map(|l| CallSheetDocLocation {
                key: l.key.clone(),
                name: l.name.clone(),
                address: l.address.clone().unwrap_or_default(),
                parking: l.parking.clone().unwrap_or_default(),
                access: l.access.clone().unwrap_or_default(),
                emergency_contact: l.contact.clone().unwrap_or_default(),
                ..Default::default()
            })
            .collect(),
        cast: src
            .cast
            .iter()
            .map(|c| CallSheetDocCast {
                key: c.key.clone(),
                actor: c.actor.clone().unwrap_or_default(),
                character: c.character.clone(),
                cast_id: c.cast_id.map(|n| n.to_string()).unwrap_or_default(),
                ..Default::default()
            })
            .collect(),
        scenes: scene_rows(src),
        practical: CallSheetPractical {
            meal_break: breaks_text(src, &["Meal"]),
            travel_notes: breaks_text(src, &["Travel", "Company Move"]),
            ..Default::default()
        },
        optional: CallSheetOptional {
            extras: (!extras.is_empty()).then_some(extras),
            departments: (!departments.is_empty()).then_some(departments),
            advance: advance_of(src),
            crew: (!crew.is_empty()).then(|| crew_rows(crew)),
            ..Default::default()
        },
        extra_fields: Vec::new(),
    }
}

/// Keep the user's value unless it still equals what the old source derived.
fn kept(current: &str, derived_before: &str, derived_now: &str) -> String {
    if current == derived_before {
        derived_now.to_string()
    } else {
        current.to_string()
    }
}

/// Apply a newer source to an edited document. Derived data (scenes, cast, locations, date,
/// times, extras, department notes, advance schedule) follows the schedule; the user's call
/// times, notes, contacts and added sections are kept where they still apply.
pub fn merge(
    doc: &CallSheetDocument,
    old: &CallSheetSource,
    new: &CallSheetSource,
) -> CallSheetDocument {
    let fresh = prefill(new, &[]);
    let before = prefill(old, &[]);
    let cast = fresh
        .cast
        .iter()
        .map(|c| match doc.cast.iter().find(|x| x.key == c.key) {
            Some(prev) => CallSheetDocCast {
                // An actor typed on the sheet stays; one that came from the cast record follows it.
                actor: kept(
                    &prev.actor,
                    before
                        .cast
                        .iter()
                        .find(|x| x.key == c.key)
                        .map(|x| x.actor.as_str())
                        .unwrap_or(""),
                    &c.actor,
                ),
                call_time: prev.call_time.clone(),
                notes: prev.notes.clone(),
                status: prev.status.clone(),
                pickup: prev.pickup.clone(),
                makeup: prev.makeup.clone(),
                contact: prev.contact.clone(),
                ..c.clone()
            },
            None => c.clone(),
        })
        .collect();
    let locations = fresh
        .locations
        .iter()
        .map(|l| {
            let was = before.locations.iter().find(|x| x.key == l.key);
            match doc.locations.iter().find(|x| x.key == l.key) {
                Some(prev) => {
                    let derived = |f: fn(&CallSheetDocLocation) -> &String| {
                        kept(f(prev), was.map(|w| f(w).as_str()).unwrap_or(""), f(l))
                    };
                    CallSheetDocLocation {
                        // An address typed on the sheet stays; one from the location record follows it.
                        address: derived(|x| &x.address),
                        parking: derived(|x| &x.parking),
                        access: derived(|x| &x.access),
                        emergency_contact: derived(|x| &x.emergency_contact),
                        meeting_point: prev.meeting_point.clone(),
                        hospital: prev.hospital.clone(),
                        notes: prev.notes.clone(),
                        ..l.clone()
                    }
                }
                None => l.clone(),
            }
        })
        .collect();
    let scenes = fresh
        .scenes
        .iter()
        .map(|s| {
            let was = before.scenes.iter().find(|x| x.key == s.key);
            match doc.scenes.iter().find(|x| x.key == s.key) {
                Some(prev) => CallSheetDocScene {
                    start: kept(
                        &prev.start,
                        was.map(|w| w.start.as_str()).unwrap_or(""),
                        &s.start,
                    ),
                    ..s.clone()
                },
                None => s.clone(),
            }
        })
        .collect();
    let o = &doc.optional;
    let extras = match (&o.extras, &before.optional.extras, &fresh.optional.extras) {
        // The user removed a section the old source filled: keep it removed.
        (None, Some(_), _) => None,
        (cur, _, Some(now)) => Some(
            now.iter()
                .map(|e| {
                    match cur
                        .as_ref()
                        .and_then(|list| list.iter().find(|x| x.key == e.key))
                    {
                        Some(prev) => CallSheetExtra {
                            description: e.description.clone(),
                            ..prev.clone()
                        },
                        None => e.clone(),
                    }
                })
                .chain(
                    // Rows the user typed (no key from the schedule) stay.
                    cur.iter().flatten().filter(|x| x.key.is_empty()).cloned(),
                )
                .collect(),
        ),
        (cur, _, None) => cur
            .as_ref()
            .map(|list| list.iter().filter(|x| x.key.is_empty()).cloned().collect()),
    };
    let departments = match (
        &o.departments,
        &before.optional.departments,
        &fresh.optional.departments,
    ) {
        (None, Some(_), _) => None,
        (cur, was, now) => {
            let mut out: Vec<CallSheetDepartmentNote> = Vec::new();
            for d in now.iter().flatten() {
                let prev = cur.iter().flatten().find(|x| x.department == d.department);
                let old_derived = was
                    .iter()
                    .flatten()
                    .find(|x| x.department == d.department)
                    .map(|x| x.notes.as_str())
                    .unwrap_or("");
                out.push(CallSheetDepartmentNote {
                    department: d.department.clone(),
                    notes: match prev {
                        Some(p) => kept(&p.notes, old_derived, &d.notes),
                        None => d.notes.clone(),
                    },
                });
            }
            // Departments the user wrote that the schedule does not fill stay.
            for p in cur.iter().flatten() {
                let derived_before = was
                    .iter()
                    .flatten()
                    .any(|x| x.department == p.department && x.notes == p.notes);
                if !out.iter().any(|x| x.department == p.department) && !derived_before {
                    out.push(p.clone());
                }
            }
            if cur.is_none() && out.is_empty() {
                None
            } else {
                Some(out)
            }
        }
    };
    let advance = match (
        &o.advance,
        &before.optional.advance,
        &fresh.optional.advance,
    ) {
        (None, Some(_), _) => None,
        (Some(prev), was, now) => {
            let blank = CallSheetAdvance::default();
            let was = was.as_ref().unwrap_or(&blank);
            let now = now.as_ref().unwrap_or(&blank);
            Some(CallSheetAdvance {
                day: kept(&prev.day, &was.day, &now.day),
                location: kept(&prev.location, &was.location, &now.location),
                cast: kept(&prev.cast, &was.cast, &now.cast),
                props: kept(&prev.props, &was.props, &now.props),
                wardrobe: kept(&prev.wardrobe, &was.wardrobe, &now.wardrobe),
                preparation: prev.preparation.clone(),
            })
        }
        (None, None, now) => now.clone(),
    };
    CallSheetDocument {
        version: DOC_VERSION,
        title: kept(&doc.title, &before.title, &fresh.title),
        header: doc.header.clone(),
        day_label: fresh.day_label,
        day_of: fresh.day_of,
        date: fresh.date,
        crew_call: kept(&doc.crew_call, &before.crew_call, &fresh.crew_call),
        times: CallSheetTimes {
            general_call: doc.times.general_call.clone(),
            breakfast: doc.times.breakfast.clone(),
            shooting_call: kept(
                &doc.times.shooting_call,
                &before.times.shooting_call,
                &fresh.times.shooting_call,
            ),
            lunch: kept(&doc.times.lunch, &before.times.lunch, &fresh.times.lunch),
            estimated_wrap: kept(
                &doc.times.estimated_wrap,
                &before.times.estimated_wrap,
                &fresh.times.estimated_wrap,
            ),
        },
        day_notes: kept(&doc.day_notes, &before.day_notes, &fresh.day_notes),
        locations,
        cast,
        scenes,
        practical: CallSheetPractical {
            meal_break: kept(
                &doc.practical.meal_break,
                &before.practical.meal_break,
                &fresh.practical.meal_break,
            ),
            travel_notes: kept(
                &doc.practical.travel_notes,
                &before.practical.travel_notes,
                &fresh.practical.travel_notes,
            ),
            ..doc.practical.clone()
        },
        optional: CallSheetOptional {
            extras,
            departments,
            advance,
            ..doc.optional.clone()
        },
        extra_fields: doc.extra_fields.clone(),
    }
}

/// Read a call sheet time ("18:30", or "01:30 (+1)" for a time after midnight as the schedule
/// timeline writes it) into minutes after midnight of the shoot date.
fn parse_clock(s: &str) -> Option<i64> {
    let s = s.trim();
    let (time, days) = match s.split_once(" (+") {
        Some((t, rest)) => (t, rest.strip_suffix(')')?.parse::<i64>().ok()?),
        None => (s, 0),
    };
    if !(0..=6).contains(&days) || time.split_once(':').is_none_or(|(_, m)| m.len() != 2) {
        return None;
    }
    clock_minutes(time).map(|m| m + days * 1440)
}

/// Start time of every scene schedule row: the row's own time, or (shown as an estimate) the
/// end of the row before it, counted from the shooting call. Rows before the first known time
/// stay empty.
pub fn estimated_starts(doc: &CallSheetDocument) -> Vec<String> {
    let mut clock_at = parse_clock(&doc.times.shooting_call);
    doc.scenes
        .iter()
        .map(|row| {
            if let Some(t) = parse_clock(&row.start) {
                clock_at = Some(t);
            }
            let shown = clock_at.map(clock).unwrap_or_default();
            clock_at = match (clock_at, row.minutes) {
                (Some(c), Some(m)) => Some(c + m),
                _ => None,
            };
            shown
        })
        .collect()
}

// ------------------------------------------------------------------ update preview

/// What changed in one area between the source a sheet was built from and the schedule now.
#[derive(Debug, Clone, Default, Serialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetChanges {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

impl CallSheetChanges {
    fn count(&self) -> usize {
        self.added.len() + self.removed.len() + self.changed.len()
    }
    fn describe(&self) -> String {
        let mut parts = Vec::new();
        for (word, list) in [
            ("added", &self.added),
            ("removed", &self.removed),
            ("changed", &self.changed),
        ] {
            if !list.is_empty() {
                parts.push(format!("{word} {}", list.join(", ")));
            }
        }
        parts.join("; ")
    }
}

fn diff_by_key<T, K, L>(old: &[T], new: &[T], key: K, label: L) -> CallSheetChanges
where
    T: PartialEq,
    K: Fn(&T) -> &str,
    L: Fn(&T) -> String,
{
    let mut out = CallSheetChanges::default();
    for n in new {
        match old.iter().find(|o| key(o) == key(n)) {
            None => out.added.push(label(n)),
            Some(o) if o != n => out.changed.push(label(n)),
            Some(_) => {}
        }
    }
    for o in old {
        if !new.iter().any(|n| key(n) == key(o)) {
            out.removed.push(label(o));
        }
    }
    out
}

fn summary_line(n: usize, one: &str, many: &str, detail: &str) -> String {
    match n {
        0 => format!("{} unchanged", capitalize(many)),
        1 => format!("1 {one} changed ({detail})"),
        n => format!("{n} {many} changed ({detail})"),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

struct SourceDiff {
    scenes: CallSheetChanges,
    cast: CallSheetChanges,
    locations: CallSheetChanges,
    date_changed: bool,
    other: Vec<String>,
}

fn diff_sources(old: &CallSheetSource, new: &CallSheetSource) -> SourceDiff {
    let scenes = diff_by_key(
        &old.scenes,
        &new.scenes,
        |s| s.key.as_str(),
        |s| {
            if s.number.is_empty() {
                s.heading.clone()
            } else {
                s.number.clone()
            }
        },
    );
    let cast = diff_by_key(
        &old.cast,
        &new.cast,
        |c| c.key.as_str(),
        |c| c.character.clone(),
    );
    let locations = diff_by_key(
        &old.locations,
        &new.locations,
        |l| l.key.as_str(),
        |l| l.name.clone(),
    );
    let mut other = Vec::new();
    if old.day_number != new.day_number || old.total_days != new.total_days {
        other.push("Day number".to_string());
    }
    // Start times and lengths of rows on both sheets; added or removed rows are already
    // reported as scene or break changes.
    let timing = |rows: &[super::callsheet_source::CallSheetSourceRow]| {
        rows.iter()
            .map(|r| (r.key.clone(), (r.start.clone(), r.minutes)))
            .collect::<HashMap<_, _>>()
    };
    let (was, now) = (timing(&old.rows), timing(&new.rows));
    let rows_retimed = was.iter().any(|(k, v)| now.get(k).is_some_and(|n| n != v));
    if old.times != new.times || rows_retimed {
        other.push("Times".to_string());
    }
    if old.breaks != new.breaks {
        other.push("Breaks".to_string());
    }
    if old.day_notes != new.day_notes {
        other.push("Day notes".to_string());
    }
    if old.extras != new.extras {
        other.push("Extras".to_string());
    }
    if old.departments != new.departments {
        other.push("Department notes".to_string());
    }
    if old.advance != new.advance {
        other.push("Advance schedule".to_string());
    }
    if old.production_title != new.production_title {
        other.push("Production title".to_string());
    }
    SourceDiff {
        scenes,
        cast,
        locations,
        date_changed: old.date != new.date,
        other,
    }
}

/// Human list of what differs between two sources (FSD §125: say what changed).
fn changed_areas(old: &CallSheetSource, new: &CallSheetSource) -> Vec<String> {
    let d = diff_sources(old, new);
    let mut v = Vec::new();
    if d.date_changed {
        v.push("Date".to_string());
    }
    if d.scenes.count() > 0 {
        v.push("Scenes".to_string());
    }
    if d.cast.count() > 0 {
        v.push("Cast".to_string());
    }
    if d.locations.count() > 0 {
        v.push("Locations".to_string());
    }
    v.extend(d.other);
    v
}

// ------------------------------------------------------------------ rows

struct SheetRow {
    id: String,
    schedule_id: String,
    day_id: String,
    revision: i64,
    title: String,
    status: String,
    needs_refresh: bool,
    source: CallSheetSource,
    fingerprint: String,
    captured_at: i64,
    document: CallSheetDocument,
    snapshot_id: Option<String>,
    finalized_at: Option<i64>,
    superseded_by: Option<String>,
    previous_id: Option<String>,
    updated_at: i64,
    rev: i64,
}

fn parse_doc(json: &str) -> AppResult<CallSheetDocument> {
    serde_json::from_str(json).map_err(|e| {
        AppError::new(
            "integrity.call_sheet",
            "This call sheet could not be read. Restore it from a backup or Recently Deleted.",
        )
        .with_detail(e.to_string())
    })
}

fn load_sheet(c: &Connection, id: &str) -> AppResult<SheetRow> {
    let row = c
        .query_row(
            "SELECT id, schedule_id, shoot_day_id, revision, title, status, needs_refresh, source_snapshot_json,
                source_fingerprint, source_captured_at, document_json, snapshot_id, finalized_at,
                superseded_by, previous_id, updated_at, rev
         FROM call_sheet WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| {
                let src: String = r.get(7)?;
                Ok((
                    SheetRow {
                        id: r.get(0)?,
                        schedule_id: r.get(1)?,
                        day_id: r.get(2)?,
                        revision: r.get(3)?,
                        title: r.get(4)?,
                        status: r.get(5)?,
                        needs_refresh: r.get::<_, i64>(6)? != 0,
                        source: serde_json::from_str(&src).unwrap_or_else(|_| empty_source()),
                        fingerprint: r.get(8)?,
                        captured_at: r.get(9)?,
                        document: CallSheetDocument::default(),
                        snapshot_id: r.get(11)?,
                        finalized_at: r.get(12)?,
                        superseded_by: r.get(13)?,
                        previous_id: r.get(14)?,
                        updated_at: r.get(15)?,
                        rev: r.get(16)?,
                    },
                    r.get::<_, String>(10)?,
                ))
            },
        )
        .optional()?;
    let (mut sheet, doc) = row.ok_or_else(|| AppError::not_found("call sheet"))?;
    sheet.document = parse_doc(&doc)?;
    Ok(sheet)
}

fn empty_source() -> CallSheetSource {
    CallSheetSource::default()
}

/// Final (and the words stored before migration 0016 for it) is a frozen snapshot.
pub fn is_final(status: &str) -> bool {
    matches!(status, "Final" | "Issued" | "Superseded")
}

fn ensure_editable(s: &SheetRow) -> AppResult<()> {
    if is_final(&s.status) {
        return Err(AppError::new(
            "conflict.call_sheet_final",
            "This call sheet is final. Start a new revision to make changes.",
        ));
    }
    Ok(())
}

/// The current source of a sheet's day, or None when the day (or schedule) no longer exists.
fn current_source(c: &Connection, s: &SheetRow) -> AppResult<Option<CallSheetSource>> {
    let sched = match load_schedule(c, &s.schedule_id) {
        Ok(x) => x,
        Err(e) if e.is("not_found") => return Ok(None),
        Err(e) => return Err(e),
    };
    let board = Board::load(c, sched)?;
    Ok(board.day(&s.day_id).map(|d| board.call_sheet_source(d)))
}

fn title_for(src: &CallSheetSource, revision: i64) -> String {
    let base = format!("Call Sheet: {}", src.day_label.replace("Shoot Day", "Day"));
    if revision > 1 {
        format!("{base} (v{revision})")
    } else {
        base
    }
}

/// Call times from a call sheet (for the Daily View).
pub(crate) fn call_times(
    c: &Connection,
    id: &str,
) -> AppResult<(Option<String>, HashMap<String, String>)> {
    let doc: Option<String> = c
        .query_row(
            "SELECT document_json FROM call_sheet WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    let doc: CallSheetDocument = doc
        .and_then(|d| serde_json::from_str(&d).ok())
        .unwrap_or_default();
    let crew = Some(doc.crew_call.trim().to_string()).filter(|s| !s.is_empty());
    let times = doc
        .cast
        .into_iter()
        .filter(|c| !c.call_time.trim().is_empty())
        .map(|c| (c.key, c.call_time.trim().to_string()))
        .collect();
    Ok((crew, times))
}

/// Paths of values that still need input (shown as "Needs input", never invented).
pub fn missing_values(doc: &CallSheetDocument) -> Vec<String> {
    let mut v = Vec::new();
    if doc.date.is_none() {
        v.push("date".to_string());
    }
    if doc.crew_call.trim().is_empty() {
        v.push("crewCall".to_string());
    }
    for (i, l) in doc.locations.iter().enumerate() {
        if l.address.trim().is_empty() {
            v.push(format!("locations.{i}.address"));
        }
        if l.hospital.trim().is_empty() {
            v.push(format!("locations.{i}.hospital"));
        }
    }
    for (i, c) in doc.cast.iter().enumerate() {
        if c.actor.trim().is_empty() {
            v.push(format!("cast.{i}.actor"));
        }
        if c.call_time.trim().is_empty() {
            v.push(format!("cast.{i}.callTime"));
        }
    }
    if doc.practical.emergency_contact.trim().is_empty() {
        v.push("practical.emergencyContact".to_string());
    }
    v
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetListRow {
    pub id: String,
    pub title: String,
    pub day_id: String,
    pub day_label: String,
    pub date: Option<String>,
    pub date_label: Option<String>,
    #[ts(type = "number")]
    pub revision: i64,
    /// Draft / Needs refresh / Final.
    pub status: String,
    pub source_changed: bool,
    pub day_exists: bool,
    /// A newer revision of this sheet exists.
    pub replaced: bool,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetStale {
    /// What changed, e.g. ["Scenes", "Date"].
    pub areas: Vec<String>,
    /// When the sheet was prepared from the schedule.
    #[ts(type = "number")]
    pub prepared_at: i64,
    pub day_deleted: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetRevisionRef {
    pub id: String,
    #[ts(type = "number")]
    pub revision: i64,
    /// Draft / Needs refresh / Final.
    pub status: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSectionDef {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetDetail {
    pub id: String,
    pub title: String,
    #[ts(type = "number")]
    pub revision: i64,
    /// Draft / Needs refresh / Final.
    pub status: String,
    pub editable: bool,
    pub source_changed: bool,
    pub stale: Option<CallSheetStale>,
    pub day_id: String,
    /// "Shoot Day 4 · Mon 14 Jun 2027".
    pub day_title: String,
    pub date_long: Option<String>,
    pub document: CallSheetDocument,
    pub source: CallSheetSource,
    pub missing: Vec<String>,
    /// Start time shown for each scene schedule row (own or estimated).
    pub estimated_starts: Vec<String>,
    pub default_sections: Vec<String>,
    pub optional_sections: Vec<CallSheetSectionDef>,
    pub snapshot_id: Option<String>,
    #[ts(type = "number | null")]
    pub finalized_at: Option<i64>,
    /// The newer revision that replaced this one.
    pub superseded_by: Option<String>,
    pub previous_id: Option<String>,
    pub revisions: Vec<CallSheetRevisionRef>,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetListArgs {}

fn list(core: &AppCore, actor: &Actor, _: CallSheetListArgs) -> AppResult<Vec<CallSheetListRow>> {
    read(core, actor, |c| {
        let mut st = c.prepare(
            "SELECT id, schedule_id FROM call_sheet WHERE deleted_at IS NULL ORDER BY created_at, revision",
        )?;
        let ids: Vec<(String, String)> = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let mut boards: HashMap<String, Option<Board>> = HashMap::new();
        let mut out = Vec::new();
        for (id, sid) in ids {
            let sheet = load_sheet(c, &id)?;
            if !boards.contains_key(&sid) {
                let b = match load_schedule(c, &sid) {
                    Ok(s) => Some(Board::load(c, s)?),
                    Err(e) if e.is("not_found") => None,
                    Err(e) => return Err(e),
                };
                boards.insert(sid.clone(), b);
            }
            let board = boards.get(&sid).and_then(|b| b.as_ref());
            let day = board.and_then(|b| b.day(&sheet.day_id));
            let changed = match (board, day) {
                (Some(b), Some(d)) => {
                    sheet.needs_refresh
                        || !b
                            .call_sheet_source(d)
                            .matches(&sheet.fingerprint, sheet.source.version)
                }
                _ => true,
            };
            let date = day
                .and_then(|d| d.date.clone())
                .or(sheet.source.date.clone());
            out.push(CallSheetListRow {
                id: sheet.id,
                title: sheet.title,
                day_id: sheet.day_id,
                day_label: day
                    .map(|d| d.label())
                    .unwrap_or_else(|| sheet.source.day_label.clone()),
                date_label: date.as_deref().map(date_short),
                date,
                revision: sheet.revision,
                status: effective_status(&sheet.status, changed),
                source_changed: changed,
                day_exists: day.is_some(),
                replaced: sheet.superseded_by.is_some(),
                updated_at: sheet.updated_at,
            });
        }
        out.sort_by(|a, b| {
            (a.date.is_none(), &a.date, &a.day_label, -a.revision).cmp(&(
                b.date.is_none(),
                &b.date,
                &b.day_label,
                -b.revision,
            ))
        });
        Ok(out)
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetIdArgs {
    pub id: String,
}

fn detail(c: &Connection, id: &str) -> AppResult<CallSheetDetail> {
    let s = load_sheet(c, id)?;
    let current = current_source(c, &s)?;
    let (changed, stale) = match &current {
        Some(src) => {
            let changed = s.needs_refresh || !src.matches(&s.fingerprint, s.source.version);
            let stale = changed.then(|| CallSheetStale {
                areas: changed_areas(&s.source, src),
                prepared_at: s.captured_at,
                day_deleted: false,
            });
            (changed, stale)
        }
        None => (
            true,
            Some(CallSheetStale {
                areas: Vec::new(),
                prepared_at: s.captured_at,
                day_deleted: true,
            }),
        ),
    };
    let mut st = c.prepare(
        "SELECT id, revision, status FROM call_sheet WHERE shoot_day_id = ?1 AND deleted_at IS NULL ORDER BY revision DESC",
    )?;
    let revisions = st
        .query_map([&s.day_id], |r| {
            let status: String = r.get(2)?;
            Ok(CallSheetRevisionRef {
                id: r.get(0)?,
                revision: r.get(1)?,
                status: effective_status(&status, false),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let day_title = match &current {
        Some(src) => match &src.date {
            Some(d) => format!("{} · {}", src.day_label, date_short(d)),
            None => src.day_label.clone(),
        },
        None => format!("{} (deleted)", s.source.day_label),
    };
    Ok(CallSheetDetail {
        editable: !is_final(&s.status),
        status: effective_status(&s.status, changed),
        source_changed: changed,
        stale,
        day_title,
        date_long: s.document.date.as_deref().map(date_long_upper),
        missing: missing_values(&s.document),
        estimated_starts: estimated_starts(&s.document),
        default_sections: DEFAULT_SECTIONS.iter().map(|x| x.to_string()).collect(),
        optional_sections: OPTIONAL_SECTIONS
            .iter()
            .map(|(k, l)| CallSheetSectionDef {
                key: k.to_string(),
                label: l.to_string(),
            })
            .collect(),
        id: s.id,
        title: s.title,
        revision: s.revision,
        day_id: s.day_id,
        document: s.document,
        source: s.source,
        snapshot_id: s.snapshot_id,
        finalized_at: s.finalized_at,
        superseded_by: s.superseded_by,
        previous_id: s.previous_id,
        revisions,
        updated_at: s.updated_at,
        rev: s.rev,
    })
}

fn get(core: &AppCore, actor: &Actor, a: CallSheetIdArgs) -> AppResult<CallSheetDetail> {
    read(core, actor, |c| detail(c, &a.id))
}

// ------------------------------------------------------------------ create

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetCreateArgs {
    pub day_id: String,
    /// A call sheet template to start from (built-in, personal or project). Its content is
    /// copied into the new sheet; later template edits never change the sheet.
    #[serde(default)]
    #[ts(optional)]
    pub template: Option<crate::modules::notes::templates::TemplateRefArgs>,
}

fn insert_sheet(
    tx: &Tx<'_>,
    schedule_id: &str,
    src: &CallSheetSource,
    doc: &CallSheetDocument,
    revision: i64,
    previous_id: Option<&str>,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO call_sheet(id, schedule_id, shoot_day_id, revision, title, status, needs_refresh, source_snapshot_json,
             source_fingerprint, source_captured_at, document_json, previous_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'Draft', 0, ?6, ?7, ?8, ?9, ?10, ?8, ?8)",
        params![
            id,
            schedule_id,
            src.day_id,
            revision,
            title_for(src, revision),
            serde_json::to_string(src).map_err(|e| AppError::internal(e.to_string()))?,
            src.fingerprint(),
            now,
            serde_json::to_string(doc).map_err(|e| AppError::internal(e.to_string()))?,
            previous_id,
        ],
    )?;
    Ok(id)
}

/// FSD-CALL-001: "Create Call Sheet" from one shooting day.
fn create(core: &AppCore, actor: &Actor, a: CallSheetCreateArgs) -> AppResult<String> {
    let label = super::ops::read(core, actor, |c| {
        let sid = super::board::day_schedule_id(c, &a.day_id)?
            .ok_or_else(|| AppError::not_found("shooting day"))?;
        let d = load_days(c, &sid)?
            .into_iter()
            .find(|d| d.id == a.day_id)
            .ok_or_else(|| AppError::not_found("shooting day"))?;
        Ok(d.label())
    })?;
    // The template is read once and copied; the sheet never refers back to it.
    let template = match &a.template {
        Some(t) => Some(crate::modules::notes::templates::call_sheet_layout(
            core, actor, t,
        )?),
        None => None,
    };
    let meta = MutationMeta::new(
        "callsheets.create",
        format!("Created a call sheet for {label}"),
        Capability::Edit,
    )
    .target("shooting_day", &a.day_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let sid = super::board::day_schedule_id(c, &a.day_id)?
            .ok_or_else(|| AppError::not_found("shooting day"))?;
        let board = Board::load(c, load_schedule(c, &sid)?)?;
        let day = board
            .day(&a.day_id)
            .ok_or_else(|| AppError::not_found("shooting day"))?;
        if day.off {
            return Err(AppError::validation(
                "off_day",
                "Off days have no call sheet. Choose a shooting day.",
            ));
        }
        if let Some(existing) = board.call_sheet_for_day(&day.id) {
            return Err(AppError::new(
                "conflict.call_sheet_exists",
                format!(
                    "{} already has a call sheet. Open it to make changes.",
                    day.label()
                ),
            )
            .with_detail(existing.id.clone()));
        }
        let src = board.call_sheet_source(day);
        let mut doc = prefill(&src, &crew_refs(c)?);
        if let Some(layout) = &template {
            layout.apply(&mut doc);
        }
        let prior: i64 = c.query_row(
            "SELECT COALESCE(MAX(revision), 0) FROM call_sheet WHERE shoot_day_id = ?1",
            [&day.id],
            |r| r.get(0),
        )?;
        insert_sheet(tx, &sid, &src, &doc, prior + 1, None)
    })
}

// ------------------------------------------------------------------ edit

const MAX_DOC_BYTES: usize = 512 * 1024;
const MAX_ROWS: usize = 500;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetUpdateArgs {
    pub id: String,
    pub document: CallSheetDocument,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

fn check_doc(doc: &CallSheetDocument) -> AppResult<String> {
    let o = &doc.optional;
    let too_many = doc.scenes.len() > MAX_ROWS
        || doc.cast.len() > MAX_ROWS
        || doc.locations.len() > MAX_ROWS
        || doc.extra_fields.len() > MAX_ROWS
        || o.extras.as_ref().is_some_and(|v| v.len() > MAX_ROWS)
        || o.crew.as_ref().is_some_and(|v| v.len() > MAX_ROWS)
        || o.departments.as_ref().is_some_and(|v| v.len() > MAX_ROWS);
    let json = serde_json::to_string(doc).map_err(|e| AppError::internal(e.to_string()))?;
    if too_many || json.len() > MAX_DOC_BYTES {
        return Err(AppError::invalid_input(
            "This call sheet is too large. Shorten the notes or move text into attachments.",
        ));
    }
    Ok(json)
}

/// FSD-CALL-005/006/008: edit the document only. The screenplay, breakdown and schedule are
/// never touched.
fn update(core: &AppCore, actor: &Actor, a: CallSheetUpdateArgs) -> AppResult<i64> {
    let mut document = a.document;
    document.version = DOC_VERSION;
    let json = check_doc(&document)?;
    let meta = MutationMeta::new("callsheets.update", "Edited a call sheet", Capability::Edit)
        .target("call_sheet", &a.id)
        .coalesce(format!("callsheet:{}", a.id));
    write(core, actor, meta, |tx| {
        let s = load_sheet(tx.conn(), &a.id)?;
        ensure_editable(&s)?;
        update_fields(
            tx.conn(),
            "call_sheet",
            &a.id,
            &[("document_json", text(json.clone()))],
            &["document_json"],
            a.expected_rev,
            "call sheet",
        )
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetAddSectionArgs {
    pub id: String,
    /// One of the optional section keys (weather, extras, departments, advance, crew,
    /// communication, referenceImages, attachments, specialNotes).
    pub section: String,
}

/// "Add Section": show an optional section, filled from the project where it has the data
/// (crew records, the day's extras and department items, the next shooting day).
fn add_section(core: &AppCore, actor: &Actor, a: CallSheetAddSectionArgs) -> AppResult<()> {
    let label = OPTIONAL_SECTIONS
        .iter()
        .find(|(k, _)| *k == a.section)
        .map(|(_, l)| *l)
        .ok_or_else(|| AppError::invalid_input("Choose one of the sections in the list."))?;
    let meta = MutationMeta::new(
        "callsheets.add_section",
        format!("Added {label} to a call sheet"),
        Capability::Edit,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_sheet(c, &a.id)?;
        ensure_editable(&s)?;
        let src = current_source(c, &s)?.unwrap_or_else(|| s.source.clone());
        let mut doc = s.document.clone();
        let o = &mut doc.optional;
        match a.section.as_str() {
            "weather" => {
                o.weather.get_or_insert_with(String::new);
            }
            "extras" => {
                o.extras.get_or_insert_with(|| extras_rows(&src));
            }
            "departments" => {
                o.departments.get_or_insert_with(|| {
                    let rows = department_rows(&src);
                    if rows.is_empty() {
                        [
                            "Set dressing",
                            "Props",
                            "Costumes",
                            "Sound",
                            "Makeup / Hair",
                            "Art",
                            "Camera",
                            "Production",
                        ]
                        .iter()
                        .map(|d| CallSheetDepartmentNote {
                            department: d.to_string(),
                            notes: String::new(),
                        })
                        .collect()
                    } else {
                        rows
                    }
                });
            }
            "advance" => {
                if o.advance.is_none() {
                    o.advance = Some(advance_of(&src).unwrap_or_default());
                }
            }
            "crew" => {
                if o.crew.is_none() {
                    o.crew = Some(crew_rows(&crew_refs(c)?));
                }
            }
            "communication" => {
                o.communication.get_or_insert_with(Default::default);
            }
            "referenceImages" => {
                o.reference_images.get_or_insert_with(Vec::new);
            }
            "attachments" => {
                o.attachments.get_or_insert_with(Vec::new);
            }
            "specialNotes" => {
                o.special_notes.get_or_insert_with(String::new);
            }
            _ => unreachable!("checked against OPTIONAL_SECTIONS"),
        }
        doc.version = DOC_VERSION;
        let json = check_doc(&doc)?;
        update_fields(
            c,
            "call_sheet",
            &a.id,
            &[("document_json", text(json))],
            &["document_json"],
            None,
            "call sheet",
        )?;
        Ok(())
    })
}

// ------------------------------------------------------------------ refresh

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetRefreshRow {
    pub area: String,
    pub now: String,
    pub after: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetRefreshPreview {
    /// "3 scenes changed (added 22; removed 17; changed 12)", "Cast calls unchanged",
    /// "1 location changed (added Level Crossing)", "Day date unchanged".
    pub summary: Vec<String>,
    pub scenes: CallSheetChanges,
    pub cast: CallSheetChanges,
    pub locations: CallSheetChanges,
    pub date_changed: bool,
    /// Other areas that change (times, breaks, extras, department notes, advance schedule).
    pub other: Vec<String>,
    /// Side by side: what the sheet shows now and after the update.
    pub rows: Vec<CallSheetRefreshRow>,
    pub has_changes: bool,
    /// True for Final sheets: applying creates a new revision.
    pub creates_revision: bool,
    pub day_deleted: bool,
}

fn summarize(doc: &CallSheetDocument) -> Vec<(String, String)> {
    let dash = |s: &str| {
        if s.trim().is_empty() {
            "None".to_string()
        } else {
            s.to_string()
        }
    };
    let join = |v: Vec<String>| {
        if v.is_empty() {
            "None".to_string()
        } else {
            v.join("\n")
        }
    };
    vec![
        (
            "Date".into(),
            doc.date
                .as_deref()
                .map(date_short)
                .unwrap_or_else(|| "No date".into()),
        ),
        ("Day".into(), doc.day_label.clone()),
        (
            "Scenes".into(),
            join(
                doc.scenes
                    .iter()
                    .filter(|s| s.is_scene())
                    .map(|s| format!("{} {}", s.number, s.heading))
                    .collect(),
            ),
        ),
        (
            "Schedule rows".into(),
            join(
                doc.scenes
                    .iter()
                    .filter(|s| !s.is_scene())
                    .map(|s| {
                        if s.start.is_empty() {
                            s.heading.clone()
                        } else {
                            format!("{} {}", s.start, s.heading)
                        }
                    })
                    .collect(),
            ),
        ),
        (
            "Cast".into(),
            join(
                doc.cast
                    .iter()
                    .map(|c| {
                        let who = if c.actor.is_empty() {
                            c.character.clone()
                        } else {
                            format!("{} ({})", c.actor, c.character)
                        };
                        if c.call_time.is_empty() {
                            who
                        } else {
                            format!("{who} · {}", c.call_time)
                        }
                    })
                    .collect(),
            ),
        ),
        (
            "Locations".into(),
            join(
                doc.locations
                    .iter()
                    .map(|l| {
                        if l.address.is_empty() {
                            l.name.clone()
                        } else {
                            format!("{}, {}", l.name, l.address)
                        }
                    })
                    .collect(),
            ),
        ),
        (
            "Times".into(),
            join(
                [
                    ("Crew call", &doc.crew_call),
                    ("Shooting call", &doc.times.shooting_call),
                    ("Lunch", &doc.times.lunch),
                    ("Estimated wrap", &doc.times.estimated_wrap),
                ]
                .into_iter()
                .filter(|(_, v)| !v.trim().is_empty())
                .map(|(k, v)| format!("{k} {v}"))
                .collect(),
            ),
        ),
        ("Day notes".into(), dash(&doc.day_notes)),
        ("Meal and breaks".into(), dash(&doc.practical.meal_break)),
        ("Travel".into(), dash(&doc.practical.travel_notes)),
    ]
}

fn preview(s: &SheetRow, src: &CallSheetSource) -> CallSheetRefreshPreview {
    let after = merge(&s.document, &s.source, src);
    let rows: Vec<CallSheetRefreshRow> = summarize(&s.document)
        .into_iter()
        .zip(summarize(&after))
        .map(|((area, now), (_, aft))| CallSheetRefreshRow {
            changed: now != aft,
            area,
            now,
            after: aft,
        })
        .collect();
    let d = diff_sources(&s.source, src);
    let summary = vec![
        summary_line(d.scenes.count(), "scene", "scenes", &d.scenes.describe()),
        summary_line(
            d.cast.count(),
            "cast call",
            "cast calls",
            &d.cast.describe(),
        ),
        summary_line(
            d.locations.count(),
            "location",
            "locations",
            &d.locations.describe(),
        ),
        if d.date_changed {
            "Day date changed".to_string()
        } else {
            "Day date unchanged".to_string()
        },
    ];
    CallSheetRefreshPreview {
        has_changes: rows.iter().any(|r| r.changed)
            || !src.matches(&s.fingerprint, s.source.version),
        summary,
        scenes: d.scenes,
        cast: d.cast,
        locations: d.locations,
        date_changed: d.date_changed,
        other: d.other,
        rows,
        creates_revision: is_final(&s.status),
        day_deleted: false,
    }
}

/// FSD-CALL-010: preview differences; nothing is applied until "Apply Update".
fn refresh_preview(
    core: &AppCore,
    actor: &Actor,
    a: CallSheetIdArgs,
) -> AppResult<CallSheetRefreshPreview> {
    read(core, actor, |c| {
        let s = load_sheet(c, &a.id)?;
        let Some(src) = current_source(c, &s)? else {
            return Ok(CallSheetRefreshPreview {
                summary: Vec::new(),
                scenes: Default::default(),
                cast: Default::default(),
                locations: Default::default(),
                date_changed: false,
                other: Vec::new(),
                rows: Vec::new(),
                has_changes: false,
                creates_revision: false,
                day_deleted: true,
            });
        };
        Ok(preview(&s, &src))
    })
}

/// Apply the refresh to an editable sheet (Final sheets use a new revision).
fn refresh(core: &AppCore, actor: &Actor, a: CallSheetIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "callsheets.refresh",
        "Updated a call sheet from the schedule",
        Capability::Edit,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_sheet(c, &a.id)?;
        ensure_editable(&s)?;
        let src = current_source(c, &s)?
            .ok_or_else(|| AppError::conflict("The shooting day of this call sheet was deleted, so there is nothing to refresh from."))?;
        let doc = merge(&s.document, &s.source, &src);
        update_fields(
            c,
            "call_sheet",
            &a.id,
            &[
                ("document_json", text(check_doc(&doc)?)),
                (
                    "source_snapshot_json",
                    text(
                        serde_json::to_string(&src)
                            .map_err(|e| AppError::internal(e.to_string()))?,
                    ),
                ),
                ("source_fingerprint", text(src.fingerprint())),
                ("source_captured_at", int(now_ms())),
                ("needs_refresh", int(0)),
                ("status", text("Draft")),
                ("title", text(title_for(&src, s.revision))),
            ],
            &[
                "document_json",
                "source_snapshot_json",
                "source_fingerprint",
                "source_captured_at",
                "needs_refresh",
                "status",
                "title",
            ],
            None,
            "call sheet",
        )?;
        Ok(())
    })
}

// ------------------------------------------------------------------ finalize / revise

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetFinalizeArgs {
    pub id: String,
    /// Finalize even though the schedule changed after this sheet was prepared.
    #[serde(default)]
    pub acknowledge_stale: bool,
}

/// FSD §38.8 / §148: freeze an immutable snapshot of exactly this document.
fn finalize(core: &AppCore, actor: &Actor, a: CallSheetFinalizeArgs) -> AppResult<String> {
    let meta = MutationMeta::new(
        "callsheets.finalize",
        "Finalized a call sheet",
        Capability::LockOrFinalize,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_sheet(c, &a.id)?;
        ensure_editable(&s)?;
        if s.document.crew_call.trim().is_empty() {
            return Err(AppError::validation(
                "crew_call",
                "Enter the crew call time before finalizing.",
            ));
        }
        let changed = match current_source(c, &s)? {
            Some(src) => s.needs_refresh || !src.matches(&s.fingerprint, s.source.version),
            None => true,
        };
        if changed && !a.acknowledge_stale {
            return Err(AppError::new(
                "validation.call_sheet_stale",
                "The schedule changed after this call sheet was prepared. Refresh it first, or finalize this version anyway.",
            ));
        }
        let now = now_ms();
        let snapshot_id = new_id();
        let label = format!("{} · v{}", s.title, s.revision);
        let content = json!({ "document": s.document, "source": s.source, "finalizedAt": now, "revision": s.revision });
        c.execute(
            "INSERT INTO snapshot(id, snapshot_type, source_type, source_id, source_version, label, content_json, created_by, created_at, updated_at)
             VALUES (?1, 'Call Sheet', 'call_sheet', ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![snapshot_id, s.id, format!("v{}", s.revision), label, content.to_string(), tx.actor().user_id, now],
        )?;
        c.execute(
            "UPDATE call_sheet SET status = 'Final', finalized_at = ?1, finalized_by = ?2, snapshot_id = ?3, rev = rev + 1, updated_at = ?1 WHERE id = ?4",
            params![now, tx.actor().user_id, snapshot_id, s.id],
        )?;
        Ok(snapshot_id)
    })
}

/// A new revision document from the current day; the final one stays unchanged and records
/// which revision replaced it (FSD §148, §126).
fn new_revision(core: &AppCore, actor: &Actor, a: CallSheetIdArgs) -> AppResult<String> {
    let meta = MutationMeta::new(
        "callsheets.new_revision",
        "Started a new call sheet revision",
        Capability::Edit,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let s = load_sheet(c, &a.id)?;
        if !is_final(&s.status) {
            return Err(AppError::invalid_input(
                "This call sheet can still be edited. Use Refresh from schedule instead.",
            ));
        }
        if s.superseded_by.is_some() {
            return Err(AppError::conflict(
                "A newer revision of this call sheet already exists.",
            ));
        }
        let src = current_source(c, &s)?
            .ok_or_else(|| AppError::conflict("The shooting day of this call sheet was deleted, so a new revision can't be prepared."))?;
        let doc = merge(&s.document, &s.source, &src);
        let prior: i64 = c.query_row(
            "SELECT COALESCE(MAX(revision), 0) FROM call_sheet WHERE shoot_day_id = ?1",
            [&s.day_id],
            |r| r.get(0),
        )?;
        let id = insert_sheet(tx, &s.schedule_id, &src, &doc, prior + 1, Some(&s.id))?;
        c.execute(
            "UPDATE call_sheet SET superseded_by = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3",
            params![id, now_ms(), s.id],
        )?;
        Ok(id)
    })
}

// ------------------------------------------------------------------ attachments

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallSheetAttachArgs {
    pub id: String,
    /// "attachments" | "referenceImages".
    pub section: String,
    pub path: String,
}

fn attach(core: &AppCore, actor: &Actor, a: CallSheetAttachArgs) -> AppResult<CallSheetAttachment> {
    if a.section != "attachments" && a.section != "referenceImages" {
        return Err(AppError::invalid_input(
            "Choose Attachments or Reference images.",
        ));
    }
    let path = PathBuf::from(&a.path);
    if !path.is_file() {
        return Err(AppError::invalid_input("That file could not be found."));
    }
    let meta = MutationMeta::new(
        "callsheets.attach",
        "Attached a file to a call sheet",
        Capability::Edit,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let s = load_sheet(tx.conn(), &a.id)?;
        ensure_editable(&s)?;
        let asset = ingest_file(tx, &path)?;
        if a.section == "referenceImages" && !asset.media_type.starts_with("image/") {
            return Err(AppError::invalid_input(
                "Reference images must be image files (JPEG, PNG or WebP).",
            ));
        }
        let att = CallSheetAttachment {
            asset_id: asset.id.clone(),
            name: asset.original_name.clone(),
            media_type: asset.media_type.clone(),
        };
        let mut doc = s.document.clone();
        let list = if a.section == "attachments" {
            &mut doc.optional.attachments
        } else {
            &mut doc.optional.reference_images
        };
        list.get_or_insert_with(Vec::new).push(att.clone());
        doc.version = DOC_VERSION;
        update_fields(
            tx.conn(),
            "call_sheet",
            &a.id,
            &[("document_json", text(check_doc(&doc)?))],
            &["document_json"],
            None,
            "call sheet",
        )?;
        Ok(att)
    })
}

// ------------------------------------------------------------------ delete

fn delete(core: &AppCore, actor: &Actor, a: CallSheetIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "callsheets.delete",
        "Deleted a call sheet",
        Capability::SoftDelete,
    )
    .target("call_sheet", &a.id);
    write(core, actor, meta, |tx| {
        let s = load_sheet(tx.conn(), &a.id)?;
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "call_sheet",
                table: "call_sheet",
                id: &s.id,
                title: Some(s.title.clone()),
                parent_type: Some("shooting_day"),
                parent_id: Some(s.day_id.clone()),
                position: None,
            },
        )?;
        tx.reindex("shooting_day", &s.day_id);
        Ok(())
    })
}

fn purge_call_sheet(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let now = now_ms();
    c.execute("UPDATE call_sheet SET superseded_by = NULL, rev = rev + 1, updated_at = ?1 WHERE superseded_by = ?2", params![now, row.object_id])?;
    c.execute("UPDATE call_sheet SET previous_id = NULL, rev = rev + 1, updated_at = ?1 WHERE previous_id = ?2", params![now, row.object_id])?;
    // The final snapshot (if any) stays: final documents are historical (FSD §126).
    c.execute("DELETE FROM call_sheet WHERE id = ?1", [&row.object_id])?;
    Ok(())
}

// ------------------------------------------------------------------ search

fn index_call_sheet(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let Ok(s) = load_sheet(c, id) else {
        return Ok(None);
    };
    let d = &s.document;
    let mut body: Vec<String> = Vec::new();
    body.extend(
        d.scenes
            .iter()
            .map(|x| format!("{} {} {}", x.number, x.heading, x.description)),
    );
    body.extend(
        d.cast
            .iter()
            .map(|x| format!("{} {}", x.actor, x.character)),
    );
    body.extend(
        d.locations
            .iter()
            .map(|x| format!("{} {} {}", x.name, x.address, x.notes)),
    );
    let h = &d.header;
    body.push(format!("{} {} {}", h.company, h.director, h.producer));
    body.push(d.day_notes.clone());
    body.push(d.practical.production_notes.clone());
    body.push(d.practical.special_instructions.clone());
    let o = &d.optional;
    if let Some(n) = &o.special_notes {
        body.push(n.clone());
    }
    if let Some(w) = &o.weather {
        body.push(w.clone());
    }
    for x in o.departments.iter().flatten() {
        body.push(format!("{} {}", x.department, x.notes));
    }
    for x in o.extras.iter().flatten() {
        body.push(format!("{} {}", x.description, x.notes));
    }
    if let Some(a) = &o.advance {
        body.push(format!("{} {} {}", a.day, a.location, a.preparation));
    }
    if let Some(x) = &o.communication {
        body.push(x.note.clone());
    }
    let when = d
        .date
        .as_deref()
        .map(date_short)
        .map(|x| format!(" · {x}"))
        .unwrap_or_default();
    Ok(Some(SearchDoc {
        entity_type: "call_sheet".into(),
        title: format!("{}{when}", s.title),
        body: body
            .into_iter()
            .filter(|x| !x.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        context: "Call Sheets".into(),
        nav: json!({ "workspace": "callsheets", "callSheetId": id }),
        owner_user_id: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::schedule::callsheet_source::{
        CallSheetSourceCast, CallSheetSourceRow, CallSheetSourceScene,
    };

    fn src(scenes: &[&str], cast: &[(&str, &str)], date: Option<&str>) -> CallSheetSource {
        CallSheetSource {
            version: crate::modules::schedule::callsheet_source::SOURCE_VERSION,
            production_title: "Black Rain".into(),
            source_label: "Draft 6".into(),
            day_id: "d".into(),
            day_number: Some(4),
            day_label: "Shoot Day 4".into(),
            date: date.map(|d| d.to_string()),
            day_notes: None,
            scenes: scenes
                .iter()
                .map(|n| CallSheetSourceScene {
                    key: format!("L{n}"),
                    number: n.to_string(),
                    heading: format!("INT. X {n}"),
                    description: String::new(),
                    pages: "1".into(),
                    int_ext: "INT".into(),
                    day_night: "NIGHT".into(),
                    location: "X".into(),
                    cast_ids: "1".into(),
                    minutes: Some(60),
                })
                .collect(),
            cast: cast
                .iter()
                .enumerate()
                .map(|(i, (k, a))| CallSheetSourceCast {
                    key: k.to_string(),
                    character: k.to_uppercase(),
                    actor: Some(a.to_string()),
                    cast_id: Some(i as i64 + 1),
                })
                .collect(),
            locations: Vec::new(),
            breaks: Vec::new(),
            total_days: Some(12),
            times: Default::default(),
            rows: scenes
                .iter()
                .map(|n| CallSheetSourceRow {
                    key: format!("L{n}"),
                    kind: "Scene".into(),
                    start: None,
                    minutes: Some(60),
                })
                .collect(),
            extras: Vec::new(),
            departments: Vec::new(),
            advance: None,
        }
    }

    #[test]
    fn merge_keeps_a_typed_actor_and_follows_the_cast_record_otherwise() {
        let old = src(&["12"], &[("arjun", "Karthik"), ("meera", "Divya")], None);
        let new = src(
            &["12"],
            &[("arjun", "Karthik Menon"), ("meera", "Divya Nair")],
            None,
        );
        let mut doc = prefill(&old, &[]);
        doc.cast[1].actor = "Typed on the sheet".into();
        let m = merge(&doc, &old, &new);
        assert_eq!(
            m.cast[0].actor, "Karthik Menon",
            "a record value follows the record"
        );
        assert_eq!(m.cast[1].actor, "Typed on the sheet", "a typed value stays");
    }

    #[test]
    fn merge_keeps_call_times_and_follows_schedule() {
        let old = src(&["12", "17"], &[("arjun", "Karthik")], Some("2027-06-14"));
        let mut doc = prefill(&old, &[]);
        doc.crew_call = "18:00".into();
        doc.cast[0].call_time = "18:15".into();
        doc.cast[0].pickup = "17:30".into();
        doc.day_notes = "Blackout the windows".into();
        let new = src(
            &["12", "19"],
            &[("arjun", "Karthik"), ("meera", "Divya")],
            Some("2027-06-15"),
        );
        let merged = merge(&doc, &old, &new);
        assert_eq!(merged.crew_call, "18:00");
        assert_eq!(
            merged.cast[0].call_time, "18:15",
            "edited call times are kept where they still apply"
        );
        assert_eq!(merged.cast[0].pickup, "17:30");
        assert_eq!(
            merged.cast[1].call_time, "",
            "new cast gets a blank (needs input) call time"
        );
        assert_eq!(
            merged
                .scenes
                .iter()
                .map(|s| s.number.as_str())
                .collect::<Vec<_>>(),
            vec!["12", "19"]
        );
        assert_eq!(merged.date.as_deref(), Some("2027-06-15"));
        assert_eq!(
            merged.day_notes, "Blackout the windows",
            "user-edited notes are kept"
        );
        assert_eq!(changed_areas(&old, &new), vec!["Date", "Scenes", "Cast"]);
        let d = diff_sources(&old, &new);
        assert_eq!(d.scenes.added, vec!["19"]);
        assert_eq!(d.scenes.removed, vec!["17"]);
        assert_eq!(d.cast.added, vec!["MEERA"]);
    }

    #[test]
    fn estimated_starts_count_from_the_shooting_call_and_restart_at_own_times() {
        let mut doc = prefill(&src(&["1", "2", "3"], &[], None), &[]);
        assert_eq!(estimated_starts(&doc), vec!["", "", ""], "nothing invented");
        doc.times.shooting_call = "08:00".into();
        assert_eq!(estimated_starts(&doc), vec!["08:00", "09:00", "10:00"]);
        doc.scenes[1].start = "09:30".into();
        assert_eq!(estimated_starts(&doc), vec!["08:00", "09:30", "10:30"]);
        doc.scenes[1].minutes = None;
        assert_eq!(
            estimated_starts(&doc),
            vec!["08:00", "09:30", ""],
            "an unknown length stops the estimate"
        );
    }

    #[test]
    fn documents_written_before_version_2_still_read() {
        let v1 = r#"{"title":"Black Rain","dayLabel":"SHOOT DAY 4","date":"2027-06-14","crewCall":"18:00",
            "dayNotes":"","locations":[{"key":"loc:1","name":"Police Station","address":"District Rd","meetingPoint":"Gate","parking":"East lot"}],
            "cast":[{"key":"char:1","actor":"Karthik","character":"ARJUN","callTime":"18:15","notes":""}],
            "scenes":[{"key":"L12","number":"12","heading":"INT. POLICE STATION - NIGHT","description":"","pages":"1"}],
            "practical":{"parking":"","meetingPoint":"","travelNotes":"","mealBreak":"Meal 20:30","emergencyContact":"","productionNotes":""},
            "optional":{"weather":"Rain","specialNotes":null,"attachments":null,"referenceImages":null},"extraFields":[]}"#;
        let doc = parse_doc(v1).expect("a version 1 document reads");
        assert_eq!(doc.version, 0);
        assert_eq!(doc.crew_call, "18:00");
        assert_eq!(doc.locations[0].parking, "East lot");
        assert_eq!(doc.optional.weather.as_deref(), Some("Rain"));
        assert!(
            doc.optional.crew.is_none(),
            "new optional sections stay hidden"
        );
        assert!(doc.scenes[0].is_scene());
        assert!(
            parse_doc("{\"title\": 5}").is_err(),
            "a damaged document is an error, never an empty sheet"
        );
    }

    #[test]
    fn statuses_use_the_three_words() {
        assert_eq!(effective_status("Draft", false), "Draft");
        assert_eq!(effective_status("Draft", true), "Needs refresh");
        assert_eq!(effective_status("Ready", false), "Draft");
        assert_eq!(effective_status("Final", true), "Final");
        assert_eq!(effective_status("Issued", false), "Final");
        assert_eq!(effective_status("Superseded", true), "Final");
    }
}

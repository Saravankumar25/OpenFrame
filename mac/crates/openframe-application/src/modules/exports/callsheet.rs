//! Call Sheet PDF (FSD §38.9 "PDF is required. The call sheet is a snapshot suitable for
//! sending"; Import/Export §6 "the exact Call Sheet document state being exported … does not
//! update the Shooting Schedule"; production depth spec §29 and §30).
//!
//! Paper layout of mock 143: production header, day and date, crew call, then dark section
//! bars for the six default sections (production header details, day and times, locations,
//! scene schedule with start times and schedule rows, talent, practical notes) and then the
//! optional sections the user added. Empty optional sections are left out. A value nobody has
//! entered yet prints "Needs input". A Final sheet is exported from its immutable finalize
//! snapshot, never from later state.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{Block, Column, Document, Grid, GridCell, Table};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde_json::Value;
use ts_rs::TS;

use super::{
    Fmt, HINT_EDIT, Job, Output, date_from_ms, deliver, destination, footer, image_warnings,
    load_image, nonblank, plural, require_export,
};
use crate::core::AppCore;
use crate::modules::schedule::board::project_title;
use crate::modules::schedule::callsheet::{
    CallSheetDocument, NEEDS_INPUT, estimated_starts, is_final,
};
use crate::modules::schedule::fmt::{date_long_upper, minutes};

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportCallSheetArgs {
    /// Call sheet id.
    pub id: String,
    /// "pdf"
    pub format: String,
    pub path: String,
    /// Optional sections the user added (weather, extras, department notes, advance schedule,
    /// crew, communication, special notes, attachments list, reference images). Default true.
    #[serde(default)]
    #[ts(optional)]
    pub include_optional: Option<bool>,
}

/// The value, or "Needs input" when a value the call sheet needs is empty.
fn need(v: &str) -> String {
    nonblank(Some(v)).unwrap_or_else(|| NEEDS_INPUT.into())
}

fn key_values(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .filter_map(|(k, v)| nonblank(Some(v)).map(|v| (k.to_string(), v)))
        .collect()
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportCallSheetArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf])?;
    crate::util::require_id(&a.id, "call sheet")?;
    let optional = a.include_optional.unwrap_or(true);
    let dest = destination(core, &a.path, fmt)?;
    let session = core.project()?;
    let root = session.layout.root().to_path_buf();

    struct Sheet {
        title: String,
        status: String,
        revision: i64,
        document: CallSheetDocument,
        source: Value,
        frozen_at: Option<i64>,
        from_snapshot: bool,
        replaced: bool,
        project: String,
        images: Vec<(String, Option<crate::util::AssetInfo>)>,
    }
    let sheet = session.store.read(|c| {
        #[allow(clippy::type_complexity)]
        let row: Option<(String, String, i64, String, String, Option<String>, Option<i64>, Option<String>)> = c
            .query_row(
                "SELECT title, status, revision, document_json, source_snapshot_json, snapshot_id, finalized_at, superseded_by
                 FROM call_sheet WHERE id = ?1 AND deleted_at IS NULL",
                [&a.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
            )
            .optional()?;
        let (title, status, revision, doc_json, src_json, snapshot_id, finalized_at, superseded_by) =
            row.ok_or_else(|| AppError::not_found("call sheet"))?;
        let frozen = is_final(&status);
        // Final documents come from their immutable snapshot (FSD §38.8, §148).
        let snapshot: Option<Value> = match (&snapshot_id, frozen) {
            (Some(sid), true) => c
                .query_row("SELECT content_json FROM snapshot WHERE id = ?1", [sid], |r| r.get::<_, String>(0))
                .optional()?
                .and_then(|j| serde_json::from_str(&j).ok()),
            _ => None,
        };
        let damaged = |e: serde_json::Error| {
            AppError::new(
                "integrity.call_sheet",
                "This call sheet could not be read, so no PDF was written. Restore it from a backup.",
            )
            .with_detail(e.to_string())
        };
        let (document, source, from_snapshot) = match snapshot {
            Some(v) => (
                serde_json::from_value::<CallSheetDocument>(v["document"].clone()).map_err(damaged)?,
                v["source"].clone(),
                true,
            ),
            None => (
                serde_json::from_str::<CallSheetDocument>(&doc_json).map_err(damaged)?,
                serde_json::from_str::<Value>(&src_json).unwrap_or(Value::Null),
                false,
            ),
        };
        let mut images = Vec::new();
        if optional {
            for att in document.optional.reference_images.iter().flatten() {
                let info = crate::util::load_asset(c, &root, &att.asset_id).ok();
                images.push((att.name.clone(), info));
            }
        }
        Ok(Sheet {
            title,
            status,
            revision,
            document,
            source,
            frozen_at: finalized_at,
            from_snapshot,
            replaced: superseded_by.is_some(),
            project: project_title(c)?,
            images,
        })
    })?;

    let d = &sheet.document;
    let production = nonblank(Some(&d.title))
        .or_else(|| {
            sheet.source["productionTitle"]
                .as_str()
                .and_then(|s| nonblank(Some(s)))
        })
        .unwrap_or_else(|| sheet.project.clone())
        .to_uppercase();
    let source_label = sheet.source["sourceLabel"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let mut header = vec![d.day_label.trim().to_uppercase()];
    if let Some(of) = nonblank(Some(&d.day_of)) {
        header.push(of.to_uppercase());
    }
    header.push(
        d.date
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(date_long_upper)
            .unwrap_or_else(|| format!("DATE: {}", NEEDS_INPUT.to_uppercase())),
    );
    let header: Vec<String> = header.into_iter().filter(|s| !s.is_empty()).collect();
    let mut doc = Document::new(if production.is_empty() {
        "CALL SHEET".to_string()
    } else {
        production.clone()
    })
    .centered_header(true)
    .subtitle(header.join(" · "))
    .footer(footer(&sheet.project, &sheet.title));
    let crew = nonblank(Some(&d.crew_call));
    doc.push(Block::Centered {
        text: format!("CREW CALL: {}", need(&d.crew_call)),
        size: 13.0,
        bold: true,
    });
    let state = if is_final(&sheet.status) {
        format!(
            "Final{}{}",
            sheet
                .frozen_at
                .map(|t| format!(" {}", date_from_ms(t)))
                .unwrap_or_default(),
            if sheet.replaced {
                " (replaced by a newer revision)"
            } else {
                ""
            }
        )
    } else {
        "DRAFT, not final".into()
    };
    let mut status_line = vec![sheet.title.clone(), format!("v{}", sheet.revision), state];
    if !source_label.is_empty() {
        status_line.push(format!("Script: {source_label}"));
    }
    doc.push(Block::Centered {
        text: status_line.join(" · "),
        size: 8.5,
        bold: false,
    });
    doc.push(Block::Spacer(4.0));

    // 1. Production header
    let h = &d.header;
    let head = key_values(&[
        ("Production company", &h.company),
        ("Production address", &h.address),
        ("Producer", &h.producer),
        ("Director", &h.director),
        ("1st AD", &h.first_ad),
        ("Production contact", &h.contact),
    ]);
    if !head.is_empty() {
        doc.push(Block::SectionBar("Production".into()));
        doc.push(Block::KeyValues(head));
    }

    // 2. Day and times
    doc.push(Block::SectionBar("Day and times".into()));
    let t = &d.times;
    let mut times = vec![("Crew call".to_string(), need(&d.crew_call))];
    times.extend(key_values(&[
        ("General call", &t.general_call),
        ("Shooting call", &t.shooting_call),
        ("Breakfast", &t.breakfast),
        ("Lunch", &t.lunch),
        ("Estimated wrap", &t.estimated_wrap),
    ]));
    doc.push(Block::KeyValues(times));

    // 3. Locations
    doc.push(Block::SectionBar("Locations".into()));
    if d.locations.is_empty() {
        doc.push(Block::Note("No location for this day.".into()));
    }
    for l in &d.locations {
        let mut kv = vec![
            ("Location".to_string(), l.name.clone()),
            ("Address".to_string(), need(&l.address)),
        ];
        kv.extend(key_values(&[
            ("Meeting point", &l.meeting_point),
            ("Access", &l.access),
            ("Parking", &l.parking),
        ]));
        kv.push(("Nearest hospital".to_string(), need(&l.hospital)));
        kv.extend(key_values(&[
            ("Emergency contact", &l.emergency_contact),
            ("Notes", &l.notes),
        ]));
        doc.push(Block::KeyValues(kv));
    }

    // 4. Scene schedule
    doc.push(Block::SectionBar("Scene schedule".into()));
    if d.scenes.is_empty() {
        doc.push(Block::Note("No scenes on this day.".into()));
    } else {
        let starts = estimated_starts(d);
        let mut t = Table::new(
            "",
            vec![
                Column::new("Start").weight(0.5),
                Column::new("Sc").weight(0.35),
                Column::new("Heading").weight(1.9),
                Column::new("Description").weight(1.8),
                Column::new("D/N").weight(0.45),
                Column::new("Cast").weight(0.55),
                Column::right("Pages").weight(0.5),
                Column::right("Est.").weight(0.5),
            ],
        );
        for (i, s) in d.scenes.iter().enumerate() {
            let start = starts.get(i).cloned().unwrap_or_default();
            let est = s.minutes.map(minutes).unwrap_or_default();
            if s.is_scene() {
                let heading = if s.location.trim().is_empty()
                    || s.heading
                        .to_uppercase()
                        .contains(&s.location.to_uppercase())
                {
                    s.heading.clone()
                } else {
                    format!("{} ({})", s.heading, s.location)
                };
                t.push(vec![
                    start,
                    s.number.clone(),
                    heading,
                    s.description.clone(),
                    s.day_night.clone(),
                    s.cast_ids.clone(),
                    s.pages.clone(),
                    est,
                ]);
            } else {
                t.push(vec![
                    start,
                    String::new(),
                    format!("{}: {}", s.kind.to_uppercase(), s.heading),
                    s.description.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                    est,
                ]);
            }
        }
        doc.push(Block::Table(t));
    }

    // 5. Talent
    doc.push(Block::SectionBar("Talent".into()));
    if d.cast.is_empty() {
        doc.push(Block::Note("No cast called.".into()));
    } else {
        let any = |f: fn(&crate::modules::schedule::callsheet::CallSheetDocCast) -> &String| {
            d.cast.iter().any(|c| !f(c).trim().is_empty())
        };
        let (has_status, has_pickup, has_makeup, has_contact, has_notes) = (
            any(|c| &c.status),
            any(|c| &c.pickup),
            any(|c| &c.makeup),
            any(|c| &c.contact),
            any(|c| &c.notes),
        );
        let mut cols = vec![
            Column::new("ID").weight(0.35),
            Column::new("Actor").weight(1.3),
            Column::new("Character").weight(1.2),
        ];
        if has_status {
            cols.push(Column::new("Status").weight(0.5));
        }
        if has_pickup {
            cols.push(Column::new("Pickup").weight(0.6));
        }
        if has_makeup {
            cols.push(Column::new("Hair / MU").weight(0.6));
        }
        cols.push(Column::new("Call").weight(0.6));
        if has_contact {
            cols.push(Column::new("Contact").weight(1.0));
        }
        if has_notes {
            cols.push(Column::new("Notes").weight(1.4));
        }
        let mut t = Table::new("", cols);
        for c in &d.cast {
            let mut r = vec![c.cast_id.clone(), need(&c.actor), c.character.clone()];
            if has_status {
                r.push(c.status.clone());
            }
            if has_pickup {
                r.push(c.pickup.clone());
            }
            if has_makeup {
                r.push(c.makeup.clone());
            }
            r.push(need(&c.call_time));
            if has_contact {
                r.push(c.contact.clone());
            }
            if has_notes {
                r.push(c.notes.clone());
            }
            t.push(r);
        }
        doc.push(Block::Table(t));
    }

    // 6. Practical notes
    doc.push(Block::SectionBar("Practical notes".into()));
    let p = &d.practical;
    let mut kv = key_values(&[
        ("Parking", &p.parking),
        ("Meeting point", &p.meeting_point),
        ("Transport", &p.travel_notes),
        ("Meals", &p.meal_break),
        ("Toilets", &p.toilets),
        ("Access", &p.access),
        ("Safety", &p.safety),
    ]);
    kv.push(("Emergency contact".to_string(), need(&p.emergency_contact)));
    kv.extend(key_values(&[
        ("Special instructions", &p.special_instructions),
        ("Production notes", &p.production_notes),
        ("Day notes", &d.day_notes),
    ]));
    for f in &d.extra_fields {
        if let (Some(k), Some(v)) = (nonblank(Some(&f.label)), nonblank(Some(&f.value))) {
            kv.push((k, v));
        }
    }
    doc.push(Block::KeyValues(kv));

    // Optional sections: only the ones the user added and filled.
    let mut missing = 0;
    let mut unreadable = 0;
    if optional {
        let o = &d.optional;
        if o.weather.is_some() {
            let kv = key_values(&[
                ("Forecast", o.weather.as_deref().unwrap_or("")),
                ("Sunrise", &o.sunrise),
                ("Sunset", &o.sunset),
            ]);
            if !kv.is_empty() {
                doc.push(Block::SectionBar("Weather".into()));
                doc.push(Block::KeyValues(kv));
            }
        }
        if let Some(list) = o.extras.as_ref().filter(|l| {
            l.iter()
                .any(|e| !e.description.trim().is_empty() || !e.quantity.trim().is_empty())
        }) {
            doc.push(Block::SectionBar("Extras".into()));
            let mut t = Table::new(
                "",
                vec![
                    Column::new("Group").weight(1.6),
                    Column::right("Qty").weight(0.4),
                    Column::new("Call").weight(0.6),
                    Column::new("On set").weight(0.6),
                    Column::new("Notes").weight(1.6),
                ],
            );
            for e in list {
                t.push(vec![
                    e.description.clone(),
                    e.quantity.clone(),
                    need(&e.call_time),
                    e.on_set.clone(),
                    e.notes.clone(),
                ]);
            }
            doc.push(Block::Table(t));
        }
        if let Some(list) = o.departments.as_ref() {
            let kv: Vec<(String, String)> = list
                .iter()
                .filter_map(|x| {
                    nonblank(Some(&x.notes)).map(|n| {
                        (
                            nonblank(Some(&x.department)).unwrap_or_else(|| "Other".into()),
                            n,
                        )
                    })
                })
                .collect();
            if !kv.is_empty() {
                doc.push(Block::SectionBar("Department notes".into()));
                doc.push(Block::KeyValues(kv));
            }
        }
        if let Some(adv) = &o.advance {
            let kv = key_values(&[
                ("Day", &adv.day),
                ("Location", &adv.location),
                ("Cast", &adv.cast),
                ("Props", &adv.props),
                ("Wardrobe", &adv.wardrobe),
                ("Preparation", &adv.preparation),
            ]);
            if !kv.is_empty() {
                doc.push(Block::SectionBar("Advance schedule".into()));
                doc.push(Block::KeyValues(kv));
            }
        }
        if let Some(list) = o.crew.as_ref().filter(|l| !l.is_empty()) {
            doc.push(Block::SectionBar("Crew".into()));
            let has_contact = list.iter().any(|c| !c.contact.trim().is_empty());
            let mut cols = vec![
                Column::new("Department").weight(1.0),
                Column::new("Role").weight(1.1),
                Column::new("Name").weight(1.3),
                Column::new("Call").weight(0.6),
            ];
            if has_contact {
                cols.push(Column::new("Contact").weight(1.1));
            }
            let mut t = Table::new("", cols);
            let mut sorted: Vec<_> = list.iter().collect();
            sorted.sort_by(|a, b| {
                a.department
                    .to_lowercase()
                    .cmp(&b.department.to_lowercase())
            });
            for c in sorted {
                let mut r = vec![
                    c.department.clone(),
                    c.role.clone(),
                    c.name.clone(),
                    nonblank(Some(&c.call_time)).unwrap_or_else(|| need(&d.crew_call)),
                ];
                if has_contact {
                    r.push(c.contact.clone());
                }
                t.push(r);
            }
            doc.push(Block::Table(t));
        }
        if let Some(cm) = &o.communication {
            let kv = key_values(&[
                ("Walkie channels", &cm.walkie_channels),
                ("Emergency contact", &cm.emergency_contact),
                ("Production contact", &cm.production_contact),
                ("Note", &cm.note),
            ]);
            if !kv.is_empty() {
                doc.push(Block::SectionBar("Walkie channels and contacts".into()));
                doc.push(Block::KeyValues(kv));
            }
        }
        if let Some(s) = nonblank(o.special_notes.as_deref()) {
            doc.push(Block::SectionBar("Special notes".into()));
            doc.push(Block::Paragraph(s));
        }
        if let Some(list) = o.attachments.as_ref().filter(|l| !l.is_empty()) {
            doc.push(Block::SectionBar("Attachments".into()));
            doc.push(Block::Paragraph(format!(
                "Sent separately: {}",
                list.iter()
                    .map(|x| x.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        if !sheet.images.is_empty() {
            doc.push(Block::SectionBar("Reference images".into()));
            let cells = sheet
                .images
                .iter()
                .map(|(name, info)| {
                    let mut cell = GridCell {
                        lines: vec![name.clone()],
                        ..Default::default()
                    };
                    match info.as_ref().map(load_image) {
                        Some(Ok(img)) => cell.image = Some(img),
                        Some(Err("unreadable")) | Some(Err("too large")) => {
                            unreadable += 1;
                            cell.placeholder = Some("Image can't be printed".into());
                        }
                        _ => {
                            missing += 1;
                            cell.placeholder = Some("Image unavailable".into());
                        }
                    }
                    cell
                })
                .collect();
            doc.push(Block::Grid(Grid {
                columns: 3,
                frame_ratio: 0.66,
                cells,
            }));
        }
    }

    let mut out = Output::new(doc, HINT_EDIT);
    out.warnings = image_warnings(missing, unreadable, "reference image");
    if is_final(&sheet.status) {
        if sheet.replaced {
            out.warnings.push(
                "A newer revision of this call sheet exists. The PDF shows this final version."
                    .into(),
            );
        }
    } else {
        out.warnings
            .push("This call sheet is not final yet, so the PDF is marked Draft.".into());
    }
    if crew.is_none() {
        out.warnings.push("The crew call time is empty.".into());
    }
    let job = Job {
        action: "callsheets.export_pdf",
        document: sheet.title.clone(),
        source_label: if sheet.from_snapshot {
            format!("{} · v{} (final snapshot)", sheet.title, sheet.revision)
        } else {
            format!("{} · v{}", sheet.title, sheet.revision)
        },
        scope_label: "Current call sheet".into(),
        contents_label: format!(
            "{} · {}",
            plural(
                d.scenes.iter().filter(|s| s.is_scene()).count(),
                "scene",
                "scenes"
            ),
            plural(d.cast.len(), "cast call", "cast calls")
        ),
        target: Some(("call_sheet", a.id.clone())),
    };
    deliver(core, actor, &dest, fmt, job, out)
}

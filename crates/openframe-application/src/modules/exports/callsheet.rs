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
use openframe_import_export::report::{
    Align, Block, Callout, CalloutCol, Card, CardHead, CardPart, Cards, Cell, CellItem, Chip, Col,
    Document, Field, HeaderStyle, Masthead, Rgb, Row, Section, Side, StyledTable, Tile, Tone, Txt,
    pal,
};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde_json::Value;
use ts_rs::TS;

use super::style::{
    cell, cell_bold, cell_or_missing, day_number, long_date, nb, number_pills, split_list,
};
use super::{
    Fmt, HINT_EDIT, Job, Output, date_from_ms, deliver, destination, footer, image_warnings,
    load_image, nonblank, plural, require_export,
};
use crate::core::AppCore;
use crate::modules::schedule::board::project_title;
use crate::modules::schedule::callsheet::{
    CallSheetDocument, NEEDS_INPUT, estimated_starts, is_final,
};
use crate::modules::schedule::fmt::minutes;

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
    let crew = nonblank(Some(&d.crew_call));
    let final_sheet = is_final(&sheet.status);
    let scene_count = d.scenes.iter().filter(|s| s.is_scene()).count();
    let doc_title = if production.is_empty() {
        "CALL SHEET".to_string()
    } else {
        production.clone()
    };
    let mut doc = Document::new(&doc_title).footer(footer(&sheet.project, &sheet.title));

    // ---- masthead: production first, the shoot day in its own block
    let mut mast = Masthead::new(doc_title.clone()).eyebrow(
        format!("Call sheet · Revision {}", sheet.revision),
        pal::SLATE,
    );
    if final_sheet {
        mast.badges.push(Chip::label(
            format!(
                "Final{}",
                sheet
                    .frozen_at
                    .map(|t| format!(" · {}", date_from_ms(t)))
                    .unwrap_or_default()
            ),
            Tone::Ok,
        ));
        if sheet.replaced {
            mast.badges
                .push(Chip::label("Replaced by a newer revision", Tone::Danger));
        }
    } else {
        mast.badges
            .push(Chip::label("Draft · Not final", Tone::Warn));
    }
    if !source_label.is_empty() {
        mast.badge_note = Some(format!("Script: {source_label}"));
    }
    mast.lines.push(format!(
        "{} · {} · {}",
        plural(scene_count, "scene", "scenes"),
        plural(d.cast.len(), "cast member", "cast members"),
        plural(d.locations.len(), "location", "locations")
    ));
    let mut side_lines: Vec<(String, bool)> = Vec::new();
    match d.date.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(date) => match long_date(date) {
            Some((wd, rest)) => {
                side_lines.push((wd, true));
                side_lines.push((rest, true));
            }
            None => side_lines.push((date.to_string(), true)),
        },
        None => side_lines.push(("Date needs input".into(), true)),
    }
    let day_label = d.day_label.trim();
    mast.side = Some(Side::Dark {
        label: if day_number(day_label).is_some() || day_label.is_empty() {
            "Shoot day".into()
        } else {
            day_label.to_string()
        },
        big: day_number(day_label),
        lines: side_lines,
        foot: nonblank(Some(&d.day_of)),
    });
    doc.push(Block::Masthead(mast));

    // ---- crew call and the day's other times
    let t = &d.times;
    let time_cols: Vec<(&str, &str)> = [
        ("General call", t.general_call.as_str()),
        ("Shooting call", t.shooting_call.as_str()),
        ("Breakfast", t.breakfast.as_str()),
        ("Lunch", t.lunch.as_str()),
        ("Est. wrap", t.estimated_wrap.as_str()),
    ]
    .into_iter()
    .filter(|(_, v)| !v.trim().is_empty())
    .collect();
    let mut cols = Vec::new();
    let mut first = CalloutCol::new(1.5);
    first.eyebrow = Some(
        Txt::new(if crew.is_some() {
            "CREW CALL"
        } else {
            "CREW CALL · REQUIRED"
        })
        .size(7.6)
        .bold()
        .track(1.4)
        .color(pal::ORANGE_DEEP),
    );
    match &crew {
        Some(c) => {
            first.big = Some(
                Txt::new(c.clone())
                    .size(26.0)
                    .bold()
                    .color(pal::NAVY)
                    .leading(1.1),
            )
        }
        None => first.chip = Some(Chip::label(NEEDS_INPUT, Tone::Danger)),
    }
    cols.push(first);
    for (label, value) in &time_cols {
        let mut c = CalloutCol::new(1.0);
        c.eyebrow = Some(Txt::label(*label));
        c.big = Some(
            Txt::new(value.trim())
                .size(13.0)
                .bold()
                .color(pal::NAVY)
                .leading(1.2),
        );
        cols.push(c);
    }
    if crew.is_none() && !final_sheet {
        let mut c = CalloutCol::new(1.7);
        c.lines.push(
            Txt::new("Not ready to issue")
                .size(9.2)
                .bold()
                .color(pal::ORANGE_DEEP)
                .after(2.0),
        );
        c.lines.push(
            Txt::new("Complete the missing call-sheet details, then finalize.")
                .size(8.4)
                .color(pal::ORANGE_DEEP),
        );
        cols.push(c);
    }
    doc.push(Block::Callout(Callout {
        tone: if crew.is_some() {
            Tone::Accent
        } else {
            Tone::Warn
        },
        cols,
    }));

    // ---- production header details
    let h = &d.header;
    let head: Vec<Field> = [
        ("Production company", &h.company),
        ("Production address", &h.address),
        ("Producer", &h.producer),
        ("Director", &h.director),
        ("1st AD", &h.first_ad),
        ("Production contact", &h.contact),
    ]
    .into_iter()
    .filter_map(|(k, v)| nb(v).map(|v| Field::new(k, v)))
    .collect();
    if !head.is_empty() {
        doc.push(Block::Section(Section::new("Production", None)));
        let half = head.len().div_ceil(2);
        let cards = head
            .chunks(half)
            .map(|c| Card {
                parts: vec![CardPart::Fields(c.to_vec())],
                ..Card::new()
            })
            .collect::<Vec<_>>();
        doc.push(Block::Cards(Cards::new(2, cards)));
    }

    // ---- locations
    doc.push(Block::Section(Section::new(
        "Locations",
        Some(plural(d.locations.len(), "location", "locations")),
    )));
    if d.locations.is_empty() {
        doc.push(Block::Text(
            Txt::new("No location for this day.")
                .size(8.8)
                .color(pal::FAINT)
                .after(6.0),
        ));
    } else {
        let cards = d
            .locations
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let mut head = CardHead::new(l.name.to_uppercase());
                head.tile = Some(Tile::new(format!("{:02}", i + 1), pal::TEAL, pal::WHITE));
                head.fill = pal::TINT;
                let mut card = Card {
                    head: Some(head),
                    ..Card::new()
                };
                let mut fields = Vec::new();
                match nb(&l.address) {
                    Some(a) => card.parts.push(CardPart::Lead(
                        Txt::new(a).size(9.6).bold().color(pal::INK).leading(1.3),
                    )),
                    None => fields.push(Field::missing("Address", NEEDS_INPUT)),
                }
                if let Some(v) = nb(&l.meeting_point) {
                    fields.push(Field::new("Meeting point", v));
                }
                if let Some(v) = nb(&l.access) {
                    fields.push(Field::new("Access", v));
                }
                if let Some(v) = nb(&l.parking) {
                    fields.push(Field::new("Parking", v));
                }
                fields.push(match nb(&l.hospital) {
                    Some(v) => Field::new("Nearest hospital", v),
                    None => Field::missing("Nearest hospital", NEEDS_INPUT),
                });
                fields.push(match nb(&l.emergency_contact) {
                    Some(v) => Field::new("Emergency contact", v),
                    None => Field::missing("Emergency contact", NEEDS_INPUT),
                });
                if let Some(v) = nb(&l.notes) {
                    fields.push(Field::new("Notes", v));
                }
                card.parts.push(CardPart::Fields(fields));
                card
            })
            .collect::<Vec<_>>();
        let columns = if d.locations.len() == 1 { 1 } else { 2 };
        doc.push(Block::Cards(Cards::new(columns, cards)));
    }

    // ---- scene schedule
    let any_entered_start = d.scenes.iter().any(|s| !s.start.trim().is_empty());
    doc.push(Block::Section(Section::new(
        "Scene schedule",
        Some(format!(
            "{}{}",
            plural(scene_count, "scheduled scene", "scheduled scenes"),
            if any_entered_start {
                " · Start times from the schedule"
            } else {
                " · Start times not entered"
            }
        )),
    )));
    if d.scenes.is_empty() {
        doc.push(Block::Text(
            Txt::new("No scenes on this day.")
                .size(8.8)
                .color(pal::FAINT)
                .after(6.0),
        ));
    } else {
        let starts = estimated_starts(d);
        let mut tbl = StyledTable::new(vec![
            Col::pt("Start", 50.0),
            Col::pt("Sc.", 36.0),
            Col::weight("Heading / description", 1.0),
            Col::pt("Cast", 64.0),
            Col::pt("Pages", 48.0),
            Col::pt("Est.", 48.0).align(Align::Right),
        ]);
        tbl.header = HeaderStyle::Navy;
        tbl.boxed = true;
        tbl.zebra = true;
        tbl.continued = Some("Scene schedule".into());
        for (i, s) in d.scenes.iter().enumerate() {
            let estimated = starts.get(i).cloned().unwrap_or_default();
            let start_cell = if let Some(v) = nb(&s.start) {
                cell_bold(v)
            } else if !estimated.is_empty() {
                Cell::text(Txt::new(estimated.clone()).size(8.6).color(pal::SLATE)).item(
                    CellItem::Text(
                        Txt::new("ESTIMATED")
                            .size(5.8)
                            .bold()
                            .track(0.5)
                            .color(pal::FAINT),
                    ),
                )
            } else {
                Cell::text(Txt::new("Not set").size(8.4).bold().color(pal::FAINT))
            };
            let est = s.minutes.map(minutes).unwrap_or_default();
            let est_cell = Cell::text(
                Txt::new(if est.is_empty() {
                    "\u{2014}".to_string()
                } else {
                    est
                })
                .size(8.4)
                .color(pal::SLATE),
            )
            .align(Align::Right);
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
                let mut body = Cell::text(
                    Txt::new(heading.to_uppercase())
                        .size(8.8)
                        .bold()
                        .color(pal::NAVY)
                        .leading(1.25),
                );
                if let Some(desc) = nb(&s.description) {
                    body = body.item(CellItem::Text(
                        Txt::new(desc).size(8.6).color(pal::SLATE).leading(1.3),
                    ));
                }
                let mut chips = Vec::new();
                for v in [&s.int_ext, &s.day_night] {
                    if let Some(v) = nb(v) {
                        chips.push(Chip::label(v, Tone::Neutral).small());
                    }
                }
                if !chips.is_empty() {
                    body = body.item(CellItem::Chips(chips));
                }
                let ids = split_list(&s.cast_ids);
                let cast_cell = if ids.is_empty() {
                    Cell::empty()
                } else {
                    Cell::new().item(CellItem::Chips(number_pills(&ids, Tone::Info)))
                };
                tbl.rows.push(Row::new(vec![
                    start_cell,
                    Cell::new().item(CellItem::Tile(s.number.clone())),
                    body,
                    cast_cell,
                    cell(s.pages.clone()),
                    est_cell,
                ]));
            } else {
                // Meal, travel, company move and custom rows keep their meaning in a tinted row.
                let (fill, stripe) = match s.kind.as_str() {
                    "Meal" => (Rgb::hex(0xfff6e3), pal::ORANGE),
                    "Travel" | "Company Move" => (Rgb::hex(0xeaf3fa), pal::TEAL),
                    _ => (pal::PANEL, pal::FAINT),
                };
                let mut body = Cell::new().item(CellItem::Chips(vec![
                    Chip::label(s.kind.clone(), Tone::Neutral).small(),
                ]));
                if let Some(h) = nb(&s.heading) {
                    body = body.item(CellItem::Text(
                        Txt::new(h).size(8.8).bold().color(pal::NAVY),
                    ));
                }
                if let Some(desc) = nb(&s.description) {
                    body = body.item(CellItem::Text(Txt::new(desc).size(8.4).color(pal::SLATE)));
                }
                let mut row = Row::new(vec![
                    start_cell,
                    Cell::empty(),
                    body,
                    Cell::empty(),
                    Cell::empty(),
                    est_cell,
                ]);
                row.fill = Some(fill);
                row.stripe = Some(stripe);
                tbl.rows.push(row);
            }
        }
        doc.push(Block::Styled(tbl));
    }

    // ---- talent
    doc.push(Block::Section(Section::new(
        "Talent / Cast",
        Some(plural(d.cast.len(), "cast member", "cast members")),
    )));
    if d.cast.is_empty() {
        doc.push(Block::Text(
            Txt::new("No cast called.")
                .size(8.8)
                .color(pal::FAINT)
                .after(6.0),
        ));
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
            Col::pt("ID", 32.0),
            Col::weight("Actor", 1.3),
            Col::weight("Character", 1.2),
        ];
        if has_status {
            cols.push(Col::pt("Status", 44.0));
        }
        if has_pickup {
            cols.push(Col::pt("Pickup", 52.0));
        }
        if has_makeup {
            cols.push(Col::pt("Hair / MU", 56.0));
        }
        cols.push(Col::weight("Call", 0.8));
        if has_contact {
            cols.push(Col::weight("Contact", 1.0));
        }
        if has_notes {
            cols.push(Col::weight("Notes", 1.3));
        }
        let mut tbl = StyledTable::new(cols);
        tbl.continued = Some("Talent / Cast".into());
        for c in &d.cast {
            let mut cells = vec![
                Cell::text(
                    Txt::new(c.cast_id.clone())
                        .size(8.8)
                        .bold()
                        .color(pal::TEAL),
                ),
                cell_or_missing(&c.actor, true),
                cell_bold(c.character.clone()),
            ];
            if has_status {
                cells.push(cell(c.status.clone()));
            }
            if has_pickup {
                cells.push(cell(c.pickup.clone()));
            }
            if has_makeup {
                cells.push(cell(c.makeup.clone()));
            }
            cells.push(cell_or_missing(&c.call_time, true));
            if has_contact {
                cells.push(cell(c.contact.clone()));
            }
            if has_notes {
                cells.push(cell(c.notes.clone()));
            }
            tbl.rows.push(Row::new(cells));
        }
        doc.push(Block::Styled(tbl));
    }

    // ---- practical / safety
    let p = &d.practical;
    doc.push(Block::Section(Section::new(
        "Practical / Safety information",
        Some(if final_sheet {
            "Quick reference".to_string()
        } else {
            "Review before issuing".to_string()
        }),
    )));
    let labeled = |label: String, value: &str| Card {
        parts: vec![CardPart::Labeled {
            label,
            value: nb(value).unwrap_or(NEEDS_INPUT).to_string(),
            missing: nb(value).is_none(),
        }],
        ..Card::new()
    };
    let mut safety: Vec<Card> = vec![labeled(
        "Production emergency contact".into(),
        &p.emergency_contact,
    )];
    for l in &d.locations {
        safety.push(labeled(
            format!("{} · Nearest hospital", l.name),
            &l.hospital,
        ));
    }
    let safety_cols = safety.len().clamp(1, 3);
    doc.push(Block::Cards(Cards::new(safety_cols, safety)));
    if !final_sheet {
        doc.push(Block::Callout(Callout {
            tone: Tone::Info,
            cols: vec![{
                let mut c = CalloutCol::new(1.0);
                c.lines.push(
                    Txt::new(
                        "Before issuing: confirm the crew call, all cast call times, the nearest \
                         hospital for each location and the production emergency contact. This is \
                         a draft reference, not a final call sheet.",
                    )
                    .size(8.6)
                    .color(Tone::Info.text())
                    .leading(1.35),
                );
                c
            }],
        }));
    }
    let mut practical: Vec<Field> = [
        ("Parking", &p.parking),
        ("Meeting point", &p.meeting_point),
        ("Transport", &p.travel_notes),
        ("Meals", &p.meal_break),
        ("Toilets", &p.toilets),
        ("Access", &p.access),
        ("Safety", &p.safety),
        ("Special instructions", &p.special_instructions),
        ("Production notes", &p.production_notes),
        ("Day notes", &d.day_notes),
    ]
    .into_iter()
    .filter_map(|(k, v)| nb(v).map(|v| Field::new(k, v)))
    .collect();
    for f in &d.extra_fields {
        if let (Some(k), Some(v)) = (nb(&f.label), nb(&f.value)) {
            practical.push(Field::new(k, v));
        }
    }
    if !practical.is_empty() {
        doc.push(Block::Section(Section::new("Practical notes", None)));
        doc.push(Block::Cards(Cards::new(
            1,
            vec![Card {
                parts: vec![CardPart::Fields(practical)],
                ..Card::new()
            }],
        )));
    }

    // ---- optional sections: only the ones the user added and filled
    let mut missing = 0;
    let mut unreadable = 0;
    if optional {
        let o = &d.optional;
        let one_card = |f: Vec<Field>| {
            Block::Cards(Cards::new(
                1,
                vec![Card {
                    parts: vec![CardPart::Fields(f)],
                    ..Card::new()
                }],
            ))
        };
        let fields_of = |pairs: &[(&str, &str)]| -> Vec<Field> {
            pairs
                .iter()
                .filter_map(|(k, v)| nb(v).map(|v| Field::new(*k, v)))
                .collect()
        };
        if o.weather.is_some() {
            let f = fields_of(&[
                ("Forecast", o.weather.as_deref().unwrap_or("")),
                ("Sunrise", o.sunrise.as_str()),
                ("Sunset", o.sunset.as_str()),
            ]);
            if !f.is_empty() {
                doc.push(Block::Section(Section::new("Weather", None)));
                doc.push(one_card(f));
            }
        }
        if let Some(list) = o.extras.as_ref().filter(|l| {
            l.iter()
                .any(|e| !e.description.trim().is_empty() || !e.quantity.trim().is_empty())
        }) {
            doc.push(Block::Section(Section::new("Extras", None)));
            let mut tbl = StyledTable::new(vec![
                Col::weight("Group", 1.6),
                Col::pt("Qty", 40.0).align(Align::Right),
                Col::weight("Call", 0.7),
                Col::weight("On set", 0.7),
                Col::weight("Notes", 1.5),
            ]);
            tbl.continued = Some("Extras".into());
            for e in list {
                tbl.rows.push(Row::new(vec![
                    cell_bold(e.description.clone()),
                    cell(e.quantity.clone()).align(Align::Right),
                    cell_or_missing(&e.call_time, false),
                    cell(e.on_set.clone()),
                    cell(e.notes.clone()),
                ]));
            }
            doc.push(Block::Styled(tbl));
        }
        if let Some(list) = o.departments.as_ref() {
            let cards: Vec<Card> = list
                .iter()
                .filter_map(|x| {
                    nb(&x.notes).map(|n| {
                        let lines: Vec<String> = n
                            .split('\n')
                            .flat_map(|l| l.split("; "))
                            .map(str::trim)
                            .filter(|l| !l.is_empty())
                            .map(str::to_string)
                            .collect();
                        Card {
                            head: Some(CardHead::dark(
                                nb(&x.department).unwrap_or("Other").to_uppercase(),
                            )),
                            parts: vec![CardPart::Lines(lines)],
                            ..Card::new()
                        }
                    })
                })
                .collect();
            if !cards.is_empty() {
                doc.push(Block::Section(Section::new(
                    "Department notes",
                    Some("From current production data".into()),
                )));
                doc.push(Block::Cards(Cards::new(2, cards)));
            }
        }
        if let Some(adv) = &o.advance {
            let f = fields_of(&[
                ("Day", &adv.day),
                ("Location", &adv.location),
                ("Cast", &adv.cast),
                ("Props", &adv.props),
                ("Wardrobe", &adv.wardrobe),
                ("Preparation", &adv.preparation),
            ]);
            if !f.is_empty() {
                doc.push(Block::Section(Section::new("Advance schedule", None)));
                doc.push(one_card(f));
            }
        }
        if let Some(list) = o.crew.as_ref().filter(|l| !l.is_empty()) {
            doc.push(Block::Section(Section::new(
                "Crew",
                Some(plural(
                    list.len(),
                    "crew member listed",
                    "crew members listed",
                )),
            )));
            let has_contact = list.iter().any(|c| !c.contact.trim().is_empty());
            let mut cols = vec![
                Col::weight("Department", 1.0),
                Col::weight("Role", 1.1),
                Col::weight("Name", 1.3),
                Col::weight("Call", 0.8),
            ];
            if has_contact {
                cols.push(Col::weight("Contact", 1.1));
            }
            let mut tbl = StyledTable::new(cols);
            tbl.continued = Some("Crew".into());
            let mut sorted: Vec<_> = list.iter().collect();
            sorted.sort_by(|a, b| {
                a.department
                    .to_lowercase()
                    .cmp(&b.department.to_lowercase())
            });
            let mut last = String::new();
            let mut group = 0usize;
            for c in sorted {
                let dep = c.department.trim().to_lowercase();
                let new_group = dep != last;
                if new_group {
                    group += 1;
                    last = dep;
                }
                let call = nonblank(Some(&c.call_time)).unwrap_or_else(|| need(&d.crew_call));
                let mut cells = vec![
                    if new_group {
                        cell_bold(c.department.clone())
                    } else {
                        Cell::empty()
                    },
                    cell(c.role.clone()),
                    cell(c.name.clone()),
                    cell_or_missing(if call == NEEDS_INPUT { "" } else { &call }, false),
                ];
                if has_contact {
                    cells.push(cell(c.contact.clone()));
                }
                let mut row = Row::new(cells);
                if group % 2 == 0 {
                    row.fill = Some(pal::PANEL);
                }
                tbl.rows.push(row);
            }
            doc.push(Block::Styled(tbl));
        }
        if let Some(cm) = &o.communication {
            let f = fields_of(&[
                ("Walkie channels", &cm.walkie_channels),
                ("Emergency contact", &cm.emergency_contact),
                ("Production contact", &cm.production_contact),
                ("Note", &cm.note),
            ]);
            if !f.is_empty() {
                doc.push(Block::Section(Section::new(
                    "Walkie channels and contacts",
                    None,
                )));
                doc.push(one_card(f));
            }
        }
        if let Some(s) = nonblank(o.special_notes.as_deref()) {
            doc.push(Block::Section(Section::new("Special notes", None)));
            doc.push(Block::Callout(Callout {
                tone: Tone::Warn,
                cols: vec![{
                    let mut c = CalloutCol::new(1.0);
                    c.lines
                        .push(Txt::new(s).size(9.0).color(Tone::Warn.text()).leading(1.4));
                    c
                }],
            }));
        }
        if let Some(list) = o.attachments.as_ref().filter(|l| !l.is_empty()) {
            doc.push(Block::Section(Section::new(
                "Attachments",
                Some("Sent separately".into()),
            )));
            doc.push(Block::Text(
                Txt::new(
                    list.iter()
                        .map(|x| x.name.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
                .size(8.8)
                .color(pal::INK)
                .after(6.0),
            ));
        }
        if !sheet.images.is_empty() {
            doc.push(Block::Section(Section::new("Reference images", None)));
            let cards = sheet
                .images
                .iter()
                .map(|(name, info)| {
                    let (img, placeholder) = match info.as_ref().map(load_image) {
                        Some(Ok(img)) => (Some(img), None),
                        Some(Err("unreadable")) | Some(Err("too large")) => {
                            unreadable += 1;
                            (None, Some("Image can't be printed".to_string()))
                        }
                        _ => {
                            missing += 1;
                            (None, Some("Image unavailable".to_string()))
                        }
                    };
                    Card {
                        parts: vec![
                            CardPart::Image {
                                img,
                                placeholder,
                                ratio: 0.66,
                            },
                            CardPart::Centered(vec![
                                Txt::new(name.clone())
                                    .size(8.2)
                                    .color(pal::SLATE)
                                    .align(Align::Center),
                            ]),
                        ],
                        ..Card::new()
                    }
                })
                .collect::<Vec<_>>();
            doc.push(Block::Cards(Cards::new(3, cards)));
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

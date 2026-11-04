//! Production: locations (and where a scene is shot), tasks, permits and call sheet status.
//! Reads `location`, `catalog_item`, `breakdown_element`, `task`, `call_sheet`.

use serde_json::Value;
use straft_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, SceneRef, safe};
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::queries;
use crate::modules::ai::turn::{AnswerType, TurnRead};
use crate::modules::production::availability::{self as av, AvailabilityOwner};

/// Production > Locations: each location with its status, address, notes, hospital and availability,
/// the places a scene is shot, and (for the roles that may see them) contact details.
pub fn locations_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    let ctx = &ctx.topic(&["locations"]);
    safe("locations", || fetch(ctx, read, r))
}

struct Loc {
    id: String,
    name: String,
    status: String,
    address: String,
    notes: String,
    hospital: String,
    contact: String,
    emergency: String,
}

fn locations(ctx: &FetchCtx<'_>) -> AppResult<Vec<Loc>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, name, status, COALESCE(address, ''), COALESCE(notes_json, '{}'), COALESCE(nearest_hospital, ''),
                COALESCE(contact, ''), COALESCE(emergency_contact, '')
         FROM location WHERE deleted_at IS NULL AND archived = 0 ORDER BY name LIMIT 300",
    )?;
    let rows = st
        .query_map([], |r| {
            Ok(Loc {
                id: r.get(0)?,
                name: r.get(1)?,
                status: r.get(2)?,
                address: r.get(3)?,
                notes: r.get(4)?,
                hospital: r.get(5)?,
                contact: r.get(6)?,
                emergency: r.get(7)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// The notes of a location as `kind: text; kind: text`.
fn notes_text(notes_json: &str) -> String {
    match serde_json::from_str::<Value>(notes_json) {
        Ok(Value::Object(o)) => o
            .iter()
            .filter_map(|(k, v)| {
                v.as_str()
                    .filter(|t| !t.trim().is_empty())
                    .map(|t| format!("{k}: {}", clip(t, 260)))
            })
            .collect::<Vec<_>>()
            .join("; "),
        _ => String::new(),
    }
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let mut out: Vec<Fact> = Vec::new();
    let all = locations(ctx)?;
    let scenes = ctx.focus_scenes(r);
    let asks_loc = m.stem("location") || !r.locations.is_empty();
    let asks_where = m.any(&[
        "where",
        "location",
        "locations",
        "shot",
        "shoot",
        "filmed",
        "set",
    ]);
    let asks_notes = m.any(&[
        "safety", "safe", "hospital", "issue", "issues", "rules", "permit", "permits", "parking",
        "access", "notes", "risk", "hazard",
    ]);
    let asks_contact = m.any(&["contact", "phone", "email", "reach", "emergency"]);
    let asks_avail = m.any(&["available", "availability", "free", "unavailable"]);

    // ---- where a scene is shot
    if !scenes.is_empty() && (asks_loc || asks_where) {
        for s in scenes.iter().take(4) {
            let (fact, ids) = scene_location(ctx, s, &all)?;
            out.extend(fact);
            if asks_notes || asks_contact {
                for l in all.iter().filter(|l| ids.contains(&l.id)) {
                    out.push(location_card(ctx, l, asks_contact, asks_avail, true)?);
                }
            }
        }
    }

    // ---- a named location
    for h in &r.locations {
        if let Some(l) = all.iter().find(|l| l.id == h.id) {
            out.push(location_card(ctx, l, asks_contact, asks_avail, true)?);
        }
    }
    if asks_contact && !ctx.guard.contacts && (!r.locations.is_empty() || m.stem("location")) {
        out.push(Fact::missing(
            "Contact details are not shown for your role.",
        ));
    }

    // ---- the locations as a whole
    if r.locations.is_empty() && scenes.is_empty() && m.stem("location") {
        if all.is_empty() {
            // "Locations used in the script" are the scene headings' places (the Screenplay
            // fetcher); an empty Production list says nothing about them.
            if !(m.stem("script") || m.stem("screenplay")) {
                out.push(Fact::absent("any locations", "Production"));
            }
        } else {
            out.push(
                Fact::new("Count", "")
                    .f("what", "locations")
                    .f("value", all.len().to_string())
                    .prio(P_HIGH)
                    .require(if t == AnswerType::Count && !status_asked(ctx) {
                        all.len().to_string()
                    } else {
                        String::new()
                    }),
            );
            let mut by_status: Vec<(String, usize)> = Vec::new();
            for l in &all {
                match by_status.iter_mut().find(|(s, _)| *s == l.status) {
                    Some(e) => e.1 += 1,
                    None => by_status.push((l.status.clone(), 1)),
                }
            }
            for (status, n) in &by_status {
                let asked = ctx.m.has(&status.to_lowercase());
                let mut f = Fact::new("Count", "")
                    .f("what", format!("locations with status {status}"))
                    .f("value", n.to_string())
                    .prio(P_HIGH);
                if t == AnswerType::Count && asked {
                    f = f.require(n.to_string());
                }
                out.push(f);
            }
            let wanted_status: Vec<&str> =
                ["idea", "shortlisted", "confirmed", "rejected", "backup"]
                    .into_iter()
                    .filter(|s| m.has(s))
                    .collect();
            for l in &all {
                let hit = wanted_status.iter().any(|s| l.status.to_lowercase() == *s);
                let mut f = Fact::new("Location", &l.id)
                    .f("name", &l.name)
                    .f("status", &l.status)
                    .prio(if hit { P_HIGH } else { P_MID });
                if hit && matches!(t, AnswerType::List | AnswerType::Lookup) {
                    f = f.require(&l.name);
                }
                out.push(f);
            }
        }
    }

    Ok(out)
}

fn status_asked(ctx: &FetchCtx<'_>) -> bool {
    ctx.m.any(&[
        "idea",
        "shortlisted",
        "confirmed",
        "rejected",
        "backup",
        "status",
    ])
}

fn location_card(
    ctx: &FetchCtx<'_>,
    l: &Loc,
    contact: bool,
    availability: bool,
    full: bool,
) -> AppResult<Fact> {
    let mut f = Fact::new("Location", &l.id)
        .f("name", &l.name)
        .f("status", &l.status)
        .prio(P_HIGH);
    if full {
        f = f
            .f("address", clip(&l.address, 160))
            .f("nearest hospital", clip(&l.hospital, 160))
            .f("notes", notes_text(&l.notes));
    }
    if availability {
        let w = av::windows_for(ctx.conn, AvailabilityOwner::Location, &l.id)?;
        if !w.is_empty() {
            f = f.f("availability", av::summary(&w));
        }
    }
    if contact && ctx.guard.contacts {
        f = f
            .f("contact", &l.contact)
            .f("emergency contact", &l.emergency);
        for v in [&l.contact, &l.emergency] {
            for part in v.split([';', ',']).map(str::trim).filter(|p| !p.is_empty()) {
                f = f.require(part);
            }
        }
    }
    Ok(f)
}

/// The places a scene is shot: its Location / Set breakdown elements, with the status of the
/// location each points to; else the place in the scene heading.
fn scene_location(
    ctx: &FetchCtx<'_>,
    s: &SceneRef,
    all: &[Loc],
) -> AppResult<(Option<Fact>, Vec<String>)> {
    let mut st = ctx.conn.prepare(
        "SELECT b.display_name, c.location_id FROM breakdown_element b
         LEFT JOIN catalog_item c ON c.id = b.catalog_item_id
         WHERE b.scene_id = ?1 AND b.category = 'Location / Set' AND b.deleted_at IS NULL AND b.archived = 0
           AND b.confirmation_state <> 'Rejected' ORDER BY b.display_name",
    )?;
    let rows: Vec<(String, Option<String>)> = st
        .query_map([&s.id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut ids = Vec::new();
    let mut parts = Vec::new();
    let mut f = Fact::new("Scene location", &s.id)
        .f("scene", s.number.to_string())
        .prio(P_HIGH);
    for (name, loc) in &rows {
        let status = loc
            .as_ref()
            .and_then(|id| all.iter().find(|l| l.id == *id))
            .map(|l| l.status.clone());
        if let Some(id) = loc {
            ids.push(id.clone());
        }
        parts.push(match status {
            Some(s) => format!("{name} ({s})"),
            None => name.clone(),
        });
        f = f.require(name);
    }
    if parts.is_empty() {
        let h = queries::parse_heading(&s.heading);
        if h.location.is_empty() {
            return Ok((None, ids));
        }
        f = f
            .f("location (from the heading)", &h.location)
            .require(&h.location);
    } else {
        f = f.f("locations", parts.join("; "));
    }
    Ok((Some(f), ids))
}

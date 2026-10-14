//! Availability windows for locations, cast and crew (Production depth contract section 3).
//!
//! A window is an inclusive range of calendar days marked Available or Unavailable, with an
//! optional note ("exams", "station closed for repairs"). Availability is advisory data the
//! schedule reads through [`availability_on`]; it never moves a scene by itself. The free-text
//! availability note on cast and crew stays for anything a date range cannot say.

use std::collections::HashMap;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::{Registry, hidden as h};
use crate::store::{MutationMeta, Tx};
use crate::util::optional_text;

openframe_domain::string_enum!(
    /// Whether a window says the owner can or cannot be used.
    AvailabilityKind {
        Available = "Available",
        Unavailable = "Unavailable",
    }
);

openframe_domain::string_enum!(
    /// What an availability window belongs to.
    AvailabilityOwner {
        Location = "location",
        Cast = "cast_member",
        Crew = "crew_member",
    }
);

/// Upper bound per owner, so one row can never grow without limit.
pub const MAX_WINDOWS: usize = 200;

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Production");
    r.command("availability.set", set).meta(
        M::edit("Replace the availability windows of a location, cast member or crew member.")
            .hidden(h::STRUCTURED_FORM),
    );
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityWindowDto {
    pub id: String,
    /// First day, "YYYY-MM-DD".
    pub start_date: String,
    /// Last day (inclusive), "YYYY-MM-DD".
    pub end_date: String,
    pub kind: AvailabilityKind,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AvailabilityWindowInput {
    pub start_date: String,
    pub end_date: String,
    pub kind: AvailabilityKind,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AvailabilitySetArgs {
    pub owner_type: AvailabilityOwner,
    pub owner_id: String,
    pub windows: Vec<AvailabilityWindowInput>,
}

/// The availability of an owner on one day, for schedule warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DayAvailability {
    /// No window covers the day.
    Unknown,
    Available,
    /// An Unavailable window covers the day (it wins over an overlapping Available one).
    Unavailable(Option<String>),
}

/// Parse a calendar day; rejects impossible dates such as 2027-02-30.
pub(crate) fn parse_day(value: &str, field: &str) -> AppResult<time::Date> {
    let bad = || {
        AppError::validation(
            field,
            format!("“{value}” is not a date. Use the format 2027-06-14."),
        )
    };
    let v = value.trim();
    let parts: Vec<&str> = v.split('-').collect();
    let digits = |p: &str, n: usize| p.len() == n && p.bytes().all(|b| b.is_ascii_digit());
    if parts.len() != 3 || !digits(parts[0], 4) || !digits(parts[1], 2) || !digits(parts[2], 2) {
        return Err(bad());
    }
    let y: i32 = parts[0].parse().map_err(|_| bad())?;
    let m: u8 = parts[1].parse().map_err(|_| bad())?;
    let d: u8 = parts[2].parse().map_err(|_| bad())?;
    let month = time::Month::try_from(m).map_err(|_| bad())?;
    time::Date::from_calendar_date(y, month, d).map_err(|_| bad())
}

fn owner_exists(c: &Connection, owner: AvailabilityOwner, id: &str) -> AppResult<Option<String>> {
    let sql = match owner {
        AvailabilityOwner::Location => {
            "SELECT name FROM location WHERE id = ?1 AND deleted_at IS NULL"
        }
        AvailabilityOwner::Cast => {
            "SELECT person_name FROM cast_member WHERE id = ?1 AND deleted_at IS NULL"
        }
        AvailabilityOwner::Crew => {
            "SELECT person_name FROM crew_member WHERE id = ?1 AND deleted_at IS NULL"
        }
    };
    Ok(c.query_row(sql, [id], |r| r.get(0)).optional()?)
}

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<(String, AvailabilityWindowDto)> {
    let kind: String = r.get(4)?;
    Ok((
        r.get(0)?,
        AvailabilityWindowDto {
            id: r.get(1)?,
            start_date: r.get(2)?,
            end_date: r.get(3)?,
            kind: AvailabilityKind::parse(&kind).unwrap_or(AvailabilityKind::Unavailable),
            note: r.get(5)?,
        },
    ))
}

/// Windows of every owner of one type, ordered by start day.
pub(crate) fn windows_by_owner(
    c: &Connection,
    owner: AvailabilityOwner,
) -> AppResult<HashMap<String, Vec<AvailabilityWindowDto>>> {
    let mut stmt = c.prepare(
        "SELECT owner_id, id, start_date, end_date, kind, note FROM availability_window
         WHERE owner_type = ?1 ORDER BY start_date, position, id",
    )?;
    let mut out: HashMap<String, Vec<AvailabilityWindowDto>> = HashMap::new();
    for r in stmt.query_map([owner.as_str()], row)? {
        let (owner_id, w) = r?;
        out.entry(owner_id).or_default().push(w);
    }
    Ok(out)
}

pub(crate) fn windows_for(
    c: &Connection,
    owner: AvailabilityOwner,
    owner_id: &str,
) -> AppResult<Vec<AvailabilityWindowDto>> {
    let mut stmt = c.prepare(
        "SELECT owner_id, id, start_date, end_date, kind, note FROM availability_window
         WHERE owner_type = ?1 AND owner_id = ?2 ORDER BY start_date, position, id",
    )?;
    let rows = stmt
        .query_map(params![owner.as_str(), owner_id], row)?
        .map(|r| r.map(|(_, w)| w))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Typed read model for the schedule: availability of an owner on a day ("YYYY-MM-DD").
pub fn availability_on(
    c: &Connection,
    owner: AvailabilityOwner,
    owner_id: &str,
    day: &str,
) -> AppResult<DayAvailability> {
    parse_day(day, "date")?;
    let day = day.trim();
    let mut stmt = c.prepare(
        "SELECT kind, note FROM availability_window
         WHERE owner_type = ?1 AND owner_id = ?2 AND start_date <= ?3 AND end_date >= ?3",
    )?;
    let hits: Vec<(String, Option<String>)> = stmt
        .query_map(params![owner.as_str(), owner_id, day], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?
        .collect::<Result<_, _>>()?;
    if let Some((_, note)) = hits.iter().find(|(k, _)| k == "Unavailable") {
        return Ok(DayAvailability::Unavailable(note.clone()));
    }
    Ok(if hits.is_empty() {
        DayAvailability::Unknown
    } else {
        DayAvailability::Available
    })
}

/// One Unavailable window, for the schedule's warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnavailableWindow {
    pub start_date: String,
    pub end_date: String,
    pub note: Option<String>,
}

/// A cast member the schedule can match to a character, with their Unavailable windows.
#[derive(Debug, Clone)]
pub struct CastUnavailability {
    pub character_id: Option<String>,
    pub character_name: Option<String>,
    pub windows: Vec<UnavailableWindow>,
}

/// Typed read model for the schedule: every Unavailable window of active cast members (primary
/// cast first, the order the schedule uses to pick the actor it shows for a character) and of
/// locations (keyed by location id). Available windows say nothing the schedule warns about,
/// and an Unavailable window wins over an overlapping Available one, so only Unavailable
/// windows are returned.
pub fn unavailable_for_schedule(
    c: &Connection,
) -> AppResult<(
    Vec<CastUnavailability>,
    HashMap<String, Vec<UnavailableWindow>>,
)> {
    let pick = |all: HashMap<String, Vec<AvailabilityWindowDto>>| {
        all.into_iter()
            .map(|(owner, ws)| {
                let ws: Vec<UnavailableWindow> = ws
                    .into_iter()
                    .filter(|w| w.kind == AvailabilityKind::Unavailable)
                    .map(|w| UnavailableWindow {
                        start_date: w.start_date,
                        end_date: w.end_date,
                        note: w.note,
                    })
                    .collect();
                (owner, ws)
            })
            .filter(|(_, ws)| !ws.is_empty())
            .collect::<HashMap<_, _>>()
    };
    let mut cast_windows = pick(windows_by_owner(c, AvailabilityOwner::Cast)?);
    let locations = pick(windows_by_owner(c, AvailabilityOwner::Location)?);
    let mut cast = Vec::new();
    if !cast_windows.is_empty() {
        let mut stmt = c.prepare(
            "SELECT id, character_id, character_name FROM cast_member
             WHERE deleted_at IS NULL AND archived = 0
             ORDER BY is_primary DESC, created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            let (id, character_id, character_name) = row?;
            cast.push(CastUnavailability {
                character_id,
                character_name,
                windows: cast_windows.remove(&id).unwrap_or_default(),
            });
        }
    }
    Ok((cast, locations))
}

/// Remove an owner's windows when the owner itself is purged from Recently Deleted.
pub(crate) fn purge_owner(tx: &Tx<'_>, owner: AvailabilityOwner, owner_id: &str) -> AppResult<()> {
    tx.conn().execute(
        "DELETE FROM availability_window WHERE owner_type = ?1 AND owner_id = ?2",
        params![owner.as_str(), owner_id],
    )?;
    Ok(())
}

/// Validated windows, sorted by start day.
pub(crate) fn clean(
    windows: Vec<AvailabilityWindowInput>,
) -> AppResult<Vec<AvailabilityWindowInput>> {
    if windows.len() > MAX_WINDOWS {
        return Err(AppError::invalid_input(format!(
            "Keep at most {MAX_WINDOWS} availability periods for one person or place."
        )));
    }
    let mut out = Vec::with_capacity(windows.len());
    for w in windows {
        let start = parse_day(&w.start_date, "startDate")?;
        let end = parse_day(&w.end_date, "endDate")?;
        if end < start {
            return Err(AppError::validation(
                "endDate",
                "The last day must be on or after the first day.",
            ));
        }
        out.push(AvailabilityWindowInput {
            start_date: w.start_date.trim().to_string(),
            end_date: w.end_date.trim().to_string(),
            kind: w.kind,
            note: optional_text(w.note, "Availability note", 500)?,
        });
    }
    out.sort_by(|a, b| a.start_date.cmp(&b.start_date));
    Ok(out)
}

fn set(
    core: &AppCore,
    actor: &Actor,
    args: AvailabilitySetArgs,
) -> AppResult<Vec<AvailabilityWindowDto>> {
    let windows = clean(args.windows)?;
    let s = core.project()?;
    let name = s
        .store
        .read(|c| owner_exists(c, args.owner_type, &args.owner_id))?
        .ok_or_else(|| AppError::not_found("person or location"))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "availability.set",
            format!("Updated availability of {name}"),
            Capability::Edit,
        )
        .target(args.owner_type.as_str(), &args.owner_id),
        |tx| {
            if owner_exists(tx.conn(), args.owner_type, &args.owner_id)?.is_none() {
                return Err(AppError::not_found("person or location"));
            }
            tx.conn().execute(
                "DELETE FROM availability_window WHERE owner_type = ?1 AND owner_id = ?2",
                params![args.owner_type.as_str(), args.owner_id],
            )?;
            let now = now_ms();
            for (i, w) in windows.iter().enumerate() {
                tx.conn().execute(
                    "INSERT INTO availability_window(id, owner_type, owner_id, start_date, end_date, kind, note, position, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                    params![
                        new_id(),
                        args.owner_type.as_str(),
                        args.owner_id,
                        w.start_date,
                        w.end_date,
                        w.kind.as_str(),
                        w.note,
                        i as i64,
                        now
                    ],
                )?;
            }
            // Touch the owner row so the change belongs to it: its search document (which
            // lists the windows) is rebuilt on save and again on undo and redo, and other
            // windows refresh the owner's record.
            let touch = match args.owner_type {
                AvailabilityOwner::Location => "UPDATE location SET updated_at = ?1 WHERE id = ?2",
                AvailabilityOwner::Cast => "UPDATE cast_member SET updated_at = ?1 WHERE id = ?2",
                AvailabilityOwner::Crew => "UPDATE crew_member SET updated_at = ?1 WHERE id = ?2",
            };
            tx.conn().execute(touch, params![now, args.owner_id])?;
            Ok(())
        },
    )?;
    s.store
        .read(|c| windows_for(c, args.owner_type, &args.owner_id))
}

/// "Unavailable 9 to 20 Jun 2027 (exams)" style one-liners for read tools and search.
pub(crate) fn summary(windows: &[AvailabilityWindowDto]) -> String {
    windows
        .iter()
        .map(|w| {
            let range = if w.start_date == w.end_date {
                w.start_date.clone()
            } else {
                format!("{} to {}", w.start_date, w.end_date)
            };
            match &w.note {
                Some(n) => format!("{} {range} ({n})", w.kind.as_str()),
                None => format!("{} {range}", w.kind.as_str()),
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_are_strict_calendar_dates() {
        assert!(parse_day("2027-06-14", "d").is_ok());
        assert!(parse_day("2028-02-29", "d").is_ok(), "leap day");
        for bad in [
            "2027-02-29",
            "2027-13-01",
            "27-06-14",
            "2027/06/14",
            "",
            "2027-6-1",
            "२०२७-०६-१४",
            "+027-06-14",
            "2027-+6-14",
        ] {
            assert!(parse_day(bad, "d").is_err(), "{bad}");
        }
    }

    #[test]
    fn clean_rejects_reversed_ranges_and_sorts() {
        let w = |a: &str, b: &str| AvailabilityWindowInput {
            start_date: a.into(),
            end_date: b.into(),
            kind: AvailabilityKind::Unavailable,
            note: None,
        };
        assert!(clean(vec![w("2027-06-10", "2027-06-09")]).is_err());
        let out = clean(vec![
            w("2027-07-01", "2027-07-02"),
            w("2027-06-01", "2027-06-01"),
        ])
        .unwrap();
        assert_eq!(out[0].start_date, "2027-06-01");
        assert!(
            clean(
                (0..=MAX_WINDOWS)
                    .map(|_| w("2027-06-01", "2027-06-01"))
                    .collect()
            )
            .is_err()
        );
    }
}

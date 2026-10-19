//! Cast and location availability as the schedule reads it (contract doc 33
//! section 3: "Availability is data the schedule reads; it is advisory").
//!
//! The records themselves (`availability_window`, migration 0014) belong to the
//! Breakdown, catalog, locations, cast and crew area. This module is the one place
//! the schedule reads them, through the typed read model
//! `production::availability::unavailable_for_schedule`, as dated "unavailable"
//! ranges keyed by the schedule's stable cast and location keys. Free-text
//! availability notes are not dates and never produce a warning: no data, no
//! warning, nothing invented.

use std::collections::HashMap;

use openframe_domain::AppResult;
use rusqlite::Connection;

/// One date range (inclusive, `YYYY-MM-DD`) when a person or place cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unavailable {
    pub from: String,
    pub to: String,
    pub note: Option<String>,
}

impl Unavailable {
    pub fn covers(&self, date: &str) -> bool {
        self.from.as_str() <= date && date <= self.to.as_str()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Availability {
    /// Cast key (`char:<id>` / `name:<NAME>`) to unavailable ranges.
    pub cast: HashMap<String, Vec<Unavailable>>,
    /// Location key (`loc:<id>` and so on) to unavailable ranges.
    pub locations: HashMap<String, Vec<Unavailable>>,
}

impl Availability {
    pub fn is_empty(&self) -> bool {
        self.cast.is_empty() && self.locations.is_empty()
    }
    pub fn cast_conflict(&self, key: &str, date: &str) -> Option<&Unavailable> {
        self.cast.get(key)?.iter().find(|u| u.covers(date))
    }
    pub fn location_conflict(&self, key: &str, date: &str) -> Option<&Unavailable> {
        self.locations.get(key)?.iter().find(|u| u.covers(date))
    }
}

/// Dated Unavailable windows for the open project, keyed the way the schedule names
/// cast (`char:<character id>`, or `name:<CHARACTER NAME>` for a cast member linked
/// by name only) and locations (`loc:<location id>`). When several actors play one
/// character, the windows of the one the schedule shows (primary cast first) count.
pub fn load(c: &Connection) -> AppResult<Availability> {
    let (cast, locations) = crate::modules::production::availability::unavailable_for_schedule(c)?;
    let ranges = |ws: Vec<crate::modules::production::availability::UnavailableWindow>| {
        ws.into_iter()
            .map(|w| Unavailable {
                from: w.start_date,
                to: w.end_date,
                note: w.note,
            })
            .collect::<Vec<_>>()
    };
    let mut out = Availability::default();
    for member in cast {
        let key = match (&member.character_id, &member.character_name) {
            (Some(id), _) => format!("char:{id}"),
            (None, Some(name)) if !name.trim().is_empty() => {
                format!("name:{}", name.trim().to_uppercase())
            }
            _ => continue,
        };
        // The first cast member for a key is the one the schedule shows; later ones
        // (other actors for the same character) do not override it.
        if out.cast.contains_key(&key) {
            continue;
        }
        let ws = ranges(member.windows);
        out.cast.insert(key, ws);
    }
    out.cast.retain(|_, ws| !ws.is_empty());
    for (id, ws) in locations {
        out.locations.insert(format!("loc:{id}"), ranges(ws));
    }
    Ok(out)
}

/// What one shooting day needs, for the availability rule.
pub struct DayNeeds<'a> {
    pub day_id: &'a str,
    pub day_label: String,
    pub date: Option<&'a str>,
    /// (cast key, display name, strip ids needing them).
    pub cast: Vec<(String, String, Vec<String>)>,
    /// (location key, display name, strip ids needing it).
    pub locations: Vec<(String, String, Vec<String>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailabilityIssue {
    /// "cast_unavailable" | "location_unavailable".
    pub kind: &'static str,
    pub key: String,
    pub day_id: String,
    pub message: String,
    pub detail: Option<String>,
    pub strip_ids: Vec<String>,
}

fn range_text(u: &Unavailable) -> String {
    let span = if u.from == u.to {
        u.from.clone()
    } else {
        format!("{} to {}", u.from, u.to)
    };
    match u.note.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => format!("Unavailable {span}: {n}"),
        None => format!("Unavailable {span}"),
    }
}

/// Advisory warnings for cast and locations needed on a day they are marked
/// unavailable. Undated days and missing data produce nothing.
pub fn issues(avail: &Availability, days: &[DayNeeds<'_>]) -> Vec<AvailabilityIssue> {
    let mut out = Vec::new();
    if avail.is_empty() {
        return out;
    }
    for d in days {
        let Some(date) = d.date else { continue };
        for (key, name, strips) in &d.cast {
            if let Some(u) = avail.cast_conflict(key, date) {
                out.push(AvailabilityIssue {
                    kind: "cast_unavailable",
                    key: format!("cast_unavailable:{}:{key}", d.day_id),
                    day_id: d.day_id.to_string(),
                    message: format!("{name} is marked unavailable on {}.", d.day_label),
                    detail: Some(range_text(u)),
                    strip_ids: strips.clone(),
                });
            }
        }
        for (key, name, strips) in &d.locations {
            if let Some(u) = avail.location_conflict(key, date) {
                out.push(AvailabilityIssue {
                    kind: "location_unavailable",
                    key: format!("location_unavailable:{}:{key}", d.day_id),
                    day_id: d.day_id.to_string(),
                    message: format!("{name} is marked unavailable on {}.", d.day_label),
                    detail: Some(range_text(u)),
                    strip_ids: strips.clone(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn needs<'a>(date: Option<&'a str>) -> DayNeeds<'a> {
        DayNeeds {
            day_id: "d1",
            day_label: "Shoot Day 1".into(),
            date,
            cast: vec![(
                "char:arjun".into(),
                "Karthik Menon (Arjun)".into(),
                vec!["s1".into()],
            )],
            locations: vec![(
                "loc:rail".into(),
                "Old Railway Station".into(),
                vec!["s2".into()],
            )],
        }
    }

    #[test]
    fn no_dated_availability_means_no_warning() {
        let a = Availability::default();
        assert!(issues(&a, &[needs(Some("2027-06-14"))]).is_empty());
    }

    #[test]
    fn unavailable_cast_and_location_warn_on_dated_days_only() {
        let mut a = Availability::default();
        a.cast.insert(
            "char:arjun".into(),
            vec![Unavailable {
                from: "2027-06-13".into(),
                to: "2027-06-15".into(),
                note: Some("Another shoot".into()),
            }],
        );
        a.locations.insert(
            "loc:rail".into(),
            vec![Unavailable {
                from: "2027-06-14".into(),
                to: "2027-06-14".into(),
                note: None,
            }],
        );
        let got = issues(&a, &[needs(Some("2027-06-14"))]);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].kind, "cast_unavailable");
        assert_eq!(
            got[0].message,
            "Karthik Menon (Arjun) is marked unavailable on Shoot Day 1."
        );
        assert_eq!(
            got[0].detail.as_deref(),
            Some("Unavailable 2027-06-13 to 2027-06-15: Another shoot")
        );
        assert_eq!(got[1].kind, "location_unavailable");
        assert_eq!(got[1].strip_ids, vec!["s2".to_string()]);
        assert!(issues(&a, &[needs(Some("2027-06-16"))]).is_empty());
        assert!(
            issues(&a, &[needs(None)]).is_empty(),
            "undated days never warn"
        );
    }
}

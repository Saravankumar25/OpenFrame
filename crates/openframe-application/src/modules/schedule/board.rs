//! In-memory view of one shooting schedule: days, strips, markers and the
//! Production Source data they derive from. Everything shown on the stripboard,
//! the daily view, warnings and call-sheet prefill is computed from here, so
//! all derived values have exactly one definition.

use std::collections::{BTreeMap, HashMap, HashSet};

use openframe_domain::{AppError, AppResult};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::fmt::{date_short, fnv_hex, minutes, pages};
use super::source::{
    self, SceneInfo, ScheduleCastRef, ScheduleItemRef, ScheduleLocRef, ScheduleSourceInfo,
    SourceData,
};

// ------------------------------------------------------------------ rows

#[derive(Debug, Clone)]
pub struct ScheduleRow {
    pub id: String,
    pub source_id: String,
    pub name: String,
    pub status: String,
    pub strict: bool,
    pub day_minutes: i64,
    pub rev: i64,
}

const SCHEDULE_COLS: &str =
    "id, source_id, name, status, strict_validation, day_duration_minutes, rev";

fn schedule_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ScheduleRow> {
    Ok(ScheduleRow {
        id: r.get(0)?,
        source_id: r.get(1)?,
        name: r.get(2)?,
        status: r.get(3)?,
        strict: r.get::<_, i64>(4)? != 0,
        day_minutes: r.get(5)?,
        rev: r.get(6)?,
    })
}

/// The project's current schedule (the most recently created non-deleted one).
pub fn current_schedule(c: &Connection) -> AppResult<Option<ScheduleRow>> {
    Ok(c.query_row(
        &format!(
            "SELECT {SCHEDULE_COLS} FROM shooting_schedule WHERE deleted_at IS NULL ORDER BY created_at DESC, id DESC LIMIT 1"
        ),
        [],
        schedule_from_row,
    )
    .optional()?)
}

pub fn load_schedule(c: &Connection, id: &str) -> AppResult<ScheduleRow> {
    c.query_row(
        &format!(
            "SELECT {SCHEDULE_COLS} FROM shooting_schedule WHERE id = ?1 AND deleted_at IS NULL"
        ),
        [id],
        schedule_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("shooting schedule"))
}

#[derive(Debug, Clone)]
pub struct DayRow {
    pub id: String,
    pub schedule_id: String,
    pub position: i64,
    pub date: Option<String>,
    pub notes: Option<String>,
    pub planned_minutes: Option<i64>,
    pub off: bool,
    pub rev: i64,
    /// Shoot day number (off days are not numbered).
    pub number: Option<i64>,
}

impl DayRow {
    /// "Shoot Day 4" / "Off Day".
    pub fn label(&self) -> String {
        match self.number {
            Some(n) => format!("Shoot Day {n}"),
            None => "Off Day".to_string(),
        }
    }
    /// "Shoot Day 4 — Mon 14 Jun 2027".
    pub fn title(&self) -> String {
        match &self.date {
            Some(d) => format!("{} — {}", self.label(), date_short(d)),
            None => self.label(),
        }
    }
}

/// Days of a schedule in order, numbered (deleted days excluded).
pub fn load_days(c: &Connection, schedule_id: &str) -> AppResult<Vec<DayRow>> {
    let mut st = c.prepare(
        "SELECT id, schedule_id, position, shoot_date, notes, planned_minutes, is_off_day, rev
         FROM shooting_day WHERE schedule_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let rows = st.query_map([schedule_id], |r| {
        Ok(DayRow {
            id: r.get(0)?,
            schedule_id: r.get(1)?,
            position: r.get(2)?,
            date: r.get(3)?,
            notes: r.get(4)?,
            planned_minutes: r.get(5)?,
            off: r.get::<_, i64>(6)? != 0,
            rev: r.get(7)?,
            number: None,
        })
    })?;
    let mut days: Vec<DayRow> = rows.collect::<Result<_, _>>()?;
    let mut n = 0;
    for d in days.iter_mut() {
        if !d.off {
            n += 1;
            d.number = Some(n);
        }
    }
    Ok(days)
}

/// A day row regardless of its deleted state (for restore/purge/labels).
pub fn day_schedule_id(c: &Connection, day_id: &str) -> AppResult<Option<String>> {
    Ok(c.query_row(
        "SELECT schedule_id FROM shooting_day WHERE id = ?1",
        [day_id],
        |r| r.get(0),
    )
    .optional()?)
}

#[derive(Debug, Clone)]
pub struct StripRow {
    pub id: String,
    pub schedule_id: String,
    pub day_id: Option<String>,
    pub scene_id: String,
    pub lineage_id: String,
    pub position: i64,
    pub estimated_minutes: Option<i64>,
    pub pages_override: Option<i64>,
    pub source_heading: String,
    pub source_text_hash: String,
    pub source_page_eighths: i64,
    pub source_state: String,
    pub change_kinds: Vec<String>,
    pub archived: bool,
    pub notes: Option<String>,
    pub rev: i64,
}

pub const STRIP_COLS: &str = "id, schedule_id, day_id, scene_id, scene_lineage_id, position, estimated_minutes,
    page_eighths_override, source_heading, source_text_hash, source_page_eighths, source_state, change_kinds,
    archived, notes, rev";

pub fn strip_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<StripRow> {
    let kinds: String = r.get(12)?;
    Ok(StripRow {
        id: r.get(0)?,
        schedule_id: r.get(1)?,
        day_id: r.get(2)?,
        scene_id: r.get(3)?,
        lineage_id: r.get(4)?,
        position: r.get(5)?,
        estimated_minutes: r.get(6)?,
        pages_override: r.get(7)?,
        source_heading: r.get(8)?,
        source_text_hash: r.get(9)?,
        source_page_eighths: r.get(10)?,
        source_state: r.get(11)?,
        change_kinds: serde_json::from_str(&kinds).unwrap_or_default(),
        archived: r.get::<_, i64>(13)? != 0,
        notes: r.get(14)?,
        rev: r.get(15)?,
    })
}

pub fn load_strip(c: &Connection, id: &str) -> AppResult<StripRow> {
    c.query_row(
        &format!("SELECT {STRIP_COLS} FROM schedule_strip WHERE id = ?1 AND deleted_at IS NULL"),
        [id],
        strip_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("scene strip"))
}

pub fn load_strips(
    c: &Connection,
    schedule_id: &str,
    include_archived: bool,
) -> AppResult<Vec<StripRow>> {
    let mut st = c.prepare(&format!(
        "SELECT {STRIP_COLS} FROM schedule_strip WHERE schedule_id = ?1 AND deleted_at IS NULL {} ORDER BY position, id",
        if include_archived { "" } else { "AND archived = 0" }
    ))?;
    let rows = st.query_map([schedule_id], strip_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[derive(Debug, Clone)]
pub struct MarkerRow {
    pub id: String,
    pub day_id: String,
    pub marker_type: String,
    pub label: String,
    pub at_time: Option<String>,
    pub duration_minutes: Option<i64>,
    pub notes: Option<String>,
    pub position: i64,
    pub rev: i64,
}

pub const MARKER_COLS: &str = "m.id, m.day_id, m.marker_type, m.label, m.at_time, m.duration_minutes, m.notes, m.position, m.rev";

pub fn marker_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<MarkerRow> {
    Ok(MarkerRow {
        id: r.get(0)?,
        day_id: r.get(1)?,
        marker_type: r.get(2)?,
        label: r.get(3)?,
        at_time: r.get(4)?,
        duration_minutes: r.get(5)?,
        notes: r.get(6)?,
        position: r.get(7)?,
        rev: r.get(8)?,
    })
}

pub fn load_markers(c: &Connection, schedule_id: &str) -> AppResult<Vec<MarkerRow>> {
    let mut st = c.prepare(&format!(
        "SELECT {MARKER_COLS} FROM schedule_marker m JOIN shooting_day d ON d.id = m.day_id
         WHERE d.schedule_id = ?1 AND d.deleted_at IS NULL AND m.deleted_at IS NULL ORDER BY m.position, m.id"
    ))?;
    let rows = st.query_map([schedule_id], marker_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[derive(Debug, Clone)]
pub struct CallSheetRow {
    pub id: String,
    pub day_id: String,
    pub revision: i64,
    pub status: String,
    pub fingerprint: String,
    pub needs_refresh: bool,
}

/// Current (non-superseded, non-deleted) call sheets of a schedule.
pub fn load_call_sheets(c: &Connection, schedule_id: &str) -> AppResult<Vec<CallSheetRow>> {
    let mut st = c.prepare(
        "SELECT id, shoot_day_id, revision, status, source_fingerprint, needs_refresh FROM call_sheet
         WHERE schedule_id = ?1 AND deleted_at IS NULL AND status <> 'Superseded' ORDER BY revision DESC, created_at DESC",
    )?;
    let rows = st.query_map([schedule_id], |r| {
        Ok(CallSheetRow {
            id: r.get(0)?,
            day_id: r.get(1)?,
            revision: r.get(2)?,
            status: r.get(3)?,
            fingerprint: r.get(4)?,
            needs_refresh: r.get::<_, i64>(5)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Lifecycle status shown to the user: editable sheets whose source day changed
/// read as "Needs Refresh" (FSD §38.7); final/issued stay historical.
pub fn effective_status(stored: &str, source_changed: bool) -> String {
    match stored {
        "Draft" | "Ready" | "Needs Refresh" if source_changed => "Needs Refresh".to_string(),
        "Needs Refresh" => "Draft".to_string(),
        s => s.to_string(),
    }
}

pub fn project_title(c: &Connection) -> AppResult<String> {
    Ok(c.query_row(
        "SELECT title FROM project ORDER BY created_at LIMIT 1",
        [],
        |r| r.get(0),
    )
    .optional()?
    .unwrap_or_default())
}

// ------------------------------------------------------------------ board

pub struct Board {
    pub schedule: ScheduleRow,
    pub source: Option<SourceData>,
    pub days: Vec<DayRow>,
    pub strips: Vec<StripRow>,
    pub markers: Vec<MarkerRow>,
    pub call_sheets: Vec<CallSheetRow>,
    pub project_title: String,
    day_index: HashMap<String, usize>,
}

pub enum Item<'a> {
    Strip(&'a StripRow),
    Marker(&'a MarkerRow),
}

impl Item<'_> {
    pub fn id(&self) -> &str {
        match self {
            Item::Strip(s) => &s.id,
            Item::Marker(m) => &m.id,
        }
    }
    pub fn is_strip(&self) -> bool {
        matches!(self, Item::Strip(_))
    }
}

impl Board {
    pub fn load(c: &Connection, schedule: ScheduleRow) -> AppResult<Board> {
        let source = source::load_source(c, &schedule.source_id)?;
        let days = load_days(c, &schedule.id)?;
        let strips = load_strips(c, &schedule.id, false)?;
        let markers = load_markers(c, &schedule.id)?;
        let call_sheets = load_call_sheets(c, &schedule.id)?;
        let day_index = days
            .iter()
            .enumerate()
            .map(|(i, d)| (d.id.clone(), i))
            .collect();
        Ok(Board {
            project_title: project_title(c)?,
            schedule,
            source,
            days,
            strips,
            markers,
            call_sheets,
            day_index,
        })
    }

    pub fn day(&self, id: &str) -> Option<&DayRow> {
        self.day_index.get(id).map(|i| &self.days[*i])
    }

    /// The day a strip is effectively on (None = Unscheduled, including strips of deleted days).
    pub fn strip_day<'a>(&self, s: &'a StripRow) -> Option<&'a str> {
        s.day_id
            .as_deref()
            .filter(|d| self.day_index.contains_key(*d))
    }

    pub fn day_items(&self, day_id: &str) -> Vec<Item<'_>> {
        let mut items: Vec<(i64, u8, &str, Item<'_>)> = Vec::new();
        for s in &self.strips {
            if s.day_id.as_deref() == Some(day_id) {
                items.push((s.position, 0, &s.id, Item::Strip(s)));
            }
        }
        for m in &self.markers {
            if m.day_id == day_id {
                items.push((m.position, 1, &m.id, Item::Marker(m)));
            }
        }
        items.sort_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));
        items.into_iter().map(|t| t.3).collect()
    }

    pub fn day_strips(&self, day_id: &str) -> Vec<&StripRow> {
        self.day_items(day_id)
            .into_iter()
            .filter_map(|i| match i {
                Item::Strip(s) => Some(s),
                Item::Marker(_) => None,
            })
            .collect()
    }

    pub fn unscheduled(&self) -> Vec<&StripRow> {
        let mut v: Vec<&StripRow> = self
            .strips
            .iter()
            .filter(|s| self.strip_day(s).is_none())
            .collect();
        v.sort_by_key(|s| {
            (
                self.scene(s).map(|sc| sc.number).unwrap_or(i64::MAX),
                s.id.clone(),
            )
        });
        v
    }

    pub fn scene(&self, s: &StripRow) -> Option<&SceneInfo> {
        self.source.as_ref().and_then(|src| src.scene(&s.scene_id))
    }

    pub fn cast(&self, s: &StripRow) -> Vec<ScheduleCastRef> {
        self.source
            .as_ref()
            .map(|src| src.cast(&s.scene_id).to_vec())
            .unwrap_or_default()
    }

    pub fn locations(&self, s: &StripRow) -> Vec<ScheduleLocRef> {
        match (self.source.as_ref(), self.scene(s)) {
            (Some(src), Some(sc)) => src.locations(sc),
            _ => Vec::new(),
        }
    }

    pub fn items(&self, s: &StripRow) -> Vec<ScheduleItemRef> {
        self.source
            .as_ref()
            .map(|src| src.items(&s.scene_id).to_vec())
            .unwrap_or_default()
    }

    pub fn page_eighths(&self, s: &StripRow) -> i64 {
        s.pages_override
            .or_else(|| self.scene(s).map(|sc| sc.page_eighths))
            .unwrap_or(s.source_page_eighths)
    }

    pub fn scene_number(&self, s: &StripRow) -> Option<i64> {
        self.scene(s).map(|sc| sc.number)
    }

    pub fn scene_label(&self, s: &StripRow) -> String {
        match self.scene_number(s) {
            Some(n) => format!("Scene {n}"),
            None => {
                if s.source_heading.is_empty() {
                    "Scene".to_string()
                } else {
                    format!("Scene “{}”", s.source_heading)
                }
            }
        }
    }

    pub fn target_minutes(&self, d: &DayRow) -> i64 {
        d.planned_minutes.unwrap_or(self.schedule.day_minutes)
    }

    pub fn day_summary(&self, d: &DayRow) -> ScheduleShootDaySummary {
        let strips = self.day_strips(&d.id);
        let mut est = 0;
        let mut missing = 0;
        let mut eighths = 0;
        let mut cast: Vec<ScheduleCastRef> = Vec::new();
        let mut locs: Vec<ScheduleLocRef> = Vec::new();
        for s in &strips {
            match s.estimated_minutes {
                Some(m) => est += m,
                None => missing += 1,
            }
            eighths += self.page_eighths(s);
            for c in self.cast(s) {
                if !cast.iter().any(|x| x.key == c.key) {
                    cast.push(c);
                }
            }
            for l in self.locations(s) {
                if !locs.iter().any(|x| x.key == l.key) {
                    locs.push(l);
                }
            }
        }
        for m in self.markers.iter().filter(|m| m.day_id == d.id) {
            est += m.duration_minutes.unwrap_or(0);
        }
        let target = self.target_minutes(d);
        let n = strips.len() as i64;
        let mut total_label = format!("{n} scene{}", if n == 1 { "" } else { "s" });
        if est > 0 {
            total_label.push_str(&format!(" · {}", minutes(est)));
        }
        ScheduleShootDaySummary {
            scene_count: n,
            estimated_minutes: est,
            missing_estimates: missing,
            page_eighths: eighths,
            pages_label: pages(eighths),
            target_minutes: target,
            over_target: est > target,
            total_label,
            cast,
            locations: locs,
        }
    }

    /// The day data a call sheet is prefilled from, and compared against for staleness.
    pub fn call_sheet_source(&self, d: &DayRow) -> CallSheetSource {
        let strips = self.day_strips(&d.id);
        let mut scenes = Vec::new();
        let mut cast: Vec<CallSheetSourceCast> = Vec::new();
        let mut locations: Vec<CallSheetSourceLocation> = Vec::new();
        for s in &strips {
            let sc = self.scene(s);
            scenes.push(CallSheetSourceScene {
                key: s.lineage_id.clone(),
                number: sc.map(|x| x.number.to_string()).unwrap_or_default(),
                heading: sc
                    .map(|x| x.heading.clone())
                    .unwrap_or_else(|| s.source_heading.clone()),
                description: sc.map(|x| x.synopsis.clone()).unwrap_or_default(),
                pages: pages(self.page_eighths(s)),
            });
            for c in self.cast(s) {
                if !cast.iter().any(|x| x.key == c.key) {
                    cast.push(CallSheetSourceCast {
                        key: c.key,
                        character: c.character.to_uppercase(),
                        actor: c.actor,
                    });
                }
            }
            for l in self.locations(s) {
                if !locations.iter().any(|x| x.key == l.key) {
                    locations.push(CallSheetSourceLocation {
                        key: l.key,
                        name: l.name,
                        address: l.address,
                    });
                }
            }
        }
        let breaks = self
            .markers
            .iter()
            .filter(|m| m.day_id == d.id)
            .map(|m| CallSheetSourceBreak {
                marker_type: m.marker_type.clone(),
                label: m.label.clone(),
                at_time: m.at_time.clone(),
            })
            .collect();
        CallSheetSource {
            production_title: self.project_title.clone(),
            source_label: self
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
        }
    }

    pub fn call_sheet_for_day(&self, day_id: &str) -> Option<&CallSheetRow> {
        self.call_sheets.iter().find(|c| c.day_id == day_id)
    }

    pub fn call_sheet_brief(&self, d: &DayRow) -> Option<CallSheetBrief> {
        self.call_sheet_for_day(&d.id).map(|cs| {
            let changed =
                cs.needs_refresh || cs.fingerprint != self.call_sheet_source(d).fingerprint();
            CallSheetBrief {
                id: cs.id.clone(),
                revision: cs.revision,
                status: effective_status(&cs.status, changed),
                source_changed: changed,
            }
        })
    }

    // -------------------------------------------------------------- DTOs

    pub fn strip_dto(&self, s: &StripRow) -> ScheduleStripDto {
        let sc = self.scene(s);
        let int_ext = sc.map(|x| x.int_ext.clone()).unwrap_or_default();
        let day_night = sc.map(|x| x.day_night.clone()).unwrap_or_default();
        let ie_short = match int_ext.as_str() {
            "INT/EXT" => "I/E",
            "" => "—",
            other => other,
        };
        let ie_label = if day_night.is_empty() {
            ie_short.to_string()
        } else {
            format!("{ie_short}·{day_night}")
        };
        let strip_class = format!(
            "{}{}",
            if int_ext == "EXT" || int_ext == "INT/EXT" {
                "x"
            } else {
                "i"
            },
            if day_night == "N" { "n" } else { "d" }
        );
        let locations = self.locations(s);
        let eighths = self.page_eighths(s);
        ScheduleStripDto {
            id: s.id.clone(),
            scene_id: s.scene_id.clone(),
            lineage_id: s.lineage_id.clone(),
            number: sc.map(|x| x.number),
            heading: sc
                .map(|x| x.heading.clone())
                .unwrap_or_else(|| s.source_heading.clone()),
            int_ext,
            day_night,
            ie_label,
            strip_class,
            location_name: locations.first().map(|l| l.name.clone()),
            locations,
            synopsis: sc.map(|x| x.synopsis.clone()).unwrap_or_default(),
            cast: self.cast(s),
            page_eighths: eighths,
            pages_label: pages(eighths),
            pages_overridden: s.pages_override.is_some(),
            estimated_minutes: s.estimated_minutes,
            day_id: self.strip_day(s).map(|d| d.to_string()),
            source_state: s.source_state.clone(),
            change_kinds: s.change_kinds.clone(),
            missing_from_source: sc.is_none(),
            notes: s.notes.clone(),
            rev: s.rev,
        }
    }

    pub fn marker_dto(&self, m: &MarkerRow) -> ScheduleMarkerDto {
        ScheduleMarkerDto {
            id: m.id.clone(),
            day_id: m.day_id.clone(),
            marker_type: m.marker_type.clone(),
            label: m.label.clone(),
            at_time: m.at_time.clone(),
            duration_minutes: m.duration_minutes,
            notes: m.notes.clone(),
            rev: m.rev,
        }
    }

    pub fn day_dto(&self, d: &DayRow, warnings: &[ScheduleWarningDto]) -> ScheduleShootDayDto {
        let items = self
            .day_items(&d.id)
            .into_iter()
            .map(|i| match i {
                Item::Strip(s) => ScheduleShootDayItemDto {
                    kind: "strip".into(),
                    strip: Some(self.strip_dto(s)),
                    marker: None,
                },
                Item::Marker(m) => ScheduleShootDayItemDto {
                    kind: "marker".into(),
                    strip: None,
                    marker: Some(self.marker_dto(m)),
                },
            })
            .collect();
        ScheduleShootDayDto {
            id: d.id.clone(),
            number: d.number,
            label: d.label(),
            title: d.title(),
            date: d.date.clone(),
            date_label: d.date.as_deref().map(date_short),
            notes: d.notes.clone(),
            planned_minutes: d.planned_minutes,
            is_off_day: d.off,
            items,
            summary: self.day_summary(d),
            call_sheet: self.call_sheet_brief(d),
            open_warnings: warnings
                .iter()
                .filter(|w| w.day_id.as_deref() == Some(d.id.as_str()) && w.decision.is_none())
                .count() as i64,
            rev: d.rev,
        }
    }

    pub fn schedule_dto(&self) -> ScheduleDto {
        ScheduleDto {
            id: self.schedule.id.clone(),
            name: self.schedule.name.clone(),
            status: self.schedule.status.clone(),
            strict_validation: self.schedule.strict,
            day_duration_minutes: self.schedule.day_minutes,
            source: self.source.as_ref().map(|s| s.info.clone()),
            rev: self.schedule.rev,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn make_warning(
        &self,
        acks: &HashMap<String, (String, String)>,
        key: String,
        kind: &str,
        day: Option<&DayRow>,
        message: String,
        detail: Option<String>,
        strips: &[&StripRow],
    ) -> ScheduleWarningDto {
        let hash = fnv_hex(&format!(
            "{message}\u{1}{}",
            detail.clone().unwrap_or_default()
        ));
        let decision = acks
            .get(&key)
            .filter(|(h, _)| *h == hash)
            .map(|(_, d)| d.clone());
        ScheduleWarningDto {
            blocking: self.schedule.strict && decision.is_none() && super::ops::blocking_kind(kind),
            key,
            kind: kind.to_string(),
            day_id: day.map(|d| d.id.clone()),
            day_label: day.map(|d| d.title()),
            message,
            detail,
            strip_ids: strips.iter().map(|s| s.id.clone()).collect(),
            scene_numbers: strips.iter().filter_map(|s| self.scene_number(s)).collect(),
            detail_hash: hash,
            decision,
        }
    }

    // ---------------------------------------------------------- warnings

    /// Advisory warnings (FSD §36.4, §146). Never blocking unless strict validation is on.
    pub fn warnings(&self, acks: &HashMap<String, (String, String)>) -> Vec<ScheduleWarningDto> {
        let mut out: Vec<ScheduleWarningDto> = Vec::new();
        let mut push = |key: String,
                        kind: &str,
                        day: Option<&DayRow>,
                        message: String,
                        detail: Option<String>,
                        strips: Vec<&StripRow>| {
            out.push(self.make_warning(acks, key, kind, day, message, detail, &strips));
        };

        for d in self.days.iter().filter(|d| !d.off) {
            let strips = self.day_strips(&d.id);
            // Same actor needed at different locations on the same day.
            let mut by_cast: BTreeMap<String, (String, Vec<&StripRow>, Vec<(String, String)>)> =
                BTreeMap::new();
            for s in &strips {
                let Some(loc) = self.locations(s).into_iter().next() else {
                    continue;
                };
                for c in self.cast(s) {
                    let who = match &c.actor {
                        Some(a) => format!("{a} ({})", c.character),
                        None => c.character.clone(),
                    };
                    let e = by_cast
                        .entry(c.key.clone())
                        .or_insert_with(|| (who, Vec::new(), Vec::new()));
                    e.1.push(s);
                    if !e.2.iter().any(|(k, _)| *k == loc.key) {
                        e.2.push((loc.key.clone(), loc.name.clone()));
                    }
                }
            }
            for (key, (who, ss, locs)) in by_cast {
                if locs.len() < 2 {
                    continue;
                }
                let count = if locs.len() == 2 {
                    "two".to_string()
                } else {
                    locs.len().to_string()
                };
                let names: Vec<String> = locs.iter().map(|(_, n)| n.clone()).collect();
                let scenes: Vec<String> = ss.iter().map(|s| self.scene_label(s)).collect();
                push(
                    format!("actor:{}:{key}", d.id),
                    "actor_conflict",
                    Some(d),
                    format!(
                        "Potential actor conflict: {who} is needed at {count} locations on {}.",
                        d.label()
                    ),
                    Some(format!("{} — {}", scenes.join(", "), names.join(" / "))),
                    ss,
                );
            }

            // Scenes missing location information.
            let missing: Vec<&StripRow> = strips
                .iter()
                .copied()
                .filter(|s| {
                    let locs = self.locations(s);
                    locs.is_empty()
                        || locs
                            .iter()
                            .all(|l| !l.from_breakdown || l.address.is_none())
                })
                .collect();
            if !missing.is_empty() {
                let scenes: Vec<String> = missing.iter().map(|s| self.scene_label(s)).collect();
                push(
                    format!("missing_location:{}", d.id),
                    "missing_location",
                    Some(d),
                    format!(
                        "{} on {} {} missing location information.",
                        scenes.join(", "),
                        d.label(),
                        if missing.len() == 1 { "is" } else { "are" }
                    ),
                    Some("Confirm a location with an address in Breakdown or Locations. Missing values are never invented.".into()),
                    missing,
                );
            }

            // Estimated time exceeds the day's target.
            let sum = self.day_summary(d);
            if sum.over_target {
                push(
                    format!("overflow:{}", d.id),
                    "duration_overflow",
                    Some(d),
                    format!(
                        "Estimated day duration exceeds the target on {} ({} of {}).",
                        d.label(),
                        minutes(sum.estimated_minutes),
                        minutes(sum.target_minutes)
                    ),
                    None,
                    strips.clone(),
                );
            }

            // Call sheet prepared from an older version of this day.
            if let Some(cs) = self.call_sheet_brief(d)
                && cs.source_changed
            {
                push(
                    format!("callsheet:{}", cs.id),
                    "call_sheet_stale",
                    Some(d),
                    format!(
                        "The call sheet for {} was prepared before this day changed. Refresh it to review the update.",
                        d.label()
                    ),
                    Some(self.call_sheet_source(d).fingerprint()),
                    Vec::new(),
                );
            }

            // Scenes removed from the script but still scheduled.
            for s in strips.iter().filter(|s| s.source_state == "Removed") {
                push(
                    format!("removed:{}", s.id),
                    "script_removed",
                    Some(d),
                    format!(
                        "{} was removed from the script but is still scheduled on {}. Confirm the removal to take it off the schedule.",
                        self.scene_label(s),
                        d.label()
                    ),
                    None,
                    vec![s],
                );
            }
        }

        // The same location needed by two shooting days on the same date.
        let mut by_date: BTreeMap<&str, Vec<&DayRow>> = BTreeMap::new();
        for d in self.days.iter().filter(|d| !d.off) {
            if let Some(date) = d.date.as_deref() {
                by_date.entry(date).or_default().push(d);
            }
        }
        for (date, days) in by_date {
            if days.len() < 2 {
                continue;
            }
            let mut by_loc: BTreeMap<String, (String, Vec<&DayRow>, Vec<&StripRow>)> =
                BTreeMap::new();
            for d in &days {
                for s in self.day_strips(&d.id) {
                    for l in self.locations(s) {
                        let e = by_loc
                            .entry(l.key.clone())
                            .or_insert_with(|| (l.name.clone(), Vec::new(), Vec::new()));
                        if !e.1.iter().any(|x| x.id == d.id) {
                            e.1.push(d);
                        }
                        e.2.push(s);
                    }
                }
            }
            for (key, (name, ds, ss)) in by_loc {
                if ds.len() < 2 {
                    continue;
                }
                let labels: Vec<String> = ds.iter().map(|d| d.label()).collect();
                push(
                    format!("location:{date}:{key}"),
                    "location_conflict",
                    Some(ds[0]),
                    format!(
                        "Potential location conflict: {name} is needed by {} on the same date ({}).",
                        labels.join(" and "),
                        date_short(date)
                    ),
                    None,
                    ss,
                );
            }
        }
        out
    }
}

// ------------------------------------------------------------------ call sheet source

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
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceCast {
    pub key: String,
    pub character: String,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceLocation {
    pub key: String,
    pub name: String,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSourceBreak {
    pub marker_type: String,
    pub label: String,
    pub at_time: Option<String>,
}

/// Snapshot of the shooting day a call sheet was generated from.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetSource {
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
}

impl CallSheetSource {
    pub fn fingerprint(&self) -> String {
        fnv_hex(&serde_json::to_string(self).unwrap_or_default())
    }
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleStripDto {
    pub id: String,
    pub scene_id: String,
    pub lineage_id: String,
    #[ts(type = "number | null")]
    pub number: Option<i64>,
    pub heading: String,
    pub int_ext: String,
    pub day_night: String,
    /// e.g. "EXT·N".
    pub ie_label: String,
    /// Stripboard colour class: xd / xn / id / in.
    pub strip_class: String,
    pub location_name: Option<String>,
    pub locations: Vec<ScheduleLocRef>,
    pub synopsis: String,
    pub cast: Vec<ScheduleCastRef>,
    #[ts(type = "number")]
    pub page_eighths: i64,
    pub pages_label: String,
    pub pages_overridden: bool,
    #[ts(type = "number | null")]
    pub estimated_minutes: Option<i64>,
    pub day_id: Option<String>,
    /// Current / Changed / Removed / New (script reconciliation, FSD §56).
    pub source_state: String,
    pub change_kinds: Vec<String>,
    pub missing_from_source: bool,
    pub notes: Option<String>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleMarkerDto {
    pub id: String,
    pub day_id: String,
    /// Meal / Travel / Company Move / Custom.
    pub marker_type: String,
    pub label: String,
    pub at_time: Option<String>,
    #[ts(type = "number | null")]
    pub duration_minutes: Option<i64>,
    pub notes: Option<String>,
    #[ts(type = "number")]
    pub rev: i64,
}

/// One ordered item of a day: a scene strip or a break marker.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleShootDayItemDto {
    /// "strip" | "marker".
    pub kind: String,
    pub strip: Option<ScheduleStripDto>,
    pub marker: Option<ScheduleMarkerDto>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleShootDaySummary {
    #[ts(type = "number")]
    pub scene_count: i64,
    #[ts(type = "number")]
    pub estimated_minutes: i64,
    /// Scenes without an entered estimate (never invented).
    #[ts(type = "number")]
    pub missing_estimates: i64,
    #[ts(type = "number")]
    pub page_eighths: i64,
    pub pages_label: String,
    #[ts(type = "number")]
    pub target_minutes: i64,
    pub over_target: bool,
    /// "3 scenes · 4h 30m".
    pub total_label: String,
    pub cast: Vec<ScheduleCastRef>,
    pub locations: Vec<ScheduleLocRef>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CallSheetBrief {
    pub id: String,
    #[ts(type = "number")]
    pub revision: i64,
    /// Draft / Needs Refresh / Ready / Final / Issued.
    pub status: String,
    pub source_changed: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleShootDayDto {
    pub id: String,
    #[ts(type = "number | null")]
    pub number: Option<i64>,
    /// "Shoot Day 4" or "Off Day".
    pub label: String,
    /// "Shoot Day 4 — Mon 14 Jun 2027".
    pub title: String,
    pub date: Option<String>,
    pub date_label: Option<String>,
    pub notes: Option<String>,
    #[ts(type = "number | null")]
    pub planned_minutes: Option<i64>,
    pub is_off_day: bool,
    pub items: Vec<ScheduleShootDayItemDto>,
    pub summary: ScheduleShootDaySummary,
    pub call_sheet: Option<CallSheetBrief>,
    #[ts(type = "number")]
    pub open_warnings: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDto {
    pub id: String,
    pub name: String,
    pub status: String,
    pub strict_validation: bool,
    #[ts(type = "number")]
    pub day_duration_minutes: i64,
    pub source: Option<ScheduleSourceInfo>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleWarningDto {
    /// Stable key used for Keep Anyway / Dismiss decisions.
    pub key: String,
    /// actor_conflict / location_conflict / missing_location / duration_overflow /
    /// call_sheet_stale / script_removed.
    pub kind: String,
    pub day_id: Option<String>,
    pub day_label: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    pub strip_ids: Vec<String>,
    #[ts(type = "number[]")]
    pub scene_numbers: Vec<i64>,
    pub detail_hash: String,
    /// "Kept" / "Dismissed" when the user already decided.
    pub decision: Option<String>,
    /// True only when strict validation is enabled and nobody decided yet.
    pub blocking: bool,
}

pub fn load_acks(
    c: &Connection,
    schedule_id: &str,
) -> AppResult<HashMap<String, (String, String)>> {
    let mut st =
        c.prepare("SELECT warning_key, detail_hash, decision FROM schedule_warning_ack WHERE schedule_id = ?1")?;
    let rows = st.query_map(params![schedule_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            (r.get::<_, String>(1)?, r.get::<_, String>(2)?),
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Keys of warnings nobody has decided on yet.
pub fn open_warning_keys(
    board: &Board,
    acks: &HashMap<String, (String, String)>,
) -> HashSet<String> {
    board
        .warnings(acks)
        .into_iter()
        .filter(|w| w.decision.is_none())
        .map(|w| w.key)
        .collect()
}

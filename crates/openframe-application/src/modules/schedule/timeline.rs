//! Day timeline (Screenplay and Production Depth spec section 26.1, contract
//! doc 33 section 4): crew call, shooting start, each scene and break in
//! position order with its duration, the estimated wrap, and the total against
//! the target.
//!
//! Everything here is derived and pure. Stored times are clock minutes after
//! midnight (0 to 1439); a time earlier than the day's crew call (or shooting
//! start) is read as after midnight, so a night shoot can wrap the next morning.
//! The timeline is an estimate: it never moves a scene and never invents a
//! duration (a scene without an estimate counts as zero and is flagged).

use serde::Serialize;
use ts_rs::TS;

use super::fmt::{clock, minutes};

/// Times the user entered for one shooting day (clock minutes, 0 to 1439).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DayTimes {
    pub crew_call: Option<i64>,
    pub shooting_start: Option<i64>,
    pub target_wrap: Option<i64>,
}

/// One scene or break of the day, in position order.
#[derive(Debug, Clone)]
pub struct TimelineInput {
    pub id: String,
    /// "strip" or "marker".
    pub kind: &'static str,
    pub label: String,
    /// Meal / Travel / Company Move / Custom (markers only).
    pub marker_type: Option<String>,
    /// Entered duration; None for a scene without an estimate.
    pub duration: Option<i64>,
    /// Start time the user pinned (clock minutes), if any.
    pub pinned: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleTimelineRow {
    /// "crew_call" | "shooting_start" | "strip" | "marker" | "wrap".
    pub kind: String,
    pub item_id: Option<String>,
    pub label: String,
    pub marker_type: Option<String>,
    /// Minutes after midnight of the shoot date (can pass 1440 after midnight).
    #[ts(type = "number | null")]
    pub start: Option<i64>,
    #[ts(type = "number | null")]
    pub end: Option<i64>,
    /// "07:00" or "01:30 (+1)".
    pub start_label: Option<String>,
    #[ts(type = "number | null")]
    pub duration_minutes: Option<i64>,
    /// The start was entered by the user rather than computed.
    pub pinned: bool,
    /// A scene without a time estimate (counted as zero).
    pub missing_estimate: bool,
    /// The pinned start is earlier than the end of the item before it.
    #[ts(type = "number")]
    pub overlap_minutes: i64,
    /// Idle time between the end of the item before it and this pinned start.
    #[ts(type = "number")]
    pub gap_minutes: i64,
}

#[derive(Debug, Clone, Serialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDayTimeline {
    pub rows: Vec<ScheduleTimelineRow>,
    #[ts(type = "number | null")]
    pub crew_call: Option<i64>,
    #[ts(type = "number | null")]
    pub shooting_start: Option<i64>,
    /// Normalised (after midnight when earlier than the crew call).
    #[ts(type = "number | null")]
    pub target_wrap: Option<i64>,
    #[ts(type = "number | null")]
    pub estimated_wrap: Option<i64>,
    pub crew_call_label: Option<String>,
    pub shooting_start_label: Option<String>,
    pub target_wrap_label: Option<String>,
    pub estimated_wrap_label: Option<String>,
    /// Sum of scene estimates and break durations.
    #[ts(type = "number")]
    pub working_minutes: i64,
    /// Travel and company move durations included in `working_minutes`.
    #[ts(type = "number")]
    pub travel_minutes: i64,
    /// Crew call (or shooting start) to estimated wrap, when both are known.
    #[ts(type = "number | null")]
    pub span_minutes: Option<i64>,
    /// The estimated wrap is later than the target wrap.
    pub over_target_wrap: bool,
    /// The day has a crew call or shooting start, so clock times can be shown.
    pub anchored: bool,
    /// "07:00 crew call · wrap about 18:30 (target 19:00)".
    pub summary: String,
}

/// Read a clock time as after midnight when it is earlier than `anchor`.
pub fn after(anchor: Option<i64>, t: i64) -> i64 {
    match anchor {
        Some(a) if t < a % 1440 && t < 1440 => t + 1440,
        Some(a) if a >= 1440 && t < 1440 => t + 1440,
        _ => t,
    }
}

pub fn compute(times: DayTimes, items: &[TimelineInput]) -> ScheduleDayTimeline {
    let crew = times.crew_call;
    let start = times.shooting_start.map(|s| after(crew, s));
    let day_anchor = start.or(crew);
    let wrap_target = times.target_wrap.map(|w| after(crew.or(start), w));
    let mut rows = Vec::new();
    let marker = |kind: &str, label: &str, at: i64| ScheduleTimelineRow {
        kind: kind.into(),
        item_id: None,
        label: label.into(),
        marker_type: None,
        start: Some(at),
        end: None,
        start_label: Some(clock(at)),
        duration_minutes: None,
        pinned: true,
        missing_estimate: false,
        overlap_minutes: 0,
        gap_minutes: 0,
    };
    if let Some(c) = crew {
        rows.push(marker("crew_call", "Crew call", c));
    }
    if let Some(s) = start {
        rows.push(marker("shooting_start", "Shooting starts", s));
    }
    let mut cursor = day_anchor;
    let mut working = 0;
    let mut travel = 0;
    for it in items {
        let pinned = it.pinned.map(|p| after(day_anchor, p));
        let (overlap, gap) = match (pinned, cursor) {
            (Some(p), Some(c)) if p < c => (c - p, 0),
            (Some(p), Some(c)) => (0, p - c),
            _ => (0, 0),
        };
        let at = pinned.or(cursor);
        let dur = it.duration.unwrap_or(0).max(0);
        working += dur;
        if matches!(it.marker_type.as_deref(), Some("Travel" | "Company Move")) {
            travel += dur;
        }
        let end = at.map(|a| a + dur);
        rows.push(ScheduleTimelineRow {
            kind: it.kind.into(),
            item_id: Some(it.id.clone()),
            label: it.label.clone(),
            marker_type: it.marker_type.clone(),
            start: at,
            end,
            start_label: at.map(clock),
            duration_minutes: it.duration,
            pinned: pinned.is_some(),
            missing_estimate: it.kind == "strip" && it.duration.is_none(),
            overlap_minutes: overlap,
            gap_minutes: gap,
        });
        cursor = end;
    }
    let estimated_wrap = if items.is_empty() { None } else { cursor };
    if let Some(w) = estimated_wrap {
        rows.push(marker("wrap", "Estimated wrap", w));
    }
    let over_target_wrap = matches!((estimated_wrap, wrap_target), (Some(e), Some(t)) if e > t);
    let span = match (crew.or(start), estimated_wrap) {
        (Some(a), Some(w)) => Some(w - a),
        _ => None,
    };
    let mut parts = Vec::new();
    if let Some(c) = crew {
        parts.push(format!("{} crew call", clock(c)));
    }
    match (estimated_wrap, wrap_target) {
        (Some(e), Some(t)) => parts.push(format!("wrap about {} (target {})", clock(e), clock(t))),
        (Some(e), None) => parts.push(format!("wrap about {}", clock(e))),
        (None, Some(t)) => parts.push(format!("target wrap {}", clock(t))),
        (None, None) => {}
    }
    if parts.is_empty() && working > 0 {
        parts.push(format!("{} of scenes and breaks", minutes(working)));
    }
    ScheduleDayTimeline {
        rows,
        crew_call: crew,
        shooting_start: start,
        target_wrap: wrap_target,
        estimated_wrap,
        crew_call_label: crew.map(clock),
        shooting_start_label: start.map(clock),
        target_wrap_label: wrap_target.map(clock),
        estimated_wrap_label: estimated_wrap.map(clock),
        working_minutes: working,
        travel_minutes: travel,
        span_minutes: span,
        over_target_wrap,
        anchored: day_anchor.is_some(),
        summary: parts.join(" · "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(id: &str, dur: Option<i64>, pinned: Option<i64>) -> TimelineInput {
        TimelineInput {
            id: id.into(),
            kind: "strip",
            label: format!("Scene {id}"),
            marker_type: None,
            duration: dur,
            pinned,
        }
    }

    fn brk(id: &str, t: &str, dur: i64, pinned: Option<i64>) -> TimelineInput {
        TimelineInput {
            id: id.into(),
            kind: "marker",
            label: t.into(),
            marker_type: Some(t.into()),
            duration: Some(dur),
            pinned,
        }
    }

    /// The worked example of spec section 26.1, row for row.
    #[test]
    fn spec_example_day_reads_like_a_call_sheet() {
        let t = compute(
            DayTimes {
                crew_call: Some(7 * 60),
                shooting_start: Some(8 * 60),
                target_wrap: Some(19 * 60),
            },
            &[
                strip("1", Some(60), None),
                strip("3", Some(90), None),
                brk("b", "Custom", 30, None),
                strip("9", Some(60), None),
                brk("m", "Company Move", 30, None),
                strip("2", Some(120), None),
                brk("l", "Meal", 60, None),
                strip("11", Some(60), None),
                strip("6", Some(120), None),
            ],
        );
        let starts: Vec<String> = t
            .rows
            .iter()
            .filter_map(|r| r.start_label.clone())
            .collect();
        assert_eq!(
            starts,
            [
                "07:00", "08:00", "08:00", "09:00", "10:30", "11:00", "12:00", "12:30", "14:30",
                "15:30", "16:30", "18:30"
            ]
        );
        assert_eq!(t.estimated_wrap_label.as_deref(), Some("18:30"));
        assert_eq!(t.working_minutes, 630);
        assert_eq!(t.travel_minutes, 30);
        assert_eq!(t.span_minutes, Some(690));
        assert!(!t.over_target_wrap);
        assert_eq!(
            t.summary,
            "07:00 crew call · wrap about 18:30 (target 19:00)"
        );
    }

    #[test]
    fn pinned_starts_show_gaps_and_overlaps_and_reanchor_the_day() {
        let t = compute(
            DayTimes {
                crew_call: Some(6 * 60),
                ..Default::default()
            },
            &[
                strip("1", Some(60), None),
                strip("2", Some(30), Some(8 * 60)),
                strip("3", Some(30), Some(8 * 60 + 15)),
            ],
        );
        let r: Vec<&ScheduleTimelineRow> = t.rows.iter().filter(|r| r.kind == "strip").collect();
        assert_eq!((r[1].gap_minutes, r[1].pinned), (60, true));
        assert_eq!(r[2].overlap_minutes, 15);
        assert_eq!(t.estimated_wrap, Some(8 * 60 + 45));
    }

    #[test]
    fn night_shoot_wraps_after_midnight_and_reports_over_target() {
        let t = compute(
            DayTimes {
                crew_call: Some(18 * 60),
                shooting_start: Some(19 * 60),
                target_wrap: Some(4 * 60),
            },
            &[strip("1", Some(300), None), strip("2", Some(240), None)],
        );
        assert_eq!(t.target_wrap, Some(28 * 60));
        assert_eq!(t.estimated_wrap, Some(28 * 60));
        assert_eq!(t.estimated_wrap_label.as_deref(), Some("04:00 (+1)"));
        assert!(!t.over_target_wrap);
        let t = compute(
            DayTimes {
                crew_call: Some(18 * 60),
                shooting_start: Some(19 * 60),
                target_wrap: Some(4 * 60),
            },
            &[strip("1", Some(300), None), strip("2", Some(241), None)],
        );
        assert!(t.over_target_wrap);
    }

    #[test]
    fn without_times_the_day_still_totals_and_never_invents_estimates() {
        let t = compute(
            DayTimes::default(),
            &[strip("1", None, None), brk("t", "Travel", 45, None)],
        );
        assert!(!t.anchored);
        assert_eq!(t.estimated_wrap, None);
        assert!(t.rows.iter().all(|r| r.start.is_none()));
        assert!(t.rows[0].missing_estimate);
        assert_eq!(t.working_minutes, 45);
        assert_eq!(t.summary, "45m of scenes and breaks");
        let empty = compute(
            DayTimes {
                crew_call: Some(420),
                ..Default::default()
            },
            &[],
        );
        assert_eq!(empty.estimated_wrap, None);
        assert_eq!(empty.rows.len(), 1);
    }
}

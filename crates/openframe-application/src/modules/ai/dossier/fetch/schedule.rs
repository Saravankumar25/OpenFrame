//! Schedule: the shooting days, which scenes are on which day, call times, schedule warnings and
//! call sheets. Reads `shooting_schedule`, `shooting_day`, `schedule_strip`, `call_sheet` through
//! `schedule::board` (the same days, numbering and warnings the Schedule screen shows).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use openframe_domain::AppResult;
use regex::Regex;

use crate::modules::ai::dossier::asked;
use crate::modules::ai::dossier::ctx::{FetchCtx, SceneRef, safe};
use crate::modules::ai::dossier::words::{Msg, clip, clock, numbers};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::queries;
use crate::modules::ai::turn::{AnswerType, TurnRead};
use crate::modules::schedule::board::{self, Board, DayRow};
use crate::modules::schedule::callsheet::CallSheetDocument;

static DAY_N: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bday\s+#?(\d{1,3})\b").expect("valid pattern"));
static WORD_DAY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b([a-z]{3,})\s+day\b").expect("valid pattern"));

/// Words before "day" that name no scene ("shoot day", "which day").
const NOT_A_PLACE: &[&str] = &[
    "shoot", "shooting", "which", "what", "that", "this", "each", "every", "same", "next", "last",
    "first", "off", "the", "per", "one", "any", "all", "whole", "full", "long", "half", "per",
    "wrap", "pickup", "pick", "rest", "our", "your",
];

/// Words that do not narrow a question about where and when a character appears.
const WHERE_WORDS: &[&str] = &[
    "day", "days", "date", "dates", "scene", "scenes", "shoot", "shooting", "shot", "schedule",
    "scheduled", "when", "work", "working", "works", "appear", "appears", "last", "first", "final",
    "plays", "play", "free",
];

/// "What is on Day 4", "what was planned", "which scenes are on it": the question asks what a named
/// day holds, not one of its times.
static ASKS_THE_DAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:what(?:s| is| are| was| were)? (?:on|planned|scheduled|happening|shooting)|scheduled|planned|(?:which|what) scenes?|scenes? (?:are|is|on)|how many scenes)\b")
        .expect("valid pattern")
});

/// Status words of a call sheet a question can filter by.
const SHEET_STATUS: &[(&str, &[&str])] = &[
    ("final", &["final", "finals", "finalised", "finalized"]),
    ("draft", &["draft", "drafts"]),
    ("stale", &["stale", "out of date", "outdated"]),
];

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("schedule", || fetch(ctx, read, r))
}

/// The scenes of the days a message names, in running order.
pub fn scenes_of_days(ctx: &FetchCtx<'_>, want: &[u32]) -> AppResult<Vec<SceneRef>> {
    if want.is_empty() {
        return Ok(Vec::new());
    }
    let strips = strips(ctx)?;
    let mut out = Vec::new();
    for d in ctx.days().iter() {
        if d.number.is_none_or(|n| !want.contains(&(n as u32))) {
            continue;
        }
        for id in strips.get(&d.id).into_iter().flatten() {
            out.extend(ctx.scene_by_id(id));
        }
    }
    Ok(out)
}

/// The Call Sheets section (ADR-0041): all of them, the ones of a chosen day, or the chosen sheet.
pub fn call_sheets_section(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("call sheets", || {
        let sheet = ctx
            .picked
            .and_then(|p| p.call_sheet.as_ref())
            .map(|s| s.id.as_str());
        let day = ctx.picked.and_then(|p| p.day.as_ref());
        let mut want = named_days(ctx, r);
        // A question about a call or a time with no day takes the day the conversation was on.
        let times = Msg::new(&ctx.message).without(&["call sheet", "call sheets"]);
        if want.is_empty()
            && day.is_none()
            && times.any(&["call", "calls", "earliest", "latest", "time", "times"])
            && let Some(previous) = &ctx.previous
        {
            want = DAY_N
                .captures_iter(previous)
                .filter_map(|c| c[1].parse().ok())
                .collect();
        }
        let narrowed = sheet.is_some() || day.is_some() || !want.is_empty();
        let mut facts = call_sheets(ctx, read, &want, narrowed)?;
        if let Some(id) = sheet {
            facts.retain(|f| f.record_id == id);
        }
        if let Some(day) = day.filter(|d| d.number.is_none()) {
            let on_that_day = sheets_of_day(ctx, &day.id)?;
            facts.retain(|f| on_that_day.contains(&f.record_id));
        }
        Ok(facts)
    })
}

/// The current call sheets of one day.
fn sheets_of_day(ctx: &FetchCtx<'_>, day_id: &str) -> AppResult<Vec<String>> {
    let mut st = ctx.conn.prepare(
        "SELECT id FROM call_sheet
         WHERE shoot_day_id = ?1 AND deleted_at IS NULL AND superseded_by IS NULL",
    )?;
    Ok(st
        .query_map([day_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?)
}

/// The shooting days a message names: the resolved ones and "day 4". A day the person picked replaces
/// them: "Day 4" in the message does not widen a pick of Day 3. An off day has no number, so a pick of
/// one names no day here and [`call_sheets_section`] holds it by its id.
pub fn named_days(ctx: &FetchCtx<'_>, r: &Resolved) -> Vec<u32> {
    if let Some(day) = ctx.picked.and_then(|p| p.day.as_ref()) {
        return day.number.into_iter().collect();
    }
    let mut out: Vec<u32> = r
        .days
        .iter()
        .filter_map(|h| ctx.index.entity(&h.id).and_then(|e| e.ordinal))
        .collect();
    for c in DAY_N.captures_iter(&ctx.message) {
        if let Ok(n) = c[1].parse::<u32>()
            && !out.contains(&n)
        {
            out.push(n);
        }
    }
    out.sort_unstable();
    out
}

type Strips = BTreeMap<String, Vec<String>>;

/// Scene ids by day id, in running order.
fn strips(ctx: &FetchCtx<'_>) -> AppResult<Strips> {
    let mut st = ctx.conn.prepare(
        "SELECT day_id, scene_id FROM schedule_strip
         WHERE day_id IS NOT NULL AND deleted_at IS NULL AND archived = 0 ORDER BY day_id, position",
    )?;
    let rows: Vec<(String, String)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut out = Strips::new();
    for (d, s) in rows {
        out.entry(d).or_default().push(s);
    }
    Ok(out)
}

fn day_label(d: &DayRow) -> String {
    match d.number {
        Some(n) => format!("Day {n}"),
        None => "Off day".into(),
    }
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let mut out: Vec<Fact> = Vec::new();
    let days = ctx.days();
    let worth_a_gap = m.any(&[
        "schedule",
        "scheduled",
        "shoot",
        "shooting",
        "shot",
        "day",
        "days",
        "when",
        "date",
        "dates",
        "call",
        "calendar",
    ]);
    let scenes = ctx.focus_scenes(r);
    if days.is_empty() {
        if worth_a_gap || !scenes.is_empty() {
            out.push(Fact::absent(
                "a shooting schedule or any shoot days",
                "the Schedule",
            ));
        }
        if m.any(&["call sheet", "call sheets", "sides"]) {
            out.extend(call_sheets(ctx, read, &named_days(ctx, r), false)?);
        }
        return Ok(out);
    }
    let shoot_days: Vec<&DayRow> = days.iter().filter(|d| !d.off).collect();
    let strips = strips(ctx)?;

    // ---- how many days, from when to when
    let asks_days = m.stem("day") || m.has("shoot") || m.has("shooting");
    let asks_period =
        m.any(&["dates", "period", "start", "end", "begin", "from", "range"]) && asks_days;
    let specific = !named_days(ctx, r).is_empty()
        || !scenes.is_empty()
        || !r.characters.is_empty()
        || m.stem("warning")
        || m.any(&[
            "call sheet",
            "call sheets",
            "sides",
            "clash",
            "clashes",
            "conflict",
            "conflicts",
        ]);
    if (asks_days && !specific) || asks_period {
        out.push(
            Fact::new("Count", "")
                .f("what", "shoot days")
                .f("value", shoot_days.len().to_string())
                .prio(P_HIGH)
                .require(if t == AnswerType::Count {
                    shoot_days.len().to_string()
                } else {
                    String::new()
                }),
        );
        let first = shoot_days.first().and_then(|d| d.date.clone());
        let last = shoot_days.last().and_then(|d| d.date.clone());
        if let (Some(a), Some(b)) = (first, last) {
            out.push(
                Fact::new("Shoot period", "")
                    .f("from", &a)
                    .f("to", &b)
                    .prio(P_HIGH)
                    .require(if asks_period { a } else { String::new() })
                    .require(if asks_period { b } else { String::new() }),
            );
        }
    }

    // ---- scenes with no day
    if m.any(&["unscheduled", "not scheduled", "unassigned", "no day"]) {
        out.extend(unscheduled(ctx, t)?);
    }

    // ---- the days to show
    let mut target: BTreeSet<u32> = named_days(ctx, r).into_iter().collect();
    let by_scene = |scene_id: &str| -> Option<&DayRow> {
        strips
            .iter()
            .find(|(_, v)| v.iter().any(|s| s == scene_id))
            .and_then(|(d, _)| days.iter().find(|x| x.id == *d))
    };
    let asks_when = m.any(&[
        "when",
        "which day",
        "what day",
        "day",
        "days",
        "date",
        "shoot",
        "shot",
        "schedule",
        "scheduled",
        "free",
    ]);
    for s in &scenes {
        if let Some(d) = by_scene(&s.id) {
            if let Some(n) = d.number {
                target.insert(n as u32);
            }
            out.push(
                Fact::new("Scene day", &s.id)
                    .f("scene", s.number.to_string())
                    .f("day", day_label(d))
                    .f("date", d.date.clone().unwrap_or_default())
                    .prio(P_HIGH)
                    .require(if asks_when {
                        day_label(d)
                    } else {
                        String::new()
                    }),
            );
        } else if asks_when {
            out.push(Fact::absent(
                format!("a shooting day for scene {}", s.number),
                "the Schedule",
            ));
        }
    }
    // A character's scenes on the days: "which day do we shoot Young Anjali's scenes".
    if !r.characters.is_empty() && asks_when {
        for h in &r.characters {
            let name = ctx
                .index
                .entity(&h.id)
                .map_or(h.name.clone(), |e| e.name.clone());
            let Some(sp) = ctx.screenplay(r) else {
                continue;
            };
            let Some(draft) = ctx.draft(&sp.id) else {
                continue;
            };
            let presence = ctx.presence(&draft.id);
            let Some(nums) = presence.by_name.get(&name) else {
                continue;
            };
            let ids: Vec<String> = queries::scenes(ctx.conn, &draft.id)?
                .into_iter()
                .filter(|s| nums.contains(&(s.number as u32)))
                .map(|s| s.id)
                .collect();
            let numbers_by_id = ctx.scene_numbers(&ids);
            let mut per_day: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
            for id in &ids {
                if let (Some(d), Some((n, _))) = (by_scene(id), numbers_by_id.get(id))
                    && let Some(dn) = d.number
                {
                    per_day.entry(dn as u32).or_default().push(*n);
                }
            }
            if per_day.is_empty() {
                continue;
            }
            let mut f = Fact::new("Character days", &h.id)
                .f("character", &name)
                .f(
                    "days",
                    per_day
                        .iter()
                        .map(|(d, ns)| format!("Day {d} (scene {})", numbers(ns.iter().copied())))
                        .collect::<Vec<_>>()
                        .join("; "),
                )
                .prio(P_HIGH);
            // "Varghese's last scene" is his last day alone. A question that says more than who and
            // when ("the scene where Jose confesses") leaves the days as reading.
            let narrowed = asked::asks_more_than(
                &ctx.message,
                std::slice::from_ref(&name),
                WHERE_WORDS,
            );
            let wanted: Vec<u32> = if m.any(&["last scene", "final scene", "last day"]) {
                per_day.keys().next_back().copied().into_iter().collect()
            } else if m.any(&["first scene", "first day"]) {
                per_day.keys().next().copied().into_iter().collect()
            } else if narrowed {
                Vec::new()
            } else {
                per_day.keys().copied().collect()
            };
            for d in wanted {
                f = f.require(format!("Day {d}"));
            }
            out.push(f);
        }
    }
    // A scene named by its place: "the sea day".
    if target.is_empty() && scenes.is_empty() {
        for c in WORD_DAY.captures_iter(&ctx.message) {
            let w = c[1].to_lowercase();
            if NOT_A_PLACE.contains(&w.as_str()) {
                continue;
            }
            if let Some(sp) = ctx.screenplay(r)
                && let Some(draft) = ctx.draft(&sp.id)
            {
                for s in queries::scenes(ctx.conn, &draft.id)? {
                    if s.heading.to_lowercase().contains(&w)
                        && let Some(n) = by_scene(&s.id).and_then(|d| d.number)
                    {
                        target.insert(n as u32);
                    }
                }
            }
        }
    }

    for n in &target {
        let Some(d) = days.iter().find(|d| d.number == Some(*n as i64)) else {
            out.push(Fact::absent(
                format!("shooting day {n}"),
                format!(
                    "the Schedule, which has {} shoot {}",
                    shoot_days.len(),
                    if shoot_days.len() == 1 { "day" } else { "days" }
                ),
            ));
            continue;
        };
        let ids = strips.get(&d.id).cloned().unwrap_or_default();
        let nums = ctx.scene_numbers(&ids);
        let mut shown: Vec<(u32, String)> =
            ids.iter().filter_map(|i| nums.get(i).cloned()).collect();
        shown.sort_unstable();
        let mut f = Fact::new("Day", &d.id)
            .f("day", format!("Day {n}"))
            .f("date", d.date.clone().unwrap_or_default())
            .f(
                "shooting start",
                d.shooting_start.map(clock).unwrap_or_default(),
            )
            .f("target wrap", d.target_wrap.map(clock).unwrap_or_default())
            .f("scenes", numbers(shown.iter().map(|s| s.0)))
            .f(
                "places",
                shown
                    .iter()
                    .map(|(n, h)| format!("{n}: {}", clip(&place(h), 48)))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
            .f("notes", clip(d.notes.as_deref().unwrap_or(""), 160))
            .prio(P_HIGH);
        // The times written on the day's call sheet; the day's own times when it has no sheet.
        let sheet = sheet_of_day(ctx, &d.id);
        let crew_call = sheet
            .as_ref()
            .map(|s| s.crew_call.trim().to_string())
            .filter(|c| !c.is_empty())
            .or_else(|| d.crew_call.map(clock));
        f = f.f("crew call", crew_call.clone().unwrap_or_default());
        if let Some(sh) = &sheet {
            f = f
                .f("general call", &sh.times.general_call)
                .f("shooting call", &sh.times.shooting_call)
                .f("lunch", &sh.times.lunch)
                .f("estimated wrap", &sh.times.estimated_wrap);
        }
        // "Call sheet" names a document; it does not ask for a call time.
        let times = m.without(&["call sheet", "call sheets", "callsheet", "callsheets"]);
        if times.any(&["call", "call time", "crew call", "calls"])
            && let Some(c) = crew_call
        {
            f = f.require(c);
        }
        let about_the_day = ASKS_THE_DAY.is_match(&Msg::new(&ctx.message).norm);
        if about_the_day || m.stem("scene") {
            for (n, _) in &shown {
                f = f.require(n.to_string());
            }
        }
        if m.stem("note") {
            f = f.require(clip(d.notes.as_deref().unwrap_or(""), 160));
        }
        out.push(f);
    }

    // ---- warnings: actors and places that clash with the schedule
    if m.any(&[
        "warning",
        "warnings",
        "clash",
        "clashes",
        "conflict",
        "conflicts",
        "problem",
        "problems",
        "unavailable",
    ]) {
        out.extend(warnings(ctx, read, &target)?);
    }

    // ---- call sheets of the days shown, with the times and the crew on them
    if !target.is_empty() || m.any(&["call sheet", "call sheets", "sides"]) {
        let detailed = !target.is_empty()
            && m.any(&[
                "call",
                "calls",
                "crew",
                "makeup",
                "lunch",
                "wrap",
                "time",
                "times",
                "meal",
                "parking",
                "safety",
                "emergency",
                "weather",
                "sunrise",
                "sunset",
                "hospital",
            ]);
        out.extend(call_sheets(
            ctx,
            read,
            &target.iter().copied().collect::<Vec<_>>(),
            detailed,
        )?);
    }
    Ok(out)
}

/// How many scenes of the board have no shooting day.
fn unscheduled(ctx: &FetchCtx<'_>, t: AnswerType) -> AppResult<Vec<Fact>> {
    let Some(sched) = board::current_schedule(ctx.conn)? else {
        return Ok(Vec::new());
    };
    let n: i64 = ctx.conn.query_row(
        "SELECT count(*) FROM schedule_strip WHERE schedule_id = ?1
         AND deleted_at IS NULL AND archived = 0 AND day_id IS NULL",
        [&sched.id],
        |r| r.get(0),
    )?;
    let mut f = Fact::new("Count", "")
        .f("what", "unscheduled scenes")
        .f("value", n.to_string())
        .prio(P_HIGH);
    if t == AnswerType::Count {
        f = f.require(n.to_string());
    }
    Ok(vec![f])
}

/// The place in a scene heading: `EXT. COAST RADIO STATION - NIGHT` to `COAST RADIO STATION`.
fn place(heading: &str) -> String {
    let h = queries::parse_heading(heading);
    if h.location.is_empty() {
        heading.to_string()
    } else {
        h.location
    }
}

fn warnings(ctx: &FetchCtx<'_>, read: &TurnRead, days: &BTreeSet<u32>) -> AppResult<Vec<Fact>> {
    let Some(sched) = board::current_schedule(ctx.conn)? else {
        return Ok(Vec::new());
    };
    let b = Board::load(ctx.conn, sched)?;
    let acks = board::load_acks(ctx.conn, &b.schedule.id)?;
    let all = b.warnings(&acks);
    let day_ids: Vec<String> = ctx
        .days()
        .iter()
        .filter(|d| d.number.is_some_and(|n| days.contains(&(n as u32))))
        .map(|d| d.id.clone())
        .collect();
    let on_days = |id: &Option<String>| {
        days.is_empty()
            || id
                .as_deref()
                .is_some_and(|i| day_ids.iter().any(|d| d == i))
    };
    let open: Vec<_> = all
        .iter()
        .filter(|w| w.decision.is_none() && on_days(&w.day_id))
        .collect();
    let t = read.answer_type;
    let mut out = vec![
        Fact::new("Count", "")
            .f("what", "open schedule warnings")
            .f("value", open.len().to_string())
            .prio(P_HIGH)
            .require(if t == AnswerType::Count {
                open.len().to_string()
            } else {
                String::new()
            }),
    ];
    let actors = ctx.m.any(&["actor", "actors", "cast", "who"]);
    // One fact per clash, a sentence with the person or place and the day, never the warning code.
    for w in open.iter().take(24) {
        let actor = w.kind.contains("actor") || w.kind.starts_with("cast");
        let mut f = Fact::new("Schedule warning", &w.key)
            .said(&w.message)
            .f("detail", w.detail.clone().unwrap_or_default())
            .f("scenes", numbers(w.scene_numbers.iter().map(|n| *n as u32)))
            .prio(if actors && actor { P_HIGH } else { P_MID });
        // The actor who cannot work the day the schedule needs them is the one who clashes.
        if actors
            && w.kind == "cast_unavailable"
            && let Some((name, _)) = w.message.split_once(" (")
        {
            f = f.require(name);
        }
        out.push(f);
    }
    Ok(out)
}

/// The live call sheet of a shooting day, when it has one.
fn sheet_of_day(ctx: &FetchCtx<'_>, day_id: &str) -> Option<CallSheetDocument> {
    let doc: String = ctx
        .conn
        .query_row(
            "SELECT document_json FROM call_sheet WHERE shoot_day_id = ?1 AND deleted_at IS NULL
             AND superseded_by IS NULL ORDER BY revision DESC LIMIT 1",
            [day_id],
            |r| r.get(0),
        )
        .ok()?;
    serde_json::from_str(&doc).ok()
}

/// Call sheets: their status and day; with `detailed` the times and lists written on the sheet.
/// `want` limits them to some shoot days.
pub fn call_sheets(
    ctx: &FetchCtx<'_>,
    read: &TurnRead,
    want: &[u32],
    detailed: bool,
) -> AppResult<Vec<Fact>> {
    let t = read.answer_type;
    let days = ctx.days();
    let mut st = ctx.conn.prepare(
        "SELECT id, title, status, revision, needs_refresh, shoot_day_id, document_json FROM call_sheet
         WHERE deleted_at IS NULL AND superseded_by IS NULL ORDER BY created_at, revision LIMIT 100",
    )?;
    type Row = (String, String, String, i64, bool, String, String);
    let rows: Vec<Row> = st
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    let label_of = |day_id: &str| -> Option<(u32, String)> {
        days.iter()
            .find(|d| d.id == day_id)
            .and_then(|d| Some((d.number? as u32, d.date.clone().unwrap_or_default())))
    };
    let asked = ctx.m.any(&[
        "call sheet",
        "call sheets",
        "callsheet",
        "callsheets",
        "sides",
        "call",
    ]);
    if ctx.m.has("sides") {
        let n: i64 = ctx
            .conn
            .query_row(
                "SELECT count(*) FROM side WHERE deleted_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if n == 0 && asked {
            out.push(Fact::absent(
                format!(
                    "any sides{}",
                    want.first()
                        .map(|d| format!(" for Day {d}"))
                        .unwrap_or_default()
                ),
                "Call Sheets",
            ));
        }
    }
    if rows.is_empty() {
        if asked {
            out.push(Fact::absent("any call sheets", "Call Sheets"));
        }
        return Ok(out);
    }
    let mut shown = 0;
    // A status the question filters by: its sheets are the answer, by their day.
    let filter: Option<&str> = SHEET_STATUS
        .iter()
        .find(|(_, words)| ctx.m.any(words))
        .map(|(s, _)| *s);
    let night = ctx.m.has("night");
    if t == AnswerType::Count && want.is_empty() && filter.is_none() {
        out.push(
            Fact::new("Count", "")
                .f("what", "call sheets")
                .f("value", rows.len().to_string())
                .require(rows.len().to_string())
                .prio(P_HIGH),
        );
    }
    for (id, title, status, rev, refresh, day_id, doc) in &rows {
        let day = label_of(day_id);
        if !want.is_empty() && day.as_ref().is_none_or(|(n, _)| !want.contains(n)) {
            continue;
        }
        shown += 1;
        let status = board::effective_status(status, *refresh);
        let label = day
            .as_ref()
            .map(|(n, _)| format!("Day {n}"))
            .unwrap_or_default();
        let document = serde_json::from_str::<CallSheetDocument>(doc).ok();
        let crew_call = document.as_ref().map_or("", |d| d.crew_call.trim());
        let mut f = Fact::new("Call sheet", id)
            .f("title", title)
            .f("day", &label)
            .f(
                "date",
                day.as_ref().map(|(_, d)| d.clone()).unwrap_or_default(),
            )
            .f("status", &status)
            .f("revision", rev.to_string())
            .f("crew call", crew_call)
            .prio(P_HIGH);
        let is_filtered = filter.is_some_and(|w| status.to_lowercase().contains(w));
        if want.is_empty() && is_filtered && matches!(t, AnswerType::List | AnswerType::Lookup) {
            f = f.require(&label);
        }
        // A day the question names: its status, when a word asks for it.
        if !want.is_empty() && matches!(t, AnswerType::List | AnswerType::Lookup) {
            f = f.require(&status);
        }
        // A night shoot is written in the day's notes, or shows as a call in the afternoon.
        if night
            && want.is_empty()
            && document.as_ref().is_some_and(|d| {
                d.day_notes.to_lowercase().contains("night") || late_call(&d.crew_call)
            })
        {
            f = f.require(&label).require(crew_call);
        }
        out.push(f);
        if detailed && let Some(d) = &document {
            out.extend(sheet_facts(ctx, id, day.as_ref().map(|(n, _)| *n), d));
        }
    }
    if shown == 0 && !want.is_empty() && asked {
        out.push(Fact::absent(
            format!(
                "a call sheet for Day {}",
                want.iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(" or Day ")
            ),
            "Call Sheets",
        ));
    }
    Ok(out)
}

/// A crew call in the afternoon or later: the shoot runs into the night.
fn late_call(call: &str) -> bool {
    call.split(':')
        .next()
        .and_then(|h| h.trim().parse::<u32>().ok())
        .is_some_and(|h| h >= 14)
}

/// The words of a call sheet document that answer questions: times, who is called when, where, and
/// the practical notes. Contact fields are named `contact` so the guard can take them out.
fn sheet_facts(ctx: &FetchCtx<'_>, id: &str, day: Option<u32>, d: &CallSheetDocument) -> Vec<Fact> {
    let label = day.map(|n| format!("Day {n}")).unwrap_or_default();
    let base = |kind: &'static str| Fact::new(kind, id).f("day", &label).prio(P_MID);
    // The time a question names is demanded, and "the call" means the crew call.
    let m = ctx.m.without(&["call sheet", "call sheets", "callsheet", "callsheets"]);
    let plain_call = m.any(&["call", "calls", "call time"])
        && !m.any(&["general call", "shooting call", "makeup", "make up"]);
    let mut times = base("Call sheet times")
        .f("crew call", &d.crew_call)
        .f("general call", &d.times.general_call)
        .f("shooting call", &d.times.shooting_call)
        .f("breakfast", &d.times.breakfast)
        .f("lunch", &d.times.lunch)
        .f("estimated wrap", &d.times.estimated_wrap)
        .f("day notes", clip(&d.day_notes, 240))
        .prio(P_HIGH);
    for (name, value) in [
        ("general call", &d.times.general_call),
        ("shooting call", &d.times.shooting_call),
        ("breakfast", &d.times.breakfast),
        ("lunch", &d.times.lunch),
        ("wrap", &d.times.estimated_wrap),
    ] {
        if m.has(name) {
            times = times.require(value);
        }
    }
    if plain_call {
        times = times.require(&d.crew_call);
    }
    let mut out = vec![times];
    let cast: Vec<String> = d
        .cast
        .iter()
        .map(|c| {
            let call = if c.call_time.trim().is_empty() {
                String::new()
            } else {
                format!(", call {}", c.call_time)
            };
            format!("{} ({}){call}", c.character, c.actor)
        })
        .collect();
    if !cast.is_empty() {
        out.push(base("Cast calls").f("calls", clip(&cast.join("; "), 700)));
    }
    if let Some(crew) = &d.optional.crew {
        let crew: Vec<String> = crew
            .iter()
            .map(|c| {
                let call = if c.call_time.trim().is_empty() {
                    String::new()
                } else {
                    format!(", call {}", c.call_time)
                };
                format!("{}, {}{call}", c.name, c.role)
            })
            .collect();
        if !crew.is_empty() {
            // A job the question names ("the makeup call"): that person's call is demanded.
            let mut calls = base("Crew calls")
                .f("calls", clip(&crew.join("; "), 800))
                .prio(P_HIGH);
            if m.any(&["earliest", "first call"])
                && let Some(c) = d
                    .optional
                    .crew
                    .iter()
                    .flatten()
                    .filter(|c| !c.call_time.trim().is_empty())
                    .min_by_key(|c| c.call_time.trim().to_string())
            {
                calls = calls.require(&c.name).require(&c.call_time);
            }
            for c in d.optional.crew.iter().flatten() {
                let job = c.role.to_lowercase();
                let named = job
                    .split(|x: char| !x.is_alphanumeric())
                    .any(|w| w.len() >= 4 && m.has(w));
                if named && m.any(&["call", "calls", "time"]) {
                    calls = calls.require(&c.call_time);
                }
            }
            out.push(calls);
        }
    }
    for l in &d.locations {
        out.push(
            base("Call sheet place")
                .f("name", &l.name)
                .f("address", clip(&l.address, 120))
                .f("meeting point", clip(&l.meeting_point, 120))
                .f("parking", clip(&l.parking, 120))
                .f("hospital", clip(&l.hospital, 120))
                .f("access", clip(&l.access, 120))
                .f(
                    "emergency contact",
                    if ctx.guard.contacts {
                        l.emergency_contact.as_str()
                    } else {
                        ""
                    },
                ),
        );
    }
    let p = &d.practical;
    let practical = base("Call sheet practical")
        .f("meal break", clip(&p.meal_break, 120))
        .f("safety", clip(&p.safety, 240))
        .f("special instructions", clip(&p.special_instructions, 240))
        .f("production notes", clip(&p.production_notes, 240))
        .f("travel", clip(&p.travel_notes, 160))
        .f(
            "weather",
            clip(d.optional.weather.as_deref().unwrap_or(""), 120),
        )
        .f("sunrise", &d.optional.sunrise)
        .f("sunset", &d.optional.sunset);
    if practical.fields.iter().any(|(n, _)| n != "day") {
        out.push(practical);
    }
    if let Some(depts) = &d.optional.departments {
        let text: Vec<String> = depts
            .iter()
            .map(|x| format!("{}: {}", x.department, x.notes))
            .collect();
        if !text.is_empty() {
            out.push(base("Call sheet departments").f("notes", clip(&text.join("; "), 400)));
        }
    }
    if let Some(extras) = &d.optional.extras {
        let text: Vec<String> = extras
            .iter()
            .map(|x| format!("{} x{} call {}", x.description, x.quantity, x.call_time))
            .collect();
        if !text.is_empty() {
            out.push(base("Call sheet extras").f("extras", clip(&text.join("; "), 300)));
        }
    }
    out
}

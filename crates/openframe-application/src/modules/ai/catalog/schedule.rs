//! Proposal tools for the shooting schedule, call sheets, sides, saved reports
//! and the budget. Scene numbers refer to the Production Source draft. Scene
//! moves never pre-accept schedule warnings ("Keep Anyway" stays the user's
//! decision), and call-sheet edits never rewrite the schedule.

use openframe_domain::{AppError, AppResult, Capability};
use serde_json::{Value, json};

use super::super::queries::{self, SceneRow};
use super::super::toolbox::resolve::{self as rv, Found};
use super::super::toolbox::schema as sc;
use super::super::types::ChangeSetDraft;
use super::kit::{A, Draft, date_text, need_change, plural, set_text};
use super::workspace::{neutral_scope, spec};
use super::{PropCtx, ProposalSpec};

const MARKER_TYPES: &[&str] = &["Meal", "Travel", "Company Move", "Custom"];
const REPORT_TYPES: &[&str] = &[
    "scene",
    "location",
    "cast_scene",
    "prop",
    "schedule",
    "breakdown_completeness",
];
const BUDGET_CATEGORIES: &[&str] = &[
    "Cast",
    "Crew",
    "Locations",
    "Equipment",
    "Art/Props",
    "Travel/Transport",
    "Food",
    "Post/Other",
    "Contingency",
];

pub const SPECS: &[ProposalSpec] = &[
    spec!(
        "propose_schedule",
        "Prepare creating the shooting schedule from the Production Source.",
        ["schedule.create"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_schedule,
        schedule
    ),
    spec!(
        "propose_schedule_settings",
        "Prepare changing schedule settings (name, strict validation, day length).",
        ["schedule.update_settings"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_settings,
        settings
    ),
    spec!(
        "propose_schedule_status",
        "Prepare changing the schedule status (Draft, Active, Finalized).",
        ["schedule.set_status"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_status,
        status
    ),
    spec!(
        "propose_delete_schedule",
        "Prepare moving the shooting schedule to Recently Deleted.",
        ["schedule.delete"],
        [],
        SoftDelete,
        "deleting the schedule",
        "Schedule",
        s_none,
        delete_schedule
    ),
    spec!(
        "propose_shooting_day",
        "Prepare a new shooting day (or off day), optionally with a date and notes.",
        ["schedule.create_day"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_day_new,
        day_new
    ),
    spec!(
        "propose_update_shooting_day",
        "Prepare changes to a shooting day: date, notes, target length, off day.",
        [
            "schedule.set_day_date",
            "schedule.set_day_notes",
            "schedule.set_day_target",
            "schedule.set_off_day"
        ],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_day_update,
        day_update
    ),
    spec!(
        "propose_move_shooting_day",
        "Prepare moving a shooting day to another position.",
        ["schedule.move_day"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_day_move,
        day_move
    ),
    spec!(
        "propose_duplicate_shooting_day",
        "Prepare duplicating a shooting day's breaks and notes.",
        ["schedule.duplicate_day"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_day,
        day_duplicate
    ),
    spec!(
        "propose_delete_shooting_day",
        "Prepare deleting a shooting day (its scenes return to Unscheduled).",
        ["schedule.delete_day"],
        [],
        SoftDelete,
        "changing the schedule",
        "Schedule",
        s_day,
        day_delete
    ),
    spec!(
        "propose_schedule_scenes",
        "Prepare moving scenes onto a shooting day (optionally at a position), or back to Unscheduled.",
        ["schedule.move_strip", "schedule.move_strips"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_move_scenes,
        move_scenes
    ),
    spec!(
        "propose_scene_timing",
        "Prepare a scene's estimated shooting time or page-count override.",
        ["schedule.set_strip_estimate", "schedule.set_strip_pages"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_timing,
        timing
    ),
    spec!(
        "propose_schedule_break",
        "Prepare a meal, travel, company move or custom break on a shooting day.",
        ["schedule.add_marker"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_break_new,
        break_new
    ),
    spec!(
        "propose_update_schedule_break",
        "Prepare edits to a schedule break.",
        ["schedule.update_marker"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_break_update,
        break_update
    ),
    spec!(
        "propose_move_schedule_break",
        "Prepare moving a schedule break to another day or position.",
        ["schedule.move_marker"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_break_move,
        break_move
    ),
    spec!(
        "propose_delete_schedule_break",
        "Prepare removing a schedule break (recoverable).",
        ["schedule.delete_marker"],
        [],
        SoftDelete,
        "changing the schedule",
        "Schedule",
        s_break,
        break_delete
    ),
    spec!(
        "propose_reconcile_schedule",
        "Prepare reconciling the schedule with Production Source changes (new, changed and removed scenes).",
        ["schedule.reconcile"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_none,
        reconcile
    ),
    spec!(
        "propose_acknowledge_scene_change",
        "Prepare acknowledging that a scheduled scene changed in the script.",
        ["schedule.acknowledge_change"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_scene,
        acknowledge
    ),
    spec!(
        "propose_confirm_scene_removal",
        "Prepare confirming removal of a scheduled scene that was cut from the script.",
        ["schedule.confirm_removal"],
        [],
        Edit,
        "changing the schedule",
        "Schedule",
        s_removed,
        confirm_removal
    ),
    spec!(
        "propose_call_sheet",
        "Prepare a call sheet for a shooting day (prefilled from the schedule).",
        ["callsheets.create"],
        [],
        Edit,
        "changing call sheets",
        "Call Sheets",
        s_day,
        callsheet_new
    ),
    spec!(
        "propose_update_call_sheet",
        "Prepare edits to a call sheet (crew call, day notes, parking, meeting point, travel, meals, emergency contact, production notes, weather, special notes).",
        ["callsheets.update"],
        [],
        Edit,
        "changing call sheets",
        "Call Sheets",
        s_callsheet_update,
        callsheet_update
    ),
    spec!(
        "propose_call_sheet_ready",
        "Prepare marking a call sheet Ready (or back to Draft).",
        ["callsheets.set_ready"],
        [],
        Edit,
        "changing call sheets",
        "Call Sheets",
        s_callsheet_ready,
        callsheet_ready
    ),
    spec!(
        "propose_refresh_call_sheet",
        "Prepare refreshing a call sheet from the current schedule.",
        ["callsheets.refresh"],
        [],
        Edit,
        "changing call sheets",
        "Call Sheets",
        s_callsheet,
        callsheet_refresh
    ),
    spec!(
        "propose_finalize_call_sheet",
        "Prepare finalizing a call sheet (frozen snapshot).",
        ["callsheets.finalize"],
        [],
        LockOrFinalize,
        "finalizing call sheets",
        "Call Sheets",
        s_callsheet_finalize,
        callsheet_finalize
    ),
    spec!(
        "propose_issue_call_sheet",
        "Prepare marking a finalized call sheet as issued.",
        ["callsheets.issue"],
        [],
        LockOrFinalize,
        "issuing call sheets",
        "Call Sheets",
        s_callsheet,
        callsheet_issue
    ),
    spec!(
        "propose_call_sheet_revision",
        "Prepare a new revision of a finalized call sheet.",
        ["callsheets.new_revision"],
        [],
        Edit,
        "changing call sheets",
        "Call Sheets",
        s_callsheet,
        callsheet_revision
    ),
    spec!(
        "propose_delete_call_sheet",
        "Prepare moving a call sheet to Recently Deleted.",
        ["callsheets.delete"],
        [],
        SoftDelete,
        "deleting call sheets",
        "Call Sheets",
        s_callsheet,
        callsheet_delete
    ),
    spec!(
        "propose_sides",
        "Prepare saving sides (script pages) for a shooting day.",
        ["sides.create"],
        [],
        Edit,
        "saving sides",
        "Sides & Reports",
        s_sides,
        sides_new
    ),
    spec!(
        "propose_delete_sides",
        "Prepare moving saved sides to Recently Deleted.",
        ["sides.delete"],
        [],
        SoftDelete,
        "deleting sides",
        "Sides & Reports",
        s_sides_ref,
        sides_delete
    ),
    spec!(
        "propose_save_report",
        "Prepare saving a production report snapshot (scene, location, cast/scene, prop, schedule, breakdown completeness).",
        ["reports.save"],
        [],
        Edit,
        "saving reports",
        "Sides & Reports",
        s_report,
        report_save
    ),
    spec!(
        "propose_delete_report",
        "Prepare moving a saved report to Recently Deleted.",
        ["reports.delete"],
        [],
        SoftDelete,
        "deleting reports",
        "Sides & Reports",
        s_report_ref,
        report_delete
    ),
    spec!(
        "propose_budget",
        "Prepare creating the project budget in a currency.",
        ["budget.create"],
        [],
        Edit,
        "changing the budget",
        "Budget",
        s_budget,
        budget_new
    ),
    spec!(
        "propose_budget_settings",
        "Prepare budget settings (currency, planned total, contingency, notes). Amounts are whole currency units.",
        ["budget.update"],
        [],
        Edit,
        "changing the budget",
        "Budget",
        s_budget_settings,
        budget_settings
    ),
    spec!(
        "propose_budget_line",
        "Prepare a budget line (category, description, amount in whole currency units).",
        ["budget.add_line"],
        [],
        Edit,
        "changing the budget",
        "Budget",
        s_line_new,
        line_new
    ),
    spec!(
        "propose_update_budget_line",
        "Prepare edits to a budget line.",
        ["budget.update_line"],
        [],
        Edit,
        "changing the budget",
        "Budget",
        s_line_update,
        line_update
    ),
    spec!(
        "propose_delete_budget_line",
        "Prepare removing a budget line (recoverable).",
        ["budget.delete_line"],
        [],
        SoftDelete,
        "changing the budget",
        "Budget",
        s_line,
        line_delete
    ),
    spec!(
        "propose_budget_snapshot",
        "Prepare saving a labelled budget snapshot.",
        ["budget.save_snapshot"],
        [],
        Edit,
        "changing the budget",
        "Budget",
        s_snapshot,
        snapshot
    ),
    spec!(
        "propose_budget_reviewed",
        "Prepare marking the budget reviewed against production changes.",
        ["budget.mark_reviewed"],
        [],
        Edit,
        "changing the budget",
        "Budget",
        s_none,
        budget_reviewed
    ),
    spec!(
        "propose_delete_budget_snapshot",
        "Prepare deleting a saved budget snapshot (recoverable).",
        ["budget.delete_snapshot"],
        [],
        SoftDelete,
        "changing the budget",
        "Budget",
        s_snapshot_ref,
        snapshot_delete
    ),
];

// ------------------------------------------------------------------ schemas

fn s_none() -> Value {
    sc::none()
}
fn s_schedule() -> Value {
    sc::obj(&[("name", sc::s(120))], &[])
}
fn s_settings() -> Value {
    sc::obj(
        &[
            ("name", sc::s(120)),
            ("strictValidation", sc::boolean()),
            ("dayLengthMinutes", sc::int(60, 1440)),
        ],
        &[],
    )
}
fn s_status() -> Value {
    sc::obj(
        &[("status", sc::en(&["Draft", "Active", "Finalized"]))],
        &["status"],
    )
}
fn day_ref() -> Value {
    sc::sd(40, "day number like \"Day 3\" or a date YYYY-MM-DD")
}
fn s_day() -> Value {
    sc::obj(&[("day", day_ref())], &["day"])
}
fn s_day_new() -> Value {
    sc::obj(
        &[
            ("date", sc::sd(10, "YYYY-MM-DD")),
            ("notes", sc::s(2000)),
            ("offDay", sc::boolean()),
            ("afterDay", day_ref()),
        ],
        &[],
    )
}
fn s_day_update() -> Value {
    sc::obj(
        &[
            ("day", day_ref()),
            ("date", sc::sd(10, "YYYY-MM-DD")),
            ("clearDate", sc::boolean()),
            ("notes", sc::s(2000)),
            ("targetMinutes", sc::int(30, 1440)),
            ("clearTarget", sc::boolean()),
            ("offDay", sc::boolean()),
        ],
        &["day"],
    )
}
fn s_day_move() -> Value {
    sc::obj(
        &[("day", day_ref()), ("toPosition", sc::int(1, 1000))],
        &["day", "toPosition"],
    )
}
fn s_move_scenes() -> Value {
    sc::obj(
        &[
            ("sceneNumbers", sc::arr(sc::scene_number(), 60)),
            ("day", day_ref()),
            ("unschedule", sc::boolean()),
            ("position", sc::int(1, 1000)),
        ],
        &["sceneNumbers"],
    )
}
fn s_timing() -> Value {
    sc::obj(
        &[
            ("sceneNumber", sc::scene_number()),
            ("estimateMinutes", sc::int(0, 1440)),
            ("clearEstimate", sc::boolean()),
            ("pageEighths", sc::int(1, 2000)),
            ("clearPages", sc::boolean()),
        ],
        &["sceneNumber"],
    )
}
fn s_break_new() -> Value {
    sc::obj(
        &[
            ("day", day_ref()),
            ("type", sc::en(MARKER_TYPES)),
            ("label", sc::s(120)),
            ("time", sc::sd(5, "HH:MM")),
            ("durationMinutes", sc::int(0, 1440)),
            ("notes", sc::s(1000)),
        ],
        &["day", "type"],
    )
}
fn s_break_update() -> Value {
    sc::obj(
        &[
            ("break", sc::reference()),
            ("type", sc::en(MARKER_TYPES)),
            ("label", sc::s(120)),
            ("time", sc::sd(5, "HH:MM")),
            ("durationMinutes", sc::int(0, 1440)),
            ("notes", sc::s(1000)),
        ],
        &["break"],
    )
}
fn s_break_move() -> Value {
    sc::obj(
        &[
            ("break", sc::reference()),
            ("day", day_ref()),
            ("position", sc::int(1, 1000)),
        ],
        &["break", "day"],
    )
}
fn s_break() -> Value {
    sc::obj(&[("break", sc::reference())], &["break"])
}
fn s_scene() -> Value {
    sc::obj(&[("sceneNumber", sc::scene_number())], &["sceneNumber"])
}
fn s_removed() -> Value {
    sc::obj(
        &[("scene", sc::sd(200, "heading of the removed scene"))],
        &["scene"],
    )
}
fn s_callsheet() -> Value {
    sc::obj(&[("callSheet", sc::reference())], &["callSheet"])
}
const CALLSHEET_FIELDS: &[(&str, &str, &str)] = &[
    // (arg, JSON pointer, label)
    ("crewCall", "/crewCall", "Crew call"),
    ("dayNotes", "/dayNotes", "Day notes"),
    ("parking", "/practical/parking", "Parking"),
    ("meetingPoint", "/practical/meetingPoint", "Meeting point"),
    ("travelNotes", "/practical/travelNotes", "Travel notes"),
    ("mealBreak", "/practical/mealBreak", "Meal break"),
    (
        "emergencyContact",
        "/practical/emergencyContact",
        "Emergency contact",
    ),
    (
        "productionNotes",
        "/practical/productionNotes",
        "Production notes",
    ),
    ("weather", "/optional/weather", "Weather"),
    ("specialNotes", "/optional/specialNotes", "Special notes"),
];
fn s_callsheet_update() -> Value {
    let mut p = vec![("callSheet", sc::reference())];
    for (k, _, _) in CALLSHEET_FIELDS {
        p.push((k, sc::s(2000)));
    }
    sc::obj(&p, &["callSheet"])
}
fn s_callsheet_ready() -> Value {
    sc::obj(
        &[("callSheet", sc::reference()), ("ready", sc::boolean())],
        &["callSheet", "ready"],
    )
}
fn s_callsheet_finalize() -> Value {
    sc::obj(
        &[
            ("callSheet", sc::reference()),
            ("acknowledgeStale", sc::boolean()),
        ],
        &["callSheet"],
    )
}
fn s_sides() -> Value {
    sc::obj(
        &[
            ("day", day_ref()),
            ("title", sc::s(200)),
            ("includeCover", sc::boolean()),
        ],
        &["day"],
    )
}
fn s_sides_ref() -> Value {
    sc::obj(&[("sides", sc::reference())], &["sides"])
}
fn s_report() -> Value {
    sc::obj(
        &[("type", sc::en(REPORT_TYPES)), ("title", sc::s(200))],
        &["type"],
    )
}
fn s_report_ref() -> Value {
    sc::obj(&[("report", sc::reference())], &["report"])
}
fn s_budget() -> Value {
    sc::obj(
        &[("currency", sc::sd(3, "ISO code, e.g. USD, INR, EUR"))],
        &["currency"],
    )
}
fn s_budget_settings() -> Value {
    sc::obj(
        &[
            ("currency", sc::sd(3, "ISO code")),
            ("plannedTotal", sc::int(0, 100_000_000_000)),
            ("contingencyMode", sc::en(&["percent", "amount"])),
            ("contingencyValue", sc::int(0, 100_000_000_000)),
            ("notes", sc::s(4000)),
        ],
        &[],
    )
}
fn s_line_new() -> Value {
    sc::obj(
        &[
            ("category", sc::en(BUDGET_CATEGORIES)),
            ("description", sc::s(300)),
            ("amount", sc::int(0, 100_000_000_000)),
            ("notes", sc::s(2000)),
        ],
        &["category", "description", "amount"],
    )
}
fn s_line_update() -> Value {
    sc::obj(
        &[
            ("line", sc::reference()),
            ("category", sc::en(BUDGET_CATEGORIES)),
            ("description", sc::s(300)),
            ("amount", sc::int(0, 100_000_000_000)),
            ("notes", sc::s(2000)),
        ],
        &["line"],
    )
}
fn s_line() -> Value {
    sc::obj(&[("line", sc::reference())], &["line"])
}
fn s_snapshot() -> Value {
    sc::obj(&[("label", sc::s(120))], &["label"])
}
fn s_snapshot_ref() -> Value {
    sc::obj(&[("snapshot", sc::s(120))], &["snapshot"])
}

// ------------------------------------------------------------------ helpers

fn sched(ctx: &PropCtx<'_>, d: &mut Draft) -> AppResult<(String, String)> {
    let (id, name) = rv::schedule(ctx.conn)?;
    d.base("shooting_schedule", &id);
    Ok((id, name))
}

fn day(ctx: &PropCtx<'_>, a: &A<'_>, key: &str, d: &mut Draft) -> AppResult<Found> {
    let f = rv::day(ctx.conn, &a.req(key, "shooting day")?)?;
    d.target("shooting_day", &f);
    Ok(f)
}

fn source_scene(ctx: &PropCtx<'_>, n: u32, d: &mut Draft) -> AppResult<SceneRow> {
    let scope = ctx.scope.cloned().unwrap_or_else(neutral_scope);
    let draft = rv::production_draft(ctx.conn, &scope, None)?;
    let s = rv::scene(ctx.conn, &draft, n)?;
    d.base("screenplay_scene", &s.id);
    Ok(s)
}

fn strip(ctx: &PropCtx<'_>, n: u32, d: &mut Draft) -> AppResult<(SceneRow, Found)> {
    let s = source_scene(ctx, n, d)?;
    let st = rv::strip_for_scene(ctx.conn, &s)?;
    d.target(
        "schedule_strip",
        &Found {
            id: st.id.clone(),
            label: s.label(),
            rev: st.rev,
        },
    );
    Ok((s, st))
}

fn marker(ctx: &PropCtx<'_>, a: &A<'_>, d: &mut Draft) -> AppResult<Found> {
    let f = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::MARKER,
        &a.req("break", "schedule break")?,
    )?;
    d.target("schedule_marker", &f);
    Ok(f)
}

fn callsheet(ctx: &PropCtx<'_>, a: &A<'_>, d: &mut Draft) -> AppResult<Found> {
    let f = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::CALL_SHEET,
        &a.req("callSheet", "call sheet")?,
    )?;
    d.target("call_sheet", &f);
    Ok(f)
}

fn hhmm(t: &str) -> AppResult<String> {
    let bad = || AppError::ai("tool_arguments", "Times must look like 13:30.");
    let (h, m) = t.trim().split_once(':').ok_or_else(bad)?;
    let (h, m): (u8, u8) = (h.parse().map_err(|_| bad())?, m.parse().map_err(|_| bad())?);
    if h > 23 || m > 59 {
        return Err(bad());
    }
    Ok(format!("{h:02}:{m:02}"))
}

// ------------------------------------------------------------------ schedule

fn schedule(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    if rv::schedule(ctx.conn).is_ok() {
        return Err(queries::ambiguous(
            "The project already has a shooting schedule. What should I change in it?",
        ));
    }
    let (_, draft_id) = queries::production_source_draft(ctx.conn)?.ok_or_else(|| {
        AppError::ai(
            "not_found",
            "Choose a Production Source draft before creating the schedule.",
        )
    })?;
    let draft =
        queries::draft_by_id(ctx.conn, &draft_id)?.ok_or_else(|| AppError::not_found("draft"))?;
    let mut d = Draft::new(spec, args, "Proposed shooting schedule");
    let mut op = json!({});
    if let Some(n) = a.s("name") {
        op["name"] = json!(n);
        d.row("Name", n);
    }
    d.row("From", format!("Production Source ({})", draft.label()))
        .row("Scenes", "All start in Unscheduled");
    d.op("schedule.create", op, "Create the shooting schedule");
    d.done()
}

fn settings(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed schedule settings");
    let (id, name) = sched(ctx, &mut d)?;
    let (strict, minutes, rev): (bool, i64, i64) = ctx.conn.query_row(
        "SELECT strict_validation, day_duration_minutes, rev FROM shooting_schedule WHERE id=?1",
        [&id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let mut op = json!({"scheduleId": id, "name": name, "strictValidation": strict, "dayDurationMinutes": minutes, "expectedRev": rev});
    let mut changed = set_text(&mut d, &mut op, "name", "Name", a.s("name"), &name, 120)?;
    if let Some(s) = a.b("strictValidation")
        && s != strict
    {
        d.change(
            "Strict validation",
            if strict { "On" } else { "Off" },
            if s { "On" } else { "Off" },
        );
        op["strictValidation"] = json!(s);
        changed = true;
    }
    if let Some(m) = a.i("dayLengthMinutes")
        && m != minutes
    {
        d.change("Day length", &format!("{minutes} min"), &format!("{m} min"));
        op["dayDurationMinutes"] = json!(m);
        changed = true;
    }
    need_change(changed, "the schedule settings")?;
    d.op("schedule.update_settings", op, "Update schedule settings");
    d.done()
}

fn status(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed schedule status");
    let (id, _) = sched(ctx, &mut d)?;
    let new = a.req("status", "status")?;
    if new == "Finalized" && !ctx.actor.can(Capability::LockOrFinalize) {
        return Err(AppError::permission_denied("finalize the schedule"));
    }
    let old = rv::column(ctx.conn, "shooting_schedule", "status", &id)?;
    if old == new {
        return Err(queries::ambiguous(format!(
            "The schedule is already {new}."
        )));
    }
    d.change("Schedule status", &old, &new);
    d.op(
        "schedule.set_status",
        json!({"scheduleId": id, "status": new}),
        format!("Set schedule status to {new}"),
    );
    d.done()
}

fn delete_schedule(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let mut d = Draft::new(spec, args, "Proposed schedule deletion");
    let (id, name) = sched(ctx, &mut d)?;
    let days = queries::count(
        ctx.conn,
        "SELECT count(*) FROM shooting_day WHERE schedule_id=?1 AND deleted_at IS NULL",
        [&id],
    )?;
    d.row("Delete schedule", name)
        .row("Shooting days", days.to_string())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "schedule.delete",
        json!({"scheduleId": id}),
        "Delete the shooting schedule",
    );
    d.done()
}

fn day_new(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed shooting day");
    let (id, _) = sched(ctx, &mut d)?;
    let off = a.flag("offDay");
    let mut op = json!({"scheduleId": id, "offDay": off});
    d.row("New", if off { "Off day" } else { "Shooting day" });
    if let Some(dt) = a.s("date") {
        let dt = date_text(&dt)?;
        d.row("Date", dt.clone());
        op["date"] = json!(dt);
    }
    if let Some(n) = a.s("notes") {
        d.row("Notes", queries::truncate_chars(&n, 300));
        op["notes"] = json!(n);
    }
    match a.s("afterDay") {
        Some(r) => {
            let after = rv::day(ctx.conn, &r)?;
            d.base("shooting_day", &after.id)
                .row("Place", format!("After {}", after.label));
            op["afterDayId"] = json!(after.id);
        }
        None => {
            d.row("Place", "At the end of the schedule");
        }
    }
    d.op("schedule.create_day", op, "Add a shooting day");
    d.done()
}

fn day_update(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed shooting day change");
    let dy = day(ctx, &a, "day", &mut d)?;
    let mut changed = false;
    if let Some(dt) = a.s("date") {
        let dt = date_text(&dt)?;
        d.change(
            &format!("{} date", dy.label),
            &rv::column(ctx.conn, "shooting_day", "shoot_date", &dy.id)?,
            &dt,
        );
        d.op(
            "schedule.set_day_date",
            json!({"dayId": dy.id, "date": dt}),
            format!("Set date of {}", dy.label),
        );
        changed = true;
    } else if a.flag("clearDate") {
        d.row(format!("{} date", dy.label), "Cleared");
        d.op(
            "schedule.set_day_date",
            json!({"dayId": dy.id, "date": null}),
            format!("Clear date of {}", dy.label),
        );
        changed = true;
    }
    if let Some(n) = a.raw("notes") {
        let old = rv::column(ctx.conn, "shooting_day", "notes", &dy.id)?;
        if old.trim() != n {
            d.change("Day notes", &old, &n);
            d.op(
                "schedule.set_day_notes",
                json!({"dayId": dy.id, "notes": n}),
                format!("Edit notes of {}", dy.label),
            );
            changed = true;
        }
    }
    if let Some(m) = a.i("targetMinutes") {
        d.row("Target length", format!("{m} min"));
        d.op(
            "schedule.set_day_target",
            json!({"dayId": dy.id, "minutes": m}),
            format!("Set target of {}", dy.label),
        );
        changed = true;
    } else if a.flag("clearTarget") {
        d.row("Target length", "Schedule default");
        d.op(
            "schedule.set_day_target",
            json!({"dayId": dy.id, "minutes": null}),
            format!("Clear target of {}", dy.label),
        );
        changed = true;
    }
    if let Some(off) = a.b("offDay") {
        let was: bool = ctx.conn.query_row(
            "SELECT is_off_day FROM shooting_day WHERE id=?1",
            [&dy.id],
            |r| r.get(0),
        )?;
        if was != off {
            d.change(
                "Day type",
                if was { "Off day" } else { "Shooting day" },
                if off { "Off day" } else { "Shooting day" },
            );
            d.op(
                "schedule.set_off_day",
                json!({"dayId": dy.id, "offDay": off}),
                format!("Change {} type", dy.label),
            );
            changed = true;
        }
    }
    need_change(changed, &dy.label)?;
    d.done()
}

fn day_move(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed day move");
    let dy = day(ctx, &a, "day", &mut d)?;
    let to = a.u("toPosition").unwrap_or(1).max(1);
    d.row("Move", dy.label.clone())
        .row("To position", to.to_string())
        .row("Its scenes", "Move with it");
    d.op(
        "schedule.move_day",
        json!({"dayId": dy.id, "index": to - 1}),
        format!("Move {} to position {to}", dy.label),
    );
    d.done()
}

fn day_duplicate(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed day copy");
    let dy = day(ctx, &a, "day", &mut d)?;
    d.row("Duplicate", dy.label.clone())
        .row("Copies", "Breaks and notes (scenes are not duplicated)");
    d.op(
        "schedule.duplicate_day",
        json!({"dayId": dy.id}),
        format!("Duplicate {}", dy.label),
    );
    d.done()
}

fn day_delete(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed day deletion");
    let dy = day(ctx, &a, "day", &mut d)?;
    let scenes = queries::count(
        ctx.conn,
        "SELECT count(*) FROM schedule_strip WHERE day_id=?1 AND deleted_at IS NULL",
        [&dy.id],
    )?;
    d.row("Delete", dy.label.clone())
        .row(
            "Its scenes",
            format!(
                "{} return to Unscheduled",
                plural(scenes as usize, "scene", "scenes")
            ),
        )
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "schedule.delete_day",
        json!({"dayId": dy.id}),
        format!("Delete {}", dy.label),
    );
    d.done()
}

fn move_scenes(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed schedule change");
    let numbers = a.ints("sceneNumbers");
    let mut strips = Vec::new();
    let mut labels = Vec::new();
    for n in &numbers {
        let (s, st) = strip(ctx, *n, &mut d)?;
        if !strips.iter().any(|x: &Found| x.id == st.id) {
            labels.push(s.number.to_string());
            strips.push(st);
        }
    }
    if strips.is_empty() {
        return Err(queries::ambiguous("Which scenes should I schedule?"));
    }
    let scenes_label = format!(
        "{} {}",
        if strips.len() == 1 { "Scene" } else { "Scenes" },
        queries::join_and(&labels)
    );
    if a.flag("unschedule") {
        d.row("Move", scenes_label).row("To", "Unscheduled");
        for st in &strips {
            d.op(
                "schedule.move_strip",
                json!({"stripId": st.id, "dayId": null, "keepAnyway": false}),
                format!("Unschedule {}", st.label),
            );
        }
        return d.done();
    }
    let dy = day(ctx, &a, "day", &mut d)?;
    let off: bool = ctx.conn.query_row(
        "SELECT is_off_day FROM shooting_day WHERE id=?1",
        [&dy.id],
        |r| r.get(0),
    )?;
    if off {
        return Err(queries::ambiguous(format!(
            "{} is an off day. Which shooting day should the scenes go to?",
            dy.label
        )));
    }
    d.row("Move", scenes_label).row("To", dy.label.clone());
    match a.u("position") {
        Some(p) => {
            d.row("Position", format!("From position {p} in the day"));
            for (i, st) in strips.iter().enumerate() {
                d.op(
                    "schedule.move_strip",
                    json!({"stripId": st.id, "dayId": dy.id, "index": (p - 1) as usize + i, "keepAnyway": false}),
                    format!("Schedule {} on {}", st.label, dy.label),
                );
            }
        }
        None if strips.len() == 1 => {
            d.op(
                "schedule.move_strip",
                json!({"stripId": strips[0].id, "dayId": dy.id, "keepAnyway": false}),
                format!("Schedule {} on {}", strips[0].label, dy.label),
            );
        }
        None => {
            let ids: Vec<String> = strips.iter().map(|s| s.id.clone()).collect();
            d.op(
                "schedule.move_strips",
                json!({"stripIds": ids, "dayId": dy.id, "keepAnyway": false}),
                format!("Schedule {} scenes on {}", strips.len(), dy.label),
            );
        }
    }
    d.row(
        "Schedule warnings",
        "Not pre-accepted; a strict schedule refuses moves that create new warnings",
    );
    d.done()
}

fn timing(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed scene timing");
    let (s, st) = strip(ctx, a.u("sceneNumber").unwrap_or(0), &mut d)?;
    let mut changed = false;
    if let Some(m) = a.i("estimateMinutes") {
        d.row(format!("{} estimate", s.label()), format!("{m} min"));
        d.op(
            "schedule.set_strip_estimate",
            json!({"stripId": st.id, "minutes": m}),
            format!("Estimate {}", s.label()),
        );
        changed = true;
    } else if a.flag("clearEstimate") {
        d.row(format!("{} estimate", s.label()), "Not estimated");
        d.op(
            "schedule.set_strip_estimate",
            json!({"stripId": st.id, "minutes": null}),
            format!("Clear estimate of {}", s.label()),
        );
        changed = true;
    }
    if let Some(e) = a.i("pageEighths") {
        d.row(
            format!("{} pages", s.label()),
            format!("{} {}/8", e / 8, e % 8),
        );
        d.op(
            "schedule.set_strip_pages",
            json!({"stripId": st.id, "eighths": e}),
            format!("Set pages of {}", s.label()),
        );
        changed = true;
    } else if a.flag("clearPages") {
        d.row(format!("{} pages", s.label()), "Screenplay estimate");
        d.op(
            "schedule.set_strip_pages",
            json!({"stripId": st.id, "eighths": null}),
            format!("Clear page override of {}", s.label()),
        );
        changed = true;
    }
    need_change(changed, &s.label())?;
    d.done()
}

fn break_new(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed schedule break");
    let dy = day(ctx, &a, "day", &mut d)?;
    let kind = a.req("type", "break type")?;
    let mut op = json!({"dayId": dy.id, "markerType": kind});
    d.row("Break", kind.clone()).row("Day", dy.label.clone());
    if let Some(l) = a.s("label") {
        d.row("Label", l.clone());
        op["label"] = json!(l);
    }
    if let Some(t) = a.s("time") {
        let t = hhmm(&t)?;
        d.row("At", t.clone());
        op["atTime"] = json!(t);
    }
    if let Some(m) = a.i("durationMinutes") {
        d.row("Duration", format!("{m} min"));
        op["durationMinutes"] = json!(m);
    }
    if let Some(n) = a.s("notes") {
        op["notes"] = json!(n);
    }
    d.op(
        "schedule.add_marker",
        op,
        format!("Add {kind} break to {}", dy.label),
    );
    d.done()
}

fn break_update(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed break edit");
    let m = marker(ctx, &a, &mut d)?;
    let (kind, label, at, dur, notes): (String, String, Option<String>, Option<i64>, Option<String>) = ctx.conn.query_row(
        "SELECT marker_type, label, at_time, duration_minutes, notes FROM schedule_marker WHERE id=?1",
        [&m.id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    let mut op = json!({"markerId": m.id, "markerType": kind, "label": label, "atTime": at, "durationMinutes": dur, "notes": notes});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "markerType",
        "Type",
        a.s("type"),
        &kind,
        20,
    )?;
    changed |= set_text(&mut d, &mut op, "label", "Label", a.s("label"), &label, 120)?;
    if let Some(t) = a.s("time") {
        let t = hhmm(&t)?;
        changed |= set_text(
            &mut d,
            &mut op,
            "atTime",
            "At",
            Some(t),
            at.as_deref().unwrap_or(""),
            5,
        )?;
    }
    if let Some(n) = a.i("durationMinutes")
        && Some(n) != dur
    {
        d.change(
            "Duration",
            &dur.map(|x| format!("{x} min")).unwrap_or_default(),
            &format!("{n} min"),
        );
        op["durationMinutes"] = json!(n);
        changed = true;
    }
    changed |= set_text(
        &mut d,
        &mut op,
        "notes",
        "Notes",
        a.raw("notes"),
        notes.as_deref().unwrap_or(""),
        1000,
    )?;
    need_change(changed, &format!("the break “{}”", m.label))?;
    d.op(
        "schedule.update_marker",
        op,
        format!("Edit break “{}”", m.label),
    );
    d.done()
}

fn break_move(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed break move");
    let m = marker(ctx, &a, &mut d)?;
    let dy = day(ctx, &a, "day", &mut d)?;
    let mut op = json!({"markerId": m.id, "dayId": dy.id});
    if let Some(p) = a.u("position") {
        op["index"] = json!(p.max(1) - 1);
    }
    d.row("Move break", m.label.clone())
        .row("To", dy.label.clone());
    d.op(
        "schedule.move_marker",
        op,
        format!("Move break “{}” to {}", m.label, dy.label),
    );
    d.done()
}

fn break_delete(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed break removal");
    let m = marker(ctx, &a, &mut d)?;
    d.row("Remove break", m.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "schedule.delete_marker",
        json!({"markerId": m.id}),
        format!("Remove break “{}”", m.label),
    );
    d.done()
}

fn reconcile(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let mut d = Draft::new(spec, args, "Proposed schedule reconcile");
    let (id, name) = sched(ctx, &mut d)?;
    d.row("Reconcile", name)
        .row("New scenes", "Added to Unscheduled")
        .row(
            "Changed or cut scenes",
            "Flagged for your review (never removed silently)",
        );
    d.op(
        "schedule.reconcile",
        json!({"scheduleId": id}),
        "Reconcile the schedule with the Production Source",
    );
    d.done()
}

fn acknowledge(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed acknowledgement");
    let (s, st) = strip(ctx, a.u("sceneNumber").unwrap_or(0), &mut d)?;
    d.row("Acknowledge script change", s.label());
    d.op(
        "schedule.acknowledge_change",
        json!({"stripId": st.id}),
        format!("Acknowledge change to {}", s.label()),
    );
    d.done()
}

fn confirm_removal(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed removal from the schedule");
    let (id, _) = sched(ctx, &mut d)?;
    let mut stmt = ctx.conn.prepare(
        "SELECT id, source_heading, rev FROM schedule_strip WHERE schedule_id=?1 AND source_state='Removed' AND deleted_at IS NULL AND archived=0",
    )?;
    let removed: Vec<Found> = stmt
        .query_map([&id], |r| {
            Ok(Found {
                id: r.get(0)?,
                label: r.get(1)?,
                rev: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    if removed.is_empty() {
        return Err(AppError::ai(
            "not_found",
            "No scheduled scene is waiting for removal confirmation.",
        ));
    }
    let st = rv::pick("removed scene", &a.req("scene", "removed scene")?, removed)?;
    d.target("schedule_strip", &st);
    d.row("Remove from schedule", st.label.clone())
        .row("Why", "The scene was cut from the Production Source");
    d.op(
        "schedule.confirm_removal",
        json!({"stripId": st.id}),
        format!("Confirm removal of “{}”", st.label),
    );
    d.done()
}

// ------------------------------------------------------------------ call sheets

fn callsheet_new(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed call sheet");
    let dy = day(ctx, &a, "day", &mut d)?;
    d.row("Call sheet for", dy.label.clone()).row(
        "Content",
        "Prefilled from the schedule; the schedule is not changed",
    );
    d.op(
        "callsheets.create",
        json!({"dayId": dy.id}),
        format!("Create call sheet for {}", dy.label),
    );
    d.done()
}

fn callsheet_update(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed call sheet edit");
    let cs = callsheet(ctx, &a, &mut d)?;
    let status = rv::column(ctx.conn, "call_sheet", "status", &cs.id)?;
    if matches!(status.as_str(), "Final" | "Issued" | "Superseded") {
        return Err(AppError::ai(
            "locked",
            format!(
                "“{}” is {status}; start a new revision to change it.",
                cs.label
            ),
        ));
    }
    let doc_json = rv::column(ctx.conn, "call_sheet", "document_json", &cs.id)?;
    let mut doc: Value =
        serde_json::from_str(&doc_json).map_err(|e| AppError::internal(e.to_string()))?;
    let mut changed = false;
    for (key, pointer, label) in CALLSHEET_FIELDS {
        let Some(new) = a.raw(key) else { continue };
        let old = doc
            .pointer(pointer)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if old.trim() == new {
            continue;
        }
        let (parent, leaf) = pointer.rsplit_once('/').unwrap_or(("", pointer));
        if !parent.is_empty() && doc.pointer(parent).is_none_or(|p| !p.is_object()) {
            let section = parent.trim_start_matches('/');
            doc[section] = json!({});
        }
        let target = if parent.is_empty() {
            Some(&mut doc)
        } else {
            doc.pointer_mut(parent)
        };
        if let Some(obj) = target.and_then(|t| t.as_object_mut()) {
            obj.insert(leaf.to_string(), json!(new));
            d.change(label, &old, &new);
            changed = true;
        }
    }
    need_change(changed, &format!("call sheet “{}”", cs.label))?;
    d.row("Schedule", "Not changed");
    d.op(
        "callsheets.update",
        json!({"id": cs.id, "document": doc, "expectedRev": cs.rev}),
        format!("Edit call sheet “{}”", cs.label),
    );
    d.done()
}

fn callsheet_simple(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
    title: &str,
    op: &str,
    verb: &str,
    extra: Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, title);
    let cs = callsheet(ctx, &a, &mut d)?;
    d.row(verb, cs.label.clone());
    let mut o = json!({"id": cs.id});
    if let (Some(dst), Some(src)) = (o.as_object_mut(), extra.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    d.op(op, o, format!("{verb} “{}”", cs.label));
    d.done()
}

fn callsheet_ready(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let ready = A(args).flag("ready");
    callsheet_simple(
        ctx,
        spec,
        args,
        "Proposed call sheet status",
        "callsheets.set_ready",
        if ready { "Mark Ready" } else { "Back to Draft" },
        json!({"ready": ready}),
    )
}

fn callsheet_refresh(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    callsheet_simple(
        ctx,
        spec,
        args,
        "Proposed call sheet refresh",
        "callsheets.refresh",
        "Refresh from the schedule",
        json!({}),
    )
}

fn callsheet_finalize(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let ack = A(args).flag("acknowledgeStale");
    callsheet_simple(
        ctx,
        spec,
        args,
        "Proposed call sheet finalization",
        "callsheets.finalize",
        "Finalize",
        json!({"acknowledgeStale": ack}),
    )
}

fn callsheet_issue(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    callsheet_simple(
        ctx,
        spec,
        args,
        "Proposed call sheet issue",
        "callsheets.issue",
        "Mark issued",
        json!({}),
    )
}

fn callsheet_revision(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    callsheet_simple(
        ctx,
        spec,
        args,
        "Proposed call sheet revision",
        "callsheets.new_revision",
        "Start a new revision of",
        json!({}),
    )
}

fn callsheet_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    callsheet_simple(
        ctx,
        spec,
        args,
        "Proposed call sheet deletion",
        "callsheets.delete",
        "Delete",
        json!({}),
    )
}

// ------------------------------------------------------------------ sides & reports

fn sides_new(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed sides");
    let dy = day(ctx, &a, "day", &mut d)?;
    let scenes = queries::count(
        ctx.conn,
        "SELECT count(*) FROM schedule_strip WHERE day_id=?1 AND deleted_at IS NULL AND archived=0",
        [&dy.id],
    )?;
    if scenes == 0 {
        return Err(AppError::ai(
            "not_found",
            format!("{} has no scenes, so there are no sides to save.", dy.label),
        ));
    }
    let cover = a.flag("includeCover");
    let mut op =
        json!({"dayId": dy.id, "stripIds": [], "includeCover": cover, "showDraftName": true});
    if let Some(t) = a.s("title") {
        op["title"] = json!(t);
        d.row("Title", t);
    }
    d.row(
        "Sides for",
        format!(
            "{} ({})",
            dy.label,
            plural(scenes as usize, "scene", "scenes")
        ),
    )
    .row("Cover page", if cover { "Yes" } else { "No" });
    d.op("sides.create", op, format!("Save sides for {}", dy.label));
    d.done()
}

fn sides_delete(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let f = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::SIDE,
        &a.req("sides", "saved sides")?,
    )?;
    let mut d = Draft::new(spec, args, "Proposed sides deletion");
    d.target("side", &f)
        .row("Delete sides", f.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "sides.delete",
        json!({"id": f.id}),
        format!("Delete sides “{}”", f.label),
    );
    d.done()
}

fn report_save(_ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let t = a.req("type", "report type")?;
    let mut d = Draft::new(spec, args, "Proposed saved report");
    let mut op = json!({"reportType": t});
    if let Some(title) = a.s("title") {
        op["title"] = json!(title);
        d.row("Title", title);
    }
    d.row("Report", t.replace('_', " "))
        .row("Content", "A snapshot of the current data");
    d.op("reports.save", op, "Save a production report");
    d.done()
}

fn report_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let f = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::REPORT,
        &a.req("report", "saved report")?,
    )?;
    let mut d = Draft::new(spec, args, "Proposed report deletion");
    d.target("production_report", &f)
        .row("Delete report", f.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "reports.delete",
        json!({"id": f.id}),
        format!("Delete report “{}”", f.label),
    );
    d.done()
}

// ------------------------------------------------------------------ budget

fn money(minor: i64, currency: &str) -> String {
    format!("{currency} {}.{:02}", minor / 100, minor % 100)
}

fn budget_new(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    if rv::budget(ctx.conn).is_ok() {
        return Err(queries::ambiguous(
            "The project already has a budget. What should I change in it?",
        ));
    }
    let currency = a.req("currency", "currency")?.to_uppercase();
    let mut d = Draft::new(spec, args, "Proposed budget");
    d.row("Budget currency", currency.clone());
    d.op(
        "budget.create",
        json!({"currency": currency}),
        "Create the project budget",
    );
    d.done()
}

fn budget_settings(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let b = rv::budget(ctx.conn)?;
    let mut d = Draft::new(spec, args, "Proposed budget settings");
    d.target("budget_snapshot", &b);
    let (planned, mode, value, notes): (Option<i64>, String, i64, Option<String>) = ctx.conn.query_row(
        "SELECT planned_total, contingency_mode, contingency_value, notes FROM budget_snapshot WHERE id=?1",
        [&b.id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let mut op = json!({"id": b.id, "currency": b.label, "plannedTotal": planned, "contingencyMode": mode, "contingencyValue": value, "notes": notes, "expectedRev": b.rev});
    let mut changed = false;
    if let Some(c) = a.s("currency").map(|c| c.to_uppercase())
        && c != b.label
    {
        d.change("Currency", &b.label, &c);
        op["currency"] = json!(c);
        changed = true;
    }
    if let Some(t) = a.i("plannedTotal") {
        d.change(
            "Planned total",
            &planned.map(|p| money(p, &b.label)).unwrap_or_default(),
            &money(t * 100, &b.label),
        );
        op["plannedTotal"] = json!(t * 100);
        changed = true;
    }
    let new_mode = a.s("contingencyMode").unwrap_or(mode.clone());
    if let Some(v) = a.i("contingencyValue") {
        // Percent is stored in basis points (10% = 1000); amounts in minor units.
        let stored = v * 100;
        let shown = if new_mode == "percent" {
            format!("{v}%")
        } else {
            money(v * 100, &b.label)
        };
        d.row("Contingency", shown);
        op["contingencyMode"] = json!(new_mode);
        op["contingencyValue"] = json!(stored);
        changed = true;
    } else if new_mode != mode {
        d.change("Contingency mode", &mode, &new_mode);
        op["contingencyMode"] = json!(new_mode);
        changed = true;
    }
    changed |= set_text(
        &mut d,
        &mut op,
        "notes",
        "Notes",
        a.raw("notes"),
        notes.as_deref().unwrap_or(""),
        4000,
    )?;
    need_change(changed, "the budget settings")?;
    d.op("budget.update", op, "Update budget settings");
    d.done()
}

fn line_new(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let b = rv::budget(ctx.conn)?;
    let mut d = Draft::new(spec, args, "Proposed budget line");
    d.base("budget_snapshot", &b.id);
    let category = a.req("category", "category")?;
    let description = a.req("description", "line description")?;
    let amount = a.i("amount").unwrap_or(0) * 100;
    let mut op = json!({"budgetId": b.id, "category": category, "description": description, "amount": amount});
    if let Some(n) = a.s("notes") {
        op["notes"] = json!(n);
    }
    d.row("Budget line", format!("{description} ({category})"))
        .row("Amount", money(amount, &b.label));
    d.op(
        "budget.add_line",
        op,
        format!("Add budget line “{description}”"),
    );
    d.done()
}

fn line(ctx: &PropCtx<'_>, a: &A<'_>, d: &mut Draft) -> AppResult<Found> {
    let f = rv::find(
        ctx.conn,
        ctx.actor,
        &rv::BUDGET_LINE,
        &a.req("line", "budget line")?,
    )?;
    d.target("budget_line", &f);
    Ok(f)
}

fn line_update(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let b = rv::budget(ctx.conn)?;
    let mut d = Draft::new(spec, args, "Proposed budget line edit");
    let l = line(ctx, &a, &mut d)?;
    let (category, amount, notes): (String, i64, Option<String>) = ctx.conn.query_row(
        "SELECT category, amount, notes FROM budget_line WHERE id=?1",
        [&l.id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let mut op = json!({"id": l.id, "category": category, "description": l.label, "amount": amount, "notes": notes, "expectedRev": l.rev});
    let mut changed = set_text(
        &mut d,
        &mut op,
        "category",
        "Category",
        a.s("category"),
        &category,
        40,
    )?;
    changed |= set_text(
        &mut d,
        &mut op,
        "description",
        "Description",
        a.s("description"),
        &l.label,
        300,
    )?;
    if let Some(n) = a.i("amount")
        && n * 100 != amount
    {
        d.change(
            "Amount",
            &money(amount, &b.label),
            &money(n * 100, &b.label),
        );
        op["amount"] = json!(n * 100);
        changed = true;
    }
    changed |= set_text(
        &mut d,
        &mut op,
        "notes",
        "Notes",
        a.raw("notes"),
        notes.as_deref().unwrap_or(""),
        2000,
    )?;
    need_change(changed, &format!("the budget line “{}”", l.label))?;
    d.op(
        "budget.update_line",
        op,
        format!("Edit budget line “{}”", l.label),
    );
    d.done()
}

fn line_delete(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut d = Draft::new(spec, args, "Proposed budget line removal");
    let l = line(ctx, &a, &mut d)?;
    d.row("Remove budget line", l.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "budget.delete_line",
        json!({"id": l.id}),
        format!("Remove budget line “{}”", l.label),
    );
    d.done()
}

fn snapshot(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let b = rv::budget(ctx.conn)?;
    let label = a.req("label", "snapshot label")?;
    let mut d = Draft::new(spec, args, "Proposed budget snapshot");
    d.base("budget_snapshot", &b.id)
        .row("Save snapshot", label.clone())
        .row("Current budget", "Stays editable");
    d.op(
        "budget.save_snapshot",
        json!({"id": b.id, "label": label}),
        format!("Save budget snapshot “{label}”"),
    );
    d.done()
}

fn budget_reviewed(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let b = rv::budget(ctx.conn)?;
    let mut d = Draft::new(spec, args, "Proposed budget review");
    d.base("budget_snapshot", &b.id).row(
        "Mark reviewed",
        "The budget against current production changes",
    );
    d.op(
        "budget.mark_reviewed",
        json!({"id": b.id}),
        "Mark the budget reviewed",
    );
    d.done()
}

fn snapshot_delete(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a = A(args);
    let mut stmt = ctx.conn.prepare(
        "SELECT id, COALESCE(label, 'Snapshot'), rev FROM budget_snapshot WHERE is_current=0 AND deleted_at IS NULL ORDER BY frozen_at DESC",
    )?;
    let snaps: Vec<Found> = stmt
        .query_map([], |r| {
            Ok(Found {
                id: r.get(0)?,
                label: r.get(1)?,
                rev: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let reference = a.req("snapshot", "budget snapshot")?;
    let s = match snaps.iter().find(|f| f.id == reference) {
        Some(f) => f.clone(),
        None => rv::pick("budget snapshot", &reference, snaps)?,
    };
    let mut d = Draft::new(spec, args, "Proposed snapshot deletion");
    d.target("budget_snapshot", &s)
        .row("Delete snapshot", s.label.clone())
        .row("Recoverable", "Yes — from Recently Deleted");
    d.op(
        "budget.delete_snapshot",
        json!({"id": s.id}),
        format!("Delete budget snapshot “{}”", s.label),
    );
    d.done()
}

#[cfg(test)]
mod tests {
    use super::{hhmm, money};

    #[test]
    fn times_and_money_format() {
        assert_eq!(hhmm("9:05").unwrap(), "09:05");
        assert!(hhmm("25:00").is_err());
        assert_eq!(money(150_050, "USD"), "USD 1500.50");
    }
}

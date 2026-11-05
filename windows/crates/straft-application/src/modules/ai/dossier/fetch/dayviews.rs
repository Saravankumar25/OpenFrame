//! Production > Day Out of Days, Daily View and Sides. Each reads through the function its screen
//! calls (`dood::report`, `ops::daily_view`, `docs::build_sides`), so the assistant sees the same
//! numbers and codes as the page. Nothing here writes.

use std::collections::BTreeSet;

use straft_domain::AppResult;

use super::schedule::named_days;
use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::{clip, numbers};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::turn::{AnswerType, TurnRead};
use crate::modules::schedule::board::DayRow;
use crate::modules::schedule::docs::{ScheduleSidesArgs, build_sides};
use crate::modules::schedule::dood::{self, ScheduleDoodArgs};
use crate::modules::schedule::ops::daily_view;

/// The most days one question reads in full.
const DAYS: usize = 14;
/// The most scenes of one day given with their text.
const SCENES: usize = 8;

/// The shooting days a question is about: the ones it names, else the first shoot days.
fn target_days(ctx: &FetchCtx<'_>, r: &Resolved) -> Vec<DayRow> {
    let named: BTreeSet<u32> = named_days(ctx, r).into_iter().collect();
    let days = ctx.days();
    let pick: Vec<DayRow> = days
        .iter()
        .filter(|d| !d.off && d.number.is_some_and(|n| named.contains(&(n as u32))))
        .cloned()
        .collect();
    if !pick.is_empty() {
        return pick;
    }
    days.iter().filter(|d| !d.off).take(DAYS).cloned().collect()
}

/// Production > Day Out of Days: who works which day, and each cast member's first and last work day.
pub fn dood_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("day out of days", || dood_facts(ctx, read, r))
}

fn dood_facts(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let Some((_, view)) = dood::report(ctx.conn, &ScheduleDoodArgs::default())? else {
        return Ok(vec![Fact::absent(
            "a Day Out of Days report",
            "the Schedule, which has no shooting days yet",
        )]);
    };
    if view.rows.is_empty() {
        return Ok(vec![Fact::absent(
            "any cast in the Day Out of Days",
            "the Day Out of Days",
        )]);
    }
    let wanted: Vec<String> = r
        .characters
        .iter()
        .chain(r.persons.iter())
        .map(|h| h.name.to_lowercase())
        .collect();
    let named_columns: Vec<(usize, String)> = named_days(ctx, r)
        .into_iter()
        .filter_map(|n| {
            let label = format!("Day {n}");
            view.days
                .iter()
                .position(|d| d.short_label == label)
                .map(|i| (i, label))
        })
        .collect();
    let mut out = Vec::new();
    let mut count = Fact::new("Count", "")
        .f("what", "cast members in the Day Out of Days")
        .f("value", view.rows.len().to_string())
        .prio(P_HIGH);
    if read.answer_type == AnswerType::Count {
        count = count.require(view.rows.len().to_string());
    }
    out.push(count);
    for row in &view.rows {
        let label = row.label.to_lowercase();
        let hit = wanted
            .iter()
            .any(|w| !w.is_empty() && label.contains(w.as_str()));
        let mut f = Fact::new("Day out of days", &row.key)
            .f("cast", &row.label)
            .f("work days", row.work_days.to_string())
            .f("hold days", row.hold_days.to_string())
            .f("travel days", row.travel_days.to_string())
            .f(
                "first work day",
                row.start_label.clone().unwrap_or_default(),
            )
            .f(
                "last work day",
                row.finish_label.clone().unwrap_or_default(),
            )
            .prio(if hit { P_HIGH } else { P_MID });
        let codes: Vec<String> = row
            .cells
            .iter()
            .zip(view.days.iter())
            .filter(|(c, _)| !c.code.is_empty() && c.code != "-")
            .map(|(c, d)| format!("{} {}", d.short_label, c.code))
            .collect();
        if hit || view.rows.len() <= 6 {
            f = f.f("days", clip(&codes.join(", "), 360));
        }
        for (i, label) in &named_columns {
            if let Some(c) = row.cells.get(*i) {
                f = f.f(
                    &format!("on {label}"),
                    if c.code.is_empty() { "-" } else { &c.code },
                );
                if hit {
                    // The code in words: hold (H) is paid but not working.
                    f = f.f(
                        &format!("works on {label}"),
                        if dood::is_work(&c.code) { "yes" } else { "no" },
                    );
                }
            }
        }
        if hit {
            f = f
                .require(row.work_days.to_string())
                .require(row.start_label.clone().unwrap_or_default())
                .require(row.finish_label.clone().unwrap_or_default());
        }
        out.push(f);
    }
    Ok(out)
}

/// Production > Daily View: one line for each shooting day with its scenes, cast, crew call,
/// locations, call sheet status and open warnings.
pub fn daily_view_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("daily view", || daily_facts(ctx, read, r))
}

fn daily_facts(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let days = target_days(ctx, r);
    if days.is_empty() {
        return Ok(vec![Fact::absent(
            "any shooting days",
            "the Schedule, which has no shooting days yet",
        )]);
    }
    let named = !named_days(ctx, r).is_empty();
    let mut out = Vec::new();
    for d in &days {
        let Some(day) = daily_view(ctx.conn, Some(&d.id))?.day else {
            continue;
        };
        let scenes = day
            .scenes
            .iter()
            .map(|s| {
                format!(
                    "{} {}",
                    s.number.clone().unwrap_or_default(),
                    s.location_name
                        .clone()
                        .unwrap_or_else(|| clip(&s.heading, 40))
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let cast = day
            .cast
            .iter()
            .map(|c| match &c.actor {
                Some(a) => format!("{} ({a})", c.character),
                None => c.character.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        let open = day.warnings.iter().filter(|w| w.decision.is_none()).count();
        // What the day needs from the breakdown, by category: props, wardrobe, vehicles ...
        let mut needs: Vec<(String, Vec<String>)> = Vec::new();
        for item in &day.breakdown_items {
            let cat = item.category.to_lowercase();
            match needs.iter_mut().find(|(c, _)| *c == cat) {
                Some((_, v)) => v.push(item.name.clone()),
                None => needs.push((cat, vec![item.name.clone()])),
            }
        }
        let mut f = Fact::new("Daily view", &day.id)
            .f("day", &day.label)
            .f("date", day.date.clone().unwrap_or_default())
            .f("scenes", clip(&scenes, 400))
            .f("cast", clip(&cast, 300))
            .f("crew call", day.crew_call.clone().unwrap_or_default())
            .f(
                "locations",
                day.locations
                    .iter()
                    .map(|l| l.name.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .f(
                "call sheet",
                day.call_sheet
                    .as_ref()
                    .map(|c| c.status.clone())
                    .unwrap_or_else(|| "none yet".into()),
            )
            .f("open warnings", open.to_string())
            .f("length", &day.summary.total_label)
            .f("notes", clip(day.notes.as_deref().unwrap_or(""), 160))
            .prio(if named { P_HIGH } else { P_MID });
        for (cat, names) in &needs {
            f = f.f(&format!("needs {cat}"), clip(&names.join(", "), 260));
        }
        if named {
            for n in day.scenes.iter().filter_map(|s| s.number.clone()) {
                f = f.require(n);
            }
        }
        out.push(f);
    }
    if out.is_empty() {
        out.push(Fact::absent("a Daily View for that day", "the Schedule"));
    }
    let _ = read;
    Ok(out)
}

/// Production > Sides: the scenes of a shooting day as the sides pages list them. With a day named
/// each scene is given with who speaks and the start of its text; without one, one line per day.
pub fn sides_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("sides", || sides_facts(ctx, read, r))
}

fn sides_facts(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let days = target_days(ctx, r);
    if days.is_empty() {
        return Ok(vec![Fact::absent(
            "any shooting days to make sides for",
            "the Schedule",
        )]);
    }
    let named = !named_days(ctx, r).is_empty();
    let focus = ctx.focus_scenes(r);
    let mut out = Vec::new();
    for d in &days {
        let args = ScheduleSidesArgs {
            day_id: Some(d.id.clone()),
            strip_ids: Vec::new(),
            cast_keys: None,
            include_cover: false,
            show_draft_name: true,
            title: None,
        };
        let content = match build_sides(ctx.conn, &args) {
            Ok((c, _, _)) => c,
            Err(e) => {
                out.push(Fact::absent(
                    "sides for that day",
                    format!("the Sides ({})", e.message),
                ));
                continue;
            }
        };
        let label = content
            .day_label
            .clone()
            .unwrap_or_else(|| "this day".into());
        let nums: Vec<u32> = content
            .scenes
            .iter()
            .filter_map(|s| s.number.parse().ok())
            .collect();
        let mut f = Fact::new("Sides", &d.id)
            .f("day", &label)
            .f("date", content.date_label.clone().unwrap_or_default())
            .f("scene count", content.scenes.len().to_string())
            .f("scenes", numbers(nums.iter().copied()))
            .prio(if named { P_HIGH } else { P_MID });
        if named {
            f = f.require(content.scenes.len().to_string());
            for n in &nums {
                f = f.require(n.to_string());
            }
        }
        out.push(f);
        if !named && focus.is_empty() {
            continue;
        }
        for s in content.scenes.iter().take(SCENES) {
            if !focus.is_empty() && !focus.iter().any(|f| f.id == s.scene_id) {
                continue;
            }
            let mut speakers: Vec<String> = Vec::new();
            for e in s.elements.iter().filter(|e| e.element_type == "character") {
                let n = crate::modules::ai::queries::normalize_cue(&e.text);
                let n = ctx.story_name(&n);
                if !n.is_empty() && !speakers.contains(&n) {
                    speakers.push(n);
                }
            }
            let text = s
                .elements
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            out.push(
                Fact::new("Sides", &s.scene_id)
                    .f("day", &label)
                    .f("scene", &s.number)
                    .f("heading", &s.heading)
                    .f("speaking", speakers.join(", "))
                    .f(
                        "text",
                        clip(
                            &text,
                            if read.answer_type.narrates() {
                                700
                            } else {
                                320
                            },
                        ),
                    )
                    .prio(P_LOW + 30),
            );
        }
    }
    Ok(out)
}

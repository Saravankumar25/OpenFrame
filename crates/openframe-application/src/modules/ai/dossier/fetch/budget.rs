//! Budget: planned, actual and difference of a line, a category or the whole budget, computed by
//! code with the same rules as the Budget screen and the PDF (difference is planned minus actual over
//! the lines that have an actual). Reads `budget_snapshot` and `budget_line`.

use openframe_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::Msg;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::{Resolved, normalize};
use crate::modules::ai::turn::{AnswerType, TurnRead};
use crate::modules::exports::{amount_text, money};

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("budget", || fetch(ctx, read, r))
}

struct Line {
    id: String,
    category: String,
    description: String,
    planned: i64,
    actual: Option<i64>,
}

/// Words that talk about money and name no line.
const MONEY_WORDS: &[&str] = &[
    "budget",
    "budgets",
    "planned",
    "actual",
    "difference",
    "spent",
    "spend",
    "money",
    "total",
    "overall",
    "amount",
    "allocated",
    "payment",
    "investment",
    "estimate",
    "expenses",
    "expense",
    "against",
    "versus",
    "compare",
    "much",
    "cost",
    "costs",
    "paid",
    "under",
    "over",
    "plan",
    "lines",
    "line",
    "anywhere",
    "their",
    "there",
    "where",
    "which",
];

const FILLER: &[&str] = &[
    "the", "a", "an", "of", "for", "on", "our", "we", "is", "are", "what", "how", "on", "in",
    "and", "or", "to", "it", "me", "my", "did", "do", "does", "was", "were", "so", "far", "all",
    "please", "give", "show", "tell", "yet",
];

/// The lines a message names: a run of two words of the message inside the line's name, or one
/// long distinctive word ("camera", "contingency"), or a whole category name.
fn matching<'a>(m: &Msg, lines: &'a [Line], named: &[String]) -> Vec<&'a Line> {
    let words: Vec<&str> = m.words().filter(|w| !FILLER.contains(w)).collect();
    let mut out: Vec<&Line> = lines.iter().filter(|l| named.contains(&l.id)).collect();
    for l in lines {
        if out.iter().any(|o| o.id == l.id) {
            continue;
        }
        let d = normalize(&l.description);
        let padded = format!(" {d} ");
        let pair = words
            .windows(2)
            .any(|w| padded.contains(&format!(" {} {} ", w[0], w[1])));
        let single = words.iter().any(|w| {
            w.chars().count() >= 6 && !MONEY_WORDS.contains(w) && padded.contains(&format!(" {w} "))
        });
        if pair || single {
            out.push(l);
        }
    }
    if out.is_empty() {
        for l in lines {
            let c = normalize(&l.category);
            if !c.is_empty() && m.has(&c) {
                out.push(l);
            }
        }
    }
    out
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let budget: Option<(String, String, i64, String, i64)> = ctx
        .conn
        .query_row(
            "SELECT id, currency, COALESCE(planned_total, -1), contingency_mode, contingency_value FROM budget_snapshot
             WHERE deleted_at IS NULL ORDER BY is_current DESC, created_at DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .ok();
    let Some((budget_id, currency, planned_total, mode, value)) = budget else {
        return Ok(vec![Fact::missing("no budget entered in this project")]);
    };
    let mut st = ctx.conn.prepare(
        "SELECT id, category, description, amount, actual FROM budget_line
         WHERE budget_id = ?1 AND deleted_at IS NULL ORDER BY category, position LIMIT 400",
    )?;
    let lines: Vec<Line> = st
        .query_map([&budget_id], |r| {
            Ok(Line {
                id: r.get(0)?,
                category: r.get(1)?,
                description: r.get(2)?,
                planned: r.get(3)?,
                actual: r.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    if lines.is_empty() {
        return Ok(vec![Fact::missing(
            "no budget lines entered in this project",
        )]);
    }
    let mny = |v: i64| money(v, &currency);
    let plain = |v: i64| amount_text(v, &currency, true);
    let diff_text = |d: i64| match d {
        0 => format!("{} (on plan)", mny(0)),
        d if d > 0 => format!("{} (under plan)", mny(d)),
        d => format!("{} (over plan)", mny(-d)),
    };

    // ---- which fields are asked
    let all_three = t == AnswerType::Compare
        || (m.any(&["planned", "plan"])
            && m.any(&["actual", "spent"])
            && m.any(&["difference", "diff"]));
    let asks_diff = all_three
        || m.any(&[
            "difference",
            "over",
            "under",
            "remaining",
            "left",
            "vs",
            "versus",
            "against",
            "compared",
            "compare",
        ]);
    let asks_actual = all_three
        || asks_diff
        || m.any(&[
            "actual",
            "spent",
            "spend",
            "cost",
            "costs",
            "paid",
            "expense",
            "expenses",
            "so far",
            "investment",
        ]);
    let asks_planned = all_three
        || asks_diff
        || m.any(&[
            "planned",
            "plan",
            "budget",
            "budgeted",
            "allocated",
            "allocation",
            "estimate",
            "estimated",
            "investment",
        ])
        || !asks_actual;
    let names: Vec<String> = r.budget_lines.iter().map(|h| h.id.clone()).collect();
    let hit = matching(m, &lines, &names);
    let mut out: Vec<Fact> = Vec::new();

    // The budget is kept by line, never by scene.
    if hit.is_empty()
        && let Some(s) = r.scenes.first()
        && m.any(&["cost", "costs", "budget", "spend", "price", "expensive"])
    {
        let n = s.ordinal;
        return Ok(vec![Fact::missing(format!(
            "no budget for scene {n}: the budget is kept by category and line, not by scene"
        ))]);
    }

    // ---- lines with no actual yet
    let no_actual_q =
        m.any(&[
            "no actual",
            "not recorded",
            "without actual",
            "no actuals",
            "missing actual",
        ]) || (m.has("actual") && m.any(&["yet", "missing", "recorded"]) && hit.is_empty());
    if no_actual_q {
        for l in lines.iter().filter(|l| l.actual.is_none()) {
            out.push(
                Fact::new("No actual yet", &l.id)
                    .f("line", &l.description)
                    .f("category", &l.category)
                    .f("planned", mny(l.planned))
                    .require(&l.description)
                    .prio(P_HIGH),
            );
        }
    }
    // ---- lines over plan
    let over_q = m.any(&["over budget", "overspent", "overspend", "over plan", "over"])
        && hit.is_empty()
        && !no_actual_q;
    if over_q {
        for l in lines
            .iter()
            .filter(|l| l.actual.is_some_and(|a| a > l.planned))
        {
            let a = l.actual.unwrap_or_default();
            out.push(
                Fact::new("Over plan", &l.id)
                    .f("line", &l.description)
                    .f("planned", mny(l.planned))
                    .f("actual", mny(a))
                    .f("over by", mny(a - l.planned))
                    .require(&l.description)
                    .prio(P_HIGH),
            );
        }
    }

    // ---- named lines
    for l in &hit {
        let mut f = Fact::new("Budget line", &l.id)
            .f("line", &l.description)
            .f("category", &l.category)
            .prio(P_HIGH);
        if asks_planned {
            f = f.f("planned", mny(l.planned)).require(plain(l.planned));
        }
        match l.actual {
            Some(a) => {
                if asks_actual {
                    f = f.f("actual", mny(a)).require(plain(a));
                }
                if asks_diff || all_three {
                    f = f
                        .f(
                            "difference (planned minus actual)",
                            diff_text(l.planned - a),
                        )
                        .require(plain((l.planned - a).abs()));
                }
            }
            None => {
                f = f.f("actual", "none entered");
                if asks_actual {
                    out.push(Fact::missing(format!(
                        "budget line {}: no actual amount entered",
                        l.description
                    )));
                }
            }
        }
        out.push(f);
    }

    // ---- the whole budget
    let asks_total = hit.is_empty()
        && !no_actual_q
        && !over_q
        && (m.any(&[
            "total",
            "overall",
            "all lines",
            "whole",
            "so far",
            "everything",
            "all",
        ]) || t == AnswerType::Compare
            || t == AnswerType::Lookup
            || t == AnswerType::Count);
    if asks_total || (hit.is_empty() && out.is_empty()) {
        let sum: i64 = lines.iter().map(|l| l.planned).sum();
        let with_actual: Vec<&Line> = lines.iter().filter(|l| l.actual.is_some()).collect();
        let actual: i64 = with_actual.iter().filter_map(|l| l.actual).sum();
        let planned_with_actual: i64 = with_actual.iter().map(|l| l.planned).sum();
        let planned = if planned_total >= 0 {
            planned_total
        } else {
            sum
        };
        let mut f = Fact::new("Budget total", &budget_id)
            .f("lines", lines.len().to_string())
            .prio(P_HIGH);
        if asks_planned || all_three || hit.is_empty() {
            f = f.f("planned total", mny(planned)).require(plain(planned));
            if planned != sum {
                f = f.f("planned in lines", mny(sum));
            }
        }
        if asks_actual || all_three || m.has("budget") {
            f = f
                .f("actual total", mny(actual))
                .f(
                    "lines with an actual",
                    format!("{} of {}", with_actual.len(), lines.len()),
                )
                .require(plain(actual));
        }
        if asks_diff || all_three {
            f = f
                .f(
                    "difference (planned minus actual, lines with an actual only)",
                    diff_text(planned_with_actual - actual),
                )
                .require(plain((planned_with_actual - actual).abs()));
        }
        out.push(f);
    }

    // ---- contingency setting
    if m.stem("contingenc") {
        let v = if mode == "percent" {
            format!("{}.{:02}% of the planned total", value / 100, value % 100)
        } else {
            mny(value)
        };
        out.push(
            Fact::new("Contingency setting", &budget_id)
                .f("amount", v)
                .prio(P_MID),
        );
    }
    Ok(out)
}

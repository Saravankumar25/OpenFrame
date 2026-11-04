//! Production > Reports: the six reports of the Reports screen (scene, location, cast scene, prop,
//! schedule and breakdown completeness), read through `docs::build_report`, the function the
//! screen and the export call. A report is a projection of data held elsewhere; the assistant gets
//! it as the page shows it.

use straft_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::turn::TurnRead;
use crate::modules::schedule::docs::{REPORT_TYPES, build_report};

/// Rows of a named report given to the writer, and the characters of them.
const ROWS: usize = 40;
const ROW_CHARS: usize = 2_600;

/// The words of a question that name each report.
const NAMES: &[(&str, &[&str])] = &[
    ("scene", &["scene report"]),
    ("location", &["location report", "locations report"]),
    (
        "cast_scene",
        &["cast scene report", "cast report", "cast scene"],
    ),
    ("prop", &["prop report", "props report"]),
    ("schedule", &["schedule report"]),
    (
        "breakdown_completeness",
        &[
            "completeness",
            "breakdown completeness",
            "completeness report",
        ],
    ),
];

/// Production > Reports: the report the question names, else a line for each report.
pub fn reports_leaf(ctx: &FetchCtx<'_>, _read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("reports", || reports(ctx))
}

fn reports(ctx: &FetchCtx<'_>) -> AppResult<Vec<Fact>> {
    let asked: Vec<&str> = NAMES
        .iter()
        .filter(|(_, words)| ctx.m.any(words))
        .map(|(t, _)| *t)
        .collect();
    let mut out = Vec::new();
    for (kind, title) in REPORT_TYPES.iter() {
        let named = asked.contains(kind);
        let data = build_report(ctx.conn, kind, None)?;
        let mut f = Fact::new("Report", *kind)
            .f("report", *title)
            .f("rows", data.rows.len().to_string())
            .f("columns", data.columns.join(", "))
            .f("note", data.note.clone().unwrap_or_default())
            .prio(if named { P_HIGH } else { P_MID });
        if named {
            // The rows that hold a word or a number of the question come first; the rest follow in order.
            let words: Vec<String> = ctx
                .m
                .words()
                .filter(|w| w.chars().all(char::is_numeric) || w.chars().count() >= 4)
                .map(str::to_string)
                .collect();
            let hits = |row: &Vec<String>| {
                let line = row.join(" ").to_lowercase();
                words
                    .iter()
                    .filter(|w| line.split(' ').any(|c| c == w.as_str()))
                    .count()
            };
            let mut ranked: Vec<&Vec<String>> = data.rows.iter().collect();
            ranked.sort_by_key(|r| std::cmp::Reverse(hits(r)));
            let shown: Vec<String> = ranked.iter().take(ROWS).map(|r| r.join(" / ")).collect();
            f = f.f("rows shown", clip(&shown.join("; "), ROW_CHARS));
            f = f.require(data.rows.len().to_string());
        }
        out.push(f);
    }
    Ok(out)
}

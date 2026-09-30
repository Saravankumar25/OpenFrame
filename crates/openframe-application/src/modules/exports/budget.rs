//! Budget summary export (FSD §39, §109.6 "A simple budget summary can be
//! exported as PDF/CSV-style data"): planned total, contingency, category
//! totals and line items of the current budget or of a frozen snapshot.
//! Amounts are exported exactly as entered; nothing is recalculated beyond the
//! same category/contingency totals the Budget view shows.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{Block, Column, Document, Table};
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use ts_rs::TS;

use super::{
    Fmt, HINT_TABLES, Job, Output, amount_text, date_from_ms, deliver, destination, footer, money,
    nonblank, plural, require_export, subtitle,
};
use crate::core::AppCore;
use crate::modules::schedule::board::project_title;
use crate::modules::schedule::budget::BUDGET_CATEGORIES;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportBudgetArgs {
    /// "pdf" | "csv" | "xlsx"
    pub format: String,
    pub path: String,
    /// A frozen budget snapshot (default: the current budget).
    #[serde(default)]
    #[ts(optional)]
    pub budget_id: Option<String>,
    /// Line notes column. Default false.
    #[serde(default)]
    #[ts(optional)]
    pub include_notes: Option<bool>,
}

struct Budget {
    label: Option<String>,
    currency: String,
    planned: Option<i64>,
    mode: String,
    value: i64,
    notes: Option<String>,
    is_current: bool,
    frozen_at: Option<i64>,
    lines: Vec<(String, String, i64, Option<String>)>,
}

fn load(c: &Connection, id: Option<&str>) -> AppResult<Budget> {
    let id: String = match id {
        Some(i) => i.to_string(),
        None => c
            .query_row(
                "SELECT id FROM budget_snapshot WHERE is_current = 1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::export("nothing_to_export", "There is no budget to export yet."))?,
    };
    let row: Option<(Option<String>, String, Option<i64>, String, i64, Option<String>, i64, Option<i64>)> = c
        .query_row(
            "SELECT label, currency, planned_total, contingency_mode, contingency_value, notes, is_current, frozen_at
             FROM budget_snapshot WHERE id = ?1 AND deleted_at IS NULL",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
        )
        .optional()?;
    let (label, currency, planned, mode, value, notes, is_current, frozen_at) =
        row.ok_or_else(|| AppError::not_found("budget"))?;
    let mut st = c.prepare(
        "SELECT category, description, amount, notes FROM budget_line
         WHERE budget_id = ?1 AND deleted_at IS NULL ORDER BY position, created_at, id",
    )?;
    let mut lines: Vec<(String, String, i64, Option<String>)> = st
        .query_map([&id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    lines.sort_by_key(|l| {
        BUDGET_CATEGORIES
            .iter()
            .position(|c| *c == l.0)
            .unwrap_or(usize::MAX)
    });
    Ok(Budget {
        label,
        currency,
        planned,
        mode,
        value,
        notes,
        is_current: is_current != 0,
        frozen_at,
        lines,
    })
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportBudgetArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf, Fmt::Csv, Fmt::Xlsx])?;
    if let Some(id) = &a.budget_id {
        crate::util::require_id(id, "budget")?;
    }
    let notes = a.include_notes.unwrap_or(false);
    let dest = destination(core, &a.path, fmt)?;
    let session = core.project()?;
    let (project, b) = session
        .store
        .read(|c| Ok((project_title(c)?, load(c, a.budget_id.as_deref())?)))?;
    let cur = b.currency.as_str();
    let entered: i64 = b.lines.iter().map(|l| l.2).sum();
    let contingency = if b.mode == "percent" {
        let base = b.planned.unwrap_or(entered) as i128;
        ((base * b.value as i128 + 5_000) / 10_000) as i64
    } else {
        b.value
    };
    let document = if b.is_current {
        "Budget Summary".to_string()
    } else {
        format!(
            "Budget Snapshot — {}",
            b.label
                .clone()
                .or_else(|| b.frozen_at.map(date_from_ms))
                .unwrap_or_else(|| "saved".into())
        )
    };

    let mut doc = Document::new(&document)
        .subtitle(subtitle(&project, None))
        .footer(footer(&project, &document));
    doc.push(Block::Note(
        "A simple estimate for planning. Formal accounting is outside OpenFrame.".into(),
    ));
    let mut facts = vec![
        ("Currency".to_string(), cur.to_string()),
        (
            "Planned total".into(),
            b.planned
                .map(|p| money(p, cur))
                .unwrap_or_else(|| "Not set".into()),
        ),
        (
            "Contingency".into(),
            if b.mode == "percent" {
                format!(
                    "{}% · {}",
                    amount_text(b.value, "PCT", true),
                    money(contingency, cur)
                )
            } else {
                money(contingency, cur)
            },
        ),
        ("Entered so far".into(), money(entered, cur)),
    ];
    if let Some(p) = b.planned {
        let diff = p - entered;
        facts.push((
            if diff >= 0 {
                "Remaining in the plan".into()
            } else {
                "Over the plan".into()
            },
            money(diff.abs(), cur),
        ));
    }
    if let Some(t) = b.frozen_at.filter(|_| !b.is_current) {
        facts.push(("Snapshot saved".into(), date_from_ms(t)));
    }
    doc.push(Block::KeyValues(facts));

    let mut cats = Table::new(
        "Categories",
        vec![
            Column::new("Category").weight(2.0),
            Column::right("Lines").weight(0.5),
            Column::right(format!("Total ({cur})")).weight(1.0),
        ],
    );
    for cat in BUDGET_CATEGORIES {
        let ls: Vec<_> = b.lines.iter().filter(|l| l.0 == cat).collect();
        if ls.is_empty() {
            continue;
        }
        cats.push(vec![
            cat.to_string(),
            ls.len().to_string(),
            amount_text(ls.iter().map(|l| l.2).sum(), cur, false),
        ]);
    }
    // Lines in categories the app no longer lists are still exported.
    let others: Vec<_> = b
        .lines
        .iter()
        .filter(|l| !BUDGET_CATEGORIES.contains(&l.0.as_str()))
        .collect();
    if !others.is_empty() {
        cats.push(vec![
            "Other".into(),
            others.len().to_string(),
            amount_text(others.iter().map(|l| l.2).sum(), cur, false),
        ]);
    }
    let mut cols = vec![
        Column::new("Category"),
        Column::new("Line item").weight(2.0),
        Column::right(format!("Amount ({cur})")),
    ];
    if notes {
        cols.push(Column::new("Notes").weight(1.5));
    }
    let mut lines = Table::new("Line items", cols);
    for (cat, desc, amount, n) in &b.lines {
        let mut r = vec![cat.clone(), desc.clone(), amount_text(*amount, cur, false)];
        if notes {
            r.push(n.clone().unwrap_or_default());
        }
        lines.push(r);
    }
    // PDF tables show grouped amounts; spreadsheets keep plain numbers.
    let grouped = |t: &Table, col: usize| {
        let mut g = t.clone();
        g.name = String::new();
        for r in g.rows.iter_mut() {
            if let Some(v) = r.get_mut(col)
                && let Ok(minor) = v.replace('.', "").parse::<i64>()
            {
                *v = amount_text(minor, cur, true);
            }
        }
        g
    };
    doc.push(Block::Heading {
        text: "By category".into(),
        level: 2,
    });
    if cats.rows.is_empty() {
        doc.push(Block::Paragraph("No lines yet.".into()));
    } else {
        doc.push(Block::Table(grouped(&cats, 2)));
        doc.push(Block::Heading {
            text: "Line items".into(),
            level: 2,
        });
        doc.push(Block::Table(grouped(&lines, 2)));
    }
    if let Some(n) = nonblank(b.notes.as_deref()) {
        doc.push(Block::Heading {
            text: "Notes".into(),
            level: 2,
        });
        doc.push(Block::Paragraph(n));
    }
    let mut summary = Table::new(
        "Summary",
        vec![
            Column::new("Item"),
            Column::right(format!("Amount ({cur})")),
        ],
    );
    summary.push(vec![
        "Planned total".into(),
        b.planned
            .map(|p| amount_text(p, cur, false))
            .unwrap_or_default(),
    ]);
    summary.push(vec![
        "Contingency".into(),
        amount_text(contingency, cur, false),
    ]);
    summary.push(vec![
        "Entered so far".into(),
        amount_text(entered, cur, false),
    ]);

    let mut out = Output::new(doc, HINT_TABLES);
    out.tables = vec![lines, cats, summary];
    let job = Job {
        action: "budget.export_summary",
        document: document.clone(),
        source_label: if b.is_current {
            "Current budget".into()
        } else {
            document
        },
        scope_label: "Entire budget".into(),
        contents_label: plural(b.lines.len(), "line", "lines"),
        target: None,
    };
    deliver(core, actor, &dest, fmt, job, out)
}

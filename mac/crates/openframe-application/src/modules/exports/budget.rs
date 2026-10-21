//! Budget report export (FSD §39, §109.6 "A simple budget summary can be exported as PDF/CSV-style
//! data"; production depth spec §33): planned total, contingency, category totals with Planned,
//! Actual and difference, and the line items of the current budget or of a frozen snapshot.
//! Amounts are exported exactly as entered; nothing is recalculated beyond the same totals the
//! Budget view shows. A line without an Actual amount leaves the Actual and Difference cells
//! empty and never counts as "under plan".

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, Block, Callout, CalloutCol, Cell, CellItem, Col, Column, Document, HeaderStyle,
    Masthead, Metric, Metrics, Rgb, Row, Section, Side, StyledTable, Table, Tone, Txt, pal,
};
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use ts_rs::TS;

use super::style::nb;
use super::{
    Fmt, HINT_TABLES, Job, Output, amount_text, date_from_ms, deliver, destination, footer, money,
    nonblank, plural, require_export,
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

struct Line {
    category: String,
    description: String,
    planned: i64,
    actual: Option<i64>,
    notes: Option<String>,
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
    lines: Vec<Line>,
}

/// Planned, Actual (entered only) and difference over lines that have an Actual amount.
#[derive(Default)]
struct Sum {
    planned: i64,
    actual: i64,
    difference: i64,
    actual_count: usize,
}

fn sum<'a>(lines: impl Iterator<Item = &'a Line>) -> Sum {
    let mut s = Sum::default();
    for l in lines {
        s.planned = s.planned.saturating_add(l.planned);
        if let Some(a) = l.actual {
            s.actual = s.actual.saturating_add(a);
            s.difference = s.difference.saturating_add(l.planned - a);
            s.actual_count += 1;
        }
    }
    s
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
    #[allow(clippy::type_complexity)]
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
        "SELECT category, description, amount, actual, notes FROM budget_line
         WHERE budget_id = ?1 AND deleted_at IS NULL ORDER BY position, created_at, id",
    )?;
    let mut lines: Vec<Line> = st
        .query_map([&id], |r| {
            Ok(Line {
                category: r.get(0)?,
                description: r.get(1)?,
                planned: r.get(2)?,
                actual: r.get(3)?,
                notes: r.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    lines.sort_by_key(|l| {
        BUDGET_CATEGORIES
            .iter()
            .position(|c| *c == l.category)
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
    let (out, job) = render(&project, &b, notes);
    deliver(core, actor, &dest, fmt, job, out)
}

/// Accent colour of a budget category (dot and share bar).
fn budget_color(name: &str) -> Rgb {
    Rgb::hex(match name {
        "Cast" => 0x7a4cc2,
        "Crew" => 0x1f9a8a,
        "Locations" => 0x1b84be,
        "Equipment" => 0x6b7a8f,
        "Art/Props" => 0xc0623a,
        "Travel/Transport" => 0xd99a1a,
        "Food" => 0x4f9a3d,
        "Post/Other" => 0xc23a96,
        _ => 0x8792a5,
    })
}

fn render(project: &str, b: &Budget, notes: bool) -> (Output, Job) {
    let cur = b.currency.as_str();
    let all = sum(b.lines.iter());
    let contingency = if b.mode == "percent" {
        let base = b.planned.unwrap_or(all.planned) as i128;
        ((base * b.value as i128 + 5_000) / 10_000) as i64
    } else {
        b.value
    };
    let document = if b.is_current {
        "Budget Report".to_string()
    } else {
        format!(
            "Budget Snapshot: {}",
            b.label
                .clone()
                .or_else(|| b.frozen_at.map(date_from_ms))
                .unwrap_or_else(|| "saved".into())
        )
    };
    let plain = |v: i64| amount_text(v, cur, false);
    let opt_plain = |v: Option<i64>| v.map(plain).unwrap_or_default();

    let mut doc = Document::new(&document).footer(footer(project, &document));
    let mut mast = Masthead::new(document.clone()).eyebrow("Production · Budget", pal::ORANGE);
    if let Some(p) = nb(project) {
        mast.subtitle = Some(p.to_string());
    }
    mast.lines
        .push("A simple estimate for planning. Formal accounting is outside OpenFrame.".into());
    let mut side = vec![
        (format!("Currency {cur}"), true),
        (
            if b.is_current {
                "Current budget".to_string()
            } else {
                "Saved snapshot".to_string()
            },
            false,
        ),
    ];
    if let Some(t) = b.frozen_at.filter(|_| !b.is_current) {
        side.push((format!("Saved {}", date_from_ms(t)), false));
    }
    side.push((format!("Exported {}", super::today_label()), false));
    mast.side = Some(Side::Panel { lines: side });
    mast.rule = true;
    doc.push(Block::Masthead(mast));
    let under = all.difference >= 0;
    doc.push(Block::Metrics(Metrics {
        items: vec![
            Metric::new(
                b.planned
                    .map(|p| money(p, cur))
                    .unwrap_or_else(|| "Not set".into()),
                "Planned total",
            ),
            Metric::new(
                if b.mode == "percent" {
                    format!(
                        "{}% · {}",
                        amount_text(b.value, "PCT", true),
                        money(contingency, cur)
                    )
                } else {
                    money(contingency, cur)
                },
                "Contingency",
            ),
            Metric::new(
                money(all.actual, cur),
                format!(
                    "Actual so far, {} of {}",
                    plural(all.actual_count, "line", "lines"),
                    b.lines.len()
                ),
            ),
            Metric::new(
                money(all.difference.abs(), cur),
                if under {
                    "Under plan, lines with an actual"
                } else {
                    "Over plan, lines with an actual"
                },
            )
            .tone(if under { Tone::Ok } else { Tone::Danger }),
        ],
        compact: false,
    }));
    let mut notices: Vec<String> = vec![format!("Planned in lines: {}.", money(all.planned, cur))];
    if let Some(p) = b.planned {
        let diff = p - all.planned;
        notices.push(if diff >= 0 {
            format!(
                "Planned total not yet in lines: {}.",
                money(diff.abs(), cur)
            )
        } else {
            format!("Lines over the planned total: {}.", money(diff.abs(), cur))
        });
    }
    doc.push(Block::Text(
        Txt::new(notices.join(" "))
            .size(8.6)
            .color(pal::SLATE)
            .after(4.0),
    ));

    let money_cols = |first: Vec<Column>| {
        let mut v = first;
        v.push(Column::right(format!("Planned ({cur})")));
        v.push(Column::right(format!("Actual ({cur})")));
        v.push(Column::right(format!("Difference ({cur})")));
        v
    };
    let mut cats = Table::new(
        "Categories",
        money_cols(vec![
            Column::new("Category").weight(2.0),
            Column::right("Lines").weight(0.5),
        ]),
    );
    let mut cat_row = |name: &str, ls: Vec<&Line>| {
        if ls.is_empty() {
            return;
        }
        let s = sum(ls.iter().copied());
        cats.push(vec![
            name.to_string(),
            ls.len().to_string(),
            plain(s.planned),
            if s.actual_count > 0 {
                plain(s.actual)
            } else {
                String::new()
            },
            if s.actual_count > 0 {
                plain(s.difference)
            } else {
                String::new()
            },
        ]);
    };
    for cat in BUDGET_CATEGORIES {
        cat_row(cat, b.lines.iter().filter(|l| l.category == cat).collect());
    }
    // Lines in categories the app no longer lists are still exported.
    cat_row(
        "Other",
        b.lines
            .iter()
            .filter(|l| !BUDGET_CATEGORIES.contains(&l.category.as_str()))
            .collect(),
    );
    let mut cols = money_cols(vec![
        Column::new("Category"),
        Column::new("Line item").weight(2.0),
    ]);
    if notes {
        cols.push(Column::new("Notes").weight(1.5));
    }
    let mut lines = Table::new("Line items", cols);
    for l in &b.lines {
        let mut r = vec![
            l.category.clone(),
            l.description.clone(),
            plain(l.planned),
            opt_plain(l.actual),
            opt_plain(l.actual.map(|a| l.planned - a)),
        ];
        if notes {
            r.push(l.notes.clone().unwrap_or_default());
        }
        lines.push(r);
    }
    // PDF tables show grouped amounts; spreadsheets keep plain numbers.
    let grouped = |v: i64| amount_text(v, cur, true);
    let signed = |v: i64| -> Cell {
        let (color, bold) = if v < 0 {
            (pal::RED_TEXT, true)
        } else {
            (pal::INK, false)
        };
        let mut t = Txt::new(grouped(v))
            .size(8.8)
            .color(color)
            .align(Align::Right);
        if bold {
            t = t.bold();
        }
        Cell::text(t)
    };
    let amount = |v: i64, bold: bool| -> Cell {
        let mut t = Txt::new(grouped(v))
            .size(8.8)
            .color(pal::INK)
            .align(Align::Right);
        if bold {
            t = t.bold().color(pal::NAVY);
        }
        Cell::text(t)
    };
    doc.push(Block::Section(Section::new("By category", None)));
    if cats.rows.is_empty() {
        doc.push(Block::Text(
            Txt::new("No lines yet.")
                .size(8.8)
                .color(pal::FAINT)
                .after(6.0),
        ));
    } else {
        let mut names: Vec<String> = BUDGET_CATEGORIES.iter().map(|c| c.to_string()).collect();
        let other: Vec<&Line> = b
            .lines
            .iter()
            .filter(|l| !BUDGET_CATEGORIES.contains(&l.category.as_str()))
            .collect();
        if !other.is_empty() {
            names.push("Other".into());
        }
        let in_cat = |name: &str| -> Vec<&Line> {
            if name == "Other" {
                b.lines
                    .iter()
                    .filter(|l| !BUDGET_CATEGORIES.contains(&l.category.as_str()))
                    .collect()
            } else {
                b.lines.iter().filter(|l| l.category == name).collect()
            }
        };
        let mut t = StyledTable::new(vec![
            Col::weight("Category", 2.0),
            Col::pt("Lines", 40.0).align(Align::Right),
            Col::pt(format!("Planned ({cur})"), 86.0).align(Align::Right),
            Col::pt(format!("Actual ({cur})"), 86.0).align(Align::Right),
            Col::pt(format!("Difference ({cur})"), 92.0).align(Align::Right),
            Col::weight("Share of planned", 1.0),
        ]);
        t.boxed = true;
        t.header = HeaderStyle::Tint;
        t.continued = Some("By category".into());
        let planned_total = all.planned.max(1);
        for name in &names {
            let ls = in_cat(name);
            if ls.is_empty() {
                continue;
            }
            let s = sum(ls.iter().copied());
            let has_actual = s.actual_count > 0;
            t.rows.push(Row::new(vec![
                Cell::new().item(CellItem::Heading {
                    dot: Some(budget_color(name)),
                    text: name.clone(),
                    right: None,
                }),
                Cell::text(
                    Txt::new(ls.len().to_string())
                        .size(8.8)
                        .color(pal::SLATE)
                        .align(Align::Right),
                ),
                amount(s.planned, true),
                if has_actual {
                    amount(s.actual, false)
                } else {
                    Cell::empty()
                },
                if has_actual {
                    signed(s.difference)
                } else {
                    Cell::empty()
                },
                Cell::new()
                    .item(CellItem::Bar {
                        share: s.planned as f32 / planned_total as f32,
                        color: budget_color(name),
                    })
                    .middle(),
            ]));
        }
        let mut total = Row::new(vec![
            Cell::text(
                Txt::new("Total in lines")
                    .size(9.0)
                    .bold()
                    .color(pal::WHITE),
            ),
            Cell::text(
                Txt::new(b.lines.len().to_string())
                    .size(8.8)
                    .bold()
                    .color(pal::WHITE)
                    .align(Align::Right),
            ),
            Cell::text(
                Txt::new(grouped(all.planned))
                    .size(8.8)
                    .bold()
                    .color(pal::WHITE)
                    .align(Align::Right),
            ),
            if all.actual_count > 0 {
                Cell::text(
                    Txt::new(grouped(all.actual))
                        .size(8.8)
                        .bold()
                        .color(pal::WHITE)
                        .align(Align::Right),
                )
            } else {
                Cell::empty()
            },
            if all.actual_count > 0 {
                Cell::text(
                    Txt::new(grouped(all.difference))
                        .size(8.8)
                        .bold()
                        .color(pal::WHITE)
                        .align(Align::Right),
                )
            } else {
                Cell::empty()
            },
            Cell::empty(),
        ]);
        total.fill = Some(pal::NAVY);
        t.rows.push(total);
        doc.push(Block::Styled(t));

        doc.push(Block::Section(Section::new(
            "Line items",
            Some(plural(b.lines.len(), "line", "lines")),
        )));
        let mut li = StyledTable::new(vec![
            Col::weight("Line item", 2.6),
            Col::pt(format!("Planned ({cur})"), 90.0).align(Align::Right),
            Col::pt(format!("Actual ({cur})"), 90.0).align(Align::Right),
            Col::pt(format!("Difference ({cur})"), 96.0).align(Align::Right),
        ]);
        li.boxed = true;
        li.header = HeaderStyle::Tint;
        li.continued = Some("Line items".into());
        for name in &names {
            let ls = in_cat(name);
            if ls.is_empty() {
                continue;
            }
            let mut band = Row::spanning(Cell::new().item(CellItem::Heading {
                dot: Some(budget_color(name)),
                text: name.clone(),
                right: Some(plural(ls.len(), "line", "lines")),
            }));
            band.fill = Some(pal::PANEL);
            band.keep_next = true;
            li.rows.push(band);
            for l in ls {
                let mut desc = Cell::text(
                    Txt::new(l.description.clone())
                        .size(9.0)
                        .bold()
                        .color(pal::NAVY)
                        .leading(1.25),
                );
                if notes && let Some(n) = nonblank(l.notes.as_deref()) {
                    desc = desc.item(CellItem::Text(
                        Txt::new(n).size(8.2).color(pal::SLATE).leading(1.3),
                    ));
                }
                li.rows.push(Row::new(vec![
                    desc,
                    amount(l.planned, false),
                    match l.actual {
                        Some(a) => amount(a, false),
                        None => Cell::empty(),
                    },
                    match l.actual {
                        Some(a) => signed(l.planned - a),
                        None => Cell::empty(),
                    },
                ]));
            }
        }
        doc.push(Block::Styled(li));
    }
    if let Some(n) = nonblank(b.notes.as_deref()) {
        doc.push(Block::Section(Section::new("Notes", None)));
        doc.push(Block::Callout(Callout {
            tone: Tone::Info,
            cols: vec![{
                let mut c = CalloutCol::new(1.0);
                c.lines
                    .push(Txt::new(n).size(9.0).color(Tone::Info.text()).leading(1.4));
                c
            }],
        }));
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
        b.planned.map(plain).unwrap_or_default(),
    ]);
    summary.push(vec!["Contingency".into(), plain(contingency)]);
    summary.push(vec!["Planned in lines".into(), plain(all.planned)]);
    summary.push(vec!["Actual so far".into(), plain(all.actual)]);
    summary.push(vec![
        "Difference (lines with an actual)".into(),
        plain(all.difference),
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
    (out, job)
}

//! Budget report export (FSD §39, §109.6 "A simple budget summary can be exported as PDF/CSV-style
//! data"; production depth spec §33): planned total, contingency, category totals with Planned,
//! Actual and difference, and the line items of the current budget or of a frozen snapshot.
//! Amounts are exported exactly as entered; nothing is recalculated beyond the same totals the
//! Budget view shows. A line without an Actual amount leaves the Actual and Difference cells
//! empty and never counts as "under plan".

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, Block, Callout, CalloutCol, Cell, CellItem, Chip, Col, Column, Document, HeaderStyle,
    Masthead, Metric, Metrics, Rgb, Row, Section, Side, StyledTable, Table, Tone, Txt, pal,
    text_width,
};
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use ts_rs::TS;

use super::style::nb;
use super::{
    Fmt, HINT_TABLES, Job, Output, amount_text, date_from_ms, deliver, destination, money,
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

/// Secondary text that passes 4.5:1 on white (the palette's FAINT is decoration only).
const MUTED: Rgb = Rgb::hex(0x66728a);
/// What prints where a line has no Actual amount (an en dash: the writing standard has no em dash).
const NONE: &str = "\u{2013}";

/// "LOW TIDE RADIO · BUDGET · INR": names the production and the currency on every page.
fn running_header(project: &str, b: &Budget) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(p) = nb(project) {
        parts.push(p.to_uppercase());
    }
    parts.push(if b.is_current {
        "BUDGET".into()
    } else {
        "BUDGET SNAPSHOT".into()
    });
    parts.push(b.currency.to_uppercase());
    parts.join(" \u{b7} ")
}

/// Amount cells of one document. Amounts are right aligned; when any amount of the document
/// prints decimals, whole amounts keep the room of ".00" so the digits line up.
struct Amounts<'a> {
    cur: &'a str,
    decimals: bool,
}

impl Amounts<'_> {
    fn text(&self, v: i64) -> String {
        amount_text(v, self.cur, true)
    }
    fn txt(&self, v: i64, size: f32, bold: bool, color: Rgb) -> Txt {
        let t = Txt::new(self.text(v))
            .size(size)
            .color(color)
            .align(Align::Right);
        if bold { t.bold() } else { t }
    }
    fn inset(&self, v: i64, size: f32, bold: bool) -> f32 {
        if self.decimals && !self.text(v).contains('.') {
            text_width(".00", size, bold)
        } else {
            0.0
        }
    }
    fn cell(&self, v: i64, size: f32, bold: bool, color: Rgb) -> Cell {
        Cell::text(self.txt(v, size, bold, color)).inset_right(self.inset(v, size, bold))
    }
    /// A difference: a minus sign marks over plan, and a line also gets a shaded cell and the
    /// words OVER PLAN (the sign alone would not survive a grayscale print).
    fn diff(&self, v: i64, size: f32, bold: bool, flag: bool) -> Cell {
        if v >= 0 {
            return self.cell(v, size, bold, pal::INK);
        }
        let mut c = Cell::text(self.txt(v, size, true, pal::RED_TEXT))
            .inset_right(self.inset(v, size, true));
        if flag {
            c = c.item(CellItem::Text(
                Txt::new("OVER PLAN")
                    .size(6.6)
                    .bold()
                    .track(0.5)
                    .color(pal::RED_TEXT)
                    .align(Align::Right),
            ));
            c = c.fill(pal::RED_FILL);
        }
        c
    }
    /// No Actual amount entered.
    fn none(&self, size: f32) -> Cell {
        let inset = if self.decimals {
            text_width(".00", size, false)
        } else {
            0.0
        };
        Cell::text(Txt::new(NONE).size(size).color(MUTED).align(Align::Right)).inset_right(inset)
    }
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

    let mut doc = Document::new(&document).running_header(Some(running_header(project, b)));
    let mut mast = Masthead::new(document.clone()).eyebrow("Production \u{b7} Budget", pal::ORANGE);
    mast.title_size = 26.0;
    if let Some(p) = nb(project) {
        mast.subtitle = Some(p.to_string());
    }
    mast.lines
        .push("A simple estimate for planning. Formal accounting is outside OpenFrame.".into());
    let mut side = vec![(format!("Currency {cur}"), true)];
    if b.is_current {
        side.push(("Current budget".to_string(), false));
    } else {
        side.push(("Saved snapshot".to_string(), false));
        if let Some(t) = b.frozen_at {
            side.push((format!("Saved {}", date_from_ms(t)), false));
        }
    }
    side.push((format!("Exported {}", super::today_label()), false));
    mast.side = Some(if b.is_current {
        Side::Panel { lines: side }
    } else {
        Side::Badge {
            chip: Chip::label("Snapshot", Tone::Accent),
            lines: side,
        }
    });
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
                money(contingency, cur),
                if b.mode == "percent" {
                    format!("Contingency, {}%", amount_text(b.value, "PCT", true))
                } else {
                    "Contingency".to_string()
                },
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

    doc.push(Block::Section(Section::new("By category", None)));
    if cats.rows.is_empty() {
        doc.push(Block::Text(
            Txt::new("No lines yet.").size(8.8).color(MUTED).after(6.0),
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
        let names: Vec<String> = names
            .into_iter()
            .filter(|n| !in_cat(n).is_empty())
            .collect();
        // One decimal convention for the whole document: any amount with decimals lines up
        // the whole ones in every table.
        let every_amount = names
            .iter()
            .flat_map(|n| {
                let s = sum(in_cat(n).into_iter());
                [s.planned, s.actual, s.difference]
            })
            .chain(b.lines.iter().flat_map(|l| {
                [Some(l.planned), l.actual, l.actual.map(|a| l.planned - a)]
                    .into_iter()
                    .flatten()
            }))
            .chain([all.planned, all.actual, all.difference]);
        let decimals = every_amount
            .into_iter()
            .any(|v| amount_text(v, cur, true).contains('.'));
        let m = Amounts { cur, decimals };

        // ---- By category
        let mut t = StyledTable::new(vec![
            Col::weight("Category", 1.0),
            Col::pt("Lines", 46.0).align(Align::Right),
            Col::pt("Share of planned", 84.0),
            Col::pt(format!("Planned ({cur})"), 84.0).align(Align::Right),
            Col::pt(format!("Actual ({cur})"), 84.0).align(Align::Right),
            Col::pt(format!("Difference ({cur})"), 92.0).align(Align::Right),
        ]);
        t.boxed = true;
        t.header = HeaderStyle::Tint;
        t.pad_x = 6.0;
        let planned_total = all.planned.max(1);
        for name in &names {
            let ls = in_cat(name);
            let s = sum(ls.iter().copied());
            let has_actual = s.actual_count > 0;
            let mut lines_cell = Cell::text(
                Txt::new(ls.len().to_string())
                    .size(8.8)
                    .color(pal::SLATE)
                    .align(Align::Right),
            );
            if has_actual {
                lines_cell = lines_cell.item(CellItem::Text(
                    Txt::new(format!("{} actual", s.actual_count))
                        .size(6.8)
                        .color(MUTED)
                        .align(Align::Right),
                ));
            }
            t.rows.push(Row::new(vec![
                Cell::new().item(CellItem::Name {
                    dot: Some(budget_color(name)),
                    text: Txt::new(name.clone())
                        .size(9.2)
                        .bold()
                        .color(pal::INK)
                        .leading(1.3),
                    small: None,
                }),
                lines_cell,
                Cell::new()
                    .item(CellItem::Bar {
                        share: (s.planned as f32 / planned_total as f32).clamp(0.0, 1.0),
                        color: pal::SLATE,
                    })
                    .middle(),
                m.cell(s.planned, 8.4, true, pal::NAVY),
                if has_actual {
                    m.cell(s.actual, 8.4, false, pal::INK)
                } else {
                    m.none(8.4)
                },
                if has_actual {
                    m.diff(s.difference, 8.4, false, true)
                } else {
                    m.none(8.4)
                },
            ]));
        }
        let white =
            |v: i64| Cell::text(m.txt(v, 8.8, true, pal::WHITE)).inset_right(m.inset(v, 8.8, true));
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
            Cell::empty(),
            white(all.planned),
            if all.actual_count > 0 {
                white(all.actual)
            } else {
                Cell::text(
                    Txt::new(NONE)
                        .size(8.8)
                        .color(pal::WHITE)
                        .align(Align::Right),
                )
            },
            if all.actual_count > 0 {
                white(all.difference)
            } else {
                Cell::text(
                    Txt::new(NONE)
                        .size(8.8)
                        .color(pal::WHITE)
                        .align(Align::Right),
                )
            },
        ]);
        total.fill = Some(pal::NAVY);
        total.keep_prev = 2;
        t.rows.push(total);
        doc.push(Block::Styled(t));
        doc.push(Block::Text(
            Txt::new(
                "How to read the amounts. Planned adds every line. Actual and Difference add only \
                 the lines that have an Actual amount, so they are not comparable with the whole \
                 Planned column. Difference is Planned minus Actual: a positive amount is under \
                 plan, a minus sign (shaded, with the words OVER PLAN) is over plan. A dash means \
                 no Actual amount has been entered.",
            )
            .size(7.4)
            .color(MUTED)
            .leading(1.35)
            .after(4.0),
        ));

        // ---- Line items: one table per category, its band and column labels in the header
        doc.push(Block::Section(Section::new(
            "Line items",
            Some(plural(b.lines.len(), "line", "lines")),
        )));
        let label = |text: String| {
            Cell::text(
                Txt::new(text.to_uppercase())
                    .size(7.0)
                    .bold()
                    .track(0.8)
                    .color(pal::SLATE),
            )
        };
        let header = |name: &str, count: String| -> Vec<Cell> {
            vec![
                Cell::new().item(CellItem::Name {
                    dot: Some(budget_color(name)),
                    text: Txt::new(name.to_string())
                        .size(10.0)
                        .bold()
                        .color(pal::NAVY)
                        .leading(1.3),
                    small: Some(Txt::new(count).size(7.4).color(pal::SLATE)),
                }),
                label(format!("Planned ({cur})")),
                label(format!("Actual ({cur})")),
                label(format!("Difference ({cur})")),
            ]
        };
        let last_name = names.last().cloned();
        for name in &names {
            let ls = in_cat(name);
            let count = plural(ls.len(), "line", "lines");
            let mut li = StyledTable::new(vec![
                Col::weight("Line item", 1.0),
                Col::pt(format!("Planned ({cur})"), 84.0).align(Align::Right),
                Col::pt(format!("Actual ({cur})"), 84.0).align(Align::Right),
                Col::pt(format!("Difference ({cur})"), 92.0).align(Align::Right),
            ]);
            li.boxed = true;
            li.header = HeaderStyle::Tint;
            li.pad_x = 6.0;
            li.header_cells = Some(header(name, count.clone()));
            li.header_continued = Some(header(name, format!("{count} \u{b7} continued")));
            for l in &ls {
                let mut desc = Cell::text(
                    Txt::new(l.description.clone())
                        .size(8.8)
                        .bold()
                        .color(pal::NAVY)
                        .leading(1.25),
                );
                if notes && let Some(n) = nonblank(l.notes.as_deref()) {
                    desc = desc.item(CellItem::Text(
                        Txt::new(n).size(7.8).color(pal::SLATE).leading(1.3),
                    ));
                }
                li.rows.push(Row::new(vec![
                    desc,
                    m.cell(l.planned, 8.4, false, pal::INK),
                    match l.actual {
                        Some(a) => m.cell(a, 8.4, false, pal::INK),
                        None => m.none(8.4),
                    },
                    match l.actual {
                        Some(a) => m.diff(l.planned - a, 8.4, false, true),
                        None => m.none(8.4),
                    },
                ]));
            }
            // The subtotal travels with the last two lines it adds up.
            let s = sum(ls.iter().copied());
            let bold_num = |v: i64| m.cell(v, 8.4, true, pal::NAVY);
            let mut sub = Row::new(vec![
                Cell::text(
                    Txt::new(format!("SUBTOTAL \u{b7} {}", name.to_uppercase()))
                        .size(7.4)
                        .bold()
                        .track(0.6)
                        .color(pal::SLATE),
                ),
                bold_num(s.planned),
                if s.actual_count > 0 {
                    bold_num(s.actual)
                } else {
                    m.none(8.4)
                },
                if s.actual_count > 0 {
                    m.diff(s.difference, 8.4, true, true)
                } else {
                    m.none(8.4)
                },
            ]);
            sub.fill = Some(pal::PANEL);
            sub.keep_prev = 2;
            li.rows.push(sub);
            if Some(name) == last_name.as_ref() {
                // The grand total closes the last table and stays with its subtotal.
                let tot = |v: i64| {
                    Cell::text(m.txt(v, 8.8, true, pal::WHITE)).inset_right(m.inset(v, 8.8, true))
                };
                let dash = || {
                    Cell::text(
                        Txt::new(NONE)
                            .size(8.8)
                            .color(pal::WHITE)
                            .align(Align::Right),
                    )
                };
                let mut total = Row::new(vec![
                    Cell::text(
                        Txt::new("Total in lines")
                            .size(9.0)
                            .bold()
                            .color(pal::WHITE),
                    ),
                    tot(all.planned),
                    if all.actual_count > 0 {
                        tot(all.actual)
                    } else {
                        dash()
                    },
                    if all.actual_count > 0 {
                        tot(all.difference)
                    } else {
                        dash()
                    },
                ]);
                total.fill = Some(pal::NAVY);
                total.keep_prev = 3;
                li.rows.push(total);
            }
            doc.push(Block::Styled(li));
        }
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

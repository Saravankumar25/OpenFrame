//! Lightweight budget snapshot (FSD §39, §109; FSD-PROD-025/026).
//!
//! Advisory only: a few categories with description + amount lines, a planned
//! total and an optional contingency. Values never change automatically — no
//! production, schedule or script change touches a monetary value (FSD §39.6).
//! Excluded by design (§39.5): payroll, tax, union rules, ledgers, POs, invoices.

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, next_position, opt_int, opt_text, text, update_fields};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::ops::{read, write};
use super::source;
use crate::core::AppCore;
use crate::registry::{Registry, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{optional_text, required_text};

/// FSD §39.2 categories, exactly.
pub const BUDGET_CATEGORIES: [&str; 9] = [
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

/// Upper bound for any amount (minor units) — far beyond any real film budget.
const MAX_AMOUNT: i64 = 1_000_000_000_000_000;

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Budget");
    r.query("budget.get", get).meta(M::read(
        "Current budget with lines, totals and saved snapshots.",
    ));
    r.query("budget.get_snapshot", get_snapshot)
        .meta(M::read("One saved budget snapshot."));
    r.command("budget.create", create)
        .meta(M::edit("Create the project budget."));
    r.command("budget.update", update).meta(M::edit(
        "Change budget settings (currency, planned total, contingency, notes).",
    ));
    r.command("budget.add_line", add_line)
        .meta(M::edit("Add a budget line."));
    r.command("budget.update_line", update_line)
        .meta(M::edit("Edit a budget line."));
    r.command("budget.delete_line", delete_line)
        .meta(M::soft_delete("Remove a budget line (recoverable)."));
    r.command("budget.save_snapshot", save_snapshot)
        .meta(M::edit("Save a labelled budget snapshot."));
    r.command("budget.mark_reviewed", mark_reviewed)
        .meta(M::edit(
            "Mark the budget reviewed against production changes.",
        ));
    r.command("budget.delete_snapshot", delete_snapshot)
        .meta(M::soft_delete(
            "Delete a saved budget snapshot (recoverable).",
        ));
    r.trash_handler(TrashHandler {
        object_type: "budget_snapshot",
        table: "budget_snapshot",
        label: "Budget snapshot",
        restore: None,
        purge: purge_snapshot,
    });
    r.trash_handler(TrashHandler {
        object_type: "budget_line",
        table: "budget_line",
        label: "Budget line",
        restore: None,
        purge: purge_line,
    });
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BudgetLineDto {
    pub id: String,
    pub category: String,
    pub description: String,
    /// Minor units (cents/paise).
    #[ts(type = "number")]
    pub amount: i64,
    pub notes: Option<String>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BudgetCategoryTotal {
    pub category: String,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BudgetDto {
    pub id: String,
    pub label: Option<String>,
    pub currency: String,
    #[ts(type = "number | null")]
    pub planned_total: Option<i64>,
    /// "percent" (value in basis points: 1000 = 10%) or "amount" (minor units).
    pub contingency_mode: String,
    #[ts(type = "number")]
    pub contingency_value: i64,
    /// Computed contingency in minor units (percent of the planned total, or of
    /// the entered total when no plan is set).
    #[ts(type = "number")]
    pub contingency_amount: i64,
    /// Sum of all lines ("Entered so far").
    #[ts(type = "number")]
    pub entered_total: i64,
    pub categories: Vec<BudgetCategoryTotal>,
    pub lines: Vec<BudgetLineDto>,
    pub notes: Option<String>,
    pub is_current: bool,
    #[ts(type = "number | null")]
    pub frozen_at: Option<i64>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BudgetSnapshotRow {
    pub id: String,
    pub label: Option<String>,
    pub currency: String,
    #[ts(type = "number")]
    pub entered_total: i64,
    #[ts(type = "number | null")]
    pub frozen_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BudgetView {
    pub categories: Vec<String>,
    pub current: Option<BudgetDto>,
    pub snapshots: Vec<BudgetSnapshotRow>,
    /// Manual "Review budget" reminder (FSD §39.6, §109): set when the active
    /// production source changed since the budget was last reviewed. Nothing
    /// monetary changes until the user edits it.
    pub review_reminder: Option<String>,
}

fn load_budget(c: &Connection, id: &str) -> AppResult<BudgetDto> {
    #[allow(clippy::type_complexity)]
    let row: Option<(String, Option<String>, String, Option<i64>, String, i64, Option<String>, i64, Option<i64>, i64)> = c
        .query_row(
            "SELECT id, label, currency, planned_total, contingency_mode, contingency_value, notes, is_current, frozen_at, rev
             FROM budget_snapshot WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?)),
        )
        .optional()?;
    let (id, label, currency, planned_total, mode, value, notes, is_current, frozen_at, rev) =
        row.ok_or_else(|| AppError::not_found("budget"))?;
    let mut st = c.prepare(
        "SELECT id, category, description, amount, notes, rev FROM budget_line
         WHERE budget_id = ?1 AND deleted_at IS NULL ORDER BY position, created_at, id",
    )?;
    let mut lines: Vec<BudgetLineDto> = st
        .query_map([&id], |r| {
            Ok(BudgetLineDto {
                id: r.get(0)?,
                category: r.get(1)?,
                description: r.get(2)?,
                amount: r.get(3)?,
                notes: r.get(4)?,
                rev: r.get(5)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    lines.sort_by_key(|l| {
        BUDGET_CATEGORIES
            .iter()
            .position(|c| *c == l.category)
            .unwrap_or(usize::MAX)
    });
    let entered_total: i64 = lines.iter().map(|l| l.amount).sum();
    let categories = BUDGET_CATEGORIES
        .iter()
        .map(|cat| {
            let ls: Vec<&BudgetLineDto> = lines.iter().filter(|l| l.category == *cat).collect();
            BudgetCategoryTotal {
                category: cat.to_string(),
                total: ls.iter().map(|l| l.amount).sum(),
                count: ls.len() as i64,
            }
        })
        .collect();
    let contingency_amount = if mode == "percent" {
        let base = planned_total.unwrap_or(entered_total) as i128;
        ((base * value as i128 + 5_000) / 10_000) as i64
    } else {
        value
    };
    Ok(BudgetDto {
        id,
        label,
        currency,
        planned_total,
        contingency_mode: mode,
        contingency_value: value,
        contingency_amount,
        entered_total,
        categories,
        lines,
        notes,
        is_current: is_current != 0,
        frozen_at,
        rev,
    })
}

fn current_id(c: &Connection) -> AppResult<Option<String>> {
    Ok(c
        .query_row(
            "SELECT id FROM budget_snapshot WHERE is_current = 1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?)
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetGetArgs {}

fn get(core: &AppCore, actor: &Actor, _: BudgetGetArgs) -> AppResult<BudgetView> {
    read(core, actor, |c| {
        let current = match current_id(c)? {
            Some(id) => Some(load_budget(c, &id)?),
            None => None,
        };
        let mut st = c.prepare(
            "SELECT id FROM budget_snapshot WHERE is_current = 0 AND deleted_at IS NULL ORDER BY frozen_at DESC, created_at DESC",
        )?;
        let ids: Vec<String> = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        let snapshots = ids
            .iter()
            .map(|id| {
                load_budget(c, id).map(|b| BudgetSnapshotRow {
                    id: b.id,
                    label: b.label,
                    currency: b.currency,
                    entered_total: b.entered_total,
                    frozen_at: b.frozen_at,
                })
            })
            .collect::<AppResult<_>>()?;
        let review_reminder = match current.as_ref() {
            Some(b) => review_reminder(c, &b.id)?,
            None => None,
        };
        Ok(BudgetView {
            categories: BUDGET_CATEGORIES.iter().map(|s| s.to_string()).collect(),
            current,
            snapshots,
            review_reminder,
        })
    })
}

/// A reminder when the production source moved on since the last review.
fn review_reminder(c: &Connection, budget_id: &str) -> AppResult<Option<String>> {
    let Some(active) = source::active_source(c)? else {
        return Ok(None);
    };
    let reviewed: Option<String> = c
        .query_row(
            "SELECT reviewed_source_id FROM budget_snapshot WHERE id = ?1",
            [budget_id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    if reviewed.as_deref() == Some(active.id.as_str()) {
        return Ok(None);
    }
    Ok(Some(format!(
        "The production source is now {} — {}. Review the budget if the script change affects costs. Amounts never change automatically.",
        active.screenplay_title, active.label
    )))
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetIdArgs {
    pub id: String,
}

fn get_snapshot(core: &AppCore, actor: &Actor, a: BudgetIdArgs) -> AppResult<BudgetDto> {
    read(core, actor, |c| load_budget(c, &a.id))
}

fn clean_currency(s: &str) -> AppResult<String> {
    let v = s.trim().to_uppercase();
    if v.len() == 3 && v.chars().all(|c| c.is_ascii_uppercase()) {
        Ok(v)
    } else {
        Err(AppError::invalid_input(
            "Choose a currency such as USD, EUR or INR.",
        ))
    }
}

fn check_amount(v: i64, what: &str) -> AppResult<i64> {
    if !(0..=MAX_AMOUNT).contains(&v) {
        return Err(AppError::validation(
            "amount",
            format!("{what} must be zero or a positive amount."),
        ));
    }
    Ok(v)
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetCreateArgs {
    pub currency: String,
}

fn create(core: &AppCore, actor: &Actor, a: BudgetCreateArgs) -> AppResult<String> {
    let currency = clean_currency(&a.currency)?;
    write(
        core,
        actor,
        MutationMeta::new(
            "budget.create",
            "Started a budget snapshot",
            Capability::Edit,
        ),
        |tx| {
            let c = tx.conn();
            if current_id(c)?.is_some() {
                return Err(AppError::conflict("This project already has a budget."));
            }
            let id = new_id();
            let now = now_ms();
            let reviewed = source::active_source(c)?.map(|s| s.id);
            c.execute(
            "INSERT INTO budget_snapshot(id, currency, contingency_mode, contingency_value, is_current, reviewed_source_id, created_at, updated_at)
             VALUES (?1, ?2, 'percent', 0, 1, ?3, ?4, ?4)",
            params![id, currency, reviewed, now],
        )?;
            Ok(id)
        },
    )
}

/// "Mark reviewed": the user looked at the budget against the current source.
/// Records only which source was reviewed — never touches an amount.
fn mark_reviewed(core: &AppCore, actor: &Actor, a: BudgetIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "budget.mark_reviewed",
        "Marked the budget as reviewed",
        Capability::Edit,
    )
    .target("budget_snapshot", &a.id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        ensure_current(c, &a.id)?;
        let active = source::active_source(c)?.ok_or_else(|| {
            AppError::invalid_input("There is no production source to review against yet.")
        })?;
        update_fields(
            c,
            "budget_snapshot",
            &a.id,
            &[("reviewed_source_id", text(active.id))],
            &["reviewed_source_id"],
            None,
            "budget",
        )?;
        Ok(())
    })
}

/// Delete a saved (frozen) snapshot. The working budget itself is never deleted
/// here — remove its lines instead.
fn delete_snapshot(core: &AppCore, actor: &Actor, a: BudgetIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "budget.delete_snapshot",
        "Deleted a budget snapshot",
        Capability::SoftDelete,
    )
    .target("budget_snapshot", &a.id);
    write(core, actor, meta, |tx| {
        let row: Option<(Option<String>, i64)> = tx
            .conn()
            .query_row("SELECT label, is_current FROM budget_snapshot WHERE id = ?1 AND deleted_at IS NULL", [&a.id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?;
        let (label, is_current) = row.ok_or_else(|| AppError::not_found("budget snapshot"))?;
        if is_current != 0 {
            return Err(AppError::invalid_input(
                "Only saved budget snapshots can be deleted.",
            ));
        }
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "budget_snapshot",
                table: "budget_snapshot",
                id: &a.id,
                title: Some(label.unwrap_or_else(|| "Budget snapshot".into())),
                parent_type: None,
                parent_id: None,
                position: None,
            },
        )
    })
}

fn purge_snapshot(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    c.execute(
        "DELETE FROM budget_line WHERE budget_id = ?1",
        [&row.object_id],
    )?;
    c.execute(
        "DELETE FROM budget_snapshot WHERE id = ?1",
        [&row.object_id],
    )?;
    Ok(())
}

fn ensure_current(c: &Connection, id: &str) -> AppResult<()> {
    let cur: Option<i64> = c
        .query_row(
            "SELECT is_current FROM budget_snapshot WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    match cur {
        None => Err(AppError::not_found("budget")),
        Some(0) => Err(AppError::conflict(
            "Saved budget snapshots can't be changed.",
        )),
        Some(_) => Ok(()),
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetUpdateArgs {
    pub id: String,
    pub currency: String,
    #[ts(type = "number | null")]
    pub planned_total: Option<i64>,
    pub contingency_mode: String,
    #[ts(type = "number")]
    pub contingency_value: i64,
    pub notes: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

/// Explicit user edit of the planned figures (never automatic).
fn update(core: &AppCore, actor: &Actor, a: BudgetUpdateArgs) -> AppResult<()> {
    let currency = clean_currency(&a.currency)?;
    if let Some(p) = a.planned_total {
        check_amount(p, "The planned total")?;
    }
    match a.contingency_mode.as_str() {
        "percent" if !(0..=10_000).contains(&a.contingency_value) => {
            return Err(AppError::validation(
                "amount",
                "Contingency must be between 0% and 100%.",
            ));
        }
        "percent" => {}
        "amount" => {
            check_amount(a.contingency_value, "Contingency")?;
        }
        _ => {
            return Err(AppError::invalid_input(
                "Choose a percentage or a fixed amount for contingency.",
            ));
        }
    }
    let notes = optional_text(a.notes, "Notes", 4000)?;
    let meta = MutationMeta::new("budget.update", "Changed the budget plan", Capability::Edit)
        .target("budget_snapshot", &a.id);
    write(core, actor, meta, |tx| {
        ensure_current(tx.conn(), &a.id)?;
        update_fields(
            tx.conn(),
            "budget_snapshot",
            &a.id,
            &[
                ("currency", text(currency.clone())),
                ("planned_total", opt_int(a.planned_total)),
                ("contingency_mode", text(a.contingency_mode.clone())),
                ("contingency_value", int(a.contingency_value)),
                ("notes", opt_text(notes.clone())),
            ],
            &[
                "currency",
                "planned_total",
                "contingency_mode",
                "contingency_value",
                "notes",
            ],
            a.expected_rev,
            "budget",
        )?;
        Ok(())
    })
}

fn check_line(
    category: &str,
    description: &str,
    amount: i64,
    notes: Option<String>,
) -> AppResult<(String, i64, Option<String>)> {
    if !BUDGET_CATEGORIES.contains(&category) {
        return Err(AppError::invalid_input(
            "Choose one of the budget categories.",
        ));
    }
    let description = required_text(description, "A description", 200)?;
    let amount = check_amount(amount, "The amount")?;
    Ok((description, amount, optional_text(notes, "Notes", 2000)?))
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetLineAddArgs {
    pub budget_id: String,
    pub category: String,
    pub description: String,
    #[ts(type = "number")]
    pub amount: i64,
    #[serde(default)]
    pub notes: Option<String>,
}

fn add_line(core: &AppCore, actor: &Actor, a: BudgetLineAddArgs) -> AppResult<String> {
    let (description, amount, notes) = check_line(&a.category, &a.description, a.amount, a.notes)?;
    let meta = MutationMeta::new(
        "budget.add_line",
        format!("Added budget line “{description}”"),
        Capability::Edit,
    )
    .target("budget_snapshot", &a.budget_id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        ensure_current(c, &a.budget_id)?;
        let id = new_id();
        let now = now_ms();
        let pos = next_position(
            c,
            "budget_line",
            "budget_id = ?1",
            &[text(a.budget_id.clone())],
        )?;
        c.execute(
            "INSERT INTO budget_line(id, budget_id, category, description, amount, notes, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![id, a.budget_id, a.category, description, amount, notes, pos, now],
        )?;
        Ok(id)
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetLineUpdateArgs {
    pub id: String,
    pub category: String,
    pub description: String,
    #[ts(type = "number")]
    pub amount: i64,
    pub notes: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

fn line_budget(c: &Connection, id: &str) -> AppResult<String> {
    c.query_row(
        "SELECT budget_id FROM budget_line WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("budget line"))
}

fn update_line(core: &AppCore, actor: &Actor, a: BudgetLineUpdateArgs) -> AppResult<()> {
    let (description, amount, notes) = check_line(&a.category, &a.description, a.amount, a.notes)?;
    let meta = MutationMeta::new(
        "budget.update_line",
        format!("Edited budget line “{description}”"),
        Capability::Edit,
    )
    .target("budget_line", &a.id)
    .coalesce(format!("budget-line:{}", a.id));
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        ensure_current(c, &line_budget(c, &a.id)?)?;
        update_fields(
            c,
            "budget_line",
            &a.id,
            &[
                ("category", text(a.category.clone())),
                ("description", text(description.clone())),
                ("amount", int(amount)),
                ("notes", opt_text(notes.clone())),
            ],
            &["category", "description", "amount", "notes"],
            a.expected_rev,
            "budget line",
        )?;
        Ok(())
    })
}

fn delete_line(core: &AppCore, actor: &Actor, a: BudgetIdArgs) -> AppResult<()> {
    let meta = MutationMeta::new(
        "budget.delete_line",
        "Deleted a budget line",
        Capability::SoftDelete,
    )
    .target("budget_line", &a.id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        let budget = line_budget(c, &a.id)?;
        ensure_current(c, &budget)?;
        let (desc, pos): (String, i64) = c.query_row(
            "SELECT description, position FROM budget_line WHERE id = ?1",
            [&a.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "budget_line",
                table: "budget_line",
                id: &a.id,
                title: Some(desc),
                parent_type: Some("budget_snapshot"),
                parent_id: Some(budget),
                position: Some(pos),
            },
        )
    })
}

fn purge_line(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn()
        .execute("DELETE FROM budget_line WHERE id = ?1", [&row.object_id])?;
    Ok(())
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetSnapshotArgs {
    pub id: String,
    pub label: String,
}

/// Freeze a copy of the current budget as a historical snapshot (new identities).
fn save_snapshot(core: &AppCore, actor: &Actor, a: BudgetSnapshotArgs) -> AppResult<String> {
    let label = required_text(&a.label, "A snapshot name", 120)?;
    let meta = MutationMeta::new(
        "budget.save_snapshot",
        format!("Saved budget snapshot “{label}”"),
        Capability::Edit,
    )
    .target("budget_snapshot", &a.id);
    write(core, actor, meta, |tx| {
        let c = tx.conn();
        ensure_current(c, &a.id)?;
        let b = load_budget(c, &a.id)?;
        let id = new_id();
        let now = now_ms();
        c.execute(
            "INSERT INTO budget_snapshot(id, label, currency, planned_total, contingency_mode, contingency_value, notes, is_current, frozen_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?8, ?8)",
            params![id, label, b.currency, b.planned_total, b.contingency_mode, b.contingency_value, b.notes, now],
        )?;
        for (i, l) in b.lines.iter().enumerate() {
            c.execute(
                "INSERT INTO budget_line(id, budget_id, category, description, amount, notes, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                params![new_id(), id, l.category, l.description, l.amount, l.notes, i as i64 + 1, now],
            )?;
        }
        Ok(id)
    })
}

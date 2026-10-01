//! Change Sets (Domain §4 Change Set; AI spec §20; FSD §42.7, §42.15).
//!
//! ```text
//! prepare ─► Pending ─accept─► revalidate ─┬─ changed ──► Stale    (not applied; "Re-check & Review")
//!                │                         ├─ deleted ──► Conflict (not applied)
//!                │                         └─ valid ────► Accepted ─► apply every op via the registry
//!                │                                                    ├─ all ok ─► Applied (one undo step)
//!                │                                                    └─ any fail ► undo applied ops ─► Failed
//!                └─reject─► Rejected (project untouched)
//! ```
//!
//! Apply is all-or-nothing: each operation runs through `core.dispatch` as a
//! normal command (permissions, validation, undo, activity, search — exactly
//! as from the UI) with the actor's origin marked AI. If any operation fails,
//! the ones already applied are undone through history. On success, their
//! undo steps are grouped into one "AI-applied: …" step.
//!
//! A multi-step request ends in ONE composite Change Set ([`compose`]): the
//! parts' operations run in order, previews are shown part by part, and
//! "Re-check & Review" rebuilds every part from its recorded source.
//!
//! Review is human-only (agentic spec §3, §40): accept / reject / re-check are
//! refused for any actor that is not the local user acting in the UI — in
//! particular for AI-origin actors (the identity operations run under while a
//! Change Set is applied) — and a Change Set can never contain an `ai.*`
//! operation, so acceptance can't be nested or chained.

use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::undo::{self, RowChange};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use ts_rs::TS;

use super::catalog;
use super::records::{ai_meta, set_result_status};
use super::scope::{self, AiScopeArgs, AiScopeKind};
use super::toolbox;
use super::tools::ToolCtx;
use super::types::*;
use crate::core::AppCore;
use crate::registry::OpKind;
use crate::store::{MutationMeta, Store, Tx};

pub const STALE_MESSAGE: &str = "The project changed after this suggestion was prepared. Review is required before applying it.";

/// Most parts one composite Change Set may combine.
pub const MAX_PARTS: usize = 6;

/// `ai.change_set.accept` arguments. `confirmDestructive` is the product's extra
/// destructive-change confirmation (required when the preview contains one).
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiAcceptArgs")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptArgs {
    pub id: String,
    #[serde(default)]
    #[ts(optional)]
    pub confirm_destructive: Option<bool>,
}

/// Review actions are performed by a person in the UI, never by the assistant, an
/// applied Change Set or a received package (agentic spec §40).
pub fn require_human(actor: &Actor) -> AppResult<()> {
    match actor.origin {
        ActorOrigin::Local => Ok(()),
        _ => Err(AppError::new(
            "permission.denied",
            "Only you can review proposed changes, from the AI Assistant panel.",
        )
        .with_detail("change set review requires a local user action")),
    }
}

/// Combine the proposals of one request into a single reviewable Change Set
/// (contract C4). Operations keep their order; previews are grouped by part;
/// targets, modules and base rows are unioned. `None` when there is nothing.
pub fn compose(parts: Vec<ChangeSetDraft>) -> Option<ChangeSetDraft> {
    let n = parts.len();
    if n <= 1 {
        return parts.into_iter().next().map(|mut d| {
            d.sources = d.all_sources();
            d
        });
    }
    let titles: Vec<String> = parts.iter().map(|p| p.title.clone()).collect();
    let first = &parts[0];
    let mut out = ChangeSetDraft {
        title: super::queries::truncate_chars(&format!("{n} changes: {}", titles.join(" · ")), 160),
        summary: super::queries::truncate_chars(
            &format!(
                "I prepared {n} changes for you to review together: {}. Nothing changes until you apply them.",
                titles.join("; ")
            ),
            1_000,
        ),
        operations: Vec::new(),
        preview: Vec::new(),
        exclusions: Vec::new(),
        targets: Vec::new(),
        modules: Vec::new(),
        base_rows: Vec::new(),
        source_tool: first.source_tool.clone(),
        source_args: first.source_args.clone(),
        sources: Vec::new(),
    };
    for (i, p) in parts.into_iter().enumerate() {
        out.sources.extend(p.all_sources());
        out.preview.push(PreviewRow::section(
            p.title.clone(),
            format!("Change {} of {n}", i + 1),
        ));
        out.preview.extend(p.preview);
        for x in p.exclusions {
            if !out.exclusions.contains(&x) {
                out.exclusions.push(x);
            }
        }
        out.operations.extend(p.operations);
        for t in p.targets {
            if !out
                .targets
                .iter()
                .any(|o| o.table == t.table && o.id == t.id)
            {
                out.targets.push(t);
            }
        }
        for m in p.modules {
            if !out.modules.contains(&m) {
                out.modules.push(m);
            }
        }
        for r in p.base_rows {
            if !out.base_rows.contains(&r) {
                out.base_rows.push(r);
            }
        }
    }
    Some(out)
}

fn source_json(d: &ChangeSetDraft) -> Value {
    let sources: Vec<Value> = d
        .all_sources()
        .into_iter()
        .map(|(tool, args)| json!({"tool": tool, "args": args}))
        .collect();
    // `tool` / `args` stay readable by older builds (first part).
    json!({"tool": d.source_tool, "args": d.source_args, "sources": sources})
}

/// Sources recorded with a stored Change Set (older records carry only `tool`/`args`).
fn stored_sources(source: &Value) -> Vec<(String, Value)> {
    if let Some(list) = source.get("sources").and_then(|v| v.as_array())
        && !list.is_empty()
    {
        return list
            .iter()
            .map(|s| {
                (
                    s.get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    s.get("args").cloned().unwrap_or(Value::Null),
                )
            })
            .collect();
    }
    vec![(
        source
            .get("tool")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        source.get("args").cloned().unwrap_or(Value::Null),
    )]
}

/// Persist a prepared Change Set (inside the caller's AI-record transaction).
pub fn insert(
    tx: &Tx<'_>,
    request_id: Option<&str>,
    result_id: Option<&str>,
    d: &ChangeSetDraft,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    let base = json!({
        "projectId": catalog::project_id(tx.conn())?,
        "rows": catalog::snapshot(tx.conn(), &d.base_rows)?,
    });
    let source = source_json(d);
    tx.conn().execute(
        "INSERT INTO change_set(id, origin, ai_request_id, ai_result_id, requesting_user_id, requesting_role, title, summary,
                                target_objects_json, affected_modules_json, operations_json, preview_json, exclusions_json,
                                base_version, review_state, validation_state, source_json, created_at, updated_at)
         VALUES (?1, 'AI', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'Pending', 'Valid', ?14, ?15, ?15)",
        params![
            id,
            request_id,
            result_id,
            tx.actor().user_id,
            tx.actor().role.as_str(),
            d.title,
            d.summary,
            serde_json::to_string(&d.targets).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&d.modules).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&d.operations).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&d.preview).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&d.exclusions).unwrap_or_else(|_| "[]".into()),
            base.to_string(),
            source.to_string(),
            now
        ],
    )?;
    Ok(id)
}

/// Create a standalone Change Set (no AI request). Used by callers that build
/// proposals programmatically; the AI orchestrator uses `insert` inside its own transaction.
pub fn create(core: &AppCore, actor: &Actor, d: &ChangeSetDraft) -> AppResult<ChangeSetDto> {
    actor.require(Capability::UseAi, "use the assistant")?;
    let s = core.project()?;
    let id = s
        .store
        .mutate(actor, ai_meta("ai.change_set.prepare"), |tx| {
            insert(tx, None, None, d)
        })?;
    get(core, actor, &id)
}

struct Row {
    id: String,
    title: String,
    state: String,
    requesting_role: String,
    ai_request_id: Option<String>,
    operations: Vec<OpCall>,
    base: Value,
    source: Value,
    destructive: bool,
}

fn load_row(c: &Connection, actor: &Actor, id: &str) -> AppResult<Row> {
    c.query_row(
        "SELECT id, title, review_state, requesting_role, ai_request_id, operations_json, base_version, source_json, preview_json
         FROM change_set WHERE id=?1 AND requesting_user_id=?2 AND deleted_at IS NULL",
        params![id, actor.user_id],
        |r| {
            let ops: String = r.get(5)?;
            let base: String = r.get(6)?;
            let source: Option<String> = r.get(7)?;
            let preview: Vec<PreviewRow> = parse_json(r.get(8)?);
            Ok(Row {
                destructive: preview.iter().any(|x| x.tone == TONE_DESTRUCTIVE),
                id: r.get(0)?,
                title: r.get(1)?,
                state: r.get(2)?,
                requesting_role: r.get(3)?,
                ai_request_id: r.get(4)?,
                operations: serde_json::from_str(&ops).unwrap_or_default(),
                base: serde_json::from_str(&base).unwrap_or(Value::Null),
                source: source.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null),
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("proposal"))
}

fn parse_json<T: serde::de::DeserializeOwned + Default>(s: String) -> T {
    serde_json::from_str(&s).unwrap_or_default()
}

pub fn load_dto(c: &Connection, actor: &Actor, id: &str) -> AppResult<Option<ChangeSetDto>> {
    Ok(c.query_row(
        "SELECT id, title, summary, review_state, validation_state, preview_json, exclusions_json, affected_modules_json,
                target_objects_json, operations_json, stale_reason, error_message, created_at, approved_at, applied_at, applied_operations,
                source_json
         FROM change_set WHERE id=?1 AND requesting_user_id=?2 AND deleted_at IS NULL",
        params![id, actor.user_id],
        |r| {
            let ops: Vec<OpCall> = parse_json(r.get(9)?);
            let rows: Vec<PreviewRow> = parse_json(r.get(5)?);
            let source: Value = parse_json(r.get(16)?);
            Ok(ChangeSetDto {
                id: r.get(0)?,
                title: r.get(1)?,
                summary: r.get(2)?,
                state: r.get(3)?,
                validation_state: r.get(4)?,
                part_count: stored_sources(&source).len().max(1) as u32,
                requires_confirmation: rows.iter().any(|x| x.tone == TONE_DESTRUCTIVE),
                rows,
                exclusions: parse_json(r.get(6)?),
                affected_modules: parse_json(r.get(7)?),
                targets: parse_json(r.get(8)?),
                operation_count: ops.len() as u32,
                stale_reason: r.get(10)?,
                error_message: r.get(11)?,
                created_at: r.get(12)?,
                approved_at: r.get(13)?,
                applied_at: r.get(14)?,
                applied_operations: r.get::<_, i64>(15)? as u32,
            })
        },
    )
    .optional()?)
}

pub fn get(core: &AppCore, actor: &Actor, id: &str) -> AppResult<ChangeSetDto> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    s.store
        .read(|c| load_dto(c, actor, id))?
        .ok_or_else(|| AppError::not_found("proposal"))
}

/// Revalidate the base version: None = still valid, Some((state, reason)) otherwise.
fn revalidate(
    c: &Connection,
    row: &Row,
    actor: &Actor,
) -> AppResult<Option<(&'static str, String)>> {
    let project = catalog::project_id(c)?;
    if row.base.get("projectId").and_then(|v| v.as_str()) != project.as_deref() {
        return Ok(Some(("Conflict", "This proposal was prepared for a different version of the project. It has not been applied.".into())));
    }
    if row.requesting_role != actor.role.as_str() {
        return Ok(Some((
            "Stale",
            "Your permissions changed after this proposal was prepared. It has not been applied."
                .into(),
        )));
    }
    let mut changed = 0usize;
    let mut deleted = 0usize;
    for r in row
        .base
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
    {
        let table = r.get("table").and_then(|v| v.as_str()).unwrap_or("");
        let id = r.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let base_rev = r.get("rev").and_then(|v| v.as_i64());
        let now = super::queries::row_rev(c, table, id)?;
        match (base_rev, now) {
            (Some(_), None) => deleted += 1,
            (None, Some(_)) => changed += 1,
            (Some(a), Some(b)) if a != b => changed += 1,
            _ => {}
        }
    }
    if deleted > 0 {
        return Ok(Some((
            "Conflict",
            format!(
                "{} in this proposal {} deleted after it was prepared. It has not been applied.",
                if deleted == 1 {
                    "1 item".to_string()
                } else {
                    format!("{deleted} items")
                },
                if deleted == 1 { "was" } else { "were" }
            ),
        )));
    }
    if changed > 0 {
        return Ok(Some((
            "Stale",
            format!(
                "The project changed after this proposal was prepared ({} edited). It has not been applied.",
                if changed == 1 {
                    "1 item".to_string()
                } else {
                    format!("{changed} items")
                }
            ),
        )));
    }
    Ok(None)
}

fn update_state(
    store: &Store,
    actor: &Actor,
    meta: MutationMeta,
    id: &str,
    state: &str,
    validation: &str,
    reason: Option<&str>,
    error: Option<&str>,
    result_status: AiResultStatus,
) -> AppResult<()> {
    store.mutate(actor, meta, |tx| {
        tx.conn().execute(
            "UPDATE change_set SET review_state=?1, validation_state=?2, stale_reason=?3, error_message=?4, updated_at=?5, rev=rev+1
             WHERE id=?6 AND requesting_user_id=?7",
            params![state, validation, reason, error, now_ms(), id, actor.user_id],
        )?;
        set_result_status(tx, id, result_status)
    })
}

fn max_undo_seq(store: &Store, actor: &Actor) -> AppResult<i64> {
    store.read(|c| {
        Ok(c.query_row(
            "SELECT COALESCE(MAX(seq), 0) FROM sys_undo WHERE actor_id IS ?1",
            [&actor.user_id],
            |r| r.get(0),
        )?)
    })
}

fn steps_since(store: &Store, actor: &Actor, seq: i64) -> AppResult<i64> {
    store.read(|c| {
        Ok(c.query_row(
            "SELECT count(*) FROM sys_undo WHERE actor_id IS ?1 AND seq > ?2 AND state='done'",
            params![actor.user_id, seq],
            |r| r.get(0),
        )?)
    })
}

/// Stop the user's latest undo step from absorbing the first AI-applied
/// operation (typing coalescing), so the applied Change Set stays its own step.
fn seal_undo_top(store: &Store, actor: &Actor) -> AppResult<()> {
    store.with_writer(|c| {
        c.execute(
            "UPDATE sys_undo SET coalesce_key=NULL WHERE seq=(SELECT MAX(seq) FROM sys_undo WHERE actor_id IS ?1 AND state='done')",
            [&actor.user_id],
        )?;
        Ok(())
    })
}

/// Merge the undo steps created by an applied Change Set into one labelled step,
/// so "Undo" reverts the whole AI-applied change (FSD §42.18, AI spec §20.4).
fn group_undo_steps(store: &Store, actor: &Actor, after_seq: i64, label: &str) -> AppResult<()> {
    store.with_writer(|c| {
        c.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| -> AppResult<()> {
            let mut stmt = c.prepare(
                "SELECT seq, changes_json FROM sys_undo WHERE actor_id IS ?1 AND seq > ?2 AND state='done' ORDER BY seq",
            )?;
            let steps: Vec<(i64, String)> =
                stmt.query_map(params![actor.user_id, after_seq], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
            drop(stmt);
            if steps.len() < 2 {
                if let Some((seq, _)) = steps.first() {
                    c.execute("UPDATE sys_undo SET label=?1 WHERE seq=?2", params![label, seq])?;
                }
                return Ok(());
            }
            let mut merged: Vec<RowChange> = Vec::new();
            for (_, json) in &steps {
                let changes: Vec<RowChange> = serde_json::from_str(json).map_err(|e| AppError::internal(e.to_string()))?;
                merged = undo::merge(merged, changes);
            }
            let first = steps[0].0;
            let json = serde_json::to_string(&merged).map_err(|e| AppError::internal(e.to_string()))?;
            c.execute(
                "UPDATE sys_undo SET label=?1, changes_json=?2, coalesce_key=NULL, updated_at=?3 WHERE seq=?4",
                params![label, json, now_ms(), first],
            )?;
            c.execute("DELETE FROM sys_undo WHERE actor_id IS ?1 AND seq > ?2 AND state='done'", params![actor.user_id, first])?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                c.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(e) => {
                let _ = c.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    })
}

/// Explicit acceptance: revalidate, then apply all operations or none.
pub fn accept(core: &AppCore, actor: &Actor, id: &str) -> AppResult<ChangeSetDto> {
    accept_with(core, actor, id, false)
}

/// Explicit acceptance from the review card. `confirm_destructive` carries the
/// product's additional confirmation for destructive changes.
pub fn accept_with(
    core: &AppCore,
    actor: &Actor,
    id: &str,
    confirm_destructive: bool,
) -> AppResult<ChangeSetDto> {
    require_human(actor)?;
    actor.require(Capability::ApplyChangeSet, "apply proposed changes")?;
    let s = core.project()?;
    let store = &s.store;
    let row = store.read(|c| load_row(c, actor, id))?;
    if row.state == "Pending" && row.destructive && !confirm_destructive {
        return Err(AppError::new(
            "validation.confirmation_required",
            "These changes delete or permanently change items. Confirm them before applying.",
        ));
    }
    match row.state.as_str() {
        "Pending" => {}
        "Applied" => return get(core, actor, id),
        "Stale" | "Conflict" => {
            return Err(AppError::conflict(
                "This proposal is out of date. Re-check it before applying.",
            )
            .retryable());
        }
        other => {
            return Err(AppError::conflict(format!(
                "This proposal can't be applied because it is {}.",
                other.to_lowercase()
            )));
        }
    }
    // 1. Base-version / permission revalidation — a stale proposal is never applied blindly.
    if let Some((state, reason)) = store.read(|c| revalidate(c, &row, actor))? {
        let status = if state == "Conflict" {
            AiResultStatus::Conflict
        } else {
            AiResultStatus::Stale
        };
        update_state(
            store,
            actor,
            ai_meta("ai.change_set.revalidate"),
            id,
            state,
            "Needs Review",
            Some(&reason),
            None,
            status,
        )?;
        return get(core, actor, id);
    }
    // 2. Every operation must be an allow-listed, registered command the user may perform.
    for op in &row.operations {
        let registered = core
            .registry
            .get(&op.op)
            .is_some_and(|e| e.kind == OpKind::Command);
        // Never an assistant operation (no nested acceptance, no recursive requests).
        let assistant_op = op.op.starts_with("ai.");
        if assistant_op || !catalog::allowed_op(&op.op) || !registered {
            let msg = "This proposal contains a change OpenFrame can't apply. Nothing was changed.";
            update_state(
                store,
                actor,
                ai_meta("ai.change_set.validate"),
                id,
                "Failed",
                "Invalid",
                None,
                Some(msg),
                AiResultStatus::Failed,
            )?;
            return get(core, actor, id);
        }
        if !actor.can(catalog::op_capability(&op.op)) {
            let msg = format!(
                "You are {} on this project, so you can't apply this change. Nothing was changed.",
                actor.role.label()
            );
            update_state(
                store,
                actor,
                ai_meta("ai.change_set.validate"),
                id,
                "Failed",
                "Invalid",
                None,
                Some(&msg),
                AiResultStatus::Failed,
            )?;
            return get(core, actor, id);
        }
    }
    // 3. Record the explicit approval. The state transition is a compare-and-set on
    //    `Pending`: two concurrent accepts (double-click, two windows) can't both apply.
    store.mutate(actor, ai_meta("ai.change_set.accept"), |tx| {
        let claimed = tx.conn().execute(
            "UPDATE change_set SET review_state='Accepted', validation_state='Valid', approver_user_id=?1, approved_at=?2, approval_scope='All operations',
                 stale_reason=NULL, updated_at=?2, rev=rev+1 WHERE id=?3 AND review_state='Pending'",
            params![actor.user_id, now_ms(), id],
        )?;
        if claimed != 1 {
            return Err(AppError::conflict(
                "This proposal is already being applied or has changed. Nothing was changed.",
            ));
        }
        set_result_status(tx, id, AiResultStatus::Accepted)
    })?;
    // 4. Apply through the normal command pipeline, all-or-nothing.
    let ai_actor = Actor {
        origin: ActorOrigin::Ai {
            request_id: row.ai_request_id.clone().unwrap_or_else(|| row.id.clone()),
        },
        ..actor.clone()
    };
    seal_undo_top(store, actor)?;
    let before = max_undo_seq(store, actor)?;
    for op in &row.operations {
        if let Err(e) = core.dispatch(&ai_actor, &op.op, op.args.clone()) {
            let steps = steps_since(store, actor, before)?;
            let mut rolled_back = true;
            for _ in 0..steps {
                if let Err(undo_err) = store.undo(actor) {
                    tracing::error!(
                        code = undo_err.code_str(),
                        "could not roll back a partially applied AI change set"
                    );
                    rolled_back = false;
                    break;
                }
            }
            let msg = if rolled_back {
                format!(
                    "{} Nothing was changed.",
                    e.message.trim_end_matches(" Nothing was changed.")
                )
            } else {
                format!(
                    "{} Some changes could not be rolled back automatically; use Undo to review them.",
                    e.message
                )
            };
            update_state(
                store,
                actor,
                ai_meta("ai.change_set.fail"),
                id,
                "Failed",
                "Invalid",
                None,
                Some(&msg),
                AiResultStatus::Failed,
            )?;
            return get(core, actor, id);
        }
    }
    let label = format!("AI-applied: {}", row.title);
    if let Err(e) = group_undo_steps(store, actor, before, &label) {
        // Grouping is a convenience; the individual steps remain undoable.
        tracing::warn!(code = e.code_str(), "could not group AI-applied undo steps");
    }
    // 5. Applied — recorded in Activity as an AI-assisted action by this user.
    let applied = row.operations.len() as i64;
    store.mutate(
        &ai_actor,
        MutationMeta::new("ai.change_set.apply", label.clone(), Capability::UseAi).not_undoable().target("change_set", id),
        |tx| {
            tx.conn().execute(
                "UPDATE change_set SET review_state='Applied', applied_at=?1, applied_operations=?2, updated_at=?1, rev=rev+1 WHERE id=?3",
                params![now_ms(), applied, id],
            )?;
            set_result_status(tx, id, AiResultStatus::Applied)
        },
    )?;
    get(core, actor, id)
}

/// Rejecting leaves project content unchanged (AI-AC-008).
pub fn reject(core: &AppCore, actor: &Actor, id: &str) -> AppResult<ChangeSetDto> {
    require_human(actor)?;
    actor.require(Capability::UseAi, "use the assistant")?;
    let s = core.project()?;
    let row = s.store.read(|c| load_row(c, actor, id))?;
    if !matches!(row.state.as_str(), "Pending" | "Stale" | "Conflict") {
        return Err(AppError::conflict(format!(
            "This proposal is already {}.",
            row.state.to_lowercase()
        )));
    }
    update_state(
        &s.store,
        actor,
        ai_meta("ai.change_set.reject"),
        id,
        "Rejected",
        "Not Checked",
        None,
        None,
        AiResultStatus::Rejected,
    )?;
    get(core, actor, id)
}

/// "Re-check & Review": rebuild every part of the proposal from its recorded
/// source against the current project and return it to Pending for a fresh,
/// explicit review. Re-checking never applies anything.
pub fn recheck(core: &AppCore, actor: &Actor, id: &str) -> AppResult<ChangeSetDto> {
    require_human(actor)?;
    actor.require(Capability::UseAi, "use the assistant")?;
    let s = core.project()?;
    let row = s.store.read(|c| load_row(c, actor, id))?;
    if !matches!(row.state.as_str(), "Pending" | "Stale" | "Conflict") {
        return Err(AppError::conflict(format!(
            "This proposal is already {}.",
            row.state.to_lowercase()
        )));
    }
    let sources = stored_sources(&row.source);
    let rebuilt: AppResult<Vec<ChangeSetDraft>> = s.store.read(|c| {
        // Proposal builders take no scope-specific context; a neutral scope is enough.
        let scope = scope::resolve(
            c,
            actor,
            &AiScopeArgs {
                kind: AiScopeKind::WholeProject,
                draft_id: None,
                scene_id: None,
                selection: Vec::new(),
                shooting_day_id: None,
                call_sheet_id: None,
            },
        )?;
        let ctx = ToolCtx {
            conn: c,
            actor,
            scope: &scope,
            request_text: "",
        };
        Ok(sources
            .iter()
            .take(MAX_PARTS)
            .map(|(tool, args)| toolbox::rebuild_proposal(core, &ctx, tool, args))
            .collect())
    })?;
    if let Err(e) = &rebuilt
        && e.is("permission")
    {
        let msg = format!(
            "{} Nothing was changed.",
            e.message.trim_end_matches(" Nothing was changed.")
        );
        update_state(
            &s.store,
            actor,
            ai_meta("ai.change_set.recheck"),
            id,
            "Failed",
            "Invalid",
            None,
            Some(&msg),
            AiResultStatus::Failed,
        )?;
        return get(core, actor, id);
    }
    let rebuilt = rebuilt.and_then(|parts| {
        compose(parts).ok_or_else(|| AppError::internal("proposal without sources"))
    });
    match rebuilt {
        Ok(d) => {
            s.store.mutate(actor, ai_meta("ai.change_set.recheck"), |tx| {
                let base = json!({
                    "projectId": catalog::project_id(tx.conn())?,
                    "rows": catalog::snapshot(tx.conn(), &d.base_rows)?,
                });
                tx.conn().execute(
                    "UPDATE change_set SET title=?1, summary=?2, target_objects_json=?3, affected_modules_json=?4, operations_json=?5,
                         preview_json=?6, exclusions_json=?7, base_version=?8, requesting_role=?9, review_state='Pending',
                         validation_state='Valid', stale_reason=NULL, error_message=NULL, updated_at=?10, rev=rev+1
                     WHERE id=?11 AND requesting_user_id=?12",
                    params![
                        d.title,
                        d.summary,
                        serde_json::to_string(&d.targets).unwrap_or_else(|_| "[]".into()),
                        serde_json::to_string(&d.modules).unwrap_or_else(|_| "[]".into()),
                        serde_json::to_string(&d.operations).unwrap_or_else(|_| "[]".into()),
                        serde_json::to_string(&d.preview).unwrap_or_else(|_| "[]".into()),
                        serde_json::to_string(&d.exclusions).unwrap_or_else(|_| "[]".into()),
                        base.to_string(),
                        actor.role.as_str(),
                        now_ms(),
                        id,
                        actor.user_id
                    ],
                )?;
                set_result_status(tx, id, AiResultStatus::PendingApproval)
            })?;
        }
        Err(e) => {
            let reason = format!("{} It has not been applied.", e.message);
            update_state(
                &s.store,
                actor,
                ai_meta("ai.change_set.recheck"),
                id,
                "Conflict",
                "Invalid",
                Some(&reason),
                None,
                AiResultStatus::Conflict,
            )?;
        }
    }
    get(core, actor, id)
}

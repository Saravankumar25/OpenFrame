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

use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::undo::{self, RowChange};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use super::catalog::{self, BuildCtx};
use super::records::{ai_meta, set_result_status};
use super::types::*;
use crate::core::AppCore;
use crate::registry::OpKind;
use crate::store::{MutationMeta, Store, Tx};

pub const STALE_MESSAGE: &str = "The project changed after this suggestion was prepared. Review is required before applying it.";

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
    let source = json!({"tool": d.source_tool, "args": d.source_args});
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
}

fn load_row(c: &Connection, actor: &Actor, id: &str) -> AppResult<Row> {
    c.query_row(
        "SELECT id, title, review_state, requesting_role, ai_request_id, operations_json, base_version, source_json
         FROM change_set WHERE id=?1 AND requesting_user_id=?2 AND deleted_at IS NULL",
        params![id, actor.user_id],
        |r| {
            let ops: String = r.get(5)?;
            let base: String = r.get(6)?;
            let source: Option<String> = r.get(7)?;
            Ok(Row {
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
                target_objects_json, operations_json, stale_reason, error_message, created_at, approved_at, applied_at, applied_operations
         FROM change_set WHERE id=?1 AND requesting_user_id=?2 AND deleted_at IS NULL",
        params![id, actor.user_id],
        |r| {
            let ops: Vec<OpCall> = parse_json(r.get(9)?);
            Ok(ChangeSetDto {
                id: r.get(0)?,
                title: r.get(1)?,
                summary: r.get(2)?,
                state: r.get(3)?,
                validation_state: r.get(4)?,
                rows: parse_json(r.get(5)?),
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
    actor.require(Capability::ApplyChangeSet, "apply proposed changes")?;
    let s = core.project()?;
    let store = &s.store;
    let row = store.read(|c| load_row(c, actor, id))?;
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
        if !catalog::allowed_op(&op.op) || !registered {
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
    // 3. Record the explicit approval.
    store.mutate(actor, ai_meta("ai.change_set.accept"), |tx| {
        tx.conn().execute(
            "UPDATE change_set SET review_state='Accepted', validation_state='Valid', approver_user_id=?1, approved_at=?2, approval_scope='All operations',
                 stale_reason=NULL, updated_at=?2, rev=rev+1 WHERE id=?3",
            params![actor.user_id, now_ms(), id],
        )?;
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

/// "Re-check & Review": rebuild the proposal against the current project and
/// return it to Pending for a fresh, explicit review (never auto-applied).
pub fn recheck(core: &AppCore, actor: &Actor, id: &str) -> AppResult<ChangeSetDto> {
    actor.require(Capability::UseAi, "use the assistant")?;
    let s = core.project()?;
    let row = s.store.read(|c| load_row(c, actor, id))?;
    if !matches!(row.state.as_str(), "Pending" | "Stale" | "Conflict") {
        return Err(AppError::conflict(format!(
            "This proposal is already {}.",
            row.state.to_lowercase()
        )));
    }
    let tool = row
        .source
        .get("tool")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let args = row.source.get("args").cloned().unwrap_or(Value::Null);
    if let Some(spec) = catalog::spec(&tool)
        && !actor.can(spec.cap)
    {
        let msg = format!(
            "{} Nothing was changed.",
            catalog::denial_message(actor, spec)
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
    let rebuilt = s.store.read(|c| {
        let ctx = BuildCtx {
            conn: c,
            actor,
            registry: &core.registry,
        };
        Ok(catalog::build(&ctx, &tool, &args))
    })?;
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

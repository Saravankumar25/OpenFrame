//! Persistence of AI Requests, Results and Tool Invocations (Domain §4; AI spec §50).
//! AI records are personal (filtered by user) and never undoable project edits.
//! No hidden reasoning and no full-context copies are stored.

use openframe_domain::{Actor, AppResult, Capability, new_id, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use super::change_set;
use super::types::*;
use crate::store::{MutationMeta, Tx};

/// Metadata for AI bookkeeping writes: the user's own AI history, not project edits
/// (no undo step, no activity entry).
pub fn ai_meta(action: &'static str) -> MutationMeta {
    MutationMeta::new(action, "Assistant history", Capability::UseAi)
        .not_undoable()
        .quiet()
}

pub struct NewRequest<'a> {
    pub id: &'a str,
    pub project_id: Option<String>,
    pub conversation_id: &'a str,
    pub scope_kind: &'a str,
    pub scope_json: &'a Value,
    pub scope_label: &'a str,
    pub text: &'a str,
    pub model_reference: Option<&'a str>,
}

pub fn insert_request(tx: &Tx<'_>, r: &NewRequest<'_>) -> AppResult<()> {
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO ai_request(id, user_id, project_id, session_id, scope_kind, scope_json, scope_label, request_text,
                                external_processing_state, model_reference, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'Local', ?9, 'Running', ?10, ?10)",
        params![
            r.id,
            tx.actor().user_id,
            r.project_id,
            r.conversation_id,
            r.scope_kind,
            r.scope_json.to_string(),
            r.scope_label,
            r.text,
            r.model_reference,
            now
        ],
    )?;
    Ok(())
}

pub struct RequestOutcome<'a> {
    pub intent: Option<&'a str>,
    pub class: Option<OperationClass>,
    pub targets: &'a [ObjRef],
    pub authorization: &'a str,
    pub status: &'a str,
    /// 'Local' when the local model processed the request, 'Not Sent' otherwise.
    pub processing: &'a str,
}

pub fn finish_request(tx: &Tx<'_>, id: &str, o: &RequestOutcome<'_>) -> AppResult<()> {
    let now = now_ms();
    tx.conn().execute(
        "UPDATE ai_request SET intent=?1, operation_class=?2, target_objects_json=?3, authorization_state=?4,
             status=?5, external_processing_state=?6, completed_at=?7, updated_at=?7, rev=rev+1
         WHERE id=?8 AND user_id=?9",
        params![
            o.intent,
            o.class.map(|c| c.as_str()),
            serde_json::to_string(o.targets).unwrap_or_else(|_| "[]".into()),
            o.authorization,
            o.status,
            o.processing,
            now,
            id,
            tx.actor().user_id
        ],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn insert_invocation(
    tx: &Tx<'_>,
    request_id: &str,
    tool: &str,
    params_json: &Value,
    targets: &[ObjRef],
    authorization: &str,
    execution: &str,
    result_ref: Option<&str>,
    error_code: Option<&str>,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO ai_tool_invocation(id, request_id, tool_name, parameters_json, target_objects_json, authorization_state,
                                        execution_state, result_reference, error_code, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            id,
            request_id,
            tool,
            params_json.to_string(),
            serde_json::to_string(targets).unwrap_or_else(|_| "[]".into()),
            authorization,
            execution,
            result_ref,
            error_code,
            now
        ],
    )?;
    Ok(id)
}

pub struct NewResult<'a> {
    pub id: &'a str,
    pub request_id: &'a str,
    pub kind: AiResultKind,
    pub status: AiResultStatus,
    pub content: &'a str,
    pub details: &'a [String],
    pub structured: Option<&'a Value>,
    pub provenance: &'a [Provenance],
    pub items: &'a [ResultItem],
    pub nav: Option<&'a NavTarget>,
    pub confidence: Option<Confidence>,
    pub change_set_id: Option<&'a str>,
    pub error_code: Option<&'a str>,
}

pub fn insert_result(tx: &Tx<'_>, r: &NewResult<'_>) -> AppResult<()> {
    let now = now_ms();
    // Items are part of the structured payload (deterministic values + navigation).
    let structured = serde_json::json!({ "data": r.structured, "items": r.items });
    tx.conn().execute(
        "INSERT INTO ai_result(id, request_id, result_kind, content, details_json, structured_json, provenance_json, nav_json,
                               change_set_id, confidence_state, status, error_code, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
        params![
            r.id,
            r.request_id,
            r.kind.as_str(),
            r.content,
            serde_json::to_string(r.details).unwrap_or_else(|_| "[]".into()),
            structured.to_string(),
            serde_json::to_string(r.provenance).unwrap_or_else(|_| "[]".into()),
            r.nav.map(|n| serde_json::to_string(n).unwrap_or_default()),
            r.change_set_id,
            r.confidence.map(|c| c.as_str()),
            r.status.as_str(),
            r.error_code,
            now
        ],
    )?;
    Ok(())
}

pub fn set_result_status(
    tx: &Tx<'_>,
    change_set_id: &str,
    status: AiResultStatus,
) -> AppResult<()> {
    tx.conn().execute(
        "UPDATE ai_result SET status=?1, updated_at=?2, rev=rev+1 WHERE change_set_id=?3",
        params![status.as_str(), now_ms(), change_set_id],
    )?;
    Ok(())
}

fn parse<T: serde::de::DeserializeOwned + Default>(s: Option<String>) -> T {
    s.and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Load one exchange (the requesting user's only).
pub fn load_exchange(
    c: &Connection,
    actor: &Actor,
    request_id: &str,
) -> AppResult<Option<AiExchange>> {
    let row: Option<(String, String, String, Option<String>, Option<String>, i64)> = c
        .query_row(
            "SELECT id, session_id, request_text, operation_class, model_reference, created_at FROM ai_request
             WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL",
            params![request_id, actor.user_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()?;
    let Some((id, session, text, class, model, created_at)) = row else {
        return Ok(None);
    };
    let scope_label: String = c.query_row(
        "SELECT scope_label FROM ai_request WHERE id=?1",
        [&id],
        |r| r.get(0),
    )?;
    let result = load_result(c, actor, &id)?;
    let Some(result) = result else {
        return Ok(None);
    };
    Ok(Some(AiExchange {
        request_id: id,
        conversation_id: session,
        request_text: text,
        scope_label,
        operation_class: class.and_then(|c| match c.as_str() {
            "Read" => Some(OperationClass::Read),
            "Compute" => Some(OperationClass::Compute),
            "Navigate" => Some(OperationClass::Navigate),
            "Suggest" => Some(OperationClass::Suggest),
            "Mutate" => Some(OperationClass::Mutate),
            _ => None,
        }),
        model_reference: model,
        result,
        created_at,
    }))
}

type ResultRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    i64,
);

fn load_result(c: &Connection, actor: &Actor, request_id: &str) -> AppResult<Option<AiResultDto>> {
    let row: Option<ResultRow> = c
        .query_row(
            "SELECT id, result_kind, content, details_json, structured_json, provenance_json, nav_json, change_set_id,
                    confidence_state, status, error_code, created_at
             FROM ai_result WHERE request_id=?1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1",
            [request_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                    r.get(10)?,
                    r.get(11)?,
                ))
            },
        )
        .optional()?;
    let Some((
        id,
        kind,
        content,
        details,
        structured,
        prov,
        nav,
        cs_id,
        conf,
        status,
        err,
        created_at,
    )) = row
    else {
        return Ok(None);
    };
    let structured: Value = parse(structured);
    let items: Vec<ResultItem> = structured
        .get("items")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let change_set = match cs_id {
        Some(cs) => change_set::load_dto(c, actor, &cs)?,
        None => None,
    };
    Ok(Some(AiResultDto {
        id,
        kind: AiResultKind::parse(&kind),
        status: AiResultStatus::parse(&status),
        content,
        details: parse(Some(details)),
        confidence: conf.and_then(|c| Confidence::parse(&c)),
        provenance: parse(Some(prov)),
        items,
        nav: nav.and_then(|n| serde_json::from_str(&n).ok()),
        change_set,
        error_code: err,
        created_at,
    }))
}

/// Conversation history (oldest first), the requesting user's only.
pub fn history(
    c: &Connection,
    actor: &Actor,
    conversation_id: Option<&str>,
    limit: usize,
) -> AppResult<Vec<AiExchange>> {
    let ids: Vec<String> = match conversation_id {
        Some(conv) => {
            let mut stmt = c.prepare(
                "SELECT id FROM ai_request WHERE user_id=?1 AND session_id=?2 AND deleted_at IS NULL ORDER BY created_at DESC, id DESC LIMIT ?3",
            )?;
            stmt.query_map(params![actor.user_id, conv, limit as i64], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        }
        None => {
            let mut stmt = c.prepare(
                "SELECT id FROM ai_request WHERE user_id=?1 AND deleted_at IS NULL ORDER BY created_at DESC, id DESC LIMIT ?2",
            )?;
            stmt.query_map(params![actor.user_id, limit as i64], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        }
    };
    let mut out = Vec::new();
    for id in ids.iter().rev() {
        if let Some(x) = load_exchange(c, actor, id)? {
            out.push(x);
        }
    }
    Ok(out)
}

/// The user's latest conversation id, if any (so reopening the panel continues it).
pub fn latest_conversation(c: &Connection, actor: &Actor) -> AppResult<Option<String>> {
    Ok(c
        .query_row(
            "SELECT session_id FROM ai_request WHERE user_id=?1 AND deleted_at IS NULL ORDER BY created_at DESC, id DESC LIMIT 1",
            [&actor.user_id],
            |r| r.get(0),
        )
        .optional()?)
}

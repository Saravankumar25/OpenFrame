//! "Save this conversation for review" (ADR-0028 decision 8, amended 2026-10-24).
//!
//! The one place the assistant's words leave the app database, and only by the person's own action:
//! the Save button of the AI panel calls `ai.export_conversation` with a path chosen in the Save
//! dialog. Nothing is written on any other path and nothing is sent anywhere (ADR-0021). The file
//! holds the requesting user's own exchanges of one conversation, loaded through the same filter as
//! `ai.history`, so another person's conversation or private material can never appear.

use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use serde::Deserialize;
use serde_json::{Value, json};
use ts_rs::TS;

use super::trace::{self, AiReportResult};
use super::{orchestrator, records};
use crate::core::AppCore;

/// More exchanges than any conversation holds; the panel itself shows the last 50.
const EXPORT_LIMIT: usize = 1_000;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiConversationExportArgs {
    pub conversation_id: String,
    /// Destination chosen in the Save dialog (`.json` is added when missing).
    pub path: String,
}

fn nothing_to_save() -> AppError {
    AppError::invalid_input(
        "There is no conversation to save yet. Ask the assistant something first.",
    )
}

/// Write one conversation of the requesting user to a file the person chose. `rows` is the number
/// of exchanges written.
pub fn export_conversation(
    core: &AppCore,
    actor: &Actor,
    a: AiConversationExportArgs,
) -> AppResult<AiReportResult> {
    actor.require(Capability::UseAi, "use the assistant")?;
    if !orchestrator::valid_conversation_id(&a.conversation_id) {
        return Err(nothing_to_save());
    }
    let dest = trace::destination(&a.path)?;
    crate::util::check_output_file(core, &dest)?;
    let s = core.project()?;
    let exchanges = s
        .store
        .read(|c| records::history(c, actor, Some(&a.conversation_id), EXPORT_LIMIT))?;
    if exchanges.is_empty() {
        return Err(nothing_to_save());
    }
    let numbers = core.with_app_db(|c| {
        exchanges
            .iter()
            .map(|x| trace::request_numbers(c, &x.request_id))
            .collect::<AppResult<Vec<_>>>()
    })?;
    let rows: Vec<Value> = exchanges
        .iter()
        .zip(numbers)
        .map(|(x, trace)| {
            let r = &x.result;
            json!({
                "request": x.request_text,
                "scope": x.scope_label,
                "answer": r.content,
                "details": r.details,
                "sources": r.provenance.iter().map(|p| p.label.as_str()).collect::<Vec<_>>(),
                "kind": r.kind.as_str(),
                "status": r.status.as_str(),
                "steps": r.steps.iter().map(|st| st.tool.as_str()).collect::<Vec<_>>(),
                "trace": trace,
            })
        })
        .collect();
    let doc = json!({
        "format": "openframe-ai-conversation",
        "version": 1,
        "exportedAt": now_ms() / 1000,
        "conversationId": a.conversation_id,
        "exchanges": rows,
    });
    let bytes = serde_json::to_vec_pretty(&doc).map_err(|e| AppError::internal(e.to_string()))?;
    openframe_project_format::atomic_write(&dest, &bytes)?;
    tracing::info!(exchanges = rows.len(), "AI conversation saved for review");
    Ok(AiReportResult {
        file_name: dest
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        rows: rows.len() as u32,
    })
}

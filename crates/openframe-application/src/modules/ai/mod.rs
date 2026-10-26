//! Offline AI Assistant (FSD §42, FSD-AI-001..027; AI spec; Local AI Runtime spec;
//! UX §3.40; mockups 174–181).
//!
//! Operations:
//! - `ai.status`, `ai.stop_runtime`, `ai.diagnostics`, `ai.download.*`: the engine bundled with
//!   the app and the models downloaded on the person's click (ADR-0026; application-level, no
//!   project required; see `offline`);
//! - `ai.ask`: one request → a bounded multi-step agent run → one AI Result
//!   (answer / navigation / suggestion / one composite proposal / denial /
//!   clarification), persisted with per-step audit records and provenance;
//! - `ai.change_set.get|accept|reject|recheck`: explicit, human-only review of proposals;
//! - `ai.history`: the user's own conversation history;
//! - `ai.memory_list`, `ai.memory_delete`: what the assistant remembers about the user (see `memory`).
//!
//! OpenFrame stays fully usable when the AI is not downloaded, missing or damaged: nothing else
//! depends on this module.

mod briefing;
mod cancel;
pub mod catalog;
pub mod change_set;
mod chat;
mod commands;
mod conversation_export;
pub mod dossier;
pub mod index;
pub mod intelligence;
mod intent;
pub mod knowledge;
mod lanes;
pub mod memory;
pub mod offline;
pub mod orchestrator;
pub mod queries;
mod recipe;
pub mod records;
pub mod retrieval;
mod sanitize;
pub mod scope;
pub mod service;
#[cfg(test)]
mod test_script;
pub mod toolbox;
pub mod tools;
pub mod trace;
pub mod turn;
pub mod types;
mod typo;
mod understand;
mod verify;

use openframe_domain::{Actor, AppResult, Capability};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::Registry;
pub use commands::set_direct_routes;
pub use intent::intent_target;
pub use offline::{
    AiComponentDto, AiDiagnosticsDto, AiDownloadDto, AiImportArgs, AiImportResult, AiStatusDto,
};
use offline::{
    diagnostics, download_import, download_pause, download_remove, download_start, download_status,
    set_graphics, status, stop_runtime,
};
pub use orchestrator::AskArgs;
pub use service::{AiService, service};
pub use types::*;

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.module("Offline AI");
    r.query("ai.status", status).meta(
        M::app_query("Offline AI status: available, downloaded models check, engine state.")
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.stop_runtime", stop_runtime)
        .meta(M::app_command("Stop the local AI engine.").hidden(h::AI_RUNTIME));
    r.command("ai.graphics.set", set_graphics).meta(
        M::app_command("Settings, Offline AI, Graphics card: Automatic or Off.")
            .hidden(h::AI_RUNTIME),
    );
    r.query("ai.download.status", download_status).meta(
        M::app_query("The Download OpenFrame AI screen: size, free space, progress.")
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.download.start", download_start).meta(
        M::app_command("Start or resume the OpenFrame AI download (the person's click; the app's only network use).")
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.download.pause", download_pause).meta(
        M::app_command("Pause the OpenFrame AI download; it resumes where it stopped.")
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.download.import", download_import).meta(
        M::app_command("Use a model file the person already has; it is copied and used only when it matches a pinned file.")
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.download.remove", download_remove).meta(
        M::app_command("Remove the downloaded OpenFrame AI models; projects are not touched.")
            .hidden(h::AI_RUNTIME),
    );
    r.query("ai.health", trace::health).meta(
        M::app_query("The AI health card: medians, apply rate, passes and the weakest tool, from the local numbers-only trace.")
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.export_report", trace::export_report).meta(
        M::app_command("Write the local AI trace (numbers and ids only) and the health figures to a file the person chose.")
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.export_conversation", conversation_export::export_conversation)
        .meta(
            M::command(
                Capability::UseAi,
                "Write one of the person's own conversations (their questions, the answers and the sources) to a file they chose.",
            )
            .hidden(h::USER_PATH),
        );
    r.query("ai.diagnostics", diagnostics).meta(
        M::app_query(
            "Offline AI technical details for Settings (versions, licences, hashes, paths).",
        )
        .hidden(h::AI_RUNTIME),
    );
    r.command("ai.ask", ask).meta(
        M::command(
            Capability::UseAi,
            "Ask the assistant (one request, one AI Result).",
        )
        .hidden(h::AI_SELF),
    );
    r.command("ai.cancel", cancel_request).meta(
        M::command(
            Capability::UseAi,
            "Stop the assistant request that is running in a conversation (nothing is changed).",
        )
        .hidden(h::AI_SELF),
    );
    r.query("ai.history", history).meta(
        M::query(
            Capability::UseAi,
            "The user's own assistant conversation history.",
        )
        .hidden(h::AI_SELF),
    );
    r.query("ai.change_set.get", change_set_get).meta(
        M::read("One prepared Change Set (proposal) of the requesting user.")
            .hidden(h::HUMAN_APPROVAL),
    );
    r.command("ai.change_set.accept", change_set_accept).meta(
        M::command(
            Capability::ApplyChangeSet,
            "Apply a proposal after explicit human approval.",
        )
        .hidden(h::HUMAN_APPROVAL),
    );
    r.command("ai.change_set.reject", change_set_reject).meta(
        M::command(Capability::UseAi, "Reject a proposal (project unchanged).")
            .hidden(h::HUMAN_APPROVAL),
    );
    r.command("ai.change_set.recheck", change_set_recheck).meta(
        M::command(
            Capability::UseAi,
            "Re-check a stale proposal against the current project.",
        )
        .hidden(h::HUMAN_APPROVAL),
    );
    memory::register(r);
    retrieval::register(r);
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiEmptyArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiHistoryArgs {
    /// Conversation to load; the latest conversation when omitted.
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiChangeSetArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiCancelArgs {
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiCancelDto {
    /// A request of this conversation was running and has been asked to stop.
    pub stopping: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiHistoryDto {
    pub conversation_id: Option<String>,
    pub exchanges: Vec<AiExchange>,
}

// ------------------------------------------------------------------ assistant

fn ask(core: &AppCore, actor: &Actor, args: AskArgs) -> AppResult<AiExchange> {
    orchestrator::ask(core, actor, args)
}

fn cancel_request(_core: &AppCore, actor: &Actor, args: AiCancelArgs) -> AppResult<AiCancelDto> {
    actor.require(Capability::UseAi, "use the assistant")?;
    Ok(AiCancelDto {
        stopping: cancel::stop(&actor.user_id, &args.conversation_id),
    })
}

fn history(core: &AppCore, actor: &Actor, args: AiHistoryArgs) -> AppResult<AiHistoryDto> {
    actor.require(Capability::UseAi, "use the assistant")?;
    let s = core.project()?;
    let limit = args.limit.unwrap_or(50).clamp(1, 200) as usize;
    s.store.read(|c| {
        let conv = match args.conversation_id {
            Some(c) => Some(c),
            None => records::latest_conversation(c, actor)?,
        };
        let exchanges = match &conv {
            Some(id) => records::history(c, actor, Some(id), limit)?,
            None => Vec::new(),
        };
        Ok(AiHistoryDto {
            conversation_id: conv,
            exchanges,
        })
    })
}

fn change_set_get(core: &AppCore, actor: &Actor, args: AiChangeSetArgs) -> AppResult<ChangeSetDto> {
    change_set::get(core, actor, &args.id)
}

fn change_set_accept(
    core: &AppCore,
    actor: &Actor,
    args: change_set::AcceptArgs,
) -> AppResult<ChangeSetDto> {
    let dto = change_set::accept_with(
        core,
        actor,
        &args.id,
        args.confirm_destructive.unwrap_or(false),
    )?;
    if let Some(request) = trace::request_of_change_set(core, &args.id) {
        trace::set_outcome(core, &request, "applied");
    }
    Ok(dto)
}

fn change_set_reject(
    core: &AppCore,
    actor: &Actor,
    args: AiChangeSetArgs,
) -> AppResult<ChangeSetDto> {
    let dto = change_set::reject(core, actor, &args.id)?;
    if let Some(request) = trace::request_of_change_set(core, &args.id) {
        trace::set_outcome(core, &request, "rejected");
    }
    Ok(dto)
}

fn change_set_recheck(
    core: &AppCore,
    actor: &Actor,
    args: AiChangeSetArgs,
) -> AppResult<ChangeSetDto> {
    change_set::recheck(core, actor, &args.id)
}

//! Offline AI Assistant (FSD §42, FSD-AI-001..027; AI spec; Local AI Runtime spec;
//! UX §3.40; mockups 174–181).
//!
//! Operations:
//! - `ai.status`, `ai.setup_info`, `ai.install`, `ai.cancel_install`, `ai.uninstall`,
//!   `ai.stop_runtime`, `ai.diagnostics` — the one-click Offline AI package (application-level;
//!   no project required; see `offline`);
//! - `ai.ask` — one request → a bounded multi-step agent run → one AI Result
//!   (answer / navigation / suggestion / one composite proposal / denial /
//!   clarification), persisted with per-step audit records and provenance;
//! - `ai.change_set.get|accept|reject|recheck` — explicit, human-only review of proposals;
//! - `ai.history` — the user's own conversation history.
//!
//! OpenFrame stays fully usable when Offline AI is not installed: nothing else
//! depends on this module.

pub mod catalog;
pub mod change_set;
pub mod intelligence;
pub mod knowledge;
pub mod offline;
pub mod orchestrator;
pub mod queries;
pub mod records;
pub mod retrieval;
pub mod scope;
pub mod service;
pub mod toolbox;
pub mod tools;
pub mod types;

use openframe_domain::{Actor, AppResult, Capability};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::Registry;
pub use offline::{
    AiCancelInstallArgs, AiComponentDto, AiDiagnosticsDto, AiGpuDto, AiInstallStarted,
    AiSetupInfoDto, AiStatusDto, AiUninstallResult,
};
use offline::{cancel_install, diagnostics, install, setup_info, status, stop_runtime, uninstall};
pub use orchestrator::AskArgs;
pub use service::{AiService, InstallProgress, service};
pub use types::*;

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    r.module("Offline AI");
    r.query("ai.status", status).meta(
        M::app_query("Offline AI status: installed, runtime state, install progress.")
            .hidden(h::AI_RUNTIME),
    );
    r.query("ai.setup_info", setup_info).meta(
        M::app_query("Offline AI download size, disk space and device check before installing.")
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.install", install).meta(
        M::app_command("Download and install Offline AI (resumable background task).")
            .fs(Fs::ProjectStorage)
            .long_running()
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.cancel_install", cancel_install)
        .meta(M::app_command("Pause or cancel the Offline AI download.").hidden(h::AI_RUNTIME));
    r.command("ai.uninstall", uninstall).meta(
        M::app_command("Remove the installed Offline AI.")
            .fs(Fs::ProjectStorage)
            .destructive()
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.stop_runtime", stop_runtime)
        .meta(M::app_command("Stop the local AI engine.").hidden(h::AI_RUNTIME));
    r.query("ai.diagnostics", diagnostics).meta(
        M::app_query("Offline AI technical details for Settings (versions, paths, licences).")
            .hidden(h::AI_RUNTIME),
    );
    r.command("ai.ask", ask).meta(
        M::command(
            Capability::UseAi,
            "Ask the assistant (one request, one AI Result).",
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
    change_set::accept_with(
        core,
        actor,
        &args.id,
        args.confirm_destructive.unwrap_or(false),
    )
}

fn change_set_reject(
    core: &AppCore,
    actor: &Actor,
    args: AiChangeSetArgs,
) -> AppResult<ChangeSetDto> {
    change_set::reject(core, actor, &args.id)
}

fn change_set_recheck(
    core: &AppCore,
    actor: &Actor,
    args: AiChangeSetArgs,
) -> AppResult<ChangeSetDto> {
    change_set::recheck(core, actor, &args.id)
}

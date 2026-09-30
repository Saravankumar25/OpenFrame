//! Offline AI Assistant (FSD §42, FSD-AI-001..027; AI spec; Local AI Runtime spec;
//! UX §3.40; mockups 174–181).
//!
//! Operations:
//! - `ai.status`, `ai.hardware`, `ai.install`, `ai.cancel_install`, `ai.remove_model`,
//!   `ai.stop_runtime` — model manager (application-level; no project required);
//! - `ai.ask` — one request → one AI Result (answer / navigation / suggestion /
//!   proposal / denial / clarification), persisted with provenance;
//! - `ai.change_set.get|accept|reject|recheck` — explicit review of proposals;
//! - `ai.history` — the user's own conversation history.
//!
//! OpenFrame stays fully usable when Offline AI is not installed: nothing else
//! depends on this module.

pub mod catalog;
pub mod change_set;
pub mod knowledge;
pub mod orchestrator;
pub mod queries;
pub mod records;
pub mod scope;
pub mod service;
pub mod tools;
pub mod types;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use openframe_ai::download::Control;
use openframe_ai::selection::{self, human_size};
use openframe_domain::{Actor, AppError, AppResult, Capability};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::Registry;
pub use orchestrator::AskArgs;
pub use service::{AiService, InstallProgress, service};
pub use types::*;

pub fn register(r: &mut Registry) {
    r.query("ai.status", status);
    r.query("ai.hardware", hardware);
    r.command("ai.install", install);
    r.command("ai.cancel_install", cancel_install);
    r.command("ai.remove_model", remove_model);
    r.command("ai.stop_runtime", stop_runtime);
    r.command("ai.ask", ask);
    r.query("ai.history", history);
    r.query("ai.change_set.get", change_set_get);
    r.command("ai.change_set.accept", change_set_accept);
    r.command("ai.change_set.reject", change_set_reject);
    r.command("ai.change_set.recheck", change_set_recheck);
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiEmptyArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInstallArgs {
    /// A profile id from `ai.status`; the recommended profile when omitted.
    #[serde(default)]
    pub profile_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiCancelInstallArgs {
    /// false = Pause (keep the partial download to resume later); true = Cancel (discard it).
    #[serde(default)]
    pub discard: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiProfileArgs {
    pub profile_id: String,
}

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

/// A model profile in simple terms (never file names or quantization codes).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiProfileDto {
    pub profile_id: String,
    /// "Lightweight" | "Recommended" | "High Quality"
    pub tier: String,
    pub name: String,
    #[ts(type = "number")]
    pub size_bytes: u64,
    pub size_label: String,
    pub installed: bool,
    pub active: bool,
    pub recommended: bool,
    pub suitable: bool,
    pub likely_slow: bool,
    pub note: Option<String>,
    #[ts(type = "number")]
    pub downloaded_bytes: u64,
    pub start_failed: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiStatusDto {
    /// A verified model is installed and active.
    pub installed: bool,
    /// Header chip: "Local model" or "Off".
    pub mode: String,
    /// NotInstalled | Installed | Starting | LoadingModel | Ready | Busy | Stopping | Failed
    pub runtime_state: String,
    pub runtime_message: Option<String>,
    pub active_profile: Option<AiProfileDto>,
    pub profiles: Vec<AiProfileDto>,
    pub install: Option<InstallProgress>,
    /// Offline AI processes everything on this computer.
    pub local_only: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiGpuDto {
    pub name: String,
    #[ts(type = "number")]
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiRecommendationDto {
    pub profile_id: String,
    pub tier: String,
    pub name: String,
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// What still has to be downloaded (engine + model, minus finished parts).
    #[ts(type = "number")]
    pub download_bytes: u64,
    #[ts(type = "number")]
    pub required_free_bytes: u64,
    /// "Graphics card" | "Processor"
    pub runs_on: String,
    pub likely_slow: bool,
    pub warnings: Vec<String>,
    pub enough_disk: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiHardwareDto {
    #[ts(type = "number")]
    pub memory_bytes: u64,
    #[ts(type = "number")]
    pub available_memory_bytes: u64,
    pub processor: String,
    pub processor_threads: u32,
    pub graphics: Vec<AiGpuDto>,
    #[ts(type = "number | null")]
    pub free_disk_bytes: Option<u64>,
    pub recommendation: AiRecommendationDto,
    pub profiles: Vec<AiProfileDto>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiInstallStarted {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiHistoryDto {
    pub conversation_id: Option<String>,
    pub exchanges: Vec<AiExchange>,
}

// ------------------------------------------------------------ model manager

fn profiles(svc: &AiService) -> AppResult<Vec<AiProfileDto>> {
    let m = svc.manager()?;
    let manifest = m.manifest()?;
    let hw = m.hardware(false);
    let rec = selection::recommend(&hw, &manifest);
    let backend = rec
        .as_ref()
        .map(|r| r.backend)
        .unwrap_or(openframe_ai::Backend::Cpu);
    let active = m.active();
    let installed = m.installed_models();
    let mut out: Vec<AiProfileDto> = manifest
        .models
        .iter()
        .map(|e| {
            let fit = selection::assess(&hw, e, backend);
            let md = installed
                .iter()
                .find(|(md, _)| md.profile_id == e.profile_id);
            AiProfileDto {
                profile_id: e.profile_id.clone(),
                tier: e.tier.label().to_string(),
                name: e.tier.label().to_string(),
                size_bytes: e.bytes,
                size_label: human_size(e.bytes),
                installed: m.paths().model_installed(e),
                active: active
                    .as_ref()
                    .is_some_and(|a| a.profile_id == e.profile_id),
                recommended: rec.as_ref().is_some_and(|r| r.profile_id == e.profile_id),
                suitable: fit.suitable,
                likely_slow: fit.likely_slow,
                note: fit.note,
                downloaded_bytes: m.partial_bytes(&e.profile_id),
                start_failed: md.is_some_and(|(md, _)| md.health_check_failed),
            }
        })
        .collect();
    out.sort_by_key(|p| match p.tier.as_str() {
        "Lightweight" => 0,
        "Recommended" => 1,
        _ => 2,
    });
    Ok(out)
}

fn status(core: &AppCore, _actor: &Actor, _: AiEmptyArgs) -> AppResult<AiStatusDto> {
    let svc = service(core);
    let install = svc.install.lock().clone();
    let Ok(m) = svc.manager() else {
        return Ok(AiStatusDto {
            installed: svc.has_attached_model(),
            mode: if svc.has_attached_model() { "Local model".into() } else { "Off".into() },
            runtime_state: "Failed".into(),
            runtime_message: Some("AI is currently unavailable. OpenFrame's core workflows continue to work normally.".into()),
            active_profile: None,
            profiles: Vec::new(),
            install,
            local_only: true,
        });
    };
    let rs = m.runtime_status();
    let profiles = profiles(&svc).unwrap_or_default();
    let active_profile = profiles.iter().find(|p| p.active).cloned();
    let installed = active_profile.is_some() || svc.has_attached_model();
    let state = if svc.has_attached_model() && rs.state == openframe_ai::RuntimeState::NotInstalled
    {
        "Ready".to_string()
    } else {
        format!("{:?}", rs.state)
    };
    Ok(AiStatusDto {
        installed,
        mode: if installed {
            "Local model".into()
        } else {
            "Off".into()
        },
        runtime_state: state,
        runtime_message: rs.last_error,
        active_profile,
        profiles,
        install,
        local_only: true,
    })
}

fn hardware(core: &AppCore, _actor: &Actor, _: AiEmptyArgs) -> AppResult<AiHardwareDto> {
    let svc = service(core);
    let m = svc.manager()?;
    let hw = m.hardware(true);
    let manifest = m.manifest()?;
    let rec = selection::recommend(&hw, &manifest).ok_or_else(|| {
        AppError::ai(
            "unsupported",
            "Offline AI isn't available for this kind of computer yet.",
        )
    })?;
    let model = manifest
        .model(&rec.profile_id)
        .ok_or_else(|| AppError::not_found("AI profile"))?;
    let runtime = manifest
        .runtime(&rec.runtime_id)
        .ok_or_else(|| AppError::not_found("AI engine"))?;
    let model_left = if m.paths().model_installed(model) {
        0
    } else {
        model
            .bytes
            .saturating_sub(m.partial_bytes(&model.profile_id))
    };
    let runtime_left = if m.paths().runtime_installed(runtime) {
        0
    } else {
        runtime.bytes
    };
    let required = selection::required_free_bytes(model_left, runtime_left);
    Ok(AiHardwareDto {
        memory_bytes: hw.total_ram_bytes,
        available_memory_bytes: hw.available_ram_bytes,
        processor: if hw.cpu_brand.is_empty() {
            "Processor".into()
        } else {
            hw.cpu_brand.clone()
        },
        processor_threads: hw.logical_cpus as u32,
        graphics: hw
            .gpus
            .iter()
            .filter(|g| !g.software)
            .map(|g| AiGpuDto {
                name: g.name.clone(),
                memory_bytes: g.dedicated_vram_bytes,
            })
            .collect(),
        free_disk_bytes: hw.free_disk_bytes,
        recommendation: AiRecommendationDto {
            profile_id: rec.profile_id.clone(),
            tier: rec.tier.label().into(),
            name: rec.tier.label().into(),
            size_bytes: model.bytes,
            download_bytes: model_left + runtime_left,
            required_free_bytes: required,
            runs_on: rec.backend.label().into(),
            likely_slow: rec.likely_slow,
            warnings: rec.warnings.clone(),
            enough_disk: hw.free_disk_bytes.is_none_or(|f| f >= required),
        },
        profiles: profiles(&svc)?,
    })
}

/// Start (or resume) downloading Offline AI in the background.
/// Installing the application's AI is not a project edit (no project needed).
fn install(core: &AppCore, _actor: &Actor, args: AiInstallArgs) -> AppResult<AiInstallStarted> {
    let svc = service(core);
    let mgr = svc.manager()?.clone();
    // Held across spawning so the task can only clear its registration after it exists.
    let mut running = svc.install_task.lock();
    if let Some((id, _)) = running.as_ref() {
        return Ok(AiInstallStarted {
            task_id: id.clone(),
        });
    }
    let profile = args.profile_id.filter(|p| !p.trim().is_empty());
    if let Some(p) = &profile
        && mgr.manifest()?.model(p).is_none()
    {
        return Err(AppError::not_found("AI profile"));
    }
    let discard = Arc::new(AtomicBool::new(false));
    let svc_task = svc.clone();
    let discard_task = discard.clone();
    let profile_task = profile.clone();
    let task_id = core.spawn_task("ai.install", "Downloading Offline AI", move |_core, h| {
        svc_task.set_install(Some(InstallProgress {
            task_id: h.id.clone(),
            profile_id: profile_task.clone(),
            phase: "checking".into(),
            bytes_done: 0,
            bytes_total: 0,
            message: "Checking this computer…".into(),
            error: None,
            active: true,
        }));
        let control = Control::from_flag(h.cancel_flag());
        let svc_ev = svc_task.clone();
        let h_ev = h.clone();
        let events = move |ev: openframe_ai::InstallEvent| {
            svc_ev.update_install(&ev);
            let (_, msg) = service::phase_text(ev.phase);
            let fraction = if ev.total > 0 {
                ev.done as f64 / ev.total as f64
            } else {
                0.0
            };
            h_ev.progress(fraction, msg);
        };
        let result = mgr.install(profile_task.as_deref(), &control, &events);
        let finish = |phase: &str, message: String, error: Option<AppError>| {
            let mut p = svc_task.install.lock().clone().unwrap_or(InstallProgress {
                task_id: h.id.clone(),
                profile_id: profile_task.clone(),
                phase: String::new(),
                bytes_done: 0,
                bytes_total: 0,
                message: String::new(),
                error: None,
                active: false,
            });
            p.phase = phase.into();
            p.message = message;
            p.error = error;
            p.active = false;
            *svc_task.install_task.lock() = None;
            svc_task.set_install(Some(p));
        };
        match result {
            Ok(outcome) => {
                finish("ready", "AI Ready".into(), None);
                Ok(json!(outcome))
            }
            Err(e) if e.code_str() == "internal.cancelled" => {
                if discard_task.load(std::sync::atomic::Ordering::SeqCst) {
                    mgr.discard_partials();
                    finish(
                        "cancelled",
                        "Download cancelled. Nothing was installed.".into(),
                        None,
                    );
                } else {
                    finish(
                        "paused",
                        "Paused. You can resume the download at any time.".into(),
                        None,
                    );
                }
                Err(e)
            }
            Err(e) => {
                finish("failed", e.message.clone(), Some(e.clone()));
                Err(e)
            }
        }
    });
    *running = Some((task_id.clone(), discard));
    Ok(AiInstallStarted { task_id })
}

/// Pause (keep the partial download) or cancel (discard it).
fn cancel_install(core: &AppCore, _actor: &Actor, args: AiCancelInstallArgs) -> AppResult<()> {
    let svc = service(core);
    let running = svc.install_task.lock().clone();
    match running {
        Some((task_id, discard)) => {
            discard.store(args.discard, std::sync::atomic::Ordering::SeqCst);
            core.tasks.cancel(&task_id);
        }
        None => {
            if args.discard {
                svc.manager()?.discard_partials();
                let mut p = svc.install.lock().clone();
                if let Some(p) = p.as_mut() {
                    p.phase = "cancelled".into();
                    p.message = "Download cancelled. Nothing was installed.".into();
                    p.active = false;
                    p.error = None;
                }
                svc.set_install(p);
            }
        }
    }
    Ok(())
}

fn remove_model(core: &AppCore, _actor: &Actor, args: AiProfileArgs) -> AppResult<()> {
    let svc = service(core);
    svc.manager()?.remove_model(&args.profile_id)?;
    svc.set_install(None);
    Ok(())
}

fn stop_runtime(core: &AppCore, _actor: &Actor, _: AiEmptyArgs) -> AppResult<()> {
    let svc = service(core);
    svc.manager()?.stop_runtime();
    Ok(())
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
    args: AiChangeSetArgs,
) -> AppResult<ChangeSetDto> {
    change_set::accept(core, actor, &args.id)
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

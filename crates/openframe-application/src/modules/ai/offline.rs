//! Offline AI installation and status operations (agentic AI spec §1, §12, §23–§25, §36;
//! Local AI Runtime spec). Application-level: no project is needed, and nothing else in
//! OpenFrame depends on these operations (OpenFrame stays fully usable without AI).
//!
//! - `ai.status` — simple state for ordinary UI ("Offline AI" / "AI Ready"; no model names);
//! - `ai.setup_info` — exact download size, disk preflight and device check, shown BEFORE the
//!   download starts;
//! - `ai.install` — one click: downloads runtime + chat model + embedding model with one
//!   combined progress, verifies, health-checks and activates (usable without restart);
//! - `ai.cancel_install` — pause (keep partial downloads) or cancel (discard them);
//! - `ai.uninstall` — remove Offline AI from this computer;
//! - `ai.stop_runtime` — free memory (the sidecars start again on the next request);
//! - `ai.diagnostics` — technical details for Settings → Offline AI (model names, versions,
//!   licences, hashes, runtime backend, hardware).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use openframe_ai::download::Control;
use openframe_ai::selection::human_size;
use openframe_ai::{ComponentKind, InstallPhase};
use openframe_domain::{Actor, AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::service::{self, InstallProgress, service};
use crate::core::AppCore;

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiCancelInstallArgs {
    /// false = Pause (keep the partial download to resume later); true = Cancel (discard it).
    #[serde(default)]
    pub discard: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiStatusDto {
    /// Offline AI is installed, verified and active.
    pub installed: bool,
    /// Header chip: "AI Ready" or "Off".
    pub mode: String,
    /// NotInstalled | Installed | Starting | LoadingModel | Ready | Busy | Stopping | Failed
    pub runtime_state: String,
    pub runtime_message: Option<String>,
    pub install: Option<InstallProgress>,
    /// Offline AI processes everything on this computer.
    pub local_only: bool,
    /// A newer verified Offline AI package is available.
    pub update_available: bool,
    /// Disk space used by the downloaded AI components.
    #[ts(type = "number")]
    pub installed_bytes: u64,
}

/// Everything shown before the user clicks "Download Offline AI".
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiSetupInfoDto {
    /// Offline AI can be installed on this kind of computer.
    pub supported: bool,
    /// Why not, in plain language (when unsupported).
    pub message: Option<String>,
    /// Exact bytes still to download (resumed parts excluded).
    #[ts(type = "number")]
    pub download_bytes: u64,
    /// Full size of what will be installed.
    #[ts(type = "number")]
    pub total_bytes: u64,
    /// Bytes already downloaded by a paused download.
    #[ts(type = "number")]
    pub downloaded_bytes: u64,
    #[ts(type = "number")]
    pub required_free_bytes: u64,
    #[ts(type = "number | null")]
    pub free_disk_bytes: Option<u64>,
    pub enough_disk: bool,
    /// "Graphics card" | "Processor"
    pub runs_on: String,
    pub likely_slow: bool,
    pub warnings: Vec<String>,
    /// The installed Offline AI is already this version (nothing to download).
    pub up_to_date: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiComponentDto {
    /// "AI engine" | "Language model" | "Search model"
    pub role: String,
    /// Technical name (diagnostics only).
    pub name: String,
    pub id: String,
    pub version: String,
    pub license_id: String,
    pub license_url: Option<String>,
    #[ts(type = "number")]
    pub bytes: u64,
    pub sha256: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiGpuDto {
    pub name: String,
    #[ts(type = "number")]
    pub memory_bytes: u64,
}

/// Technical details for Settings → Offline AI (never shown in ordinary workflow UI).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiDiagnosticsDto {
    pub profile_id: String,
    pub installed_version: Option<String>,
    pub available_version: Option<String>,
    pub manifest_channel: Option<String>,
    #[ts(type = "number | null")]
    pub manifest_sequence: Option<u64>,
    /// This build trusts the development signing key (development builds only).
    pub development_key: bool,
    /// "Graphics card" | "Processor" (when installed)
    pub runs_on: Option<String>,
    pub components: Vec<AiComponentDto>,
    pub chat_state: String,
    pub search_state: String,
    #[ts(type = "number")]
    pub memory_bytes: u64,
    pub processor: String,
    pub processor_threads: u32,
    pub graphics: Vec<AiGpuDto>,
    pub vulkan_available: bool,
    #[ts(type = "number | null")]
    pub free_disk_bytes: Option<u64>,
    pub store_dir: String,
    pub log_file: String,
    #[ts(type = "number")]
    pub installed_bytes: u64,
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
pub struct AiUninstallResult {
    #[ts(type = "number")]
    pub freed_bytes: u64,
    pub freed_label: String,
}

// ------------------------------------------------------------------ handlers

pub fn status(core: &AppCore, _actor: &Actor, _: super::AiEmptyArgs) -> AppResult<AiStatusDto> {
    let svc = service(core);
    let install = svc.install.lock().clone();
    let attached = svc.has_attached_model();
    let Ok(m) = svc.manager() else {
        return Ok(AiStatusDto {
            installed: attached,
            mode: if attached { "AI Ready".into() } else { "Off".into() },
            runtime_state: if attached { "Ready".into() } else { "Failed".into() },
            runtime_message: Some(
                "AI is currently unavailable. OpenFrame's core workflows continue to work normally."
                    .into(),
            ),
            install,
            local_only: true,
            update_available: false,
            installed_bytes: 0,
        });
    };
    let st = m.state();
    let rs = m.runtime_status();
    let installed = st.installed || attached;
    let state = if attached && rs.state == openframe_ai::RuntimeState::NotInstalled {
        "Ready".to_string()
    } else {
        format!("{:?}", rs.state)
    };
    Ok(AiStatusDto {
        installed,
        mode: if installed {
            "AI Ready".into()
        } else {
            "Off".into()
        },
        runtime_state: state,
        runtime_message: rs.last_error,
        install,
        local_only: true,
        update_available: st.update_available,
        installed_bytes: st.installed_bytes,
    })
}

pub fn setup_info(
    core: &AppCore,
    _actor: &Actor,
    _: super::AiEmptyArgs,
) -> AppResult<AiSetupInfoDto> {
    let svc = service(core);
    let m = svc.manager()?;
    // A fresh device check every time the setup view opens.
    m.hardware(true);
    let plan = match m.plan() {
        Ok(p) => p,
        Err(e) if e.code_str() == "ai.unsupported" => {
            return Ok(AiSetupInfoDto {
                supported: false,
                message: Some(e.message),
                download_bytes: 0,
                total_bytes: 0,
                downloaded_bytes: 0,
                required_free_bytes: 0,
                free_disk_bytes: None,
                enough_disk: false,
                runs_on: String::new(),
                likely_slow: false,
                warnings: Vec::new(),
                up_to_date: false,
            });
        }
        Err(e) => return Err(e),
    };
    Ok(AiSetupInfoDto {
        supported: true,
        message: None,
        download_bytes: plan.download_bytes,
        total_bytes: plan.total_bytes,
        downloaded_bytes: plan.downloaded_bytes,
        required_free_bytes: plan.required_free_bytes,
        free_disk_bytes: plan.free_disk_bytes,
        enough_disk: plan.enough_disk,
        runs_on: plan.backend.label().into(),
        likely_slow: plan.fitness.likely_slow,
        warnings: plan.fitness.warnings,
        up_to_date: plan.up_to_date,
    })
}

fn blank_progress(task_id: &str) -> InstallProgress {
    InstallProgress {
        task_id: task_id.to_string(),
        phase: String::new(),
        bytes_done: 0,
        bytes_total: 0,
        message: String::new(),
        error: None,
        active: false,
    }
}

/// Start (or resume) installing Offline AI in the background: one button, no choices.
/// Installing the application's AI is not a project edit (no project needed).
pub fn install(
    core: &AppCore,
    _actor: &Actor,
    _: super::AiEmptyArgs,
) -> AppResult<AiInstallStarted> {
    let svc = service(core);
    let mgr = svc.manager()?.clone();
    // Held across spawning so the task can only clear its registration after it exists.
    let mut running = svc.install_task.lock();
    if let Some((id, _)) = running.as_ref() {
        return Ok(AiInstallStarted {
            task_id: id.clone(),
        });
    }
    let discard = Arc::new(AtomicBool::new(false));
    let svc_task = svc.clone();
    let discard_task = discard.clone();
    let task_id = core.spawn_task("ai.install", "Downloading Offline AI", move |_core, h| {
        let (phase, message) = service::phase_text(InstallPhase::Checking);
        svc_task.set_install(Some(InstallProgress {
            phase: phase.into(),
            message: message.into(),
            active: true,
            ..blank_progress(&h.id)
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
        let result = mgr.install(&control, &events);
        let finish = |phase: &str, message: String, error: Option<AppError>| {
            let mut p = svc_task
                .install
                .lock()
                .clone()
                .unwrap_or_else(|| blank_progress(&h.id));
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
                Ok(json!({
                    "profileVersion": outcome.profile_version,
                    "runsOn": outcome.backend.label(),
                }))
            }
            Err(e) if e.code_str() == "internal.cancelled" => {
                if discard_task.load(Ordering::SeqCst) {
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
pub fn cancel_install(core: &AppCore, _actor: &Actor, args: AiCancelInstallArgs) -> AppResult<()> {
    let svc = service(core);
    let running = svc.install_task.lock().clone();
    match running {
        Some((task_id, discard)) => {
            discard.store(args.discard, Ordering::SeqCst);
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

/// Remove Offline AI from this computer. Projects and assistant history are not affected.
pub fn uninstall(
    core: &AppCore,
    _actor: &Actor,
    _: super::AiEmptyArgs,
) -> AppResult<AiUninstallResult> {
    let svc = service(core);
    let freed = svc.manager()?.uninstall()?;
    svc.set_install(None);
    Ok(AiUninstallResult {
        freed_bytes: freed,
        freed_label: human_size(freed),
    })
}

pub fn stop_runtime(core: &AppCore, _actor: &Actor, _: super::AiEmptyArgs) -> AppResult<()> {
    let svc = service(core);
    svc.manager()?.stop_runtime();
    Ok(())
}

fn role_label(k: ComponentKind) -> &'static str {
    match k {
        ComponentKind::Runtime => "AI engine",
        ComponentKind::ChatModel => "Language model",
        ComponentKind::EmbeddingModel => "Search model",
    }
}

pub fn diagnostics(
    core: &AppCore,
    _actor: &Actor,
    _: super::AiEmptyArgs,
) -> AppResult<AiDiagnosticsDto> {
    let svc = service(core);
    let d = svc.manager()?.diagnostics();
    Ok(AiDiagnosticsDto {
        profile_id: d.profile_id,
        installed_version: d.profile_version,
        available_version: d.available_version,
        manifest_channel: d.manifest_channel,
        manifest_sequence: d.manifest_sequence,
        development_key: d.development_key,
        runs_on: d.backend.map(|b| b.label().to_string()),
        components: d
            .components
            .into_iter()
            .map(|c| AiComponentDto {
                role: role_label(c.kind).into(),
                name: c.name,
                id: c.id,
                version: c.version,
                license_id: c.license_id,
                license_url: c.license_url,
                bytes: c.bytes,
                sha256: c.sha256,
                installed: c.installed,
            })
            .collect(),
        chat_state: format!("{:?}", d.chat_runtime.state),
        search_state: format!("{:?}", d.embedding_runtime.state),
        memory_bytes: d.hardware.total_ram_bytes,
        processor: if d.hardware.cpu_brand.is_empty() {
            "Processor".into()
        } else {
            d.hardware.cpu_brand.clone()
        },
        processor_threads: d.hardware.logical_cpus as u32,
        graphics: d
            .hardware
            .gpus
            .iter()
            .filter(|g| !g.software)
            .map(|g| AiGpuDto {
                name: g.name.clone(),
                memory_bytes: g.dedicated_vram_bytes,
            })
            .collect(),
        vulkan_available: d.hardware.vulkan_available,
        free_disk_bytes: d.hardware.free_disk_bytes,
        store_dir: d.store_dir,
        log_file: d.log_file,
        installed_bytes: d.installed_bytes,
    })
}

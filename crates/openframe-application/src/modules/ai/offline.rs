//! Offline AI status and download operations (agentic AI spec §1, §12, §36; Local AI Runtime spec;
//! ADR-0026). Application-level: no project is needed, and nothing else in OpenFrame depends on
//! these operations (OpenFrame stays fully usable without AI).
//!
//! - `ai.status`: simple state for ordinary UI ("AI Ready" / "Off"; no model names), plus the
//!   plain-language reason when the AI is not downloaded, missing or damaged;
//! - `ai.download.status`: what the Download OpenFrame AI screen shows (size and free space
//!   before the click, without any network use; progress after it);
//! - `ai.download.start` / `ai.download.pause`: the person's click; the only network use of
//!   the app;
//! - `ai.download.remove`: delete the downloaded models (AI is off until the next download);
//! - `ai.stop_runtime`: free memory (the engines start again on the next request);
//! - `ai.diagnostics`: technical details for Settings > Offline AI (model names, versions,
//!   licences, hashes, where the bundle is installed, this computer).

use openframe_ai::{BundleState, ComponentKind, DownloadPhase, RuntimeState};
use openframe_domain::{Actor, AppError, AppResult};
use serde::Serialize;
use ts_rs::TS;

use super::service::service;
use crate::core::AppCore;

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiStatusDto {
    /// Offline AI can be used: the bundled AI is present and not known to be damaged.
    pub available: bool,
    /// Header chip: "AI Ready" or "Off".
    pub mode: String,
    /// NotInstalled | NotChecked | Checking | Verified | Missing | Damaged | Unsupported
    pub bundle_state: String,
    /// Why AI is off (not downloaded, missing, damaged or unsupported), in plain language.
    pub problem: Option<AppError>,
    /// Unavailable | Installed | Starting | LoadingModel | Ready | Busy | Stopping | Failed
    pub runtime_state: String,
    /// Plain-language reason when the engine stopped after a problem.
    pub runtime_message: Option<String>,
    /// Offline AI processes everything on this computer.
    pub local_only: bool,
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
}

/// The Download OpenFrame AI screen.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiDownloadDto {
    /// This build has a model host (a release build always has).
    pub can_download: bool,
    /// Idle | Checking | Downloading | Installing | Paused | Failed | Done
    pub phase: String,
    /// The size of the download, known before the click (from the compiled lock).
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// Free space the download needs: its size plus a safety margin.
    #[ts(type = "number")]
    pub needed_bytes: u64,
    /// Free space on the drive that holds the models folder, when Windows can say.
    #[ts(type = "number | null")]
    pub free_bytes: Option<u64>,
    #[ts(type = "number")]
    pub done_bytes: u64,
    #[ts(type = "number")]
    pub total_bytes: u64,
    /// Why the last download stopped, in plain language.
    pub problem: Option<AppError>,
}

/// Technical details for Settings > Offline AI (never shown in ordinary workflow UI).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiDiagnosticsDto {
    pub bundle_id: String,
    pub bundle_version: String,
    /// NotInstalled | NotChecked | Checking | Verified | Missing | Damaged | Unsupported
    pub bundle_state: String,
    pub problem: Option<String>,
    pub components: Vec<AiComponentDto>,
    pub chat_state: String,
    pub search_state: String,
    #[ts(type = "number")]
    pub memory_bytes: u64,
    pub processor: String,
    pub processor_threads: u32,
    /// Answers may be slow on this computer.
    pub likely_slow: bool,
    pub warnings: Vec<String>,
    /// Read-only folder of the bundled AI in this installation.
    pub bundle_dir: String,
    #[ts(type = "number")]
    pub bundle_bytes: u64,
    pub log_file: String,
}

// ------------------------------------------------------------------ handlers

/// The message and code only: technical detail (file names, folders) stays in the log.
fn plain(mut e: AppError) -> AppError {
    e.detail = None;
    e
}

fn bundle_label(b: BundleState) -> String {
    format!("{b:?}")
}

pub fn status(core: &AppCore, _actor: &Actor, _: super::AiEmptyArgs) -> AppResult<AiStatusDto> {
    let svc = service(core);
    let attached = svc.has_attached_model();
    let Ok(m) = svc.manager() else {
        return Ok(AiStatusDto {
            available: attached,
            mode: if attached {
                "AI Ready".into()
            } else {
                "Off".into()
            },
            bundle_state: if attached {
                "Verified".into()
            } else {
                "Missing".into()
            },
            problem: (!attached).then(|| plain(svc.unavailable_reason())),
            runtime_state: if attached {
                "Ready".into()
            } else {
                "Unavailable".into()
            },
            runtime_message: None,
            local_only: true,
        });
    };
    let st = m.state();
    let rs = m.runtime_status();
    let available = st.available || attached;
    let runtime_state = if attached && rs.state == RuntimeState::Unavailable {
        "Ready".to_string()
    } else {
        format!("{:?}", rs.state)
    };
    Ok(AiStatusDto {
        available,
        mode: if available {
            "AI Ready".into()
        } else {
            "Off".into()
        },
        bundle_state: bundle_label(st.bundle),
        problem: if attached {
            None
        } else {
            st.problem.map(plain)
        },
        runtime_state,
        runtime_message: rs.last_error,
        local_only: true,
    })
}

pub fn download_status(
    core: &AppCore,
    _actor: &Actor,
    _: super::AiEmptyArgs,
) -> AppResult<AiDownloadDto> {
    let m = service(core).manager()?.clone();
    let st = m.download_status();
    let size = m.download_size();
    // The drive of the models folder (or of app data before the first download).
    let models = core.config.app_data_dir.join("models");
    let probe = if models.is_dir() {
        models
    } else {
        core.config.app_data_dir.clone()
    };
    Ok(AiDownloadDto {
        can_download: m.can_download(),
        phase: format!("{:?}", st.phase),
        size_bytes: size,
        needed_bytes: size + openframe_ai::install::PREFLIGHT_MARGIN_BYTES,
        free_bytes: openframe_platform::drives::free_bytes(&probe),
        done_bytes: st.done_bytes,
        total_bytes: st.total_bytes,
        problem: if st.phase == DownloadPhase::Failed {
            st.problem.map(plain)
        } else {
            None
        },
    })
}

pub fn download_start(core: &AppCore, _actor: &Actor, _: super::AiEmptyArgs) -> AppResult<()> {
    service(core).manager()?.start_download()
}

pub fn download_pause(core: &AppCore, _actor: &Actor, _: super::AiEmptyArgs) -> AppResult<()> {
    service(core).manager()?.pause_download();
    Ok(())
}

pub fn download_remove(core: &AppCore, _actor: &Actor, _: super::AiEmptyArgs) -> AppResult<()> {
    service(core).manager()?.remove_models().map_err(plain)
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
        bundle_id: d.bundle_id,
        bundle_version: d.bundle_version,
        bundle_state: bundle_label(d.bundle),
        problem: d.problem,
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
        likely_slow: d.fitness.likely_slow,
        warnings: d.fitness.warnings,
        bundle_dir: d.bundle_dir,
        bundle_bytes: d.bundle_bytes,
        log_file: d.log_file,
    })
}

//! Download after install, the whole flow (ADR-0026 decisions 3 to 5, ADR-0032; Windows phase W8).
//!
//! ```text
//! click ─► preflight (free space) ─► Downloading: each pinned file from Hugging Face to .partial,
//!           resumable, then its size and SHA-256 against the pins (a bad file is quarantined)
//!        ─► Installing: activate the set
//!        ─► the app reloads and checks the set, starts the engine on it (the health check)
//!           ├─ healthy ─► Done (the previous set is removed)
//!           └─ not    ─► rolled back to the previous set, or to none; Failed with a plain message
//! pause ─► Paused (the .partial files stay; the next start resumes them)
//! ```
//!
//! Nothing here starts on its own: the flow runs only from `AiManager::start_download`, which the
//! person's click calls. What to fetch and what to trust come from the compiled lock
//! (`download::pins`); where from is `fetch::ModelSource`.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use openframe_domain::{AppError, AppResult};
use parking_lot::Mutex;
use serde::Serialize;

use crate::download::Tier;
use crate::fetch::{self, ModelSource};
use crate::install::ModelsDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadPhase {
    Idle,
    Checking,
    Downloading,
    Installing,
    Paused,
    Failed,
    Done,
}

/// What the download screen shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStatus {
    pub phase: DownloadPhase,
    pub done_bytes: u64,
    pub total_bytes: u64,
    /// The plain-language reason after Failed.
    pub problem: Option<AppError>,
}

/// The one download of this app session: its status and its pause switch.
pub struct ModelDownload {
    status: Mutex<DownloadStatus>,
    cancel: AtomicBool,
    running: AtomicBool,
}

impl Default for ModelDownload {
    fn default() -> Self {
        ModelDownload {
            status: Mutex::new(DownloadStatus {
                phase: DownloadPhase::Idle,
                done_bytes: 0,
                total_bytes: 0,
                problem: None,
            }),
            cancel: AtomicBool::new(false),
            running: AtomicBool::new(false),
        }
    }
}

impl ModelDownload {
    pub fn status(&self) -> DownloadStatus {
        self.status.lock().clone()
    }

    /// Claim the download for one run; `false` when one is already running.
    pub fn begin(&self) -> bool {
        if self.running.swap(true, Ordering::SeqCst) {
            return false;
        }
        self.cancel.store(false, Ordering::SeqCst);
        self.set(DownloadPhase::Checking, None);
        true
    }

    /// Hold the download slot for another change to the models folder (Remove), so a download
    /// cannot start in the middle of it. `false` when a download is running.
    pub fn claim(&self) -> bool {
        !self.running.swap(true, Ordering::SeqCst)
    }

    /// Give back the slot taken by `claim`.
    pub fn release(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// The run ended: record how.
    pub fn end(&self, r: &AppResult<()>) {
        match r {
            Ok(()) => self.set(DownloadPhase::Done, None),
            Err(e) if e.is("ai.download_paused") => self.set(DownloadPhase::Paused, None),
            Err(e) => self.set(DownloadPhase::Failed, Some(e.clone())),
        }
        self.running.store(false, Ordering::SeqCst);
    }

    /// Ask a running download to stop; what is already on disk stays for the next start.
    pub fn pause(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        if !self.running.load(Ordering::SeqCst) {
            self.set(DownloadPhase::Idle, None);
        }
    }

    fn set(&self, phase: DownloadPhase, problem: Option<AppError>) {
        let mut s = self.status.lock();
        s.phase = phase;
        s.problem = problem;
    }

    fn progress(&self, done: u64, total: u64) {
        let mut s = self.status.lock();
        s.done_bytes = done;
        s.total_bytes = total;
    }

    /// Download and check every file of `tier` into staging, then activate the set as `version`.
    /// Files already checked in staging (from an earlier download or an import) are kept. The
    /// caller reloads the app's AI and runs the health check.
    pub async fn fetch_files(
        &self,
        source: &ModelSource,
        dir: &ModelsDir,
        version: &str,
        tier: &Tier,
    ) -> AppResult<()> {
        std::fs::create_dir_all(dir.staging(version)).map_err(|e| write_failed(e.kind()))?;
        dir.sweep_staging(version);
        dir.preflight(
            version,
            tier,
            openframe_platform::drives::free_bytes(&dir.root),
        )?;
        let http = fetch::client(source)?;
        let total = tier.total_bytes();
        self.set(DownloadPhase::Downloading, None);
        let mut finished = 0u64;
        for f in &tier.files {
            if dir.checked(version, f).is_file() {
                finished += f.bytes;
                self.progress(finished, total);
                continue;
            }
            let base = finished;
            let progress = |held: u64| self.progress(base + held, total);
            fetch::download_file(
                &http,
                source,
                f,
                &dir.partial(version, f),
                &progress,
                &self.cancel,
            )
            .await?;
            if self.cancel.load(Ordering::SeqCst) {
                return Err(fetch::cancelled());
            }
            // Hashing a large file blocks: off the runtime that serves the engines.
            let (d, v, e) = (
                ModelsDir::new(dir.root.clone()),
                version.to_string(),
                f.clone(),
            );
            tokio::task::spawn_blocking(move || d.check_file(&v, &e))
                .await
                .map_err(|_| AppError::internal("the file check stopped"))??;
            finished += f.bytes;
            self.progress(finished, total);
        }
        self.set(DownloadPhase::Installing, None);
        dir.activate(version, tier)?;
        Ok(())
    }
}

fn write_failed(kind: std::io::ErrorKind) -> AppError {
    AppError::ai("write_failed", "OpenFrame AI could not be saved.").with_detail(kind.to_string())
}

/// `<app data>/models`.
pub fn models_dir(app_data_dir: &Path) -> ModelsDir {
    ModelsDir::new(app_data_dir.join("models"))
}

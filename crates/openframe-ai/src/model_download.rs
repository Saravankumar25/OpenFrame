//! Download after install, the whole flow (ADR-0026 decisions 3 to 5; Windows phase W8).
//!
//! ```text
//! click ─► Checking: fetch the manifest, check its signature, pick the tier, check the engine
//!        ─► preflight (free space) ─► Downloading: each file to .partial, resumable, then its
//!           size and SHA-256 (a bad file is quarantined) ─► Installing: activate the set
//!        ─► the app reloads and checks the set, starts the engine on it (the health check)
//!           ├─ healthy ─► Done (the previous set is removed)
//!           └─ not    ─► rolled back to the previous set, or to none; Failed with a plain message
//! pause ─► Paused (the .partial files stay; the next start resumes them)
//! ```
//!
//! Nothing here starts on its own: the flow runs only from `AiManager::start_download`, which the
//! person's click calls. The model host and keys are fixed at build time (`ModelHost`).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use base64::Engine as _;
use ed25519_dalek::VerifyingKey;
use openframe_domain::{AppError, AppResult};
use parking_lot::Mutex;
use serde::Serialize;

use crate::download::{self, Manifest, Tier};
use crate::fetch;
use crate::install::ModelsDir;

/// The manifest's name on the model host, next to the files it lists.
pub const MANIFEST_NAME: &str = "manifest.json";
/// The model profile this build runs (ADR-0013 format 2): context window, vector size and pooling
/// in the compiled lock. A manifest for another profile needs another app version.
pub const PROFILE_VERSION: u32 = 2;

/// The model host and the manifest keys this build was made with (ADR-0026 decision 8).
#[derive(Debug, Clone)]
pub struct ModelHost {
    pub base: String,
    pub keys: Vec<VerifyingKey>,
}

impl ModelHost {
    /// The build values: an `https://` address (or the loopback test host) and one or more
    /// base64 Ed25519 public keys separated by commas. `None` when either is missing or invalid.
    pub fn from_build(base: Option<&str>, keys: Option<&str>) -> Option<ModelHost> {
        let base = base?.trim();
        let b64 = base64::engine::general_purpose::STANDARD;
        let keys: Vec<VerifyingKey> = keys?
            .split(',')
            .map(|k| {
                let raw: [u8; 32] = b64.decode(k.trim()).ok()?.try_into().ok()?;
                VerifyingKey::from_bytes(&raw).ok()
            })
            .collect::<Option<_>>()?;
        (!base.is_empty() && !keys.is_empty() && fetch::Source::new(base, true).is_ok()).then(
            || ModelHost {
                base: base.to_string(),
                keys,
            },
        )
    }
}

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

    /// Fetch and check the signed manifest and pick the tier this build can run. Returns the
    /// manifest bytes (kept with the set), the manifest and the tier id.
    pub async fn fetch_plan(
        &self,
        host: &ModelHost,
        runtime_version: &str,
    ) -> AppResult<(Vec<u8>, Manifest, String)> {
        let source = fetch::Source::new(&host.base, true)?;
        let http = fetch::client(&source)?;
        let envelope = fetch::fetch_manifest(&http, &source, MANIFEST_NAME).await?;
        let m = download::verify(&envelope, &host.keys)?;
        if m.profile_version != PROFILE_VERSION {
            return Err(AppError::ai(
                "needs_update",
                "This OpenFrame AI download needs a newer version of OpenFrame Studio. Nothing was downloaded.",
            )
            .with_detail(format!("profile {}", m.profile_version)));
        }
        // shortcut: only the small tier runs on this build (its models are the ones the compiled
        // lock describes); the larger tier is offered once W6 picks its model (ADR-0028 decision 4).
        let tier = m
            .tier("small")
            .ok_or_else(|| download_refused("the manifest has no standard tier"))?;
        if tier.requires_runtime != runtime_version {
            return Err(AppError::ai(
                "needs_update",
                "This OpenFrame AI download needs a newer version of OpenFrame Studio. Nothing was downloaded.",
            )
            .with_detail(format!(
                "needs engine {}, this app has {runtime_version}",
                tier.requires_runtime
            )));
        }
        let id = tier.id.clone();
        Ok((envelope, m, id))
    }

    /// Download and check every file of `tier` into staging, then activate the set. The caller
    /// reloads the app's AI and runs the health check.
    pub async fn fetch_files(
        &self,
        host: &ModelHost,
        dir: &ModelsDir,
        envelope: &[u8],
        m: &Manifest,
        tier: &Tier,
    ) -> AppResult<()> {
        let version = m.manifest_version.as_str();
        std::fs::create_dir_all(&dir.root).map_err(|e| write_failed(e.kind()))?;
        dir.sweep_staging(version);
        dir.preflight(
            version,
            tier,
            openframe_platform::drives::free_bytes(&dir.root),
        )?;
        dir.stage_manifest(version, envelope)?;
        let source = fetch::Source::new(&host.base, true)?;
        let http = fetch::client(&source)?;
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
                &source,
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
        dir.activate(version, tier, m.sequence)?;
        Ok(())
    }
}

fn download_refused(detail: &str) -> AppError {
    AppError::ai(
        "manifest_invalid",
        "The OpenFrame AI download list could not be checked, so nothing was downloaded. Try again later.",
    )
    .with_detail(detail.to_string())
}

fn write_failed(kind: std::io::ErrorKind) -> AppError {
    AppError::ai("write_failed", "OpenFrame AI could not be saved.").with_detail(kind.to_string())
}

/// `<app data>/models`.
pub fn models_dir(app_data_dir: &Path) -> ModelsDir {
    ModelsDir::new(app_data_dir.join("models"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_build_values_need_an_https_or_loopback_host_and_valid_keys() {
        let key = base64::engine::general_purpose::STANDARD.encode(
            crate::download::tests::test_key()
                .verifying_key()
                .to_bytes(),
        );
        let ok = |b: &str, k: &str| ModelHost::from_build(Some(b), Some(k)).is_some();
        assert!(ok("https://models.example.com/ai/", &key));
        assert!(ok("http://127.0.0.1:47811/", &key));
        assert!(ok("https://models.example.com/", &format!("{key}, {key}")));
        assert!(
            !ok("http://models.example.com/", &key),
            "plain HTTP elsewhere"
        );
        assert!(!ok("https://models.example.com/", "not a key"));
        assert!(!ok("https://models.example.com/", ""));
        assert!(!ok("", &key));
        assert!(ModelHost::from_build(None, Some(&key)).is_none());
        assert!(ModelHost::from_build(Some("https://models.example.com/"), None).is_none());
    }
}

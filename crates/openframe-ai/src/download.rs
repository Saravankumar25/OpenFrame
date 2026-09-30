//! Resumable, verified downloads (Local AI Runtime spec §7).
//!
//! - writes to a `.partial` file; resumes with HTTP range requests when the
//!   server supports them (falls back to a clean restart when it doesn't);
//! - remembers what the partial file is for (`.partial.json`): a partial for a
//!   different URL/hash is discarded rather than spliced;
//! - retries transient failures with exponential backoff;
//! - pause/cancel through a shared flag (the partial file is kept for resume);
//! - verifies size + SHA-256 before the file is moved (atomic rename) to its
//!   destination; a mismatch is quarantined and never used.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use openframe_domain::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::store::{read_json, write_json_atomic};

#[derive(Debug, Clone)]
pub struct DownloadSpec {
    pub url: String,
    pub bytes: u64,
    pub sha256: String,
    pub partial: PathBuf,
    pub dest: PathBuf,
    /// Where a file that fails verification is moved.
    pub quarantine_dir: PathBuf,
    /// Short label for quarantine file names / logs (no user content).
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub max_attempts: u32,
    pub base_backoff: Duration,
    pub max_backoff: Duration,
    /// Abort a stalled connection after this long without data.
    pub stall_timeout: Duration,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            max_attempts: 6,
            base_backoff: Duration::from_secs(2),
            max_backoff: Duration::from_secs(60),
            stall_timeout: Duration::from_secs(60),
        }
    }
}

/// Shared pause/cancel control. Pausing and cancelling both stop the transfer
/// and keep the partial file; the caller decides whether to discard it.
#[derive(Debug, Clone, Default)]
pub struct Control {
    stop: Arc<AtomicBool>,
}

impl Control {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_flag(flag: Arc<AtomicBool>) -> Self {
        Self { stop: flag }
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
    pub fn check(&self) -> AppResult<()> {
        if self.is_stopped() {
            Err(AppError::cancelled())
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadPhase {
    Transferring,
    Verifying,
}

/// Progress callback: (phase, bytes done, bytes total).
pub type ProgressFn<'a> = &'a (dyn Fn(DownloadPhase, u64, u64) + Send + Sync);

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct PartialInfo {
    url: String,
    sha256: String,
    bytes: u64,
}

fn partial_info_path(partial: &Path) -> PathBuf {
    let mut s = partial.as_os_str().to_owned();
    s.push(".json");
    PathBuf::from(s)
}

pub fn integrity_error() -> AppError {
    AppError::ai(
        "integrity",
        "The download didn't pass OpenFrame's safety check, so it wasn't used. Any AI model you already had is still in place. You can try again.",
    )
    .retryable()
}

fn unavailable(detail: impl Into<String>) -> AppError {
    AppError::ai(
        "download_unavailable",
        "The Offline AI download isn't available right now. Please try again later.",
    )
    .with_detail(detail)
}

/// Remove a partial download and its bookkeeping.
pub fn discard_partial(partial: &Path) {
    let _ = std::fs::remove_file(partial);
    let _ = std::fs::remove_file(partial_info_path(partial));
}

/// Bytes already downloaded for this exact spec (0 if none or for another file).
pub fn partial_len(spec: &DownloadSpec) -> u64 {
    let info: Option<PartialInfo> = read_json(&partial_info_path(&spec.partial));
    match info {
        Some(i) if i.url == spec.url && i.sha256 == spec.sha256 && i.bytes == spec.bytes => {
            std::fs::metadata(&spec.partial)
                .map(|m| m.len())
                .unwrap_or(0)
        }
        _ => 0,
    }
}

enum Attempt {
    Complete,
    /// Transient: retry after backoff (message is technical detail).
    Retry(String),
}

/// Download, verify and atomically move into place. Returns the destination.
pub async fn download_verified(
    client: &reqwest::Client,
    spec: &DownloadSpec,
    opts: &DownloadOptions,
    control: &Control,
    progress: ProgressFn<'_>,
) -> AppResult<PathBuf> {
    if let Some(parent) = spec.partial.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = spec.dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // A partial for a different file is never spliced into this one.
    let info_path = partial_info_path(&spec.partial);
    let expected_info = PartialInfo {
        url: spec.url.clone(),
        sha256: spec.sha256.clone(),
        bytes: spec.bytes,
    };
    if read_json::<PartialInfo>(&info_path).as_ref() != Some(&expected_info) {
        discard_partial(&spec.partial);
    }
    write_json_atomic(&info_path, &expected_info)?;

    let mut attempt = 0u32;
    let mut last_error = String::new();
    loop {
        control.check()?;
        match transfer_once(client, spec, opts, control, progress).await? {
            Attempt::Complete => break,
            Attempt::Retry(msg) => {
                attempt += 1;
                tracing::warn!(label = %spec.label, attempt, error = %openframe_security::redact(&msg), "download interrupted; will retry");
                last_error = msg;
                if attempt >= opts.max_attempts {
                    return Err(AppError::network(last_error));
                }
                let delay = opts
                    .base_backoff
                    .saturating_mul(1u32 << (attempt - 1).min(10))
                    .min(opts.max_backoff);
                sleep_cancellable(delay, control).await?;
            }
        }
    }
    let _ = last_error;

    // Verify before activation.
    progress(DownloadPhase::Verifying, 0, spec.bytes);
    let partial = spec.partial.clone();
    let sha = tokio::task::spawn_blocking(move || hash_file(&partial))
        .await
        .map_err(|e| AppError::internal(e.to_string()))??;
    let size = std::fs::metadata(&spec.partial)?.len();
    if size != spec.bytes || sha != spec.sha256 {
        let q = quarantine(&spec.partial, &spec.quarantine_dir, &spec.label)?;
        let _ = std::fs::remove_file(&info_path);
        tracing::warn!(label = %spec.label, quarantined = %q.display(), "download failed verification");
        return Err(integrity_error().with_detail(format!(
            "expected {} bytes sha256 {}, got {size} bytes sha256 {sha}",
            spec.bytes, spec.sha256
        )));
    }
    progress(DownloadPhase::Verifying, spec.bytes, spec.bytes);
    if spec.dest.exists() {
        std::fs::remove_file(&spec.dest)?;
    }
    std::fs::rename(&spec.partial, &spec.dest)?;
    let _ = std::fs::remove_file(&info_path);
    Ok(spec.dest.clone())
}

fn quarantine(file: &Path, dir: &Path, label: &str) -> AppResult<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let dest = dir.join(format!("{}-{label}.rejected", openframe_domain::now_ms()));
    std::fs::rename(file, &dest)?;
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        files.sort();
        while files.len() > 3 {
            let _ = std::fs::remove_file(files.remove(0));
        }
    }
    Ok(dest)
}

async fn sleep_cancellable(total: Duration, control: &Control) -> AppResult<()> {
    let step = Duration::from_millis(100);
    let mut waited = Duration::ZERO;
    while waited < total {
        control.check()?;
        tokio::time::sleep(step).await;
        waited += step;
    }
    Ok(())
}

async fn transfer_once(
    client: &reqwest::Client,
    spec: &DownloadSpec,
    opts: &DownloadOptions,
    control: &Control,
    progress: ProgressFn<'_>,
) -> AppResult<Attempt> {
    let mut have = std::fs::metadata(&spec.partial)
        .map(|m| m.len())
        .unwrap_or(0);
    if have > spec.bytes {
        discard_partial(&spec.partial);
        write_json_atomic(
            &partial_info_path(&spec.partial),
            &PartialInfo {
                url: spec.url.clone(),
                sha256: spec.sha256.clone(),
                bytes: spec.bytes,
            },
        )?;
        have = 0;
    }
    if have == spec.bytes {
        return Ok(Attempt::Complete);
    }
    let mut req = client.get(&spec.url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let resp = match tokio::time::timeout(opts.stall_timeout, req.send()).await {
        Err(_) => return Ok(Attempt::Retry("connect timeout".into())),
        Ok(Err(e)) => return Ok(Attempt::Retry(e.to_string())),
        Ok(Ok(r)) => r,
    };
    let status = resp.status();
    let append = match status.as_u16() {
        206 => {
            // Only splice when the server resumes exactly where we are.
            let start = resp
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("bytes "))
                .and_then(|v| v.split('-').next())
                .and_then(|v| v.parse::<u64>().ok());
            if start != Some(have) {
                discard_partial(&spec.partial);
                write_json_atomic(
                    &partial_info_path(&spec.partial),
                    &PartialInfo {
                        url: spec.url.clone(),
                        sha256: spec.sha256.clone(),
                        bytes: spec.bytes,
                    },
                )?;
                return Ok(Attempt::Retry(
                    "server resumed at an unexpected offset".into(),
                ));
            }
            true
        }
        200 => false,
        416 => {
            // Range not satisfiable: what we have is wrong/stale — restart cleanly.
            discard_partial(&spec.partial);
            write_json_atomic(
                &partial_info_path(&spec.partial),
                &PartialInfo {
                    url: spec.url.clone(),
                    sha256: spec.sha256.clone(),
                    bytes: spec.bytes,
                },
            )?;
            return Ok(Attempt::Retry("range not satisfiable".into()));
        }
        s if (500..600).contains(&s) || s == 408 || s == 429 => {
            return Ok(Attempt::Retry(format!("HTTP {s}")));
        }
        s => return Err(unavailable(format!("HTTP {s}"))),
    };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(append)
        .write(true)
        .truncate(!append)
        .open(&spec.partial)?;
    if append {
        file.seek(SeekFrom::End(0))?;
    }
    let mut done = if append { have } else { 0 };
    progress(DownloadPhase::Transferring, done, spec.bytes);
    let mut resp = resp;
    let mut since_flush: u64 = 0;
    loop {
        if control.is_stopped() {
            file.flush()?;
            file.sync_all()?;
            return Err(AppError::cancelled());
        }
        let chunk = match tokio::time::timeout(opts.stall_timeout, resp.chunk()).await {
            Err(_) => {
                file.sync_all()?;
                return Ok(Attempt::Retry("stalled".into()));
            }
            Ok(Err(e)) => {
                file.sync_all()?;
                return Ok(Attempt::Retry(e.to_string()));
            }
            Ok(Ok(None)) => break,
            Ok(Ok(Some(c))) => c,
        };
        if done + chunk.len() as u64 > spec.bytes {
            // More data than the manifest promised: never trust it.
            drop(file);
            quarantine(&spec.partial, &spec.quarantine_dir, &spec.label)?;
            let _ = std::fs::remove_file(partial_info_path(&spec.partial));
            return Err(integrity_error().with_detail("server sent more bytes than expected"));
        }
        file.write_all(&chunk)?;
        done += chunk.len() as u64;
        since_flush += chunk.len() as u64;
        if since_flush >= 32 * 1024 * 1024 {
            file.flush()?;
            since_flush = 0;
        }
        progress(DownloadPhase::Transferring, done, spec.bytes);
    }
    file.flush()?;
    file.sync_all()?;
    if done < spec.bytes {
        return Ok(Attempt::Retry(format!(
            "connection closed at {done}/{} bytes",
            spec.bytes
        )));
    }
    Ok(Attempt::Complete)
}

/// Streaming SHA-256 (lowercase hex).
pub fn hash_file(path: &Path) -> AppResult<String> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 4 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Refuse to start when the drive lacks space (spec §14 "Insufficient disk").
pub fn check_disk_space(dir: &Path, required: u64) -> AppResult<()> {
    let Some(free) = crate::hardware::free_space(dir) else {
        return Ok(());
    };
    check_space_numbers(free, required)
}

pub fn check_space_numbers(free: u64, required: u64) -> AppResult<()> {
    if free >= required {
        return Ok(());
    }
    Err(AppError::ai(
        "disk_space",
        format!(
            "Offline AI needs about {} of free space on this drive, but only {} is free. Free up {} and try again. Nothing was downloaded.",
            crate::selection::human_size(required),
            crate::selection::human_size(free),
            crate::selection::human_size(required - free)
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_preflight_refuses_when_space_is_short() {
        assert!(check_space_numbers(10, 5).is_ok());
        let e = check_space_numbers(1 << 30, 3 << 30).unwrap_err();
        assert_eq!(e.code_str(), "ai.disk_space");
        assert!(e.message.contains("Free up"));
    }
}

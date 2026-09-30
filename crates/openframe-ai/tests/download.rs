//! Resumable, verified downloads against a local test HTTP server
//! (Local AI Runtime spec §7, §14). No real models are downloaded.

mod common;

use std::sync::atomic::Ordering;
use std::time::Duration;

use common::{TestServer, payload, sha256_hex};
use openframe_ai::download::{
    Control, DownloadOptions, DownloadPhase, DownloadSpec, download_verified, partial_len,
};
use openframe_ai::rt::AiRuntime;

fn opts() -> DownloadOptions {
    DownloadOptions {
        max_attempts: 4,
        base_backoff: Duration::from_millis(10),
        max_backoff: Duration::from_millis(50),
        stall_timeout: Duration::from_secs(10),
    }
}

fn spec(dir: &std::path::Path, url: String, bytes: &[u8], sha: Option<String>) -> DownloadSpec {
    DownloadSpec {
        url,
        bytes: bytes.len() as u64,
        sha256: sha.unwrap_or_else(|| sha256_hex(bytes)),
        partial: dir.join(".downloads").join("m.gguf.partial"),
        dest: dir.join("profile").join("model.gguf"),
        quarantine_dir: dir.join(".quarantine"),
        label: "test-profile".into(),
    }
}

fn run(
    rt: &AiRuntime,
    s: &DownloadSpec,
    o: DownloadOptions,
    c: Control,
) -> Result<std::path::PathBuf, openframe_domain::AppError> {
    let s = s.clone();
    rt.run(async move {
        let http = reqwest::Client::new();
        download_verified(&http, &s, &o, &c, &|_, _, _| {}).await
    })
}

#[test]
fn download_is_verified_and_moved_into_place() {
    let server = TestServer::start();
    let data = payload(300_000, 3);
    server.put("/m.gguf", data.clone());
    let dir = tempfile::tempdir().unwrap();
    let rt = AiRuntime::new().unwrap();
    let s = spec(dir.path(), server.url("/m.gguf"), &data, None);
    let dest = run(&rt, &s, opts(), Control::new()).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), data);
    assert!(
        !s.partial.exists(),
        "partial is renamed atomically into place"
    );
}

#[test]
fn interrupted_download_resumes_from_partial_with_a_range_request() {
    let server = TestServer::start();
    let data = payload(500_000, 7);
    server.put("/m.gguf", data.clone());
    *server.state.fail_after.lock() = Some(200_000);
    let dir = tempfile::tempdir().unwrap();
    let rt = AiRuntime::new().unwrap();
    let s = spec(dir.path(), server.url("/m.gguf"), &data, None);
    let dest = run(&rt, &s, opts(), Control::new()).unwrap();
    assert_eq!(
        std::fs::read(&dest).unwrap(),
        data,
        "resumed file is byte-identical"
    );
    let ranges = server.state.ranges.lock().clone();
    assert_eq!(ranges.len(), 1, "exactly one resume request: {ranges:?}");
    let from: u64 = ranges[0]
        .trim_start_matches("bytes=")
        .trim_end_matches('-')
        .parse()
        .unwrap();
    assert!(
        from > 0 && from <= 200_000,
        "resumed from the partial offset, got {from}"
    );
}

#[test]
fn paused_download_keeps_partial_and_resumes_later() {
    let server = TestServer::start();
    let data = payload(2_000_000, 11);
    server.put("/m.gguf", data.clone());
    let dir = tempfile::tempdir().unwrap();
    let rt = AiRuntime::new().unwrap();
    let s = spec(dir.path(), server.url("/m.gguf"), &data, None);
    // Pause as soon as some bytes have arrived.
    let control = Control::new();
    let c2 = control.clone();
    let s2 = s.clone();
    let err = rt
        .run(async move {
            let http = reqwest::Client::new();
            download_verified(&http, &s2, &opts(), &c2, &|phase, done, _| {
                if phase == DownloadPhase::Transferring && done > 0 {
                    c2.stop();
                }
            })
            .await
        })
        .unwrap_err();
    let _ = control;
    assert_eq!(err.code_str(), "internal.cancelled");
    let have = partial_len(&s);
    assert!(
        have > 0 && have < data.len() as u64,
        "partial kept for resume ({have} bytes)"
    );
    // Resume: only the remainder is requested.
    let dest = run(&rt, &s, opts(), Control::new()).unwrap();
    assert_eq!(std::fs::read(dest).unwrap(), data);
    assert!(
        server
            .state
            .ranges
            .lock()
            .iter()
            .any(|r| r == &format!("bytes={have}-"))
    );
}

#[test]
fn server_without_range_support_restarts_cleanly() {
    let server = TestServer::start();
    let data = payload(120_000, 5);
    server.put("/m.gguf", data.clone());
    server.state.no_range_support.store(true, Ordering::SeqCst);
    *server.state.fail_after.lock() = Some(50_000);
    let dir = tempfile::tempdir().unwrap();
    let rt = AiRuntime::new().unwrap();
    let s = spec(dir.path(), server.url("/m.gguf"), &data, None);
    let dest = run(&rt, &s, opts(), Control::new()).unwrap();
    assert_eq!(
        std::fs::read(dest).unwrap(),
        data,
        "a 200 answer to a range request restarts from zero, never splices"
    );
}

#[test]
fn hash_mismatch_is_quarantined_and_never_used() {
    let server = TestServer::start();
    let data = payload(100_000, 9);
    server.put("/m.gguf", data.clone());
    let dir = tempfile::tempdir().unwrap();
    let rt = AiRuntime::new().unwrap();
    let wrong = sha256_hex(b"something else");
    let s = spec(dir.path(), server.url("/m.gguf"), &data, Some(wrong));
    let err = run(&rt, &s, opts(), Control::new()).unwrap_err();
    assert_eq!(err.code_str(), "ai.integrity");
    assert!(
        !s.dest.exists(),
        "unverified file never reaches its destination"
    );
    assert!(!s.partial.exists(), "rejected partial is not resumed");
    let quarantined: Vec<_> = std::fs::read_dir(&s.quarantine_dir)
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(quarantined.len(), 1, "rejected download is quarantined");
}

#[test]
fn stale_partial_for_another_file_is_discarded() {
    let server = TestServer::start();
    let data = payload(90_000, 13);
    server.put("/m.gguf", data.clone());
    let dir = tempfile::tempdir().unwrap();
    let rt = AiRuntime::new().unwrap();
    let s = spec(dir.path(), server.url("/m.gguf"), &data, None);
    std::fs::create_dir_all(s.partial.parent().unwrap()).unwrap();
    std::fs::write(&s.partial, vec![0xAB; 40_000]).unwrap();
    // No bookkeeping file → the partial's origin is unknown → it must not be spliced.
    assert_eq!(partial_len(&s), 0);
    let dest = run(&rt, &s, opts(), Control::new()).unwrap();
    assert_eq!(std::fs::read(dest).unwrap(), data);
    assert!(
        server.state.ranges.lock().is_empty(),
        "no range request for an unknown partial"
    );
}

#[test]
fn missing_file_is_a_clear_failure_without_retry_storm() {
    let server = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    let rt = AiRuntime::new().unwrap();
    let s = spec(dir.path(), server.url("/missing.gguf"), b"x", None);
    let err = run(&rt, &s, opts(), Control::new()).unwrap_err();
    assert_eq!(err.code_str(), "ai.download_unavailable");
    assert_eq!(
        server.state.requests.load(Ordering::SeqCst),
        1,
        "4xx is permanent: no retries"
    );
}

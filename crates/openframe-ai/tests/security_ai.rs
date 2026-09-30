//! Regression tests for the 2026-09-30 security review (AI-03 model removal path,
//! REL-01 manifest trust when the embedded manifest isn't signed by the trusted key).

mod common;

use std::time::Duration;

use common::{TestKey, sha256_hex};
use openframe_ai::download::DownloadOptions;
use openframe_ai::manager::{AiManager, ManagerConfig};
use openframe_ai::store::AiPaths;
use openframe_ai::supervisor::SupervisorConfig;
use serde_json::json;

fn manifest(sequence: u64) -> Vec<u8> {
    let sha = sha256_hex(b"m");
    let rt = |backend: &str| {
        json!({
            "runtimeId": format!("test-runtime-{backend}"), "engine": "llama.cpp", "version": "t1", "backend": backend,
            "os": std::env::consts::OS, "arch": std::env::consts::ARCH, "url": "https://example.invalid/rt.zip",
            "bytes": 10, "sha256": sha, "archive": "zip", "executable": "llama-server.exe", "licenseId": "MIT"
        })
    };
    serde_json::to_vec_pretty(&json!({
        "manifestVersion": 1, "sequence": sequence, "channel": "test", "issuedAt": "2026-09-30",
        "runtimes": [rt("cpu"), rt("vulkan")],
        "models": [{
            "profileId": "test-light", "tier": "lightweight", "displayName": "Test", "engine": "llama.cpp",
            "architecture": "test", "bytes": 1, "sha256": sha, "url": "https://example.invalid/m.gguf",
            "minRamBytes": 1, "recommendedRamBytes": 1, "gpuVramBytes": 1, "contextTokens": 2048, "licenseId": "Apache-2.0"
        }]
    }))
    .unwrap()
}

fn config(
    dir: &std::path::Path,
    trusted: &TestKey,
    embedded: Vec<u8>,
    sig: String,
) -> ManagerConfig {
    ManagerConfig {
        app_data_dir: dir.to_path_buf(),
        trusted_public_key: trusted.public_b64(),
        embedded_manifest: (embedded, sig),
        distribution_base_url: None,
        download: DownloadOptions {
            max_attempts: 1,
            base_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(1),
            stall_timeout: Duration::from_secs(1),
        },
        supervisor: SupervisorConfig::default(),
        health_timeout: Duration::from_secs(1),
    }
}

#[test]
fn remove_model_cannot_escape_the_models_folder() {
    let dir = tempfile::tempdir().unwrap();
    let key = TestKey::new();
    let m = manifest(1);
    let sig = key.sign_b64(&m);
    let mgr = AiManager::new(config(dir.path(), &key, m, sig)).unwrap();
    // A sibling folder that a crafted id could otherwise reach.
    let victim = dir.path().join("victim");
    std::fs::create_dir_all(&victim).unwrap();
    std::fs::write(victim.join("keep.txt"), b"x").unwrap();
    let cwd = std::env::current_dir().unwrap();
    for bad in [
        "C:",
        "c:",
        "D:",
        "..",
        ".",
        "",
        "con",
        "nul",
        "a/b",
        "a\\b",
        "..\\victim",
        "x:y",
        "com1",
    ] {
        let e = mgr.remove_model(bad).unwrap_err();
        assert_eq!(e.code_str(), "validation.invalid_input", "{bad}");
    }
    assert!(victim.join("keep.txt").is_file());
    assert!(cwd.is_dir(), "the process working directory is untouched");
    // A well-formed id that isn't installed is simply not found.
    assert_eq!(
        mgr.remove_model("not-installed").unwrap_err().code_str(),
        "not_found.ai_model"
    );
}

#[test]
fn untrusted_embedded_manifest_is_never_used_but_a_verified_cache_is() {
    // A release build trusts its production key while still embedding the development-signed
    // manifest: the embedded manifest must be ignored and a verified cached one used.
    let dir = tempfile::tempdir().unwrap();
    let release = TestKey::new();
    let dev = TestKey::new();
    let embedded = manifest(1);
    let dev_sig = dev.sign_b64(&embedded);
    // No cache yet: nothing usable, and the signature error is reported.
    let mgr = AiManager::new(config(
        dir.path(),
        &release,
        embedded.clone(),
        dev_sig.clone(),
    ))
    .unwrap();
    assert_eq!(
        mgr.manifest().unwrap_err().code_str(),
        "security.signature_invalid"
    );
    // A cached manifest signed with the trusted key is used (even with a lower sequence).
    let paths = AiPaths::new(dir.path());
    std::fs::create_dir_all(paths.models_dir()).unwrap();
    let cached = manifest(0);
    std::fs::write(paths.manifest_cache(), &cached).unwrap();
    std::fs::write(paths.manifest_cache_sig(), release.sign_b64(&cached)).unwrap();
    let mgr = AiManager::new(config(dir.path(), &release, embedded, dev_sig)).unwrap();
    assert_eq!(mgr.manifest().unwrap().sequence, 0);
}

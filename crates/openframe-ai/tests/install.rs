//! Manifest trust and installation safety (Local AI Runtime spec §6–§8, §14–§15).
//! Uses a throwaway signing key and a local test server; no real models.

mod common;

use std::time::Duration;

use common::{TestKey, TestServer, payload, sha256_hex};
use openframe_ai::download::{Control, DownloadOptions};
use openframe_ai::manager::{AiManager, ManagerConfig};
use openframe_ai::store::{ActiveSelection, AiPaths};
use openframe_ai::supervisor::SupervisorConfig;
use serde_json::json;

struct Fixture {
    server: TestServer,
    key: TestKey,
    dir: tempfile::TempDir,
}

fn runtime_zip() -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("rt.zip");
    openframe_security::archive::write_zip(
        &p,
        vec![(
            "llama-server.exe".to_string(),
            openframe_security::archive::EntrySource::Bytes(b"MZ-not-a-real-server"),
        )],
    )
    .unwrap();
    std::fs::read(p).unwrap()
}

impl Fixture {
    fn new() -> Self {
        Self {
            server: TestServer::start(),
            key: TestKey::new(),
            dir: tempfile::tempdir().unwrap(),
        }
    }

    /// Manifest with runtimes for both backends (this machine picks one) and two profiles.
    fn manifest(&self, sequence: u64, model_sha: &str, model_len: usize) -> Vec<u8> {
        let zip = runtime_zip();
        self.server.put("/rt.zip", zip.clone());
        let rt = |backend: &str| {
            json!({
                "runtimeId": format!("test-runtime-{backend}"), "engine": "llama.cpp", "version": "t1", "backend": backend,
                "os": std::env::consts::OS, "arch": std::env::consts::ARCH, "url": self.server.url("/rt.zip"),
                "bytes": zip.len(), "sha256": sha256_hex(&zip), "archive": "zip", "executable": "llama-server.exe", "licenseId": "MIT"
            })
        };
        let model = |id: &str, tier: &str| {
            json!({
                "profileId": id, "tier": tier, "displayName": format!("Test {tier}"), "engine": "llama.cpp", "architecture": "test",
                "bytes": model_len, "sha256": model_sha, "url": self.server.url(&format!("/{id}.gguf")),
                "minRamBytes": 1, "recommendedRamBytes": 1, "gpuVramBytes": 1, "contextTokens": 2048, "licenseId": "Apache-2.0"
            })
        };
        serde_json::to_vec_pretty(&json!({
            "manifestVersion": 1, "sequence": sequence, "channel": "test", "issuedAt": "2026-09-30",
            "runtimes": [rt("cpu"), rt("vulkan")],
            "models": [model("test-light", "lightweight"), model("test-rec", "recommended")]
        }))
        .unwrap()
    }

    fn config(&self, manifest: Vec<u8>) -> ManagerConfig {
        let sig = self.key.sign_b64(&manifest);
        ManagerConfig {
            app_data_dir: self.dir.path().to_path_buf(),
            trusted_public_key: self.key.public_b64(),
            embedded_manifest: (manifest, sig),
            distribution_base_url: None,
            download: DownloadOptions {
                max_attempts: 2,
                base_backoff: Duration::from_millis(10),
                max_backoff: Duration::from_millis(20),
                stall_timeout: Duration::from_secs(10),
            },
            supervisor: SupervisorConfig {
                start_timeout: Duration::from_secs(5),
                ..SupervisorConfig::default()
            },
            health_timeout: Duration::from_secs(5),
        }
    }
}

#[test]
fn tampered_manifest_is_rejected() {
    let f = Fixture::new();
    let good = f.manifest(1, &sha256_hex(b"m"), 1);
    let mut cfg = f.config(good.clone());
    // Swap a byte after signing: the download address/hash can't be altered undetected.
    let mut bad = good.clone();
    let i = bad.iter().position(|b| *b == b'1').unwrap();
    bad[i] = b'2';
    cfg.embedded_manifest.0 = bad;
    let m = AiManager::new(cfg).unwrap();
    assert_eq!(
        m.manifest().unwrap_err().code_str(),
        "security.signature_invalid"
    );
}

#[test]
fn manifest_signed_by_another_key_is_rejected() {
    let f = Fixture::new();
    let good = f.manifest(1, &sha256_hex(b"m"), 1);
    let mut cfg = f.config(good.clone());
    cfg.embedded_manifest.1 = TestKey::new().sign_b64(&good);
    let m = AiManager::new(cfg).unwrap();
    assert_eq!(
        m.manifest().unwrap_err().code_str(),
        "security.signature_invalid"
    );
}

#[test]
fn tampered_manifest_cache_is_ignored_and_removed() {
    let f = Fixture::new();
    let embedded = f.manifest(1, &sha256_hex(b"m"), 1);
    let cfg = f.config(embedded);
    let paths = AiPaths::new(f.dir.path());
    std::fs::create_dir_all(paths.models_dir()).unwrap();
    // A "newer" cached manifest whose bytes no longer match their signature.
    let newer = f.manifest(9, &sha256_hex(b"m"), 1);
    let sig = f.key.sign_b64(&newer);
    let mut tampered = newer.clone();
    let i = tampered.iter().position(|b| *b == b'9').unwrap();
    tampered[i] = b'8';
    std::fs::write(paths.manifest_cache(), &tampered).unwrap();
    std::fs::write(paths.manifest_cache_sig(), sig).unwrap();
    let m = AiManager::new(cfg).unwrap();
    assert_eq!(
        m.manifest().unwrap().sequence,
        1,
        "falls back to the embedded, verified manifest"
    );
    assert!(
        !paths.manifest_cache().exists(),
        "tampered cache is discarded"
    );
}

#[test]
fn remote_manifest_cannot_roll_back_to_an_older_sequence() {
    let f = Fixture::new();
    let embedded = f.manifest(5, &sha256_hex(b"m"), 1);
    let older = f.manifest(2, &sha256_hex(b"m"), 1);
    f.server.put("/dist/manifest.json", older.clone());
    f.server.put(
        "/dist/manifest.json.sig",
        f.key.sign_b64(&older).into_bytes(),
    );
    let mut cfg = f.config(embedded);
    cfg.distribution_base_url = Some(f.server.url("/dist"));
    let m = AiManager::new(cfg).unwrap();
    assert_eq!(m.refresh_manifest().unwrap().sequence, 5);
    let newer = f.manifest(7, &sha256_hex(b"m"), 1);
    f.server.put("/dist/manifest.json", newer.clone());
    f.server.put(
        "/dist/manifest.json.sig",
        f.key.sign_b64(&newer).into_bytes(),
    );
    assert_eq!(
        m.refresh_manifest().unwrap().sequence,
        7,
        "a newer signed manifest is accepted and cached"
    );
    assert!(AiPaths::new(f.dir.path()).manifest_cache().exists());
}

#[test]
fn hash_mismatch_is_quarantined_and_previous_model_stays_active() {
    let f = Fixture::new();
    let real = payload(64_000, 21);
    let served = payload(64_000, 22); // what the (compromised) server sends
    let manifest = f.manifest(1, &sha256_hex(&real), real.len());
    f.server.put("/test-rec.gguf", served);
    let cfg = f.config(manifest);
    let paths = AiPaths::new(f.dir.path());

    // A previously installed, verified model is active.
    let old_dir = paths.model_dir("old-profile");
    std::fs::create_dir_all(&old_dir).unwrap();
    std::fs::write(old_dir.join("model.gguf"), b"previous model").unwrap();
    paths
        .set_active(Some(&ActiveSelection {
            profile_id: "old-profile".into(),
            runtime_id: "old-runtime".into(),
            activated_at: 1,
            previous_profile_id: None,
        }))
        .unwrap();

    let m = AiManager::new(cfg).unwrap();
    let err = m
        .install(Some("test-rec"), &Control::new(), &|_| {})
        .unwrap_err();
    assert_eq!(err.code_str(), "ai.integrity");
    let active = paths.active().unwrap();
    assert_eq!(
        active.profile_id, "old-profile",
        "previous model remains active"
    );
    assert_eq!(
        std::fs::read(old_dir.join("model.gguf")).unwrap(),
        b"previous model",
        "previous model untouched"
    );
    assert!(
        !paths.model_file("test-rec").exists(),
        "rejected model never installed"
    );
    assert_eq!(
        std::fs::read_dir(paths.quarantine_dir()).unwrap().count(),
        1,
        "rejected download quarantined"
    );
}

#[test]
fn verified_model_that_cannot_start_is_not_activated() {
    let f = Fixture::new();
    let real = payload(32_000, 31);
    let manifest = f.manifest(1, &sha256_hex(&real), real.len());
    f.server.put("/test-light.gguf", real.clone());
    let cfg = f.config(manifest);
    let paths = AiPaths::new(f.dir.path());
    let m = AiManager::new(cfg).unwrap();
    // The fixture runtime is not a real server, so the health check must fail.
    let err = m
        .install(Some("test-light"), &Control::new(), &|_| {})
        .unwrap_err();
    assert_eq!(err.code_str(), "ai.start_failed");
    assert!(
        paths.active().is_none(),
        "nothing activated after a failed health check"
    );
    assert_eq!(
        std::fs::read(paths.model_file("test-light")).unwrap(),
        real,
        "verified model kept for retry"
    );
    assert!(
        paths
            .model_metadata("test-light")
            .unwrap()
            .health_check_failed
    );
    // Removing it works and leaves AI off.
    m.remove_model("test-light").unwrap();
    assert!(!paths.model_dir("test-light").exists());
}

#[test]
fn insufficient_disk_space_blocks_download_before_it_starts() {
    let e = openframe_ai::download::check_space_numbers(1 << 30, 5 << 30).unwrap_err();
    assert_eq!(e.code_str(), "ai.disk_space");
    assert!(e.message.contains("Nothing was downloaded"));
}

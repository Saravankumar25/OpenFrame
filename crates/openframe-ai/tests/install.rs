//! Manifest trust and installation safety (Local AI Runtime spec §6–§8, §14–§15; agentic AI
//! spec §23–§24). Uses a throwaway signing key and a local test server; no real models.
//! The full successful lifecycle (health check, activation, update, fallback, uninstall) is in
//! `tests/lifecycle.rs`.

mod common;

use common::{ManifestSpec, TestKey, TestServer, payload, runtime_zip, sha256_hex, test_config};
use openframe_ai::Backend;
use openframe_ai::download::Control;
use openframe_ai::manager::{AiManager, ManagerConfig};
use openframe_ai::store::{ACTIVE_SCHEMA, ActiveInstall, AiPaths};

struct Fixture {
    server: TestServer,
    key: TestKey,
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            server: TestServer::start(),
            key: TestKey::new(),
            dir: tempfile::tempdir().unwrap(),
        }
    }

    /// A manifest whose "runtime" is not a real server (health checks fail).
    fn spec(&self) -> ManifestSpec {
        ManifestSpec::new(
            payload(32_000, 11),
            payload(4_000, 12),
            runtime_zip(b"MZ-not-a-real-server", &[]),
        )
    }

    fn manifest(&self, sequence: u64) -> Vec<u8> {
        let mut s = self.spec();
        s.sequence = sequence;
        s.publish(&self.server)
    }

    fn config(&self, manifest: Vec<u8>) -> ManagerConfig {
        test_config(self.dir.path(), &self.key, manifest)
    }

    fn paths(&self) -> AiPaths {
        AiPaths::new(self.dir.path())
    }
}

#[test]
fn tampered_manifest_is_rejected() {
    let f = Fixture::new();
    let good = f.manifest(1);
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
    // Nothing can be planned or installed from an unverified manifest.
    assert!(m.plan().is_err());
    assert!(m.install(&Control::new(), &|_| {}).is_err());
}

#[test]
fn manifest_signed_by_another_key_is_rejected() {
    let f = Fixture::new();
    let good = f.manifest(1);
    let mut cfg = f.config(good.clone());
    cfg.embedded_manifest.1 = TestKey::new().sign_b64(&good);
    let m = AiManager::new(cfg).unwrap();
    assert_eq!(
        m.manifest().unwrap_err().code_str(),
        "security.signature_invalid"
    );
}

#[test]
fn retired_three_tier_manifest_format_is_refused_even_when_signed() {
    let f = Fixture::new();
    let old = serde_json::to_vec(&serde_json::json!({
        "manifestVersion": 1, "sequence": 9, "channel": "dev", "issuedAt": "2026-09-30",
        "runtimes": [], "models": []
    }))
    .unwrap();
    let m = AiManager::new(f.config(old)).unwrap();
    assert_eq!(
        m.manifest().unwrap_err().code_str(),
        "security.manifest_invalid"
    );
}

#[test]
fn tampered_manifest_cache_is_ignored_and_removed() {
    let f = Fixture::new();
    let cfg = f.config(f.manifest(1));
    let paths = f.paths();
    std::fs::create_dir_all(paths.models_dir()).unwrap();
    // A "newer" cached manifest whose bytes no longer match their signature.
    let newer = f.manifest(9);
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
    let older = f.manifest(2);
    f.server.put("/dist/manifest.json", older.clone());
    f.server.put(
        "/dist/manifest.json.sig",
        f.key.sign_b64(&older).into_bytes(),
    );
    let mut cfg = f.config(f.manifest(5));
    cfg.distribution_base_url = Some(f.server.url("/dist"));
    let m = AiManager::new(cfg).unwrap();
    assert_eq!(m.refresh_manifest().unwrap().sequence, 5);
    let newer = f.manifest(7);
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
    assert!(f.paths().manifest_cache().exists());
}

#[test]
fn exact_download_size_and_disk_need_are_known_before_downloading() {
    let f = Fixture::new();
    let spec = f.spec();
    let m = AiManager::new(f.config(spec.publish(&f.server))).unwrap();
    let plan = m.plan().unwrap();
    let expected = spec.runtimes[0].2.len() + spec.chat.len() + spec.embedding.len();
    assert_eq!(plan.download_bytes, expected as u64);
    assert_eq!(
        plan.components.len(),
        3,
        "runtime + chat model + embedding model"
    );
    assert!(plan.required_free_bytes > plan.download_bytes);
    assert_eq!(
        f.server
            .state
            .requests
            .load(std::sync::atomic::Ordering::SeqCst),
        0,
        "planning never downloads"
    );
}

#[test]
fn hash_mismatch_is_quarantined_and_previous_install_stays_active() {
    let f = Fixture::new();
    let spec = f.spec();
    let manifest = spec.publish(&f.server);
    // The (compromised) server now serves different bytes for the chat model.
    f.server.put("/test-chat-v1.gguf", payload(32_000, 99));
    let cfg = f.config(manifest);
    let paths = f.paths();

    // A previously installed, verified install is active.
    let old_dir = paths.model_dir("old-chat");
    std::fs::create_dir_all(&old_dir).unwrap();
    std::fs::write(old_dir.join("model.gguf"), b"previous model").unwrap();
    let previous = ActiveInstall {
        schema: ACTIVE_SCHEMA,
        profile_id: "openframe-local-ai-v1".into(),
        profile_version: "0".into(),
        runtime_id: "old-runtime".into(),
        backend: Backend::Cpu,
        chat_model_id: "old-chat".into(),
        embedding_model_id: "old-embedding".into(),
        activated_at: 1,
    };
    paths.set_active(Some(&previous)).unwrap();

    let m = AiManager::new(cfg).unwrap();
    let err = m.install(&Control::new(), &|_| {}).unwrap_err();
    assert_eq!(err.code_str(), "ai.integrity");
    assert_eq!(
        paths.active().unwrap(),
        previous,
        "previous install remains active"
    );
    assert_eq!(
        std::fs::read(old_dir.join("model.gguf")).unwrap(),
        b"previous model",
        "previous model untouched"
    );
    assert!(
        !paths.model_file("test-chat-v1").exists(),
        "rejected model never installed"
    );
    assert_eq!(
        std::fs::read_dir(paths.quarantine_dir()).unwrap().count(),
        1,
        "rejected download quarantined"
    );
}

#[test]
fn verified_components_that_cannot_start_are_not_activated() {
    let f = Fixture::new();
    let spec = f.spec();
    let cfg = f.config(spec.publish(&f.server));
    let paths = f.paths();
    let m = AiManager::new(cfg).unwrap();
    // The fixture runtime is not a real server, so the health check must fail.
    let err = m.install(&Control::new(), &|_| {}).unwrap_err();
    assert_eq!(err.code_str(), "ai.start_failed");
    assert!(!err.message.to_lowercase().contains("gguf"));
    assert!(
        paths.active().is_none(),
        "nothing activated after a failed health check"
    );
    assert!(!m.state().installed);
    assert_eq!(
        std::fs::read(paths.model_file("test-chat-v1")).unwrap(),
        spec.chat,
        "verified model kept for retry"
    );
    assert!(
        paths
            .model_metadata("test-chat-v1")
            .unwrap()
            .health_check_failed
    );
    // A retry downloads nothing again.
    assert_eq!(m.plan().unwrap().download_bytes, 0);
    // Uninstall removes it all and leaves AI off.
    m.uninstall().unwrap();
    assert!(!paths.model_dir("test-chat-v1").exists());
    assert!(m.plan().unwrap().download_bytes > 0);
}

#[test]
fn a_pointer_from_the_retired_profile_system_reads_as_not_installed() {
    let f = Fixture::new();
    let paths = f.paths();
    std::fs::create_dir_all(paths.models_dir()).unwrap();
    std::fs::write(
        paths.active_file(),
        br#"{"profileId":"qwen3-recommended-win-x64-v1","runtimeId":"llama-cpp-b1","activatedAt":1,"previousProfileId":null}"#,
    )
    .unwrap();
    let m = AiManager::new(f.config(f.manifest(1))).unwrap();
    assert!(m.active().is_none());
    assert!(!m.state().installed);
    assert!(m.chat_model().is_none());
}

#[test]
fn insufficient_disk_space_blocks_download_before_it_starts() {
    let e = openframe_ai::download::check_space_numbers(1 << 30, 5 << 30).unwrap_err();
    assert_eq!(e.code_str(), "ai.disk_space");
    assert!(e.message.contains("Nothing was downloaded"));
    let _ = sha256_hex(b"");
}

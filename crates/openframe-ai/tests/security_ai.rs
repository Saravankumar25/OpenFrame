//! Regression tests for the 2026-09-30 security review (REL-01 manifest trust when the embedded
//! manifest isn't signed by the trusted key) and the uninstall/clean-up boundary (AI-03: only the
//! AI component folders may ever be deleted).

mod common;

use common::{ManifestSpec, TestKey, TestServer, payload, runtime_zip, test_config};
use openframe_ai::Backend;
use openframe_ai::manager::AiManager;
use openframe_ai::store::{ACTIVE_SCHEMA, ActiveInstall, AiPaths};

fn manifest(server: &TestServer, sequence: u64) -> Vec<u8> {
    let mut s = ManifestSpec::new(payload(10, 1), payload(10, 2), runtime_zip(b"x", &[]));
    s.sequence = sequence;
    s.publish(server)
}

#[test]
fn uninstall_and_cleanup_never_leave_the_ai_folders() {
    let dir = tempfile::tempdir().unwrap();
    let server = TestServer::start();
    let key = TestKey::new();
    let mgr = AiManager::new(test_config(dir.path(), &key, manifest(&server, 1))).unwrap();
    let paths = AiPaths::new(dir.path());
    // Files next to (not inside) the AI folders.
    let victim = dir.path().join("victim");
    std::fs::create_dir_all(&victim).unwrap();
    std::fs::write(victim.join("keep.txt"), b"x").unwrap();
    std::fs::write(dir.path().join("app.sqlite"), b"x").unwrap();
    std::fs::create_dir_all(paths.model_dir("some-model")).unwrap();
    std::fs::create_dir_all(paths.runtime_dir("some-runtime")).unwrap();
    // Clean-up after an activation only removes unreferenced component folders.
    let active = ActiveInstall {
        schema: ACTIVE_SCHEMA,
        profile_id: "openframe-local-ai-v1".into(),
        profile_version: "1".into(),
        runtime_id: "some-runtime".into(),
        backend: Backend::Cpu,
        chat_model_id: "some-model".into(),
        embedding_model_id: "other".into(),
        activated_at: 1,
    };
    std::fs::create_dir_all(paths.model_dir("stale-model")).unwrap();
    paths.remove_unreferenced(&active);
    assert!(paths.model_dir("some-model").is_dir());
    assert!(!paths.model_dir("stale-model").exists());
    // Uninstall removes the AI folders' contents and nothing else.
    mgr.uninstall().unwrap();
    assert!(!paths.model_dir("some-model").exists());
    assert!(!paths.runtime_dir("some-runtime").exists());
    assert!(victim.join("keep.txt").is_file());
    assert!(dir.path().join("app.sqlite").is_file());
    assert!(std::env::current_dir().unwrap().is_dir());
}

#[test]
fn untrusted_embedded_manifest_is_never_used_but_a_verified_cache_is() {
    // A release build trusts its production key while still embedding the development-signed
    // manifest: the embedded manifest must be ignored and a verified cached one used.
    let dir = tempfile::tempdir().unwrap();
    let server = TestServer::start();
    let release = TestKey::new();
    let dev = TestKey::new();
    let embedded = manifest(&server, 1);
    let mut cfg = test_config(dir.path(), &release, embedded.clone());
    cfg.embedded_manifest.1 = dev.sign_b64(&embedded);
    // No cache yet: nothing usable, and the signature error is reported.
    let mgr = AiManager::new(cfg.clone()).unwrap();
    assert_eq!(
        mgr.manifest().unwrap_err().code_str(),
        "security.signature_invalid"
    );
    // A cached manifest signed with the trusted key is used (even with a lower sequence).
    let paths = AiPaths::new(dir.path());
    std::fs::create_dir_all(paths.models_dir()).unwrap();
    let cached = manifest(&server, 0);
    std::fs::write(paths.manifest_cache(), &cached).unwrap();
    std::fs::write(paths.manifest_cache_sig(), release.sign_b64(&cached)).unwrap();
    let mgr = AiManager::new(cfg).unwrap();
    assert_eq!(mgr.manifest().unwrap().sequence, 0);
}

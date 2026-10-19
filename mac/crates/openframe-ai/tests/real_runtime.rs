//! Validation with the REAL engine and models (ignored by default; needs the engine staged by
//! `node scripts/prepare-ai-assets.mjs` and the ~1.1 GB of pinned models in `.ai-assets` or
//! OPENFRAME_AI_ASSETS_DIR, installed in the test's app data as a finished download would; ADR-0026):
//!
//! ```text
//! OPENFRAME_AI_BUNDLE_DIR=<repo>/.build-cache/ai-bundle cargo test -p openframe-ai --test real_runtime -- --ignored --nocapture
//! ```
//!
//! It runs the product path end to end with the lock compiled into this build: presence check,
//! the full hash check against the pinned hashes, chat, grammar-constrained JSON, embeddings and
//! the shortcut on the next start (the check record in app data). Nothing is downloaded here.

use std::time::{Duration, Instant};

use openframe_ai::{AiManager, BundleState, ChatMessage, ChatRequest, ManagerConfig};

fn config(bundle: &std::path::Path, data: &std::path::Path) -> ManagerConfig {
    let assets = std::env::var_os("OPENFRAME_AI_ASSETS_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.ai-assets")
        });
    let host = openframe_ai::test_support::install_pinned_models(
        data,
        &assets,
        &openframe_ai::AssetLock::embedded().unwrap(),
    )
    .unwrap();
    let mut cfg = ManagerConfig::production(data, Some(bundle.to_path_buf()), Some(host)).unwrap();
    cfg.deterministic = true;
    cfg.ready_timeout = Duration::from_secs(300);
    cfg.supervisor.start_timeout = Duration::from_secs(300);
    cfg.embedding_supervisor.start_timeout = Duration::from_secs(300);
    cfg
}

#[test]
#[ignore]
fn real_bundled_runtime_checks_then_chats_and_embeds() {
    let Ok(src) = std::env::var("OPENFRAME_AI_BUNDLE_DIR") else {
        eprintln!("OPENFRAME_AI_BUNDLE_DIR not set; skipping");
        return;
    };
    let bundle = std::path::PathBuf::from(src);
    let data = tempfile::tempdir().unwrap();
    let m = std::sync::Arc::new(AiManager::new(config(&bundle, data.path())).unwrap());
    assert_eq!(
        m.state().bundle,
        BundleState::NotChecked,
        "{:?}",
        m.state().problem
    );

    let t = Instant::now();
    m.ensure_verified()
        .expect("the staged bundle matches the compiled lock");
    println!("full check: {:.1}s", t.elapsed().as_secs_f32());
    assert_eq!(m.state().bundle, BundleState::Verified);
    assert!(m.paths().bundle_check().is_file());

    let model = m
        .chat_model()
        .expect("chat model available on first launch");
    let t = Instant::now();
    let answer = model
        .chat(
            &ChatRequest::new(vec![ChatMessage::user(
                "In one sentence: what does a script supervisor do on a film set?",
            )])
            .max_tokens(60),
        )
        .unwrap();
    println!("chat: {:.1}s -> {answer}", t.elapsed().as_secs_f32());
    assert!(!answer.trim().is_empty());
    // Deterministic settings: the same question gives the same answer.
    let again = model
        .chat(
            &ChatRequest::new(vec![ChatMessage::user(
                "In one sentence: what does a script supervisor do on a film set?",
            )])
            .max_tokens(60),
        )
        .unwrap();
    assert_eq!(
        answer, again,
        "temperature 0 and a fixed seed are deterministic"
    );

    let schema = serde_json::json!({"type": "object", "properties": {
        "tool": {"enum": ["count_scenes", "open_scene"]},
        "arguments": {"type": "object", "properties": {}, "additionalProperties": false}
    }, "required": ["tool", "arguments"], "additionalProperties": false});
    let json = model
        .chat(
            &ChatRequest::new(vec![ChatMessage::user(
                "Choose the tool for: How many scenes are in the draft?",
            )])
            .with_schema(schema)
            .max_tokens(60),
        )
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).expect("schema-shaped JSON");
    assert!(v["tool"].is_string());

    let info = m.embedding_info().unwrap();
    assert_eq!(info.dim, 384);
    let q = m
        .embed(&[
            "Ravi argues with Anjali at the railway station".into(),
            "Anjali confronts Ravi on the train platform at night".into(),
            "The catering budget for day three".into(),
        ])
        .unwrap();
    let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    println!(
        "cosine near {:.3} far {:.3}",
        dot(&q[0], &q[1]),
        dot(&q[0], &q[2])
    );
    assert!(dot(&q[0], &q[1]) > dot(&q[0], &q[2]));
    let long = m.embed(&["word ".repeat(3_000)]).unwrap();
    assert_eq!(
        long[0].len(),
        384,
        "a long input is shortened, not rejected"
    );
    m.stop_runtime();
    drop(model);
    drop(m);

    // Next start of the same installed version: the engine files come from the record; the models
    // in writable app data are read again in full (ADR-0026 implementation note 6).
    let m2 = AiManager::new(config(&bundle, data.path())).unwrap();
    let t = Instant::now();
    m2.ensure_verified().unwrap();
    let quick = t.elapsed();
    println!("check on the next start: {:.2}s", quick.as_secs_f32());
    assert!(
        quick < Duration::from_secs(15),
        "the next start check stays short"
    );
}

/// The graphics card path on THIS computer with the real engines (ADR-0029 decision 9): detection,
/// the device match, the probe on the processor and on the card, and one longer answer in each
/// place. Prints the figures and, with OPENFRAME_EVIDENCE_OUT, writes them as JSON evidence.
#[test]
#[ignore]
fn real_graphics_card_probe_and_benchmark() {
    let Ok(src) = std::env::var("OPENFRAME_AI_BUNDLE_DIR") else {
        eprintln!("OPENFRAME_AI_BUNDLE_DIR not set; skipping");
        return;
    };
    let bundle = std::path::PathBuf::from(src);
    let data = tempfile::tempdir().unwrap();
    let m = std::sync::Arc::new(AiManager::new(config(&bundle, data.path())).unwrap());
    let facts = openframe_ai::placement::detect();
    println!(
        "adapters: {:?}; Vulkan loader: {}",
        facts.adapters, facts.vulkan_loader
    );
    let request = || {
        ChatRequest::new(vec![ChatMessage::user(
            "Write a short paragraph describing a rainy bus stop at night for a screenplay.",
        )])
        .max_tokens(160)
    };
    let place = |off: bool| {
        m.set_graphics_off(off).unwrap();
        let model = m.chat_model().expect("AI available");
        let t = Instant::now();
        let first = model.chat(&request()).unwrap();
        let first_s = t.elapsed().as_secs_f32();
        let t = Instant::now();
        let warm = model.chat(&request()).unwrap();
        let warm_s = t.elapsed().as_secs_f32();
        let words = warm.split_whitespace().count();
        let text = m.diagnostics().placement;
        println!(
            "{text}
  first answer {first_s:.1}s ({} words), warm answer {warm_s:.1}s ({words} words)",
            first.split_whitespace().count()
        );
        serde_json::json!({ "placement": text, "firstAnswerSeconds": first_s, "warmAnswerSeconds": warm_s, "warmWords": words })
    };
    let card = place(false);
    let processor = place(true);
    let probe: serde_json::Value = std::fs::read(data.path().join("ai").join("graphics.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    println!("probe record: {probe}");
    if let Ok(out) = std::env::var("OPENFRAME_EVIDENCE_OUT") {
        let adapters: Vec<serde_json::Value> = facts
            .adapters
            .iter()
            .map(|(a, d)| serde_json::json!({ "name": a.name, "vendorId": a.vendor_id, "dedicatedMb": a.dedicated_mb, "sharedMb": a.shared_mb, "software": a.software, "driver": d }))
            .collect();
        let v = serde_json::json!({ "adapters": adapters, "vulkanLoader": facts.vulkan_loader, "automatic": card, "processorOnly": processor, "probe": probe });
        std::fs::write(out, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    }
    m.stop_runtime();
}

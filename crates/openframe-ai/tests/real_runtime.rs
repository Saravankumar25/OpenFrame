//! Validation with the REAL bundled runtime and models (ignored by default; needs the ~1.1 GB of
//! pinned files staged by `node scripts/prepare-ai-assets.mjs`, ADR-0025):
//!
//! ```text
//! OPENFRAME_AI_BUNDLE_DIR=<repo>/.build-cache/ai-bundle cargo test -p openframe-ai --test real_runtime -- --ignored --nocapture
//! ```
//!
//! It runs the product path end to end with the lock compiled into this build: presence check,
//! the full hash check against the pinned hashes, chat, grammar-constrained JSON, embeddings and
//! the shortcut on the next start (the check record in app data). Nothing is downloaded.

use std::time::{Duration, Instant};

use openframe_ai::{AiManager, BundleState, ChatMessage, ChatRequest, ManagerConfig};

fn config(bundle: &std::path::Path, data: &std::path::Path) -> ManagerConfig {
    let mut cfg = ManagerConfig::production(data, Some(bundle.to_path_buf())).unwrap();
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

    // Next start of the same installed version: the record lets the check skip the full read.
    let m2 = AiManager::new(config(&bundle, data.path())).unwrap();
    let t = Instant::now();
    m2.ensure_verified().unwrap();
    let quick = t.elapsed();
    println!("check on the next start: {:.2}s", quick.as_secs_f32());
    assert!(
        quick < Duration::from_secs(2),
        "the recorded check is reused"
    );
}

//! Manual validation with the REAL pinned runtime and models (ignored by default; needs ~1.2 GB
//! of verified files in `.dev-models/`, see `scripts/ai-benchmark.mjs`):
//!
//! ```text
//! OPENFRAME_REAL_AI_DIR=<repo>/.dev-models cargo test -p openframe-ai --test real_runtime -- --ignored --nocapture
//! ```
//!
//! The embedded manifest is used unchanged except that its download URLs point at a loopback
//! mirror of those files (bytes and SHA-256 must still match), re-signed with a throwaway key.
//! It then runs the product path end to end: plan → install (download, verify, extract, health
//! check of both sidecars, activate) → chat → embeddings → uninstall.

mod common;

use std::time::Instant;

use common::{TestKey, TestServer, test_config};
use openframe_ai::download::Control;
use openframe_ai::{AiManager, ChatMessage, ChatRequest};

#[test]
#[ignore]
fn real_runtime_install_chat_embed_uninstall() {
    let Ok(src) = std::env::var("OPENFRAME_REAL_AI_DIR") else {
        eprintln!("OPENFRAME_REAL_AI_DIR not set; skipping");
        return;
    };
    let src = std::path::PathBuf::from(src);
    let server = TestServer::start();
    let mut manifest = openframe_ai::manifest::embedded().unwrap();
    let mirror = |url: &mut String| {
        let name = url.rsplit('/').next().unwrap().to_string();
        let path = src.join(&name);
        if let Ok(bytes) = std::fs::read(&path) {
            server.put(&format!("/{name}"), bytes);
        }
        *url = server.url(&format!("/{name}"));
    };
    for r in &mut manifest.runtimes {
        mirror(&mut r.url);
    }
    for m in &mut manifest.models {
        mirror(&mut m.url);
    }
    let bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    let key = TestKey::new();
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = test_config(dir.path(), &key, bytes);
    cfg.health_timeout = std::time::Duration::from_secs(300);
    cfg.supervisor.start_timeout = std::time::Duration::from_secs(300);
    cfg.embedding_supervisor.start_timeout = std::time::Duration::from_secs(300);
    let m = AiManager::new(cfg).unwrap();

    let plan = m.plan().unwrap();
    println!(
        "plan: download {} bytes, runs on {:?}, needs {} free",
        plan.download_bytes, plan.backend, plan.required_free_bytes
    );
    let t = Instant::now();
    let last = parking_lot::Mutex::new(None);
    let outcome = m
        .install(&Control::new(), &|e| {
            *last.lock() = Some((e.phase, e.done, e.total))
        })
        .expect("real install");
    println!(
        "install: {:.1}s → {:?} (last event {:?})",
        t.elapsed().as_secs_f32(),
        outcome,
        last.lock()
    );
    assert!(m.state().installed);

    let model = m.chat_model().unwrap();
    let t = Instant::now();
    let answer = model
        .chat(
            &ChatRequest::new(vec![ChatMessage::user(
                "In one sentence: what does a script supervisor do on a film set?",
            )])
            .max_tokens(60),
        )
        .unwrap();
    println!("chat: {:.1}s → {answer}", t.elapsed().as_secs_f32());
    assert!(!answer.trim().is_empty());

    let schema = serde_json::json!({"type": "object", "properties": {
        "tool": {"enum": ["count_scenes", "open_scene"]},
        "arguments": {"type": "object", "properties": {}, "additionalProperties": false}
    }, "required": ["tool", "arguments"], "additionalProperties": false});
    let t = Instant::now();
    let json = model
        .chat(
            &ChatRequest::new(vec![ChatMessage::user(
                "Choose the tool for: How many scenes are in the draft?",
            )])
            .with_schema(schema)
            .max_tokens(60),
        )
        .unwrap();
    println!(
        "grammar-constrained: {:.1}s → {json}",
        t.elapsed().as_secs_f32()
    );
    let v: serde_json::Value = serde_json::from_str(&json).expect("schema-shaped JSON");
    assert!(v["tool"].is_string());

    let info = m.embedding_info().unwrap();
    assert_eq!(info.dim, 384);
    let texts: Vec<String> = (0..32)
        .map(|i| {
            format!("Scene {i}. INT. RAILWAY STATION - NIGHT. Ravi waits for Anjali in the rain.")
        })
        .collect();
    let t = Instant::now();
    let v = m.embed(&texts).unwrap();
    println!("embed 32 chunks: {:.2}s", t.elapsed().as_secs_f32());
    assert_eq!(v.len(), 32);
    assert!(v.iter().all(|x| x.len() == 384));
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
    // An input longer than the 512-token window is shortened, not rejected.
    let long = m.embed(&["word ".repeat(3_000)]).unwrap();
    assert_eq!(long[0].len(), 384);

    let freed = m.uninstall().unwrap();
    println!("uninstall freed {freed} bytes");
    assert!(!m.state().installed);
}

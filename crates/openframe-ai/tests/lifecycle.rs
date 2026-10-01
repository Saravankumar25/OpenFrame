//! End-to-end Offline AI lifecycle through the REAL install path (agentic AI spec §24):
//! signed manifest → one combined download (runtime + chat model + embedding model) →
//! verification → extraction → health check of BOTH sidecars → atomic activation → chat and
//! embeddings usable immediately (no restart) → update with clean-up → graphics-card → processor
//! fallback → rollback → uninstall.
//!
//! No real model is involved: the "runtime" zipped into the manifest is THIS test executable.
//! `harness = false` lets `main` recognise when it is launched as `llama-server.exe` (it gets
//! `--host 127.0.0.1 --port N …`) and then behave like a tiny llama.cpp server: `/health`
//! (503 while "loading", then 200), `/v1/chat/completions` and — with `--embedding` —
//! `/v1/embeddings`, all requiring the per-launch API key. A `fail-start.txt` file next to the
//! executable makes it exit at once (a runtime that cannot start on this computer).

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use common::{ManifestSpec, TestKey, TestServer, payload, runtime_zip, test_config};
use openframe_ai::download::Control;
use openframe_ai::manifest::Backend;
use openframe_ai::store::AiPaths;
use openframe_ai::{AiManager, ChatMessage, ChatRequest, InstallPhase, ManagerConfig};

// ----------------------------------------------------------------- fake llama-server

fn fake_server(args: &[String]) -> ! {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap();
    if exe_dir.join("fail-start.txt").exists() {
        // Simulates a runtime that cannot start (bad graphics driver, unsupported CPU, …).
        std::process::exit(2);
    }
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let port: u16 = arg("--port").and_then(|p| p.parse().ok()).unwrap();
    let embedding = args.iter().any(|a| a == "--embedding");
    let key = std::env::var("LLAMA_API_KEY").unwrap_or_default();
    let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
    let mut health_calls = 0;
    for stream in listener.incoming() {
        let Ok(mut s) = stream else { continue };
        let mut reader = BufReader::new(s.try_clone().unwrap());
        let mut first = String::new();
        reader.read_line(&mut first).unwrap_or(0);
        let (mut auth, mut len) = (String::new(), 0usize);
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                break;
            }
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("authorization:") {
                auth = line.split_once(':').unwrap().1.trim().to_string();
            }
            if lower.starts_with("content-length:") {
                len = line.split_once(':').unwrap().1.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0u8; len];
        let _ = reader.read_exact(&mut body);
        let path = first.split_whitespace().nth(1).unwrap_or("/").to_string();
        let authed = auth == format!("Bearer {key}");
        let (status, out) = match path.as_str() {
            "/health" => {
                health_calls += 1;
                if health_calls <= 2 {
                    (
                        "503 Service Unavailable",
                        r#"{"error":{"message":"Loading model"}}"#.to_string(),
                    )
                } else {
                    ("200 OK", r#"{"status":"ok"}"#.to_string())
                }
            }
            "/v1/embeddings" if embedding && authed => {
                let req: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
                let inputs = req["input"].as_array().cloned().unwrap_or_default();
                // Like llama.cpp: an input longer than the model window is refused with 500.
                if inputs
                    .iter()
                    .any(|t| t.as_str().unwrap_or_default().len() > 400)
                {
                    let msg = r#"{"error":{"code":500,"message":"input is too large to process"}}"#;
                    let _ = write!(
                        s,
                        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{msg}",
                        msg.len()
                    );
                    continue;
                }
                let data: Vec<serde_json::Value> = inputs
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let t = t.as_str().unwrap_or_default();
                        // Deterministic, NOT normalised (the client must normalise).
                        let v: Vec<f32> = (0..8)
                            .map(|k| ((t.len() + 3 * k) % 7) as f32 + 1.0)
                            .collect();
                        serde_json::json!({"index": i, "embedding": v})
                    })
                    .collect();
                ("200 OK", serde_json::json!({"data": data}).to_string())
            }
            "/v1/chat/completions" if !embedding && authed => (
                "200 OK",
                r#"{"choices":[{"message":{"role":"assistant","content":"ready"}}]}"#.to_string(),
            ),
            "/v1/chat/completions" | "/v1/embeddings" if !authed => {
                ("401 Unauthorized", r#"{"error":"bad key"}"#.to_string())
            }
            _ => ("404 Not Found", "{}".to_string()),
        };
        let _ = write!(
            s,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{out}",
            out.len()
        );
    }
    std::process::exit(0)
}

// --------------------------------------------------------------------------- fixture

struct Fixture {
    server: TestServer,
    key: TestKey,
    dir: tempfile::TempDir,
    exe: Vec<u8>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            server: TestServer::start(),
            key: TestKey::new(),
            dir: tempfile::tempdir().unwrap(),
            exe: std::fs::read(std::env::current_exe().unwrap()).unwrap(),
        }
    }
    fn spec(&self) -> ManifestSpec {
        ManifestSpec::new(
            payload(40_000, 1),
            payload(9_000, 2),
            runtime_zip(&self.exe, &[]),
        )
    }
    fn config(&self, spec: &ManifestSpec) -> ManagerConfig {
        test_config(self.dir.path(), &self.key, spec.publish(&self.server))
    }
    fn paths(&self) -> AiPaths {
        AiPaths::new(self.dir.path())
    }
}

fn install(m: &AiManager) -> Vec<(InstallPhase, u64, u64)> {
    let seen = parking_lot::Mutex::new(Vec::new());
    m.install(&Control::new(), &|e| {
        seen.lock().push((e.phase, e.done, e.total))
    })
    .expect("install");
    seen.into_inner()
}

fn chat_works(m: &AiManager) {
    let model = m
        .chat_model()
        .expect("chat model available without restart");
    let out = model
        .chat(&ChatRequest::new(vec![ChatMessage::user("hi")]))
        .unwrap();
    assert_eq!(out, "ready");
}

// ------------------------------------------------------------------------- scenarios

fn one_click_install_is_verified_combined_healthy_and_usable_without_restart() {
    let f = Fixture::new();
    let spec = f.spec();
    let m = AiManager::new(f.config(&spec)).unwrap();
    assert!(!m.state().installed);
    assert!(m.embedding_info().is_none());
    assert_eq!(
        m.embed(&["x".into()]).unwrap_err().code_str(),
        "ai.not_installed"
    );

    // The exact download size is known before anything is downloaded.
    let plan = m.plan().unwrap();
    let runtime_len = spec.runtimes[0].2.len() as u64;
    let expected = runtime_len + spec.chat.len() as u64 + spec.embedding.len() as u64;
    assert_eq!(plan.download_bytes, expected);
    assert_eq!(plan.total_bytes, expected);
    assert!(plan.required_free_bytes > expected);
    assert!(!plan.up_to_date);

    let events = install(&m);
    // One combined progress: the total never changes and progress never goes backwards.
    let totals: Vec<u64> = events.iter().skip(1).map(|e| e.2).collect();
    assert!(totals.iter().all(|t| *t == expected), "{totals:?}");
    let dones: Vec<u64> = events.iter().map(|e| e.1).collect();
    assert!(dones.windows(2).all(|w| w[0] <= w[1]), "{dones:?}");
    // The user-visible steps, in order.
    let mut phases: Vec<InstallPhase> = events.iter().map(|e| e.0).collect();
    phases.dedup();
    let order = |p: InstallPhase| phases.iter().position(|x| *x == p).unwrap();
    assert!(order(InstallPhase::Checking) < order(InstallPhase::Downloading));
    assert!(order(InstallPhase::Downloading) < order(InstallPhase::Verifying));
    assert!(order(InstallPhase::Verifying) < order(InstallPhase::Starting));
    assert_eq!(
        *events.last().unwrap(),
        (InstallPhase::Ready, expected, expected)
    );

    // Active, both sidecars healthy, usable immediately.
    let state = m.state();
    assert!(state.installed && !state.update_available);
    let active = state.active.unwrap();
    assert_eq!(active.profile_id, "openframe-local-ai-v1");
    assert_eq!(active.chat_model_id, "test-chat-v1");
    chat_works(&m);
    let info = m.embedding_info().unwrap();
    assert_eq!((info.model_id.as_str(), info.dim), ("test-embedding-v1", 8));
    let texts: Vec<String> = (0..70).map(|i| format!("chunk {i}")).collect();
    let vectors = m.embed(&texts).unwrap();
    assert_eq!(vectors.len(), 70, "batched internally, order kept");
    for v in &vectors {
        assert_eq!(v.len(), 8);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "L2-normalised");
    }
    assert_eq!(m.embed(&[]).unwrap().len(), 0);
    // An input the runtime refuses as too long is shortened and retried; the others in its
    // batch are unaffected and the order is kept.
    let mixed = vec![
        "short".to_string(),
        "x".repeat(5_000),
        "also short".to_string(),
    ];
    let v = m.embed(&mixed).unwrap();
    assert_eq!(v.len(), 3);
    assert_eq!(v[0], m.embed(&["short".into()]).unwrap()[0]);
    assert!(m.plan().unwrap().up_to_date);
    m.stop_runtime();

    // A fresh manager (application restart) picks up the same install.
    drop(m);
    let m = AiManager::new(f.config(&spec)).unwrap();
    assert!(m.state().installed);
    assert_eq!(m.embedding_info().unwrap(), info);
    chat_works(&m);
    m.stop_runtime();
}

fn update_replaces_components_only_after_the_new_version_is_healthy() {
    let f = Fixture::new();
    let v1 = f.spec();
    let m = AiManager::new(f.config(&v1)).unwrap();
    install(&m);
    // Leftover of the retired three-tier system and a stale pointer format.
    let legacy = f.paths().model_dir("qwen3-recommended-win-x64-v1");
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(legacy.join("model.gguf"), b"old").unwrap();
    m.stop_runtime();
    drop(m);

    let mut v2 = f.spec();
    v2.sequence = 2;
    v2.version = "2".into();
    v2.chat_id = "test-chat-v2".into();
    v2.chat = payload(41_000, 3);
    let m = AiManager::new(f.config(&v2)).unwrap();
    assert!(m.state().update_available, "newer profile version offered");
    let plan = m.plan().unwrap();
    assert_eq!(
        plan.download_bytes,
        v2.chat.len() as u64,
        "only the changed model is downloaded"
    );
    install(&m);
    let a = m.active().unwrap();
    assert_eq!(
        (a.profile_version.as_str(), a.chat_model_id.as_str()),
        ("2", "test-chat-v2")
    );
    assert!(
        !f.paths().model_dir("test-chat-v1").exists(),
        "superseded model removed after success"
    );
    assert!(!legacy.exists(), "retired-profile leftovers removed");
    assert!(
        f.paths().model_dir("test-embedding-v1").exists(),
        "unchanged component kept"
    );
    chat_works(&m);
    m.stop_runtime();
}

fn graphics_runtime_failure_falls_back_to_the_processor_build() {
    let f = Fixture::new();
    let mut spec = f.spec();
    spec.runtimes[1].2 = runtime_zip(&f.exe, &[("fail-start.txt", b"x")]);
    let mut cfg = f.config(&spec);
    cfg.backend_override = Some(Backend::Vulkan);
    let m = AiManager::new(cfg).unwrap();
    assert_eq!(m.plan().unwrap().backend, Backend::Vulkan);
    let events = install(&m);
    let a = m.active().unwrap();
    assert_eq!(a.backend, Backend::Cpu);
    assert_eq!(a.runtime_id, "test-runtime-cpu");
    // The processor build was added to the same progress, which still ends complete.
    let last = events.last().unwrap();
    assert_eq!(last.1, last.2);
    assert!(
        !f.paths().runtime_dir("test-runtime-vulkan").exists(),
        "unused runtime removed"
    );
    chat_works(&m);
    assert_eq!(m.embed(&["x".into()]).unwrap().len(), 1);
    m.stop_runtime();
}

fn a_new_version_that_cannot_start_keeps_the_previous_install() {
    let f = Fixture::new();
    let v1 = f.spec();
    let m = AiManager::new(f.config(&v1)).unwrap();
    install(&m);
    m.stop_runtime();
    drop(m);
    let mut v2 = f.spec();
    v2.sequence = 2;
    v2.version = "2".into();
    v2.chat_id = "test-chat-v2".into();
    v2.chat = payload(41_000, 4);
    for rt in &mut v2.runtimes {
        rt.0 = format!("{}-2", rt.0);
        rt.2 = runtime_zip(&f.exe, &[("fail-start.txt", b"x")]);
    }
    let m = AiManager::new(f.config(&v2)).unwrap();
    let err = m.install(&Control::new(), &|_| {}).unwrap_err();
    assert_eq!(err.code_str(), "ai.start_failed");
    let a = m.active().unwrap();
    assert_eq!(a.profile_version, "1", "previous install still active");
    assert!(f.paths().model_file("test-chat-v1").exists());
    assert!(
        f.paths().model_file("test-chat-v2").exists(),
        "verified download kept for retry"
    );
    assert!(
        f.paths()
            .model_metadata("test-chat-v2")
            .unwrap()
            .health_check_failed
    );
    chat_works(&m);
    m.stop_runtime();
}

fn pause_keeps_progress_and_resume_finishes_cancel_discards() {
    let f = Fixture::new();
    let mut spec = f.spec();
    spec.chat = payload(3_000_000, 5);
    let m = AiManager::new(f.config(&spec)).unwrap();
    // A slow connection, so the pause lands part-way through the chat model.
    *f.server.state.throttle.lock() = Some(std::time::Duration::from_millis(4));
    let control = Control::new();
    let stop = Arc::new(AtomicBool::new(false));
    let (c2, s2) = (control.clone(), stop.clone());
    let chat_len = spec.chat.len() as u64;
    let runtime_len = spec.runtimes[0].2.len() as u64;
    let err = m
        .install(&control, &move |e| {
            if e.phase == InstallPhase::Downloading
                && e.done > runtime_len + 1024
                && e.done < runtime_len + chat_len
                && !s2.swap(true, Ordering::SeqCst)
            {
                c2.stop();
            }
        })
        .unwrap_err();
    assert_eq!(err.code_str(), "internal.cancelled");
    assert!(!m.state().installed);
    let plan = m.plan().unwrap();
    assert!(plan.downloaded_bytes > 0, "partial download kept");
    assert!(plan.download_bytes < plan.total_bytes);
    // Resume completes from where it stopped (with a range request, not from zero).
    *f.server.state.throttle.lock() = None;
    install(&m);
    assert!(
        f.server.state.ranges.lock().iter().any(|r| r != "bytes=0-"),
        "resumed with a range request"
    );
    assert!(m.state().installed);
    m.stop_runtime();

    // Cancel = discard partials.
    let f2 = Fixture::new();
    let mut spec2 = f2.spec();
    spec2.chat = payload(3_000_000, 6);
    let m2 = AiManager::new(f2.config(&spec2)).unwrap();
    *f2.server.state.throttle.lock() = Some(std::time::Duration::from_millis(4));
    let control = Control::new();
    let c3 = control.clone();
    let _ = m2.install(&control, &move |e| {
        if e.phase == InstallPhase::Downloading && e.done > 0 {
            c3.stop();
        }
    });
    m2.discard_partials();
    assert_eq!(m2.plan().unwrap().downloaded_bytes, 0);
}

fn uninstall_removes_everything_and_ai_is_off() {
    let f = Fixture::new();
    let spec = f.spec();
    let m = AiManager::new(f.config(&spec)).unwrap();
    install(&m);
    // Something unrelated in the app data folder must survive.
    let keep = f.dir.path().join("app.sqlite");
    std::fs::write(&keep, b"projects index").unwrap();
    let freed = m.uninstall().unwrap();
    assert!(freed >= spec.chat.len() as u64);
    assert!(!m.state().installed);
    assert!(m.active().is_none());
    assert!(m.chat_model().is_none());
    assert!(m.embedding_info().is_none());
    assert_eq!(
        m.embed(&["x".into()]).unwrap_err().code_str(),
        "ai.not_installed"
    );
    assert!(!f.paths().model_dir("test-chat-v1").exists());
    assert_eq!(
        std::fs::read_dir(f.paths().runtimes_dir()).unwrap().count(),
        0
    );
    assert!(keep.is_file(), "nothing outside the AI folders is touched");
    // It can be installed again later.
    install(&m);
    assert!(m.state().installed);
    m.stop_runtime();
}

#[cfg(windows)]
fn uninstall_never_follows_a_junction_out_of_the_ai_folders() {
    let f = Fixture::new();
    let victim = f.dir.path().join("victim");
    std::fs::create_dir_all(&victim).unwrap();
    std::fs::write(victim.join("keep.txt"), b"x").unwrap();
    std::fs::create_dir_all(f.paths().models_dir()).unwrap();
    let link = f.paths().models_dir().join("evil-link");
    let ok = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&victim)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("   (skipped: junctions unavailable)");
        return;
    }
    let spec = f.spec();
    let m = AiManager::new(f.config(&spec)).unwrap();
    m.uninstall().unwrap();
    assert!(
        victim.join("keep.txt").is_file(),
        "junction target untouched"
    );
}

#[cfg(not(windows))]
fn uninstall_never_follows_a_junction_out_of_the_ai_folders() {}

// ------------------------------------------------------------------------------ main

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--host") {
        fake_server(&args);
    }
    let filter = args.iter().find(|a| !a.starts_with('-')).cloned();
    let tests: &[(&str, fn())] = &[
        (
            "one_click_install_is_verified_combined_healthy_and_usable_without_restart",
            one_click_install_is_verified_combined_healthy_and_usable_without_restart,
        ),
        (
            "update_replaces_components_only_after_the_new_version_is_healthy",
            update_replaces_components_only_after_the_new_version_is_healthy,
        ),
        (
            "graphics_runtime_failure_falls_back_to_the_processor_build",
            graphics_runtime_failure_falls_back_to_the_processor_build,
        ),
        (
            "a_new_version_that_cannot_start_keeps_the_previous_install",
            a_new_version_that_cannot_start_keeps_the_previous_install,
        ),
        (
            "pause_keeps_progress_and_resume_finishes_cancel_discards",
            pause_keeps_progress_and_resume_finishes_cancel_discards,
        ),
        (
            "uninstall_removes_everything_and_ai_is_off",
            uninstall_removes_everything_and_ai_is_off,
        ),
        (
            "uninstall_never_follows_a_junction_out_of_the_ai_folders",
            uninstall_never_follows_a_junction_out_of_the_ai_folders,
        ),
    ];
    let mut failed = 0;
    let mut ran = 0;
    for (name, t) in tests {
        if filter.as_ref().is_some_and(|f| !name.contains(f.as_str())) {
            continue;
        }
        ran += 1;
        let r = std::panic::catch_unwind(t);
        println!(
            "test {name} ... {}",
            if r.is_ok() { "ok" } else { "FAILED" }
        );
        if r.is_err() {
            failed += 1;
        }
    }
    println!(
        "\ntest result: {}. {} passed; {failed} failed",
        if failed == 0 { "ok" } else { "FAILED" },
        ran - failed
    );
    if failed > 0 {
        std::process::exit(101);
    }
}

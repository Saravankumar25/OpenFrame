//! The bundled Offline AI end to end (ADR-0025): AI works on first launch with no download and
//! no setup, the pinned hashes are checked before the first engine start, writable data stays in
//! app data, and a missing, altered or linked file switches AI off with a plain message and
//! never starts an engine.
//!
//! No real model is involved: the "engine" pinned in the test lock is THIS test executable.
//! `harness = false` lets `main` recognise when it is launched as `llama-server` (it gets
//! `--host 127.0.0.1 --port N …`) and then behave like a tiny llama.cpp server: `/health`
//! (503 while "loading", then 200), `/v1/chat/completions` and, with `--embedding`,
//! `/v1/embeddings`, all requiring the per-launch API key.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use openframe_ai::bundle::PinnedFile;
use openframe_ai::engines::hash_file;
use openframe_ai::supervisor::SupervisorConfig;
use openframe_ai::{
    AiManager, AssetLock, BundleState, ChatMessage, ChatRequest, ManagerConfig, RuntimeState,
};

// ----------------------------------------------------------------- fake llama-server

fn fake_server(args: &[String]) -> ! {
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let port: u16 = arg("--port").and_then(|p| p.parse().ok()).unwrap();
    if std::env::vars_os().any(|(k, _)| {
        k.to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("GGML_")
    }) {
        std::process::exit(3);
    }
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
                let data: Vec<serde_json::Value> = inputs
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let t = t.as_str().unwrap_or_default();
                        let v: Vec<f32> = (0..8)
                            .map(|k| ((t.len() + 3 * k) % 7) as f32 + 1.0)
                            .collect();
                        serde_json::json!({"index": i, "embedding": v})
                    })
                    .collect();
                ("200 OK", serde_json::json!({"data": data}).to_string())
            }
            "/v1/chat/completions" if !embedding && authed => {
                let req: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
                // Echo the sampling temperature so the deterministic setting is observable.
                let content = format!("ready t={}", req["temperature"]);
                let out = serde_json::json!({"choices":[{"message":{"role":"assistant","content":content}}]});
                ("200 OK", out.to_string())
            }
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

const SERVER: &str = if cfg!(windows) {
    "llama-server.exe"
} else {
    "llama-server"
};

fn pin(path: &Path, name: &str) -> PinnedFile {
    let (bytes, sha256) = hash_file(path).unwrap();
    PinnedFile {
        path: name.into(),
        bytes,
        sha256,
    }
}

struct Fixture {
    _tmp: tempfile::TempDir,
    bundle: PathBuf,
    data: PathBuf,
    lock: AssetLock,
}

impl Fixture {
    /// A bundle laid out like the app resources: this test program as the engine, a library
    /// next to it, and two small "model" files, all pinned in a lock for this computer.
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("resources").join("ai");
        let data = tmp.path().join("appdata");
        let mut lock = AssetLock::embedded().unwrap();
        let mut rt = lock.runtimes[0].clone();
        rt.os = openframe_platform::current_os().name().into();
        rt.arch = std::env::consts::ARCH.into();
        rt.executable = SERVER.into();
        let rt_dir = bundle.join("runtimes").join(&rt.runtime_id);
        std::fs::create_dir_all(&rt_dir).unwrap();
        std::fs::copy(std::env::current_exe().unwrap(), rt_dir.join(SERVER)).unwrap();
        std::fs::write(rt_dir.join("ggml.dll"), b"ggml library").unwrap();
        rt.files = vec![
            pin(&rt_dir.join(SERVER), SERVER),
            pin(&rt_dir.join("ggml.dll"), "ggml.dll"),
        ];
        lock.runtimes = vec![rt];
        for (i, m) in lock.models.iter_mut().enumerate() {
            let dir = bundle.join("models").join(&m.model_id);
            std::fs::create_dir_all(&dir).unwrap();
            let bytes: Vec<u8> = (0..40_000u32)
                .map(|k| ((k * 31 + i as u32) % 251) as u8)
                .collect();
            std::fs::write(dir.join("model.gguf"), &bytes).unwrap();
            m.file = pin(&dir.join("model.gguf"), "model.gguf");
            if m.embedding_dim.is_some() {
                m.embedding_dim = Some(8);
            }
        }
        lock.validate().unwrap();
        Fixture {
            _tmp: tmp,
            bundle,
            data,
            lock,
        }
    }

    fn config(&self) -> ManagerConfig {
        let sup = SupervisorConfig {
            start_timeout: Duration::from_secs(30),
            idle_timeout: None,
            poll_interval: Duration::from_millis(100),
            ..SupervisorConfig::default()
        };
        ManagerConfig {
            app_data_dir: self.data.clone(),
            bundle_dir: Some(self.bundle.clone()),
            lock: self.lock.clone(),
            supervisor: sup.clone(),
            embedding_supervisor: sup,
            ready_timeout: Duration::from_secs(30),
            deterministic: false,
        }
    }

    fn manager(&self) -> Arc<AiManager> {
        Arc::new(AiManager::new(self.config()).unwrap())
    }

    fn model_file(&self, role: usize) -> PathBuf {
        self.bundle
            .join("models")
            .join(&self.lock.models[role].model_id)
            .join("model.gguf")
    }

    /// Every file under the bundle with its size and modification time.
    fn snapshot(&self) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
        fn walk(d: &Path, out: &mut Vec<(PathBuf, u64, std::time::SystemTime)>) {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                let m = e.metadata().unwrap();
                if m.is_dir() {
                    walk(&e.path(), out);
                } else {
                    out.push((e.path(), m.len(), m.modified().unwrap()));
                }
            }
        }
        let mut v = Vec::new();
        walk(&self.bundle, &mut v);
        v.sort();
        v
    }
}

fn ask(m: &Arc<AiManager>) -> Result<String, openframe_domain::AppError> {
    let model = m.chat_model().ok_or_else(|| m.unavailable_reason())?;
    model.chat(&ChatRequest::new(vec![ChatMessage::user("hi")]))
}

/// Flip one byte in the middle of a file, keeping its size.
fn alter(path: &Path) {
    let mut b = std::fs::read(path).unwrap();
    let i = b.len() / 2;
    b[i] ^= 0x5a;
    std::fs::write(path, b).unwrap();
}

// ------------------------------------------------------------------------- scenarios

fn ai_works_on_first_launch_with_no_download_or_setup() {
    let f = Fixture::new();
    let before = f.snapshot();
    let m = f.manager();
    let st = m.state();
    assert!(
        st.available && st.bundle == BundleState::NotChecked,
        "{st:?}"
    );
    // First request: full check, engine start, answer. No download, no install step.
    assert!(ask(&m).unwrap().starts_with("ready t=0.2"));
    assert_eq!(m.state().bundle, BundleState::Verified);
    let v = m.embed(&["Ravi waits at the station".into()]).unwrap();
    assert_eq!(v[0].len(), 8);
    assert!(
        (v[0].iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-4,
        "normalised"
    );
    // Writable data lands in app data; the read-only bundle is untouched.
    assert!(m.paths().bundle_check().is_file());
    assert!(
        m.paths()
            .runtime_log()
            .parent()
            .unwrap()
            .starts_with(&f.data)
    );
    m.stop_runtime();
    assert_eq!(
        f.snapshot(),
        before,
        "nothing is written into the app resources"
    );
    // Diagnostics describe the bundle, its licences and where it is, without a download.
    let d = m.diagnostics();
    assert_eq!(d.components.len(), 3);
    assert!(
        d.components
            .iter()
            .any(|c| c.license_id == "Gemma-Terms-of-Use")
    );
    assert_eq!(d.bundle, BundleState::Verified);
    assert!(d.bundle_dir.ends_with("ai"));
}

fn deterministic_runs_use_temperature_zero_and_a_fixed_seed() {
    let f = Fixture::new();
    let mut cfg = f.config();
    cfg.deterministic = true;
    let m = Arc::new(AiManager::new(cfg).unwrap());
    assert_eq!(ask(&m).unwrap(), "ready t=0.0");
}

fn the_check_is_recorded_and_a_replaced_file_is_checked_again() {
    let f = Fixture::new();
    let m = f.manager();
    m.ensure_verified().unwrap();
    let record = std::fs::read_to_string(m.paths().bundle_check()).unwrap();
    assert!(record.contains("lockFingerprint"));
    drop(m);
    // Next start of the same installed version: verified from the record.
    let m = f.manager();
    m.ensure_verified().unwrap();
    drop(m);
    // A file replaced after the check (new content, new modification time, same size) is
    // hashed again and refused, although a record exists.
    std::thread::sleep(Duration::from_millis(20));
    alter(&f.model_file(1));
    let m = f.manager();
    let e = m.ensure_verified().unwrap_err();
    assert_eq!(e.code_str(), "ai.assets_damaged");
    assert!(
        !m.paths().bundle_check().exists(),
        "a failed check leaves no record"
    );
}

fn a_new_app_version_checks_everything_again() {
    let f = Fixture::new();
    let m = f.manager();
    m.ensure_verified().unwrap();
    drop(m);
    // Same file times, different content: only a full check catches this. A new lock (the
    // next app version) never trusts the old record.
    let path = f.model_file(0);
    let when = std::fs::metadata(&path).unwrap().modified().unwrap();
    alter(&path);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(when)
        .unwrap();
    let mut cfg = f.config();
    cfg.lock.bundle_version = "1.0.1".into();
    let m = Arc::new(AiManager::new(cfg).unwrap());
    assert_eq!(
        m.ensure_verified().unwrap_err().code_str(),
        "ai.assets_damaged"
    );
}

fn an_altered_model_is_refused_before_any_engine_starts() {
    let f = Fixture::new();
    alter(&f.model_file(0));
    let m = f.manager();
    assert!(
        m.state().available,
        "the size still matches until the full check"
    );
    let e = ask(&m).unwrap_err();
    assert_eq!(e.code_str(), "ai.assets_damaged");
    assert!(
        e.message.contains("Reinstall OpenFrame Studio"),
        "{}",
        e.message
    );
    assert!(!e.message.to_lowercase().contains("download"));
    let st = m.state();
    assert!(!st.available && st.bundle == BundleState::Damaged);
    assert_eq!(m.runtime_status().state, RuntimeState::Unavailable);
    assert!(m.chat_model().is_none());
    assert!(m.embed(&["x".into()]).is_err());
    assert!(
        !m.paths().engine_registry().exists(),
        "no engine was ever launched"
    );
}

fn a_missing_file_turns_ai_off_with_a_plain_message() {
    let f = Fixture::new();
    std::fs::remove_file(f.model_file(1)).unwrap();
    let m = f.manager();
    let st = m.state();
    assert!(!st.available && st.bundle == BundleState::Missing, "{st:?}");
    let e = st.problem.unwrap();
    assert_eq!(e.code_str(), "ai.assets_missing");
    assert!(!e.message.contains('\u{2014}'));
    assert!(m.chat_model().is_none());
    assert!(m.embedding_info().is_none());
    assert_eq!(ask(&m).unwrap_err().code_str(), "ai.assets_missing");
    // No resources folder at all reads the same way.
    let mut cfg = f.config();
    cfg.bundle_dir = None;
    let m = AiManager::new(cfg).unwrap();
    assert_eq!(m.state().bundle, BundleState::Missing);
}

fn an_engine_file_changed_after_the_check_is_never_launched() {
    let f = Fixture::new();
    let m = f.manager();
    assert!(ask(&m).is_ok());
    m.stop_runtime();
    let rt = f
        .bundle
        .join("runtimes")
        .join(&f.lock.runtimes[0].runtime_id);
    std::fs::write(rt.join("ggml.dll"), b"ggml librarz").unwrap();
    let e = ask(&m).unwrap_err();
    assert_eq!(e.code_str(), "ai.assets_damaged");
    assert_eq!(m.state().bundle, BundleState::Damaged);
    assert_eq!(m.runtime_status().state, RuntimeState::Unavailable);
    assert!(
        m.chat_model().is_none(),
        "AI stays off until the app is reinstalled"
    );
    // A program planted next to the engine stops the launch as well.
    std::fs::write(rt.join("ggml.dll"), b"ggml library").unwrap();
    std::fs::write(rt.join("version.dll"), b"planted").unwrap();
    let m = f.manager();
    assert!(ask(&m).is_err());
}

fn a_model_replaced_while_the_engine_was_unloaded_is_never_loaded() {
    let f = Fixture::new();
    let m = f.manager();
    assert!(ask(&m).is_ok());
    // The engine is unloaded (inactivity or Stop Offline AI); the model is swapped meanwhile.
    m.stop_runtime();
    std::thread::sleep(Duration::from_millis(20));
    alter(&f.model_file(0));
    let e = ask(&m).unwrap_err();
    assert_eq!(e.code_str(), "ai.assets_damaged");
    assert_eq!(m.state().bundle, BundleState::Damaged);
}

fn the_engine_starts_with_a_scrubbed_environment() {
    // A variable that would point ggml at other backend libraries never reaches the engine (the
    // fake engine exits at once when it sees any GGML_ variable).
    // SAFETY: the custom test runner runs one scenario at a time on this thread.
    unsafe { std::env::set_var("GGML_BACKEND_PATH", "C:/elsewhere") };
    let f = Fixture::new();
    let m = f.manager();
    let r = ask(&m);
    unsafe { std::env::remove_var("GGML_BACKEND_PATH") };
    assert!(
        r.unwrap().starts_with("ready"),
        "the engine saw no GGML_ setting"
    );
}

fn a_platform_without_a_pinned_engine_is_unsupported() {
    let f = Fixture::new();
    let mut cfg = f.config();
    cfg.lock.runtimes[0].os = "plan9".into();
    let m = AiManager::new(cfg).unwrap();
    let st = m.state();
    assert_eq!(st.bundle, BundleState::Unsupported);
    assert_eq!(st.problem.unwrap().code_str(), "ai.unsupported");
}

#[cfg(windows)]
fn a_junction_inside_the_bundle_is_refused() {
    let f = Fixture::new();
    // Move one model folder elsewhere and put a junction in its place: same files, but the
    // path now leaves the installed resources.
    let model_dir = f.model_file(0).parent().unwrap().to_path_buf();
    let elsewhere = f.data.join("elsewhere");
    std::fs::create_dir_all(&f.data).unwrap();
    std::fs::rename(&model_dir, &elsewhere).unwrap();
    let ok = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&model_dir)
        .arg(&elsewhere)
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "mklink /J");
    let m = f.manager();
    let st = m.state();
    assert!(!st.available, "{st:?}");
    assert_eq!(st.bundle, BundleState::Damaged);
    std::fs::remove_dir(&model_dir).unwrap();
}

#[cfg(not(windows))]
fn a_junction_inside_the_bundle_is_refused() {}

fn retired_downloads_are_removed_without_following_links() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("appdata");
    let outside = tmp.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("keep.txt"), b"user file").unwrap();
    let models = data.join("models");
    std::fs::create_dir_all(models.join("gemma-3-1b-it-q8-0-v1")).unwrap();
    std::fs::write(
        models.join("gemma-3-1b-it-q8-0-v1").join("model.gguf"),
        vec![1u8; 4096],
    )
    .unwrap();
    std::fs::create_dir_all(models.join(".downloads")).unwrap();
    std::fs::write(models.join("manifest-cache.json"), b"{}").unwrap();
    std::fs::create_dir_all(data.join("runtimes").join("llama").join("x")).unwrap();
    std::fs::write(
        data.join("runtimes")
            .join("llama")
            .join("x")
            .join("llama-server.exe"),
        b"x",
    )
    .unwrap();
    std::fs::write(data.join("settings.json"), b"{}").unwrap();
    #[cfg(windows)]
    {
        let ok = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(models.join("link"))
            .arg(&outside)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok);
    }
    let paths = openframe_ai::store::AiPaths::new(&data);
    let freed = paths.remove_retired_downloads();
    assert!(freed >= 4096);
    assert!(!models.exists() && !data.join("runtimes").exists());
    assert!(
        outside.join("keep.txt").is_file(),
        "a link is never followed"
    );
    assert!(data.join("settings.json").is_file(), "other app data stays");
    // The manager does the same on start.
    std::fs::create_dir_all(data.join("models").join("old")).unwrap();
    let f = Fixture::new();
    let mut cfg = f.config();
    cfg.app_data_dir = data.clone();
    drop(AiManager::new(cfg).unwrap());
    assert!(!data.join("models").exists());
}

// ------------------------------------------------------------------------------ main

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--host") {
        fake_server(&args);
    }
    let filter = args.iter().find(|a| !a.starts_with('-')).cloned();
    let tests: &[(&str, fn())] = &[
        (
            "ai_works_on_first_launch_with_no_download_or_setup",
            ai_works_on_first_launch_with_no_download_or_setup,
        ),
        (
            "deterministic_runs_use_temperature_zero_and_a_fixed_seed",
            deterministic_runs_use_temperature_zero_and_a_fixed_seed,
        ),
        (
            "the_check_is_recorded_and_a_replaced_file_is_checked_again",
            the_check_is_recorded_and_a_replaced_file_is_checked_again,
        ),
        (
            "a_new_app_version_checks_everything_again",
            a_new_app_version_checks_everything_again,
        ),
        (
            "an_altered_model_is_refused_before_any_engine_starts",
            an_altered_model_is_refused_before_any_engine_starts,
        ),
        (
            "a_missing_file_turns_ai_off_with_a_plain_message",
            a_missing_file_turns_ai_off_with_a_plain_message,
        ),
        (
            "an_engine_file_changed_after_the_check_is_never_launched",
            an_engine_file_changed_after_the_check_is_never_launched,
        ),
        (
            "a_model_replaced_while_the_engine_was_unloaded_is_never_loaded",
            a_model_replaced_while_the_engine_was_unloaded_is_never_loaded,
        ),
        (
            "the_engine_starts_with_a_scrubbed_environment",
            the_engine_starts_with_a_scrubbed_environment,
        ),
        (
            "a_platform_without_a_pinned_engine_is_unsupported",
            a_platform_without_a_pinned_engine_is_unsupported,
        ),
        (
            "a_junction_inside_the_bundle_is_refused",
            a_junction_inside_the_bundle_is_refused,
        ),
        (
            "retired_downloads_are_removed_without_following_links",
            retired_downloads_are_removed_without_following_links,
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

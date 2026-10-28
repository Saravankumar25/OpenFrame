//! Sidecar supervisor lifecycle against a fake loopback server.
//!
//! The fake server is this test binary re-launched with an environment
//! variable (see `fake_llama_server_entry`), so no real runtime or model is
//! needed: it answers `/health` (503 while "loading", then 200), requires the
//! per-launch API key on `/v1/chat/completions`, and exits on `/crash`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use openframe_ai::client::{self, ChatMessage, ChatRequest};
use openframe_ai::rt::AiRuntime;
use openframe_ai::supervisor::{
    LaunchSpec, Launcher, LlamaServerLauncher, RuntimeState, START_BIND_ATTEMPTS, Supervisor,
    SupervisorConfig,
};

const ENV_PORT: &str = "OF_FAKE_LLAMA_PORT";
const ENV_EXIT: &str = "OF_FAKE_LLAMA_EXIT";

/// Entry point of the fake server process (a no-op in normal test runs).
#[test]
fn fake_llama_server_entry() {
    let Ok(port) = std::env::var(ENV_PORT) else {
        return;
    };
    if std::env::var(ENV_EXIT).is_ok() {
        // Simulates a runtime that cannot start (bad driver, unsupported CPU, …).
        std::process::exit(2);
    }
    let key = std::env::var("LLAMA_API_KEY").unwrap_or_default();
    let listener = match TcpListener::bind(("127.0.0.1", port.parse::<u16>().unwrap())) {
        Ok(l) => l,
        Err(_) => {
            // What llama-server does when its port is taken: report and exit at once.
            eprintln!("couldn't bind HTTP server socket, hostname: 127.0.0.1");
            std::process::exit(1);
        }
    };
    let mut health_calls = 0;
    for stream in listener.incoming() {
        let Ok(mut s) = stream else { continue };
        let mut reader = BufReader::new(s.try_clone().unwrap());
        let mut first = String::new();
        reader.read_line(&mut first).unwrap_or(0);
        let mut auth = String::new();
        let mut len = 0usize;
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
        let (status, payload) = match path.as_str() {
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
            "/v1/chat/completions" if auth == format!("Bearer {key}") => {
                let req: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
                let constrained = req.get("response_format").is_some();
                let content = if constrained {
                    r#"{\"tool\":\"count_scenes\",\"arguments\":{}}"#
                } else {
                    "<think>hidden</think>ready"
                };
                (
                    "200 OK",
                    format!(
                        r#"{{"choices":[{{"message":{{"role":"assistant","content":"{content}"}}}}]}}"#
                    ),
                )
            }
            "/v1/chat/completions" => ("401 Unauthorized", r#"{"error":"bad key"}"#.to_string()),
            "/crash" => std::process::exit(3),
            _ => ("404 Not Found", "{}".to_string()),
        };
        let _ = write!(
            s,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
            payload.len()
        );
    }
}

struct FakeLauncher;
struct BrokenLauncher;

fn fake_spec(port: u16, api_key: &str) -> LaunchSpec {
    LaunchSpec {
        program: std::env::current_exe().unwrap(),
        args: vec![
            "fake_llama_server_entry".into(),
            "--exact".into(),
            "--nocapture".into(),
            "--test-threads=1".into(),
        ],
        env: vec![
            (ENV_PORT.into(), port.to_string()),
            ("LLAMA_API_KEY".into(), api_key.to_string()),
        ],
        cwd: None,
    }
}

impl Launcher for BrokenLauncher {
    fn launch_spec(&self, port: u16, api_key: &str) -> LaunchSpec {
        let mut spec = fake_spec(port, api_key);
        spec.env.push((ENV_EXIT.into(), "1".into()));
        spec
    }
    fn model_reference(&self) -> String {
        "broken-profile".into()
    }
}

impl Launcher for FakeLauncher {
    fn launch_spec(&self, port: u16, api_key: &str) -> LaunchSpec {
        fake_spec(port, api_key)
    }
    fn model_reference(&self) -> String {
        "fake-profile".into()
    }
}

fn cfg() -> SupervisorConfig {
    SupervisorConfig {
        start_timeout: Duration::from_secs(30),
        idle_timeout: None,
        poll_interval: Duration::from_millis(100),
        max_restarts: 2,
        restart_window: Duration::from_secs(60),
        log_path: None,
    }
}

#[test]
fn supervisor_starts_serves_restarts_after_crash_and_stops() {
    let rt = Arc::new(AiRuntime::new().unwrap());
    let sup = Supervisor::new(cfg(), rt.handle());
    let states = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let s2 = states.clone();
    sup.set_listener(Arc::new(move |s| s2.lock().push(s)));

    assert_eq!(sup.state(), RuntimeState::Unavailable);
    assert_eq!(sup.start().unwrap_err().code_str(), "ai.assets_missing");
    sup.configure(Some(Arc::new(FakeLauncher)));
    assert_eq!(sup.state(), RuntimeState::Installed);

    let ep = sup.ensure_ready(Duration::from_secs(30)).unwrap();
    assert!(
        ep.base_url.starts_with("http://127.0.0.1:"),
        "loopback only"
    );
    {
        let seen = states.lock().clone();
        assert!(seen.contains(&RuntimeState::Starting));
        assert!(
            seen.contains(&RuntimeState::LoadingModel),
            "503 while loading is reported: {seen:?}"
        );
        assert!(seen.contains(&RuntimeState::Ready));
    }

    // Authenticated chat; reasoning never surfaces.
    let http = sup.http().clone();
    let req = ChatRequest::new(vec![ChatMessage::user("hi")]);
    let ep2 = ep.clone();
    let text = rt
        .run(async move { client::chat(&http, &ep2, &req).await })
        .unwrap()
        .0;
    assert_eq!(text, "ready");
    // A wrong key is refused by the server.
    let http = sup.http().clone();
    let bad = client::Endpoint {
        base_url: ep.base_url.clone(),
        api_key: "wrong".into(),
    };
    let req = ChatRequest::new(vec![ChatMessage::user("hi")]);
    assert!(
        rt.run(async move { client::chat(&http, &bad, &req).await })
            .is_err()
    );

    // Busy while a request is in flight.
    {
        let _g = sup.begin_request();
        assert_eq!(sup.state(), RuntimeState::Busy);
    }
    assert_eq!(sup.state(), RuntimeState::Ready);

    // Unexpected exit → automatic restart on a fresh port and key.
    {
        let mut s =
            std::net::TcpStream::connect(ep.base_url.trim_start_matches("http://")).unwrap();
        write!(s, "GET /crash HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
        let mut sink = Vec::new();
        let _ = s.read_to_end(&mut sink);
    }
    // Wait until the monitor notices the exit and relaunches.
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while !states
        .lock()
        .iter()
        .filter(|s| **s == RuntimeState::Starting)
        .nth(1)
        .is_some()
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    let ep_after = sup.wait_ready(Duration::from_secs(30)).unwrap();
    assert_ne!(ep_after.api_key, ep.api_key, "each launch gets a new key");

    sup.stop();
    assert_eq!(sup.state(), RuntimeState::Installed);
    // Dropping the supervisor never leaves a process behind (stop is idempotent).
    drop(sup);
}

#[test]
fn llama_server_launch_is_loopback_only_and_key_is_not_on_the_command_line() {
    let l = LlamaServerLauncher {
        executable: PathBuf::from("C:/rt/llama-server.exe"),
        engine_files: None,
        model_path: PathBuf::from("C:/m/model.gguf"),
        profile_id: "p".into(),
        context_tokens: 8192,
        threads: 4,
        seed: None,
        model_stamp: None,
        gpu_device: None,
    };
    let spec = l.launch_spec(51234, "secret-key");
    let joined = spec.args.join(" ");
    assert!(joined.contains("--host 127.0.0.1"));
    assert!(!joined.contains("0.0.0.0"));
    assert!(joined.contains("--port 51234"));
    assert!(joined.contains("--no-webui"));
    assert!(joined.contains("--offline"));
    assert!(
        !joined.contains("secret-key"),
        "API key is passed via the environment"
    );
    assert!(
        spec.env
            .iter()
            .any(|(k, v)| k == "LLAMA_API_KEY" && v == "secret-key")
    );
    // One slot, and a bounded store for the work on earlier prompts (the engine default is 8 GiB).
    assert!(joined.contains("--parallel 1"), "{joined}");
    assert!(
        joined.contains(&format!(
            "--cache-ram {}",
            openframe_ai::supervisor::PROMPT_STORE_MB
        )),
        "{joined}"
    );
    // On the processor unless the launch plan names a card; no seed unless a deterministic run asks.
    assert!(joined.contains("-ngl 0"));
    assert!(!joined.contains("--device"));
    assert!(!joined.contains("--seed"));
    let card = LlamaServerLauncher {
        gpu_device: Some("Vulkan1".into()),
        ..l.clone()
    };
    let card = card.launch_spec(1, "k").args.join(" ");
    assert!(
        card.contains("-ngl 999") && card.contains("--device Vulkan1"),
        "{card}"
    );
    let seeded = LlamaServerLauncher {
        seed: Some(42),
        ..l.clone()
    };
    assert!(
        seeded
            .launch_spec(1, "k")
            .args
            .join(" ")
            .contains("--seed 42")
    );
}

#[test]
fn idle_model_is_unloaded_and_reloads_on_the_next_request() {
    let rt = Arc::new(AiRuntime::new().unwrap());
    let sup = Supervisor::new(
        SupervisorConfig {
            idle_timeout: Some(Duration::from_millis(300)),
            ..cfg()
        },
        rt.handle(),
    );
    sup.configure(Some(Arc::new(FakeLauncher)));
    sup.ensure_ready(Duration::from_secs(30)).unwrap();
    // Nothing in flight: after the idle timeout the model is unloaded (memory released).
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while sup.state() != RuntimeState::Installed && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        sup.state(),
        RuntimeState::Installed,
        "idle runtime was stopped"
    );
    // The next request starts it again transparently.
    let ep = sup.ensure_ready(Duration::from_secs(30)).unwrap();
    assert!(ep.base_url.starts_with("http://127.0.0.1:"));
    sup.stop();
}

#[test]
fn runtime_that_cannot_start_fails_cleanly_without_restart_loops() {
    let rt = Arc::new(AiRuntime::new().unwrap());
    let sup = Supervisor::new(cfg(), rt.handle());
    let states = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let s2 = states.clone();
    sup.set_listener(Arc::new(move |s| s2.lock().push(s)));
    sup.configure(Some(Arc::new(BrokenLauncher)));
    let err = sup.ensure_ready(Duration::from_secs(30)).unwrap_err();
    assert_eq!(err.code_str(), "ai.unavailable");
    assert_eq!(sup.state(), RuntimeState::Failed);
    let status = sup.status();
    assert_eq!(
        status.last_error.as_deref(),
        Some("Offline AI could not start on this computer.")
    );
    assert_eq!(
        states
            .lock()
            .iter()
            .filter(|s| **s == RuntimeState::Starting)
            .count(),
        1,
        "a start failure is not retried automatically"
    );
    // An explicit retry is allowed (and fails the same way).
    assert!(sup.ensure_ready(Duration::from_secs(30)).is_err());
}

#[test]
fn embedding_server_launch_is_loopback_only_processor_only_and_keyed_by_environment() {
    use openframe_ai::bundle::Pooling;
    use openframe_ai::supervisor::EmbeddingServerLauncher;
    let l = EmbeddingServerLauncher {
        executable: PathBuf::from("C:/rt/llama-server.exe"),
        engine_files: None,
        model_path: PathBuf::from("C:/m/embedding.gguf"),
        model_id: "bge-small-en-v1.5-q8-0-v1".into(),
        pooling: Pooling::Cls,
        context_tokens: 512,
        threads: 2,
        model_stamp: None,
    };
    let spec = l.launch_spec(51235, "secret-key");
    let joined = spec.args.join(" ");
    assert!(joined.contains("--host 127.0.0.1"));
    assert!(!joined.contains("0.0.0.0"));
    assert!(joined.contains("--embedding"));
    assert!(joined.contains("--pooling cls"));
    assert!(
        joined.contains("-ngl 0"),
        "embeddings stay on the processor"
    );
    assert!(joined.contains("-ub 512"), "one input fits one batch");
    assert!(joined.contains("--no-webui") && joined.contains("--offline"));
    assert!(!joined.contains("secret-key"));
    assert_eq!(l.model_reference(), "bge-small-en-v1.5-q8-0-v1");
}

/// The free-port race: another program binds the loopback port the supervisor picked before the
/// runtime does. The first `squat` launches find their port taken (as llama-server would, they
/// exit at once); every launch is recorded with its port and key.
struct SquattingLauncher {
    squat: usize,
    launches: parking_lot::Mutex<Vec<(u16, String)>>,
    squatters: parking_lot::Mutex<Vec<TcpListener>>,
}

impl SquattingLauncher {
    fn new(squat: usize) -> Self {
        Self {
            squat,
            launches: parking_lot::Mutex::new(Vec::new()),
            squatters: parking_lot::Mutex::new(Vec::new()),
        }
    }
    fn launches(&self) -> Vec<(u16, String)> {
        self.launches.lock().clone()
    }
}

impl Launcher for SquattingLauncher {
    fn launch_spec(&self, port: u16, api_key: &str) -> LaunchSpec {
        let mut launches = self.launches.lock();
        if launches.len() < self.squat {
            let squatter = TcpListener::bind(("127.0.0.1", port))
                .expect("the picked port is free until someone else takes it");
            self.squatters.lock().push(squatter);
        }
        launches.push((port, api_key.to_string()));
        fake_spec(port, api_key)
    }
    fn model_reference(&self) -> String {
        "squatted-profile".into()
    }
}

fn port_of(base_url: &str) -> u16 {
    base_url
        .strip_prefix("http://127.0.0.1:")
        .expect("loopback only")
        .parse()
        .unwrap()
}

#[test]
fn m1_10_sidecar_start_retries_on_a_new_port_when_the_picked_port_is_taken() {
    let rt = Arc::new(AiRuntime::new().unwrap());
    let sup = Supervisor::new(cfg(), rt.handle());
    let launcher = Arc::new(SquattingLauncher::new(2));
    sup.configure(Some(launcher.clone()));

    let ep = sup.ensure_ready(Duration::from_secs(60)).unwrap();
    assert_eq!(sup.state(), RuntimeState::Ready);
    let launches = launcher.launches();
    assert_eq!(
        launches.len(),
        3,
        "two collisions, then a launch on a free port: {launches:?}"
    );
    let ports: std::collections::BTreeSet<u16> = launches.iter().map(|(p, _)| *p).collect();
    assert_eq!(ports.len(), 3, "every retry gets a new port");
    let keys: std::collections::BTreeSet<&str> = launches.iter().map(|(_, k)| k.as_str()).collect();
    assert_eq!(keys.len(), 3, "every launch gets a new per-launch key");
    let (last_port, last_key) = launches.last().unwrap().clone();
    assert!(
        ep.base_url.starts_with("http://127.0.0.1:"),
        "loopback only"
    );
    assert_eq!(port_of(&ep.base_url), last_port);
    assert_eq!(ep.api_key, last_key);
    for (p, _) in &launches[..2] {
        assert_ne!(*p, port_of(&ep.base_url), "never the taken port");
    }

    // The endpoint is our runtime (it knows this launch's key), not the program on the old port.
    let http = sup.http().clone();
    let req = ChatRequest::new(vec![ChatMessage::user("hi")]);
    let ep2 = ep.clone();
    let text = rt
        .run(async move { client::chat(&http, &ep2, &req).await })
        .unwrap()
        .0;
    assert_eq!(text, "ready");
    sup.stop();
    assert_eq!(sup.state(), RuntimeState::Installed);
}

#[test]
fn m1_10_sidecar_port_retries_are_bounded_and_reset_on_an_explicit_start() {
    let rt = Arc::new(AiRuntime::new().unwrap());
    let sup = Supervisor::new(cfg(), rt.handle());
    // Taken for the whole first start, and for the first two launches of the second.
    let squat = START_BIND_ATTEMPTS as usize + 2;
    let launcher = Arc::new(SquattingLauncher::new(squat));
    sup.configure(Some(launcher.clone()));

    let err = sup.ensure_ready(Duration::from_secs(60)).unwrap_err();
    assert_eq!(err.code_str(), "ai.unavailable");
    assert_eq!(sup.state(), RuntimeState::Failed);
    assert_eq!(
        sup.status().last_error.as_deref(),
        Some("Offline AI could not start on this computer.")
    );
    assert_eq!(
        launcher.launches().len(),
        START_BIND_ATTEMPTS as usize,
        "at most {START_BIND_ATTEMPTS} launches per start"
    );
    // No retry loop continues in the background once the start has failed.
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(launcher.launches().len(), START_BIND_ATTEMPTS as usize);
    assert_eq!(sup.state(), RuntimeState::Failed);

    // An explicit start gets a fresh retry budget.
    let ep = sup.ensure_ready(Duration::from_secs(60)).unwrap();
    let launches = launcher.launches();
    assert_eq!(launches.len(), squat + 1);
    assert_eq!(port_of(&ep.base_url), launches.last().unwrap().0);
    sup.stop();
}

/// The engine files are hashed before every launch: an engine with nothing pinned, or with a
/// file changed after installation, is never started (AI engine lifecycle).
#[test]
fn an_engine_whose_files_changed_after_install_is_not_started() {
    use openframe_ai::bundle::PinnedFile;
    use openframe_ai::engines::{EngineFiles, hash_file};
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("llama-server.exe"), b"engine").unwrap();
    std::fs::write(dir.path().join("ggml.dll"), b"ggml").unwrap();
    let launcher = |files: Option<EngineFiles>| LlamaServerLauncher {
        executable: dir.path().join("llama-server.exe"),
        engine_files: files,
        model_path: PathBuf::from("C:/m/model.gguf"),
        profile_id: "p".into(),
        context_tokens: 8192,
        threads: 4,
        seed: None,
        model_stamp: None,
        gpu_device: None,
    };
    let rt = Arc::new(AiRuntime::new().unwrap());
    let sup = Supervisor::new(cfg(), rt.handle());
    for (case, files) in [
        ("no record", None),
        (
            "changed dll",
            Some(EngineFiles::pinned(
                dir.path(),
                &["llama-server.exe", "ggml.dll"].map(|n| {
                    let (bytes, sha256) = hash_file(&dir.path().join(n)).unwrap();
                    PinnedFile {
                        path: n.into(),
                        bytes,
                        sha256,
                    }
                }),
            )),
        ),
    ] {
        if case == "changed dll" {
            std::fs::write(dir.path().join("ggml.dll"), b"planted").unwrap();
        }
        sup.configure(Some(Arc::new(launcher(files))));
        let err = sup.start().unwrap_err();
        assert_eq!(err.code_str(), "ai.assets_damaged", "{case}");
        assert_eq!(sup.state(), RuntimeState::Failed, "{case}");
        assert!(
            sup.status()
                .last_error
                .unwrap_or_default()
                .contains("Reinstall OpenFrame Studio"),
            "{case}"
        );
    }
}

/// Engines launched by the supervisor are recorded while they run and forgotten when they stop.
#[cfg(windows)]
#[test]
fn launched_engines_are_recorded_until_they_stop() {
    use openframe_ai::engines::EngineRegistry;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("engines.json");
    let rt = Arc::new(AiRuntime::new().unwrap());
    let sup = Supervisor::new(cfg(), rt.handle());
    sup.set_registry(Arc::new(EngineRegistry::new(path.clone())));
    sup.configure(Some(Arc::new(FakeLauncher)));
    sup.ensure_ready(Duration::from_secs(30)).unwrap();
    let record = std::fs::read_to_string(&path).unwrap();
    assert!(record.contains("\"pid\""), "{record}");
    sup.stop();
    assert!(!path.exists(), "a stopped engine is forgotten");
}

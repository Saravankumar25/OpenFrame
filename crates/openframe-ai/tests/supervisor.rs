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
    LaunchSpec, Launcher, LlamaServerLauncher, RuntimeState, Supervisor, SupervisorConfig,
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
    let listener = TcpListener::bind(("127.0.0.1", port.parse::<u16>().unwrap())).unwrap();
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

    assert_eq!(sup.state(), RuntimeState::NotInstalled);
    assert_eq!(sup.start().unwrap_err().code_str(), "ai.not_installed");
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
        .unwrap();
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
        model_path: PathBuf::from("C:/m/model.gguf"),
        profile_id: "p".into(),
        context_tokens: 8192,
        gpu_layers: 0,
        threads: 4,
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
    use openframe_ai::manifest::Pooling;
    use openframe_ai::supervisor::EmbeddingServerLauncher;
    let l = EmbeddingServerLauncher {
        executable: PathBuf::from("C:/rt/llama-server.exe"),
        model_path: PathBuf::from("C:/m/embedding.gguf"),
        model_id: "bge-small-en-v1.5-q8-0-v1".into(),
        pooling: Pooling::Cls,
        context_tokens: 512,
        threads: 2,
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

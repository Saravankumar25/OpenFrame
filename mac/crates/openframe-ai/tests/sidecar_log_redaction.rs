//! The sidecar log never receives secrets or personal data (log redaction test, Security baseline E7).
//!
//! The real supervisor launches this test binary as its "runtime" (`fake_sidecar_entry`), which
//! prints llama-server-like lines with distinctive markers to stdout and stderr and exits. The
//! supervisor's own output pipe (`pipe_log` in `supervisor.rs`) writes the sanitized copy to
//! `log_path`; the tests read that file. No real runtime or model is involved.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_ai::rt::AiRuntime;
use openframe_ai::supervisor::{LaunchSpec, Launcher, Supervisor, SupervisorConfig};

const ENV_MODE: &str = "OF_FAKE_SIDECAR_LOG";
const SENTINEL: &str = "OF-SIDECAR-SENTINEL-END";

/// Lines a runtime can print that the sanitized log must neutralise today.
fn covered_lines() -> Vec<String> {
    vec![
        r"llama_model_loader: loaded meta data from C:\Users\OfMarkerUser7731\AppData\Local\OpenFrame\models\chat.gguf".into(),
        "srv  log_server_r: request from ofmarker.contact7731@example.com".into(),
        "main: token=OFMARKERTOKEN7731".into(),
        "main: api_key=OFMARKERKEY7731 (from environment)".into(),
        "main: x-api-key OFMARKERKEY7731".into(),
        "Authorization: Bearer OFMARKERBEARER7731".into(),
        "settings: password: OFMARKERPASS7731".into(),
        format!("{}OFMARKERTAIL7731", "x".repeat(420)),
    ]
}
const COVERED_MARKERS: [&str; 8] = [
    "OfMarkerUser7731",
    "ofmarker.contact7731@example.com",
    "OFMARKERTOKEN7731",
    "OFMARKERKEY7731",
    "OFMARKERBEARER7731",
    "OFMARKERPASS7731",
    "OFMARKERTAIL7731",
    "x-api-key",
];

/// Content that a verbose or failing runtime could echo.
fn content_lines() -> Vec<String> {
    vec![
        "slot launch_slot_: prompt: OFMARKERPROMPT7731 He places the red folder".into(),
        r"error: cannot open D:\OFMARKERFOLDER7731\Railway.openframe\cache".into(),
    ]
}
const CONTENT_MARKERS: [&str; 2] = ["OFMARKERPROMPT7731", "OFMARKERFOLDER7731"];

/// Entry point of the fake runtime process (a no-op in normal test runs).
#[test]
fn fake_sidecar_entry() {
    let Ok(mode) = std::env::var(ENV_MODE) else {
        return;
    };
    let mut lines = covered_lines();
    if mode == "content" {
        lines.extend(content_lines());
    }
    for (i, line) in lines.iter().enumerate() {
        if i % 2 == 0 {
            println!("{line}");
        } else {
            eprintln!("{line}");
        }
    }
    println!("{SENTINEL} stdout");
    eprintln!("{SENTINEL} stderr");
    std::process::exit(0);
}

struct FakeRuntime {
    mode: &'static str,
}

impl Launcher for FakeRuntime {
    fn launch_spec(&self, _port: u16, api_key: &str) -> LaunchSpec {
        LaunchSpec {
            program: std::env::current_exe().unwrap(),
            args: vec![
                "fake_sidecar_entry".into(),
                "--exact".into(),
                "--nocapture".into(),
                "--test-threads=1".into(),
            ],
            env: vec![
                (ENV_MODE.into(), self.mode.into()),
                ("LLAMA_API_KEY".into(), api_key.to_string()),
            ],
            cwd: None,
        }
    }
    fn model_reference(&self) -> String {
        "fake-profile".into()
    }
}

/// Run the fake runtime under the real supervisor and return the sanitized log.
fn sidecar_log(mode: &'static str, dir: &Path) -> String {
    let log_path: PathBuf = dir.join("logs").join("ai-runtime.log");
    let cfg = SupervisorConfig {
        start_timeout: Duration::from_secs(30),
        idle_timeout: None,
        poll_interval: Duration::from_millis(100),
        max_restarts: 0,
        restart_window: Duration::from_secs(60),
        log_path: Some(log_path.clone()),
    };
    let rt = Arc::new(AiRuntime::new().unwrap());
    let sup = Supervisor::new(cfg, rt.handle());
    sup.configure(Some(Arc::new(FakeRuntime { mode })));
    sup.start().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut log = String::new();
    while Instant::now() < deadline {
        log = std::fs::read_to_string(&log_path).unwrap_or_default();
        if log.contains(&format!("{SENTINEL} stdout"))
            && log.contains(&format!("{SENTINEL} stderr"))
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    sup.stop();
    assert!(
        log.contains(SENTINEL),
        "the sidecar output reached the log:\n{log}"
    );
    log
}

#[test]
fn sidecar_log_redacts_home_paths_emails_and_keys() {
    let dir = tempfile::tempdir().unwrap();
    let log = sidecar_log("covered", dir.path());
    for marker in COVERED_MARKERS {
        assert!(
            !log.to_lowercase().contains(&marker.to_lowercase()),
            "sidecar log contains {marker}:\n{log}"
        );
    }
    // Positive controls: the lines arrived, redacted rather than dropped wholesale.
    assert!(
        log.contains(r"%USERPROFILE%\AppData\Local\OpenFrame\models"),
        "{log}"
    );
    assert!(log.contains("[email]"), "{log}");
    assert!(log.contains("token=[redacted]"), "{log}");
    assert!(log.contains("password: [redacted]"), "{log}");
    assert!(
        log.lines().all(|l| l.len() <= 400),
        "lines are truncated at 400 bytes"
    );
}

/// Prompt text and project paths that a verbose or failing runtime prints never reach the log.
/// The lines still arrive, with the content replaced by a placeholder.
#[test]
fn sidecar_log_never_contains_prompt_text_or_project_paths() {
    let dir = tempfile::tempdir().unwrap();
    let log = sidecar_log("content", dir.path());
    for marker in CONTENT_MARKERS {
        assert!(
            !log.contains(marker),
            "sidecar log contains {marker}:\n{log}"
        );
    }
    assert!(log.contains("prompt: [redacted]"), "{log}");
    assert!(log.contains("cannot open [path]"), "{log}");
}

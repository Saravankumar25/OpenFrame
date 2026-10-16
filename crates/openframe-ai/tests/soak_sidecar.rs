//! Sidecar soak (build plan item soak tests and failure injection).
//!
//! The managed `llama-server` crashes again and again while requests are in
//! flight and requests are cancelled at random points. The supervisor must restart it every
//! time on a fresh loopback port and key, never leave a process behind, never wedge in Busy,
//! give up with a plain-language error on a crash storm and recover on an explicit start.
//! The fake server is this test binary re-launched with an environment variable (see
//! `fake_llama_server_entry`), as in `tests/supervisor.rs`; no real runtime or model is used.
//!
//! `OPENFRAME_SOAK_SECS` sets the crash-loop duration (short profile 20 s; the ignored long
//! profile 1800 s):
//!
//! ```text
//! cargo test -p openframe-ai --test soak_sidecar
//! OPENFRAME_SOAK_SECS=1800 cargo test -p openframe-ai --test soak_sidecar -- --ignored --nocapture m1_08_sidecar_soak_long
//! ```

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use openframe_ai::client::{self, ChatMessage, ChatRequest, Endpoint};
use openframe_ai::rt::AiRuntime;
use openframe_ai::supervisor::{LaunchSpec, Launcher, RuntimeState, Supervisor, SupervisorConfig};

const ENV_PORT: &str = "OF_SOAK_LLAMA_PORT";
const SECS_ENV: &str = "OPENFRAME_SOAK_SECS";
/// Every wait in the loop is bounded; a hang fails the run.
const BOUND: Duration = Duration::from_secs(30);
/// How long the fake server takes to answer a request marked "slow" (so it can be cancelled).
const SLOW_MS: u64 = 250;
/// Working-set growth allowed over the loop.
const MAX_WS_GROWTH_MB: f64 = 64.0;

// ----------------------------------------------------------------------- fake llama-server

/// Entry point of the fake server process (a no-op in normal test runs).
#[test]
fn fake_llama_server_entry() {
    let Ok(port) = std::env::var(ENV_PORT) else {
        return;
    };
    let key = std::env::var("LLAMA_API_KEY").unwrap_or_default();
    let Ok(listener) = TcpListener::bind(("127.0.0.1", port.parse::<u16>().unwrap())) else {
        eprintln!("couldn't bind HTTP server socket, hostname: 127.0.0.1");
        std::process::exit(1);
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
                if health_calls <= 1 {
                    (
                        "503 Service Unavailable",
                        r#"{"error":{"message":"Loading model"}}"#,
                    )
                } else {
                    ("200 OK", r#"{"status":"ok"}"#)
                }
            }
            "/v1/chat/completions" if auth == format!("Bearer {key}") => {
                if String::from_utf8_lossy(&body).contains("slow") {
                    std::thread::sleep(Duration::from_millis(SLOW_MS));
                }
                (
                    "200 OK",
                    r#"{"choices":[{"message":{"role":"assistant","content":"ready"}}]}"#,
                )
            }
            "/v1/chat/completions" => ("401 Unauthorized", r#"{"error":"bad key"}"#),
            "/crash" => std::process::exit(3),
            _ => ("404 Not Found", "{}"),
        };
        let _ = write!(
            s,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
            payload.len()
        );
    }
}

struct FakeLauncher;

impl Launcher for FakeLauncher {
    fn launch_spec(&self, port: u16, api_key: &str) -> LaunchSpec {
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
    fn model_reference(&self) -> String {
        "soak-profile".into()
    }
}

fn cfg(restart_window: Duration) -> SupervisorConfig {
    SupervisorConfig {
        start_timeout: BOUND,
        idle_timeout: None,
        poll_interval: Duration::from_millis(50),
        max_restarts: 2,
        restart_window,
        log_path: None,
    }
}

// ---------------------------------------------------------------------------------- helpers

fn port_of(ep: &Endpoint) -> u16 {
    ep.base_url.rsplit(':').next().unwrap().parse().unwrap()
}

fn listening(port: u16) -> bool {
    TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(200),
    )
    .is_ok()
}

fn chat(rt: &AiRuntime, sup: &Supervisor, ep: &Endpoint, text: &str) -> String {
    let http = sup.http().clone();
    let ep = ep.clone();
    let req = ChatRequest::new(vec![ChatMessage::user(text)]);
    rt.run(async move { client::chat(&http, &ep, &req).await })
        .map(|(text, _usage)| text)
        .unwrap_or_else(|e| panic!("chat failed: {} {:?}", e.message, e.detail))
}

fn crash(ep: &Endpoint) {
    if let Ok(mut s) = TcpStream::connect(ep.base_url.trim_start_matches("http://")) {
        let _ = write!(s, "GET /crash HTTP/1.1\r\nHost: x\r\n\r\n");
        let mut sink = Vec::new();
        let _ = s.read_to_end(&mut sink);
    }
}

fn wait_until(what: &str, mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + BOUND;
    while !f() {
        assert!(Instant::now() < deadline, "timed out ({BOUND:?}): {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Number of running fake servers this test process started: processes with this test binary's
/// image name whose parent is this process. Counting by image name alone also counted the same
/// test running in another worktree (Cargo gives the binary the same name everywhere), which made
/// the soak fail whenever two checkouts ran it at once.
fn own_image_processes() -> Option<usize> {
    let exe = std::env::current_exe().ok()?;
    let name = exe.file_name()?.to_string_lossy().into_owned();
    let filter = format!(
        "Name='{}' AND ParentProcessId={}",
        name.replace('\'', "''"),
        std::process::id()
    );
    let out = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!("@(Get-CimInstance Win32_Process -Filter \"{filter}\").Count"),
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

fn working_set_mb() -> Option<f64> {
    let out = std::process::Command::new("tasklist")
        .args([
            "/FI",
            &format!("PID eq {}", std::process::id()),
            "/FO",
            "CSV",
            "/NH",
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let field = s.split("\",\"").last()?;
    let digits: String = field.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<f64>().ok().map(|kb| kb / 1024.0)
}

fn secs(default: u64) -> Duration {
    Duration::from_secs(
        std::env::var(SECS_ENV)
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(default),
    )
}

// ------------------------------------------------------------------- soak tests crash/restart loop

fn sidecar_soak(total: Duration) {
    let started = Instant::now();
    let rt = Arc::new(AiRuntime::new().unwrap());
    // A 1 ms window makes every crash an isolated one, so the supervisor must restart each time.
    let sup = Supervisor::new(cfg(Duration::from_millis(1)), rt.handle());
    let launches = Arc::new(AtomicUsize::new(0));
    let l2 = launches.clone();
    sup.set_listener(Arc::new(move |s| {
        if s == RuntimeState::Starting {
            l2.fetch_add(1, Ordering::SeqCst);
        }
    }));
    sup.configure(Some(Arc::new(FakeLauncher)));
    let baseline_procs = own_image_processes();

    let mut ep = sup.ensure_ready(BOUND).expect("first start");
    let mut ports = vec![port_of(&ep)];
    let mut keys = vec![ep.api_key.clone()];
    let (mut rounds, mut crashes, mut cancels, mut stops) = (0u64, 0u64, 0u64, 0u64);
    let mut ws = Vec::new();
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut rand = |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    while started.elapsed() < total || rounds < 8 {
        rounds += 1;
        assert!(
            ep.base_url.starts_with("http://127.0.0.1:"),
            "loopback only"
        );
        assert_eq!(chat(&rt, &sup, &ep, "hello"), "ready");

        // Cancel requests in flight at random points (the user presses Stop).
        for _ in 0..3 {
            let guard = sup.begin_request();
            assert!(matches!(sup.state(), RuntimeState::Busy));
            let http = sup.http().clone();
            let e2 = ep.clone();
            let req = ChatRequest::new(vec![ChatMessage::user("slow request")]);
            let h = rt
                .handle()
                .spawn(async move { client::chat(&http, &e2, &req).await });
            std::thread::sleep(Duration::from_millis(rand(SLOW_MS)));
            h.abort();
            drop(guard);
            cancels += 1;
            assert_eq!(
                sup.state(),
                RuntimeState::Ready,
                "a cancelled request never leaves the runtime Busy"
            );
        }
        // The server still answers after the cancellations.
        assert_eq!(chat(&rt, &sup, &ep, "after cancel"), "ready");

        if rounds % 5 == 0 {
            // Explicit stop and start (Settings → Stop Offline AI, then the next request).
            sup.stop();
            stops += 1;
            assert_eq!(sup.state(), RuntimeState::Installed);
            let old = port_of(&ep);
            wait_until("a stopped sidecar releases its port", || !listening(old));
            ep = sup.ensure_ready(BOUND).expect("start after stop");
        } else {
            // Crash in the middle of serving (with a request marked in flight half the time).
            let guard = (rand(2) == 0).then(|| sup.begin_request());
            let before = launches.load(Ordering::SeqCst);
            let old = ep.clone();
            crash(&old);
            crashes += 1;
            wait_until("the supervisor relaunches the crashed sidecar", || {
                launches.load(Ordering::SeqCst) > before
            });
            drop(guard);
            ep = sup
                .wait_ready(BOUND)
                .expect("ready after an automatic restart");
            wait_until("the crashed sidecar's port is closed", || {
                !listening(port_of(&old))
            });
        }
        assert!(!keys.contains(&ep.api_key), "each launch gets a new key");
        keys.push(ep.api_key.clone());
        ports.push(port_of(&ep));
        if rounds % 10 == 1
            && let Some(mb) = working_set_mb()
        {
            ws.push(mb);
        }
    }
    sup.stop();
    assert_eq!(sup.state(), RuntimeState::Installed);
    for p in &ports {
        assert!(
            !listening(*p),
            "port {p} still answers after stop: a sidecar was left behind"
        );
    }
    if let (Some(base), Some(now)) = (baseline_procs, own_image_processes()) {
        assert_eq!(now, base, "no fake sidecar process survives the loop");
    }
    if ws.len() >= 3 {
        let (first, last) = (ws[1], *ws.last().unwrap());
        assert!(
            last - first < MAX_WS_GROWTH_MB,
            "working set grew from {first:.0} MB to {last:.0} MB: {ws:?}"
        );
    }

    // Crash storm: with the default window, the third crash in a row is not restarted
    // automatically. The runtime reports Failed with a plain-language reason, and an explicit
    // start recovers it.
    let storm = Supervisor::new(cfg(Duration::from_secs(600)), rt.handle());
    storm.configure(Some(Arc::new(FakeLauncher)));
    let mut ep = storm.ensure_ready(BOUND).unwrap();
    for i in 0..3 {
        crash(&ep);
        if i < 2 {
            wait_until("automatic restart within the crash budget", || {
                matches!(storm.state(), RuntimeState::Ready)
                    && storm
                        .wait_ready(Duration::from_millis(10))
                        .is_ok_and(|e| e.api_key != ep.api_key)
            });
            ep = storm.wait_ready(BOUND).unwrap();
        }
    }
    wait_until("a crash storm ends in Failed", || {
        storm.state() == RuntimeState::Failed
    });
    let status = storm.status();
    let reason = status.last_error.expect("a reason for Failed");
    assert!(
        reason.contains("Your project was not changed"),
        "plain-language reason: {reason}"
    );
    let e = storm.wait_ready(Duration::from_millis(10)).unwrap_err();
    assert!(e.code_str().starts_with("ai."), "{}", e.code_str());
    let ep = storm
        .ensure_ready(BOUND)
        .expect("explicit start recovers after a crash storm");
    assert_eq!(chat(&rt, &storm, &ep, "back"), "ready");
    storm.stop();

    println!(
        "soak tests sidecar soak: {:.0} s, {rounds} rounds, {crashes} crashes with automatic restart, \
         {stops} explicit stop/start, {cancels} cancelled requests, {} launches, working set samples {ws:?} MB",
        started.elapsed().as_secs_f64(),
        launches.load(Ordering::SeqCst)
    );
}

#[test]
fn m1_08_sidecar_crash_restart_and_cancel_loop() {
    sidecar_soak(secs(20));
}

#[test]
#[ignore = "long soak: OPENFRAME_SOAK_SECS=1800 cargo test -p openframe-ai --test soak_sidecar -- --ignored --nocapture m1_08_sidecar_soak_long"]
fn m1_08_sidecar_soak_long() {
    sidecar_soak(secs(1800));
}

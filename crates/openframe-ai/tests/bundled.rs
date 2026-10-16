//! The Offline AI end to end (ADR-0026): nothing is downloaded until the person starts it; the
//! download fetches the pinned files from a local test host at their Hugging Face paths, resumes
//! after a cut,
//! refuses and sets aside a damaged file, keeps the last good set until the new one passes its
//! health check, and Remove takes it all away. Once a set is installed, the pinned hashes are
//! checked before the first engine start, writable data stays in app data, and a missing, altered
//! or linked file switches AI off with a plain message and never starts an engine.
//!
//! No real model is involved: the "engine" pinned in the test lock is THIS test executable.
//! `harness = false` lets `main` recognise when it is launched as `llama-server` (it gets
//! `--host 127.0.0.1 --port N …`) and then behave like a tiny llama.cpp server: `/health`
//! (503 while "loading", then 200), `/v1/chat/completions` and, with `--embedding`,
//! `/v1/embeddings`, all requiring the per-launch API key.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use openframe_ai::bundle::PinnedFile;
use openframe_ai::download::pins;
use openframe_ai::engines::hash_file;
use openframe_ai::gpu::{Adapter, GraphicsRecord, Remembered};
use openframe_ai::placement::GraphicsFacts;
use openframe_ai::supervisor::SupervisorConfig;
use openframe_ai::{
    AiManager, AssetLock, BundleState, ChatMessage, ChatRequest, DownloadPhase, ManagerConfig,
    ModelSource, RuntimeState,
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
    // A model that cannot load (the failed health check of a new download).
    if arg("-m")
        .and_then(|m| std::fs::read(m).ok())
        .is_some_and(|b| b.starts_with(b"BROKEN"))
    {
        std::process::exit(4);
    }
    let embedding = args.iter().any(|a| a == "--embedding");
    let device = arg("--device");
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
                let probe = req["response_format"].is_object();
                // The fake cards (see FAKE_DEVICES): Vulkan2 stops on any request but the probe,
                // Vulkan3 answers the probe with text that is not the schema's object.
                if device.as_deref() == Some("Vulkan2") && !probe {
                    std::process::exit(6);
                }
                let content = if probe && device.as_deref() != Some("Vulkan3") {
                    r#"{"text":"one two three"}"#.to_string()
                } else {
                    // Echo the sampling temperature so the deterministic setting is observable.
                    format!("ready t={}", req["temperature"])
                };
                // 20 tokens: the processor at 20 a second, Vulkan1 at 4, the other cards at 200.
                let ms = match device.as_deref() {
                    None => 1000,
                    Some("Vulkan1") => 5000,
                    Some(_) => 100,
                };
                let out = serde_json::json!({
                    "choices": [{ "message": { "role": "assistant", "content": content } }],
                    "usage": { "prompt_tokens": 30, "completion_tokens": 20 },
                    "timings": { "prompt_ms": 10, "predicted_ms": ms }
                });
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

// ------------------------------------------------------------------- local model host

/// Requests the host saw: the path and the range start it asked for.
type Requests = Arc<Mutex<Vec<(String, Option<u64>)>>>;

/// A tiny model host on loopback: serves the files under `root`, honours `Range: bytes=N-`, and
/// can cut one answer short (an interrupted download).
struct Host {
    base: String,
    root: PathBuf,
    /// Every request: path and the range it asked for.
    requests: Requests,
    /// The next answer for this path stops after this many body bytes.
    cut_once: Arc<Mutex<Option<(String, usize)>>>,
}

impl Host {
    fn start(root: PathBuf) -> Host {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let base = format!(
            "http://127.0.0.1:{}/",
            listener.local_addr().unwrap().port()
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let cut_once: Arc<Mutex<Option<(String, usize)>>> = Arc::new(Mutex::new(None));
        let (dir, reqs, cut) = (root.clone(), requests.clone(), cut_once.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { continue };
                let mut reader = BufReader::new(s.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap_or(0);
                let mut from = None;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                        from = v.trim().trim_end_matches('-').parse::<u64>().ok();
                    }
                }
                let path = first
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .trim_start_matches('/')
                    .to_string();
                reqs.lock().unwrap().push((path.clone(), from));
                let Ok(body) = std::fs::read(dir.join(&path)) else {
                    let _ = s.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                };
                let start = from.unwrap_or(0).min(body.len() as u64) as usize;
                let part = &body[start..];
                let status = if from.is_some() {
                    format!(
                        "206 Partial Content\r\nContent-Range: bytes {start}-{}/{}",
                        body.len().saturating_sub(1),
                        body.len()
                    )
                } else {
                    "200 OK".to_string()
                };
                let _ = write!(
                    s,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    part.len()
                );
                let mut c = cut.lock().unwrap();
                let send = match c.as_ref() {
                    Some((p, n)) if *p == path => {
                        let n = *n;
                        *c = None;
                        &part[..n.min(part.len())]
                    }
                    _ => part,
                };
                let _ = s.write_all(send);
            }
        });
        Host {
            base,
            root,
            requests,
            cut_once,
        }
    }

    fn requests(&self) -> Vec<(String, Option<u64>)> {
        self.requests.lock().unwrap().clone()
    }
}

/// Model bytes of `len`, different for each `seed`.
fn model_bytes(seed: u32, len: u32) -> Vec<u8> {
    (0..len).map(|k| ((k * 31 + seed) % 251) as u8).collect()
}

/// Pin `chat` and `embed` in `lock` (as a new app version would) at Hugging Face addresses with the
/// commit `rev` (one hex digit repeated), and put the bytes on the local host at those paths.
fn publish(host_root: &Path, lock: &mut AssetLock, rev: char, chat: &[u8], embed: &[u8]) {
    let rev: String = std::iter::repeat_n(rev, 40).collect();
    for (m, body) in lock.models.iter_mut().zip([chat, embed]) {
        let path = format!("test/{0}/resolve/{rev}/{0}.gguf", m.model_id);
        m.source.url = format!("https://huggingface.co/{path}");
        let file = host_root.join(&path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, body).unwrap();
        m.file = pin(&file, "model.gguf");
    }
}

/// The host path of model `i` (0 chat, 1 embedding) as the lock pins it.
fn host_path(lock: &AssetLock, i: usize) -> String {
    lock.models[i]
        .source
        .url
        .strip_prefix("https://huggingface.co/")
        .unwrap()
        .to_string()
}

// ----------------------------------------------------------------------- pinned files

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

// --------------------------------------------------------------------------- fixture

struct Fixture {
    _tmp: tempfile::TempDir,
    bundle: PathBuf,
    data: PathBuf,
    lock: AssetLock,
    host: Host,
}

impl Fixture {
    /// The engine laid out like the app resources (this test program as the server, a library next
    /// to it), the two models pinned in the lock and served by the local host at their Hugging Face
    /// paths, and installed in app data the way a finished download leaves them.
    fn new() -> Self {
        let f = Self::without_models();
        f.install(&f.lock);
        f
    }

    /// Install the set `lock` pins from the host, as a finished download leaves it.
    fn install(&self, lock: &AssetLock) {
        let (version, tier) = pins(lock);
        let set = self.data.join("models").join(&version);
        std::fs::create_dir_all(&set).unwrap();
        for (i, f) in tier.files.iter().enumerate() {
            std::fs::copy(self.host.root.join(host_path(lock, i)), set.join(&f.name)).unwrap();
        }
        std::fs::write(
            self.data.join("models").join("active.json"),
            format!(r#"{{"version":"{version}","tier":"small","previous":null,"activatedAt":0}}"#),
        )
        .unwrap();
    }

    /// The same, before anything was downloaded.
    fn without_models() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("resources").join("ai");
        let data = tmp.path().join("appdata");
        let mut lock = AssetLock::embedded().unwrap();
        let mut rt = lock
            .runtimes
            .iter()
            .find(|r| r.backend == "cpu")
            .unwrap()
            .clone();
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
        // The Vulkan build beside it: the same fake program (it reads --device).
        let mut vk = rt.clone();
        vk.runtime_id = format!("{}-vulkan", rt.runtime_id);
        vk.backend = "vulkan".into();
        let vk_dir = bundle.join("runtimes").join(&vk.runtime_id);
        std::fs::create_dir_all(&vk_dir).unwrap();
        std::fs::copy(std::env::current_exe().unwrap(), vk_dir.join(SERVER)).unwrap();
        std::fs::write(vk_dir.join("ggml.dll"), b"ggml vulkan library").unwrap();
        vk.files = vec![
            pin(&vk_dir.join(SERVER), SERVER),
            pin(&vk_dir.join("ggml.dll"), "ggml.dll"),
        ];
        lock.runtimes = vec![rt, vk];
        for m in lock.models.iter_mut() {
            if m.embedding_dim.is_some() {
                m.embedding_dim = Some(8);
            }
        }
        let host_root = tmp.path().join("host");
        publish(
            &host_root,
            &mut lock,
            'a',
            &model_bytes(0, 40_000),
            &model_bytes(1, 40_000),
        );
        lock.validate().unwrap();
        Fixture {
            _tmp: tmp,
            bundle,
            data,
            lock,
            host: Host::start(host_root),
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
            model_source: Some(ModelSource::test_host(&self.host.base).unwrap()),
            lock: self.lock.clone(),
            supervisor: sup.clone(),
            embedding_supervisor: sup,
            ready_timeout: Duration::from_secs(30),
            deterministic: false,
            // No graphics card unless a scenario gives one (with_card).
            graphics: Arc::new(GraphicsFacts::default),
        }
    }

    /// The configuration of a computer with one dedicated card of this name and a working driver.
    fn with_card(&self, name: &str) -> ManagerConfig {
        let mut cfg = self.config();
        let name = name.to_string();
        cfg.graphics = Arc::new(move || GraphicsFacts {
            adapters: vec![(
                Adapter {
                    name: name.clone(),
                    vendor_id: 0x10DE,
                    dedicated_mb: 8192,
                    shared_mb: 8000,
                    software: false,
                },
                "31.0.15.5222".into(),
            )],
            vulkan_loader: true,
        });
        cfg
    }

    fn graphics(&self) -> GraphicsRecord {
        serde_json::from_slice(&std::fs::read(self.data.join("ai").join("graphics.json")).unwrap())
            .unwrap()
    }

    fn manager(&self) -> Arc<AiManager> {
        Arc::new(AiManager::new(self.config()).unwrap())
    }

    /// 0: the chat model, 1: the embedding model of the installed set.
    fn model_file(&self, role: usize) -> PathBuf {
        let active: serde_json::Value = serde_json::from_slice(
            &std::fs::read(self.data.join("models").join("active.json")).unwrap(),
        )
        .unwrap();
        self.data
            .join("models")
            .join(active["version"].as_str().unwrap())
            .join(format!("{}.gguf", self.lock.models[role].model_id))
    }

    /// The pinned set version of this app.
    fn version(&self) -> String {
        pins(&self.lock).0
    }

    /// Every file under the app resources with its size and modification time.
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

/// Start the download and wait until it ends; returns the final phase.
fn download(m: &Arc<AiManager>) -> DownloadPhase {
    m.start_download().unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        let st = m.download_status();
        // `start_download` set Checking before it returned, so an end phase is this run's.
        if matches!(
            st.phase,
            DownloadPhase::Done | DownloadPhase::Failed | DownloadPhase::Paused
        ) {
            return st.phase;
        }
        assert!(Instant::now() < until, "the download never ended: {st:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn files_under(d: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(d)
        .map(|r| {
            r.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

// --------------------------------------------------------------- scenarios: download

fn nothing_is_downloaded_until_the_person_starts_it() {
    let f = Fixture::without_models();
    let m = f.manager();
    let st = m.state();
    assert!(
        !st.available && st.bundle == BundleState::NotInstalled,
        "{st:?}"
    );
    let e = st.problem.unwrap();
    assert_eq!(e.code_str(), "ai.not_installed");
    assert!(
        e.message.contains("Download it in Settings"),
        "{}",
        e.message
    );
    assert!(m.chat_model().is_none() && m.embedding_info().is_none());
    assert_eq!(ask(&m).unwrap_err().code_str(), "ai.not_installed");
    assert!(m.warm_up().is_err());
    assert_eq!(m.download_status().phase, DownloadPhase::Idle);
    assert!(
        m.download_size() > 0,
        "the size is known before any network use"
    );
    assert!(f.host.requests().is_empty(), "no request before the click");
}

fn the_download_installs_checks_and_starts_the_ai() {
    let f = Fixture::without_models();
    let m = f.manager();
    assert_eq!(
        download(&m),
        DownloadPhase::Done,
        "{:?}",
        m.download_status()
    );
    let st = m.download_status();
    assert_eq!((st.done_bytes, st.total_bytes), (80_000, 80_000));
    // The health check started the engine on the new set; AI answers.
    assert!(m.state().available);
    assert!(ask(&m).unwrap().starts_with("ready"));
    assert_eq!(m.embed(&["Ravi".into()]).unwrap()[0].len(), 8);
    assert_eq!(
        m.embedding_info().unwrap().sha256,
        hash_file(&f.model_file(1)).unwrap().1
    );
    // Exactly the two pinned files were asked for, nothing else.
    let paths: Vec<String> = f.host.requests().into_iter().map(|r| r.0).collect();
    assert_eq!(paths, [host_path(&f.lock, 0), host_path(&f.lock, 1)]);
    // Only the set, its pointer and an empty staging area are left.
    let mut names: Vec<String> = f
        .lock
        .models
        .iter()
        .map(|m| format!("{}.gguf", m.model_id))
        .collect();
    names.sort();
    assert_eq!(files_under(&f.data.join("models").join(f.version())), names);
    assert!(files_under(&f.data.join("models").join("staging")).is_empty());
    // A second click with the set in use asks for nothing at all.
    assert_eq!(download(&m), DownloadPhase::Done);
    assert_eq!(f.host.requests().len(), 2);
    // A new start of the app finds the set without any network use.
    m.stop_runtime();
    drop(m);
    let m = f.manager();
    assert!(ask(&m).unwrap().starts_with("ready"));
    assert_eq!(f.host.requests().len(), 2);
}

fn an_interrupted_download_resumes_where_it_stopped() {
    let f = Fixture::without_models();
    let chat = host_path(&f.lock, 0);
    *f.host.cut_once.lock().unwrap() = Some((chat.clone(), 15_000));
    let m = f.manager();
    assert_eq!(download(&m), DownloadPhase::Failed);
    let e = m.download_status().problem.unwrap();
    assert_eq!(e.code_str(), "ai.download_failed");
    assert!(e.message.contains("resume"), "{}", e.message);
    assert_eq!(m.state().bundle, BundleState::NotInstalled);
    let partial = f
        .data
        .join("models")
        .join("staging")
        .join(f.version())
        .join(format!("{}.gguf.partial", f.lock.models[0].model_id));
    assert_eq!(std::fs::metadata(&partial).unwrap().len(), 15_000);
    // The next click asks only for the rest of the file.
    assert_eq!(
        download(&m),
        DownloadPhase::Done,
        "{:?}",
        m.download_status()
    );
    assert!(f.host.requests().contains(&(chat.clone(), Some(15_000))));
    assert!(ask(&m).unwrap().starts_with("ready"));
    assert_eq!(
        hash_file(&f.model_file(0)).unwrap().1,
        hash_file(&f.host.root.join(&chat)).unwrap().1
    );
}

fn a_damaged_file_is_refused_set_aside_and_never_used() {
    let f = Fixture::without_models();
    // The host now serves other bytes of the same size than the app pins.
    let embed = f.host.root.join(host_path(&f.lock, 1));
    alter(&embed);
    let m = f.manager();
    assert_eq!(download(&m), DownloadPhase::Failed);
    let e = m.download_status().problem.unwrap();
    assert_eq!(e.code_str(), "ai.file_damaged");
    assert!(e.message.contains("set aside"), "{}", e.message);
    assert_eq!(
        files_under(&f.data.join("models").join("quarantine")).len(),
        1
    );
    assert!(!f.data.join("models").join("active.json").exists());
    assert_eq!(m.state().bundle, BundleState::NotInstalled);
    assert!(m.chat_model().is_none());
    // The host is repaired: the next download completes (the chat file was kept and checked).
    alter(&embed);
    assert_eq!(
        download(&m),
        DownloadPhase::Done,
        "{:?}",
        m.download_status()
    );
    assert!(ask(&m).is_ok());
}

fn a_file_gone_from_the_host_stops_plainly_and_manual_import_still_works() {
    let f = Fixture::without_models();
    // Keep the person's copies, then the host loses the files.
    let copies = f.data.join("my-copies");
    std::fs::create_dir_all(&copies).unwrap();
    let mut mine = Vec::new();
    for i in 0..2 {
        let src = f.host.root.join(host_path(&f.lock, i));
        let dst = copies.join(format!("copy-{i}.gguf"));
        std::fs::copy(&src, &dst).unwrap();
        std::fs::remove_file(&src).unwrap();
        mine.push(dst);
    }
    let m = f.manager();
    assert_eq!(download(&m), DownloadPhase::Failed);
    let e = m.download_status().problem.unwrap();
    assert_eq!(e.code_str(), "ai.download_failed", "{e:?}");
    // A file that is not a pinned model: refused, left where it was.
    let other = copies.join("other.gguf");
    std::fs::write(&other, b"not a model").unwrap();
    assert_eq!(
        m.import_model_file(&other).unwrap_err().code_str(),
        "ai.not_a_model"
    );
    assert!(other.is_file());
    // The two real files: one at a time, then OpenFrame AI is complete and answers.
    assert!(!m.import_model_file(&mine[0]).unwrap(), "one of two");
    assert!(m.import_model_file(&mine[1]).unwrap(), "complete");
    assert!(mine.iter().all(|p| p.is_file()), "copied, never moved");
    assert!(ask(&m).unwrap().starts_with("ready"));
    // Importing again while the set is whole replaces nothing and leaves no copy behind.
    let before = std::fs::read_to_string(f.data.join("models").join("active.json")).unwrap();
    assert!(m.import_model_file(&mine[0]).unwrap(), "already complete");
    assert!(files_under(&f.data.join("models").join("staging")).is_empty());
    assert_eq!(
        std::fs::read_to_string(f.data.join("models").join("active.json")).unwrap(),
        before
    );
    assert!(ask(&m).unwrap().starts_with("ready"));
}

fn a_new_set_that_fails_its_health_check_falls_back_to_the_last_good_one() {
    let f = Fixture::new();
    let old = f.version();
    assert!(ask(&f.manager()).is_ok());
    // An app update pins a new chat model that cannot load.
    let mut broken = b"BROKEN".to_vec();
    broken.extend(model_bytes(5, 40_000));
    let mut update = f.lock.clone();
    publish(
        &f.host.root,
        &mut update,
        'b',
        &broken,
        &model_bytes(6, 40_000),
    );
    let new = pins(&update).0;
    let mut cfg = f.config();
    cfg.lock = update;
    let m = Arc::new(AiManager::new(cfg).unwrap());
    assert_eq!(
        m.state().bundle,
        BundleState::NotInstalled,
        "other pins: download again"
    );
    assert_eq!(download(&m), DownloadPhase::Failed);
    // The last good set is the active one again, whole; the bad set is set aside.
    let active = std::fs::read_to_string(f.data.join("models").join("active.json")).unwrap();
    assert!(active.contains(&old) && !active.contains(&new), "{active}");
    assert!(!f.data.join("models").join(&new).exists());
    assert!(ask(&f.manager()).unwrap().starts_with("ready"));
    // A good update replaces it, and the old set is removed.
    let mut good = f.lock.clone();
    publish(
        &f.host.root,
        &mut good,
        'c',
        &model_bytes(7, 40_000),
        &model_bytes(8, 40_000),
    );
    let mut cfg = f.config();
    cfg.lock = good;
    let m = Arc::new(AiManager::new(cfg).unwrap());
    assert_eq!(
        download(&m),
        DownloadPhase::Done,
        "{:?}",
        m.download_status()
    );
    assert!(!f.data.join("models").join(&old).exists());
    assert!(ask(&m).is_ok());
}

fn a_damaged_set_of_the_same_version_is_removed_before_it_is_downloaded_again() {
    let f = Fixture::new();
    alter(&f.model_file(0));
    let m = f.manager();
    assert_eq!(ask(&m).unwrap_err().code_str(), "ai.models_damaged");
    // Download again without Remove: refused at once, before any file is fetched.
    assert_eq!(download(&m), DownloadPhase::Failed);
    assert_eq!(
        m.download_status().problem.unwrap().code_str(),
        "ai.remove_first"
    );
    assert!(f.host.requests().is_empty(), "nothing was fetched");
    // Remove, then download: whole again.
    m.remove_models().unwrap();
    assert_eq!(
        download(&m),
        DownloadPhase::Done,
        "{:?}",
        m.download_status()
    );
    assert!(ask(&m).is_ok());
}

fn remove_turns_the_ai_off_and_frees_the_space() {
    let f = Fixture::new();
    let m = f.manager();
    assert!(ask(&m).is_ok());
    m.remove_models().unwrap();
    assert_eq!(m.state().bundle, BundleState::NotInstalled);
    assert_eq!(m.runtime_status().state, RuntimeState::Unavailable);
    assert!(m.chat_model().is_none());
    assert!(files_under(&f.data.join("models")).is_empty());
    // The engine in the app resources is untouched; downloading again works.
    assert_eq!(download(&m), DownloadPhase::Done);
    assert!(ask(&m).is_ok());
}

fn a_build_that_cannot_download_still_uses_an_installed_set() {
    let f = Fixture::new();
    let mut cfg = f.config();
    cfg.model_source = None;
    let m = Arc::new(AiManager::new(cfg).unwrap());
    assert!(!m.can_download());
    assert_eq!(
        m.start_download().unwrap_err().code_str(),
        "ai.download_unavailable"
    );
    // The pins are in the app, so the set on disk is checked and used without any network.
    assert!(ask(&m).unwrap().starts_with("ready"));
    assert!(f.host.requests().is_empty());
}

// ---------------------------------------------------------------- scenarios: graphics card

fn a_download_checked_while_the_app_warms_up_keeps_the_new_set() {
    // Found by the release run of 2026-10-16: on a computer with a card, the screen warms the AI
    // up while the new set is in its health check; the graphics card check then restarts the
    // engine, and the health check took that for a broken set and threw the download away.
    let f = Fixture::without_models();
    let m = Arc::new(AiManager::new(f.with_card("Fast Card")).unwrap());
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let warm = {
        let (m, stop) = (m.clone(), stop.clone());
        std::thread::spawn(move || {
            while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                let _ = m.warm_up();
                std::thread::sleep(Duration::from_millis(5));
            }
        })
    };
    let phase = download(&m);
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    warm.join().unwrap();
    assert_eq!(phase, DownloadPhase::Done, "{:?}", m.download_status());
    assert!(files_under(&f.data.join("models").join("quarantine")).is_empty());
    assert!(ask(&m).unwrap().starts_with("ready"));
}

fn a_card_that_passes_its_check_runs_the_chat_engine_and_is_remembered() {
    let f = Fixture::new();
    let m = Arc::new(AiManager::new(f.with_card("Fast Card")).unwrap());
    assert!(ask(&m).unwrap().starts_with("ready"));
    let text = m.diagnostics().placement;
    assert!(text.contains("graphics card (Fast Card)"), "{text}");
    let g = f.graphics();
    assert_eq!(g.probes.len(), 1);
    let p = &g.probes[0];
    assert_eq!(p.result, Remembered::Passed);
    assert!(p.gpu_tokens_per_second.unwrap() > p.cpu_tokens_per_second.unwrap());
    assert_eq!(p.driver, "31.0.15.5222");
    // The next session uses the card without probing again.
    m.stop_runtime();
    drop(m);
    let m = Arc::new(AiManager::new(f.with_card("Fast Card")).unwrap());
    assert!(ask(&m).is_ok());
    assert_eq!(f.graphics().probes, g.probes, "no second probe");
    assert!(m.diagnostics().placement.contains("Fast Card"));
}

fn a_card_slower_than_the_processor_or_with_bad_output_is_not_used() {
    // A card the engine does not list: the processor this session, and nothing remembered (the
    // check could not run, so it runs again next time).
    let f = Fixture::new();
    let m = Arc::new(AiManager::new(f.with_card("Ghost Card")).unwrap());
    assert!(ask(&m).is_ok());
    assert!(m.diagnostics().placement.contains("Runs on the processor"));
    assert!(!f.data.join("ai").join("graphics.json").exists());
    for card in ["Slow Card", "Bad Card"] {
        let f = Fixture::new();
        let m = Arc::new(AiManager::new(f.with_card(card)).unwrap());
        assert!(ask(&m).unwrap().starts_with("ready"), "{card}");
        let text = m.diagnostics().placement;
        assert!(text.contains("Runs on the processor"), "{card}: {text}");
        assert!(text.contains("did not pass its check"), "{card}: {text}");
        assert_eq!(f.graphics().probes[0].result, Remembered::Failed, "{card}");
        // Remembered: the next session goes straight to the processor.
        m.stop_runtime();
        drop(m);
        let m = Arc::new(AiManager::new(f.with_card(card)).unwrap());
        assert!(ask(&m).is_ok());
        assert_eq!(f.graphics().probes.len(), 1, "{card}: no second probe");
    }
}

fn the_setting_off_keeps_the_processor_and_runs_no_check() {
    let f = Fixture::new();
    let m = Arc::new(AiManager::new(f.with_card("Fast Card")).unwrap());
    m.set_graphics_off(true).unwrap();
    assert!(m.graphics_off());
    assert!(ask(&m).is_ok());
    assert!(
        m.diagnostics()
            .placement
            .contains("switched off in Settings")
    );
    assert!(f.graphics().probes.is_empty(), "no probe while Off");
    // Back to Automatic: the next request checks the card and uses it.
    m.set_graphics_off(false).unwrap();
    assert!(ask(&m).is_ok());
    assert!(m.diagnostics().placement.contains("Fast Card"));
}

fn a_card_engine_that_stops_in_use_falls_back_to_the_processor() {
    let f = Fixture::new();
    let m = Arc::new(AiManager::new(f.with_card("Crash Card")).unwrap());
    // The probe passes; the first real request stops the engine on the card. The answer still
    // comes, from the processor, and the card is remembered as failed.
    assert!(ask(&m).unwrap().starts_with("ready"));
    assert!(m.diagnostics().placement.contains("Runs on the processor"));
    assert_eq!(f.graphics().probes[0].result, Remembered::Failed);
    assert!(ask(&m).is_ok());
}

fn a_computer_without_a_card_or_driver_runs_on_the_processor_without_a_check() {
    let f = Fixture::new();
    let m = f.manager();
    assert!(ask(&m).is_ok());
    assert!(
        m.diagnostics()
            .placement
            .contains("No graphics card was found")
    );
    assert!(!f.data.join("ai").join("graphics.json").exists());
    let mut cfg = f.with_card("Fast Card");
    let card = (cfg.graphics)();
    cfg.graphics = Arc::new(move || GraphicsFacts {
        vulkan_loader: false,
        ..card.clone()
    });
    let m = Arc::new(AiManager::new(cfg).unwrap());
    assert!(ask(&m).is_ok());
    assert!(m.diagnostics().placement.contains("graphics driver"));
}

// ------------------------------------------------------------- scenarios: installed set

fn ai_works_once_the_models_are_downloaded() {
    let f = Fixture::new();
    let before = f.snapshot();
    let m = f.manager();
    let st = m.state();
    assert!(
        st.available && st.bundle == BundleState::NotChecked,
        "{st:?}"
    );
    // First request: full check, engine start, answer. No network use.
    assert!(ask(&m).unwrap().starts_with("ready t=0.2"));
    assert_eq!(m.state().bundle, BundleState::Verified);
    let v = m.embed(&["Ravi waits at the station".into()]).unwrap();
    assert_eq!(v[0].len(), 8);
    assert!(
        (v[0].iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-4,
        "normalised"
    );
    // Writable data lands in app data; the read-only resources are untouched.
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
    assert!(f.host.requests().is_empty());
    // Diagnostics describe the engine and the downloaded models, their licences and sizes.
    let d = m.diagnostics();
    assert_eq!(d.components.len(), 3);
    assert!(
        d.components
            .iter()
            .any(|c| c.license_id == "Gemma-Terms-of-Use")
    );
    assert_eq!(d.components[1].bytes, 40_000);
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
    assert_eq!(e.code_str(), "ai.models_damaged");
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
        "ai.models_damaged"
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
    assert_eq!(e.code_str(), "ai.models_damaged");
    assert!(e.message.contains("download it again"), "{}", e.message);
    assert!(!e.message.contains("Reinstall"));
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
    assert!(!st.available && st.bundle == BundleState::Damaged, "{st:?}");
    let e = st.problem.unwrap();
    assert_eq!(e.code_str(), "ai.models_damaged");
    assert!(!e.message.contains('\u{2014}'));
    assert!(m.chat_model().is_none());
    assert!(m.embedding_info().is_none());
    assert_eq!(ask(&m).unwrap_err().code_str(), "ai.models_damaged");
    // A missing engine says reinstall.
    let mut cfg = f.config();
    cfg.bundle_dir = None;
    let m = AiManager::new(cfg).unwrap();
    assert_eq!(m.state().bundle, BundleState::Missing);
    assert!(
        m.state()
            .problem
            .unwrap()
            .message
            .contains("Reinstall OpenFrame Studio")
    );
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

fn a_model_cannot_be_changed_while_openframe_uses_it() {
    let f = Fixture::new();
    let m = f.manager();
    assert!(ask(&m).is_ok());
    // The engine is unloaded (inactivity or Stop Offline AI); another program tries to change or
    // delete the model meanwhile.
    m.stop_runtime();
    std::thread::sleep(Duration::from_millis(20));
    let path = f.model_file(0);
    if cfg!(windows) {
        // Held open for reading only since its check: writing, renaming and deleting are refused.
        assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
        assert!(std::fs::remove_file(&path).is_err());
        assert!(std::fs::rename(&path, path.with_extension("moved")).is_err());
        assert!(ask(&m).unwrap().starts_with("ready"));
    } else {
        // Without the share mode, the check before the next launch catches it.
        alter(&path);
        assert_eq!(ask(&m).unwrap_err().code_str(), "ai.models_damaged");
        assert_eq!(m.state().bundle, BundleState::Damaged);
    }
    // Removing OpenFrame AI lets go of the files first.
    m.remove_models().unwrap();
    assert!(!path.exists());
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
fn a_junction_inside_the_models_folder_is_refused() {
    let f = Fixture::new();
    // Move the set elsewhere and put a junction in its place: same files, but the path now
    // leaves the models folder.
    let set = f.data.join("models").join(f.version());
    let elsewhere = f.data.join("elsewhere");
    std::fs::rename(&set, &elsewhere).unwrap();
    let ok = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&set)
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
    std::fs::remove_dir(&set).unwrap();
}

#[cfg(not(windows))]
fn a_junction_inside_the_models_folder_is_refused() {}

// ------------------------------------------------------------------------------ main

/// What the fake Vulkan engine lists: a fast card, a slow card, a card whose engine stops in use,
/// and a card that answers the probe badly.
const FAKE_DEVICES: &str = "Available devices:
  Vulkan0: Fast Card (8192 MiB, 8000 MiB free)
  Vulkan1: Slow Card (8192 MiB, 8000 MiB free)
  Vulkan2: Crash Card (8192 MiB, 8000 MiB free)
  Vulkan3: Bad Card (8192 MiB, 8000 MiB free)
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--list-devices") {
        print!("{FAKE_DEVICES}");
        std::process::exit(0);
    }
    if args.iter().any(|a| a == "--host") {
        fake_server(&args);
    }
    let filter = args.iter().find(|a| !a.starts_with('-')).cloned();
    let tests: &[(&str, fn())] = &[
        (
            "a_download_checked_while_the_app_warms_up_keeps_the_new_set",
            a_download_checked_while_the_app_warms_up_keeps_the_new_set,
        ),
        (
            "a_card_that_passes_its_check_runs_the_chat_engine_and_is_remembered",
            a_card_that_passes_its_check_runs_the_chat_engine_and_is_remembered,
        ),
        (
            "a_card_slower_than_the_processor_or_with_bad_output_is_not_used",
            a_card_slower_than_the_processor_or_with_bad_output_is_not_used,
        ),
        (
            "the_setting_off_keeps_the_processor_and_runs_no_check",
            the_setting_off_keeps_the_processor_and_runs_no_check,
        ),
        (
            "a_card_engine_that_stops_in_use_falls_back_to_the_processor",
            a_card_engine_that_stops_in_use_falls_back_to_the_processor,
        ),
        (
            "a_computer_without_a_card_or_driver_runs_on_the_processor_without_a_check",
            a_computer_without_a_card_or_driver_runs_on_the_processor_without_a_check,
        ),
        (
            "nothing_is_downloaded_until_the_person_starts_it",
            nothing_is_downloaded_until_the_person_starts_it,
        ),
        (
            "the_download_installs_checks_and_starts_the_ai",
            the_download_installs_checks_and_starts_the_ai,
        ),
        (
            "an_interrupted_download_resumes_where_it_stopped",
            an_interrupted_download_resumes_where_it_stopped,
        ),
        (
            "a_damaged_file_is_refused_set_aside_and_never_used",
            a_damaged_file_is_refused_set_aside_and_never_used,
        ),
        (
            "a_file_gone_from_the_host_stops_plainly_and_manual_import_still_works",
            a_file_gone_from_the_host_stops_plainly_and_manual_import_still_works,
        ),
        (
            "a_new_set_that_fails_its_health_check_falls_back_to_the_last_good_one",
            a_new_set_that_fails_its_health_check_falls_back_to_the_last_good_one,
        ),
        (
            "a_damaged_set_of_the_same_version_is_removed_before_it_is_downloaded_again",
            a_damaged_set_of_the_same_version_is_removed_before_it_is_downloaded_again,
        ),
        (
            "remove_turns_the_ai_off_and_frees_the_space",
            remove_turns_the_ai_off_and_frees_the_space,
        ),
        (
            "a_build_that_cannot_download_still_uses_an_installed_set",
            a_build_that_cannot_download_still_uses_an_installed_set,
        ),
        (
            "ai_works_once_the_models_are_downloaded",
            ai_works_once_the_models_are_downloaded,
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
            "a_model_cannot_be_changed_while_openframe_uses_it",
            a_model_cannot_be_changed_while_openframe_uses_it,
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
            "a_junction_inside_the_models_folder_is_refused",
            a_junction_inside_the_models_folder_is_refused,
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

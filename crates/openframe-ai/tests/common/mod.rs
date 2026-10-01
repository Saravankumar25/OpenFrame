//! Test fixtures: a tiny local HTTP/1.1 file server (tokio) with optional
//! range support and fault injection, plus manifest signing with a throwaway key.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use base64::Engine;
use ed25519_dalek::Signer;
use parking_lot::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Default)]
pub struct ServerState {
    pub files: Mutex<HashMap<String, Vec<u8>>>,
    /// Range headers received, in order.
    pub ranges: Mutex<Vec<String>>,
    /// Drop the connection after this many body bytes on the next response.
    pub fail_after: Mutex<Option<usize>>,
    pub no_range_support: AtomicBool,
    pub requests: AtomicUsize,
    /// Pause this long after every 16 KB sent (simulates a slow connection).
    pub throttle: Mutex<Option<std::time::Duration>>,
}

pub struct TestServer {
    pub base: String,
    pub state: Arc<ServerState>,
    _rt: tokio::runtime::Runtime,
}

impl TestServer {
    pub fn start() -> TestServer {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let state = Arc::new(ServerState::default());
        let listener = rt
            .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let st = state.clone();
        rt.spawn(async move {
            loop {
                let Ok((sock, _)) = listener.accept().await else {
                    break;
                };
                let st = st.clone();
                tokio::spawn(async move {
                    let _ = serve(sock, st).await;
                });
            }
        });
        TestServer {
            base: format!("http://127.0.0.1:{port}"),
            state,
            _rt: rt,
        }
    }

    pub fn put(&self, path: &str, bytes: Vec<u8>) {
        self.state.files.lock().insert(path.to_string(), bytes);
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }
}

async fn serve(mut sock: tokio::net::TcpStream, st: Arc<ServerState>) -> std::io::Result<()> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = sock.read(&mut tmp).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    st.requests.fetch_add(1, Ordering::SeqCst);
    let head = String::from_utf8_lossy(&buf).to_string();
    let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
    let range = head
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("range:"))
        .map(|l| l.split_once(':').unwrap().1.trim().to_string());
    let Some(body) = st.files.lock().get(&path).cloned() else {
        sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await?;
        return Ok(());
    };
    let total = body.len();
    let mut start = 0usize;
    let mut status = "200 OK";
    let mut extra = String::new();
    if let Some(r) = &range {
        st.ranges.lock().push(r.clone());
        if !st.no_range_support.load(Ordering::SeqCst) {
            let from: usize = r
                .trim_start_matches("bytes=")
                .trim_end_matches('-')
                .parse()
                .unwrap_or(0);
            if from >= total {
                sock.write_all(format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await?;
                return Ok(());
            }
            start = from;
            status = "206 Partial Content";
            extra = format!("Content-Range: bytes {start}-{}/{total}\r\n", total - 1);
        }
    }
    let slice = &body[start..];
    sock.write_all(
        format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\n{extra}Connection: close\r\n\r\n", slice.len()).as_bytes(),
    )
    .await?;
    let fail = st.fail_after.lock().take();
    match fail {
        Some(n) if n < slice.len() => {
            sock.write_all(&slice[..n]).await?;
            sock.flush().await?;
            // Abrupt close mid-body (simulates losing the connection).
            drop(sock);
        }
        _ => {
            let throttle = *st.throttle.lock();
            for chunk in slice.chunks(16 * 1024) {
                sock.write_all(chunk).await?;
                if let Some(d) = throttle {
                    tokio::time::sleep(d).await;
                }
            }
            sock.flush().await?;
        }
    }
    Ok(())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    openframe_security::sha256_bytes(bytes)
}

/// Deterministic pseudo-random payload.
pub fn payload(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| {
            ((i as u32)
                .wrapping_mul(2654435761)
                .rotate_left(seed as u32 % 31)
                >> 13) as u8
                ^ seed
        })
        .collect()
}

/// A throwaway signing key for test manifests (never the product key).
pub struct TestKey {
    sk: ed25519_dalek::SigningKey,
}

impl TestKey {
    pub fn new() -> Self {
        let seed: [u8; 32] = rand::random();
        Self {
            sk: ed25519_dalek::SigningKey::from_bytes(&seed),
        }
    }
    pub fn public_b64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(self.sk.verifying_key().to_bytes())
    }
    pub fn sign_b64(&self, bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(self.sk.sign(bytes).to_bytes())
    }
}

/// Builds a format-2 Offline AI manifest (one profile = runtime + chat model + embedding model)
/// for tests. Files are served by `TestServer` under `/<id>.gguf` and `/<runtime-id>.zip`.
pub struct ManifestSpec {
    pub sequence: u64,
    pub version: String,
    pub chat_id: String,
    pub chat: Vec<u8>,
    pub embedding_id: String,
    pub embedding: Vec<u8>,
    pub embedding_dim: u32,
    /// (runtime id, backend, zip bytes)
    pub runtimes: Vec<(String, String, Vec<u8>)>,
}

impl ManifestSpec {
    pub fn new(chat: Vec<u8>, embedding: Vec<u8>, runtime_zip: Vec<u8>) -> Self {
        Self {
            sequence: 1,
            version: "1".into(),
            chat_id: "test-chat-v1".into(),
            chat,
            embedding_id: "test-embedding-v1".into(),
            embedding,
            embedding_dim: 8,
            runtimes: vec![
                ("test-runtime-cpu".into(), "cpu".into(), runtime_zip.clone()),
                ("test-runtime-vulkan".into(), "vulkan".into(), runtime_zip),
            ],
        }
    }

    /// Publish every file on `server` and return the manifest bytes.
    pub fn publish(&self, server: &TestServer) -> Vec<u8> {
        let runtimes: Vec<serde_json::Value> = self
            .runtimes
            .iter()
            .map(|(id, backend, zip)| {
                server.put(&format!("/{id}.zip"), zip.clone());
                serde_json::json!({
                    "runtimeId": id, "engine": "llama.cpp", "version": "t1", "backend": backend,
                    "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
                    "url": server.url(&format!("/{id}.zip")), "bytes": zip.len(),
                    "sha256": sha256_hex(zip), "archive": "zip", "executable": "llama-server.exe",
                    "licenseId": "MIT"
                })
            })
            .collect();
        server.put(&format!("/{}.gguf", self.chat_id), self.chat.clone());
        server.put(
            &format!("/{}.gguf", self.embedding_id),
            self.embedding.clone(),
        );
        let model = |id: &str, bytes: &[u8], role: &str| {
            let mut v = serde_json::json!({
                "modelId": id, "role": role, "displayName": format!("Test {role} model"),
                "family": "test", "quantization": "Q8_0", "version": "1",
                "url": server.url(&format!("/{id}.gguf")), "bytes": bytes.len(), "sha256": sha256_hex(bytes),
                "licenseId": "MIT", "licenseUrl": "https://example.com/license",
                "contextTokens": if role == "chat" { 4096 } else { 512 },
                "minRamBytes": 1, "recommendedRamBytes": 1, "gpuVramBytes": 1, "layers": 4
            });
            if role == "embedding" {
                v["embeddingDim"] = serde_json::json!(self.embedding_dim);
                v["pooling"] = serde_json::json!("cls");
            }
            v
        };
        serde_json::to_vec_pretty(&serde_json::json!({
            "manifestVersion": 2, "sequence": self.sequence, "channel": "test", "issuedAt": "2026-09-30",
            "profile": {
                "profileId": "openframe-local-ai-v1", "version": self.version,
                "chatModelId": self.chat_id, "embeddingModelId": self.embedding_id
            },
            "runtimes": runtimes,
            "models": [
                model(&self.chat_id, &self.chat, "chat"),
                model(&self.embedding_id, &self.embedding, "embedding")
            ]
        }))
        .unwrap()
    }
}

/// A zip holding `llama-server.exe` (the given bytes) plus optional extra files.
pub fn runtime_zip(exe: &[u8], extra: &[(&str, &[u8])]) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("rt.zip");
    let mut entries = vec![(
        "llama-server.exe".to_string(),
        openframe_security::archive::EntrySource::Bytes(exe),
    )];
    for (name, bytes) in extra {
        entries.push((
            name.to_string(),
            openframe_security::archive::EntrySource::Bytes(bytes),
        ));
    }
    openframe_security::archive::write_zip(&p, entries).unwrap();
    std::fs::read(p).unwrap()
}

/// Manager configuration for tests: throwaway key, isolated folder, short timeouts.
pub fn test_config(
    dir: &std::path::Path,
    key: &TestKey,
    manifest: Vec<u8>,
) -> openframe_ai::ManagerConfig {
    use std::time::Duration;
    let sig = key.sign_b64(&manifest);
    let sup = openframe_ai::supervisor::SupervisorConfig {
        start_timeout: Duration::from_secs(30),
        idle_timeout: None,
        poll_interval: Duration::from_millis(100),
        ..openframe_ai::supervisor::SupervisorConfig::default()
    };
    openframe_ai::ManagerConfig {
        app_data_dir: dir.to_path_buf(),
        trusted_public_key: key.public_b64(),
        embedded_manifest: (manifest, sig),
        distribution_base_url: None,
        download: openframe_ai::download::DownloadOptions {
            max_attempts: 2,
            base_backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(20),
            stall_timeout: Duration::from_secs(10),
        },
        supervisor: sup.clone(),
        embedding_supervisor: sup,
        health_timeout: Duration::from_secs(30),
        backend_override: None,
    }
}

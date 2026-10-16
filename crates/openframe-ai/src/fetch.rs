//! Download after install, step 2: fetching (ADR-0026 decision 5; Windows phase W8).
//!
//! HTTPS only: the base address must be `https://`. The one exception is a build whose compiled-in
//! model host is the loopback address (the local test host of the test suite and CI, ADR-0026
//! decision 10): it may use plain HTTP there and nowhere else. The address is fixed at build time, so
//! nothing at run time can turn a production build towards plain HTTP. A redirect is
//! followed only to the same scheme, host and port. Every file goes to `<name>.partial` and resumes
//! with an HTTP range request; the bytes written can never exceed the size the signed manifest gives.
//! Each read has a time limit, so a stalled host ends in an error instead of a hang, and a pause is a
//! cancel that keeps the `.partial` file for the next resume. Nothing here makes a file usable: the
//! size and hash check and the atomic activation come after (`install.rs`).

use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use openframe_domain::{AppError, AppResult};
use reqwest::{StatusCode, Url, header};

use crate::download::{FileEntry, MAX_ENVELOPE_BYTES};

/// The model host, as configured at build time (`MODEL_DISTRIBUTION_BASE_URL`).
#[derive(Debug, Clone)]
pub struct Source {
    base: Url,
}

fn failed(detail: impl Into<String>) -> AppError {
    AppError::ai(
        "download_failed",
        "The OpenFrame AI download stopped. Nothing that was already working was changed; you can resume it later.",
    )
    .with_detail(detail.into())
}

impl Source {
    /// `allow_loopback_http` lets a build whose compiled-in host is the local test host reach it
    /// over plain HTTP; any other address must be https.
    pub fn new(base: &str, allow_loopback_http: bool) -> AppResult<Self> {
        let mut url =
            Url::parse(base.trim()).map_err(|_| failed("the model host address is not valid"))?;
        let loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
        let ok = match url.scheme() {
            "https" => true,
            "http" => allow_loopback_http && loopback,
            _ => false,
        };
        if !ok || url.cannot_be_a_base() || !url.username().is_empty() || url.password().is_some() {
            return Err(failed("the model host must be an https address"));
        }
        if !url.path().ends_with('/') {
            let p = format!("{}/", url.path());
            url.set_path(&p);
        }
        Ok(Source { base: url })
    }

    /// The address of a file the manifest names (already checked to be a safe relative path).
    pub fn url(&self, rel: &str) -> AppResult<Url> {
        let u = self
            .base
            .join(rel)
            .map_err(|_| failed("a file address is not valid"))?;
        if !same_origin(&u, &self.base) || !u.path().starts_with(self.base.path()) {
            return Err(failed("a file address leaves the model host"));
        }
        Ok(u)
    }
}

fn same_origin(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}

/// The HTTP client: redirects only within the model host, no cookies, bounded waits.
pub fn client(source: &Source) -> AppResult<reqwest::Client> {
    let base = source.base.clone();
    let loopback = matches!(base.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    let builder = if loopback {
        reqwest::Client::builder().no_proxy()
    } else {
        reqwest::Client::builder()
    };
    builder
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 5 || !same_origin(attempt.url(), &base) {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        .https_only(!loopback)
        .user_agent(concat!("OpenFrameStudio/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| failed(e.to_string()))
}

/// The signed manifest envelope, at most `MAX_ENVELOPE_BYTES`.
pub async fn fetch_manifest(
    http: &reqwest::Client,
    source: &Source,
    rel: &str,
) -> AppResult<Vec<u8>> {
    let mut resp = http
        .get(source.url(rel)?)
        .send()
        .await
        .map_err(|e| failed(e.to_string()))?;
    if resp.status() != StatusCode::OK {
        return Err(failed(format!(
            "the host answered {}",
            resp.status().as_u16()
        )));
    }
    if !same_origin(resp.url(), &source.base) {
        return Err(failed("the host sent the list from somewhere else"));
    }
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| failed(e.to_string()))? {
        body.extend_from_slice(&chunk);
        if body.len() > MAX_ENVELOPE_BYTES {
            return Err(failed("the download list is larger than allowed"));
        }
    }
    Ok(body)
}

pub fn cancelled() -> AppError {
    AppError::ai(
        "download_paused",
        "The OpenFrame AI download is paused. It resumes from where it stopped.",
    )
}

/// Download one file into `partial`, resuming what is already there. `progress` receives the bytes
/// held so far. Returns when the file holds exactly `entry.bytes` bytes (not yet hash-checked).
pub async fn download_file(
    http: &reqwest::Client,
    source: &Source,
    entry: &FileEntry,
    partial: &Path,
    progress: &(dyn Fn(u64) + Send + Sync),
    cancel: &AtomicBool,
) -> AppResult<()> {
    let mut have = std::fs::metadata(partial).map(|m| m.len()).unwrap_or(0);
    if have > entry.bytes {
        have = 0; // a longer file is not this file: start again
    }
    if have == entry.bytes {
        progress(have);
        return Ok(());
    }
    let mut req = http.get(source.url(&entry.path)?);
    if have > 0 {
        req = req.header(header::RANGE, format!("bytes={have}-"));
    }
    let mut resp = req.send().await.map_err(|e| failed(e.to_string()))?;
    if !same_origin(resp.url(), &source.base) {
        return Err(failed("the host sent the file from somewhere else"));
    }
    let resume = match resp.status() {
        StatusCode::PARTIAL_CONTENT => {
            let start = resp
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("bytes "))
                .and_then(|v| v.split('-').next())
                .and_then(|v| v.parse::<u64>().ok());
            if start != Some(have) {
                return Err(failed("the host resumed at the wrong place"));
            }
            true
        }
        StatusCode::OK => false,
        s => return Err(failed(format!("the host answered {}", s.as_u16()))),
    };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(!resume)
        .open(partial)
        .map_err(|e| failed(format!("cannot write the download: {}", e.kind())))?;
    let mut held = if resume { have } else { 0 };
    file.seek(SeekFrom::Start(held))
        .map_err(|e| failed(e.kind().to_string()))?;
    progress(held);
    while let Some(chunk) = resp.chunk().await.map_err(|e| failed(e.to_string()))? {
        if cancel.load(Ordering::Relaxed) {
            file.flush().ok();
            return Err(cancelled());
        }
        held += chunk.len() as u64;
        if held > entry.bytes {
            // More than the signed size: never keep it.
            drop(file);
            let _ = std::fs::remove_file(partial);
            return Err(failed("the host sent more than the signed size"));
        }
        file.write_all(&chunk)
            .map_err(|e| failed(format!("cannot write the download: {}", e.kind())))?;
        progress(held);
    }
    file.sync_all().map_err(|e| failed(e.kind().to_string()))?;
    if held != entry.bytes {
        return Err(failed("the download ended early"));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    /// A tiny local HTTP host: serves `files` (path, bytes), honours `Range: bytes=N-`, and can
    /// answer a redirect, cut a response short, or pretend not to support ranges.
    pub(crate) struct TestHost {
        pub base: String,
        pub requests: Arc<AtomicUsize>,
    }

    #[derive(Clone, Copy, PartialEq)]
    pub(crate) enum Mode {
        Normal,
        /// Send only this many bytes of the body, then close.
        CutAfter(usize),
        /// Ignore Range and always send the whole file with 200.
        NoRanges,
        /// Answer every request with a redirect to another host.
        RedirectAway,
    }

    pub(crate) fn host(files: Vec<(String, Vec<u8>)>, mode: Mode) -> TestHost {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}/", l.local_addr().unwrap().port());
        let requests = Arc::new(AtomicUsize::new(0));
        let count = requests.clone();
        std::thread::spawn(move || {
            for stream in l.incoming().flatten() {
                count.fetch_add(1, Ordering::SeqCst);
                let mut s = stream;
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut first = String::new();
                if r.read_line(&mut first).is_err() {
                    continue;
                }
                let path = first
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .trim_start_matches('/')
                    .to_string();
                let mut range: Option<u64> = None;
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                        range = v.trim().trim_end_matches('-').parse().ok();
                    }
                }
                if mode == Mode::RedirectAway {
                    let _ = s.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://evil.example/x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    continue;
                }
                let Some((_, body)) = files.iter().find(|(p, _)| *p == path) else {
                    let _ = s.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                };
                let (status, from) = match range {
                    Some(n) if mode != Mode::NoRanges && (n as usize) < body.len() => {
                        ("206 Partial Content", n as usize)
                    }
                    _ => ("200 OK", 0),
                };
                let part = &body[from..];
                let mut head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
                    part.len()
                );
                if from > 0 {
                    head.push_str(&format!(
                        "Content-Range: bytes {from}-{}/{}\r\n",
                        body.len() - 1,
                        body.len()
                    ));
                }
                head.push_str("\r\n");
                let _ = s.write_all(head.as_bytes());
                let send = match mode {
                    Mode::CutAfter(n) => &part[..n.min(part.len())],
                    _ => part,
                };
                let _ = s.write_all(send);
            }
        });
        TestHost { base, requests }
    }

    fn entry(name: &str, bytes: u64) -> FileEntry {
        FileEntry {
            name: name.into(),
            path: format!("models/{name}"),
            bytes,
            sha256: "0".repeat(64),
            role: "chat".into(),
        }
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn only_https_or_the_loopback_test_host_is_accepted() {
        assert!(Source::new("https://models.example.com/ai", false).is_ok());
        assert!(Source::new("http://models.example.com/ai", true).is_err());
        assert!(Source::new("ftp://models.example.com/", true).is_err());
        assert!(Source::new("https://user:pw@models.example.com/", false).is_err());
        assert!(Source::new("http://127.0.0.1:9/", true).is_ok());
        assert!(Source::new("http://127.0.0.1:9/", false).is_err());
        let s = Source::new("https://models.example.com/ai", false).unwrap();
        assert_eq!(
            s.url("models/x.gguf").unwrap().as_str(),
            "https://models.example.com/ai/models/x.gguf"
        );
        assert!(s.url("https://evil.example/x").is_err());
        assert!(s.url("//evil.example/x").is_err());
    }

    #[test]
    fn a_file_downloads_then_an_interrupted_one_resumes_where_it_stopped() {
        let body: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let dir = tempfile::tempdir().unwrap();
        let partial = dir.path().join("chat.gguf.partial");
        let e = entry("chat.gguf", body.len() as u64);
        let rt = rt();
        // A host that cuts the first answer short leaves a .partial file and an error.
        let cut = host(
            vec![("models/chat.gguf".into(), body.clone())],
            Mode::CutAfter(70_000),
        );
        let src = Source::new(&cut.base, true).unwrap();
        let http = client(&src).unwrap();
        let no = AtomicBool::new(false);
        assert!(
            rt.block_on(download_file(&http, &src, &e, &partial, &|_| {}, &no))
                .is_err()
        );
        let after_cut = std::fs::metadata(&partial).unwrap().len();
        assert!(after_cut > 0 && after_cut < body.len() as u64);
        // A whole host resumes from that point with a range request.
        let ok = host(
            vec![("models/chat.gguf".into(), body.clone())],
            Mode::Normal,
        );
        let src = Source::new(&ok.base, true).unwrap();
        let http = client(&src).unwrap();
        let seen = std::sync::Mutex::new(Vec::new());
        rt.block_on(download_file(
            &http,
            &src,
            &e,
            &partial,
            &|n| seen.lock().unwrap().push(n),
            &no,
        ))
        .unwrap();
        assert_eq!(std::fs::read(&partial).unwrap(), body);
        assert_eq!(
            seen.lock().unwrap()[0],
            after_cut,
            "the resume started where the cut left off"
        );
        // Already complete: no request at all.
        let before = ok.requests.load(Ordering::SeqCst);
        rt.block_on(download_file(&http, &src, &e, &partial, &|_| {}, &no))
            .unwrap();
        assert_eq!(ok.requests.load(Ordering::SeqCst), before);
    }

    #[test]
    fn a_host_without_ranges_restarts_cleanly_and_never_doubles_the_file() {
        let body = vec![5u8; 50_000];
        let dir = tempfile::tempdir().unwrap();
        let partial = dir.path().join("x.partial");
        std::fs::write(&partial, vec![5u8; 20_000]).unwrap();
        let h = host(vec![("models/x.gguf".into(), body.clone())], Mode::NoRanges);
        let src = Source::new(&h.base, true).unwrap();
        let http = client(&src).unwrap();
        rt().block_on(download_file(
            &http,
            &src,
            &entry("x.gguf", 50_000),
            &partial,
            &|_| {},
            &AtomicBool::new(false),
        ))
        .unwrap();
        assert_eq!(std::fs::read(&partial).unwrap(), body);
    }

    #[test]
    fn more_than_the_signed_size_a_redirect_away_and_a_pause_are_all_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let rt = rt();
        let no = AtomicBool::new(false);
        // The host sends 60 000 bytes for a file the manifest signs as 10 000.
        let big = host(
            vec![("models/x.gguf".into(), vec![1u8; 60_000])],
            Mode::Normal,
        );
        let src = Source::new(&big.base, true).unwrap();
        let http = client(&src).unwrap();
        let partial = dir.path().join("x.partial");
        assert!(
            rt.block_on(download_file(
                &http,
                &src,
                &entry("x.gguf", 10_000),
                &partial,
                &|_| {},
                &no
            ))
            .is_err()
        );
        assert!(!partial.exists(), "an oversized download is not kept");
        // A redirect to another host is never followed.
        let away = host(vec![], Mode::RedirectAway);
        let src = Source::new(&away.base, true).unwrap();
        let http = client(&src).unwrap();
        assert!(
            rt.block_on(download_file(
                &http,
                &src,
                &entry("x.gguf", 10),
                &partial,
                &|_| {},
                &no
            ))
            .is_err()
        );
        assert!(
            rt.block_on(fetch_manifest(&http, &src, "manifest.json"))
                .is_err()
        );
        // Pause: the cancel flag stops the copy and keeps what was written.
        let h = host(
            vec![("models/y.gguf".into(), vec![2u8; 300_000])],
            Mode::Normal,
        );
        let src = Source::new(&h.base, true).unwrap();
        let http = client(&src).unwrap();
        let stop = AtomicBool::new(true);
        let err = rt
            .block_on(download_file(
                &http,
                &src,
                &entry("y.gguf", 300_000),
                &dir.path().join("y.partial"),
                &|_| {},
                &stop,
            ))
            .unwrap_err();
        assert!(err.is("ai.download_paused"));
    }
}

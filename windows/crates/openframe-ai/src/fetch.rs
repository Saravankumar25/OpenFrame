//! Download after install: fetching (ADR-0026 decision 5, ADR-0032; Windows phase W8).
//!
//! Every model file comes from its Hugging Face `resolve` address at the commit pinned in the app,
//! over HTTPS. A redirect is followed only to `huggingface.co` or a host under `hf.co`, at most
//! five times; nothing else is ever contacted. A test build may name a loopback test host instead
//! (`OPENFRAME_TEST_MODEL_SOURCE`): plain HTTP there and nowhere else. Every file goes to
//! `<name>.partial` and resumes with an HTTP range request; the bytes written can never exceed the
//! pinned size. Each read has a time limit, so a stalled host ends in an error instead of a hang, and
//! a pause is a cancel that keeps the `.partial` file for the next resume. Nothing here makes a file
//! usable: the size and hash check and the atomic activation come after (`install.rs`).

use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use openframe_domain::{AppError, AppResult};
use reqwest::{StatusCode, Url, header};

use crate::download::{FileEntry, huggingface_url_ok};

/// Where the model files come from: Hugging Face (every build for people), or the loopback test
/// host a test build is made with (ADR-0032 decision 3).
#[derive(Debug, Clone)]
pub struct ModelSource {
    test: Option<Url>,
}

fn failed(detail: impl Into<String>) -> AppError {
    AppError::ai(
        "download_failed",
        "The OpenFrame AI download stopped. Nothing that was already working was changed; you can resume it later.",
    )
    .with_detail(detail.into())
}

impl ModelSource {
    pub fn huggingface() -> Self {
        ModelSource { test: None }
    }

    /// The loopback test host: plain HTTP on `127.0.0.1` or `localhost`, nothing else.
    pub fn test_host(base: &str) -> AppResult<Self> {
        let mut url =
            Url::parse(base.trim()).map_err(|_| failed("the test host address is not valid"))?;
        let loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost"));
        if url.scheme() != "http"
            || !loopback
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(failed("the test host must be http on the loopback address"));
        }
        if !url.path().ends_with('/') {
            let p = format!("{}/", url.path());
            url.set_path(&p);
        }
        Ok(ModelSource { test: Some(url) })
    }

    /// The build value `OPENFRAME_TEST_MODEL_SOURCE`: absent means Hugging Face; a loopback address
    /// means that test host; anything else means this build cannot download (the shell logs it).
    pub fn from_build(test: Option<&str>) -> Option<Self> {
        match test.map(str::trim).filter(|t| !t.is_empty()) {
            None => Some(Self::huggingface()),
            Some(t) => Self::test_host(t).ok(),
        }
    }

    pub fn is_test(&self) -> bool {
        self.test.is_some()
    }

    /// The address of a pinned file: its Hugging Face address, or the same path on the test host.
    pub fn url_for(&self, pinned: &str) -> AppResult<Url> {
        if !huggingface_url_ok(pinned) {
            return Err(failed("not a pinned Hugging Face address"));
        }
        let u = Url::parse(pinned).map_err(|_| failed("a file address is not valid"))?;
        match &self.test {
            None => Ok(u),
            Some(t) => t
                .join(u.path().trim_start_matches('/'))
                .map_err(|_| failed("a file address is not valid")),
        }
    }

    /// A response or a redirect may come only from Hugging Face's own hosts over https, or from
    /// the test host itself.
    pub fn allowed(&self, u: &Url) -> bool {
        match &self.test {
            Some(t) => {
                u.scheme() == t.scheme()
                    && u.host_str() == t.host_str()
                    && u.port_or_known_default() == t.port_or_known_default()
            }
            None => {
                u.scheme() == "https"
                    && u.username().is_empty()
                    && u.password().is_none()
                    && u.port().is_none()
                    && matches!(u.host_str(), Some(h) if h == "huggingface.co" || h.ends_with(".hf.co"))
            }
        }
    }
}

/// The HTTP client: redirects only to allowed hosts, no cookies, bounded waits.
pub fn client(source: &ModelSource) -> AppResult<reqwest::Client> {
    let check = source.clone();
    let builder = if source.is_test() {
        reqwest::Client::builder().no_proxy()
    } else {
        reqwest::Client::builder()
    };
    builder
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 5 || !check.allowed(attempt.url()) {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        .https_only(!source.is_test())
        .user_agent(concat!("OpenFrameStudio/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| failed(e.to_string()))
}

/// Resolves once `cancel` is set (checked every quarter second).
async fn paused(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
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
    source: &ModelSource,
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
    let mut req = http.get(source.url_for(&entry.url)?);
    if have > 0 {
        req = req.header(header::RANGE, format!("bytes={have}-"));
    }
    let mut resp = req.send().await.map_err(|e| failed(e.to_string()))?;
    if !source.allowed(resp.url()) {
        return Err(failed(
            "the file came from somewhere other than Hugging Face",
        ));
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
        StatusCode::RANGE_NOT_SATISFIABLE => {
            // The partial file does not fit this file on the host: start it again next time.
            let _ = std::fs::remove_file(partial);
            return Err(failed("the host refused to resume; the file starts again"));
        }
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
    loop {
        // A pause takes effect at once, not only when the next chunk arrives.
        let next = tokio::select! {
            c = resp.chunk() => c.map_err(|e| failed(e.to_string()))?,
            () = paused(cancel) => None,
        };
        if cancel.load(Ordering::Relaxed) {
            file.flush().ok();
            return Err(cancelled());
        }
        let Some(chunk) = next else { break };
        held += chunk.len() as u64;
        if held > entry.bytes {
            // More than the pinned size: never keep it.
            drop(file);
            let _ = std::fs::remove_file(partial);
            return Err(failed("the host sent more than the pinned size"));
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

    pub(crate) const REV: &str = "0123456789abcdef0123456789abcdef01234567";

    /// Where the test host serves a file of this name (its Hugging Face path).
    pub(crate) fn hf_path(name: &str) -> String {
        format!("o/r/resolve/{REV}/{name}")
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
            url: format!("https://huggingface.co/{}", hf_path(name)),
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
    fn only_hugging_face_hosts_over_https_or_the_loopback_test_host_are_used() {
        let hf = ModelSource::huggingface();
        let ok = |s: &str| hf.allowed(&Url::parse(s).unwrap());
        assert!(ok("https://huggingface.co/o/r/resolve/x/m.gguf"));
        assert!(ok("https://us.aws.cdn.hf.co/xet-bridge-us/abc?sig=1"));
        assert!(ok("https://cdn-lfs.hf.co/repos/x"));
        for bad in [
            "http://huggingface.co/x",
            "http://us.aws.cdn.hf.co/x",
            "https://evilhf.co/x",
            "https://huggingface.co.evil.example/x",
            "https://hf.co.evil.example/x",
            "https://user:pw@huggingface.co/x",
            "https://huggingface.co:8443/x",
            "https://127.0.0.1/x",
        ] {
            assert!(!ok(bad), "{bad}");
        }
        let pinned = format!("https://huggingface.co/{}", hf_path("m.gguf"));
        assert_eq!(hf.url_for(&pinned).unwrap().as_str(), pinned);
        assert!(
            hf.url_for("https://evil.example/m.gguf").is_err(),
            "only pinned addresses"
        );
        // The test host: the same path, loopback HTTP only, and nothing else allowed.
        let t = ModelSource::test_host("http://127.0.0.1:47811").unwrap();
        assert_eq!(
            t.url_for(&pinned).unwrap().as_str(),
            format!("http://127.0.0.1:47811/{}", hf_path("m.gguf"))
        );
        assert!(!t.allowed(&Url::parse("https://huggingface.co/x").unwrap()));
        assert!(ModelSource::test_host("http://models.example.com/").is_err());
        assert!(ModelSource::test_host("https://127.0.0.1/").is_err());
        assert!(ModelSource::from_build(None).is_some_and(|s| !s.is_test()));
        assert!(
            ModelSource::from_build(Some("http://127.0.0.1:47811/")).is_some_and(|s| s.is_test())
        );
        assert!(ModelSource::from_build(Some("https://models.example.com/")).is_none());
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
            vec![(hf_path("chat.gguf"), body.clone())],
            Mode::CutAfter(70_000),
        );
        let src = ModelSource::test_host(&cut.base).unwrap();
        let http = client(&src).unwrap();
        let no = AtomicBool::new(false);
        assert!(
            rt.block_on(download_file(&http, &src, &e, &partial, &|_| {}, &no))
                .is_err()
        );
        let after_cut = std::fs::metadata(&partial).unwrap().len();
        assert!(after_cut > 0 && after_cut < body.len() as u64);
        // A whole host resumes from that point with a range request.
        let ok = host(vec![(hf_path("chat.gguf"), body.clone())], Mode::Normal);
        let src = ModelSource::test_host(&ok.base).unwrap();
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
        let h = host(vec![(hf_path("x.gguf"), body.clone())], Mode::NoRanges);
        let src = ModelSource::test_host(&h.base).unwrap();
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
    fn more_than_the_pinned_size_a_redirect_away_and_a_pause_are_all_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let rt = rt();
        let no = AtomicBool::new(false);
        // The host sends 60 000 bytes for a file pinned at 10 000.
        let big = host(vec![(hf_path("x.gguf"), vec![1u8; 60_000])], Mode::Normal);
        let src = ModelSource::test_host(&big.base).unwrap();
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
        let src = ModelSource::test_host(&away.base).unwrap();
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
        // Pause: the cancel flag stops the copy and keeps what was written.
        let h = host(vec![(hf_path("y.gguf"), vec![2u8; 300_000])], Mode::Normal);
        let src = ModelSource::test_host(&h.base).unwrap();
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

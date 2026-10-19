//! AI engine safety records (build plan item "AI engine lifecycle").
//!
//! - [`EngineFiles`]: the SHA-256 of every file of the bundled engine, pinned in the committed
//!   lock (`packaging/ai/ai-assets.lock.json`, ADR-0025) and checked again before every launch. A
//!   changed or missing file, or an added program file (an `.exe`, `.dll`, `.dylib` or `.so`
//!   planted next to the engine), stops the launch.
//! - [`EngineRegistry`]: every engine OpenFrame launches is recorded with its pid, start time and
//!   program (`engines.json`) and removed when it stops. At the next start, an engine still
//!   running from a record (left behind by a crash) is stopped. A reused pid of another program is
//!   never touched: the start time and program must match too (`openframe_platform::process`).

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Child;

use openframe_domain::{AppError, AppResult};
use openframe_platform::process::{ProcessStamp, SweepOutcome, stamp, stop_if_same};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::bundle::PinnedFile;

/// Bounds for the folder walk (the engine folder holds a few dozen files).
const MAX_FILES: usize = 4_000;
const MAX_DEPTH: usize = 8;

/// The pinned hashes of the engine folder, bound to that folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineFiles {
    dir: PathBuf,
    /// Relative `/`-separated path -> (bytes, sha256).
    files: BTreeMap<String, (u64, String)>,
}

/// The error shown when the engine files do not match the pinned hashes.
pub fn unverified() -> AppError {
    crate::bundle::damaged()
}

/// SHA-256 and size of one file (streamed, never read into memory at once).
pub fn hash_file(path: &Path) -> AppResult<(u64, String)> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut bytes = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        h.update(&buf[..n]);
    }
    Ok((bytes, hex::encode(h.finalize())))
}

/// Files that can run code or change which code runs: programs and libraries, and the Windows
/// redirection files (`*.local`, `*.manifest`, `*.config`) that make a program load other DLLs.
fn is_program(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        ".exe",
        ".dll",
        ".dylib",
        ".so",
        ".metallib",
        ".local",
        ".manifest",
        ".config",
    ]
    .iter()
    .any(|e| lower.ends_with(e))
        || lower.split('/').any(|part| part.ends_with(".local"))
        || !lower.contains('.')
}

/// Every regular file under `dir` as a `/`-separated relative path. Links are refused: the
/// engine folder is installed with the app and never contains one.
fn walk(dir: &Path) -> AppResult<Vec<String>> {
    fn go(root: &Path, dir: &Path, depth: usize, out: &mut Vec<String>) -> AppResult<()> {
        if depth > MAX_DEPTH {
            return Err(unverified().with_detail("engine folder too deep"));
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let path = entry.path();
            if kind.is_symlink() {
                return Err(unverified().with_detail("link in engine folder"));
            }
            if kind.is_dir() {
                go(root, &path, depth + 1, out)?;
            } else if kind.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|_| unverified())?
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push(rel);
                if out.len() > MAX_FILES {
                    return Err(unverified().with_detail("too many files in engine folder"));
                }
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    go(dir, dir, 0, &mut out)?;
    out.sort();
    Ok(out)
}

impl EngineFiles {
    /// The engine folder `dir` with the files the lock pins for it.
    pub fn pinned(dir: &Path, files: &[PinnedFile]) -> EngineFiles {
        EngineFiles {
            dir: dir.to_path_buf(),
            files: files
                .iter()
                .map(|f| (f.path.clone(), (f.bytes, f.sha256.clone())))
                .collect(),
        }
    }

    /// Hash the folder again: every pinned file must be present and unchanged, and no program
    /// file may have been added.
    pub fn verify(&self) -> AppResult<()> {
        if self.files.is_empty() {
            return Err(unverified().with_detail("no engine files pinned"));
        }
        let present = walk(&self.dir)?;
        if let Some(extra) = present
            .iter()
            .find(|p| !self.files.contains_key(*p) && is_program(p))
        {
            return Err(unverified().with_detail(format!("unexpected program file {extra}")));
        }
        for (rel, want) in &self.files {
            let got = hash_file(&self.dir.join(rel))
                .map_err(|_| unverified().with_detail(format!("missing {rel}")))?;
            if &got != want {
                return Err(unverified().with_detail(format!("changed {rel}")));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EngineRecord {
    pid: u32,
    started: u64,
    program: PathBuf,
}

/// Engines launched by this installation, kept in `engines.json` in the AI folder.
#[derive(Debug)]
pub struct EngineRegistry {
    path: PathBuf,
    lock: Mutex<()>,
}

impl EngineRegistry {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    fn read(&self) -> Vec<EngineRecord> {
        crate::store::read_json(&self.path).unwrap_or_default()
    }

    fn write(&self, records: &[EngineRecord]) {
        if records.is_empty() {
            let _ = std::fs::remove_file(&self.path);
        } else if let Err(e) = crate::store::write_json_atomic(&self.path, &records) {
            tracing::warn!(code = e.code_str(), "could not update the AI engine record");
        }
    }

    /// Record a launched engine. Returns the pid to forget later, or `None` when the operating
    /// system does not report a start time (the engine is then not swept).
    pub fn record(&self, child: &Child, program: &Path) -> Option<u32> {
        let s = stamp(child)?;
        let _g = self.lock.lock();
        let mut records = self.read();
        records.retain(|r| r.pid != s.pid);
        records.push(EngineRecord {
            pid: s.pid,
            started: s.started,
            program: program.to_path_buf(),
        });
        self.write(&records);
        Some(s.pid)
    }

    /// Remove a stopped engine from the record.
    pub fn forget(&self, pid: u32) {
        let _g = self.lock.lock();
        let mut records = self.read();
        let before = records.len();
        records.retain(|r| r.pid != pid);
        if records.len() != before {
            self.write(&records);
        }
    }

    /// Stop every recorded engine that is still running from the recorded program, then clear
    /// the record. Run once at start, before any engine is launched. Returns how many were
    /// stopped.
    pub fn sweep(&self) -> usize {
        let _g = self.lock.lock();
        let records = self.read();
        let mut stopped = 0;
        for r in &records {
            let s = ProcessStamp {
                pid: r.pid,
                started: r.started,
            };
            if stop_if_same(s, &r.program) == SweepOutcome::Stopped {
                stopped += 1;
            }
        }
        if stopped > 0 {
            tracing::warn!(
                stopped,
                "stopped offline AI engines left running by an earlier session"
            );
        }
        self.write(&[]);
        stopped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(dir: &Path) -> Vec<PinnedFile> {
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("llama-server.exe"), b"engine").unwrap();
        std::fs::write(dir.join("ggml.dll"), b"ggml").unwrap();
        std::fs::write(dir.join("lib").join("vulkan.dll"), b"vk").unwrap();
        std::fs::write(dir.join("LICENSE"), b"MIT").unwrap();
        ["llama-server.exe", "ggml.dll", "lib/vulkan.dll", "LICENSE"]
            .iter()
            .map(|rel| {
                let (bytes, sha256) = hash_file(&dir.join(rel)).unwrap();
                PinnedFile {
                    path: rel.to_string(),
                    bytes,
                    sha256,
                }
            })
            .collect()
    }

    #[test]
    fn a_pinned_engine_verifies() {
        let tmp = tempfile::tempdir().unwrap();
        let pins = engine(tmp.path());
        EngineFiles::pinned(tmp.path(), &pins).verify().unwrap();
    }

    #[test]
    fn a_changed_missing_or_planted_program_stops_the_launch() {
        type Tamper = fn(&Path);
        let cases: [(&str, Tamper); 7] = [
            ("changed exe", |d| {
                std::fs::write(d.join("llama-server.exe"), b"evil").unwrap()
            }),
            ("changed dll", |d| {
                std::fs::write(d.join("lib").join("vulkan.dll"), b"vk2").unwrap()
            }),
            ("missing dll", |d| {
                std::fs::remove_file(d.join("ggml.dll")).unwrap()
            }),
            ("planted dll", |d| {
                std::fs::write(d.join("version.dll"), b"x").unwrap()
            }),
            ("planted program", |d| {
                std::fs::write(d.join("lib").join("helper"), b"x").unwrap()
            }),
            ("redirection manifest", |d| {
                std::fs::write(d.join("llama-server.exe.manifest"), b"<assembly/>").unwrap()
            }),
            ("redirection folder", |d| {
                std::fs::create_dir_all(d.join("llama-server.exe.local")).unwrap();
                std::fs::write(d.join("llama-server.exe.local").join("readme.txt"), b"x").unwrap()
            }),
        ];
        for (name, tamper) in cases {
            let tmp = tempfile::tempdir().unwrap();
            let pins = engine(tmp.path());
            tamper(tmp.path());
            let e = EngineFiles::pinned(tmp.path(), &pins).verify().unwrap_err();
            assert_eq!(e.code_str(), "ai.assets_damaged", "{name}");
            assert!(!e.message.contains('\u{2014}'));
        }
        // A harmless added text file is allowed.
        let tmp = tempfile::tempdir().unwrap();
        let pins = engine(tmp.path());
        std::fs::write(tmp.path().join("notes.txt"), b"x").unwrap();
        EngineFiles::pinned(tmp.path(), &pins).verify().unwrap();
    }

    #[test]
    fn an_engine_with_nothing_pinned_is_never_started() {
        let tmp = tempfile::tempdir().unwrap();
        engine(tmp.path());
        assert!(EngineFiles::pinned(tmp.path(), &[]).verify().is_err());
    }

    /// A stray engine recorded by an earlier session is stopped at the next start; the record
    /// is cleared; a stopped engine is forgotten.
    #[cfg(windows)]
    #[test]
    fn the_startup_sweep_stops_a_stray_engine_from_the_record() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = EngineRegistry::new(tmp.path().join("engines.json"));
        let comspec: PathBuf = std::env::var_os("ComSpec")
            .map(Into::into)
            .unwrap_or_else(|| r"C:\Windows\System32\cmd.exe".into());
        let mut stray = std::process::Command::new(&comspec)
            .args(["/C", "ping -n 30 127.0.0.1 > NUL"])
            .spawn()
            .unwrap();
        let pid = reg.record(&stray, &comspec).expect("recorded");
        assert!(tmp.path().join("engines.json").is_file());
        // A second, already stopped engine is forgotten normally.
        let mut done = std::process::Command::new(&comspec)
            .args(["/C", "exit 0"])
            .spawn()
            .unwrap();
        let done_pid = reg.record(&done, &comspec).unwrap();
        done.wait().unwrap();
        reg.forget(done_pid);
        assert_eq!(reg.read().len(), 1);
        assert_eq!(reg.read()[0].pid, pid);
        // Next session.
        let reg = EngineRegistry::new(tmp.path().join("engines.json"));
        assert_eq!(reg.sweep(), 1);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while stray.try_wait().unwrap().is_none() {
            assert!(
                std::time::Instant::now() < deadline,
                "stray engine still running"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(!tmp.path().join("engines.json").exists(), "record cleared");
        assert_eq!(reg.sweep(), 0, "nothing left to stop");
    }

    #[test]
    fn a_damaged_record_is_cleared_without_stopping_anything() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("engines.json"), b"{broken").unwrap();
        let reg = EngineRegistry::new(tmp.path().join("engines.json"));
        assert_eq!(reg.sweep(), 0);
    }
}

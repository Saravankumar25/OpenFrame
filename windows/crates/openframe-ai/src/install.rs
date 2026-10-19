//! Download after install, steps 3 to 5: preflight, check, activation, quarantine (ADR-0026
//! decision 5; Windows phase W8).
//!
//! The models folder (`%LOCALAPPDATA%\OpenFrame\models` on Windows) holds:
//!
//! ```text
//! staging/<version>/<name>.partial   files being downloaded (resumable)
//! staging/<version>/<name>           a file whose size and SHA-256 matched the pins in the app
//! <version>/<name>                   an activated set, moved in one rename
//! quarantine/<time>-<name>.bad       a file that failed its check, kept for support, never used
//! active.json                        { version, tier, previous, activatedAt }
//! ```
//!
//! A file becomes usable only after its size and hash match; a set becomes active only when every
//! file of the tier is there and checked, by renaming its staging folder and then replacing
//! `active.json` atomically. The previous set stays until the new one passes a health check, so a
//! failure never leaves a half-installed model active. The sizes and hashes the set is checked
//! against come from the app itself (the compiled lock, ADR-0032), never from this writable folder.

use std::io::Read;
use std::path::{Path, PathBuf};

use openframe_domain::{AppError, AppResult, now_ms};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::bundle::{InstalledModels, PinnedFile, models_damaged};
use crate::download::{FileEntry, Tier};

/// Room kept free on top of what the download needs.
pub const PREFLIGHT_MARGIN_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Active {
    pub version: String,
    pub tier: String,
    /// The set to fall back to until the new one passes its health check.
    pub previous: Option<String>,
    pub activated_at: i64,
}

/// Read a small file of this folder, refusing one larger than `max` bytes.
fn read_small(path: &Path, max: usize) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(max as u64 + 1)
        .read_to_end(&mut buf)
        .ok()?;
    (buf.len() <= max).then_some(buf)
}

fn install_error(code: &'static str, message: &'static str, detail: impl Into<String>) -> AppError {
    AppError::ai(code, message).with_detail(detail.into())
}

pub struct ModelsDir {
    pub root: PathBuf,
}

impl ModelsDir {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        ModelsDir { root: root.into() }
    }
    pub fn staging(&self, version: &str) -> PathBuf {
        self.root.join("staging").join(version)
    }
    pub fn set_dir(&self, version: &str) -> PathBuf {
        // Only a single safe name ever reaches a path that may be removed.
        assert!(crate::download::safe_name(version), "unsafe model set name");
        self.root.join(version)
    }
    pub fn partial(&self, version: &str, f: &FileEntry) -> PathBuf {
        self.staging(version).join(format!("{}.partial", f.name))
    }
    pub fn checked(&self, version: &str, f: &FileEntry) -> PathBuf {
        self.staging(version).join(&f.name)
    }

    /// Bytes still to download for this tier (what is already checked or partly fetched counts).
    pub fn remaining_bytes(&self, version: &str, tier: &Tier) -> u64 {
        tier.files
            .iter()
            .map(|f| {
                if self.checked(version, f).is_file() {
                    0
                } else {
                    let have = std::fs::metadata(self.partial(version, f))
                        .map(|m| m.len())
                        .unwrap_or(0);
                    f.bytes.saturating_sub(have.min(f.bytes))
                }
            })
            .sum()
    }

    /// Step 3: refuse to start when the disk cannot hold the rest of the download and a margin.
    /// `free` is the free space the platform reports for the models folder.
    pub fn preflight(&self, version: &str, tier: &Tier, free: Option<u64>) -> AppResult<()> {
        let need = self.remaining_bytes(version, tier) + PREFLIGHT_MARGIN_BYTES;
        match free {
            Some(f) if f >= need => Ok(()),
            Some(f) => Err(install_error(
                "disk_full",
                "There is not enough free space for OpenFrame AI. Free some space and try again; nothing was downloaded.",
                format!("needs {need} bytes, {f} free"),
            )),
            None => Err(install_error(
                "disk_unknown",
                "OpenFrame could not check the free space on this drive, so the download did not start.",
                "free space unknown",
            )),
        }
    }

    /// Step 4a: a downloaded file becomes usable only when its size and SHA-256 match the signed
    /// manifest; it is then renamed from `.partial`. A mismatch is quarantined (step 5).
    pub fn check_file(&self, version: &str, f: &FileEntry) -> AppResult<PathBuf> {
        let partial = self.partial(version, f);
        // A file that cannot be read now (another program has it open) is not damaged: it stays
        // for the next try. Only a real size or hash mismatch is set aside.
        let ok = file_matches(&partial, f).map_err(|e| {
            install_error(
                "file_busy",
                "A downloaded OpenFrame AI file could not be read. Close other programs and try again.",
                e.kind().to_string(),
            )
        })?;
        if !ok {
            let moved = self.quarantine(&partial, &f.name);
            return Err(install_error(
                "file_damaged",
                "A downloaded OpenFrame AI file was damaged and has been set aside. Nothing was changed; try the download again.",
                format!(
                    "{} failed its check (kept as {})",
                    f.name,
                    moved.map(|p| p.display().to_string()).unwrap_or_default()
                ),
            ));
        }
        let dest = self.checked(version, f);
        std::fs::rename(&partial, &dest).map_err(|e| {
            install_error(
                "write_failed",
                "OpenFrame AI could not be saved.",
                e.kind().to_string(),
            )
        })?;
        Ok(dest)
    }

    /// Step 5: move a bad file or set aside, never used again. Only the newest one is kept (for
    /// support), so repeated bad downloads never fill the disk. Returns where it went.
    pub fn quarantine(&self, path: &Path, name: &str) -> Option<PathBuf> {
        let q = self.root.join("quarantine");
        let _ = std::fs::remove_dir_all(&q);
        std::fs::create_dir_all(&q).ok()?;
        let dest = q.join(format!("{}-{}.bad", now_ms(), name));
        match std::fs::rename(path, &dest) {
            Ok(()) => Some(dest),
            Err(_) => {
                let _ = std::fs::remove_file(path);
                None
            }
        }
    }

    pub fn active(&self) -> Option<Active> {
        let text = read_small(&self.root.join("active.json"), 4096)?;
        let a: Active = serde_json::from_slice(&text).ok()?;
        let safe = crate::download::safe_name(&a.version)
            && a.previous.as_deref().is_none_or(crate::download::safe_name);
        safe.then_some(a)
    }

    fn write_active(&self, a: &Active) -> AppResult<()> {
        let bytes = serde_json::to_vec_pretty(a).map_err(|e| AppError::internal(e.to_string()))?;
        openframe_platform::atomic::replace_bytes(&self.root.join("active.json"), &bytes).map_err(
            |e| {
                install_error(
                    "write_failed",
                    "OpenFrame AI could not be saved.",
                    e.kind().to_string(),
                )
            },
        )
    }

    /// Step 4b: every file of the tier is checked, so move the set into place in one rename and
    /// point `active.json` at it. The set that was active becomes `previous` until the health check.
    pub fn activate(&self, version: &str, tier: &Tier) -> AppResult<Active> {
        for name in tier.files.iter().map(|f| f.name.as_str()) {
            if !self.staging(version).join(name).is_file() {
                return Err(install_error(
                    "incomplete",
                    "The OpenFrame AI download is not complete yet.",
                    format!("{name} is missing"),
                ));
            }
        }
        let current = self.active();
        let dest = self.set_dir(version);
        if current.as_ref().is_some_and(|a| a.version == version) {
            return Err(install_error(
                "already_active",
                "This OpenFrame AI version is already in use.",
                version.to_string(),
            ));
        }
        if dest.exists() {
            // A leftover folder of this version that was never activated: replace it.
            std::fs::remove_dir_all(&dest).map_err(|e| {
                install_error(
                    "write_failed",
                    "OpenFrame AI could not be saved.",
                    e.kind().to_string(),
                )
            })?;
        }
        std::fs::rename(self.staging(version), &dest).map_err(|e| {
            install_error(
                "write_failed",
                "OpenFrame AI could not be saved.",
                e.kind().to_string(),
            )
        })?;
        let a = Active {
            version: version.to_string(),
            tier: tier.id.clone(),
            previous: current.map(|c| c.version),
            activated_at: now_ms(),
        };
        self.write_active(&a)?;
        self.sweep_sets(
            &[a.version.as_str()]
                .into_iter()
                .chain(a.previous.as_deref())
                .collect::<Vec<_>>(),
        );
        Ok(a)
    }

    /// Remove the active set and its pointer (an import replaces a damaged set of the same version).
    pub fn remove_active_set(&self) -> AppResult<()> {
        let Some(a) = self.active() else {
            return Ok(());
        };
        let failed = |e: std::io::Error| {
            install_error(
                "remove_failed",
                "OpenFrame AI could not be removed completely. Close OpenFrame and try again.",
                e.kind().to_string(),
            )
        };
        // The folder first: if it cannot go, the pointer still names it (reported as damaged),
        // never an orphaned folder that nothing points to.
        match std::fs::remove_dir_all(self.set_dir(&a.version)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(failed(e)),
            _ => {}
        }
        std::fs::remove_file(self.root.join("active.json")).map_err(failed)
    }

    /// Delete set folders that `active.json` no longer names (a previous set that could not be
    /// removed earlier, a set left by an older download). Never the staging or quarantine folders.
    pub fn sweep_sets(&self, keep: &[&str]) {
        for e in std::fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = e.file_name().to_string_lossy().into_owned();
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            if is_dir
                && !matches!(name.as_str(), "staging" | "quarantine")
                && !keep.contains(&name.as_str())
                && let Err(err) = std::fs::remove_dir_all(e.path())
            {
                tracing::warn!(kind = %err.kind(), "an old OpenFrame AI set could not be removed yet");
            }
        }
    }

    /// The new set answered its health check: the previous set is no longer needed.
    pub fn confirm_healthy(&self) -> AppResult<()> {
        let Some(mut a) = self.active() else {
            return Ok(());
        };
        if let Some(prev) = a.previous.take() {
            self.write_active(&a)?;
            if let Err(e) = std::fs::remove_dir_all(self.set_dir(&prev)) {
                tracing::warn!(kind = %e.kind(), "the previous OpenFrame AI set could not be removed yet");
            }
        }
        Ok(())
    }

    /// The new set failed its health check: go back to the previous set, or to none.
    pub fn roll_back(&self) -> AppResult<()> {
        let Some(a) = self.active() else {
            return Ok(());
        };
        let bad = self.set_dir(&a.version);
        match a.previous {
            Some(prev) if self.set_dir(&prev).is_dir() => {
                self.write_active(&Active {
                    version: prev,
                    tier: a.tier,
                    previous: None,
                    activated_at: now_ms(),
                })?;
            }
            _ => match std::fs::remove_file(self.root.join("active.json")) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    return Err(install_error(
                        "write_failed",
                        "OpenFrame AI could not be saved.",
                        e.kind().to_string(),
                    ));
                }
                _ => {}
            },
        }
        if bad.is_dir() {
            let _ = self.quarantine(&bad, &a.version);
        }
        Ok(())
    }

    /// Remove OpenFrame AI (the Remove button): every set, every staging file, the quarantine,
    /// the pointer. Links inside are removed, never followed.
    pub fn remove_all(&self) -> AppResult<()> {
        let failed = |e: std::io::Error| {
            install_error(
                "remove_failed",
                "OpenFrame AI could not be removed completely. Close OpenFrame and try again.",
                e.kind().to_string(),
            )
        };
        match std::fs::remove_file(self.root.join("active.json")) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(failed(e)),
            _ => {}
        }
        let entries = match std::fs::read_dir(&self.root) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(failed(e)),
        };
        for e in entries {
            let e = e.map_err(failed)?;
            let p = e.path();
            let r = if e.file_type().is_ok_and(|t| t.is_dir()) {
                std::fs::remove_dir_all(&p)
            } else {
                std::fs::remove_file(&p).or_else(|_| std::fs::remove_dir(&p))
            };
            r.map_err(|e| {
                install_error(
                    "remove_failed",
                    "OpenFrame AI could not be removed completely. Close OpenFrame and try again.",
                    e.kind().to_string(),
                )
            })?;
        }
        Ok(())
    }

    /// Delete staging folders of other versions (an abandoned download of an older list).
    pub fn sweep_staging(&self, keep: &str) {
        for e in std::fs::read_dir(self.root.join("staging"))
            .into_iter()
            .flatten()
            .flatten()
        {
            if e.file_name().to_str() != Some(keep) {
                let p = e.path();
                if let Err(err) = std::fs::remove_dir_all(&p).or_else(|_| std::fs::remove_file(&p))
                {
                    tracing::warn!(kind = %err.kind(), "an old OpenFrame AI download could not be removed yet");
                }
            }
        }
    }

    /// The set the app uses: `active.json` names this app's pinned set `version`, and the
    /// sizes and hashes come from the pins in `tier`. `Ok(None)` when nothing is installed, or
    /// when the set on disk belongs to other pins (an app update with other models: it is
    /// downloaded again, and the old set is removed then). `Err(models_damaged)` when the pointer
    /// is unusable. The files themselves are checked by the caller (`BundledAi::verify`).
    pub fn installed(&self, version: &str, tier: &Tier) -> AppResult<Option<InstalledModels>> {
        if !self.root.join("active.json").exists() {
            return Ok(None);
        }
        let bad = |d: &str| models_damaged().with_detail(d.to_string());
        let a = self
            .active()
            .ok_or_else(|| bad("active.json is unreadable"))?;
        if a.version != version {
            return Ok(None);
        }
        let pick = |role: &str| {
            tier.files
                .iter()
                .find(|f| f.role == role)
                .map(|f| PinnedFile {
                    path: f.name.clone(),
                    bytes: f.bytes,
                    sha256: f.sha256.clone(),
                })
                .ok_or_else(|| bad("the pins lack a model"))
        };
        Ok(Some(InstalledModels {
            version: a.version.clone(),
            dir: self.set_dir(&a.version),
            chat: pick("chat")?,
            embedding: pick("embedding")?,
            // Pinned only by locks that carry the multilingual search model.
            multilingual: tier
                .files
                .iter()
                .any(|f| f.role == "multilingual_embedding")
                .then(|| pick("multilingual_embedding"))
                .transpose()?,
        }))
    }
}

impl ModelsDir {
    /// Manual import (ADR-0032 decision 5): a file the person picked is used only when its size
    /// and SHA-256 match a pinned model. It is copied, never moved, into staging and checked again
    /// there like a download. The person's own file is never changed or set aside.
    pub fn import_file(&self, version: &str, tier: &Tier, src: &Path) -> AppResult<FileEntry> {
        let not_model = |d: String| {
            install_error(
                "not_a_model",
                "This file is not an OpenFrame AI model file, so it was not used. Nothing was changed.",
                d,
            )
        };
        let meta = std::fs::symlink_metadata(src).map_err(|e| {
            install_error(
                "file_busy",
                "The chosen file could not be read. Nothing was changed.",
                e.kind().to_string(),
            )
        })?;
        if !meta.file_type().is_file() {
            return Err(not_model("not a regular file".into()));
        }
        let same_size: Vec<&FileEntry> = tier
            .files
            .iter()
            .filter(|f| f.bytes == meta.len())
            .collect();
        if same_size.is_empty() {
            return Err(not_model(format!(
                "{} bytes match no pinned model",
                meta.len()
            )));
        }
        let mut found = None;
        for f in same_size {
            let matches = file_matches(src, f).map_err(|e| {
                install_error(
                    "file_busy",
                    "The chosen file could not be read. Nothing was changed.",
                    e.kind().to_string(),
                )
            })?;
            if matches {
                found = Some(f);
                break;
            }
        }
        let f = found.ok_or_else(|| not_model("its SHA-256 matches no pinned model".into()))?;
        let write = |e: std::io::Error| {
            install_error(
                "write_failed",
                "OpenFrame AI could not be saved.",
                e.kind().to_string(),
            )
        };
        std::fs::create_dir_all(self.staging(version)).map_err(write)?;
        std::fs::copy(src, self.partial(version, f)).map_err(write)?;
        // The copy is hashed again: a file that changed while it was copied is refused in import
        // words, not download words.
        self.check_file(version, f).map_err(|e| {
            if e.is("ai.file_damaged") {
                not_model("the copy did not match; the file changed while it was copied".into())
            } else {
                e
            }
        })?;
        Ok(f.clone())
    }
}

/// Size and SHA-256 of a file against its pinned entry.
pub fn file_matches(path: &Path, f: &FileEntry) -> std::io::Result<bool> {
    if std::fs::metadata(path)?.len() != f.bytes {
        return Ok(false);
    }
    let mut h = Sha256::new();
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()) == f.sha256)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, body: &[u8], role: &str) -> FileEntry {
        FileEntry {
            name: name.into(),
            url: format!(
                "https://huggingface.co/o/r/resolve/{}/{name}",
                "a".repeat(40)
            ),
            bytes: body.len() as u64,
            sha256: hex::encode(Sha256::digest(body)),
            role: role.into(),
        }
    }

    fn tier(files: Vec<FileEntry>) -> Tier {
        Tier {
            id: "small".into(),
            files,
        }
    }

    fn put_partial(d: &ModelsDir, v: &str, f: &FileEntry, body: &[u8]) {
        std::fs::create_dir_all(d.staging(v)).unwrap();
        std::fs::write(d.partial(v, f), body).unwrap();
    }

    /// Every file of the tier is in the active set with its signed size.
    fn active_complete(d: &ModelsDir, t: &Tier) -> bool {
        let dir = d.set_dir(&d.active().unwrap().version);
        t.files
            .iter()
            .all(|f| std::fs::metadata(dir.join(&f.name)).is_ok_and(|m| m.len() == f.bytes))
    }

    #[test]
    fn preflight_counts_what_is_left_and_refuses_a_full_or_unknown_disk() {
        let dir = tempfile::tempdir().unwrap();
        let d = ModelsDir::new(dir.path());
        let chat = entry("chat.gguf", &[1u8; 1000], "chat");
        let t = tier(vec![
            chat.clone(),
            entry("embed.gguf", &[2u8; 500], "embedding"),
        ]);
        assert_eq!(d.remaining_bytes("v1", &t), 1500);
        put_partial(&d, "v1", &chat, &[1u8; 400]);
        assert_eq!(d.remaining_bytes("v1", &t), 1100);
        assert!(
            d.preflight("v1", &t, Some(PREFLIGHT_MARGIN_BYTES + 1100))
                .is_ok()
        );
        assert!(
            d.preflight("v1", &t, Some(PREFLIGHT_MARGIN_BYTES + 1099))
                .unwrap_err()
                .is("ai.disk_full")
        );
        assert!(
            d.preflight("v1", &t, None)
                .unwrap_err()
                .is("ai.disk_unknown")
        );
    }

    #[test]
    fn a_damaged_file_is_quarantined_and_never_becomes_usable() {
        let dir = tempfile::tempdir().unwrap();
        let d = ModelsDir::new(dir.path());
        let good = b"the real model bytes";
        let f = entry("chat.gguf", good, "chat");
        put_partial(&d, "v1", &f, b"the fake model bytes"); // same size, other content
        assert!(d.check_file("v1", &f).unwrap_err().is("ai.file_damaged"));
        assert!(!d.checked("v1", &f).exists() && !d.partial("v1", &f).exists());
        assert_eq!(
            std::fs::read_dir(dir.path().join("quarantine"))
                .unwrap()
                .count(),
            1
        );
        put_partial(&d, "v1", &f, good);
        assert!(d.check_file("v1", &f).is_ok());
        assert!(d.checked("v1", &f).is_file());
    }

    #[test]
    fn activation_is_all_or_nothing_keeps_the_last_good_set_and_rolls_back() {
        let dir = tempfile::tempdir().unwrap();
        let d = ModelsDir::new(dir.path());
        let (c1, e1) = (
            entry("chat.gguf", b"chat one", "chat"),
            entry("embed.gguf", b"embed one", "embedding"),
        );
        let t1 = tier(vec![c1.clone(), e1.clone()]);
        // Only one of two files checked: nothing is activated.
        put_partial(&d, "v1", &c1, b"chat one");
        d.check_file("v1", &c1).unwrap();
        assert!(d.activate("v1", &t1).unwrap_err().is("ai.incomplete"));
        assert!(d.active().is_none());
        put_partial(&d, "v1", &e1, b"embed one");
        d.check_file("v1", &e1).unwrap();
        let a = d.activate("v1", &t1).unwrap();
        assert_eq!((a.version.as_str(), a.previous.as_deref()), ("v1", None));
        assert!(active_complete(&d, &t1));
        // A second version becomes active; v1 stays as the fallback until the health check.
        let (c2, e2) = (
            entry("chat.gguf", b"chat two!", "chat"),
            entry("embed.gguf", b"embed two", "embedding"),
        );
        let t2 = tier(vec![c2.clone(), e2.clone()]);
        for (f, b) in [(&c2, &b"chat two!"[..]), (&e2, &b"embed two"[..])] {
            put_partial(&d, "v2", f, b);
            d.check_file("v2", f).unwrap();
        }
        let a = d.activate("v2", &t2).unwrap();
        assert_eq!(a.previous.as_deref(), Some("v1"));
        // The new set fails its health check: back to v1, the bad set is set aside.
        d.roll_back().unwrap();
        assert_eq!(d.active().unwrap().version, "v1");
        assert!(!d.set_dir("v2").exists());
        assert!(active_complete(&d, &t1));
        // Activate v2 again from a fresh download and confirm it: v1 goes away.
        for (f, b) in [(&c2, &b"chat two!"[..]), (&e2, &b"embed two"[..])] {
            put_partial(&d, "v2", f, b);
            d.check_file("v2", f).unwrap();
        }
        d.activate("v2", &t2).unwrap();
        d.confirm_healthy().unwrap();
        assert_eq!(d.active().unwrap().previous, None);
        assert!(!d.set_dir("v1").exists());
        // A file of the active set that went missing makes the set unusable, never half used.
        std::fs::remove_file(d.set_dir("v2").join("embed.gguf")).unwrap();
        assert!(!active_complete(&d, &t2));
        // Remove: nothing is left but the quarantine.
        d.remove_all().unwrap();
        assert!(d.active().is_none() && !d.set_dir("v2").exists());
    }

    #[test]
    fn a_pointer_naming_an_unsafe_folder_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("active.json"),
            br#"{"version":"..\\..\\Windows","tier":"small","previous":null,"activatedAt":0}"#,
        )
        .unwrap();
        assert!(ModelsDir::new(dir.path()).active().is_none());
        // A tampered `previous` is refused too, so a health check never removes a folder outside.
        std::fs::write(
            dir.path().join("active.json"),
            br#"{"version":"v1","tier":"small","previous":"..\\..\\Users","activatedAt":0}"#,
        )
        .unwrap();
        let d = ModelsDir::new(dir.path());
        assert!(d.active().is_none());
        d.confirm_healthy().unwrap();
        assert!(dir.path().parent().unwrap().exists());
    }

    #[test]
    fn the_set_in_use_is_the_pinned_one_and_another_set_counts_as_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let d = ModelsDir::new(dir.path());
        let t = tier(vec![
            entry("chat.gguf", b"chat", "chat"),
            entry("embed.gguf", b"embed", "embedding"),
        ]);
        assert_eq!(
            d.installed("m-1", &t).unwrap(),
            None,
            "nothing downloaded yet"
        );
        d.write_active(&Active {
            version: "m-1".into(),
            tier: "small".into(),
            previous: None,
            activated_at: 0,
        })
        .unwrap();
        let m = d.installed("m-1", &t).unwrap().unwrap();
        assert_eq!((m.chat.path.as_str(), m.embedding.bytes), ("chat.gguf", 5));
        assert_eq!(m.dir, d.set_dir("m-1"));
        // An app with other pins: the set on disk is not used (it is downloaded again).
        assert_eq!(d.installed("m-2", &t).unwrap(), None);
        // A pointer that does not parse is damage, never a quiet "nothing installed".
        std::fs::write(dir.path().join("active.json"), b"{nope").unwrap();
        assert!(d.installed("m-1", &t).unwrap_err().is("ai.models_damaged"));
    }

    #[test]
    fn an_imported_file_is_used_only_when_it_matches_a_pin_and_is_copied_not_moved() {
        let dir = tempfile::tempdir().unwrap();
        let d = ModelsDir::new(dir.path().join("models"));
        let (c, e) = (
            entry("chat.gguf", b"the chat model", "chat"),
            entry("embed.gguf", b"the embed model", "embedding"),
        );
        let t = tier(vec![c.clone(), e.clone()]);
        let mine = dir.path().join("my-copy.gguf");
        std::fs::write(&mine, b"the chat model").unwrap();
        let got = d.import_file("m-1", &t, &mine).unwrap();
        assert_eq!(got.name, "chat.gguf");
        assert!(d.checked("m-1", &c).is_file(), "ready for activation");
        assert!(mine.is_file(), "the person's file stays where it was");
        // Same size, other bytes: refused, and the person's file is not set aside.
        let wrong = dir.path().join("wrong.gguf");
        std::fs::write(&wrong, b"the CHAT model").unwrap();
        assert!(
            d.import_file("m-1", &t, &wrong)
                .unwrap_err()
                .is("ai.not_a_model")
        );
        assert!(wrong.is_file());
        assert!(!dir.path().join("models").join("quarantine").exists());
        // Another size, or a folder: refused.
        std::fs::write(&wrong, b"x").unwrap();
        assert!(
            d.import_file("m-1", &t, &wrong)
                .unwrap_err()
                .is("ai.not_a_model")
        );
        assert!(
            d.import_file("m-1", &t, dir.path())
                .unwrap_err()
                .is("ai.not_a_model")
        );
        // With the second file imported too, the set can be activated.
        let mine2 = dir.path().join("embed-copy.gguf");
        std::fs::write(&mine2, b"the embed model").unwrap();
        d.import_file("m-1", &t, &mine2).unwrap();
        assert_eq!(d.activate("m-1", &t).unwrap().version, "m-1");
    }
}

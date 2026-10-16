//! Download after install: what to fetch (ADR-0026, ADR-0032; Windows phase W8).
//!
//! The models come straight from their public Hugging Face repositories at the commit revisions
//! pinned in the compiled lock (`packaging/ai/ai-assets.lock.json`). The plan is built from the lock
//! alone: the address, the size and the SHA-256 of each file are fixed in the app, so nothing the
//! network says can change what is trusted. A file is used only when it matches its pins exactly.

use sha2::{Digest, Sha256};

use crate::bundle::{AssetLock, ModelRole};

/// The largest single model file the app will fetch.
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024 * 1024;

/// The files of one model set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tier {
    /// `small`: the models the lock describes (ADR-0028 decision 4; a larger tier comes with W6).
    pub id: String,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// The file name on this computer: one safe path component.
    pub name: String,
    /// The Hugging Face `resolve` address at the pinned commit.
    pub url: String,
    pub bytes: u64,
    pub sha256: String,
    /// `chat` or `embedding`.
    pub role: String,
}

impl Tier {
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.bytes).sum()
    }
}

/// The set the compiled lock pins: its version (named after the model hashes, so an app update
/// that keeps the same models keeps the download) and its files.
pub fn pins(lock: &AssetLock) -> (String, Tier) {
    let mut h = Sha256::new();
    let files: Vec<FileEntry> = lock
        .models
        .iter()
        .map(|m| {
            h.update(m.file.sha256.as_bytes());
            FileEntry {
                name: format!("{}.gguf", m.model_id),
                url: m.source.url.clone(),
                bytes: m.file.bytes,
                sha256: m.file.sha256.clone(),
                role: match m.role {
                    ModelRole::Chat => "chat",
                    ModelRole::Embedding => "embedding",
                }
                .into(),
            }
        })
        .collect();
    let version = format!("m-{}", &hex::encode(h.finalize())[..12]);
    (
        version,
        Tier {
            id: "small".into(),
            files,
        },
    )
}

/// A Hugging Face `resolve` address at a full commit: `https://huggingface.co/<org>/<repo>/resolve/
/// <40 hex>/<file>`, every part a safe name. Anything else is refused when the lock is read.
pub fn huggingface_url_ok(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://huggingface.co/") else {
        return false;
    };
    let parts: Vec<&str> = rest.split('/').collect();
    matches!(parts.as_slice(), [org, repo, "resolve", rev, file]
        if safe_name(org) && safe_name(repo) && safe_name(file)
            && rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()))
}

/// One path component made of safe characters: no folder separators, no `..`, no drive letters,
/// no control characters, no Windows device names.
pub fn safe_name(s: &str) -> bool {
    let stem = s.split('.').next().unwrap_or("").to_ascii_lowercase();
    let device = matches!(
        stem.as_str(),
        "con" | "prn" | "aux" | "nul" | "conin$" | "conout$"
    ) || ((stem.starts_with("com") || stem.starts_with("lpt"))
        && stem.len() == 4
        && stem.as_bytes()[3].is_ascii_digit());
    !s.is_empty()
        && s.len() <= 120
        && s != "."
        && s != ".."
        && !s.starts_with('.')
        && !s.ends_with('.')
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !device
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_comes_from_the_lock_and_its_version_from_the_model_hashes() {
        let lock = AssetLock::embedded().unwrap();
        let (v, t) = pins(&lock);
        assert!(v.starts_with("m-") && v.len() == 14 && safe_name(&v));
        assert_eq!(t.files.len(), 2);
        assert!(
            t.files
                .iter()
                .all(|f| huggingface_url_ok(&f.url) && safe_name(&f.name))
        );
        assert_eq!(
            t.total_bytes(),
            lock.models.iter().map(|m| m.file.bytes).sum::<u64>()
        );
        // Another runtime version with the same models keeps the same set.
        let mut other = lock.clone();
        other.bundle_version = "9.9.9".into();
        assert_eq!(pins(&other).0, v);
        let mut changed = lock.clone();
        changed.models[0].file.sha256 = "b".repeat(64);
        assert_ne!(pins(&changed).0, v);
    }

    #[test]
    fn only_pinned_hugging_face_addresses_are_accepted() {
        let rev = "f9c28bcd85737ffc5aef028638d3341d49869c27";
        assert!(huggingface_url_ok(&format!(
            "https://huggingface.co/ggml-org/gemma-3-1b-it-GGUF/resolve/{rev}/gemma-3-1b-it-Q8_0.gguf"
        )));
        for bad in [
            format!("http://huggingface.co/o/r/resolve/{rev}/m.gguf"),
            format!("https://huggingface.co.evil.example/o/r/resolve/{rev}/m.gguf"),
            "https://huggingface.co/o/r/resolve/main/m.gguf".to_string(),
            format!("https://huggingface.co/o/r/blob/{rev}/m.gguf"),
            format!("https://huggingface.co/o/../resolve/{rev}/m.gguf"),
            format!("https://huggingface.co/o/r/resolve/{rev}/sub/m.gguf"),
            format!(
                "https://huggingface.co/o/r/resolve/{}/m.gguf",
                rev.to_uppercase()
            ),
        ] {
            assert!(!huggingface_url_ok(&bad), "{bad}");
        }
    }

    #[test]
    fn names_are_single_safe_components() {
        for bad in [
            "con",
            "COM9",
            "lpt7.gguf",
            "..",
            ".x",
            "x.",
            "a/b",
            "a\\b",
            "a:b",
            "",
        ] {
            assert!(!safe_name(bad), "{bad}");
        }
        assert!(safe_name("gemma-3-1b-it-q8-0-v1.gguf"));
    }
}

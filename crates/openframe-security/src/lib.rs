//! Security primitives (Security Threat Model; ESD §13).
//!
//! - path confinement: every project-relative path is validated and can never
//!   escape its root (path traversal, absolute paths, UNC, drive letters,
//!   Windows reserved names, symlink escape);
//! - safe archive extraction (zip-slip, zip bombs, symlinks) — see [`archive`];
//! - streaming SHA-256 integrity hashing;
//! - Ed25519 signature verification for model/update manifests;
//! - log redaction as defence-in-depth (logs must never contain project content).

pub mod archive;

use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use base64::Engine;
use openframe_domain::{AppError, AppResult};
use sha2::{Digest, Sha256};

const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Turn arbitrary user text into a safe single file-name component.
/// Never returns an empty name, a reserved device name, or a name with separators.
pub fn sanitize_file_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    out = out.trim().trim_end_matches(['.', ' ']).to_string();
    while out.starts_with('.') {
        out.remove(0);
    }
    if out.chars().count() > 120 {
        out = out
            .chars()
            .take(120)
            .collect::<String>()
            .trim_end_matches(['.', ' '])
            .to_string();
    }
    let stem = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    if WINDOWS_RESERVED.contains(&stem.as_str()) {
        out = format!("_{out}");
    }
    if out.is_empty() {
        "Untitled".to_string()
    } else {
        out
    }
}

/// Validate a project-relative path (forward or back slashes) and return it as a
/// normalized relative `PathBuf`. Rejects anything that could escape the root.
pub fn validate_relative(rel: &str) -> AppResult<PathBuf> {
    let bad = || {
        AppError::security("path_rejected", "That file location isn't allowed.")
            .with_detail(format!("rejected path: {rel}"))
    };
    if rel.is_empty() || rel.contains('\0') || rel.starts_with("\\\\") || rel.starts_with("//") {
        return Err(bad());
    }
    let normalized = rel.replace('\\', "/");
    if normalized.starts_with('/') || normalized.chars().nth(1) == Some(':') {
        return Err(bad());
    }
    let mut out = PathBuf::new();
    for part in normalized.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(bad());
        }
        if part.contains(':') || sanitize_file_name(part) != part {
            return Err(bad());
        }
        out.push(part);
    }
    for c in out.components() {
        if !matches!(c, Component::Normal(_)) {
            return Err(bad());
        }
    }
    Ok(out)
}

/// Join a validated relative path onto `root`, refusing symlink escapes for
/// paths that already exist.
pub fn confine(root: &Path, rel: &str) -> AppResult<PathBuf> {
    let joined = root.join(validate_relative(rel)?);
    if joined.exists() {
        let canon_root = root.canonicalize()?;
        let canon = joined.canonicalize()?;
        if !canon.starts_with(&canon_root) {
            return Err(
                AppError::security("path_escape", "That file location isn't allowed.")
                    .with_detail("path resolves outside its root"),
            );
        }
    }
    Ok(joined)
}

/// True if `path` (after canonicalization) lies inside `root`.
pub fn is_within(root: &Path, path: &Path) -> bool {
    match (root.canonicalize(), path.canonicalize()) {
        (Ok(r), Ok(p)) => p.starts_with(r),
        _ => false,
    }
}

/// Validate a user-chosen file to read (import/attach): must exist, be a regular
/// file, and be within `max_bytes`.
pub fn validate_input_file(path: &Path, max_bytes: u64) -> AppResult<u64> {
    let meta = std::fs::metadata(path)?;
    if !meta.is_file() {
        return Err(AppError::invalid_input(
            "Please choose a file, not a folder.",
        ));
    }
    if meta.len() > max_bytes {
        return Err(AppError::invalid_input(format!(
            "That file is too large ({} MB). The limit for this operation is {} MB.",
            meta.len() / 1_048_576,
            max_bytes / 1_048_576
        )));
    }
    Ok(meta.len())
}

/// Streaming SHA-256 of a file (hex, lowercase).
pub fn sha256_file(path: &Path) -> AppResult<String> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Verify an Ed25519 signature (base64 key and signature) over `message`.
pub fn verify_ed25519(public_key_b64: &str, message: &[u8], signature_b64: &str) -> AppResult<()> {
    let fail = |d: &str| {
        AppError::security(
            "signature_invalid",
            "This download could not be verified, so OpenFrame will not use it.",
        )
        .with_detail(d.to_string())
    };
    let engine = base64::engine::general_purpose::STANDARD;
    let key_bytes = engine
        .decode(public_key_b64.trim())
        .map_err(|e| fail(&e.to_string()))?;
    let sig_bytes = engine
        .decode(signature_b64.trim())
        .map_err(|e| fail(&e.to_string()))?;
    let key_arr: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| fail("bad key length"))?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| fail("bad signature length"))?;
    let key =
        ed25519_dalek::VerifyingKey::from_bytes(&key_arr).map_err(|e| fail(&e.to_string()))?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);
    key.verify_strict(message, &sig)
        .map_err(|e| fail(&e.to_string()))
}

/// Defence-in-depth redaction for diagnostics text: user home paths, e-mail
/// addresses and bearer-like tokens are replaced. (Primary rule: never log content.)
pub fn redact(text: &str) -> String {
    use std::sync::OnceLock;
    static EMAIL: OnceLock<regex::Regex> = OnceLock::new();
    static TOKEN: OnceLock<regex::Regex> = OnceLock::new();
    static HOME: OnceLock<regex::Regex> = OnceLock::new();
    let email = EMAIL.get_or_init(|| {
        regex::Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap()
    });
    let token = TOKEN.get_or_init(|| {
        regex::Regex::new(r"(?i)(token|key|secret|password|authorization)([=:]\s*)\S+").unwrap()
    });
    let home = HOME.get_or_init(|| regex::Regex::new(r"(?i)[A-Z]:\\Users\\[^\\/]+").unwrap());
    let s = home.replace_all(text, r"%USERPROFILE%");
    let s = email.replace_all(&s, "[email]");
    token.replace_all(&s, "$1$2[redacted]").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_handles_hostile_names() {
        assert_eq!(sanitize_file_name("My: Film?"), "My_ Film_");
        assert_eq!(sanitize_file_name("CON"), "_CON");
        assert_eq!(sanitize_file_name("nul.txt"), "_nul.txt");
        assert_eq!(sanitize_file_name("   "), "Untitled");
        assert_eq!(sanitize_file_name("..\\..\\evil"), "_.._evil");
        assert_eq!(sanitize_file_name("trailing. "), "trailing");
    }

    #[test]
    fn relative_paths_cannot_escape() {
        for bad in [
            "../x",
            "a/../../b",
            "/etc/passwd",
            "C:\\Windows",
            "\\\\server\\share",
            "a//b",
            "a/./b",
            "a/CON",
            "a:b",
            "",
        ] {
            assert!(validate_relative(bad).is_err(), "{bad} should be rejected");
        }
        assert_eq!(
            validate_relative("assets/ab/cd.png").unwrap(),
            PathBuf::from("assets").join("ab").join("cd.png")
        );
        assert!(validate_relative("assets\\ab\\cd.png").is_ok());
    }

    #[test]
    fn confine_joins_inside_root() {
        let dir = tempfile::tempdir().unwrap();
        let p = confine(dir.path(), "assets/x.png").unwrap();
        assert!(p.starts_with(dir.path()));
        assert!(confine(dir.path(), "../x").is_err());
    }

    #[test]
    fn signatures_verify_and_reject_tampering() {
        use ed25519_dalek::Signer;
        let mut seed = [0u8; 32];
        rand::Rng::fill(&mut rand::thread_rng(), &mut seed);
        let sk = ed25519_dalek::SigningKey::from_bytes(&seed);
        let engine = base64::engine::general_purpose::STANDARD;
        let pk = engine.encode(sk.verifying_key().to_bytes());
        let msg = b"{\"manifestVersion\":1}";
        let sig = engine.encode(sk.sign(msg).to_bytes());
        assert!(verify_ed25519(&pk, msg, &sig).is_ok());
        assert!(verify_ed25519(&pk, b"{\"manifestVersion\":2}", &sig).is_err());
        assert!(verify_ed25519(&pk, msg, "not base64!").is_err());
    }

    #[test]
    fn redaction_removes_personal_data() {
        let r = redact(
            "open C:\\Users\\Ravi Kumar\\Documents failed for ravi@example.com token=abc123",
        );
        assert!(!r.contains("Ravi"));
        assert!(!r.contains("ravi@example.com"));
        assert!(!r.contains("abc123"));
    }

    #[test]
    fn hashing_is_stable() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.bin");
        std::fs::write(&f, b"abc").unwrap();
        assert_eq!(sha256_file(&f).unwrap(), sha256_bytes(b"abc"));
    }
}

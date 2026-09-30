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
    "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
    "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³", "CONIN$", "CONOUT$", "CLOCK$",
];

/// True if `name` (one path component) is a Windows device name such as `NUL`, `com1.txt`
/// or `CON .log` (Windows ignores trailing spaces/dots before the extension).
pub fn is_reserved_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("");
    let stem = stem.trim_end_matches([' ', '.']).to_uppercase();
    WINDOWS_RESERVED.contains(&stem.as_str())
}

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
    if is_reserved_name(&out) {
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

// ------------------------------------------------------------------ user-chosen paths
//
// Paths chosen in a file dialog arrive from the webview as strings. The webview is not
// trusted (threat T1/T4), and some paths come from project or package *content* (linked
// files). Everything is checked syntactically before the filesystem is touched, so a
// hostile path can't make Windows open a device (`\\.\PhysicalDrive0`, `\\.\pipe\x`,
// `\\?\GLOBALROOT\…`, `NUL`), an alternate data stream (`a.txt:secret`) or — where not
// wanted — connect to a network share (SMB/NTLM credential leak).

/// Whether a user-chosen path may point to a network share (`\\server\share\…`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkPaths {
    /// Files the user picked in a dialog: a NAS or share is a legitimate choice.
    Allow,
    /// Project locations (SQLite can't be shared safely) and paths from content.
    Refuse,
}

fn path_rejected(detail: impl Into<String>) -> AppError {
    AppError::security("path_rejected", "That file location isn't allowed.").with_detail(detail)
}

/// Syntactic validation of an absolute, user-chosen path. No filesystem access.
pub fn check_user_path(path: &Path, network: NetworkPaths) -> AppResult<()> {
    let raw = path.as_os_str().to_string_lossy();
    if raw.is_empty() || raw.len() > 32_000 || raw.chars().any(|c| c.is_control()) {
        return Err(path_rejected("empty, oversized or control characters"));
    }
    if !path.is_absolute() {
        return Err(AppError::invalid_input(
            "Choose the location with Browse… (a full path is needed).",
        ));
    }
    let mut components = path.components();
    #[cfg(windows)]
    {
        use std::path::Prefix;
        match components.next() {
            Some(Component::Prefix(p)) => match p.kind() {
                Prefix::Disk(_) | Prefix::VerbatimDisk(_) => {}
                Prefix::UNC(..) | Prefix::VerbatimUNC(..) if network == NetworkPaths::Allow => {}
                Prefix::UNC(..) | Prefix::VerbatimUNC(..) => {
                    return Err(AppError::new(
                        "project_format.network_location",
                        "OpenFrame can't use files on a network location here. Copy them to this computer first.",
                    )
                    .with_detail("network path refused"));
                }
                Prefix::Verbatim(_) | Prefix::DeviceNS(_) => {
                    return Err(path_rejected("device or verbatim namespace path"));
                }
            },
            _ => return Err(path_rejected("missing drive prefix")),
        }
    }
    for c in components {
        match c {
            Component::RootDir => {}
            Component::Normal(part) => {
                let part = part.to_string_lossy();
                if part.contains(':') || is_reserved_name(&part) {
                    return Err(path_rejected("reserved name or alternate data stream"));
                }
            }
            Component::ParentDir | Component::CurDir | Component::Prefix(_) => {
                return Err(path_rejected("relative component"));
            }
        }
    }
    Ok(())
}

/// `check_user_path` for a string (as received over IPC).
pub fn user_path(raw: &str, network: NetworkPaths) -> AppResult<PathBuf> {
    let p = PathBuf::from(raw.trim());
    check_user_path(&p, network)?;
    Ok(p)
}

/// True if `path` is a well-formed absolute path on a local (or mapped) drive — safe to probe
/// without triggering a network connection. Used for paths stored in content (linked files).
pub fn is_local_disk_path(path: &Path) -> bool {
    check_user_path(path, NetworkPaths::Refuse).is_ok()
}

/// Stored form of a canonicalized path: `\\?\C:\x` → `C:\x`, `\\?\UNC\srv\x` → `\\srv\x`.
pub fn display_path(canonical: &Path) -> PathBuf {
    let s = canonical.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        canonical.to_path_buf()
    }
}

/// File types Windows *executes* (or that execute code) when "opened" with the default
/// handler. Project files can come from received packages whose stored names/extensions
/// are attacker-chosen, so OpenFrame reveals these in Explorer instead of launching them.
const EXECUTABLE_EXTENSIONS: &[&str] = &[
    "exe",
    "com",
    "scr",
    "pif",
    "cpl",
    "msi",
    "msp",
    "mst",
    "msix",
    "msixbundle",
    "appx",
    "appxbundle",
    "appref-ms",
    "application",
    "bat",
    "cmd",
    "ps1",
    "psm1",
    "psd1",
    "ps1xml",
    "psc1",
    "vbs",
    "vbe",
    "js",
    "jse",
    "wsf",
    "wsh",
    "ws",
    "wsc",
    "hta",
    "lnk",
    "url",
    "scf",
    "reg",
    "inf",
    "ins",
    "isp",
    "jar",
    "dll",
    "sys",
    "ocx",
    "drv",
    "msc",
    "gadget",
    "diagcab",
    "settingcontent-ms",
    "library-ms",
    "search-ms",
    "searchconnector-ms",
    "vb",
    "sct",
    "shb",
    "shs",
    "xbap",
    "xll",
    "chm",
    "hlp",
    "iqy",
    "slk",
    "website",
    "theme",
    "themepack",
    "desktopthemepackfile",
    "mof",
    "sh",
    "py",
    "pyw",
    "rb",
    "pl",
    "vhd",
    "vhdx",
    "iso",
    "img",
];

/// True if opening `path` with the shell's default handler could run code.
pub fn is_dangerous_to_open(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    // Trailing dots/spaces are ignored by Windows ("a.exe." runs as "a.exe").
    let name = name.trim_end_matches(['.', ' ']);
    match name.rsplit_once('.') {
        Some((_, ext)) => EXECUTABLE_EXTENSIONS.contains(&ext),
        None => true, // no extension: Windows may still treat it as a program
    }
}

fn place_protected() -> AppError {
    AppError::invalid_input(
        "OpenFrame keeps its own files in that folder. Choose a different folder.",
    )
}

/// Destination file chosen in a save dialog (exports, packages, file copies): an absolute
/// file path with a portable name in an existing folder that is not inside any of
/// `protected` (open project folder, app data, Global Idea Vault). The folder is resolved
/// through symlinks/junctions before the check.
pub fn validate_output_file(path: &Path, protected: &[&Path]) -> AppResult<()> {
    check_user_path(path, NetworkPaths::Allow)?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::invalid_input("Choose a file name."))?;
    if sanitize_file_name(name) != name {
        return Err(AppError::invalid_input(
            "That file name can't be used on Windows. Choose a different name.",
        ));
    }
    if path.is_dir() {
        return Err(AppError::invalid_input("Choose a file name, not a folder."));
    }
    let parent = path.parent().filter(|p| p.is_dir()).ok_or_else(|| {
        AppError::new(
            "not_found.folder",
            "That folder can't be found. If it's on an external drive, reconnect it and try again.",
        )
    })?;
    ensure_outside(parent, protected)
}

/// A folder chosen to hold something new (project parent folder). Network locations are
/// refused (a project can't live on a share) and protected folders are refused.
pub fn validate_output_dir(dir: &Path, protected: &[&Path]) -> AppResult<()> {
    check_user_path(dir, NetworkPaths::Refuse)?;
    if dir.exists() {
        if !dir.is_dir() {
            return Err(AppError::invalid_input("Choose a folder."));
        }
        return ensure_outside(dir, protected);
    }
    // Not created yet: check the nearest existing ancestor.
    let mut anc = dir.parent();
    while let Some(a) = anc {
        if a.is_dir() {
            return ensure_outside(a, protected);
        }
        anc = a.parent();
    }
    Ok(())
}

fn ensure_outside(existing: &Path, protected: &[&Path]) -> AppResult<()> {
    let canon = existing.canonicalize()?;
    check_user_path(&display_path(&canon), NetworkPaths::Allow)?;
    for root in protected {
        if let Ok(r) = root.canonicalize()
            && canon.starts_with(&r)
        {
            return Err(place_protected());
        }
    }
    Ok(())
}

/// Validate a user-chosen file to read (import/attach): a well-formed absolute path
/// ([`check_user_path`], network shares allowed), an existing regular file after
/// resolving links (the link target is checked too), within `max_bytes`.
pub fn validate_input_file(path: &Path, max_bytes: u64) -> AppResult<u64> {
    check_user_path(path, NetworkPaths::Allow)?;
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        let target = path.canonicalize()?;
        check_user_path(&display_path(&target), NetworkPaths::Allow)?;
    }
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

/// A web link that may be handed to the OS browser (`of_open_url`): `http`/`https` only,
/// a non-empty host without user-info, no whitespace/control/quote characters (the value
/// goes to ShellExecute), at most 4096 characters. Returns the trimmed link.
pub fn web_link(raw: &str) -> Option<&str> {
    let url = raw.trim();
    if url.is_empty() || url.len() > 4096 {
        return None;
    }
    if url
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '"' | '<' | '>' | '\\' | '`'))
    {
        return None;
    }
    let lower = url.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit_once(':').map_or(authority, |(h, _)| h);
    if host.is_empty() || authority.contains('@') {
        return None;
    }
    Some(url)
}

/// Panic text for the log: redacted and truncated (panic messages can quote values).
pub fn panic_summary(message: &str, location: &str) -> String {
    let msg: String = redact(message).chars().take(300).collect();
    let loc = redact(location);
    // Paths of registry crates contain the build user's home; keep only the tail.
    let loc = loc
        .rsplit(['\\', '/'])
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("/");
    format!("panic at {loc}: {msg}")
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

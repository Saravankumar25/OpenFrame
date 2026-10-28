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

/// Join a validated relative path onto `root` and refuse a destination that resolves outside
/// the canonical root through a symlink or junction (Security baseline E3, link and junction safety).
///
/// The nearest existing entry on the way from the destination up to `root` is resolved: the
/// destination itself when it exists, otherwise the deepest existing folder (or link) above
/// it. So a *new* file below a junctioned `assets/` folder is refused just like an existing
/// one, and a dangling link (whose target a write would create) is refused too. A root that
/// is itself reached through a link stays valid: both sides are compared canonically.
pub fn confine(root: &Path, rel: &str) -> AppResult<PathBuf> {
    let joined = root.join(validate_relative(rel)?);
    ensure_resolves_within(root, &joined)?;
    Ok(joined)
}

fn path_escape() -> AppError {
    AppError::security("path_escape", "That file location isn't allowed.")
        .with_detail("path resolves outside its root")
}

/// `path` lies syntactically inside `root`; refuse it if its nearest existing ancestor
/// (links included, never followed for the existence test) resolves outside the canonical root.
fn ensure_resolves_within(root: &Path, path: &Path) -> AppResult<()> {
    let existing = path
        .ancestors()
        .take_while(|a| a.starts_with(root))
        .find(|a| std::fs::symlink_metadata(a).is_ok());
    // Nothing inside the root exists yet, so no link can redirect the write.
    let Some(existing) = existing else {
        return Ok(());
    };
    let canon_root = root.canonicalize().map_err(|_| path_escape())?;
    // A dangling link cannot be resolved: refuse rather than create its target.
    let canon = existing.canonicalize().map_err(|_| path_escape())?;
    if canon.starts_with(&canon_root) {
        Ok(())
    } else {
        Err(path_escape())
    }
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

/// Error code for a refused network location (UNC share or mapped network drive).
pub const NETWORK_LOCATION_CODE: &str = "project_format.network_location";

fn network_location_refused(detail: &str) -> AppError {
    AppError::new(
        NETWORK_LOCATION_CODE,
        "OpenFrame can't use files on a network location here. Copy them to this computer first.",
    )
    .with_detail(detail.to_string())
}

/// What the operating system reports for the volume behind a drive letter (platform boundary).
pub use openframe_platform::drives::{DriveKind, drive_kind};

/// Syntactic validation of an absolute, user-chosen path. The path itself is never opened;
/// with [`NetworkPaths::Refuse`] the drive letter's type is queried (Windows), or the volume
/// that holds the path's nearest existing folder (macOS), so a mapped network drive or a
/// mounted share is refused like a UNC path (network drive refusal).
pub fn check_user_path(path: &Path, network: NetworkPaths) -> AppResult<()> {
    check_user_path_with(path, network, drive_kind)
}

/// [`check_user_path`] with an injected drive classifier (tests simulate mapped drives).
fn check_user_path_with(
    path: &Path,
    network: NetworkPaths,
    classify: impl Fn(u8) -> DriveKind,
) -> AppResult<()> {
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
    // Where absolute paths carry a drive letter or a UNC prefix (Windows), the prefix decides.
    // Elsewhere `components()` never yields a prefix and the path starts at the root.
    if openframe_platform::drives::DRIVE_LETTERS {
        use std::path::Prefix;
        match components.next() {
            Some(Component::Prefix(p)) => match p.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                    if network == NetworkPaths::Refuse && classify(letter) == DriveKind::Remote {
                        return Err(network_location_refused("mapped network drive refused"));
                    }
                }
                Prefix::UNC(..) | Prefix::VerbatimUNC(..) if network == NetworkPaths::Allow => {}
                Prefix::UNC(..) | Prefix::VerbatimUNC(..) => {
                    return Err(network_location_refused("network path refused"));
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
    // Without drive letters a share is a mount anywhere in the tree (`/Volumes/<share>`).
    if network == NetworkPaths::Refuse
        && !openframe_platform::drives::DRIVE_LETTERS
        && openframe_platform::drives::is_network_volume(path)
    {
        return Err(network_location_refused("network volume refused"));
    }
    Ok(())
}

/// `check_user_path` for a string (as received over IPC).
pub fn user_path(raw: &str, network: NetworkPaths) -> AppResult<PathBuf> {
    let p = PathBuf::from(raw.trim());
    check_user_path(&p, network)?;
    Ok(p)
}

/// True if `path` is a well-formed absolute path on a local drive (not a share and not a
/// mapped network drive) — safe to probe without triggering a network connection. Used for
/// paths stored in content (linked files).
pub fn is_local_disk_path(path: &Path) -> bool {
    check_user_path(path, NetworkPaths::Refuse).is_ok()
}

fn is_unc_text(s: &str) -> bool {
    (s.starts_with(r"\\") && !s.starts_with(r"\\?\"))
        || s.starts_with(r"\\?\UNC\")
        || s.starts_with("//")
}

fn is_network_refusal(e: &AppError) -> bool {
    e.code_str() == NETWORK_LOCATION_CODE
}

/// True if a project may not live at `path` because it is, or resolves to, a network
/// location (SQLite WAL is unsafe on network file systems; network drive refusal):
///
/// 1. UNC syntax (`\\server\share`, `\\?\UNC\…`, `//server`) or a `\\.\` device path, decided
///    without touching the filesystem;
/// 2. a drive letter that Windows reports as a mapped network drive (`DRIVE_REMOTE`);
/// 3. for a well-formed local path only: its nearest existing ancestor, resolved through
///    junctions and symlinks, lies on a share or a mapped network drive.
///
/// Malformed paths return `false`; callers still run [`check_user_path`], which refuses them.
pub fn is_network_location(path: &Path) -> bool {
    is_network_location_with(path, drive_kind)
}

fn is_network_location_with(path: &Path, classify: impl Fn(u8) -> DriveKind + Copy) -> bool {
    if is_unc_text(&path.to_string_lossy()) {
        return true;
    }
    if let Err(e) = check_user_path_with(path, NetworkPaths::Refuse, classify) {
        return is_network_refusal(&e);
    }
    let Some(existing) = path
        .ancestors()
        .find(|a| std::fs::symlink_metadata(a).is_ok())
    else {
        return false;
    };
    match existing.canonicalize() {
        Ok(canon) => {
            let shown = display_path(&canon);
            is_unc_text(&shown.to_string_lossy())
                || check_user_path_with(&shown, NetworkPaths::Refuse, classify)
                    .is_err_and(|e| is_network_refusal(&e))
        }
        // A dangling link or no access: not provably a share; later checks refuse it.
        Err(_) => false,
    }
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

/// File types macOS runs, installs or follows when "opened" with the default handler: Terminal
/// and shell commands, installers, scripts and workflows, disk images, configuration profiles,
/// and location files that open any path or URL they name. Checked on every platform: on
/// Windows they only mean that a few more received files are shown instead of opened.
const MAC_EXECUTABLE_EXTENSIONS: &[&str] = &[
    "app",
    "command",
    "tool",
    "terminal",
    "term",
    "fileloc",
    "webloc",
    "inetloc",
    "afploc",
    "ftploc",
    "pkg",
    "mpkg",
    "scpt",
    "scptd",
    "applescript",
    "workflow",
    "action",
    "dmg",
    "sparseimage",
    "sparsebundle",
    "mobileconfig",
    "prefpane",
    "kext",
    "plugin",
    "bundle",
    "osax",
    "service",
    "saver",
    "shortcut",
    "dylib",
    "so",
    "zsh",
    "bash",
    "csh",
    "ksh",
    "fish",
    "jnlp",
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
        Some((_, ext)) => {
            EXECUTABLE_EXTENSIONS.contains(&ext) || MAC_EXECUTABLE_EXTENSIONS.contains(&ext)
        }
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
    ensure_outside(parent, protected, NetworkPaths::Allow)
}

/// A folder chosen to hold something new (project parent folder). Network locations are
/// refused (a project can't live on a share), also when a local folder resolves to one
/// through a junction or symlink, and protected folders are refused.
pub fn validate_output_dir(dir: &Path, protected: &[&Path]) -> AppResult<()> {
    check_user_path(dir, NetworkPaths::Refuse)?;
    if dir.exists() {
        if !dir.is_dir() {
            return Err(AppError::invalid_input("Choose a folder."));
        }
        return ensure_outside(dir, protected, NetworkPaths::Refuse);
    }
    // Not created yet: check the nearest existing ancestor.
    let mut anc = dir.parent();
    while let Some(a) = anc {
        if a.is_dir() {
            return ensure_outside(a, protected, NetworkPaths::Refuse);
        }
        anc = a.parent();
    }
    Ok(())
}

fn ensure_outside(existing: &Path, protected: &[&Path], network: NetworkPaths) -> AppResult<()> {
    let canon = existing.canonicalize()?;
    check_user_path(&display_path(&canon), network)?;
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

/// Redaction for a line printed by a helper process such as the AI engine. On top of
/// [`redact`] it removes text that follows a prompt or content label and every absolute
/// path except the app's own folders under `%USERPROFILE%\AppData` (or `~/Library` on the Mac),
/// because a verbose or failing engine can echo the user's prompt or a project folder.
pub fn redact_process_output(line: &str) -> String {
    use std::sync::OnceLock;
    static CONTENT: OnceLock<regex::Regex> = OnceLock::new();
    static WIN_PATH: OnceLock<regex::Regex> = OnceLock::new();
    static PROFILE_PATH: OnceLock<regex::Regex> = OnceLock::new();
    static UNIX_HOME: OnceLock<regex::Regex> = OnceLock::new();
    static UNIX_PATH: OnceLock<regex::Regex> = OnceLock::new();
    let content = CONTENT.get_or_init(|| {
        regex::Regex::new(
            r#"(?i)("?)\b(prompt|system_prompt|content|text|input|messages?|query|question|answer|completion)\b("?)(\s*[:=]\s*)\S.*$"#,
        )
        .unwrap()
    });
    // A drive path (also in the \\?\ form) or a UNC share, up to the next quote or space.
    let win_path = WIN_PATH.get_or_init(|| {
        regex::Regex::new(
            r#"(?i)(?:\\\\\?\\)?\b[A-Z]:[\\/][^\s"'<>|]*|\\\\[^\s"'<>|\\]+\\[^\s"'<>|]*"#,
        )
        .unwrap()
    });
    let profile_path = PROFILE_PATH
        .get_or_init(|| regex::Regex::new(r#"(?i)%USERPROFILE%([\\/][^\s"'<>|]*)?"#).unwrap());
    let unix_home =
        UNIX_HOME.get_or_init(|| regex::Regex::new(r"(?:/Users|/home)/[^/\s]+").unwrap());
    let unix_path = UNIX_PATH.get_or_init(|| {
        regex::Regex::new(
            r#"(^|[\s"'=(:])/(?:Users|home|Volumes|mnt|media|private|tmp|var|opt|srv)/[^\s"'<>|]*"#,
        )
        .unwrap()
    });
    let s = redact(line);
    let s = content.replace_all(&s, "$1$2$3$4[redacted]");
    let s = win_path.replace_all(&s, "[path]");
    let s = profile_path.replace_all(&s, |c: &regex::Captures<'_>| {
        let rest = c.get(1).map_or("", |m| m.as_str());
        let lower = rest.to_ascii_lowercase();
        if rest.is_empty() || lower.starts_with("\\appdata\\") || lower.starts_with("/appdata/") {
            c[0].to_string()
        } else {
            "%USERPROFILE%\\[path]".to_string()
        }
    });
    let s = unix_home.replace_all(&s, "~");
    let s = unix_path.replace_all(&s, "$1[path]");
    // `~/...` paths outside the app's Library folders name user content.
    static TILDE: OnceLock<regex::Regex> = OnceLock::new();
    let tilde = TILDE.get_or_init(|| regex::Regex::new(r#"(^|[\s"'=(:])~/([^\s"'<>|]*)"#).unwrap());
    tilde
        .replace_all(&s, |c: &regex::Captures<'_>| {
            if c[2].starts_with("Library/") {
                c[0].to_string()
            } else {
                format!("{}~/[path]", &c[1])
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {

    #[test]
    fn mac_files_that_run_or_open_other_things_are_shown_not_opened() {
        for name in [
            "Run me.command",
            "x.terminal",
            "home.fileloc",
            "site.webloc",
            "Setup.pkg",
            "do.scpt",
            "Auto.workflow",
            "Disk.dmg",
            "wifi.mobileconfig",
            "Tool.app",
            "LIB.DYLIB",
        ] {
            assert!(is_dangerous_to_open(Path::new(name)), "{name}");
        }
        for name in [
            "script.fdx",
            "notes.txt",
            "board.png",
            "draft.pdf",
            "a.fountain",
        ] {
            assert!(!is_dangerous_to_open(Path::new(name)), "{name}");
        }
    }
    use super::*;

    #[test]
    fn process_output_loses_prompt_text_and_project_paths() {
        let cases = [
            (
                "slot launch_slot_: prompt: He places the red folder",
                "slot launch_slot_: prompt: [redacted]",
            ),
            (
                r#"request body {"prompt": "INT. STATION - NIGHT", "n_predict": 5}"#,
                r#"request body {"prompt": [redacted]"#,
            ),
            (
                r"error: cannot open D:\Films\Railway.openframe\cache",
                "error: cannot open [path]",
            ),
            (
                r"open \\?\D:\Films\Railway.openframe failed",
                "open [path] failed",
            ),
            (r"read \\nas\share\Railway.openframe", "read [path]"),
            (
                r"load C:\Users\Asha\Documents\Railway.openframe\db",
                r"load %USERPROFILE%\[path]",
            ),
            (
                r"model C:\Users\Asha\AppData\Local\OpenFrame\models\chat.gguf",
                r"model %USERPROFILE%\AppData\Local\OpenFrame\models\chat.gguf",
            ),
            (
                "error: cannot open /Users/asha/Movies/Railway.openframe",
                "error: cannot open ~/[path]",
            ),
            (
                "model /Users/asha/Library/Containers/x/chat.gguf",
                "model ~/Library/Containers/x/chat.gguf",
            ),
            ("open /Volumes/Card/Railway.openframe", "open [path]"),
            // Telugu and Hindi prompt text is removed the same way.
            ("prompt = రైలు స్టేషన్ रात", "prompt = [redacted]"),
        ];
        for (input, expected) in cases {
            assert_eq!(redact_process_output(input), expected, "{input}");
        }
    }

    #[test]
    fn process_output_keeps_engine_statistics() {
        for line in [
            "prompt eval time =      12.50 ms /     5 tokens",
            "n_prompt_tokens_processed = 5",
            "srv  update_slots: all slots are idle",
            "main: server is listening on http://127.0.0.1:8080",
        ] {
            assert_eq!(redact_process_output(line), line);
        }
    }

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

    /// network drive refusal: the drive-type decision with an injected classifier (no real mapped drive needed).
    #[cfg(windows)]
    #[test]
    fn mapped_network_drives_are_refused_for_projects_and_content() {
        let z_is_remote = |l: u8| {
            if l.eq_ignore_ascii_case(&b'Z') {
                DriveKind::Remote
            } else {
                DriveKind::Other
            }
        };
        for mapped in [
            r"Z:\Films\Railway.openframe",
            r"z:\Films\Railway.openframe",
            r"\\?\Z:\Films\Railway.openframe",
        ] {
            let p = Path::new(mapped);
            let err = check_user_path_with(p, NetworkPaths::Refuse, z_is_remote).unwrap_err();
            assert_eq!(err.code_str(), NETWORK_LOCATION_CODE, "{mapped}");
            assert!(is_network_location_with(p, z_is_remote), "{mapped}");
            // Files picked in a dialog may still come from a mapped drive (doc 20 T4a).
            assert!(check_user_path_with(p, NetworkPaths::Allow, z_is_remote).is_ok());
        }
        // Local drives are untouched by the classifier.
        let local = Path::new(r"C:\Films\Railway.openframe");
        assert!(check_user_path_with(local, NetworkPaths::Refuse, z_is_remote).is_ok());
        assert!(!is_network_location_with(local, z_is_remote));
        // Without the drive-type check the mapped drive would pass (regression guard).
        let all_local = |_: u8| DriveKind::Other;
        assert!(
            check_user_path_with(Path::new(r"Z:\Films"), NetworkPaths::Refuse, all_local).is_ok()
        );
    }

    #[test]
    fn unc_text_is_a_network_location_without_filesystem_access() {
        let never = |_: u8| -> DriveKind { panic!("syntax decides UNC paths") };
        for unc in [
            r"\\server\share\Film.openframe",
            r"\\?\UNC\server\share\Film.openframe",
            "//server/share/Film.openframe",
            r"\\.\pipe\openframe",
        ] {
            assert!(is_network_location_with(Path::new(unc), never), "{unc}");
        }
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

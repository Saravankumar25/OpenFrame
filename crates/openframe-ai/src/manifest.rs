//! Signed model/runtime distribution manifest (Local AI Runtime spec §4, §6).
//!
//! The manifest is the ONLY place that names concrete model files, runtime
//! builds, URLs and hashes. Everything else in OpenFrame refers to simple
//! profile tiers (Lightweight / Recommended / High Quality).
//!
//! Trust model: the manifest bytes are signed with Ed25519. The public key is
//! compiled into the application; TLS alone is not trusted. A production build
//! overrides the development key via the `OPENFRAME_MANIFEST_PUBLIC_KEY`
//! build-time environment variable (release blocker: public builds must set
//! it; manifests are signed with `crates/openframe-ai/tools/sign-manifest.mjs`).

use openframe_domain::{AppError, AppResult};
use serde::{Deserialize, Serialize};

/// Development manifest signing key (public half only; the private key is not
/// part of the repository).
pub const DEV_PUBLIC_KEY_B64: &str = include_str!("../keys/manifest-dev.pub");
/// Development manifest shipped with the application (used when no newer
/// verified manifest is cached and no distribution endpoint is configured).
pub const EMBEDDED_MANIFEST: &[u8] = include_bytes!("../manifest/dev-manifest.json");
pub const EMBEDDED_MANIFEST_SIG: &str = include_str!("../manifest/dev-manifest.json.sig");

/// Build-time configured distribution endpoint (spec §6 `MODEL_DISTRIBUTION_BASE_URL`).
pub const DISTRIBUTION_BASE_URL: Option<&str> = option_env!("MODEL_DISTRIBUTION_BASE_URL");
/// Build-time production signing key (base64 Ed25519 public key).
const RELEASE_PUBLIC_KEY_B64: Option<&str> = option_env!("OPENFRAME_MANIFEST_PUBLIC_KEY");

/// The key manifests must be signed with in this build.
pub fn trusted_public_key() -> &'static str {
    RELEASE_PUBLIC_KEY_B64.unwrap_or(DEV_PUBLIC_KEY_B64).trim()
}

/// True when this build still trusts the development key (never for a public release).
pub fn using_development_key() -> bool {
    RELEASE_PUBLIC_KEY_B64.is_none()
}

pub const SUPPORTED_MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    Lightweight,
    Recommended,
    HighQuality,
}

impl Tier {
    /// The simple, user-facing name (never a quantization code).
    pub fn label(self) -> &'static str {
        match self {
            Tier::Lightweight => "Lightweight",
            Tier::Recommended => "Recommended",
            Tier::HighQuality => "High Quality",
        }
    }
    pub fn rank(self) -> u8 {
        match self {
            Tier::Lightweight => 0,
            Tier::Recommended => 1,
            Tier::HighQuality => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Backend {
    /// Runs on the processor only.
    Cpu,
    /// Uses the graphics card through Vulkan (falls back to the processor for layers that don't fit).
    Vulkan,
}

impl Backend {
    pub fn label(self) -> &'static str {
        match self {
            Backend::Cpu => "Processor",
            Backend::Vulkan => "Graphics card",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeEntry {
    pub runtime_id: String,
    pub engine: String,
    pub version: String,
    pub backend: Backend,
    pub os: String,
    pub arch: String,
    pub url: String,
    pub bytes: u64,
    pub sha256: String,
    /// Only "zip" is supported.
    pub archive: String,
    /// Server executable inside the archive (relative path).
    pub executable: String,
    pub license_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelEntry {
    pub profile_id: String,
    pub tier: Tier,
    pub display_name: String,
    pub engine: String,
    pub architecture: String,
    pub bytes: u64,
    pub sha256: String,
    pub url: String,
    /// Below this much RAM the profile is not offered as suitable.
    pub min_ram_bytes: u64,
    /// RAM for comfortable CPU inference.
    pub recommended_ram_bytes: u64,
    /// Graphics memory needed to run fully on the graphics card.
    pub gpu_vram_bytes: u64,
    pub context_tokens: u32,
    pub license_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub manifest_version: u32,
    /// Monotonic publication number; a cached/remote manifest with a lower
    /// sequence never replaces a newer one (rollback protection).
    pub sequence: u64,
    pub channel: String,
    pub issued_at: String,
    pub runtimes: Vec<RuntimeEntry>,
    pub models: Vec<ModelEntry>,
}

impl Manifest {
    pub fn model(&self, profile_id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.profile_id == profile_id)
    }
    pub fn model_for_tier(&self, tier: Tier) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.tier == tier)
    }
    pub fn runtime(&self, runtime_id: &str) -> Option<&RuntimeEntry> {
        self.runtimes.iter().find(|r| r.runtime_id == runtime_id)
    }
    /// The runtime build for this machine and backend.
    pub fn runtime_for(&self, backend: Backend) -> Option<&RuntimeEntry> {
        let arch = std::env::consts::ARCH;
        let os = std::env::consts::OS;
        self.runtimes
            .iter()
            .find(|r| r.backend == backend && r.arch == arch && r.os == os)
    }
}

fn invalid(detail: impl Into<String>) -> AppError {
    AppError::security(
        "manifest_invalid",
        "The Offline AI download information could not be verified, so OpenFrame will not use it.",
    )
    .with_detail(detail)
}

/// Identifiers become directory names (`models/<profile_id>`, `runtimes/llama/<runtime_id>`),
/// so they must be a single, portable, non-reserved path component.
pub fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        && !id.starts_with('.')
        && !id.ends_with('.')
        && !id.contains("..")
        // Windows device names (`con`, `nul`, `com1.gguf` …) can't be used as folder names.
        && openframe_security::sanitize_file_name(id) == id
}

fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// Downloads must use HTTPS. Plain HTTP is accepted only for loopback mirrors
/// (local test servers / an on-machine mirror), never for remote hosts.
///
/// The URL is parsed properly (not split on `/` and `:`), so tricks such as
/// `http://127.0.0.1:80@evil.example/x` (userinfo) are recognised as the remote host they are.
pub fn valid_url(url: &str) -> bool {
    if url.len() > 2048
        || url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
    {
        return false;
    }
    // The WHATWG parser "repairs" `https:///host` into `https://host/`; require the host
    // to follow `://` literally.
    if url
        .split_once("://")
        .is_none_or(|(_, rest)| rest.starts_with('/'))
    {
        return false;
    }
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return false;
    };
    download_url_allowed(&parsed)
}

/// Scheme/host policy for every download request, including each redirect hop.
pub fn download_url_allowed(url: &reqwest::Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let Some(host) = url.host_str().filter(|h| !h.is_empty()) else {
        return false;
    };
    match url.scheme() {
        "https" => true,
        "http" => is_loopback_host(host),
        _ => false,
    }
}

/// `host_str()` of a parsed URL: IPv6 literals keep their brackets.
fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if let Some(v6) = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        return v6
            .parse::<std::net::Ipv6Addr>()
            .is_ok_and(|ip| ip.is_loopback());
    }
    host.parse::<std::net::Ipv4Addr>()
        .is_ok_and(|ip| ip.is_loopback())
}

/// Redirect policy for the download/manifest client: at most 5 hops, and every hop must itself
/// satisfy [`download_url_allowed`] (no downgrade from HTTPS to HTTP, no `file:` or other schemes).
/// Integrity never depends on this (every artifact is SHA-256 verified), but it keeps download
/// metadata off plain-text connections.
pub fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 5 {
            attempt.error("too many redirects")
        } else if download_url_allowed(attempt.url()) {
            attempt.follow()
        } else {
            attempt.stop()
        }
    })
}

/// Verify the signature over the exact bytes, then parse and validate.
pub fn verify_and_parse(
    bytes: &[u8],
    signature_b64: &str,
    public_key_b64: &str,
) -> AppResult<Manifest> {
    openframe_security::verify_ed25519(public_key_b64, bytes, signature_b64)?;
    let manifest: Manifest =
        serde_json::from_slice(bytes).map_err(|e| invalid(format!("manifest parse: {e}")))?;
    validate(&manifest)?;
    Ok(manifest)
}

pub fn validate(m: &Manifest) -> AppResult<()> {
    if m.manifest_version != SUPPORTED_MANIFEST_VERSION {
        return Err(invalid(format!(
            "unsupported manifest version {}",
            m.manifest_version
        )));
    }
    if m.models.is_empty() || m.runtimes.is_empty() {
        return Err(invalid("manifest lists no models or runtimes"));
    }
    for r in &m.runtimes {
        if !safe_id(&r.runtime_id)
            || !valid_sha(&r.sha256)
            || !valid_url(&r.url)
            || r.bytes == 0
            || r.archive != "zip"
        {
            return Err(invalid(format!(
                "runtime entry {} is invalid",
                r.runtime_id
            )));
        }
        if openframe_security::validate_relative(&r.executable).is_err()
            || !r.executable.to_ascii_lowercase().ends_with(".exe")
        {
            return Err(invalid(format!(
                "runtime entry {} has an unsafe executable path",
                r.runtime_id
            )));
        }
        if !safe_id(&r.version) {
            return Err(invalid(format!(
                "runtime entry {} has an unsafe version",
                r.runtime_id
            )));
        }
    }
    let mut seen = std::collections::HashSet::new();
    for md in &m.models {
        if !safe_id(&md.profile_id)
            || !valid_sha(&md.sha256)
            || !valid_url(&md.url)
            || md.bytes == 0
        {
            return Err(invalid(format!("model entry {} is invalid", md.profile_id)));
        }
        if !seen.insert(md.profile_id.clone()) {
            return Err(invalid(format!("duplicate profile {}", md.profile_id)));
        }
        if md.context_tokens < 1024 {
            return Err(invalid(format!(
                "model entry {} has too small a context",
                md.profile_id
            )));
        }
    }
    Ok(())
}

/// The manifest compiled into this build (verified with the trusted key).
pub fn embedded() -> AppResult<Manifest> {
    verify_and_parse(
        EMBEDDED_MANIFEST,
        EMBEDDED_MANIFEST_SIG,
        trusted_public_key(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_manifest_verifies_with_compiled_key() {
        let m = embedded().expect("embedded dev manifest must verify");
        assert_eq!(m.manifest_version, 1);
        for tier in [Tier::Lightweight, Tier::Recommended, Tier::HighQuality] {
            assert!(m.model_for_tier(tier).is_some(), "{tier:?} profile present");
        }
        assert!(m.runtime_for(Backend::Cpu).is_some() || std::env::consts::ARCH != "x86_64");
    }

    #[test]
    fn tampered_embedded_manifest_is_rejected() {
        let mut bytes = EMBEDDED_MANIFEST.to_vec();
        // Flip one character of a hash — the classic "swap the download" attack.
        let pos = bytes.windows(8).position(|w| w == b"\"sha256\"").unwrap() + 12;
        bytes[pos] = if bytes[pos] == b'a' { b'b' } else { b'a' };
        let err =
            verify_and_parse(&bytes, EMBEDDED_MANIFEST_SIG, trusted_public_key()).unwrap_err();
        assert_eq!(err.code_str(), "security.signature_invalid");
    }

    #[test]
    fn urls_must_be_https_or_loopback() {
        assert!(valid_url("https://example.com/x.gguf"));
        assert!(valid_url("http://127.0.0.1:8080/x"));
        assert!(!valid_url("http://example.com/x"));
        assert!(!valid_url("file:///C:/x"));
        assert!(!valid_url("ftp://x"));
    }

    #[test]
    fn url_userinfo_cannot_disguise_a_remote_http_host() {
        // Regression (SEC-2026-09-30 AI-04): the old check split on '/' and ':'.
        assert!(!valid_url("http://127.0.0.1:80@evil.example/x"));
        assert!(!valid_url("http://localhost@evil.example/x"));
        assert!(!valid_url("https://user:pw@example.com/x"));
        assert!(!valid_url("http://127.0.0.1.evil.example/x"));
        assert!(!valid_url("https:///no-host"));
        assert!(!valid_url("https://exa mple.com/x"));
        assert!(valid_url("http://[::1]:9/x"));
        assert!(valid_url("http://localhost:9/x"));
    }

    #[test]
    fn ids_are_single_safe_path_components() {
        for bad in [
            "con", "nul", "com1", "aux.gguf", "a..b", "x.", ".x", "", "C:", "a/b",
        ] {
            assert!(!safe_id(bad), "{bad} must be rejected");
        }
        assert!(safe_id("qwen3-4b-q4.k.m"));
    }
}

//! Download after install, step 1: the signed manifest (ADR-0026 decisions 4 and 5; Windows phase W8).
//!
//! The model host serves one small file, the manifest envelope:
//!
//! ```json
//! { "format": "openframe-ai-manifest", "keyId": "...", "payload": "<base64>", "signature": "<base64>" }
//! ```
//!
//! `signature` is Ed25519 over the exact `payload` bytes, made with the owner's offline private key;
//! the app holds only the public key, compiled in. TLS alone is never trusted: nothing in the payload
//! is read until the signature checks out against a known key. The payload lists the model tiers
//! (each with its chat and embedding model files: name, relative path on the host, size, SHA-256) and
//! the engine runtimes they need. Every field is bounded and every name is a single safe file name.

use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};
use openframe_domain::{AppError, AppResult};
use serde::Deserialize;

/// The envelope is small; anything larger is refused before it is parsed.
pub const MAX_ENVELOPE_BYTES: usize = 256 * 1024;
/// The largest single file a manifest may describe.
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAX_TIERS: usize = 4;
const MAX_FILES_PER_TIER: usize = 8;
const MAX_RUNTIMES: usize = 8;
pub const ENVELOPE_FORMAT: &str = "openframe-ai-manifest";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    format: String,
    key_id: String,
    payload: String,
    signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub manifest_version: String,
    /// Raised with every list the owner signs. The app refuses a list older than the set it uses,
    /// so an old signed list replayed by a host or network cannot take it back to older models.
    pub sequence: u32,
    pub profile_version: u32,
    pub tiers: Vec<Tier>,
    #[serde(default)]
    pub runtimes: Vec<RuntimeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tier {
    /// `small` (the default for everyone) or `large` (16 GB of memory or a suitable card).
    pub id: String,
    pub label: String,
    pub min_memory_gb: u32,
    /// The engine version this tier needs (the runtime ships in the package).
    pub requires_runtime: String,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileEntry {
    /// The file name on this computer: one safe path component.
    pub name: String,
    /// Where it is on the host, relative to the configured base address.
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    /// `chat`, `embedding` or `tokenizer`.
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeRef {
    pub id: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub backend: String,
}

fn refused(detail: impl Into<String>) -> AppError {
    AppError::ai(
        "manifest_invalid",
        "The OpenFrame AI download list could not be checked, so nothing was downloaded. Try again later.",
    )
    .with_detail(detail.into())
}

/// Check the envelope's signature against the compiled-in keys, then read and validate the payload.
pub fn verify(envelope: &[u8], keys: &[VerifyingKey]) -> AppResult<Manifest> {
    if envelope.len() > MAX_ENVELOPE_BYTES {
        return Err(refused("the manifest is larger than allowed"));
    }
    let env: Envelope =
        serde_json::from_slice(envelope).map_err(|e| refused(format!("envelope: {e}")))?;
    if env.format != ENVELOPE_FORMAT || env.key_id.len() > 64 {
        return Err(refused("unknown manifest format"));
    }
    let b64 = base64::engine::general_purpose::STANDARD;
    let payload = b64
        .decode(env.payload.as_bytes())
        .map_err(|_| refused("payload is not base64"))?;
    let sig_bytes: [u8; 64] = b64
        .decode(env.signature.as_bytes())
        .ok()
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| refused("signature is not 64 bytes"))?;
    let sig = Signature::from_bytes(&sig_bytes);
    if !keys.iter().any(|k| k.verify_strict(&payload, &sig).is_ok()) {
        return Err(refused("the signature does not match a known key"));
    }
    let m: Manifest =
        serde_json::from_slice(&payload).map_err(|e| refused(format!("payload: {e}")))?;
    validate(&m)?;
    Ok(m)
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

/// Names the models folder uses for itself: a set version or a downloaded file may never take
/// one of them (`install.rs`).
fn reserved_in_models_folder(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    matches!(
        l.as_str(),
        "staging" | "quarantine" | "active.json" | "set.json"
    ) || l.ends_with(".partial")
        || l.ends_with(".bad")
        || l.ends_with(".tmp")
}

/// A relative path on the host made of safe components.
fn safe_rel_path(s: &str) -> bool {
    !s.is_empty() && s.len() <= 400 && s.split('/').all(safe_name)
}

fn validate(m: &Manifest) -> AppResult<()> {
    if !safe_name(&m.manifest_version) || reserved_in_models_folder(&m.manifest_version) {
        return Err(refused("manifest version"));
    }
    if m.tiers.is_empty() || m.tiers.len() > MAX_TIERS || m.runtimes.len() > MAX_RUNTIMES {
        return Err(refused("tier or runtime count"));
    }
    let mut tier_ids = std::collections::HashSet::new();
    for t in &m.tiers {
        if !matches!(t.id.as_str(), "small" | "large") || !tier_ids.insert(t.id.clone()) {
            return Err(refused(format!("tier id {}", t.id)));
        }
        if t.label.is_empty()
            || t.label.len() > 60
            || !safe_name(&t.requires_runtime)
            || t.min_memory_gb > 512
        {
            return Err(refused(format!("tier {}", t.id)));
        }
        if t.files.is_empty() || t.files.len() > MAX_FILES_PER_TIER {
            return Err(refused(format!("tier {} file count", t.id)));
        }
        let mut names = std::collections::HashSet::new();
        let (mut chat, mut emb) = (0, 0);
        for f in &t.files {
            let hex_ok = f.sha256.len() == 64
                && f.sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase());
            if !safe_name(&f.name)
                || reserved_in_models_folder(&f.name)
                || !names.insert(f.name.to_ascii_lowercase())
                || !safe_rel_path(&f.path)
                || f.bytes == 0
                || f.bytes > MAX_FILE_BYTES
                || !hex_ok
            {
                return Err(refused(format!("file {}", f.name)));
            }
            match f.role.as_str() {
                "chat" => chat += 1,
                "embedding" => emb += 1,
                "tokenizer" => {}
                _ => return Err(refused(format!("file {} role {}", f.name, f.role))),
            }
        }
        if chat != 1 || emb != 1 {
            return Err(refused(format!(
                "tier {} needs one chat and one embedding model",
                t.id
            )));
        }
    }
    for r in &m.runtimes {
        if !safe_name(&r.id)
            || !safe_name(&r.version)
            || !matches!(r.backend.as_str(), "cpu" | "vulkan" | "metal")
        {
            return Err(refused(format!("runtime {}", r.id)));
        }
    }
    Ok(())
}

impl Manifest {
    pub fn tier(&self, id: &str) -> Option<&Tier> {
        self.tiers.iter().find(|t| t.id == id)
    }
}

impl Tier {
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.bytes).sum()
    }
}

/// The tier the hardware check proposes (ADR-0028 decision 4): the larger tier on a computer with
/// enough memory for it, or with a graphics card that has 6 GB of its own; otherwise the small one.
/// The person can always choose the other tier.
pub fn propose_tier(m: &Manifest, memory_gb: u32, best_card_mb: u64) -> Option<&Tier> {
    let large = m
        .tier("large")
        .filter(|t| memory_gb >= t.min_memory_gb.max(16) || best_card_mb >= 6 * 1024);
    large
        .or_else(|| m.tier("small"))
        .or_else(|| m.tiers.first())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    /// A fixed test key: never the production key, which is generated and kept offline by the owner.
    pub(crate) fn test_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    pub(crate) fn payload() -> serde_json::Value {
        let file = |name: &str, role: &str, bytes: u64| json!({ "name": name, "path": format!("models/2026.11.1/{name}"), "bytes": bytes, "sha256": "a".repeat(64), "role": role });
        json!({
            "manifestVersion": "2026.11.1",
            "sequence": 3,
            "profileVersion": 2,
            "tiers": [
                { "id": "small", "label": "Standard", "minMemoryGb": 8, "requiresRuntime": "b11259", "files": [file("chat.gguf", "chat", 1_069_000_000), file("embed.gguf", "embedding", 37_000_000)] },
                { "id": "large", "label": "Larger", "minMemoryGb": 16, "requiresRuntime": "b11259", "files": [file("chat-4b.gguf", "chat", 4_100_000_000), file("embed.gguf", "embedding", 37_000_000)] }
            ],
            "runtimes": [{ "id": "llama-cpp-b11259-win-x64-vulkan", "version": "b11259", "os": "windows", "arch": "x86_64", "backend": "vulkan" }]
        })
    }

    pub(crate) fn envelope(payload: &serde_json::Value, key: &SigningKey) -> Vec<u8> {
        let bytes = serde_json::to_vec(payload).unwrap();
        let b64 = base64::engine::general_purpose::STANDARD;
        serde_json::to_vec(&json!({ "format": ENVELOPE_FORMAT, "keyId": "test", "payload": b64.encode(&bytes), "signature": b64.encode(key.sign(&bytes).to_bytes()) })).unwrap()
    }

    #[test]
    fn a_signed_manifest_is_read_and_its_tiers_are_offered() {
        let k = test_key();
        let m = verify(&envelope(&payload(), &k), &[k.verifying_key()]).unwrap();
        assert_eq!(m.manifest_version, "2026.11.1");
        assert_eq!(m.tier("small").unwrap().total_bytes(), 1_106_000_000);
        assert_eq!(propose_tier(&m, 8, 0).unwrap().id, "small");
        assert_eq!(propose_tier(&m, 16, 0).unwrap().id, "large");
        assert_eq!(propose_tier(&m, 8, 8192).unwrap().id, "large");
        assert_eq!(propose_tier(&m, 15, 4096).unwrap().id, "small");
    }

    #[test]
    fn a_wrong_key_a_changed_payload_or_a_broken_envelope_is_refused() {
        let k = test_key();
        let other = SigningKey::from_bytes(&[9u8; 32]);
        let good = envelope(&payload(), &k);
        assert!(
            verify(&good, &[other.verifying_key()]).is_err(),
            "signed by another key"
        );
        assert!(verify(&envelope(&payload(), &other), &[k.verifying_key()]).is_err());
        // The payload changed after signing (one size made larger): the signature no longer matches.
        let mut env: serde_json::Value = serde_json::from_slice(&good).unwrap();
        let mut p = payload();
        p["tiers"][0]["files"][0]["bytes"] = json!(999);
        let b64 = base64::engine::general_purpose::STANDARD;
        env["payload"] = json!(b64.encode(serde_json::to_vec(&p).unwrap()));
        assert!(verify(&serde_json::to_vec(&env).unwrap(), &[k.verifying_key()]).is_err());
        assert!(verify(b"not json", &[k.verifying_key()]).is_err());
        assert!(verify(&vec![b' '; MAX_ENVELOPE_BYTES + 1], &[k.verifying_key()]).is_err());
        assert!(verify(&good, &[]).is_err(), "no key: nothing is trusted");
    }

    #[test]
    fn a_signed_but_unsafe_manifest_is_still_refused() {
        let k = test_key();
        let bad = |edit: &dyn Fn(&mut serde_json::Value)| {
            let mut p = payload();
            edit(&mut p);
            verify(&envelope(&p, &k), &[k.verifying_key()]).is_err()
        };
        assert!(bad(
            &|p| p["tiers"][0]["files"][0]["name"] = json!("..\\evil.dll")
        ));
        assert!(bad(
            &|p| p["tiers"][0]["files"][0]["name"] = json!("con.gguf")
        ));
        assert!(bad(
            &|p| p["tiers"][0]["files"][0]["path"] = json!("../../etc/passwd")
        ));
        assert!(bad(
            &|p| p["tiers"][0]["files"][0]["path"] = json!("https://elsewhere.example/x.gguf")
        ));
        assert!(bad(&|p| p["tiers"][0]["files"][0]["bytes"] = json!(0)));
        assert!(bad(
            &|p| p["tiers"][0]["files"][0]["bytes"] = json!(MAX_FILE_BYTES + 1)
        ));
        assert!(bad(
            &|p| p["tiers"][0]["files"][0]["sha256"] = json!("A".repeat(64))
        ));
        assert!(bad(
            &|p| p["tiers"][0]["files"][0]["role"] = json!("program")
        ));
        assert!(bad(&|p| p["tiers"][0]["files"][1]["role"] = json!("chat")));
        assert!(bad(&|p| p["tiers"][0]["id"] = json!("huge")));
        assert!(bad(&|p| p["tiers"] = json!([])));
        assert!(bad(&|p| p["manifestVersion"] = json!("staging")));
        assert!(bad(&|p| p["manifestVersion"] = json!("quarantine")));
        assert!(bad(
            &|p| p["tiers"][0]["files"][0]["name"] = json!("set.json")
        ));
        assert!(bad(
            &|p| p["tiers"][0]["files"][0]["name"] = json!("embed.gguf.partial")
        ));
        assert!(bad(
            &|p| p["tiers"][0]["files"][0]["name"] = json!("lpt7.gguf")
        ));
        assert!(bad(&|p| p["tiers"][0]["files"][0]["name"] = json!("COM9")));
        assert!(bad(&|p| {
            p.as_object_mut().unwrap().remove("sequence");
        }));
        assert!(bad(&|p| p["extra"] = json!(1)));
        assert!(bad(&|p| p["runtimes"][0]["backend"] = json!("cuda")));
    }
}

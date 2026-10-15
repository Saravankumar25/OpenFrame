//! The build-time half of the team password gate (ADR-0024): which builds carry the gate, how
//! the password from the build machine becomes an Argon2id hash, and how a typed password is
//! checked against it.
//!
//! This file uses only `std` and the `argon2` crate, because the desktop build script includes it
//! by path (`apps/desktop/src-tauri/build.rs`) to make the hash, and the entitlement crate
//! includes it as a module to check it. One file means the build and the check can never disagree
//! on the algorithm or the cost settings. Tests live in `super::tests`.

use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};

/// The environment variable on the build machine that holds the team password. Read by the
/// desktop build script only; the app never reads it.
pub const TEAM_PASSWORD_ENV: &str = "OPENFRAME_TEAM_PASSWORD";

/// Argon2id memory cost for the embedded hash: 64 MiB.
pub const BUILD_M_COST_KIB: u32 = 64 * 1024;
/// Argon2id passes for the embedded hash.
pub const BUILD_T_COST: u32 = 3;
/// Argon2id lanes for the embedded hash.
pub const BUILD_P_COST: u32 = 1;
/// Bytes of random salt in the embedded hash.
pub const SALT_BYTES: usize = 16;
/// A team password shorter than this is refused at build time (ADR-0030: 8 for the pilot gate).
pub const MIN_PASSWORD_CHARS: usize = 8;
/// A typed password longer than this is wrong without being hashed, so a pasted book cannot keep
/// the check busy.
pub const MAX_PASSWORD_BYTES: usize = 1024;

/// What a build carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatePlan {
    /// No gate and no hash: a store package, or a development build without the variable.
    NoGate,
    /// The internal build's gate, from this password.
    Gate(String),
}

/// Decide what a build carries from its distribution (`OPENFRAME_DISTRIBUTION`: `store` or
/// `direct`), its Cargo profile (`release` or `debug`) and the team password variable.
///
/// - A store package never carries the gate or a hash, whatever the variable says.
/// - An internal release build (`direct`) without the variable, or with a password shorter than
///   [`MIN_PASSWORD_CHARS`], fails: an unprotected internal file is never built.
/// - A development build (`debug`) carries the gate when the variable is set, and none otherwise,
///   so developers and the automated tests can run the app; it is never shipped.
///
/// The error text names the variable, never its value.
pub fn build_plan(
    distribution: &str,
    profile: &str,
    password: Option<&str>,
) -> Result<GatePlan, String> {
    if distribution == "store" {
        return Ok(GatePlan::NoGate);
    }
    let password = password.filter(|p| !p.is_empty());
    match (password, profile) {
        (None, "release") => Err(format!(
            "an internal build needs the team password: set {TEAM_PASSWORD_ENV} on the build machine and build again"
        )),
        (None, _) => Ok(GatePlan::NoGate),
        (Some(p), _) if p.chars().count() < MIN_PASSWORD_CHARS => Err(format!(
            "the team password in {TEAM_PASSWORD_ENV} is too short: use at least {MIN_PASSWORD_CHARS} characters"
        )),
        (Some(p), _) if p.len() > MAX_PASSWORD_BYTES => Err(format!(
            "the team password in {TEAM_PASSWORD_ENV} is too long: use at most {MAX_PASSWORD_BYTES} bytes"
        )),
        (Some(p), _) => Ok(GatePlan::Gate(p.to_string())),
    }
}

/// The Argon2id hash (PHC string) of `password` with `salt` and the given costs. Builds use
/// [`hash_for_build`]; tests use small costs to stay fast.
pub fn hash_with_costs(
    password: &str,
    salt: &[u8],
    m_cost_kib: u32,
    t_cost: u32,
    p_cost: u32,
) -> Result<String, String> {
    let params = Params::new(m_cost_kib, t_cost, p_cost, None)
        .map_err(|e| format!("Argon2id settings refused: {e}"))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon
        .hash_password_with_salt(password.as_bytes(), salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("the team password could not be hashed: {e}"))
}

/// The hash a build embeds: Argon2id, 64 MiB, 3 passes, 1 lane, with `salt` from the build
/// machine's secure random generator.
pub fn hash_for_build(password: &str, salt: &[u8; SALT_BYTES]) -> Result<String, String> {
    hash_with_costs(password, salt, BUILD_M_COST_KIB, BUILD_T_COST, BUILD_P_COST)
}

/// True when `password` matches the Argon2id hash `phc`. The hash is computed again with the
/// salt and costs stored in `phc`, and the two outputs are compared in constant time (the
/// `password-hash` crate's `Output` comparison). Anything that is not an Argon2id hash, and any
/// password over [`MAX_PASSWORD_BYTES`], is a mismatch.
pub fn verify_team_password(phc: &str, password: &str) -> bool {
    if password.is_empty() || password.len() > MAX_PASSWORD_BYTES {
        return false;
    }
    let Ok(hash) = PasswordHash::new(phc) else {
        return false;
    };
    if hash.algorithm.as_str() != "argon2id" {
        return false;
    }
    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .is_ok()
}

//! Build script for the Mac interface crate.
//!
//! **Team password (ADR-0024, ADR-0030).** The internal Mac disk image asks the team password like
//! the Windows installer. This script reads `OPENFRAME_TEAM_PASSWORD` on the build machine, hashes
//! it with Argon2id and a random salt, and writes only the hash to `OUT_DIR/team_gate_hash.txt`,
//! embedded under `cfg(openframe_team_gate)`. The rules and the hash come from the same file the
//! Windows build uses (`openframe-entitlement/src/team_gate/build_hash.rs`), so the two can never
//! disagree. An internal release build without the variable fails; a store build
//! (`OPENFRAME_DISTRIBUTION=store`) embeds nothing. The password is never printed.
//!
//! **Model source (ADR-0032).** Only a test build (`OPENFRAME_TEST_BUILD=1`) may name the local
//! test host in `OPENFRAME_TEST_MODEL_SOURCE`, as in the Windows build script.

use std::path::PathBuf;

#[path = "../openframe-entitlement/src/team_gate/build_hash.rs"]
#[allow(dead_code)]
mod team_gate_hash;

fn main() {
    println!("cargo:rerun-if-env-changed=OPENFRAME_DISTRIBUTION");
    println!("cargo::rustc-check-cfg=cfg(openframe_store)");
    let raw = std::env::var("OPENFRAME_DISTRIBUTION").unwrap_or_default();
    let channel = match raw.trim().to_ascii_lowercase().as_str() {
        "" | "direct" => "direct",
        "store" => "store",
        other => panic!("OPENFRAME_DISTRIBUTION must be `direct` or `store`, got `{other}`"),
    };
    if channel == "store" {
        println!("cargo:rustc-cfg=openframe_store");
    }
    embed_team_gate(channel);
    check_model_source();
}

fn embed_team_gate(channel: &str) {
    use team_gate_hash::{GatePlan, SALT_BYTES, TEAM_PASSWORD_ENV, build_plan, hash_for_build};
    println!("cargo::rustc-check-cfg=cfg(openframe_team_gate)");
    let out =
        PathBuf::from(std::env::var("OUT_DIR").expect("set by Cargo")).join("team_gate_hash.txt");
    let _ = std::fs::remove_file(&out);
    if channel == "store" {
        return;
    }
    let profile = std::env::var("PROFILE").unwrap_or_default();
    let password = std::env::var(TEAM_PASSWORD_ENV).ok();
    if password.is_some() {
        // A path that never exists: Cargo reruns the script on every build and stores no value.
        println!("cargo:rerun-if-changed=team-password-is-read-on-every-build");
    } else {
        println!("cargo:rerun-if-env-changed={TEAM_PASSWORD_ENV}");
    }
    match build_plan(channel, &profile, password.as_deref()) {
        Err(message) => panic!("{message}"),
        Ok(GatePlan::NoGate) => {}
        Ok(GatePlan::Gate(password)) => {
            let mut salt = [0u8; SALT_BYTES];
            getrandom::fill(&mut salt).expect("the build machine's secure random generator");
            let hash = hash_for_build(&password, &salt).unwrap_or_else(|m| panic!("{m}"));
            std::fs::write(&out, hash).expect("write the team password hash");
            println!("cargo:rustc-cfg=openframe_team_gate");
        }
    }
}

fn check_model_source() {
    const SRC: &str = "OPENFRAME_TEST_MODEL_SOURCE";
    println!("cargo:rerun-if-env-changed={SRC}");
    println!("cargo:rerun-if-env-changed=OPENFRAME_TEST_BUILD");
    let Ok(url) = std::env::var(SRC) else {
        return;
    };
    let url = url.trim();
    if url.is_empty() {
        return;
    }
    let port = url
        .strip_prefix("http://127.0.0.1:")
        .or_else(|| url.strip_prefix("http://localhost:"))
        .map(|rest| rest.trim_end_matches('/'));
    if !port.is_some_and(|p| !p.is_empty() && p.len() <= 5 && p.chars().all(|c| c.is_ascii_digit()))
    {
        panic!("{SRC} must be the local test host, http://127.0.0.1:<port>/.");
    }
    if std::env::var("OPENFRAME_TEST_BUILD").as_deref() != Ok("1") {
        panic!(
            "{SRC} names the local test host; only a test build (OPENFRAME_TEST_BUILD=1) may use it."
        );
    }
}

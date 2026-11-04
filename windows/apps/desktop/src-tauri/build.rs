//! Build script for the Tauri adapter.
//!
//! `STRAFT_DISTRIBUTION` selects the distribution channel at build time
//! (docs/engineering/27-microsoft-store.md §5):
//!
//! * unset / `direct`: the internal team builds (Windows installer; ADR-0023,
//!   ADR-0024), and every development build;
//! * `store`: the Microsoft Store package. Sets `cfg(straft_store)` so code
//!   that must never ship in store builds (the in-app updater, the team password gate) can be
//!   compiled out entirely.
//!
//! The value is re-exported to the crate as `env!("STRAFT_DISTRIBUTION")`.
//!
//! **Team password (ADR-0024).** An internal build reads `STRAFT_TEAM_PASSWORD` on the build
//! machine, hashes it with Argon2id and a random salt, and writes only the hash to
//! `OUT_DIR/team_gate_hash.txt`, which the crate embeds under `cfg(straft_team_gate)`. An
//! internal release build without the variable fails. A store build never reads the variable and
//! embeds nothing. The password is never printed: errors name the variable, not its value. While
//! the variable is set, the script reruns on every build instead of asking Cargo to watch the
//! variable, because Cargo would otherwise keep its value in the target folder to compare later.
//!
//! **Model source (ADR-0032).** Every build downloads the models from Hugging Face at the
//! revisions pinned in `packaging/ai/ai-assets.lock.json`; nothing needs setting. Only the test
//! suite sets `STRAFT_TEST_MODEL_SOURCE` to its loopback host (`http://127.0.0.1:<port>/`),
//! read by the crate with `option_env!`. A build of any profile accepts it only with
//! `STRAFT_TEST_BUILD=1`, which only the test scripts set, so a team or store build can never
//! carry it by accident.

use std::path::PathBuf;

/// The rules and the hash, shared with the check in the app so the two never disagree.
#[path = "../../../crates/straft-entitlement/src/team_gate/build_hash.rs"]
#[allow(dead_code)]
mod team_gate_hash;

fn main() {
    println!("cargo:rerun-if-env-changed=STRAFT_DISTRIBUTION");
    println!("cargo::rustc-check-cfg=cfg(straft_store)");
    let raw = std::env::var("STRAFT_DISTRIBUTION").unwrap_or_default();
    let channel = match raw.trim().to_ascii_lowercase().as_str() {
        "" | "direct" => "direct",
        "store" => "store",
        other => panic!("STRAFT_DISTRIBUTION must be `direct` or `store`, got `{other}`"),
    };
    if channel == "store" {
        println!("cargo:rustc-cfg=straft_store");
    }
    println!("cargo:rustc-env=STRAFT_DISTRIBUTION={channel}");
    embed_team_gate(channel);
    check_model_source();
    // Only these commands may be invoked by the webview, each named in the capability file.
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(APP_COMMANDS)),
    )
    .expect("the Tauri build step");
}

/// Must match `generate_handler!` in `src/lib.rs`.
const APP_COMMANDS: &[&str] = &[
    "of_invoke",
    "of_open_asset",
    "of_reveal_path",
    "of_open_url",
    "of_close_window",
];

/// Write the team password hash for an internal build, or nothing for a store build (see the
/// module notes). Panics, which fails the build, when an internal release build has no password.
fn embed_team_gate(channel: &str) {
    use team_gate_hash::{GatePlan, SALT_BYTES, TEAM_PASSWORD_ENV, build_plan, hash_for_build};
    println!("cargo::rustc-check-cfg=cfg(straft_team_gate)");
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
            println!("cargo:rustc-cfg=straft_team_gate");
        }
    }
}

/// Panics, which fails the build, when the test model source is not a loopback http address, or
/// a build names it without `STRAFT_TEST_BUILD=1` (module notes).
fn check_model_source() {
    const SRC: &str = "STRAFT_TEST_MODEL_SOURCE";
    println!("cargo:rerun-if-env-changed={SRC}");
    println!("cargo:rerun-if-env-changed=STRAFT_TEST_BUILD");
    let Ok(url) = std::env::var(SRC) else {
        return; // every build for people: Hugging Face
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
    // Every profile: a debug build handed to someone must not carry the test host either.
    if std::env::var("STRAFT_TEST_BUILD").as_deref() != Ok("1") {
        panic!(
            "{SRC} names the local test host; only a test build (STRAFT_TEST_BUILD=1) may use it."
        );
    }
    println!(
        "cargo:warning=This build downloads Straft AI from the local test host {url}: a test build, never for the team."
    );
}

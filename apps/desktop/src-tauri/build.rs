//! Build script for the Tauri adapter.
//!
//! `OPENFRAME_DISTRIBUTION` selects the distribution channel at build time
//! (docs/engineering/27-microsoft-store.md §5):
//!
//! * unset / `direct`: the internal team builds (Windows installer, Mac disk image; ADR-0023,
//!   ADR-0024), and every development build;
//! * `store`: the Microsoft Store and Mac App Store packages. Sets `cfg(openframe_store)` so code
//!   that must never ship in store builds (the in-app updater, the team password gate) can be
//!   compiled out entirely.
//!
//! The value is re-exported to the crate as `env!("OPENFRAME_DISTRIBUTION")`.
//!
//! **Team password (ADR-0024).** An internal build reads `OPENFRAME_TEAM_PASSWORD` on the build
//! machine, hashes it with Argon2id and a random salt, and writes only the hash to
//! `OUT_DIR/team_gate_hash.txt`, which the crate embeds under `cfg(openframe_team_gate)`. An
//! internal release build without the variable fails. A store build never reads the variable and
//! embeds nothing. The password is never printed: errors name the variable, not its value. While
//! the variable is set, the script reruns on every build instead of asking Cargo to watch the
//! variable, because Cargo would otherwise keep its value in the target folder to compare later.
//!
//! **Model host (ADR-0026 decision 8).** `MODEL_DISTRIBUTION_BASE_URL` (the owner's https model
//! host) and `OPENFRAME_MANIFEST_PUBLIC_KEY` (the public half of the owner's offline Ed25519
//! manifest key, base64, several separated by commas) are read by the crate with `option_env!`.
//! A release build without both fails here. A loopback `http://127.0.0.1:<port>/` address is the
//! local test host of the test suite: a release build accepts it only with `OPENFRAME_TEST_BUILD=1`,
//! which only the test scripts set, so a team or store build can never carry it by accident.
//!
//! On a macOS target the script also builds the Swift StoreKit 2 bridge
//! (`macos/StoreKitBridge/`) and links it with the StoreKit and Foundation frameworks
//! (docs/engineering/37-platform-boundary.md). On every other target it does nothing more.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The rules and the hash, shared with the check in the app so the two never disagree.
#[path = "../../../crates/openframe-entitlement/src/team_gate/build_hash.rs"]
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
    println!("cargo:rustc-env=OPENFRAME_DISTRIBUTION={channel}");
    embed_team_gate(channel);
    check_model_host();
    // The target, not the machine running this script.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        link_storekit_bridge();
    }
    tauri_build::build()
}

/// Write the team password hash for an internal build, or nothing for a store build (see the
/// module notes). Panics, which fails the build, when an internal release build has no password.
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

/// Panics, which fails the build, when a release build lacks the model host or its key, or when
/// either is malformed (module notes).
fn check_model_host() {
    const URL: &str = "MODEL_DISTRIBUTION_BASE_URL";
    const KEY: &str = "OPENFRAME_MANIFEST_PUBLIC_KEY";
    println!("cargo:rerun-if-env-changed={URL}");
    println!("cargo:rerun-if-env-changed={KEY}");
    println!("cargo:rerun-if-env-changed=OPENFRAME_TEST_BUILD");
    let url = std::env::var(URL).unwrap_or_default();
    let keys = std::env::var(KEY).unwrap_or_default();
    let (url, keys) = (url.trim(), keys.trim());
    if url.is_empty() && keys.is_empty() && std::env::var("PROFILE").as_deref() != Ok("release") {
        return; // a development build that cannot download
    }
    if url.is_empty() || keys.is_empty() {
        panic!("A release build needs {URL} and {KEY} (ADR-0026 decision 8).");
    }
    let test_host = url.starts_with("http://127.0.0.1:");
    if !(url.starts_with("https://") || test_host) {
        panic!("{URL} must be an https address.");
    }
    let b64 = |c: char| c.is_ascii_alphanumeric() || matches!(c, '+' | '/');
    let key_ok = |k: &str| k.len() == 44 && k.ends_with('=') && k[..43].chars().all(b64);
    if !keys.split(',').map(str::trim).all(key_ok) {
        panic!(
            "{KEY} must be base64 Ed25519 public keys (44 characters each), separated by commas."
        );
    }
    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    let test_build = std::env::var("OPENFRAME_TEST_BUILD").as_deref() == Ok("1");
    if test_host && release && !test_build {
        panic!(
            "{URL} names the local test host; a release build for people needs the owner's https host."
        );
    }
    if test_host {
        println!(
            "cargo:warning=This build downloads OpenFrame AI from the local test host {url}: a test build, never for the team."
        );
    }
}

/// Build the Swift package into `OUT_DIR` and link its static library. Runs only for a macOS
/// target, which needs a Mac with Xcode or the Swift toolchain.
fn link_storekit_bridge() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("set by Cargo"));
    let package = manifest.join("macos").join("StoreKitBridge");
    let scratch = PathBuf::from(std::env::var("OUT_DIR").expect("set by Cargo")).join("swift");
    println!(
        "cargo:rerun-if-changed={}",
        package.join("Package.swift").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        package.join("Sources").display()
    );

    let args = |extra: &[&str]| -> Vec<String> {
        let mut a: Vec<String> = [
            "build",
            "-c",
            "release",
            "--product",
            "StoreKitBridge",
            "--package-path",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        a.push(package.display().to_string());
        a.push("--scratch-path".to_string());
        a.push(scratch.display().to_string());
        a.extend(extra.iter().map(|s| s.to_string()));
        a
    };
    run_swift(&args(&[]), "build the StoreKit bridge");
    let bin = run_swift(&args(&["--show-bin-path"]), "find the StoreKit bridge");
    println!("cargo:rustc-link-search=native={}", bin.trim());
    println!("cargo:rustc-link-lib=static=StoreKitBridge");
    println!("cargo:rustc-link-lib=framework=StoreKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
    // The Swift runtime: the system copy in /usr/lib/swift and the toolchain's compatibility
    // libraries, as `swift -print-target-info` reports them.
    let info = run_swift(&["-print-target-info".to_string()], "read the Swift target");
    let info: serde_json::Value =
        serde_json::from_str(&info).expect("`swift -print-target-info` printed JSON");
    let paths = info["paths"]["runtimeLibraryPaths"]
        .as_array()
        .expect("`swift -print-target-info` lists runtimeLibraryPaths");
    for path in paths.iter().filter_map(serde_json::Value::as_str) {
        if Path::new(path).is_dir() {
            println!("cargo:rustc-link-search=native={path}");
        }
    }
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
}

/// Run `swift` with `args` and return its standard output. Fails the build with `what` on error.
fn run_swift(args: &[String], what: &str) -> String {
    let out = Command::new("swift")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("could not {what}: `swift` did not start ({e})"));
    if !out.status.success() {
        panic!(
            "could not {what}: `swift {}` failed\n{}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8(out.stdout).unwrap_or_else(|_| panic!("could not {what}: output not UTF-8"))
}

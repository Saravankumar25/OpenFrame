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
//! **Model source (ADR-0032).** Every build downloads the models from Hugging Face at the
//! revisions pinned in `packaging/ai/ai-assets.lock.json`; nothing needs setting. Only the test
//! suite sets `OPENFRAME_TEST_MODEL_SOURCE` to its loopback host (`http://127.0.0.1:<port>/`),
//! read by the crate with `option_env!`. A build of any profile accepts it only with
//! `OPENFRAME_TEST_BUILD=1`, which only the test scripts set, so a team or store build can never
//! carry it by accident.
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
    check_model_source();
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

/// Panics, which fails the build, when the test model source is not a loopback http address, or
/// a build names it without `OPENFRAME_TEST_BUILD=1` (module notes).
fn check_model_source() {
    const SRC: &str = "OPENFRAME_TEST_MODEL_SOURCE";
    println!("cargo:rerun-if-env-changed={SRC}");
    println!("cargo:rerun-if-env-changed=OPENFRAME_TEST_BUILD");
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
    if std::env::var("OPENFRAME_TEST_BUILD").as_deref() != Ok("1") {
        panic!(
            "{SRC} names the local test host; only a test build (OPENFRAME_TEST_BUILD=1) may use it."
        );
    }
    println!(
        "cargo:warning=This build downloads OpenFrame AI from the local test host {url}: a test build, never for the team."
    );
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

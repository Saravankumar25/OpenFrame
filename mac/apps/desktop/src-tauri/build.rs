//! Build script for the Tauri adapter.
//!
//! `STRAFT_DISTRIBUTION` selects the distribution channel at build time
//! (docs/engineering/27-microsoft-store.md §5):
//!
//! * unset / `direct`: the internal team builds (Windows installer, Mac disk image; ADR-0023,
//!   ADR-0024), and every development build;
//! * `store`: the Microsoft Store and Mac App Store packages. Sets `cfg(straft_store)` so code
//!   that must never ship in store builds (the in-app updater) can be compiled out entirely.
//!
//! The value is re-exported to the crate as `env!("STRAFT_DISTRIBUTION")`.
//!
//! **Model source (ADR-0032).** Every build downloads the models from Hugging Face at the
//! revisions pinned in `packaging/ai/ai-assets.lock.json`; nothing needs setting. Only the test
//! suite sets `STRAFT_TEST_MODEL_SOURCE` to its loopback host (`http://127.0.0.1:<port>/`),
//! read by the crate with `option_env!`. A build of any profile accepts it only with
//! `STRAFT_TEST_BUILD=1`, which only the test scripts set, so a team or store build can never
//! carry it by accident.

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

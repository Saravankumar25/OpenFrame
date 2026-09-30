//! Build script for the Tauri adapter.
//!
//! `OPENFRAME_DISTRIBUTION` selects the distribution channel at build time
//! (docs/engineering/27-microsoft-store.md §5):
//!
//! * unset / `direct` — NSIS/MSI installers from the OpenFrame release host (default);
//! * `store`          — MSIX for the Microsoft Store. Sets `cfg(openframe_store)` so code that must
//!   never ship in Store builds (the in-app updater) can be compiled out entirely.
//!
//! The value is re-exported to the crate as `env!("OPENFRAME_DISTRIBUTION")`.

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
    tauri_build::build()
}

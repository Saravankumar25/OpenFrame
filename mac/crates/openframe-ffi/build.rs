//! Build script for the Mac interface crate.
//!
//! **Channel.** `OPENFRAME_DISTRIBUTION=store` builds the Mac App Store package (`cfg(openframe_store)`);
//! empty or `direct` builds the internal disk image.
//!
//! **Model source (ADR-0032).** Only a test build (`OPENFRAME_TEST_BUILD=1`) may name the local
//! test host in `OPENFRAME_TEST_MODEL_SOURCE`, as in the Windows build script.

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
    check_model_source();
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

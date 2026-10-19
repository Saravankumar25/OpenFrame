//! Platform adapters of the desktop shell: the Mac App Store connection.
//!
//! `lib.rs` calls the four functions below on every system and never learns which one it runs
//! on. On macOS they connect the Swift StoreKit 2 bridge and the licence server client to the
//! entitlement service and run its checks on a background thread. On Windows they do nothing, so
//! the Windows build behaves exactly as before.
//!
//! - `storekit.rs`: the bridge's JSON contract and its mapping onto the entitlement crate's
//!   types. Pure Rust; compiled on macOS and for the tests on every system.
//! - `macos_storekit.rs`: the C ABI calls into the Swift package and the crate's `StoreKitBridge`
//!   implementation. macOS only.
//! - `mac_inputs.rs`: the HTTPS licence server client, the nonce source and the placeholders the
//!   owner fills in. macOS only.
//! - `revalidation.rs`: when to check again (launch, resume, wake, focus, StoreKit changes,
//!   lease). Its decisions are tested on every system; the worker thread is macOS only.
//!
//! No Tauri command is added and the webview never reaches StoreKit or the network: it reads the
//! state through the existing `entitlement.status` operation. Build: `build.rs` compiles the Swift
//! package `apps/desktop/src-tauri/macos/StoreKitBridge/` and links it on macOS only
//! (docs/engineering/37-platform-boundary.md, "macOS shell adapter: StoreKit bridge").

#[cfg(target_os = "macos")]
mod mac_inputs;
#[cfg(target_os = "macos")]
mod macos_storekit;
#[cfg(any(target_os = "macos", test))]
mod revalidation;
#[cfg(any(target_os = "macos", test))]
mod storekit;

use openframe_entitlement::mac::MacInputs;

#[cfg(target_os = "macos")]
pub use revalidation::{on_run_event, on_window_event, start_entitlement_checks};

/// The Mac App Store adapter's inputs. Builds objects only: no StoreKit call, no request.
#[cfg(target_os = "macos")]
pub fn mac_inputs() -> Option<MacInputs> {
    Some(mac_inputs::build())
}

/// Windows: no App Store. The entitlement service uses the Windows channels.
#[cfg(not(target_os = "macos"))]
pub fn mac_inputs() -> Option<MacInputs> {
    None
}

#[cfg(not(target_os = "macos"))]
pub use no_store::{on_run_event, on_window_event, start_entitlement_checks};

/// Windows twins of the macOS hooks. The Windows channels check at startup and on "Restore
/// purchase" only, so there is nothing to start or to react to.
#[cfg(not(target_os = "macos"))]
mod no_store {
    use std::sync::Arc;

    use openframe_application::AppCore;
    use tauri::{AppHandle, RunEvent, WindowEvent};

    pub fn start_entitlement_checks(_core: &Arc<AppCore>) {}

    pub fn on_run_event(_app: &AppHandle, _event: &RunEvent) {}

    pub fn on_window_event(_event: &WindowEvent) {}
}

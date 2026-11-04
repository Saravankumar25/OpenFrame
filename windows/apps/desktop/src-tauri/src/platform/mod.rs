//! Platform adapters of the desktop shell: the Microsoft Store checks of the Windows build
//! (ADR-0039). The Tauri Mac shell and its StoreKit bridge are retired (ADR-0027); the Mac app
//! is a native SwiftUI shell over the same Rust core.
//!
//! - `microsoft_store.rs`: the real [`StoreLicenceSource`](straft_entitlement::adapters::StoreLicenceSource)
//!   over `Windows.Services.Store`: the app licence, the purchase window and the subscription to
//!   `OfflineLicensesChanged`. WinRT part Windows only; its pure helpers are tested everywhere.
//! - `store_checks.rs`: when to read the licence again (launch, a Store change notice, focus after
//!   5 minutes, wake from sleep, every 15 minutes, local midnight, the trial's end) and the one
//!   worker thread that does it. Only the Microsoft Store build starts it; the internal
//!   installer build does nothing here.
//!
//! No Tauri command is added and the web view never reaches the Store: it reads the state through
//! the existing `entitlement.status` operation and hears about a change as a `module` event.

mod microsoft_store;
mod store_checks;

#[cfg(windows)]
pub use microsoft_store::MicrosoftStore;
pub use store_checks::{on_resume, on_window_event, start_entitlement_checks};

use std::sync::{Arc, OnceLock};

use straft_entitlement::Channel;
use straft_entitlement::adapters::StoreLicenceSource;
use tauri::{AppHandle, WebviewWindow};

/// The main window's native handle, filled in once the window is built. The Store's purchase
/// window opens over it, and Store reads wait for it (20 seconds at most).
#[derive(Clone, Default)]
pub struct WindowHandle(Arc<OnceLock<isize>>);

impl WindowHandle {
    pub fn set(&self, window: &WebviewWindow) {
        #[cfg(windows)]
        if let Ok(hwnd) = window.hwnd() {
            let _ = self.0.set(hwnd.0 as isize);
        }
        #[cfg(not(windows))]
        let _ = window;
    }
}

/// The Store connection for `channel`: the real Microsoft Store source on the Microsoft Store
/// channel, nothing for every other build (the internal installer build keeps no Store link).
pub fn microsoft_store_source(
    channel: Channel,
    app: &AppHandle,
    window: &WindowHandle,
) -> Option<Box<dyn StoreLicenceSource>> {
    if !store_checks::starts_worker(channel) {
        return None;
    }
    #[cfg(windows)]
    {
        let ui = app.clone();
        Some(Box::new(MicrosoftStore::new(
            window.0.clone(),
            Box::new(move |job| ui.run_on_main_thread(job).is_ok()),
        )))
    }
    #[cfg(not(windows))]
    {
        let _ = (app, window);
        None
    }
}

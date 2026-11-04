//! The Microsoft Store connection of the Windows build (ADR-0039 items 3, 4 and 6): the real
//! [`StoreLicenceSource`] over `Windows.Services.Store`.
//!
//! - `app_licence`: `StoreContext.GetAppLicenseAsync`, read on a fresh thread with a 20 second
//!   limit. It answers from the Store's cached offline licence when the Store cannot be reached.
//!   Any error or timeout is `Unreachable`; the log carries the error code at warn level, never a
//!   store or licence identifier, and no code ever reaches a screen. A licence with an empty or
//!   unreadable SKU id on an ACTIVE licence is not a Store licence (an unpackaged or sideloaded
//!   process gets an active licence from the platform) and is also `Unreachable`, never
//!   Purchased ([`read_from`]). An inactive licence is an answer whatever its SKU, so a refund
//!   can lock.
//! - `request_purchase`: finds the product off the UI thread (20 second limit), starts
//!   `RequestPurchaseAsync` on the UI thread (the Store refuses it anywhere else, 0x80070578),
//!   then waits for the person off the UI thread with no limit, because they may keep the Store
//!   window open as long as they like.
//! - `OfflineLicensesChanged` is subscribed once, after the first read, on one kept
//!   `StoreContext`. The handler only asks the refresh worker for a read.
//!
//! The pure helpers (ticks to seconds, status mapping) are built and tested on every system. The
//! WinRT part is Windows only and needs a real Store package to answer: an unpackaged process
//! gets `Unreachable`.

// Everything but the pure helpers is Windows only; the helpers are built everywhere for tests.
#![cfg_attr(not(windows), allow(dead_code))]

use straft_entitlement::adapters::{StoreAppLicence, StorePurchaseStatus};

/// WinRT `TimeSpan` and `DateTime` count 100 nanosecond ticks.
const TICKS_PER_SECOND: u64 = 10_000_000;

/// 100 nanosecond ticks between 1601-01-01 (the `DateTime` origin) and 1970-01-01.
const UNIX_EPOCH_TICKS: i64 = 116_444_736_000_000_000;

/// A `TimeSpan` in ticks as whole seconds, rounded up (a trial with a fraction of a second left
/// is still running). Zero or less is 0.
pub fn seconds_from_span(ticks: i64) -> u64 {
    u64::try_from(ticks).map_or(0, |t| t.div_ceil(TICKS_PER_SECOND))
}

/// Whole seconds from `now_ticks` until `expiration_ticks` (both `DateTime` ticks), rounded up,
/// 0 when the moment has passed.
pub fn seconds_until(expiration_ticks: i64, now_ticks: i64) -> u64 {
    seconds_from_span(expiration_ticks.saturating_sub(now_ticks))
}

/// The trial seconds left: the Store's `TrialTimeRemaining`, and when it says zero or less, the
/// time up to the licence's `ExpirationDate`. Never anything kept on this computer.
pub fn trial_seconds(span_ticks: i64, expiration_ticks: i64, now_ticks: i64) -> u64 {
    match seconds_from_span(span_ticks) {
        0 => seconds_until(expiration_ticks, now_ticks),
        left => left,
    }
}

/// Whether a licence is a real Store licence: it names a SKU. The platform answers a process
/// without Store identity (unpackaged, sideloaded, dev-signed) with an active, non-trial licence
/// that has an empty SKU id, and that must never read as a purchase.
pub fn is_store_licence(sku_store_id: &str) -> bool {
    !sku_store_id.trim().is_empty()
}

/// The licence read, or `None` when it is not a Store licence. Only an active licence needs a SKU:
/// the platform answers a process without Store identity with an active one that names none. An
/// inactive licence (a refund, an ended trial) is an answer whatever its SKU, so it can lock.
/// `sku` is `None` when the SKU id could not be read.
pub fn read_from(
    is_active: bool,
    is_trial: bool,
    sku: Option<&str>,
    trial_seconds_left: u64,
) -> Option<StoreAppLicence> {
    if is_active && !sku.is_some_and(is_store_licence) {
        return None;
    }
    Some(StoreAppLicence {
        is_active,
        is_trial,
        trial_seconds_left: if is_trial && is_active {
            trial_seconds_left
        } else {
            0
        },
    })
}

/// `StorePurchaseStatus` as the dialog returned it. Anything unknown is a Store-side problem.
pub fn purchase_status_from(raw: i32) -> StorePurchaseStatus {
    match raw {
        0 => StorePurchaseStatus::Succeeded,
        1 => StorePurchaseStatus::AlreadyPurchased,
        2 => StorePurchaseStatus::NotPurchased,
        3 => StorePurchaseStatus::NetworkError,
        _ => StorePurchaseStatus::ServerError,
    }
}

#[cfg(windows)]
pub use store::MicrosoftStore;

#[cfg(windows)]
mod store {
    use std::ffi::c_void;
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex, OnceLock, PoisonError};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use straft_entitlement::adapters::{
        StoreAppLicence, StoreLicenceError, StoreLicenceSource, StorePurchaseStatus,
    };
    use windows::Foundation::TypedEventHandler;
    use windows::Services::Store::StoreContext;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};
    use windows::Win32::UI::Shell::IInitializeWithWindow;
    use windows::core::{IInspectable, Interface};

    use super::{UNIX_EPOCH_TICKS, purchase_status_from, read_from, trial_seconds};
    use crate::platform::store_checks::{Request, request};

    /// Runs a closure on the UI thread and says whether it was queued
    /// (`AppHandle::run_on_main_thread`, set by the shell).
    pub type MainThread = Box<dyn Fn(Box<dyn FnOnce() + Send>) -> bool + Send + Sync>;

    /// How long a Store call may take before the read counts as unreachable.
    const CALL_LIMIT: Duration = Duration::from_secs(20);

    struct Inner {
        /// The main window's handle, set once the window is built.
        hwnd: Arc<OnceLock<isize>>,
        main: MainThread,
        limit: Duration,
        /// The one context that listens for `OfflineLicensesChanged`, and its event token. Kept for
        /// the life of the app: dropping the context would end the subscription.
        listener: Mutex<Option<(StoreContext, i64)>>,
    }

    /// The Microsoft Store as the entitlement service's licence source.
    pub struct MicrosoftStore {
        inner: Arc<Inner>,
    }

    impl MicrosoftStore {
        pub fn new(hwnd: Arc<OnceLock<isize>>, main: MainThread) -> Self {
            Self::with_limit(hwnd, main, CALL_LIMIT)
        }

        pub(super) fn with_limit(
            hwnd: Arc<OnceLock<isize>>,
            main: MainThread,
            limit: Duration,
        ) -> Self {
            Self {
                inner: Arc::new(Inner {
                    hwnd,
                    main,
                    limit,
                    listener: Mutex::new(None),
                }),
            }
        }

        /// Run `work` on a fresh thread that has started its WinRT apartment, and wait for it at
        /// most the call limit. `None` when the window does not exist yet, the thread does not
        /// start, the call fails or it takes too long. A thread that is still inside the Store
        /// when the limit passes is left to finish on its own.
        fn call<T: Send + 'static>(
            &self,
            work: impl FnOnce(&Inner, isize) -> windows::core::Result<T> + Send + 'static,
        ) -> Option<T> {
            let hwnd = self.inner.wait_for_window()?;
            let (tx, rx) = mpsc::channel();
            let inner = self.inner.clone();
            std::thread::Builder::new()
                .name("straft-store-call".into())
                .spawn(move || {
                    start_apartment();
                    let _ = tx.send(work(&inner, hwnd));
                })
                .ok()?;
            match rx.recv_timeout(self.inner.limit) {
                Ok(Ok(value)) => Some(value),
                Ok(Err(e)) => {
                    tracing::warn!(code = %code(&e), "the Microsoft Store call failed");
                    None
                }
                Err(_) => {
                    tracing::warn!("the Microsoft Store did not answer in time");
                    None
                }
            }
        }
    }

    impl Inner {
        /// The window handle, waiting up to the call limit for the window to be built.
        fn wait_for_window(&self) -> Option<isize> {
            let deadline = Instant::now() + self.limit;
            loop {
                if let Some(hwnd) = self.hwnd.get() {
                    return Some(*hwnd);
                }
                if Instant::now() >= deadline {
                    tracing::warn!("the Microsoft Store was asked before the window existed");
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        /// Subscribe to `OfflineLicensesChanged` on `context`, once. Kept alive from then on.
        fn listen(&self, context: StoreContext) {
            let mut listener = self.listener.lock().unwrap_or_else(PoisonError::into_inner);
            if listener.is_some() {
                return;
            }
            let handler = TypedEventHandler::<StoreContext, IInspectable>::new(|_, _| {
                request(Request::Changed);
                Ok(())
            });
            match context.OfflineLicensesChanged(&handler) {
                Ok(token) => *listener = Some((context, token)),
                Err(e) => tracing::warn!(
                    code = %code(&e),
                    "the Store licence change notice is not available; reads run on the other triggers"
                ),
            }
        }
    }

    impl StoreLicenceSource for MicrosoftStore {
        fn app_licence(&self) -> Result<StoreAppLicence, StoreLicenceError> {
            let read = self.call(|inner, hwnd| {
                let context = context_for(hwnd)?;
                let licence = context.GetAppLicenseAsync()?.get()?;
                // An unreadable SKU id counts as empty. The id itself is never logged.
                let sku = licence.SkuStoreId().ok().map(|id| id.to_string_lossy());
                let is_active = licence.IsActive()?;
                let is_trial = licence.IsTrial()?;
                let trial_seconds_left = if is_trial && is_active {
                    let span = licence.TrialTimeRemaining().map_or(0, |t| t.Duration);
                    let ends = licence.ExpirationDate().map_or(0, |d| d.UniversalTime);
                    trial_seconds(span, ends, ticks_now())
                } else {
                    0
                };
                let read = read_from(is_active, is_trial, sku.as_deref(), trial_seconds_left);
                if read.is_none() {
                    tracing::warn!(
                        "the Store licence names no SKU; not treated as a Store licence"
                    );
                } else {
                    inner.listen(context);
                }
                Ok(read)
            });
            read.flatten().ok_or(StoreLicenceError::Unreachable)
        }

        fn request_purchase(&self) -> StorePurchaseStatus {
            let Some(product) = self.call(|_, hwnd| {
                let found = context_for(hwnd)?
                    .GetStoreProductForCurrentAppAsync()?
                    .get()?;
                found.ExtendedError()?.ok()?;
                found.Product()
            }) else {
                return StorePurchaseStatus::NetworkError;
            };
            // The Store accepts the purchase window only from the UI thread; starting it must not
            // wait for the person.
            let (started_tx, started) = mpsc::channel();
            let queued = (self.inner.main)(Box::new(move || {
                let _ = started_tx.send(product.RequestPurchaseAsync());
            }));
            if !queued {
                return StorePurchaseStatus::ServerError;
            }
            let operation = match started.recv_timeout(self.inner.limit) {
                Ok(Ok(operation)) => operation,
                Ok(Err(e)) => {
                    tracing::warn!(code = %code(&e), "the Store purchase window could not start");
                    return StorePurchaseStatus::ServerError;
                }
                Err(_) => return StorePurchaseStatus::ServerError,
            };
            // The person may keep the Store window open as long as they like: no time limit, and
            // not on the UI thread.
            let (done_tx, done) = mpsc::channel();
            let waiting = std::thread::Builder::new()
                .name("straft-store-purchase".into())
                .spawn(move || {
                    start_apartment();
                    let _ = done_tx.send(operation.get().and_then(|r| r.Status()));
                });
            if waiting.is_err() {
                return StorePurchaseStatus::ServerError;
            }
            match done.recv() {
                Ok(Ok(status)) => purchase_status_from(status.0),
                Ok(Err(e)) => {
                    tracing::warn!(code = %code(&e), "the Store purchase did not finish");
                    StorePurchaseStatus::ServerError
                }
                Err(_) => StorePurchaseStatus::ServerError,
            }
        }
    }

    /// Start this thread's WinRT apartment (multithreaded). Already started is fine.
    fn start_apartment() {
        // SAFETY: RoInitialize takes a plain enum and no pointers; the apartment lives as long as
        // the thread, which ends with the call it was made for.
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    }

    /// The default Store context with the app window set as the owner of its dialogs.
    fn context_for(hwnd: isize) -> windows::core::Result<StoreContext> {
        let context = StoreContext::GetDefault()?;
        let owner: IInitializeWithWindow = context.cast()?;
        // SAFETY: `hwnd` is the handle of this process's main window, taken from the window the
        // shell built; Initialize only remembers it for the Store's dialogs.
        unsafe { owner.Initialize(HWND(hwnd as *mut c_void)) }?;
        Ok(context)
    }

    /// Now as `DateTime` ticks (100 ns since 1601-01-01).
    fn ticks_now() -> i64 {
        let since_1970 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() / 100);
        i64::try_from(since_1970)
            .unwrap_or(0)
            .saturating_add(UNIX_EPOCH_TICKS)
    }

    /// The error code for the log only, as hex. It never reaches a screen.
    fn code(e: &windows::core::Error) -> String {
        format!("{:#010x}", e.code().0)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn store(hwnd: Option<isize>, limit: Duration) -> MicrosoftStore {
            let cell = Arc::new(OnceLock::new());
            if let Some(h) = hwnd {
                cell.set(h).unwrap();
            }
            MicrosoftStore::with_limit(cell, Box::new(|_| false), limit)
        }

        #[test]
        fn a_read_before_the_window_exists_is_unreachable_after_the_limit() {
            let started = Instant::now();
            let s = store(None, Duration::from_millis(300));
            assert_eq!(s.app_licence(), Err(StoreLicenceError::Unreachable));
            assert!(started.elapsed() < Duration::from_secs(5));
        }

        #[test]
        fn a_purchase_before_the_window_exists_is_a_network_error() {
            let s = store(None, Duration::from_millis(300));
            assert_eq!(s.request_purchase(), StorePurchaseStatus::NetworkError);
        }

        /// An unpackaged process has no Store identity. The real call must answer within the limit
        /// (no panic, no hang) and must never read as a licence. Measured 2026-11-03 (Windows 11,
        /// unpackaged test exe): before the SKU guard the platform answered "active, not a trial";
        /// with it the read is Unreachable, so the SKU id was empty or unreadable.
        #[test]
        fn the_real_source_in_an_unpackaged_process_is_never_a_licence() {
            let started = Instant::now();
            let s = store(Some(0), Duration::from_secs(3));
            assert_eq!(s.app_licence(), Err(StoreLicenceError::Unreachable));
            assert!(started.elapsed() < Duration::from_secs(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_span_rounds_up_to_whole_seconds_and_never_goes_below_zero() {
        assert_eq!(seconds_from_span(0), 0);
        assert_eq!(seconds_from_span(-1), 0);
        assert_eq!(seconds_from_span(i64::MIN), 0);
        assert_eq!(
            seconds_from_span(1),
            1,
            "a fraction of a second is still a second"
        );
        assert_eq!(seconds_from_span(10_000_000), 1);
        assert_eq!(seconds_from_span(10_000_001), 2);
        assert_eq!(seconds_from_span(7 * 24 * 3600 * 10_000_000), 7 * 24 * 3600);
        assert_eq!(seconds_from_span(i64::MAX), 922_337_203_686, "no overflow");
    }

    #[test]
    fn seconds_until_counts_from_now_to_the_expiration() {
        let now = 1_000 * 10_000_000;
        assert_eq!(seconds_until(now + 90 * 10_000_000, now), 90);
        assert_eq!(seconds_until(now, now), 0);
        assert_eq!(seconds_until(now - 5, now), 0, "already past");
        assert_eq!(seconds_until(i64::MIN, i64::MAX), 0, "no overflow");
    }

    #[test]
    fn the_store_span_wins_and_the_expiration_date_is_the_only_fallback() {
        let now = 5_000 * 10_000_000;
        let ends = now + 3_600 * 10_000_000;
        assert_eq!(trial_seconds(120 * 10_000_000, ends, now), 120);
        assert_eq!(
            trial_seconds(0, ends, now),
            3_600,
            "zero span: use the date"
        );
        assert_eq!(
            trial_seconds(-1, ends, now),
            3_600,
            "negative span: use the date"
        );
        assert_eq!(trial_seconds(0, 0, now), 0, "no date: nothing left");
        assert_eq!(trial_seconds(0, now - 1, now), 0, "the date has passed");
    }

    #[test]
    fn only_a_licence_that_names_a_sku_is_a_store_licence() {
        assert!(is_store_licence("9NBLGGH4NNS1"));
        assert!(!is_store_licence(""));
        assert!(!is_store_licence("   "));
    }

    #[test]
    fn an_inactive_licence_is_an_answer_whatever_its_sku_and_an_active_one_needs_a_sku() {
        let lic = |active, trial, left| StoreAppLicence {
            is_active: active,
            is_trial: trial,
            trial_seconds_left: left,
        };
        // A refund or an ended trial may come with an empty SKU: it still locks.
        assert_eq!(
            read_from(false, false, Some(""), 0),
            Some(lic(false, false, 0))
        );
        assert_eq!(read_from(false, true, None, 99), Some(lic(false, true, 0)));
        // An active licence without a SKU is the platform's answer for an unpackaged process.
        assert_eq!(read_from(true, false, Some(""), 0), None);
        assert_eq!(read_from(true, true, None, 5), None);
        assert_eq!(
            read_from(true, false, Some("9NBL"), 0),
            Some(lic(true, false, 0))
        );
        assert_eq!(
            read_from(true, true, Some("9NBL"), 120),
            Some(lic(true, true, 120))
        );
        assert_eq!(
            read_from(true, false, Some("9NBL"), 120),
            Some(lic(true, false, 0))
        );
    }

    #[test]
    fn every_store_purchase_status_maps_and_unknown_ones_are_server_errors() {
        use StorePurchaseStatus::*;
        assert_eq!(purchase_status_from(0), Succeeded);
        assert_eq!(purchase_status_from(1), AlreadyPurchased);
        assert_eq!(purchase_status_from(2), NotPurchased);
        assert_eq!(purchase_status_from(3), NetworkError);
        assert_eq!(purchase_status_from(4), ServerError);
        assert_eq!(purchase_status_from(5), ServerError);
        assert_eq!(purchase_status_from(-1), ServerError);
    }
}

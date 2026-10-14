//! Rust side of the StoreKit 2 bridge (Mac App Store build only).
//!
//! NOT COMPILED ON A MAC YET. Written on Windows, where this file is excluded by
//! `#[cfg(target_os = "macos")]` in `mod.rs`. `build.rs` builds and links the Swift package on
//! macOS, and the Mac workflow (`.github/workflows/mac.yml`) is the first place it compiles. It
//! has never run against StoreKit.
//!
//! The Swift functions live in `apps/desktop/src-tauri/macos/StoreKitBridge/` and are declared in
//! its `include/storekit_bridge.h`. Each returns a NUL-terminated UTF-8 JSON string allocated with
//! `strdup`, which must be released with `storekit_free` exactly once. Each call blocks until
//! StoreKit answers or the Swift side times out, and the Swift side refuses to run on the main
//! thread, so call these from a background thread only.

use std::ffi::{CString, c_char, c_int};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use openframe_entitlement::mac as ent;
use serde::de::DeserializeOwned;

use super::storekit::convert;
use super::storekit::parse::{
    is_known_product, parse_answer, parse_environment, parse_products, parse_purchase, parse_update,
};
use super::storekit::{
    AppTransactionInfo, Entitlements, MAX_JSON_BYTES, ProductPrice, PurchaseOutcome,
    StoreEnvironment, StoreKitError, StoreUpdate,
};

// Linked by build.rs on macOS: the static library `StoreKitBridge`, the `StoreKit` and `Foundation`
// frameworks and the Swift runtime (docs/engineering/37-platform-boundary.md).
unsafe extern "C" {
    fn storekit_current_entitlements() -> *mut c_char;
    fn storekit_purchase(product_id: *const c_char) -> *mut c_char;
    fn storekit_app_transaction() -> *mut c_char;
    fn storekit_environment() -> *mut c_char;
    fn storekit_products(product_ids_json: *const c_char) -> *mut c_char;
    fn storekit_start_updates(callback: Option<extern "C" fn(*const c_char)>) -> c_int;
    fn storekit_free(ptr: *mut c_char);
}

/// Length of the C string at `ptr`, or `None` if no NUL byte comes within `max` bytes.
///
/// # Safety
/// `ptr` must point to a NUL-terminated string. The loop stops at the first NUL or after `max`
/// bytes, so it never reads past the terminator.
unsafe fn bounded_len(ptr: *const c_char, max: usize) -> Option<usize> {
    // SAFETY: every byte up to and including the terminator belongs to the string, and the search
    // stops at the first NUL.
    (0..max).find(|&i| unsafe { *ptr.add(i) } == 0)
}

/// Copy a bridge string into Rust (bounded) without freeing it.
///
/// # Safety
/// `ptr` must be null or a valid NUL-terminated string for the duration of the call.
unsafe fn copy_bounded(ptr: *const c_char) -> Result<String, StoreKitError> {
    if ptr.is_null() {
        return Err(StoreKitError::BadAnswer);
    }
    // SAFETY: the caller guarantees a NUL-terminated string.
    let len = unsafe { bounded_len(ptr, MAX_JSON_BYTES + 1) }.ok_or(StoreKitError::TooLarge)?;
    if len > MAX_JSON_BYTES {
        return Err(StoreKitError::TooLarge);
    }
    // SAFETY: `len` bytes before the terminator are part of the string.
    let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    String::from_utf8(bytes.to_vec()).map_err(|_| StoreKitError::BadAnswer)
}

/// Take ownership of a string returned by a bridge function: copy it, then free it once.
///
/// # Safety
/// `ptr` must be null or a pointer returned by a `storekit_*` function that was not freed yet.
unsafe fn take(ptr: *mut c_char) -> Result<String, StoreKitError> {
    // SAFETY: forwarded from the caller.
    let text = unsafe { copy_bounded(ptr) };
    if !ptr.is_null() {
        // SAFETY: the pointer came from the bridge (strdup) and is freed exactly once, here.
        unsafe { storekit_free(ptr) };
    }
    text
}

fn call<T: DeserializeOwned>(f: unsafe extern "C" fn() -> *mut c_char) -> Result<T, StoreKitError> {
    // SAFETY: the bridge functions take no arguments and return an owned string or null.
    let text = unsafe { take(f()) }?;
    parse_answer(&text)
}

type UpdateSink = Box<dyn Fn(Result<StoreUpdate, StoreKitError>) + Send + Sync>;
static UPDATE_SINK: OnceLock<UpdateSink> = OnceLock::new();

/// Called by Swift on its own task for each `Transaction.updates` element. The string is only
/// valid during the call, so it is copied, never freed here.
extern "C" fn on_update(json: *const c_char) {
    let result = std::panic::catch_unwind(|| {
        // SAFETY: Swift passes a NUL-terminated string that lives for the duration of this call.
        let update = unsafe { copy_bounded(json) }.and_then(|text| parse_update(&text));
        if let Some(sink) = UPDATE_SINK.get() {
            sink(update);
        }
    });
    if result.is_err() {
        tracing::error!("a StoreKit update could not be handled");
    }
}

/// The C functions of the Swift bridge, typed. Every method blocks; call it from a background
/// thread. The entitlement crate sees it through [`StoreKitChannel`].
#[derive(Debug, Default, Clone, Copy)]
pub struct StoreKitFfi;

impl StoreKitFfi {
    /// Verified current entitlements (`Transaction.currentEntitlements`). Unverified ones are only
    /// counted.
    pub fn current_entitlements(&self) -> Result<Entitlements, StoreKitError> {
        call(storekit_current_entitlements)
    }

    /// Buy one of the two known products. Pending (Ask to Buy) and cancelled are distinct.
    pub fn purchase(&self, product_id: &str) -> Result<PurchaseOutcome, StoreKitError> {
        if !is_known_product(product_id) {
            return Err(StoreKitError::BadArgument);
        }
        let id = CString::new(product_id).map_err(|_| StoreKitError::BadArgument)?;
        // SAFETY: `id` is a valid C string that outlives the call; the result is owned by us.
        let text = unsafe { take(storekit_purchase(id.as_ptr())) }?;
        parse_purchase(&text)
    }

    /// `AppTransaction.shared`, verified or not, with its signed form for the server.
    pub fn app_transaction(&self) -> Result<AppTransactionInfo, StoreKitError> {
        call(storekit_app_transaction)
    }

    /// The App Store environment from the verified app transaction.
    pub fn environment(&self) -> Result<StoreEnvironment, StoreKitError> {
        // SAFETY: no arguments; the result is owned by us.
        let text = unsafe { take(storekit_environment()) }?;
        parse_environment(&text)
    }

    /// Localised names and prices for the trial sheet. Display only.
    pub fn products(&self, ids: &[&str]) -> Result<Vec<ProductPrice>, StoreKitError> {
        if ids.is_empty() || ids.len() > 2 || !ids.iter().all(|id| is_known_product(id)) {
            return Err(StoreKitError::BadArgument);
        }
        let json = serde_json::to_string(ids).map_err(|_| StoreKitError::BadArgument)?;
        let json = CString::new(json).map_err(|_| StoreKitError::BadArgument)?;
        // SAFETY: `json` is a valid C string that outlives the call; the result is owned by us.
        let text = unsafe { take(storekit_products(json.as_ptr())) }?;
        parse_products(&text)
    }

    /// Start forwarding `Transaction.updates` (refunds, revocations, purchases made elsewhere) to
    /// `on_update`. Only the first call registers a sink; later calls are accepted and ignored.
    pub fn start_updates(
        &self,
        on_update_fn: fn(Result<StoreUpdate, StoreKitError>),
    ) -> Result<(), StoreKitError> {
        let _ = UPDATE_SINK.set(Box::new(on_update_fn));
        // SAFETY: `on_update` is a plain `extern "C"` function that lives for the whole program.
        let started = unsafe { storekit_start_updates(Some(on_update)) };
        if started == 1 {
            Ok(())
        } else {
            Err(StoreKitError::Store("updates_not_started".into()))
        }
    }
}

/// The Swift bridge as the entitlement crate's [`ent::StoreKitBridge`]. It maps every answer onto
/// the crate's types (see [`convert`]) and counts what was dropped. It decides nothing.
#[derive(Debug, Default)]
pub struct StoreKitChannel {
    ffi: StoreKitFfi,
    /// Asked once, from the verified `AppTransaction`, when the adapter is built (on the
    /// entitlement worker thread, never the main thread).
    environment: OnceLock<ent::StoreEnvironment>,
    unverified: AtomicU64,
}

impl ent::StoreKitBridge for StoreKitChannel {
    fn current_entitlements(&self) -> Result<Vec<ent::VerifiedTransaction>, ent::BridgeError> {
        let answer = self
            .ffi
            .current_entitlements()
            .map_err(convert::bridge_error)?;
        let filtered = convert::entitlements(answer);
        self.unverified
            .fetch_add(filtered.unverified, Ordering::Relaxed);
        if filtered.malformed > 0 || filtered.skipped > 0 {
            tracing::info!(
                malformed = filtered.malformed,
                skipped = filtered.skipped,
                "StoreKit transactions not used"
            );
        }
        Ok(filtered.transactions)
    }

    fn purchase(&self, product_id: &str) -> Result<ent::PurchaseOutcome, ent::BridgeError> {
        let outcome = self
            .ffi
            .purchase(product_id)
            .map_err(convert::bridge_error)?;
        if outcome == PurchaseOutcome::Unverified {
            self.unverified.fetch_add(1, Ordering::Relaxed);
        }
        convert::purchase(outcome)
    }

    fn app_transaction(&self) -> Result<ent::AppTransactionInfo, ent::BridgeError> {
        convert::app_transaction(self.ffi.app_transaction().map_err(convert::bridge_error)?)
    }

    /// The environment of the verified `AppTransaction`. When StoreKit cannot say (offline on
    /// the very first launch), release builds assume Production and development builds Sandbox
    /// for this session; a wrong guess only hides transactions, it never grants anything.
    fn environment(&self) -> ent::StoreEnvironment {
        *self
            .environment
            .get_or_init(|| match self.ffi.environment() {
                Ok(env) => convert::store_environment(env),
                Err(e) => {
                    tracing::warn!(
                        error = ?convert::bridge_error(e),
                        "the App Store environment is not known; using the build's default"
                    );
                    if cfg!(debug_assertions) {
                        ent::StoreEnvironment::Sandbox
                    } else {
                        ent::StoreEnvironment::Production
                    }
                }
            })
    }

    fn product_info(&self, product_id: &str) -> Result<ent::ProductInfo, ent::BridgeError> {
        let products = self
            .ffi
            .products(&[product_id])
            .map_err(convert::bridge_error)?;
        convert::product_info(products, product_id)
    }

    fn unverified_dropped(&self) -> u64 {
        self.unverified.load(Ordering::Relaxed)
    }
}

//! The Mac App Store side of the bridge (ADR-0027, ADR-0040).
//!
//! The SwiftUI shell implements StoreKit 2 and the Keychain. This file turns that Swift object into
//! the Rust trait the entitlement crate already defines ([`StoreKitBridge`]) and hands it to the
//! core, but only in a build of the store channel. The disk image build (channel
//! `DirectInstaller`) ignores it, so the internal build never talks to StoreKit.
//!
//! The Swift object speaks JSON strings, so no StoreKit type crosses the boundary and the shapes
//! stay the ones of `openframe_entitlement::mac::bridge`. Every answer is `{"ok":true,...}` or
//! `{"ok":false,"error":"<code>"}`. The rules of the bridge contract are applied again here, so a
//! Swift bug cannot get around them: only well-formed transactions pass (`keep_verified`), Apple's
//! dates travel unchanged, and an answer that is too large or not JSON is an error.
//!
//! One worker thread runs every check (launch, resume, StoreKit change, timer), so StoreKit and the
//! Keychain are never asked on the main thread and two checks never overlap. After each check it
//! tells the shell with an `entitlement` module event, so the screens read the status again.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use openframe_application::AppCore;
use openframe_application::events::{AppEvent, EventSink};
use openframe_entitlement::Channel;
use openframe_entitlement::mac::{
    AppTransactionInfo, BridgeError, InstantClock, MacInputs, PurchaseOutcome, RawVerification,
    RevalidateReason, StoreEnvironment, StoreKitBridge, StoreProducts, VerifiedTransaction,
    WallClock, keep_verified,
};
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::Value;

/// Largest answer from the Swift side. A larger answer is refused unread.
pub const MAX_JSON_BYTES: usize = 256 * 1024;

/// What the Swift shell implements with StoreKit 2 and the Keychain. Blocking: the core calls it
/// from its worker thread or from an operation thread, never from the main thread. Each answer is
/// JSON text, except the clock.
#[uniffi::export(callback_interface)]
pub trait StoreKitCallbacks: Send + Sync {
    /// `Transaction.currentEntitlements` and the latest record of each product:
    /// `{"ok":true,"transactions":[...],"unverified":n}`.
    fn current_entitlements(&self) -> String;
    /// `Product.purchase()`: `{"ok":true,"outcome":"success"|"pending"|"cancelled","transaction":{...}}`.
    fn purchase(&self, product_id: String) -> String;
    /// `AppTransaction.shared`: `{"ok":true,"appTransactionId":?,"environment":..}`.
    fn app_transaction(&self) -> String;
    /// The environment this build talks to: `{"ok":true,"environment":"sandbox"|"production"|"xcode"}`.
    fn environment(&self) -> String;
    /// `Product.products(for:)` for both products: `{"ok":true,"lifetimeDisplayPrice":..}`, or an
    /// error when fewer than both loaded.
    fn load_products(&self) -> String;
    /// `AppStore.sync()` and a refreshed signed record: `{"ok":true,"signedDateMs":?}`.
    fn sync(&self) -> String;
    /// The guard record from the data protection Keychain: `{"ok":true,"record":?}`.
    fn load_guard(&self) -> String;
    /// Store the guard record in the data protection Keychain: `{"ok":true}`.
    fn save_guard(&self, record_json: String) -> String;
    /// The Mac's clock, Unix milliseconds UTC.
    fn wall_clock_ms(&self) -> i64;
    /// Unverified results dropped since launch.
    fn unverified_dropped(&self) -> u64;
}

fn parse(text: &str) -> Result<Value, BridgeError> {
    if text.len() > MAX_JSON_BYTES {
        return Err(BridgeError::Other);
    }
    let value: Value = serde_json::from_str(text).map_err(|_| BridgeError::Other)?;
    match value.get("ok") {
        Some(Value::Bool(true)) => Ok(value),
        Some(Value::Bool(false)) => Err(bridge_error(
            value.get("error").and_then(Value::as_str).unwrap_or(""),
        )),
        _ => Err(BridgeError::Other),
    }
}

fn bridge_error(code: &str) -> BridgeError {
    match code {
        "notSignedIn" => BridgeError::NotSignedIn,
        "noConnection" => BridgeError::NoConnection,
        "storeUnavailable" => BridgeError::StoreUnavailable,
        "productNotFound" => BridgeError::ProductNotFound,
        "purchasesNotAllowed" => BridgeError::PurchasesNotAllowed,
        "unverified" => BridgeError::Unverified,
        _ => BridgeError::Other,
    }
}

fn field<T: for<'de> Deserialize<'de>>(value: &Value, name: &str) -> Result<T, BridgeError> {
    value
        .get(name)
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .ok_or(BridgeError::Other)
}

/// The Swift StoreKit object as the Rust trait.
pub(crate) struct JsonStoreKit {
    callbacks: Arc<dyn StoreKitCallbacks>,
}

impl JsonStoreKit {
    pub(crate) fn new(callbacks: Arc<dyn StoreKitCallbacks>) -> Self {
        Self { callbacks }
    }
}

impl StoreKitBridge for JsonStoreKit {
    fn current_entitlements(&self) -> Result<Vec<VerifiedTransaction>, BridgeError> {
        let answer = parse(&self.callbacks.current_entitlements())?;
        // A transaction that does not fit the shape is dropped like an unverified one: it cannot
        // be told apart from a corrupt one, and the policy must never see it.
        let raw: Vec<Value> = field(&answer, "transactions")?;
        let verified = raw
            .into_iter()
            .map(|v| serde_json::from_value::<VerifiedTransaction>(v).map_err(|_| ()))
            .map(|r| match r {
                Ok(tx) => RawVerification::Verified(tx),
                Err(()) => RawVerification::Unverified {
                    product_id: String::new(),
                },
            });
        let set = keep_verified(verified);
        if set.unverified + set.malformed > 0 {
            tracing::warn!(
                unverified = set.unverified,
                malformed = set.malformed,
                "StoreKit results were dropped"
            );
        }
        Ok(set.transactions)
    }

    fn purchase(&self, product_id: &str) -> Result<PurchaseOutcome, BridgeError> {
        let answer = parse(&self.callbacks.purchase(product_id.to_string()))?;
        let outcome: String = field(&answer, "outcome")?;
        match outcome.as_str() {
            "pending" => Ok(PurchaseOutcome::Pending),
            "cancelled" => Ok(PurchaseOutcome::Cancelled),
            "success" => {
                let tx: VerifiedTransaction = field(&answer, "transaction")?;
                // The answer must be for the product asked for, and well formed.
                if tx.product_id != product_id || !tx.is_well_formed() {
                    return Err(BridgeError::Unverified);
                }
                Ok(PurchaseOutcome::Success(tx))
            }
            _ => Err(BridgeError::Other),
        }
    }

    fn app_transaction(&self) -> Result<AppTransactionInfo, BridgeError> {
        let answer = parse(&self.callbacks.app_transaction())?;
        serde_json::from_value(answer).map_err(|_| BridgeError::Other)
    }

    /// Production when the Swift side cannot tell, so a cache of a real build is never mistaken
    /// for a test build's, and transactions from another environment are refused by the policy.
    fn environment(&self) -> StoreEnvironment {
        parse(&self.callbacks.environment())
            .and_then(|a| field(&a, "environment"))
            .unwrap_or(StoreEnvironment::Production)
    }

    fn load_products(&self) -> Result<StoreProducts, BridgeError> {
        let answer = parse(&self.callbacks.load_products())?;
        let price: String = field(&answer, "lifetimeDisplayPrice")?;
        if price.is_empty() || price.len() > 64 {
            return Err(BridgeError::Other);
        }
        Ok(StoreProducts {
            lifetime_display_price: price,
        })
    }

    fn sync(&self) -> Result<Option<i64>, BridgeError> {
        let answer = parse(&self.callbacks.sync())?;
        Ok(answer.get("signedDateMs").and_then(Value::as_i64))
    }

    fn load_guard(&self) -> Result<Option<String>, BridgeError> {
        let answer = parse(&self.callbacks.load_guard())?;
        Ok(answer
            .get("record")
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    fn save_guard(&self, record_json: &str) -> Result<(), BridgeError> {
        parse(&self.callbacks.save_guard(record_json.to_string())).map(|_| ())
    }

    fn unverified_dropped(&self) -> u64 {
        self.callbacks.unverified_dropped()
    }
}

/// The Mac's clock from the Swift object, so the shell's tests can move it.
struct SwiftWallClock(Arc<dyn StoreKitCallbacks>);

impl WallClock for SwiftWallClock {
    fn now_ms(&self) -> i64 {
        self.0.wall_clock_ms()
    }
}

/// The inputs of the store adapter, only for the store channel and only when the shell supplied the
/// Swift object. Any other channel gets `None`: the internal build ignores the bridge.
pub(crate) fn mac_inputs(
    channel: Channel,
    storekit: Option<Box<dyn StoreKitCallbacks>>,
) -> Option<MacInputs> {
    if channel != Channel::MacAppStore {
        return None;
    }
    let storekit: Arc<dyn StoreKitCallbacks> = Arc::from(storekit?);
    Some(MacInputs {
        bridge: Box::new(JsonStoreKit::new(storekit.clone())),
        clock: Box::new(InstantClock::new()),
        wall: Box::new(SwiftWallClock(storekit)),
    })
}

/// The reason a check was asked for, as the shell names it.
pub(crate) fn reason_from_name(name: &str) -> Option<RevalidateReason> {
    match name {
        "launch" => Some(RevalidateReason::Launch),
        "resume" => Some(RevalidateReason::Resume),
        "storeKitChanged" => Some(RevalidateReason::StoreKitChanged),
        _ => None,
    }
}

/// A resume this soon after the last check adds nothing.
const RESUME_DEBOUNCE: Duration = Duration::from_secs(60);

/// How often the worker looks at what is due. The clock screen's own check runs at this pace.
const LOOK: Duration = Duration::from_secs(5);

/// The thread that runs every check of the store channel.
pub(crate) struct StoreWorker {
    queue: Mutex<Option<Sender<RevalidateReason>>>,
}

impl StoreWorker {
    /// Start the worker and queue the launch check.
    pub(crate) fn start(core: &Arc<AppCore>, events: Arc<dyn EventSink>) -> Self {
        let (tx, rx) = mpsc::channel();
        let weak: Weak<AppCore> = Arc::downgrade(core);
        let spawned = std::thread::Builder::new()
            .name("openframe-entitlement".into())
            .spawn(move || {
                let mut last_check: Option<Instant> = None;
                let mut last_look = Instant::now();
                loop {
                    let asked = match rx.recv_timeout(LOOK) {
                        Ok(reason) => Some(reason),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => return,
                    };
                    // The state can change on an operation thread too (a restore, a purchase), so
                    // what is due is read fresh each time.
                    let due = weak.upgrade().and_then(|core| core.entitlement.next_check());
                    let timed = due
                        .filter(|(wait, _)| last_look.elapsed() >= *wait)
                        .map(|(_, reason)| reason);
                    let now = Instant::now();
                    let reason = asked.or(timed);
                    let Some(mut reason) = reason else { continue };
                    // Requests that queued up meanwhile are answered by this one check. Any
                    // reason other than a resume is kept, so it is never debounced away.
                    for queued in rx.try_iter() {
                        if reason == RevalidateReason::Resume {
                            reason = queued;
                        }
                    }
                    if reason == RevalidateReason::Resume
                        && last_check.is_some_and(|t| now.duration_since(t) < RESUME_DEBOUNCE)
                    {
                        continue;
                    }
                    let Some(core) = weak.upgrade() else { return };
                    let before = core.entitlement.state();
                    let after = core.entitlement.revalidate(reason);
                    last_look = Instant::now();
                    if reason != RevalidateReason::ClockTick {
                        last_check = Some(last_look);
                    }
                    // The clock screen looks every 5 seconds; only a change is worth telling.
                    if reason != RevalidateReason::ClockTick || before != after {
                        events.emit(&AppEvent::Module {
                            module: "entitlement".into(),
                            name: "changed".into(),
                            payload: Value::Null,
                        });
                    }
                }
            });
        if spawned.is_err() {
            tracing::warn!("the licence check thread could not start; the state stays as it is");
        }
        let _ = tx.send(RevalidateReason::Launch);
        Self {
            queue: Mutex::new(Some(tx)),
        }
    }

    /// Ask for a check. Never blocks.
    pub(crate) fn request(&self, reason: RevalidateReason) {
        if let Some(tx) = self.queue.lock().as_ref() {
            let _ = tx.send(reason);
        }
    }

    /// End the thread.
    pub(crate) fn stop(&self) {
        self.queue.lock().take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CoreConfig, CoreHandle, EventListener};
    use openframe_entitlement::mac::{LIFETIME_PRODUCT_ID, TRIAL_PRODUCT_ID};
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 2026-10-06 00:00 UTC.
    const T0: i64 = 1_791_244_800_000;
    const DAY: i64 = 86_400_000;

    fn transaction(product: &str, id: &str, env: &str, revoked: Option<i64>) -> Value {
        serde_json::json!({
            "productId": product,
            "transactionId": id,
            "originalTransactionId": id,
            "appTransactionId": "705000000001",
            "purchaseDateMs": T0,
            "originalPurchaseDateMs": T0,
            "signedDateMs": T0,
            "revocationDateMs": revoked,
            "environment": env,
        })
    }

    /// A Swift StoreKit stand-in whose answers the test sets.
    #[derive(Clone, Default)]
    struct FakeStoreKit(Arc<Mutex<FakeState>>);

    #[derive(Default)]
    struct FakeState {
        entitlements: Vec<Value>,
        purchases: VecDeque<String>,
        environment: String,
        sync_after: Option<Vec<Value>>,
        unverified: AtomicU64,
        syncs: u32,
        now_ms: i64,
        products_fail: bool,
        keychain: Option<String>,
    }

    impl FakeStoreKit {
        fn new(env: &str) -> Self {
            let f = Self::default();
            f.0.lock().environment = env.to_string();
            f.0.lock().now_ms = T0;
            f
        }
        fn set(&self, list: Vec<Value>) {
            self.0.lock().entitlements = list;
        }
        fn set_now(&self, ms: i64) {
            self.0.lock().now_ms = ms;
        }
        fn queue_purchase(&self, answer: Value) {
            self.0.lock().purchases.push_back(answer.to_string());
        }
    }

    impl StoreKitCallbacks for FakeStoreKit {
        fn current_entitlements(&self) -> String {
            let s = self.0.lock();
            serde_json::json!({"ok": true, "transactions": s.entitlements, "unverified": 0})
                .to_string()
        }
        fn purchase(&self, _product_id: String) -> String {
            let mut s = self.0.lock();
            let answer = s.purchases.pop_front().unwrap_or_else(|| {
                serde_json::json!({"ok": true, "outcome": "cancelled"}).to_string()
            });
            // A successful purchase shows up in the entitlements, as StoreKit does.
            if let Ok(v) = serde_json::from_str::<Value>(&answer)
                && let Some(t) = v.get("transaction")
            {
                s.entitlements.push(t.clone());
            }
            answer
        }
        fn app_transaction(&self) -> String {
            let env = self.0.lock().environment.clone();
            serde_json::json!({"ok": true, "appTransactionId": "705000000001", "environment": env})
                .to_string()
        }
        fn environment(&self) -> String {
            let env = self.0.lock().environment.clone();
            serde_json::json!({"ok": true, "environment": env}).to_string()
        }
        fn load_products(&self) -> String {
            if self.0.lock().products_fail {
                return r#"{"ok":false,"error":"noConnection"}"#.to_string();
            }
            serde_json::json!({"ok": true, "lifetimeDisplayPrice": "{test price}"}).to_string()
        }
        fn sync(&self) -> String {
            let mut s = self.0.lock();
            s.syncs += 1;
            if let Some(list) = s.sync_after.take() {
                s.entitlements = list;
            }
            r#"{"ok":true}"#.to_string()
        }
        fn load_guard(&self) -> String {
            serde_json::json!({"ok": true, "record": self.0.lock().keychain}).to_string()
        }
        fn save_guard(&self, record_json: String) -> String {
            self.0.lock().keychain = Some(record_json);
            r#"{"ok":true}"#.to_string()
        }
        fn wall_clock_ms(&self) -> i64 {
            self.0.lock().now_ms
        }
        fn unverified_dropped(&self) -> u64 {
            self.0.lock().unverified.load(Ordering::Relaxed)
        }
    }

    struct Quiet;
    impl EventListener for Quiet {
        fn on_event(&self, _event_json: String) {}
    }

    fn start(dir: &std::path::Path, store: bool, storekit: Option<FakeStoreKit>) -> Arc<CoreHandle> {
        let config = CoreConfig {
            app_version: "0.1.0".into(),
            isolated_data_root: Some(dir.to_string_lossy().into_owned()),
            ai_bundle_dir: None,
            write_logs: false,
            test_store_channel: store,
            test_library_setup: false,
            test_ai_deterministic: false,
        };
        CoreHandle::start(
            config,
            Box::new(Quiet),
            storekit.map(|s| Box::new(s) as Box<dyn StoreKitCallbacks>),
        )
        .expect("core starts")
    }

    fn status(core: &CoreHandle) -> Value {
        serde_json::from_str(
            &core
                .invoke("entitlement.status".into(), "{}".into())
                .unwrap(),
        )
        .unwrap()
    }

    /// Wait for the worker's launch check to answer.
    fn settled(core: &CoreHandle) -> Value {
        for _ in 0..200 {
            let s = status(core);
            if s["state"]["state"] != "needsOnlineCheck" {
                return s;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        status(core)
    }

    fn restore(core: &CoreHandle) {
        core.invoke("entitlement.restore_purchase".into(), "{}".into())
            .unwrap();
    }

    #[test]
    fn the_internal_channel_ignores_the_bridge() {
        let inputs = mac_inputs(
            Channel::DirectInstaller,
            Some(Box::new(FakeStoreKit::new("production"))),
        );
        assert!(inputs.is_none());
        let dir = tempfile::tempdir().unwrap();
        // No test switch: a debug build on a test folder is still the internal channel.
        let core = start(dir.path(), false, Some(FakeStoreKit::new("production")));
        let s = status(&core);
        assert_eq!(s["channel"], "directInstaller");
        assert_eq!(s["state"]["state"], "internalBuild");
        assert!(s["lifetimePrice"].is_null());
        core.request_store_check("launch".into());
        let notice: Value = serde_json::from_str(
            &core
                .invoke("entitlement.notice".into(), "{}".into())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(notice["notice"]["kind"], "none");
        core.stop();
    }

    #[test]
    fn the_store_channel_starts_the_trial_counts_days_offline_and_buys() {
        let dir = tempfile::tempdir().unwrap();
        let kit = FakeStoreKit::new("production");
        let core = start(dir.path(), true, Some(kit.clone()));
        let s = settled(&core);
        assert_eq!(s["channel"], "macAppStore");
        assert_eq!(s["state"]["state"], "trialNotStarted");
        assert_eq!(s["lifetimePrice"], "{test price}");

        // Start the trial: Apple records the purchase and the Mac counts 168 hours from it.
        kit.queue_purchase(serde_json::json!({
            "ok": true, "outcome": "success",
            "transaction": transaction(TRIAL_PRODUCT_ID, "2000000001", "production", None)
        }));
        let out: Value = serde_json::from_str(
            &core
                .invoke("entitlement.start_trial".into(), "{}".into())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(out["route"]["route"], "store");
        assert_eq!(out["status"]["state"]["state"], "trialActive", "{out}");
        assert_eq!(out["status"]["state"]["days_left"], 7);
        assert_eq!(out["status"]["trialEndsAtMs"], T0 + 7 * DAY);
        assert!(kit.0.lock().keychain.is_some(), "the guard record is kept");

        // Days go down with the Mac's clock, with no server and no network.
        kit.0.lock().products_fail = true;
        kit.set_now(T0 + 3 * DAY);
        restore(&core);
        assert_eq!(status(&core)["state"]["days_left"], 4);

        // The trial ends.
        kit.set_now(T0 + 8 * DAY);
        restore(&core);
        assert_eq!(status(&core)["state"]["state"], "trialEnded");

        // Buying the lifetime unlock from StoreKit alone.
        kit.queue_purchase(serde_json::json!({
            "ok": true, "outcome": "success",
            "transaction": transaction(LIFETIME_PRODUCT_ID, "2000000002", "production", None)
        }));
        let out: Value =
            serde_json::from_str(&core.invoke("entitlement.buy".into(), "{}".into()).unwrap())
                .unwrap();
        assert_eq!(out["status"]["state"]["state"], "purchased", "{out}");
        core.stop();
    }

    #[test]
    fn a_clock_turned_back_locks_the_whole_app_and_clears_by_itself_once_it_is_fixed() {
        let dir = tempfile::tempdir().unwrap();
        let kit = FakeStoreKit::new("production");
        kit.set(vec![transaction(TRIAL_PRODUCT_ID, "2000000001", "production", None)]);
        kit.set_now(T0 + 2 * DAY);
        let core = start(dir.path(), true, Some(kit.clone()));
        assert_eq!(settled(&core)["state"]["days_left"], 5);
        kit.set_now(T0);
        restore(&core);
        assert_eq!(status(&core)["state"]["state"], "clockChanged");
        let refused = core.invoke("entitlement.lock_ready".into(), "{}".into());
        assert!(refused.is_ok());
        let notice: Value = serde_json::from_str(
            &core
                .invoke("entitlement.notice".into(), "{}".into())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(notice["notice"]["kind"], "clockLocked");
        assert!(core.invoke("project.current".into(), "{}".into()).is_err());
        kit.set_now(T0 + 2 * DAY);
        for _ in 0..600 {
            if status(&core)["state"]["state"] == "trialActive" {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        assert_eq!(status(&core)["state"]["state"], "trialActive");
        core.stop();
    }

    #[test]
    fn only_restore_purchase_syncs_with_the_app_store() {
        let dir = tempfile::tempdir().unwrap();
        let kit = FakeStoreKit::new("production");
        let core = start(dir.path(), true, Some(kit.clone()));
        settled(&core);
        // The launch check and later checks never ask Apple to sync: that can open a sign-in window.
        for reason in ["storeKitChanged", "resume"] {
            core.request_store_check(reason.into());
        }
        core.request_store_check("storeKitChanged".into());
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(kit.0.lock().syncs, 0);
        restore(&core);
        assert_eq!(kit.0.lock().syncs, 1);
        core.stop();
    }

    #[test]
    fn restore_revocation_and_environment_mixing_follow_the_contract() {
        let dir = tempfile::tempdir().unwrap();
        let kit = FakeStoreKit::new("production");
        let core = start(dir.path(), true, Some(kit.clone()));
        settled(&core);

        // A purchase made on another Mac comes back with a restore.
        kit.0.lock().sync_after = Some(vec![transaction(
            LIFETIME_PRODUCT_ID,
            "2000000002",
            "production",
            None,
        )]);
        restore(&core);
        assert_eq!(status(&core)["state"]["state"], "purchased");

        // Apple refunds it.
        kit.set(vec![transaction(
            LIFETIME_PRODUCT_ID,
            "2000000002",
            "production",
            Some(T0 + 1000),
        )]);
        restore(&core);
        assert_eq!(status(&core)["state"]["state"], "revoked");

        // A sandbox purchase never unlocks a production build.
        kit.set(vec![transaction(
            LIFETIME_PRODUCT_ID,
            "2000000003",
            "sandbox",
            None,
        )]);
        restore(&core);
        assert_ne!(status(&core)["state"]["state"], "purchased");
        core.stop();
    }

    #[test]
    fn unverified_and_malformed_results_never_reach_the_policy() {
        let kit = FakeStoreKit::new("production");
        kit.set(vec![
            transaction(LIFETIME_PRODUCT_ID, "2000000002", "production", None),
            // Another app's product.
            transaction("com.other.app", "2000000004", "production", None),
            // Not a transaction at all.
            serde_json::json!({"productId": 7}),
            // A transaction with no signed date.
            {
                let mut t = transaction(TRIAL_PRODUCT_ID, "2000000005", "production", None);
                t.as_object_mut().unwrap().remove("signedDateMs");
                t
            },
        ]);
        let bridge = JsonStoreKit::new(Arc::new(kit.clone()));
        let kept = bridge.current_entitlements().unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].transaction_id, "2000000002");
        // The Swift counter is passed on.
        kit.0.lock().unverified.store(3, Ordering::Relaxed);
        assert_eq!(bridge.unverified_dropped(), 3);
    }

    #[test]
    fn answers_that_are_wrong_or_too_large_become_errors() {
        struct Broken(String);
        impl StoreKitCallbacks for Broken {
            fn current_entitlements(&self) -> String {
                self.0.clone()
            }
            fn purchase(&self, _: String) -> String {
                self.0.clone()
            }
            fn app_transaction(&self) -> String {
                self.0.clone()
            }
            fn environment(&self) -> String {
                self.0.clone()
            }
            fn load_products(&self) -> String {
                self.0.clone()
            }
            fn sync(&self) -> String {
                self.0.clone()
            }
            fn load_guard(&self) -> String {
                self.0.clone()
            }
            fn save_guard(&self, _: String) -> String {
                self.0.clone()
            }
            fn wall_clock_ms(&self) -> i64 {
                0
            }
            fn unverified_dropped(&self) -> u64 {
                0
            }
        }
        let big = format!(r#"{{"ok":true,"pad":"{}"}}"#, "x".repeat(MAX_JSON_BYTES));
        for text in [String::new(), "nope".into(), r#"{"ok":true}"#.into(), big] {
            let b = JsonStoreKit::new(Arc::new(Broken(text)));
            assert!(b.current_entitlements().is_err());
            assert!(b.purchase(TRIAL_PRODUCT_ID).is_err());
            assert!(b.app_transaction().is_err());
            assert!(b.load_products().is_err());
            assert_eq!(b.environment(), StoreEnvironment::Production);
        }
        let denied = JsonStoreKit::new(Arc::new(Broken(
            r#"{"ok":false,"error":"purchasesNotAllowed"}"#.into(),
        )));
        assert_eq!(
            denied.purchase(TRIAL_PRODUCT_ID),
            Err(BridgeError::PurchasesNotAllowed)
        );
        let offline = JsonStoreKit::new(Arc::new(Broken(
            r#"{"ok":false,"error":"noConnection"}"#.into(),
        )));
        assert_eq!(offline.sync(), Err(BridgeError::NoConnection));
        assert_eq!(offline.load_guard(), Err(BridgeError::NoConnection));
        assert_eq!(offline.save_guard("{}"), Err(BridgeError::NoConnection));
    }

    #[test]
    fn a_purchase_answer_for_another_product_is_refused() {
        let kit = FakeStoreKit::new("production");
        kit.queue_purchase(serde_json::json!({
            "ok": true, "outcome": "success",
            "transaction": transaction(LIFETIME_PRODUCT_ID, "2000000002", "production", None)
        }));
        let bridge = JsonStoreKit::new(Arc::new(kit));
        assert_eq!(
            bridge.purchase(TRIAL_PRODUCT_ID),
            Err(BridgeError::Unverified)
        );
    }

    #[test]
    fn the_test_store_switch_needs_a_debug_build_on_a_test_folder() {
        assert!(crate::test_switch(true, true, true));
        assert!(!crate::test_switch(true, false, true), "release build");
        assert!(!crate::test_switch(true, true, false), "the person's own data");
        assert!(!crate::test_switch(false, true, true), "not asked");
    }

    #[test]
    fn check_reasons_have_names_and_unknown_ones_are_ignored() {
        assert_eq!(reason_from_name("launch"), Some(RevalidateReason::Launch));
        assert_eq!(
            reason_from_name("storeKitChanged"),
            Some(RevalidateReason::StoreKitChanged)
        );
        assert_eq!(reason_from_name("resume"), Some(RevalidateReason::Resume));
        assert_eq!(reason_from_name("whatever"), None);
    }
}

//! The Mac App Store side of the bridge (ADR-0018, ADR-0027).
//!
//! The SwiftUI shell implements StoreKit 2 and the HTTPS call to the licensing server. This file
//! turns those two Swift objects into the Rust traits the entitlement crate already defines
//! ([`StoreKitBridge`] and [`LicenceServer`]) and hands them to the core, but only in a build of
//! the store channel. The disk image build (channel `DirectInstaller`) ignores both objects, so
//! the internal build never talks to StoreKit.
//!
//! The Swift objects speak JSON strings, so no StoreKit type crosses the boundary and the shapes
//! stay the ones of `openframe_entitlement::mac::bridge`. Every answer is `{"ok":true,...}` or
//! `{"ok":false,"error":"<code>"}`. The rules of the bridge contract are applied again here, so a
//! Swift bug cannot get around them: only well-formed transactions pass (`keep_verified`), Apple's
//! dates and the JWS travel unchanged, and an answer that is too large or not JSON is an error.
//!
//! One worker thread runs every check (launch, resume, StoreKit change, lease end), so StoreKit
//! and the network are never asked on the main thread and two checks never overlap. After each
//! check it tells the shell with an `entitlement` module event, so the screens read the status
//! again.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use ed25519_dalek::VerifyingKey;
use openframe_application::AppCore;
use openframe_application::events::{AppEvent, EventSink};
use openframe_entitlement::Channel;
use openframe_entitlement::mac::server::NONCE_BYTES;
use openframe_entitlement::mac::{
    AppTransactionInfo, BridgeError, InstantClock, LicenceServer, MacInputs, NonceSource,
    ProductInfo, PurchaseOutcome, RawVerification, RevalidateReason, ServerError, StoreEnvironment,
    StoreKitBridge, ValidateRequest, ValidateResponse, VerifiedTransaction, keep_verified,
};
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::Value;

/// Largest answer from the Swift side. A larger answer is refused unread.
pub const MAX_JSON_BYTES: usize = 256 * 1024;

/// The licensing server's Ed25519 response public key, 64 lowercase hex characters. PLACEHOLDER:
/// empty until the owner creates the key (docs/engineering/38-mac-app-store-trial.md). With no
/// valid key the adapter never sends a request and a trial is never confirmed, so the state is
/// "check needed" (open and export only).
pub const LICENCE_RESPONSE_PUBLIC_KEY_HEX: &str = "";

/// What the Swift shell implements with StoreKit 2. Blocking: the core calls it from its worker
/// thread or from an operation thread, never from the main thread. Each answer is JSON text.
#[uniffi::export(callback_interface)]
pub trait StoreKitCallbacks: Send + Sync {
    /// `Transaction.currentEntitlements`: `{"ok":true,"transactions":[...],"unverified":n}`.
    fn current_entitlements(&self) -> String;
    /// `Product.purchase()`: `{"ok":true,"outcome":"success"|"pending"|"cancelled","transaction":{...}}`.
    fn purchase(&self, product_id: String) -> String;
    /// `AppTransaction.shared`: `{"ok":true,"appTransactionId":?,"environment":..,"signedJws":..}`.
    fn app_transaction(&self) -> String;
    /// The environment this build talks to: `{"ok":true,"environment":"sandbox"|"production"|"xcode"}`.
    fn environment(&self) -> String;
    /// `Product.products(for:)`: `{"ok":true,"productId":..,"displayPrice":..}`.
    fn product_info(&self, product_id: String) -> String;
    /// `AppStore.sync()`: `{"ok":true}`.
    fn sync(&self) -> String;
    /// Unverified results dropped since launch.
    fn unverified_dropped(&self) -> u64;
}

/// What the Swift shell implements with `URLSession`: one POST to the licensing server.
#[uniffi::export(callback_interface)]
pub trait LicenceServerCallbacks: Send + Sync {
    /// `request_json` is the `ValidateRequest`. The answer is the server's `ValidateResponse`
    /// JSON (`{"payload":..,"signature":..}`) or `{"ok":false,"error":"noConnection"|"rejected"|"unavailable"}`.
    fn validate(&self, request_json: String) -> String;
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
    callbacks: Box<dyn StoreKitCallbacks>,
}

impl JsonStoreKit {
    pub(crate) fn new(callbacks: Box<dyn StoreKitCallbacks>) -> Self {
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
        let info: AppTransactionInfo =
            serde_json::from_value(answer).map_err(|_| BridgeError::Other)?;
        if !openframe_entitlement::mac::bridge::is_compact_jws(&info.signed_jws) {
            return Err(BridgeError::Unverified);
        }
        Ok(info)
    }

    /// Production when the Swift side cannot tell, so a cache of a real build is never mistaken
    /// for a test build's, and transactions from another environment are refused by the policy.
    fn environment(&self) -> StoreEnvironment {
        parse(&self.callbacks.environment())
            .and_then(|a| field(&a, "environment"))
            .unwrap_or(StoreEnvironment::Production)
    }

    fn product_info(&self, product_id: &str) -> Result<ProductInfo, BridgeError> {
        let answer = parse(&self.callbacks.product_info(product_id.to_string()))?;
        let id: String = field(&answer, "productId")?;
        let display_price: String = field(&answer, "displayPrice")?;
        if id != product_id || display_price.is_empty() || display_price.len() > 64 {
            return Err(BridgeError::Other);
        }
        Ok(ProductInfo {
            product_id: id,
            display_price,
        })
    }

    fn sync(&self) -> Result<(), BridgeError> {
        parse(&self.callbacks.sync()).map(|_| ())
    }

    fn unverified_dropped(&self) -> u64 {
        self.callbacks.unverified_dropped()
    }
}

/// The Swift HTTPS client as the Rust trait.
pub(crate) struct JsonLicenceServer {
    callbacks: Box<dyn LicenceServerCallbacks>,
}

impl LicenceServer for JsonLicenceServer {
    fn validate(&self, req: ValidateRequest) -> Result<ValidateResponse, ServerError> {
        let request = serde_json::to_string(&req).map_err(|_| ServerError::Unavailable)?;
        let text = self.callbacks.validate(request);
        if text.len() > MAX_JSON_BYTES {
            return Err(ServerError::Unavailable);
        }
        let value: Value = serde_json::from_str(&text).map_err(|_| ServerError::Unavailable)?;
        if value.get("ok") == Some(&Value::Bool(false)) {
            return Err(match value.get("error").and_then(Value::as_str) {
                Some("noConnection") => ServerError::NoConnection,
                Some("rejected") => ServerError::Rejected,
                _ => ServerError::Unavailable,
            });
        }
        // Only the shape is read here; the adapter checks the signature, nonce and environment.
        serde_json::from_value(value).map_err(|_| ServerError::Unavailable)
    }
}

/// Nonces from the operating system's secure random generator. `None` when it fails, and the
/// request is then not sent.
struct OsNonces;

impl NonceSource for OsNonces {
    fn nonce(&self) -> Option<[u8; NONCE_BYTES]> {
        let mut bytes = [0u8; NONCE_BYTES];
        getrandom::fill(&mut bytes).ok()?;
        Some(bytes)
    }
}

/// The response key from its hex form, or `None` for the placeholder or a malformed value.
pub(crate) fn response_key(hex_key: &str) -> Option<VerifyingKey> {
    let hex_key = hex_key.trim();
    if hex_key.len() != 64 || !hex_key.bytes().all(|b| b.is_ascii_hexdigit()) {
        if !hex_key.is_empty() {
            tracing::warn!("the licence response key is not valid; trials cannot be confirmed");
        }
        return None;
    }
    let mut bytes = [0u8; 32];
    for (i, pair) in hex_key.as_bytes().chunks(2).enumerate() {
        bytes[i] = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    VerifyingKey::from_bytes(&bytes).ok()
}

/// The inputs of the store adapter, only for the store channel and only when the shell supplied
/// both Swift objects. Any other channel gets `None`: the internal build ignores the bridge.
pub(crate) fn mac_inputs(
    channel: Channel,
    storekit: Option<Box<dyn StoreKitCallbacks>>,
    licence: Option<Box<dyn LicenceServerCallbacks>>,
    response_key_hex: &str,
) -> Option<MacInputs> {
    if channel != Channel::MacAppStore {
        return None;
    }
    let storekit = storekit?;
    let licence = licence?;
    Some(MacInputs {
        bridge: Box::new(JsonStoreKit::new(storekit)),
        server: Box::new(JsonLicenceServer { callbacks: licence }),
        nonces: Box::new(OsNonces),
        clock: Box::new(InstantClock::new()),
        response_key: response_key(response_key_hex),
    })
}

/// The reason a check was asked for, as the shell names it.
pub(crate) fn reason_from_name(name: &str) -> Option<RevalidateReason> {
    match name {
        "launch" => Some(RevalidateReason::Launch),
        "resume" => Some(RevalidateReason::Resume),
        "storeKitChanged" => Some(RevalidateReason::StoreKitChanged),
        "leaseExpired" => Some(RevalidateReason::LeaseExpired),
        _ => None,
    }
}

/// A resume this soon after the last check adds nothing.
const RESUME_DEBOUNCE: Duration = Duration::from_secs(60);

/// How often the worker wakes to look at the lease.
const TICK: Duration = Duration::from_secs(60);

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
                let mut lease_ends: Option<Instant> = None;
                loop {
                    let wait = lease_ends
                        .map_or(TICK, |end| end.saturating_duration_since(Instant::now()))
                        .min(TICK);
                    let asked = match rx.recv_timeout(wait) {
                        Ok(reason) => Some(reason),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => return,
                    };
                    let now = Instant::now();
                    let reason = asked.or_else(|| {
                        lease_ends
                            .is_some_and(|end| now >= end)
                            .then_some(RevalidateReason::LeaseExpired)
                    });
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
                    core.entitlement.revalidate(reason);
                    let done = Instant::now();
                    last_check = Some(done);
                    lease_ends = core.entitlement.lease().map(|lease| done + lease);
                    events.emit(&AppEvent::Module {
                        module: "entitlement".into(),
                        name: "changed".into(),
                        payload: Value::Null,
                    });
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
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};
    use openframe_entitlement::mac::bridge::{LIFETIME_PRODUCT_ID, TRIAL_PRODUCT_ID};
    use openframe_entitlement::mac::server::{RESPONSE_DOMAIN, ResponseBody, SCHEMA, TrialVerdict};
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU64, Ordering};

    const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    /// 2026-10-06 00:00 UTC.
    const T0: i64 = 1_791_244_800_000;

    fn transaction(product: &str, id: &str, env: &str, revoked: Option<i64>) -> Value {
        serde_json::json!({
            "productId": product,
            "transactionId": id,
            "originalTransactionId": id,
            "appTransactionId": "705000000001",
            "purchaseDateMs": T0,
            "revocationDateMs": revoked,
            "environment": env,
            "signedJws": "aGVhZA.Ym9keQ.c2ln",
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
    }

    impl FakeStoreKit {
        fn new(env: &str) -> Self {
            let f = Self::default();
            f.0.lock().environment = env.to_string();
            f
        }
        fn set(&self, list: Vec<Value>) {
            self.0.lock().entitlements = list;
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
            serde_json::json!({
                "ok": true, "appTransactionId": "705000000001",
                "environment": env, "signedJws": "aGVhZGVy.YXBw.c2ln"
            })
            .to_string()
        }
        fn environment(&self) -> String {
            let env = self.0.lock().environment.clone();
            serde_json::json!({"ok": true, "environment": env}).to_string()
        }
        fn product_info(&self, product_id: String) -> String {
            serde_json::json!({"ok": true, "productId": product_id, "displayPrice": "$89.00"})
                .to_string()
        }
        fn sync(&self) -> String {
            let mut s = self.0.lock();
            s.syncs += 1;
            if let Some(list) = s.sync_after.take() {
                s.entitlements = list;
            }
            r#"{"ok":true}"#.to_string()
        }
        fn unverified_dropped(&self) -> u64 {
            self.0.lock().unverified.load(Ordering::Relaxed)
        }
    }

    /// A licensing server that signs with a test key and counts the trial from Apple's date.
    #[derive(Clone)]
    struct FakeLicence {
        key: Arc<SigningKey>,
        now_ms: Arc<Mutex<i64>>,
    }

    impl FakeLicence {
        fn new(now_ms: i64) -> Self {
            Self {
                key: Arc::new(SigningKey::from_bytes(&[7u8; 32])),
                now_ms: Arc::new(Mutex::new(now_ms)),
            }
        }
        fn public_hex(&self) -> String {
            self.key
                .verifying_key()
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        }
    }

    impl LicenceServerCallbacks for FakeLicence {
        fn validate(&self, request_json: String) -> String {
            let req: ValidateRequest = serde_json::from_str(&request_json).unwrap();
            let now = *self.now_ms.lock();
            // The fake sees the JWS only; the test uses one trial at T0.
            let has_trial = !req.transaction_jws.is_empty();
            let ends = T0 + 168 * 3_600_000;
            let remaining = ((ends - now) / 1000).max(0);
            let body = ResponseBody {
                schema: SCHEMA,
                nonce: req.nonce.clone(),
                environment: req.environment,
                server_time_ms: now,
                trial: has_trial.then(|| TrialVerdict {
                    trial_ends_at_ms: ends,
                    remaining_seconds: remaining,
                    apple_transaction_id: "2000000001".into(),
                    lease_seconds: remaining.min(3600),
                }),
                lifetime: false,
                revoked: false,
            };
            let payload = serde_json::to_vec(&body).unwrap();
            let mut message = RESPONSE_DOMAIN.to_vec();
            message.extend_from_slice(&payload);
            let sig = self.key.sign(&message);
            serde_json::json!({
                "payload": B64.encode(&payload),
                "signature": B64.encode(sig.to_bytes()),
            })
            .to_string()
        }
    }

    struct Quiet;
    impl EventListener for Quiet {
        fn on_event(&self, _event_json: String) {}
    }

    fn start(
        dir: &std::path::Path,
        key_hex: Option<String>,
        storekit: Option<FakeStoreKit>,
        licence: Option<FakeLicence>,
    ) -> Arc<CoreHandle> {
        let config = CoreConfig {
            app_version: "0.1.0".into(),
            isolated_data_root: Some(dir.to_string_lossy().into_owned()),
            ai_bundle_dir: None,
            write_logs: false,
            test_team_password_hash: None,
            test_store_response_key_hex: key_hex,
            test_library_setup: false,
            test_ai_deterministic: false,
        };
        CoreHandle::start(
            config,
            Box::new(Quiet),
            storekit.map(|s| Box::new(s) as Box<dyn StoreKitCallbacks>),
            licence.map(|s| Box::new(s) as Box<dyn LicenceServerCallbacks>),
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

    #[test]
    fn the_internal_channel_ignores_the_bridge() {
        let inputs = mac_inputs(
            Channel::DirectInstaller,
            Some(Box::new(FakeStoreKit::new("production"))),
            Some(Box::new(FakeLicence::new(T0))),
            "",
        );
        assert!(inputs.is_none());
        let dir = tempfile::tempdir().unwrap();
        // No test switch: a debug build on a test folder is still the internal channel.
        let core = start(
            dir.path(),
            None,
            Some(FakeStoreKit::new("production")),
            Some(FakeLicence::new(T0)),
        );
        let s = status(&core);
        assert_eq!(s["channel"], "directInstaller");
        assert_eq!(s["state"]["state"], "internalBuild");
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
    fn the_store_channel_starts_the_trial_counts_days_and_buys() {
        let dir = tempfile::tempdir().unwrap();
        let kit = FakeStoreKit::new("production");
        let server = FakeLicence::new(T0 + 3_600_000);
        let core = start(
            dir.path(),
            Some(server.public_hex()),
            Some(kit.clone()),
            Some(server.clone()),
        );
        let s = settled(&core);
        assert_eq!(s["channel"], "macAppStore");
        assert_eq!(s["state"]["state"], "trialNotStarted");

        // Start the trial: Apple records the purchase, the server counts 168 hours from it.
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

        // Days go down from the server's clock, not the Mac's.
        *server.now_ms.lock() = T0 + 3 * 86_400_000;
        core.invoke("entitlement.restore_purchase".into(), "{}".into())
            .unwrap();
        assert_eq!(status(&core)["state"]["days_left"], 4);

        // The trial ends.
        *server.now_ms.lock() = T0 + 8 * 86_400_000;
        core.invoke("entitlement.restore_purchase".into(), "{}".into())
            .unwrap();
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
    fn only_restore_purchase_syncs_with_the_app_store() {
        let dir = tempfile::tempdir().unwrap();
        let kit = FakeStoreKit::new("production");
        let server = FakeLicence::new(T0);
        let core = start(
            dir.path(),
            Some(server.public_hex()),
            Some(kit.clone()),
            Some(server),
        );
        settled(&core);
        // The launch check and later checks never ask Apple to sync: that can open a sign-in window.
        for reason in ["storeKitChanged", "resume", "leaseExpired"] {
            core.request_store_check(reason.into());
        }
        core.request_store_check("storeKitChanged".into());
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(kit.0.lock().syncs, 0);
        core.invoke("entitlement.restore_purchase".into(), "{}".into())
            .unwrap();
        assert_eq!(kit.0.lock().syncs, 1);
        core.stop();
    }

    #[test]
    fn restore_revocation_and_environment_mixing_follow_the_contract() {
        let dir = tempfile::tempdir().unwrap();
        let kit = FakeStoreKit::new("production");
        let server = FakeLicence::new(T0);
        let core = start(
            dir.path(),
            Some(server.public_hex()),
            Some(kit.clone()),
            Some(server),
        );
        settled(&core);

        // A purchase made on another Mac comes back with a restore.
        kit.0.lock().sync_after = Some(vec![transaction(
            LIFETIME_PRODUCT_ID,
            "2000000002",
            "production",
            None,
        )]);
        core.invoke("entitlement.restore_purchase".into(), "{}".into())
            .unwrap();
        assert_eq!(status(&core)["state"]["state"], "purchased");

        // Apple refunds it.
        kit.set(vec![transaction(
            LIFETIME_PRODUCT_ID,
            "2000000002",
            "production",
            Some(T0 + 1000),
        )]);
        core.invoke("entitlement.restore_purchase".into(), "{}".into())
            .unwrap();
        assert_eq!(status(&core)["state"]["state"], "revoked");

        // A sandbox purchase never unlocks a production build.
        kit.set(vec![transaction(
            LIFETIME_PRODUCT_ID,
            "2000000003",
            "sandbox",
            None,
        )]);
        core.invoke("entitlement.restore_purchase".into(), "{}".into())
            .unwrap();
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
            // A JWS that is not three base64url parts.
            {
                let mut t = transaction(TRIAL_PRODUCT_ID, "2000000005", "production", None);
                t["signedJws"] = "not a jws".into();
                t
            },
        ]);
        let bridge = JsonStoreKit::new(Box::new(kit.clone()));
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
            fn product_info(&self, _: String) -> String {
                self.0.clone()
            }
            fn sync(&self) -> String {
                self.0.clone()
            }
            fn unverified_dropped(&self) -> u64 {
                0
            }
        }
        let big = format!(r#"{{"ok":true,"pad":"{}"}}"#, "x".repeat(MAX_JSON_BYTES));
        for text in [String::new(), "nope".into(), r#"{"ok":true}"#.into(), big] {
            let b = JsonStoreKit::new(Box::new(Broken(text)));
            assert!(b.current_entitlements().is_err());
            assert!(b.purchase(TRIAL_PRODUCT_ID).is_err());
            assert!(b.app_transaction().is_err());
            assert!(b.product_info(LIFETIME_PRODUCT_ID).is_err());
            assert_eq!(b.environment(), StoreEnvironment::Production);
        }
        let denied = JsonStoreKit::new(Box::new(Broken(
            r#"{"ok":false,"error":"purchasesNotAllowed"}"#.into(),
        )));
        assert_eq!(
            denied.purchase(TRIAL_PRODUCT_ID),
            Err(BridgeError::PurchasesNotAllowed)
        );
        let offline = JsonStoreKit::new(Box::new(Broken(
            r#"{"ok":false,"error":"noConnection"}"#.into(),
        )));
        assert_eq!(offline.sync(), Err(BridgeError::NoConnection));
    }

    #[test]
    fn a_purchase_answer_for_another_product_is_refused() {
        let kit = FakeStoreKit::new("production");
        kit.queue_purchase(serde_json::json!({
            "ok": true, "outcome": "success",
            "transaction": transaction(LIFETIME_PRODUCT_ID, "2000000002", "production", None)
        }));
        let bridge = JsonStoreKit::new(Box::new(kit));
        assert_eq!(
            bridge.purchase(TRIAL_PRODUCT_ID),
            Err(BridgeError::Unverified)
        );
    }

    #[test]
    fn the_test_store_switch_needs_a_debug_build_on_a_test_folder() {
        let config = CoreConfig {
            app_version: "0.1.0".into(),
            isolated_data_root: None,
            ai_bundle_dir: None,
            write_logs: false,
            test_team_password_hash: None,
            test_store_response_key_hex: Some("ab".repeat(32)),
            test_library_setup: false,
            test_ai_deterministic: false,
        };
        assert!(crate::test_store_key(&config, true, true).is_some());
        assert!(
            crate::test_store_key(&config, false, true).is_none(),
            "release build"
        );
        assert!(
            crate::test_store_key(&config, true, false).is_none(),
            "the person's own data"
        );
    }

    #[test]
    fn the_response_key_must_be_valid_hex_of_a_real_key() {
        assert!(response_key("").is_none());
        assert!(response_key("zz").is_none());
        assert!(response_key(&"g".repeat(64)).is_none());
        let key = SigningKey::from_bytes(&[9u8; 32]).verifying_key();
        let hex: String = key.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(response_key(&hex), Some(key));
    }

    #[test]
    fn check_reasons_have_names_and_unknown_ones_are_ignored() {
        assert_eq!(reason_from_name("launch"), Some(RevalidateReason::Launch));
        assert_eq!(
            reason_from_name("storeKitChanged"),
            Some(RevalidateReason::StoreKitChanged)
        );
        assert_eq!(reason_from_name("resume"), Some(RevalidateReason::Resume));
        assert_eq!(
            reason_from_name("leaseExpired"),
            Some(RevalidateReason::LeaseExpired)
        );
        assert_eq!(reason_from_name("whatever"), None);
    }
}

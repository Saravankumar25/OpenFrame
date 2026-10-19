//! Attack and failure tests for the Mac App Store channel, with a fake StoreKit bridge and a fake
//! licensing server. One test per attack or failure case named in the owner decision of
//! 2026-10-06.
//!
//! The fake "Apple" signs each transaction JWS with a keyed SHA-256 so the fake server can refuse
//! an edited JWS the way the real server refuses one that fails Apple's signature check. The fake
//! server signs its responses with a real Ed25519 key, exactly as the production server must.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::adapter::{
    CachedKind, MacAppStoreAdapter, MacInputs, MonotonicClock, NonceSource, RevalidateReason,
};
use super::bridge::{
    AppTransactionInfo, BridgeError, LIFETIME_PRODUCT_ID, ProductInfo, PurchaseOutcome,
    RawVerification, StoreEnvironment, StoreKitBridge, TRIAL_PRODUCT_ID, VerifiedTransaction,
    keep_verified,
};
use super::policy::{EntitlementsView, MacState, MacTrialPolicy, OnlineCheckReason, ServerView};
use super::server::{
    DEFAULT_LEASE_SECONDS, LicenceServer, NONCE_BYTES, ResponseBody, SCHEMA, ServerError,
    TrialVerdict, ValidateRequest, ValidateResponse, sign_response, verify_response,
};
use crate::{ChannelAdapter, EntitlementState, Permission, PurchaseStatus};

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// 2026-10-06 00:00 UTC.
const T0: i64 = 1_791_244_800_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 24 * HOUR_MS;
const DAY_S: i64 = 86_400;
const TRIAL_MS: i64 = MacTrialPolicy::TRIAL_SECONDS * 1000;
const PROD: StoreEnvironment = StoreEnvironment::Production;
const SANDBOX: StoreEnvironment = StoreEnvironment::Sandbox;
const APP_TX_ID: &str = "705000000001";

/// Stands in for Apple's signing key. Only the fake server knows it.
const FAKE_APPLE_SECRET: &[u8] = b"fake-apple-jws-signing-secret";

// ------------------------------------------------------------------ fake Apple JWS

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplePayload {
    transaction_id: String,
    original_transaction_id: String,
    product_id: String,
    app_transaction_id: Option<String>,
    purchase_date: i64,
    revocation_date: Option<i64>,
    environment: StoreEnvironment,
}

fn apple_mac(signing_input: &str) -> String {
    let mut h = Sha256::new();
    h.update(FAKE_APPLE_SECRET);
    h.update(signing_input.as_bytes());
    B64.encode(h.finalize())
}

fn apple_sign(p: &ApplePayload) -> String {
    let header = B64.encode(br#"{"alg":"ES256","x5c":["fake"]}"#);
    let body = B64.encode(serde_json::to_vec(p).unwrap());
    let input = format!("{header}.{body}");
    let sig = apple_mac(&input);
    format!("{input}.{sig}")
}

fn apple_verify(jws: &str) -> Option<ApplePayload> {
    let (input, sig) = jws.rsplit_once('.')?;
    if sig != apple_mac(input) {
        return None;
    }
    let (_, body) = input.split_once('.')?;
    serde_json::from_slice(&B64.decode(body).ok()?).ok()
}

/// Re-sign a transaction's JWS from its fields, as Apple would for a genuine transaction.
fn signed(mut t: VerifiedTransaction) -> VerifiedTransaction {
    t.signed_jws = apple_sign(&ApplePayload {
        transaction_id: t.transaction_id.clone(),
        original_transaction_id: t.original_transaction_id.clone(),
        product_id: t.product_id.clone(),
        app_transaction_id: t.app_transaction_id.clone(),
        purchase_date: t.purchase_date_ms,
        revocation_date: t.revocation_date_ms,
        environment: t.environment,
    });
    t
}

fn tx(product: &str, id: &str, purchase_ms: i64, env: StoreEnvironment) -> VerifiedTransaction {
    signed(VerifiedTransaction {
        product_id: product.to_string(),
        transaction_id: id.to_string(),
        original_transaction_id: id.to_string(),
        app_transaction_id: Some(APP_TX_ID.to_string()),
        purchase_date_ms: purchase_ms,
        revocation_date_ms: None,
        environment: env,
        signed_jws: String::new(),
    })
}

fn trial(id: &str, purchase_ms: i64) -> VerifiedTransaction {
    tx(TRIAL_PRODUCT_ID, id, purchase_ms, PROD)
}

fn lifetime(id: &str, purchase_ms: i64) -> VerifiedTransaction {
    tx(LIFETIME_PRODUCT_ID, id, purchase_ms, PROD)
}

fn revoked(mut t: VerifiedTransaction, at_ms: i64) -> VerifiedTransaction {
    t.revocation_date_ms = Some(at_ms);
    signed(t)
}

// ------------------------------------------------------------------ fake StoreKit bridge

struct BridgeState {
    env: StoreEnvironment,
    entitlements: Result<Vec<VerifiedTransaction>, BridgeError>,
    purchases: VecDeque<Result<PurchaseOutcome, BridgeError>>,
    purchase_calls: Vec<String>,
    sync_calls: u32,
    after_sync: Option<Vec<VerifiedTransaction>>,
}

#[derive(Clone)]
struct FakeBridge(Arc<Mutex<BridgeState>>);

impl FakeBridge {
    fn new(env: StoreEnvironment, entitlements: Vec<VerifiedTransaction>) -> Self {
        Self(Arc::new(Mutex::new(BridgeState {
            env,
            entitlements: Ok(entitlements),
            purchases: VecDeque::new(),
            purchase_calls: Vec::new(),
            sync_calls: 0,
            after_sync: None,
        })))
    }
    fn set(&self, entitlements: Result<Vec<VerifiedTransaction>, BridgeError>) {
        self.0.lock().unwrap().entitlements = entitlements;
    }
    fn queue_purchase(&self, outcome: Result<PurchaseOutcome, BridgeError>) {
        self.0.lock().unwrap().purchases.push_back(outcome);
    }
    fn purchase_calls(&self) -> Vec<String> {
        self.0.lock().unwrap().purchase_calls.clone()
    }
    fn entitlements(&self) -> Result<Vec<VerifiedTransaction>, BridgeError> {
        self.0.lock().unwrap().entitlements.clone()
    }
}

impl StoreKitBridge for FakeBridge {
    fn current_entitlements(&self) -> Result<Vec<VerifiedTransaction>, BridgeError> {
        self.entitlements()
    }
    fn purchase(&self, product_id: &str) -> Result<PurchaseOutcome, BridgeError> {
        let mut s = self.0.lock().unwrap();
        s.purchase_calls.push(product_id.to_string());
        let outcome = s
            .purchases
            .pop_front()
            .unwrap_or(Ok(PurchaseOutcome::Cancelled));
        if let (Ok(PurchaseOutcome::Success(t)), Ok(list)) = (&outcome, &mut s.entitlements)
            && t.is_well_formed()
        {
            list.push(t.clone());
        }
        outcome
    }
    fn app_transaction(&self) -> Result<AppTransactionInfo, BridgeError> {
        let env = self.0.lock().unwrap().env;
        Ok(AppTransactionInfo {
            app_transaction_id: Some(APP_TX_ID.to_string()),
            environment: env,
            signed_jws: "aGVhZGVy.YXBw.c2ln".to_string(),
        })
    }
    fn environment(&self) -> StoreEnvironment {
        self.0.lock().unwrap().env
    }
    fn product_info(&self, product_id: &str) -> Result<ProductInfo, BridgeError> {
        Ok(ProductInfo {
            product_id: product_id.to_string(),
            display_price: "\u{20ac}49,99".to_string(),
        })
    }
    fn sync(&self) -> Result<(), BridgeError> {
        let mut s = self.0.lock().unwrap();
        s.sync_calls += 1;
        if let Some(list) = s.after_sync.take() {
            s.entitlements = Ok(list);
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ fake licensing server

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Honest,
    Offline,
    Down,
    /// Edits the signed payload after signing (an on-path attacker).
    TamperPayload,
    /// Signs with a key the app does not know (a fake server).
    WrongKey,
    /// Returns the first response it ever sent (a recorded answer played back).
    ReplayFirst,
    /// Signs a verdict for the other environment.
    WrongEnvironment,
    /// Computes the trial from the latest trial purchase instead of the earliest.
    FromLatestTrial,
}

struct ServerState {
    key: SigningKey,
    time_ms: i64,
    lease_s: i64,
    mode: Mode,
    requests: Vec<ValidateRequest>,
    responses: Vec<ValidateResponse>,
}

#[derive(Clone)]
struct FakeServer(Arc<Mutex<ServerState>>);

impl FakeServer {
    fn new(time_ms: i64) -> Self {
        Self(Arc::new(Mutex::new(ServerState {
            key: SigningKey::generate(&mut rand::rngs::OsRng),
            time_ms,
            lease_s: DEFAULT_LEASE_SECONDS,
            mode: Mode::Honest,
            requests: Vec::new(),
            responses: Vec::new(),
        })))
    }
    fn public_key(&self) -> ed25519_dalek::VerifyingKey {
        self.0.lock().unwrap().key.verifying_key()
    }
    fn set_time(&self, ms: i64) {
        self.0.lock().unwrap().time_ms = ms;
    }
    fn set_mode(&self, mode: Mode) {
        self.0.lock().unwrap().mode = mode;
    }
    fn set_lease(&self, secs: i64) {
        self.0.lock().unwrap().lease_s = secs;
    }
    fn calls(&self) -> usize {
        self.0.lock().unwrap().requests.len()
    }
    fn requests(&self) -> Vec<ValidateRequest> {
        self.0.lock().unwrap().requests.clone()
    }
    fn bodies(&self) -> Vec<ResponseBody> {
        self.0
            .lock()
            .unwrap()
            .responses
            .iter()
            .map(|r| serde_json::from_slice(&B64.decode(&r.payload).unwrap()).unwrap())
            .collect()
    }
}

/// What an honest server answers.
fn honest_body(
    req: &ValidateRequest,
    time_ms: i64,
    lease_s: i64,
    from_latest: bool,
) -> Result<ResponseBody, ServerError> {
    let nonce_ok = req.nonce.len() == NONCE_BYTES * 2 && hex::decode(&req.nonce).is_ok();
    if req.schema != SCHEMA || !nonce_ok {
        return Err(ServerError::Rejected);
    }
    let mut trials = Vec::new();
    let (mut lifetime, mut revoked) = (false, false);
    for jws in &req.transaction_jws {
        // A JWS that fails Apple's signature refuses the whole request.
        let p = apple_verify(jws).ok_or(ServerError::Rejected)?;
        if p.environment != req.environment {
            continue;
        }
        if p.revocation_date.is_some() {
            revoked = true;
            continue;
        }
        match p.product_id.as_str() {
            TRIAL_PRODUCT_ID => trials.push(p),
            LIFETIME_PRODUCT_ID => lifetime = true,
            _ => {}
        }
    }
    trials.sort_by(|a, b| {
        (a.purchase_date, &a.transaction_id).cmp(&(b.purchase_date, &b.transaction_id))
    });
    let pick = if from_latest {
        trials.last()
    } else {
        trials.first()
    };
    let trial = pick.map(|p| {
        let ends = p.purchase_date + TRIAL_MS;
        let remaining_seconds = ((ends - time_ms) / 1000).max(0);
        TrialVerdict {
            trial_ends_at_ms: ends,
            remaining_seconds,
            apple_transaction_id: p.transaction_id.clone(),
            lease_seconds: lease_s.min(remaining_seconds),
        }
    });
    Ok(ResponseBody {
        schema: SCHEMA,
        nonce: req.nonce.clone(),
        environment: req.environment,
        server_time_ms: time_ms,
        trial,
        lifetime,
        revoked,
    })
}

impl LicenceServer for FakeServer {
    fn validate(&self, req: ValidateRequest) -> Result<ValidateResponse, ServerError> {
        let mut s = self.0.lock().unwrap();
        s.requests.push(req.clone());
        match s.mode {
            Mode::Offline => return Err(ServerError::NoConnection),
            Mode::Down => return Err(ServerError::Unavailable),
            Mode::ReplayFirst => {
                if let Some(first) = s.responses.first() {
                    return Ok(first.clone());
                }
            }
            _ => {}
        }
        let mut body = honest_body(&req, s.time_ms, s.lease_s, s.mode == Mode::FromLatestTrial)?;
        if s.mode == Mode::WrongEnvironment {
            body.environment = if req.environment == PROD {
                SANDBOX
            } else {
                PROD
            };
        }
        let mut resp = if s.mode == Mode::WrongKey {
            sign_response(&SigningKey::generate(&mut rand::rngs::OsRng), &body)
        } else {
            sign_response(&s.key, &body)
        };
        if s.mode == Mode::TamperPayload {
            let mut value: serde_json::Value =
                serde_json::from_slice(&B64.decode(&resp.payload).unwrap()).unwrap();
            if let Some(t) = value.get_mut("trial").and_then(|t| t.as_object_mut()) {
                let r = t["remainingSeconds"].as_i64().unwrap();
                t.insert("remainingSeconds".to_string(), (r + 30 * DAY_S).into());
            }
            resp.payload = B64.encode(serde_json::to_vec(&value).unwrap());
        }
        s.responses.push(resp.clone());
        Ok(resp)
    }
}

// ------------------------------------------------------------------ fake clock and nonces

#[derive(Clone, Default)]
struct FakeClock(Arc<AtomicU64>);

impl FakeClock {
    fn set(&self, secs: u64) {
        self.0.store(secs, Ordering::SeqCst);
    }
}

impl MonotonicClock for FakeClock {
    fn now(&self) -> Duration {
        Duration::from_secs(self.0.load(Ordering::SeqCst))
    }
}

/// Unique nonces. The app uses the system's secure random generator instead.
struct CountingNonces(AtomicU64);

impl NonceSource for CountingNonces {
    fn nonce(&self) -> Option<[u8; NONCE_BYTES]> {
        let n = self.0.fetch_add(1, Ordering::SeqCst) + 1;
        let mut out = [0xA5; NONCE_BYTES];
        out[..8].copy_from_slice(&n.to_le_bytes());
        Some(out)
    }
}

static NONCE_SEED: AtomicU64 = AtomicU64::new(0);

type Adapter = MacAppStoreAdapter<FakeBridge, FakeServer>;

/// One launch of the app: a new adapter with a new session and a new monotonic clock.
fn launch(bridge: &FakeBridge, server: &FakeServer, app_data: &Path) -> (Adapter, FakeClock) {
    let clock = FakeClock::default();
    // Every launch draws from a different range so no two launches share a nonce.
    let seed = NONCE_SEED.fetch_add(1 << 32, Ordering::SeqCst);
    let adapter = MacAppStoreAdapter::new(
        bridge.clone(),
        server.clone(),
        Some(server.public_key()),
        app_data,
        Box::new(CountingNonces(AtomicU64::new(seed))),
        Box::new(clock.clone()),
    );
    (adapter, clock)
}

fn active(secs: i64) -> MacState {
    MacState::TrialActive {
        remaining_seconds: secs,
    }
}

fn needs(reason: OnlineCheckReason) -> MacState {
    MacState::NeedsOnlineCheck { reason }
}

const NOT_STARTED: MacState = MacState::TrialNotStarted {
    purchase_pending: false,
};

fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

// ------------------------------------------------------------------ attack and failure tests

#[test]
fn clock_rolled_back_or_forward_changes_nothing_only_the_monotonic_session_clock_counts() {
    let bridge = FakeBridge::new(PROD, vec![trial("2000000001", T0)]);
    let server = FakeServer::new(T0 + 2 * DAY_MS);
    let dir = tempdir();
    let (a, clock) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(5 * DAY_S));

    // The request carries no reading of the Mac's clock: these are all of its fields.
    let req = serde_json::to_value(&server.requests()[0]).unwrap();
    let mut keys: Vec<_> = req.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "appTransactionJws",
            "environment",
            "nonce",
            "schema",
            "transactionJws"
        ]
    );

    // Session time on the monotonic clock moves the countdown.
    clock.set(3600);
    assert_eq!(a.mac_state(), active(5 * DAY_S - 3600));
    // A reading that goes back gives nothing back.
    clock.set(5);
    assert_eq!(a.mac_state(), active(5 * DAY_S - 3600));

    // The Mac's clock set a month back or a month forward is not an input to any call, so the
    // next launch gets exactly the server's count for the server's time.
    let (b, _) = launch(&bridge, &server, dir.path());
    assert_eq!(b.refresh(), active(5 * DAY_S));
    server.set_time(T0 + 2 * DAY_MS + HOUR_MS);
    let (c, _) = launch(&bridge, &server, dir.path());
    assert_eq!(c.refresh(), active(5 * DAY_S - 3600));
}

#[test]
fn time_zone_and_daylight_saving_changes_do_not_move_the_end_which_is_exactly_168_hours() {
    // 2026-10-31 12:00 UTC: the trial spans the end of daylight saving time in the United States
    // (1 November). Seven calendar days in a local time zone would be 169 hours there.
    let purchase = 1_793_448_000_000;
    let bridge = FakeBridge::new(PROD, vec![trial("2000000002", purchase)]);
    let server = FakeServer::new(purchase + TRIAL_MS - 1000);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(1));
    let body = server.bodies().pop().unwrap();
    assert_eq!(
        body.trial.unwrap().trial_ends_at_ms,
        purchase + 168 * HOUR_MS
    );

    server.set_time(purchase + 168 * HOUR_MS);
    let (b, _) = launch(&bridge, &server, dir.path());
    assert_eq!(b.refresh(), MacState::TrialEnded);
}

#[test]
fn reinstall_or_deleted_app_data_brings_the_same_state_back_from_the_store_and_server() {
    let t = trial("2000000003", T0);
    let bridge = FakeBridge::new(PROD, vec![t.clone()]);
    let server = FakeServer::new(T0 + DAY_MS);

    let first = tempdir();
    let (a, _) = launch(&bridge, &server, first.path());
    assert_eq!(a.refresh(), active(6 * DAY_S));
    assert!(a.cache_path().exists());
    drop(a);
    first.close().unwrap();

    // A clean install: no app data at all. The trial continues; it does not start again.
    let second = tempdir();
    let (b, _) = launch(&bridge, &server, second.path());
    assert_eq!(b.read_cache(), None);
    assert_eq!(b.refresh(), active(6 * DAY_S));

    // A lifetime purchase comes back the same way.
    bridge.set(Ok(vec![t, lifetime("2000000004", T0 + HOUR_MS)]));
    let third = tempdir();
    let (c, _) = launch(&bridge, &server, third.path());
    assert_eq!(c.refresh(), MacState::Lifetime);
}

#[test]
fn backup_or_vm_rollback_of_the_cache_never_grants() {
    let bridge = FakeBridge::new(PROD, vec![trial("2000000005", T0)]);
    let server = FakeServer::new(T0 + DAY_MS);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(6 * DAY_S));
    let backup = std::fs::read(a.cache_path()).unwrap();
    assert_eq!(a.read_cache().unwrap().last_state, CachedKind::TrialActive);

    server.set_time(T0 + 8 * DAY_MS);
    let (b, _) = launch(&bridge, &server, dir.path());
    assert_eq!(b.refresh(), MacState::TrialEnded);

    // Roll the cache back to day 1.
    std::fs::write(b.cache_path(), &backup).unwrap();
    let (c, _) = launch(&bridge, &server, dir.path());
    assert_eq!(c.refresh(), MacState::TrialEnded);

    // Rolled back and offline: still no editing, only the message wording follows the cache.
    std::fs::write(c.cache_path(), &backup).unwrap();
    server.set_mode(Mode::Offline);
    let (d, _) = launch(&bridge, &server, dir.path());
    let state = d.refresh();
    assert_eq!(state, needs(OnlineCheckReason::NoConnection));
    assert!(!state.permits(Permission::Edit));

    // A cache edited to claim a lifetime purchase grants nothing.
    server.set_mode(Mode::Honest);
    std::fs::write(
        d.cache_path(),
        br#"{"schema":1,"cacheOnly":true,"environment":"production","lastState":"lifetime"}"#,
    )
    .unwrap();
    let (e, _) = launch(&bridge, &server, dir.path());
    assert_eq!(e.refresh(), MacState::TrialEnded);
}

#[test]
fn cache_copied_from_another_mac_never_grants() {
    let bridge = FakeBridge::new(PROD, vec![]);
    let server = FakeServer::new(T0);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    std::fs::create_dir_all(a.cache_path().parent().unwrap()).unwrap();
    std::fs::write(
        a.cache_path(),
        br#"{"schema":1,"cacheOnly":true,"environment":"production","lastState":"lifetime"}"#,
    )
    .unwrap();
    assert_eq!(a.read_cache().unwrap().last_state, CachedKind::Lifetime);
    assert_eq!(a.refresh(), NOT_STARTED);
    assert_eq!(server.calls(), 0);
    assert_eq!(a.purchase_status(0), PurchaseStatus::NotPurchased);

    // A cache from a sandbox build, a cache not marked as one and an oversized file are not read.
    for text in [
        r#"{"schema":1,"cacheOnly":true,"environment":"sandbox","lastState":"lifetime"}"#
            .to_string(),
        r#"{"schema":1,"cacheOnly":false,"environment":"production","lastState":"lifetime"}"#
            .to_string(),
        "x".repeat(10_000),
    ] {
        std::fs::write(a.cache_path(), text).unwrap();
        assert_eq!(a.read_cache(), None);
    }

    // The cache never holds an id, a date or a JWS.
    let bridge = FakeBridge::new(PROD, vec![trial("2000000777", T0)]);
    let (b, _) = launch(&bridge, &server, dir.path());
    assert_eq!(b.refresh(), active(7 * DAY_S));
    let text = std::fs::read_to_string(b.cache_path()).unwrap();
    assert!(
        !text.contains("2000000777") && !text.contains(APP_TX_ID),
        "{text}"
    );
    assert!(
        !text.contains(&T0.to_string()) && !text.contains('.'),
        "{text}"
    );
}

#[test]
fn network_blocked_during_the_trial_needs_an_online_check_with_no_extension_and_no_restart() {
    let bridge = FakeBridge::new(PROD, vec![trial("2000000006", T0)]);
    let server = FakeServer::new(T0 + 2 * DAY_MS);
    let dir = tempdir();
    let (a, clock) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(5 * DAY_S));

    // The connection drops while the app is open: the confirmed count keeps running down.
    server.set_mode(Mode::Offline);
    clock.set(600);
    assert_eq!(a.refresh(), active(5 * DAY_S - 600));

    // Next launch with the network blocked: no editing until the check succeeds.
    server.set_time(T0 + 3 * DAY_MS);
    let (b, _) = launch(&bridge, &server, dir.path());
    let state = b.refresh();
    assert_eq!(state, needs(OnlineCheckReason::NoConnection));
    assert!(!state.permits(Permission::Edit));
    assert!(state.permits(Permission::Open) && state.permits(Permission::Export));
    assert_eq!(
        state.to_entitlement_state(),
        EntitlementState::NeedsOnlineCheck
    );
    assert!(
        b.online_check_message(OnlineCheckReason::NoConnection)
            .contains("continue your free trial")
    );

    // A server outage reads the same way.
    server.set_mode(Mode::Down);
    let (c, _) = launch(&bridge, &server, dir.path());
    assert_eq!(c.refresh(), needs(OnlineCheckReason::CouldNotConfirm));

    // Back online: the server's count for the real elapsed time, not a new 7 days.
    server.set_mode(Mode::Honest);
    assert_eq!(b.refresh(), active(4 * DAY_S));
}

#[test]
fn repeated_trial_purchase_uses_the_earliest_apple_date_and_never_extends() {
    let bridge = FakeBridge::new(
        PROD,
        vec![
            trial("2000000008", T0 + 5 * DAY_MS),
            trial("2000000007", T0),
        ],
    );
    let server = FakeServer::new(T0 + 6 * DAY_MS);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(DAY_S));
    assert_eq!(a.last_anomalies().extra_trial_purchases, 1);

    // Asking to start the trial again does not buy it again and changes nothing.
    assert_eq!(a.purchase_trial(), Ok(active(DAY_S)));
    assert!(bridge.purchase_calls().is_empty());

    // A server that counted from the later purchase is refused.
    server.set_mode(Mode::FromLatestTrial);
    let (b, _) = launch(&bridge, &server, dir.path());
    assert_eq!(b.refresh(), needs(OnlineCheckReason::CouldNotConfirm));
}

#[test]
fn same_apple_account_on_a_second_mac_gets_the_same_trial_end() {
    let t = trial("2000000009", T0);
    let mac_one = FakeBridge::new(PROD, vec![t.clone()]);
    let mac_two = FakeBridge::new(PROD, vec![t]);
    let server = FakeServer::new(T0 + DAY_MS);
    let (d1, d2) = (tempdir(), tempdir());
    let (a, _) = launch(&mac_one, &server, d1.path());
    let (b, _) = launch(&mac_two, &server, d2.path());
    assert_eq!(a.refresh(), active(6 * DAY_S));
    assert_eq!(b.refresh(), active(6 * DAY_S));

    let requests = server.requests();
    assert_eq!(requests[0].transaction_jws, requests[1].transaction_jws);
    assert_ne!(requests[0].nonce, requests[1].nonce);
    let bodies = server.bodies();
    assert_eq!(bodies[0].trial, bodies[1].trial);
    assert_eq!(
        bodies[0].trial.as_ref().unwrap().trial_ends_at_ms,
        T0 + TRIAL_MS
    );
}

#[test]
fn tampered_jws_is_rejected_and_never_reads_as_trial_not_started() {
    // The JWS payload edited to claim a later purchase, Apple's signature kept.
    let mut t = trial("2000000010", T0);
    let parts: Vec<&str> = t.signed_jws.split('.').collect();
    let mut payload: ApplePayload = serde_json::from_slice(&B64.decode(parts[1]).unwrap()).unwrap();
    payload.purchase_date = T0 + 6 * DAY_MS;
    let forged_body = B64.encode(serde_json::to_vec(&payload).unwrap());
    t.signed_jws = format!("{}.{}.{}", parts[0], forged_body, parts[2]);
    t.purchase_date_ms = T0 + 6 * DAY_MS;

    let bridge = FakeBridge::new(PROD, vec![t]);
    let server = FakeServer::new(T0 + 8 * DAY_MS);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    let state = a.refresh();
    assert_eq!(state, needs(OnlineCheckReason::CouldNotConfirm));
    assert_ne!(state, NOT_STARTED);
    assert!(!state.permits(Permission::Edit));
    assert!(
        a.online_check_message(OnlineCheckReason::CouldNotConfirm)
            .contains("could not confirm")
    );

    // The transaction fields edited and the JWS left genuine: the server answers from Apple's
    // signed date, and Apple's date wins. The trial is over, as Apple's date says, instead of
    // running two more days as the edited date would.
    let mut t = trial("2000000011", T0);
    t.purchase_date_ms = T0 + 6 * DAY_MS;
    bridge.set(Ok(vec![t]));
    let (b, _) = launch(&bridge, &server, dir.path());
    let state = b.refresh();
    assert_eq!(state, MacState::TrialEnded);
    assert!(!state.permits(Permission::Edit));

    // Edited to an earlier date than Apple's: a verdict that would end later than the local
    // record allows is refused.
    let mut t = trial("2000000012", T0 + 6 * DAY_MS);
    t.purchase_date_ms = T0;
    bridge.set(Ok(vec![t]));
    let (c, _) = launch(&bridge, &server, dir.path());
    assert_eq!(c.refresh(), needs(OnlineCheckReason::CouldNotConfirm));
}

#[test]
fn tampered_forged_or_replayed_server_response_is_refused() {
    for mode in [Mode::TamperPayload, Mode::WrongKey, Mode::WrongEnvironment] {
        let bridge = FakeBridge::new(PROD, vec![trial("2000000012", T0)]);
        let server = FakeServer::new(T0 + DAY_MS);
        server.set_mode(mode);
        let dir = tempdir();
        let (a, _) = launch(&bridge, &server, dir.path());
        assert_eq!(
            a.refresh(),
            needs(OnlineCheckReason::CouldNotConfirm),
            "{mode:?}"
        );
    }

    // A genuine day-1 answer recorded and played back on day 8.
    let bridge = FakeBridge::new(PROD, vec![trial("2000000013", T0)]);
    let server = FakeServer::new(T0 + DAY_MS);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(6 * DAY_S));
    server.set_time(T0 + 8 * DAY_MS);
    server.set_mode(Mode::ReplayFirst);
    let (b, _) = launch(&bridge, &server, dir.path());
    assert_eq!(b.refresh(), needs(OnlineCheckReason::CouldNotConfirm));
    // Played back into the session that first received it, it still fails the new nonce, and the
    // count from the genuine answer keeps running down instead.
    assert_eq!(a.refresh(), active(6 * DAY_S));
}

#[test]
fn app_store_signed_out_needs_an_online_check_during_the_trial_and_lifetime_keeps_working() {
    let bridge = FakeBridge::new(PROD, vec![trial("2000000014", T0)]);
    let server = FakeServer::new(T0 + DAY_MS);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(6 * DAY_S));

    bridge.set(Err(BridgeError::NotSignedIn));
    let calls = server.calls();
    let (b, _) = launch(&bridge, &server, dir.path());
    let state = b.refresh();
    assert_eq!(state, needs(OnlineCheckReason::AppStoreUnavailable));
    assert!(!state.permits(Permission::Edit));
    assert_eq!(server.calls(), calls, "nothing to send without StoreKit");
    assert!(
        b.online_check_message(OnlineCheckReason::AppStoreUnavailable)
            .contains("signed in to the App Store")
    );

    // Lifetime verified earlier in this session keeps working after a sign-out.
    let buyer = FakeBridge::new(PROD, vec![lifetime("2000000015", T0)]);
    let (c, _) = launch(&buyer, &server, dir.path());
    assert_eq!(c.refresh(), MacState::Lifetime);
    buyer.set(Err(BridgeError::NotSignedIn));
    assert_eq!(c.refresh(), MacState::Lifetime);
    assert_eq!(c.purchase_status(0), PurchaseStatus::Purchased);
    assert!(c.mac_state().permits(Permission::Edit));
}

#[test]
fn app_kept_open_past_expiry_ends_the_trial_during_the_session() {
    let bridge = FakeBridge::new(PROD, vec![trial("2000000016", T0)]);
    let server = FakeServer::new(T0 + TRIAL_MS - 10_000);
    let dir = tempdir();
    let (a, clock) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(10));
    clock.set(9);
    assert_eq!(a.mac_state(), active(1));
    assert!(a.mac_state().permits(Permission::Edit));
    clock.set(10);
    let state = a.mac_state();
    assert_eq!(state, MacState::TrialEnded);
    // ADR-0022: an ended trial on the Mac locks opening and exporting too.
    assert!(!state.permits(Permission::Edit));
    assert!(!state.permits(Permission::Open) && !state.permits(Permission::Export));
    clock.set(1_000_000);
    assert_eq!(a.mac_state(), MacState::TrialEnded);
    assert_eq!(a.purchase_status(0), PurchaseStatus::NotPurchased);
}

#[test]
fn interrupted_or_deferred_purchase_is_pending_and_grants_nothing() {
    let bridge = FakeBridge::new(PROD, vec![]);
    let server = FakeServer::new(T0);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), NOT_STARTED);
    let offer = a.start_trial().unwrap();
    assert_eq!(offer.duration_days, 7);
    assert_eq!(offer.lifetime_price_display, "\u{20ac}49,99");
    assert!(!offer.what_locks.is_empty());
    assert!(
        offer
            .what_locks
            .iter()
            .all(|s| !s.contains('\u{2014}') && !s.to_lowercase().contains("export"))
    );

    bridge.queue_purchase(Ok(PurchaseOutcome::Pending));
    let state = a.purchase_trial().unwrap();
    assert_eq!(
        state,
        MacState::TrialNotStarted {
            purchase_pending: true
        }
    );
    assert!(!state.permits(Permission::Edit));
    assert_eq!(
        state.to_entitlement_state(),
        EntitlementState::TrialNotStarted
    );
    assert_eq!(server.calls(), 0);
    assert_eq!(bridge.entitlements(), Ok(vec![]));

    // An interrupted purchase (the bridge reports an error) changes nothing either.
    bridge.queue_purchase(Err(BridgeError::NoConnection));
    assert_eq!(a.purchase_trial(), Err(BridgeError::NoConnection));
    assert!(!a.mac_state().permits(Permission::Edit));

    // The approval arrives later as a verified transaction; the next check picks it up.
    bridge.set(Ok(vec![trial("2000000017", T0 + HOUR_MS)]));
    server.set_time(T0 + HOUR_MS);
    assert_eq!(a.refresh(), active(MacTrialPolicy::TRIAL_SECONDS));
}

#[test]
fn cancelled_purchase_grants_nothing_and_a_completed_one_starts_the_trial() {
    let bridge = FakeBridge::new(PROD, vec![]);
    let server = FakeServer::new(T0);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), NOT_STARTED);
    bridge.queue_purchase(Ok(PurchaseOutcome::Cancelled));
    assert_eq!(a.purchase_trial(), Ok(NOT_STARTED));
    assert_eq!(bridge.purchase_calls(), [TRIAL_PRODUCT_ID]);
    assert_eq!(server.calls(), 0);

    bridge.queue_purchase(Ok(PurchaseOutcome::Success(trial("2000000018", T0))));
    assert_eq!(
        a.purchase_trial(),
        Ok(active(MacTrialPolicy::TRIAL_SECONDS))
    );

    // A cancelled lifetime purchase during the trial changes nothing.
    bridge.queue_purchase(Ok(PurchaseOutcome::Cancelled));
    assert_eq!(
        a.purchase_lifetime(),
        Ok(active(MacTrialPolicy::TRIAL_SECONDS))
    );
    bridge.queue_purchase(Ok(PurchaseOutcome::Success(lifetime("2000000019", T0))));
    assert_eq!(a.purchase_lifetime(), Ok(MacState::Lifetime));
}

#[test]
fn restore_purchases_brings_the_lifetime_unlock_back() {
    let bridge = FakeBridge::new(PROD, vec![]);
    bridge.0.lock().unwrap().after_sync = Some(vec![lifetime("2000000020", T0)]);
    let server = FakeServer::new(T0 + 30 * DAY_MS);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), NOT_STARTED);
    assert_eq!(a.restore(), Ok(MacState::Lifetime));
    assert_eq!(bridge.0.lock().unwrap().sync_calls, 1);
    assert_eq!(a.restore_purchase(0), PurchaseStatus::Purchased);
    assert_eq!(
        MacState::Lifetime.to_entitlement_state(),
        EntitlementState::Purchased
    );
}

#[test]
fn refund_or_revocation_ends_the_unlock_and_locks_open_and_export_too() {
    let t = trial("2000000021", T0);
    let l = lifetime("2000000022", T0 + DAY_MS);
    let bridge = FakeBridge::new(PROD, vec![t.clone(), l.clone()]);
    let server = FakeServer::new(T0 + 10 * DAY_MS);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), MacState::Lifetime);

    bridge.set(Ok(vec![t.clone(), revoked(l, T0 + 3 * DAY_MS)]));
    let state = a.refresh();
    assert_eq!(state, MacState::Revoked);
    // ADR-0022: a refund on the Mac locks opening and exporting too.
    assert!(!state.permits(Permission::Edit));
    assert!(!state.permits(Permission::Open) && !state.permits(Permission::Export));
    assert_eq!(a.purchase_status(0), PurchaseStatus::Revoked);
    assert_eq!(state.to_entitlement_state(), EntitlementState::Revoked);

    // A trial transaction Apple revoked.
    bridge.set(Ok(vec![revoked(t, T0 + DAY_MS)]));
    let (b, _) = launch(&bridge, &server, dir.path());
    assert_eq!(b.refresh(), MacState::Revoked);
}

#[test]
fn expiry_boundary_one_second_left_is_active_and_zero_is_ended() {
    let bridge = FakeBridge::new(PROD, vec![trial("2000000023", T0)]);
    let server = FakeServer::new(T0 + TRIAL_MS - 1000);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    let state = a.refresh();
    assert_eq!(state, active(1));
    assert!(state.permits(Permission::Edit));
    assert_eq!(
        state.to_entitlement_state(),
        EntitlementState::TrialActive { days_left: 1 }
    );

    server.set_time(T0 + TRIAL_MS);
    let (b, _) = launch(&bridge, &server, dir.path());
    assert_eq!(b.refresh(), MacState::TrialEnded);
}

#[test]
fn sandbox_transaction_in_a_production_build_is_ignored() {
    let sandbox_trial = tx(TRIAL_PRODUCT_ID, "2000000024", T0, SANDBOX);
    let sandbox_lifetime = tx(LIFETIME_PRODUCT_ID, "2000000025", T0, SANDBOX);
    let bridge = FakeBridge::new(PROD, vec![sandbox_trial, sandbox_lifetime.clone()]);
    let server = FakeServer::new(T0);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), NOT_STARTED);
    assert_eq!(a.last_anomalies().environment_mismatch, 2);
    assert_eq!(server.calls(), 0);

    // A real trial next to a sandbox lifetime: the trial counts, the sandbox purchase does not,
    // and only the production JWS is sent.
    bridge.set(Ok(vec![trial("2000000026", T0), sandbox_lifetime]));
    let (b, _) = launch(&bridge, &server, dir.path());
    assert_eq!(b.refresh(), active(7 * DAY_S));
    assert_eq!(server.requests().last().unwrap().transaction_jws.len(), 1);

    // A sandbox (TestFlight) build ignores production purchases the same way.
    let tester = FakeBridge::new(SANDBOX, vec![lifetime("2000000027", T0)]);
    let (c, _) = launch(&tester, &server, dir.path());
    assert_eq!(c.refresh(), NOT_STARTED);

    // A purchase that comes back from the other environment is refused.
    let bridge = FakeBridge::new(PROD, vec![]);
    bridge.queue_purchase(Ok(PurchaseOutcome::Success(tx(
        LIFETIME_PRODUCT_ID,
        "2000000028",
        T0,
        SANDBOX,
    ))));
    let (d, _) = launch(&bridge, &server, dir.path());
    assert_eq!(d.refresh(), NOT_STARTED);
    assert_eq!(d.purchase_lifetime(), Err(BridgeError::Unverified));
    assert_eq!(d.mac_state(), NOT_STARTED);
}

#[test]
fn corrupt_or_unverified_transaction_never_reaches_the_policy() {
    // The bridge contract filter.
    let good = trial("2000000029", T0);
    let mut corrupt = lifetime("2000000030", T0);
    corrupt.signed_jws = "not a jws".to_string();
    let set = keep_verified([
        RawVerification::Verified(good.clone()),
        RawVerification::Unverified {
            product_id: LIFETIME_PRODUCT_ID.to_string(),
        },
        RawVerification::Verified(corrupt.clone()),
    ]);
    assert_eq!(set.transactions, vec![good]);
    assert_eq!(set.unverified, 1);
    assert_eq!(set.malformed, 1);

    // A faulty bridge that passes a corrupt transaction anyway: the adapter drops it first.
    let bridge = FakeBridge::new(PROD, vec![corrupt.clone()]);
    let server = FakeServer::new(T0);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), NOT_STARTED);

    // A purchase that returns a corrupt transaction grants nothing.
    let bridge = FakeBridge::new(PROD, vec![]);
    bridge.queue_purchase(Ok(PurchaseOutcome::Success(corrupt)));
    let (b, _) = launch(&bridge, &server, dir.path());
    assert_eq!(b.refresh(), NOT_STARTED);
    assert_eq!(b.purchase_lifetime(), Err(BridgeError::Unverified));
    assert_eq!(b.mac_state(), NOT_STARTED);
}

#[test]
fn without_the_response_key_or_fresh_nonces_a_trial_is_never_confirmed() {
    let bridge = FakeBridge::new(PROD, vec![trial("2000000031", T0)]);
    let server = FakeServer::new(T0);
    let dir = tempdir();
    let no_key = MacAppStoreAdapter::new(
        bridge.clone(),
        server.clone(),
        None,
        dir.path(),
        Box::new(CountingNonces(AtomicU64::new(0))),
        Box::new(FakeClock::default()),
    );
    assert_eq!(no_key.refresh(), needs(OnlineCheckReason::CouldNotConfirm));
    assert!(no_key.limitation().is_some());
    assert_eq!(server.calls(), 0);

    struct Stuck;
    impl NonceSource for Stuck {
        fn nonce(&self) -> Option<[u8; NONCE_BYTES]> {
            Some([7; NONCE_BYTES])
        }
    }
    let stuck = MacAppStoreAdapter::new(
        bridge,
        server.clone(),
        Some(server.public_key()),
        dir.path(),
        Box::new(Stuck),
        Box::new(FakeClock::default()),
    );
    assert_eq!(stuck.refresh(), active(7 * DAY_S));
    // The same nonce twice is refused before anything is sent; the first answer keeps counting.
    assert_eq!(stuck.refresh(), active(7 * DAY_S));
    assert_eq!(server.calls(), 1);
}

#[test]
fn a_build_without_the_app_store_connection_needs_an_online_check_and_grants_nothing() {
    let dir = tempdir();
    let a = MacAppStoreAdapter::unavailable(dir.path());
    let state = a.refresh();
    assert_eq!(state, needs(OnlineCheckReason::AppStoreUnavailable));
    assert!(state.permits(Permission::Open) && state.permits(Permission::Export));
    assert!(!state.permits(Permission::Edit));
    assert_eq!(a.purchase_trial(), Err(BridgeError::StoreUnavailable));
    assert_eq!(a.purchase_lifetime(), Err(BridgeError::StoreUnavailable));
    assert_eq!(a.restore(), Err(BridgeError::StoreUnavailable));
    assert_eq!(a.mac_state(), needs(OnlineCheckReason::AppStoreUnavailable));
    assert_eq!(a.channel_state(), Some(EntitlementState::NeedsOnlineCheck));
    assert_eq!(
        a.restore_channel_state(),
        Some(EntitlementState::NeedsOnlineCheck)
    );
    assert_eq!(
        a.limitation(),
        Some("The App Store connection is not available in this build.")
    );
    assert!(a.read_cache().is_none(), "nothing learned, nothing cached");
    assert!(!a.cache_path().exists());
}

#[test]
fn boxed_shell_inputs_drive_the_same_adapter() {
    let bridge = FakeBridge::new(PROD, vec![trial("2000000041", T0)]);
    let server = FakeServer::new(T0 + DAY_MS);
    let dir = tempdir();
    let a = MacAppStoreAdapter::from_inputs(
        MacInputs {
            bridge: Box::new(bridge),
            server: Box::new(server.clone()),
            nonces: Box::new(CountingNonces(AtomicU64::new(1 << 60))),
            clock: Box::new(FakeClock::default()),
            response_key: Some(server.public_key()),
        },
        dir.path(),
    );
    assert_eq!(
        a.channel_state(),
        Some(EntitlementState::TrialActive { days_left: 6 })
    );
    assert_eq!(a.limitation(), None);
    assert!(
        a.read_cache().is_some(),
        "a connected build keeps the cache"
    );
}

// ------------------------------------------------------------------ lease and revalidation

#[test]
fn lease_expiry_during_a_session_locks_editing_until_a_new_check_succeeds() {
    let bridge = FakeBridge::new(PROD, vec![trial("2000000050", T0)]);
    let server = FakeServer::new(T0 + DAY_MS);
    server.set_lease(3600);
    let dir = tempdir();
    let (a, clock) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(6 * DAY_S));
    assert_eq!(a.time_until_check(), Some(Duration::from_secs(3600)));

    clock.set(3599);
    assert_eq!(a.mac_state(), active(6 * DAY_S - 3599));
    assert_eq!(a.time_until_check(), Some(Duration::from_secs(1)));
    clock.set(3600);
    let state = a.mac_state();
    assert_eq!(state, needs(OnlineCheckReason::LeaseExpired));
    assert!(!state.permits(Permission::Edit));
    assert!(state.permits(Permission::Open) && state.permits(Permission::Export));
    assert_eq!(a.time_until_check(), None);
    assert!(
        a.online_check_message(OnlineCheckReason::LeaseExpired)
            .contains("check your free trial again")
    );

    // The check fails: still locked, never a fresh countdown.
    server.set_mode(Mode::Offline);
    assert_eq!(
        a.revalidate(RevalidateReason::LeaseExpired),
        needs(OnlineCheckReason::LeaseExpired)
    );
    // The check succeeds: the server's count for its own clock, an hour later.
    server.set_mode(Mode::Honest);
    server.set_time(T0 + DAY_MS + HOUR_MS);
    assert_eq!(
        a.revalidate(RevalidateReason::LeaseExpired),
        active(6 * DAY_S - 3600)
    );
    assert_eq!(a.time_until_check(), Some(Duration::from_secs(3600)));
}

#[test]
fn revalidation_after_resume_with_the_server_unreachable_keeps_the_countdown_until_the_lease_ends_then_locks()
 {
    let bridge = FakeBridge::new(PROD, vec![trial("2000000051", T0)]);
    let server = FakeServer::new(T0 + 2 * DAY_MS);
    let dir = tempdir();
    let (a, clock) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(5 * DAY_S));

    // The Mac sleeps for a day (the monotonic clock does not move) and wakes with no network.
    clock.set(600);
    server.set_time(T0 + 3 * DAY_MS);
    server.set_mode(Mode::Offline);
    assert_eq!(
        a.revalidate(RevalidateReason::Resume),
        active(5 * DAY_S - 600)
    );
    let lease = u64::try_from(DEFAULT_LEASE_SECONDS).unwrap();
    assert_eq!(a.time_until_check(), Some(Duration::from_secs(lease - 600)));
    clock.set(lease - 1);
    assert!(a.mac_state().permits(Permission::Edit));
    clock.set(lease);
    assert_eq!(a.mac_state(), needs(OnlineCheckReason::LeaseExpired));
    assert_eq!(
        a.revalidate(RevalidateReason::LeaseExpired),
        needs(OnlineCheckReason::LeaseExpired)
    );

    // Back online: the server's count, which includes the day asleep.
    server.set_mode(Mode::Honest);
    assert_eq!(a.revalidate(RevalidateReason::Resume), active(4 * DAY_S));
}

#[test]
fn a_storekit_change_that_reports_a_revocation_locks_at_once() {
    let t = trial("2000000052", T0);
    let bridge = FakeBridge::new(PROD, vec![t.clone()]);
    let server = FakeServer::new(T0 + DAY_MS);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), active(6 * DAY_S));

    // Apple revokes the trial; StoreKit reports it through Transaction.updates. The server is
    // unreachable, and the lease has hours left: the revocation still locks at once.
    server.set_mode(Mode::Offline);
    bridge.set(Ok(vec![revoked(t, T0 + DAY_MS)]));
    let state = a.revalidate(RevalidateReason::StoreKitChanged);
    assert_eq!(state, MacState::Revoked);
    assert!(!state.permits(Permission::Edit));
    assert_eq!(a.time_until_check(), None);

    // A refunded lifetime purchase the same way.
    let l = lifetime("2000000053", T0);
    let buyer = FakeBridge::new(PROD, vec![l.clone()]);
    let (b, _) = launch(&buyer, &server, dir.path());
    assert_eq!(b.refresh(), MacState::Lifetime);
    buyer.set(Ok(vec![revoked(l, T0 + DAY_MS)]));
    assert_eq!(
        b.revalidate(RevalidateReason::StoreKitChanged),
        MacState::Revoked
    );

    // The server reporting a revocation StoreKit has not shown yet locks too.
    let c_bridge = FakeBridge::new(PROD, vec![trial("2000000054", T0)]);
    let honest = FakeServer::new(T0 + DAY_MS);
    let (c, _) = launch(&c_bridge, &honest, dir.path());
    assert_eq!(c.refresh(), active(6 * DAY_S));
    let mut with_refund = c_bridge.entitlements().unwrap();
    with_refund.push(revoked(lifetime("2000000055", T0), T0 + HOUR_MS));
    c_bridge.set(Ok(with_refund));
    assert_eq!(
        c.revalidate(RevalidateReason::StoreKitChanged),
        MacState::Revoked
    );
}

#[test]
fn revalidation_never_downgrades_a_verified_lifetime_when_the_server_or_storekit_is_unreachable() {
    let bridge = FakeBridge::new(
        PROD,
        vec![trial("2000000056", T0), lifetime("2000000057", T0 + DAY_MS)],
    );
    let server = FakeServer::new(T0 + 30 * DAY_MS);
    server.set_lease(60);
    let dir = tempdir();
    let (a, clock) = launch(&bridge, &server, dir.path());
    assert_eq!(a.refresh(), MacState::Lifetime);
    assert_eq!(server.calls(), 0, "lifetime needs no server");
    assert_eq!(a.time_until_check(), None);

    for (mode, reason) in [
        (Mode::Offline, RevalidateReason::Resume),
        (Mode::Down, RevalidateReason::LeaseExpired),
        (Mode::WrongKey, RevalidateReason::StoreKitChanged),
        (Mode::Offline, RevalidateReason::Launch),
    ] {
        server.set_mode(mode);
        clock.set(clock.0.load(Ordering::SeqCst) + 10 * 24 * 3600);
        assert_eq!(
            a.revalidate(reason),
            MacState::Lifetime,
            "{mode:?} {reason:?}"
        );
    }
    // StoreKit itself unreachable on a resume: the lifetime verified earlier in this session stays.
    bridge.set(Err(BridgeError::NoConnection));
    assert_eq!(a.revalidate(RevalidateReason::Resume), MacState::Lifetime);
    assert!(a.mac_state().permits(Permission::Edit));
}

/// A small deterministic generator, so the property test is the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

fn waits_for_a_check(app: &Adapter) -> bool {
    matches!(app.mac_state(), MacState::NeedsOnlineCheck { .. })
}

/// Runs one simulated life of the app for `days` of server time and returns the seconds during
/// which editing was allowed. Every time value is a multiple of `TICK`, so counting ticks is
/// exact.
fn usable_seconds(seed: u64, days: i64, events: bool) -> i64 {
    const TICK: u64 = 300;
    let mut rng = Lcg(seed);
    let bridge = FakeBridge::new(PROD, vec![trial("2000000060", T0)]);
    let server = FakeServer::new(T0);
    let leases = [600, 3600, DEFAULT_LEASE_SECONDS];
    server.set_lease(leases[usize::try_from(seed % 3).unwrap()]);
    let dir = tempdir();
    let (mut app, mut clock) = launch(&bridge, &server, dir.path());
    app.refresh();
    let (mut mono, mut server_ms) = (0u64, T0);
    let mut usable = 0i64;
    while server_ms < T0 + days * DAY_MS {
        if app.mac_state().permits(Permission::Edit) {
            usable += i64::try_from(TICK).unwrap();
        }
        // One tick awake: both clocks move together.
        mono += TICK;
        server_ms += i64::try_from(TICK).unwrap() * 1000;
        clock.set(mono);
        server.set_time(server_ms);
        if !events {
            if waits_for_a_check(&app) {
                app.revalidate(RevalidateReason::LeaseExpired);
            }
            continue;
        }
        match rng.next() % 100 {
            // Sleep: server time moves, the monotonic clock does not. Then wake.
            0..=1 => {
                let five_minutes = i64::try_from(rng.next() % (3 * 24 * 12)).unwrap();
                server_ms += five_minutes * 300_000;
                server.set_time(server_ms);
                app.revalidate(RevalidateReason::Resume);
            }
            // The network comes and goes.
            2..=3 => server.set_mode(Mode::Offline),
            4..=5 => server.set_mode(Mode::Honest),
            // Quit and launch again: a new session with a new monotonic clock.
            6 => {
                (app, clock) = launch(&bridge, &server, dir.path());
                mono = 0;
                app.refresh();
            }
            7 => {
                app.revalidate(RevalidateReason::StoreKitChanged);
            }
            // The shell's lease timer, and its Check again button.
            _ => {
                if waits_for_a_check(&app) {
                    app.revalidate(RevalidateReason::LeaseExpired);
                }
            }
        }
    }
    usable
}

#[test]
fn total_usable_time_across_any_sequence_of_leases_never_exceeds_168_hours() {
    let trial_s = MacTrialPolicy::TRIAL_SECONDS;
    // Always online, the lease timer always answered: exactly 168 hours, then nothing.
    for seed in 0..3 {
        assert_eq!(usable_seconds(seed, 10, false), trial_s, "seed {seed}");
    }
    // Sleeps of up to three days, the network dropping, relaunches, StoreKit changes and every
    // lease length: never more than 168 hours in all.
    for seed in 1..=12 {
        let usable = usable_seconds(seed, 12, true);
        assert!(usable <= trial_s, "seed {seed}: {usable} s");
        assert!(
            usable > 0,
            "seed {seed}: the trial was usable at some point"
        );
    }
}

// ------------------------------------------------------------------ the shared wire fixture

/// One request and the response the licence server signed for it with its test key, written by
/// `services/licence-server` (`write_the_shared_wire_fixture`). Both sides are tested against
/// these exact bytes.
const WIRE_FIXTURE: &str =
    include_str!("../../../../services/licence-server/tests/fixtures/mac-validate-v1.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireFixture {
    wire_version: u32,
    server_time_ms: i64,
    response_public_key: String,
    trial_purchase_date_ms: i64,
    request: serde_json::Value,
    response: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureTransaction {
    transaction_id: String,
    original_transaction_id: String,
    product_id: String,
    purchase_date: i64,
    environment: String,
    app_transaction_id: Option<String>,
}

#[test]
fn the_shared_wire_fixture_parses_and_verifies_against_the_servers_test_key() {
    let fixture: WireFixture = serde_json::from_str(WIRE_FIXTURE).unwrap();
    assert_eq!(fixture.wire_version, SCHEMA);

    // The request is exactly the app's request type: it parses and serializes back unchanged.
    let request: ValidateRequest = serde_json::from_value(fixture.request.clone()).unwrap();
    assert_eq!(serde_json::to_value(&request).unwrap(), fixture.request);
    assert_eq!(request.schema, SCHEMA);
    assert_eq!(request.environment, SANDBOX);
    assert_eq!(request.nonce.len(), NONCE_BYTES * 2);
    assert!(
        request
            .nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    assert!(request.app_transaction_jws.is_some());
    assert_eq!(request.transaction_jws.len(), 1);

    // The response verifies with the server's test public key and the request's nonce.
    let key_bytes: [u8; 32] = B64
        .decode(&fixture.response_public_key)
        .unwrap()
        .try_into()
        .unwrap();
    let key = ed25519_dalek::VerifyingKey::from_bytes(&key_bytes).unwrap();
    let response: ValidateResponse = serde_json::from_value(fixture.response.clone()).unwrap();
    let body = verify_response(&response, &key, &request.nonce).unwrap();
    assert_eq!(body.schema, SCHEMA);
    assert_eq!(body.environment, SANDBOX);
    assert_eq!(body.server_time_ms, fixture.server_time_ms);
    assert!(!body.lifetime && !body.revoked);
    let verdict = body.trial.clone().unwrap();
    assert_eq!(
        verdict.trial_ends_at_ms,
        fixture.trial_purchase_date_ms + TRIAL_MS
    );
    assert_eq!(
        verdict.remaining_seconds,
        (verdict.trial_ends_at_ms - fixture.server_time_ms) / 1000
    );
    assert_eq!(verdict.lease_seconds, DEFAULT_LEASE_SECONDS);

    // Another nonce, another key or an edited byte is refused.
    assert!(verify_response(&response, &key, &"0".repeat(64)).is_err());
    let other = SigningKey::generate(&mut rand::rngs::OsRng).verifying_key();
    assert!(verify_response(&response, &other, &request.nonce).is_err());
    let mut edited = response.clone();
    let swap = if &edited.payload[10..11] == "A" {
        "B"
    } else {
        "A"
    };
    edited.payload.replace_range(10..11, swap);
    assert!(verify_response(&edited, &key, &request.nonce).is_err());

    // The policy accepts the verdict for the transaction in the request.
    let jws = &request.transaction_jws[0];
    let payload = jws.split('.').nth(1).unwrap();
    let t: FixtureTransaction = serde_json::from_slice(&B64.decode(payload).unwrap()).unwrap();
    assert_eq!(t.environment, "Sandbox");
    assert_eq!(t.purchase_date, fixture.trial_purchase_date_ms);
    let tx = VerifiedTransaction {
        product_id: t.product_id,
        transaction_id: t.transaction_id,
        original_transaction_id: t.original_transaction_id,
        app_transaction_id: t.app_transaction_id,
        purchase_date_ms: t.purchase_date,
        revocation_date_ms: None,
        environment: SANDBOX,
        signed_jws: jws.clone(),
    };
    assert!(tx.is_well_formed());
    let derived = MacTrialPolicy::new(SANDBOX).derive_state(
        EntitlementsView::Known(std::slice::from_ref(&tx)),
        ServerView::Confirmed(&body),
        Duration::ZERO,
        false,
    );
    assert_eq!(derived.state, active(verdict.remaining_seconds));
}

#[test]
fn the_mac_module_never_reads_the_wall_clock() {
    let sources = [
        ("mod.rs", include_str!("mod.rs")),
        ("bridge.rs", include_str!("bridge.rs")),
        ("server.rs", include_str!("server.rs")),
        ("policy.rs", include_str!("policy.rs")),
        ("adapter.rs", include_str!("adapter.rs")),
        ("unavailable.rs", include_str!("unavailable.rs")),
        ("tests.rs", include_str!("tests.rs")),
    ];
    // Built at run time so this file does not match itself.
    let banned = [
        ["System", "Time"].concat(),
        ["now", "_ms"].concat(),
        ["UNIX", "_EPOCH"].concat(),
        ["chrono", "::"].concat(),
    ];
    for (name, source) in sources {
        for word in &banned {
            assert!(!source.contains(word.as_str()), "{name} contains {word}");
        }
    }
}

// ------------------------------------------------------------------ first-run and purchase screens

/// The trial, purchase and block screens reach the Mac only through the `ChannelAdapter` trait.
/// These tests drive that trait with the fake bridge and server; a real Mac and App Store are not
/// used (the screens are unverifiable on Windows).
#[test]
fn the_screens_buy_and_start_the_trial_through_the_adapter_trait() {
    use crate::notices::{Notice, NoticeRecord, due};
    use crate::{Channel, PurchaseFailure};

    let bridge = FakeBridge::new(PROD, vec![]);
    let server = FakeServer::new(T0);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    let adapter: &dyn ChannelAdapter = &a;
    let state = adapter.channel_state().unwrap();
    assert_eq!(state, EntitlementState::TrialNotStarted);
    let none = NoticeRecord::default();
    assert_eq!(
        due(Channel::MacAppStore, state, &none, None),
        Notice::MacFirstLaunch
    );

    // "Start free trial" (mock 187): Apple's sheet is cancelled, then completed.
    bridge.queue_purchase(Ok(PurchaseOutcome::Cancelled));
    assert_eq!(
        adapter.start_trial(),
        Some(Ok(EntitlementState::TrialNotStarted))
    );
    bridge.queue_purchase(Ok(PurchaseOutcome::Success(trial("2000000101", T0))));
    assert_eq!(
        adapter.start_trial(),
        Some(Ok(EntitlementState::TrialActive { days_left: 7 }))
    );
    assert_eq!(
        bridge.purchase_calls(),
        [TRIAL_PRODUCT_ID, TRIAL_PRODUCT_ID]
    );

    // "Buy full version" while offline: plain words, nothing granted.
    bridge.queue_purchase(Err(BridgeError::NoConnection));
    assert_eq!(
        adapter.buy_full_version(),
        Some(Err(PurchaseFailure {
            message: "OpenFrame could not reach the App Store. Connect to the internet, then try again."
        }))
    );
    // Then bought: the first-run screens are due.
    bridge.queue_purchase(Ok(PurchaseOutcome::Success(lifetime("2000000102", T0))));
    let bought = adapter.buy_full_version().unwrap().unwrap();
    assert_eq!(bought, EntitlementState::Purchased);
    assert_eq!(
        due(Channel::MacAppStore, bought, &none, None),
        Notice::DataNote
    );
    assert_eq!(
        bridge.purchase_calls().last().map(String::as_str),
        Some(LIFETIME_PRODUCT_ID)
    );
}

#[test]
fn an_expired_trial_blocks_and_restore_purchase_unblocks_through_the_adapter_trait() {
    use crate::Channel;
    use crate::notices::{Notice, NoticeRecord, due};

    let bridge = FakeBridge::new(PROD, vec![trial("2000000103", T0)]);
    bridge.0.lock().unwrap().after_sync = Some(vec![
        trial("2000000103", T0),
        lifetime("2000000104", T0 + DAY_MS),
    ]);
    let server = FakeServer::new(T0 + 8 * DAY_MS);
    let dir = tempdir();
    let (a, _) = launch(&bridge, &server, dir.path());
    let adapter: &dyn ChannelAdapter = &a;
    let state = adapter.channel_state().unwrap();
    assert_eq!(state, EntitlementState::TrialEnded);
    assert_eq!(
        due(Channel::MacAppStore, state, &NoticeRecord::default(), None),
        Notice::Locked
    );
    let restored = adapter.restore_channel_state().unwrap();
    assert_eq!(restored, EntitlementState::Purchased);
    assert_eq!(
        due(
            Channel::MacAppStore,
            restored,
            &NoticeRecord::default(),
            None
        ),
        Notice::DataNote
    );
}

#[test]
fn every_purchase_failure_has_plain_words_and_no_price() {
    use super::adapter::purchase_failure;
    for e in [
        BridgeError::NotSignedIn,
        BridgeError::NoConnection,
        BridgeError::StoreUnavailable,
        BridgeError::ProductNotFound,
        BridgeError::PurchasesNotAllowed,
        BridgeError::Unverified,
        BridgeError::Other,
    ] {
        let m = purchase_failure(e).message;
        assert!(m.ends_with('.') && !m.contains('\u{2014}'), "{m}");
        assert!(!m.contains('$') && !m.contains('\u{20ac}'), "no price: {m}");
    }
}

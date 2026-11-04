//! Table and failure tests for the offline Mac App Store trial (ADR-0040): every rule of the policy,
//! the clock guard G1 to G4, and the adapter over a fake StoreKit bridge and injected clocks.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::adapter::{
    MacAppStoreAdapter, MonotonicClock, PurchaseEnd, RevalidateReason, WallClock,
};
use super::bridge::{
    AppTransactionInfo, BridgeError, LIFETIME_PRODUCT_ID, PurchaseOutcome, RawVerification,
    StoreEnvironment, StoreKitBridge, StoreProducts, TRIAL_PRODUCT_ID, VerifiedTransaction,
    keep_verified,
};
use super::clock_guard::GuardRecord;
use super::policy::{
    CLOCK_TOLERANCE_MS, EntitlementsView, MacState, MacTrialPolicy, OnlineCheckReason, TrialInputs,
    cap_high_water_at_apple_date, recover_from_future_clock,
};
use crate::{ChannelAdapter, EntitlementState, Permission, PurchaseStatus};

/// 2026-10-06 00:00 UTC.
const T0: i64 = 1_791_244_800_000;
const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 60 * MINUTE_MS;
const DAY_MS: i64 = 24 * HOUR_MS;
const TRIAL_MS: i64 = MacTrialPolicy::TRIAL_SECONDS * 1000;
const PROD: StoreEnvironment = StoreEnvironment::Production;
const SANDBOX: StoreEnvironment = StoreEnvironment::Sandbox;

fn tx(product: &str, id: &str, at: i64) -> VerifiedTransaction {
    VerifiedTransaction {
        product_id: product.to_string(),
        transaction_id: id.to_string(),
        original_transaction_id: id.to_string(),
        app_transaction_id: Some("705000000001".to_string()),
        purchase_date_ms: at,
        original_purchase_date_ms: at,
        signed_date_ms: at,
        revocation_date_ms: None,
        environment: PROD,
    }
}

fn trial(at: i64) -> VerifiedTransaction {
    tx(TRIAL_PRODUCT_ID, "2000000001", at)
}

fn lifetime(at: i64) -> VerifiedTransaction {
    tx(LIFETIME_PRODUCT_ID, "2000000002", at)
}

fn revoked(mut t: VerifiedTransaction) -> VerifiedTransaction {
    t.revocation_date_ms = Some(t.purchase_date_ms + 1000);
    t
}

/// One evaluation of the policy with the App Store reachable and StoreKit answering.
fn online(txs: &[VerifiedTransaction], now: i64, g: &GuardRecord) -> super::policy::Derived {
    MacTrialPolicy::new(PROD).derive_state(TrialInputs {
        entitlements: EntitlementsView::Known(txs),
        store_reachable: true,
        now_wall_ms: now,
        wall_moved_back: false,
        clock_guard: g,
        purchase_pending: false,
    })
}

/// One evaluation with the App Store not answering. `currentEntitlements` still gives its cache.
fn offline(txs: &[VerifiedTransaction], now: i64, g: &GuardRecord) -> super::policy::Derived {
    MacTrialPolicy::new(PROD).derive_state(TrialInputs {
        entitlements: EntitlementsView::Known(txs),
        store_reachable: false,
        now_wall_ms: now,
        wall_moved_back: false,
        clock_guard: g,
        purchase_pending: false,
    })
}

fn days_left(state: MacState) -> u32 {
    match state.to_entitlement_state() {
        EntitlementState::TrialActive { days_left } => days_left,
        other => panic!("not a running trial: {other:?}"),
    }
}

// ------------------------------------------------------------------ the policy table, rule by rule

#[test]
fn rule_1_other_environments_and_malformed_transactions_are_ignored_and_counted() {
    let mut sandbox = lifetime(T0);
    sandbox.environment = SANDBOX;
    let mut broken = trial(T0);
    broken.signed_date_ms = 0;
    let d = online(&[sandbox.clone(), broken], T0, &GuardRecord::default());
    assert_eq!(
        d.state,
        MacState::TrialNotStarted {
            purchase_pending: false
        }
    );
    assert_eq!(d.anomalies.environment_mismatch, 1);
    assert_eq!(d.anomalies.malformed, 1);

    // Apple's reviewer: a sandbox transaction in a build that talks to the sandbox counts.
    let d = MacTrialPolicy::new(SANDBOX).derive_state(TrialInputs {
        entitlements: EntitlementsView::Known(std::slice::from_ref(&sandbox)),
        store_reachable: true,
        now_wall_ms: T0,
        wall_moved_back: false,
        clock_guard: &GuardRecord::default(),
        purchase_pending: false,
    });
    assert_eq!(d.state, MacState::Lifetime);
}

#[test]
fn rule_2_a_lifetime_transaction_wins_and_is_remembered() {
    let d = online(
        &[trial(T0), lifetime(T0 + 1)],
        T0 + 30 * DAY_MS,
        &GuardRecord::default(),
    );
    assert_eq!(d.state, MacState::Lifetime);
    assert!(d.clock_guard.lifetime_seen);
    assert_eq!(
        d.clock_guard.trial_start_ms,
        Some(T0),
        "the start is kept too"
    );
}

#[test]
fn rule_3_a_refunded_lifetime_gives_revoked_and_forgets_the_purchase() {
    let g = GuardRecord {
        lifetime_seen: true,
        ..GuardRecord::default()
    };
    let d = online(&[revoked(lifetime(T0))], T0 + DAY_MS, &g);
    assert_eq!(d.state, MacState::Revoked);
    assert!(
        !d.clock_guard.lifetime_seen,
        "an offline relaunch must not read as bought"
    );
    // A valid lifetime wins over a revoked one.
    let again = tx(LIFETIME_PRODUCT_ID, "2000000009", T0 + DAY_MS);
    let d = online(&[revoked(lifetime(T0)), again], T0 + DAY_MS, &g);
    assert_eq!(d.state, MacState::Lifetime);
}

#[test]
fn rule_4_nothing_known_and_the_store_reachable_is_not_started() {
    let d = online(&[], T0, &GuardRecord::default());
    assert_eq!(
        d.state,
        MacState::TrialNotStarted {
            purchase_pending: false
        }
    );
    assert_eq!(d.clock_guard.trial_start_ms, None);
}

#[test]
fn rule_5_a_revoked_trial_gives_revoked() {
    let d = online(&[revoked(trial(T0))], T0 + DAY_MS, &GuardRecord::default());
    assert_eq!(d.state, MacState::Revoked);
}

#[test]
fn rule_6_offline_with_no_transaction_never_reads_as_no_trial_yet() {
    // First launch offline, or a new Mac.
    let d = offline(&[], T0, &GuardRecord::default());
    assert_eq!(
        d.state,
        MacState::NeedsOnlineCheck {
            reason: OnlineCheckReason::NoConnection
        }
    );
    assert!(!d.state.permits(Permission::Edit));
    assert!(d.state.permits(Permission::Open));
    // A buyer is never locked because the network is down.
    let g = GuardRecord {
        lifetime_seen: true,
        ..GuardRecord::default()
    };
    assert_eq!(offline(&[], T0, &g).state, MacState::Lifetime);
    // A known trial goes on from the guard's start.
    let g = GuardRecord {
        trial_start_ms: Some(T0),
        high_water_ms: T0,
        ..GuardRecord::default()
    };
    assert_eq!(days_left(offline(&[], T0 + 3 * DAY_MS, &g).state), 4);
    assert_eq!(
        offline(&[], T0 + TRIAL_MS, &g).state,
        MacState::TrialEnded,
        "airplane mode on day 7 still ends the trial"
    );
    // StoreKit itself failed.
    let d = MacTrialPolicy::new(PROD).derive_state(TrialInputs {
        entitlements: EntitlementsView::Unavailable,
        store_reachable: false,
        now_wall_ms: T0,
        wall_moved_back: false,
        clock_guard: &GuardRecord::default(),
        purchase_pending: false,
    });
    assert_eq!(
        d.state,
        MacState::NeedsOnlineCheck {
            reason: OnlineCheckReason::AppStoreUnavailable
        }
    );
}

#[test]
fn a_store_that_answers_and_lists_no_purchase_decides_and_restore_is_the_way_back() {
    // Signed in with another account, online: the memory of a purchase does not stand in for the
    // store, so a borrowed sign-in does not unlock this Mac while it is online.
    let g = GuardRecord {
        lifetime_seen: true,
        ..GuardRecord::default()
    };
    assert_eq!(
        online(&[], T0, &g).state,
        MacState::TrialNotStarted {
            purchase_pending: false
        }
    );
}

#[test]
fn a_buyer_offline_is_never_locked_by_what_the_cached_list_holds() {
    let g = GuardRecord {
        lifetime_seen: true,
        trial_start_ms: Some(T0),
        high_water_ms: T0 + 90 * DAY_MS,
        ..GuardRecord::default()
    };
    // A trial is listed and the lifetime record is missing, the clock is even behind.
    assert_eq!(
        offline(&[trial(T0)], T0 + 30 * DAY_MS, &g).state,
        MacState::Lifetime
    );
    assert_eq!(
        offline(&[revoked(trial(T0))], T0 + 30 * DAY_MS, &g).state,
        MacState::Lifetime
    );
}

#[test]
fn a_known_trial_with_an_empty_list_online_is_not_offered_again() {
    let g = GuardRecord {
        trial_start_ms: Some(T0),
        high_water_ms: T0,
        ..GuardRecord::default()
    };
    assert_eq!(days_left(online(&[], T0 + DAY_MS, &g).state), 6);
    assert_eq!(online(&[], T0 + 8 * DAY_MS, &g).state, MacState::TrialEnded);
}

#[test]
fn rule_7_the_start_is_the_earlier_apple_date_and_never_moves_later() {
    // The earlier of the two dates on the transaction.
    let mut t = trial(T0);
    t.original_purchase_date_ms = T0 - DAY_MS;
    let d = online(&[t], T0, &GuardRecord::default());
    assert_eq!(d.clock_guard.trial_start_ms, Some(T0 - DAY_MS));
    assert_eq!(days_left(d.state), 6);

    // A second trial transaction, listed earlier or later, uses the earliest.
    let later = tx(TRIAL_PRODUCT_ID, "2000000003", T0 + 2 * DAY_MS);
    let d = online(
        &[later.clone(), trial(T0)],
        T0 + 2 * DAY_MS,
        &GuardRecord::default(),
    );
    assert_eq!(d.clock_guard.trial_start_ms, Some(T0));
    assert_eq!(d.anomalies.extra_trial_purchases, 1);

    // A restore that brings a later date never moves the guard's start.
    let g = GuardRecord {
        trial_start_ms: Some(T0),
        ..GuardRecord::default()
    };
    let d = online(&[later], T0 + 2 * DAY_MS, &g);
    assert_eq!(d.clock_guard.trial_start_ms, Some(T0));
    assert_eq!(days_left(d.state), 5);

    // Restore on a new Mac: the guard is empty and the verified transaction gives the start.
    let d = online(&[trial(T0)], T0 + 5 * DAY_MS, &GuardRecord::default());
    assert_eq!(d.clock_guard.trial_start_ms, Some(T0));
    assert_eq!(
        d.clock_guard.trial_original_tx.as_deref(),
        Some("2000000001")
    );
    assert_eq!(days_left(d.state), 2);
}

#[test]
fn rule_9_days_left_count_down_from_seven_to_one_and_then_the_trial_ends() {
    let g = GuardRecord::default();
    for (elapsed, expected) in [
        (0, 7),
        (1, 7),
        (DAY_MS, 6),
        (2 * DAY_MS, 5),
        (3 * DAY_MS, 4),
        (4 * DAY_MS, 3),
        (5 * DAY_MS, 2),
        (6 * DAY_MS, 1),
        (TRIAL_MS - 1, 1),
    ] {
        let state = online(&[trial(T0)], T0 + elapsed, &g).state;
        assert_eq!(days_left(state), expected, "{elapsed}");
        assert_eq!(state.trial_ends_at_ms(), Some(T0 + TRIAL_MS));
    }
    assert_eq!(
        online(&[trial(T0)], T0 + TRIAL_MS, &g).state,
        MacState::TrialEnded
    );
    assert_eq!(
        online(&[trial(T0)], T0 + 30 * DAY_MS, &g).state,
        MacState::TrialEnded
    );
}

// ------------------------------------------------------------------ clock guard G1 to G4

#[test]
fn g1_the_floor_is_the_latest_of_the_high_water_mark_apple_and_what_was_read_now() {
    // Apple signed the record later than the clock: the real time is never earlier.
    let mut t = trial(T0);
    t.signed_date_ms = T0 + 3 * DAY_MS;
    let d = online(&[t], T0 + DAY_MS, &GuardRecord::default());
    assert_eq!(d.state, MacState::ClockChanged);
    assert_eq!(d.clock_guard.last_apple_signed_ms, T0 + 3 * DAY_MS);

    let g = GuardRecord {
        trial_start_ms: Some(T0),
        last_apple_signed_ms: T0 + 3 * DAY_MS,
        ..GuardRecord::default()
    };
    assert_eq!(offline(&[], T0 + DAY_MS, &g).state, MacState::ClockChanged);
}

#[test]
fn g2_a_clock_back_by_an_hour_a_day_or_a_week_locks_and_five_minutes_do_not() {
    let g = GuardRecord {
        trial_start_ms: Some(T0),
        high_water_ms: T0 + 6 * DAY_MS,
        ..GuardRecord::default()
    };
    let now = T0 + 6 * DAY_MS;
    for back in [HOUR_MS, DAY_MS, 7 * DAY_MS] {
        let d = online(&[trial(T0)], now - back, &g);
        assert_eq!(d.state, MacState::ClockChanged, "{back}");
        assert_eq!(d.clock_guard.high_water_ms, now, "the floor is not lowered");
        assert!(!d.state.permits(Permission::Open));
    }
    // Normal corrections pass.
    assert_eq!(
        days_left(online(&[trial(T0)], now - CLOCK_TOLERANCE_MS, &g).state),
        2
    );
    assert_eq!(
        online(&[trial(T0)], now - CLOCK_TOLERANCE_MS - 1, &g).state,
        MacState::ClockChanged
    );
    // Offline as well.
    assert_eq!(offline(&[], now - DAY_MS, &g).state, MacState::ClockChanged);
}

#[test]
fn g2_never_locks_a_buyer_or_a_person_with_no_trial() {
    let g = GuardRecord {
        high_water_ms: T0 + 100 * DAY_MS,
        ..GuardRecord::default()
    };
    assert_eq!(online(&[lifetime(T0)], T0, &g).state, MacState::Lifetime);
    assert_eq!(
        online(&[], T0, &g).state,
        MacState::TrialNotStarted {
            purchase_pending: false
        }
    );
}

#[test]
fn g3_the_high_water_mark_rises_with_the_clock_and_never_falls() {
    let g = GuardRecord::default();
    let d = online(&[trial(T0)], T0 + DAY_MS, &g);
    assert_eq!(d.clock_guard.high_water_ms, T0 + DAY_MS);
    let d2 = online(&[trial(T0)], T0 + DAY_MS - MINUTE_MS, &d.clock_guard);
    assert_eq!(d2.clock_guard.high_water_ms, T0 + DAY_MS);

    // The wall clock moved back while the app ran.
    let d = MacTrialPolicy::new(PROD).derive_state(TrialInputs {
        entitlements: EntitlementsView::Known(&[trial(T0)]),
        store_reachable: true,
        now_wall_ms: T0 + DAY_MS,
        wall_moved_back: true,
        clock_guard: &g,
        purchase_pending: false,
    });
    assert_eq!(d.state, MacState::ClockChanged);
}

#[test]
fn g4_a_fresh_apple_date_near_the_clock_clears_a_wrong_future_high_water_mark() {
    let now = T0 + DAY_MS;
    let mut g = GuardRecord {
        trial_start_ms: Some(T0),
        high_water_ms: T0 + 365 * DAY_MS, // set to next year by accident, then fixed
        ..GuardRecord::default()
    };
    assert_eq!(online(&[trial(T0)], now, &g).state, MacState::ClockChanged);
    // A date too far from the clock proves nothing.
    assert!(!recover_from_future_clock(
        &mut g,
        Some(now - 11 * MINUTE_MS),
        now
    ));
    assert!(!recover_from_future_clock(&mut g, None, now));
    assert_eq!(g.high_water_ms, T0 + 365 * DAY_MS);
    // Within 10 minutes it does.
    assert!(recover_from_future_clock(
        &mut g,
        Some(now - 9 * MINUTE_MS),
        now
    ));
    assert_eq!(g.high_water_ms, now);
    assert_eq!(days_left(online(&[trial(T0)], now, &g).state), 6);
    // It cannot give days back: the start did not move.
    assert_eq!(g.trial_start_ms, Some(T0));
}

#[test]
fn g4_fallback_caps_the_high_water_mark_at_the_last_apple_date_plus_two_days() {
    let mut g = GuardRecord {
        high_water_ms: T0 + 365 * DAY_MS,
        last_apple_signed_ms: T0,
        ..GuardRecord::default()
    };
    assert!(cap_high_water_at_apple_date(&mut g));
    assert_eq!(g.high_water_ms, T0 + 2 * DAY_MS);
    assert!(!cap_high_water_at_apple_date(&mut g), "nothing more to do");
    let mut none = GuardRecord {
        high_water_ms: T0,
        ..GuardRecord::default()
    };
    assert!(
        !cap_high_water_at_apple_date(&mut none),
        "no Apple date known"
    );
}

// ------------------------------------------------------------------ states

#[test]
fn only_lifetime_and_a_running_trial_permit_editing_and_a_locked_state_permits_nothing() {
    use MacState::*;
    for s in [
        Lifetime,
        TrialActive {
            remaining_seconds: 1,
            ends_at_ms: T0,
        },
        TrialEnded,
        TrialNotStarted {
            purchase_pending: true,
        },
        NeedsOnlineCheck {
            reason: OnlineCheckReason::NoConnection,
        },
        Revoked,
        ClockChanged,
    ] {
        let open = !matches!(s, TrialEnded | Revoked | ClockChanged);
        assert_eq!(s.permits(Permission::Open), open, "{s:?}");
        assert_eq!(s.permits(Permission::Export), open, "{s:?}");
        let edit = matches!(s, Lifetime | TrialActive { .. });
        assert_eq!(s.permits(Permission::Edit), edit, "{s:?}");
        let shared = s.to_entitlement_state();
        let mac = crate::Channel::MacAppStore;
        assert_eq!(shared.permits(mac, Permission::Edit), edit, "{s:?}");
        assert_eq!(shared.permits(mac, Permission::Open), open, "{s:?}");
    }
}

// ------------------------------------------------------------------ the adapter

#[derive(Default)]
struct Apple {
    entitlements: Vec<VerifiedTransaction>,
    entitlements_fail: bool,
    products_fail: bool,
    purchases: Vec<Result<PurchaseOutcome, BridgeError>>,
    purchase_calls: Vec<String>,
    sync_calls: u32,
    sync_signed: Option<i64>,
    sync_fail: bool,
    after_sync: Option<Vec<VerifiedTransaction>>,
    keychain: Option<String>,
    keychain_fail: bool,
    keychain_writes: u32,
}

#[derive(Clone)]
struct FakeBridge {
    apple: Arc<Mutex<Apple>>,
    env: StoreEnvironment,
}

impl StoreKitBridge for FakeBridge {
    fn current_entitlements(&self) -> Result<Vec<VerifiedTransaction>, BridgeError> {
        let a = self.apple.lock().unwrap();
        if a.entitlements_fail {
            return Err(BridgeError::StoreUnavailable);
        }
        Ok(a.entitlements.clone())
    }
    fn purchase(&self, product_id: &str) -> Result<PurchaseOutcome, BridgeError> {
        let mut a = self.apple.lock().unwrap();
        a.purchase_calls.push(product_id.to_string());
        let next = if a.purchases.is_empty() {
            Ok(PurchaseOutcome::Cancelled)
        } else {
            a.purchases.remove(0)
        };
        if let Ok(PurchaseOutcome::Success(t)) = &next {
            let t = t.clone();
            a.entitlements.push(t);
        }
        next
    }
    fn app_transaction(&self) -> Result<AppTransactionInfo, BridgeError> {
        Ok(AppTransactionInfo {
            app_transaction_id: None,
            environment: self.env,
        })
    }
    fn environment(&self) -> StoreEnvironment {
        self.env
    }
    fn load_products(&self) -> Result<StoreProducts, BridgeError> {
        if self.apple.lock().unwrap().products_fail {
            return Err(BridgeError::NoConnection);
        }
        Ok(StoreProducts {
            lifetime_display_price: "{test price}".to_string(),
        })
    }
    fn sync(&self) -> Result<Option<i64>, BridgeError> {
        let mut a = self.apple.lock().unwrap();
        a.sync_calls += 1;
        if a.sync_fail {
            return Err(BridgeError::NoConnection);
        }
        if let Some(list) = a.after_sync.take() {
            a.entitlements = list;
        }
        Ok(a.sync_signed)
    }
    fn load_guard(&self) -> Result<Option<String>, BridgeError> {
        let a = self.apple.lock().unwrap();
        if a.keychain_fail {
            return Err(BridgeError::Other);
        }
        Ok(a.keychain.clone())
    }
    fn save_guard(&self, record_json: &str) -> Result<(), BridgeError> {
        let mut a = self.apple.lock().unwrap();
        if a.keychain_fail {
            return Err(BridgeError::Other);
        }
        a.keychain = Some(record_json.to_string());
        a.keychain_writes += 1;
        Ok(())
    }
}

#[derive(Clone)]
struct Clocks(Arc<Mutex<(i64, Duration)>>);

impl Clocks {
    fn new(wall: i64) -> Self {
        Self(Arc::new(Mutex::new((wall, Duration::ZERO))))
    }
    /// Time really passes: both clocks move.
    fn pass(&self, ms: i64) {
        let mut c = self.0.lock().unwrap();
        c.0 += ms;
        c.1 += Duration::from_millis(ms.unsigned_abs());
    }
    /// The Mac slept: only the wall clock moves.
    fn sleep(&self, ms: i64) {
        self.0.lock().unwrap().0 += ms;
    }
    /// The person changed the clock: only the wall clock moves.
    fn set_wall(&self, wall: i64) {
        self.0.lock().unwrap().0 = wall;
    }
}

impl WallClock for Clocks {
    fn now_ms(&self) -> i64 {
        self.0.lock().unwrap().0
    }
}

impl MonotonicClock for Clocks {
    fn now(&self) -> Duration {
        self.0.lock().unwrap().1
    }
}

struct Rig {
    apple: Arc<Mutex<Apple>>,
    clocks: Clocks,
    adapter: MacAppStoreAdapter<FakeBridge>,
}

fn adapter_over(
    apple: &Arc<Mutex<Apple>>,
    clocks: &Clocks,
    env: StoreEnvironment,
) -> MacAppStoreAdapter<FakeBridge> {
    MacAppStoreAdapter::new(
        FakeBridge {
            apple: apple.clone(),
            env,
        },
        Box::new(clocks.clone()),
        Box::new(clocks.clone()),
    )
}

fn rig(now: i64) -> Rig {
    let apple = Arc::new(Mutex::new(Apple::default()));
    let clocks = Clocks::new(now);
    let adapter = adapter_over(&apple, &clocks, PROD);
    Rig {
        apple,
        clocks,
        adapter,
    }
}

impl Rig {
    /// The next launch of the app on this Mac, with the same Apple data and Keychain.
    fn relaunch(&self) -> MacAppStoreAdapter<FakeBridge> {
        adapter_over(&self.apple, &self.clocks, PROD)
    }
    fn start_trial(&self, at: i64) {
        self.apple
            .lock()
            .unwrap()
            .purchases
            .push(Ok(PurchaseOutcome::Success(trial(at))));
        self.adapter.purchase_trial().unwrap();
    }
}

#[test]
fn first_launch_online_offers_the_trial_and_starting_it_counts_seven_days() {
    let r = rig(T0);
    assert_eq!(
        r.adapter.refresh(),
        MacState::TrialNotStarted {
            purchase_pending: false
        }
    );
    assert_eq!(r.adapter.lifetime_price().as_deref(), Some("{test price}"));
    r.start_trial(T0);
    assert_eq!(days_left(r.adapter.mac_state()), 7);
    assert_eq!(
        ChannelAdapter::trial_ends_at_ms(&r.adapter),
        Some(T0 + TRIAL_MS)
    );
    // The guard record was saved with the start.
    let stored = r.apple.lock().unwrap().keychain.clone().unwrap();
    let parsed = GuardRecord::parse(Some(&stored));
    assert_eq!(parsed.record.trial_start_ms, Some(T0));
}

#[test]
fn the_trial_works_with_the_network_cable_pulled_after_the_first_online_start() {
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);

    // Offline: no products, and `currentEntitlements` empties too. The trial goes on.
    {
        let mut a = r.apple.lock().unwrap();
        a.products_fail = true;
        a.entitlements.clear();
    }
    r.clocks.sleep(3 * DAY_MS);
    let next = r.relaunch();
    assert_eq!(days_left(next.refresh()), 4);
    r.clocks.sleep(5 * DAY_MS);
    assert_eq!(
        next.revalidate(RevalidateReason::Timer),
        MacState::TrialEnded
    );
}

#[test]
fn first_launch_offline_needs_an_online_check_and_never_offers_the_trial() {
    let r = rig(T0);
    r.apple.lock().unwrap().products_fail = true;
    assert_eq!(
        r.adapter.refresh(),
        MacState::NeedsOnlineCheck {
            reason: OnlineCheckReason::NoConnection
        }
    );
    assert_eq!(r.adapter.lifetime_price(), None);
    // Back online.
    r.apple.lock().unwrap().products_fail = false;
    assert_eq!(
        r.adapter.revalidate(RevalidateReason::Resume),
        MacState::TrialNotStarted {
            purchase_pending: false
        }
    );
}

#[test]
fn a_buyer_relaunching_offline_stays_unlocked_and_a_refund_is_noticed_online() {
    let r = rig(T0);
    r.adapter.refresh();
    r.apple
        .lock()
        .unwrap()
        .purchases
        .push(Ok(PurchaseOutcome::Success(lifetime(T0))));
    assert_eq!(
        r.adapter.purchase_lifetime().unwrap(),
        PurchaseEnd::Finished(MacState::Lifetime)
    );
    {
        let mut a = r.apple.lock().unwrap();
        a.products_fail = true;
        a.entitlements.clear();
    }
    let next = r.relaunch();
    assert_eq!(next.refresh(), MacState::Lifetime);
    assert_eq!(
        next.channel_state(),
        Some(EntitlementState::Purchased),
        "no restart needed"
    );
    // The next time it is online, Apple reports the refund.
    {
        let mut a = r.apple.lock().unwrap();
        a.products_fail = false;
        a.entitlements = vec![revoked(lifetime(T0))];
    }
    assert_eq!(
        next.revalidate(RevalidateReason::StoreKitChanged),
        MacState::Revoked
    );
    // A later offline launch does not read as bought.
    {
        let mut a = r.apple.lock().unwrap();
        a.products_fail = true;
        a.entitlements.clear();
    }
    assert_ne!(r.relaunch().refresh(), MacState::Lifetime);
}

#[test]
fn a_clock_turned_back_after_a_relaunch_locks_and_clears_as_soon_as_it_is_fixed() {
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);
    r.clocks.sleep(6 * DAY_MS);
    assert_eq!(days_left(r.adapter.refresh()), 1);

    // The person moves the clock back a week and opens the app again.
    let real = r.clocks.now_ms();
    r.clocks.set_wall(real - 7 * DAY_MS);
    let next = r.relaunch();
    assert_eq!(next.refresh(), MacState::ClockChanged);
    assert_eq!(next.channel_state(), Some(EntitlementState::ClockChanged));
    // Nothing raised the floor while the clock was wrong, and the 5 second check asks nothing.
    let writes = r.apple.lock().unwrap().keychain_writes;
    assert_eq!(
        next.revalidate(RevalidateReason::ClockTick),
        MacState::ClockChanged
    );
    assert_eq!(r.apple.lock().unwrap().keychain_writes, writes);

    // Clock fixed: the next tick clears it with no click.
    r.clocks.set_wall(real);
    assert_eq!(days_left(next.revalidate(RevalidateReason::ClockTick)), 1);
}

#[test]
fn a_clock_turned_back_while_the_app_runs_is_noticed_by_the_monotonic_clock() {
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);
    r.clocks.pass(MINUTE_MS);
    assert_eq!(days_left(r.adapter.mac_state()), 7);
    // Back by more than the tolerance, but the high water mark was last saved a minute ago.
    let wall = r.clocks.now_ms();
    r.clocks.set_wall(wall - 10 * MINUTE_MS);
    assert_eq!(r.adapter.mac_state(), MacState::ClockChanged);
    r.clocks.set_wall(wall);
    assert_eq!(days_left(r.adapter.mac_state()), 7);
}

#[test]
fn a_sleeping_mac_never_reads_as_a_clock_change_and_a_change_after_the_wake_does() {
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);
    r.clocks.sleep(8 * HOUR_MS);
    assert_eq!(days_left(r.adapter.mac_state()), 7);
    let wall = r.clocks.now_ms();
    r.clocks.set_wall(wall - HOUR_MS);
    assert_eq!(r.adapter.mac_state(), MacState::ClockChanged);
}

#[test]
fn check_again_with_a_wrong_future_clock_since_fixed_recovers_from_a_fresh_apple_date() {
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);
    // The clock was next year for a while.
    r.clocks.sleep(365 * DAY_MS);
    r.adapter.revalidate(RevalidateReason::Resume);
    let wall = r.clocks.now_ms();
    r.clocks.set_wall(wall - 365 * DAY_MS + DAY_MS);
    let fixed = r.clocks.now_ms();
    let next = r.relaunch();
    assert_eq!(next.refresh(), MacState::ClockChanged);

    // An Apple date far from the clock changes nothing.
    r.apple.lock().unwrap().sync_signed = Some(fixed - DAY_MS);
    assert_eq!(next.restore().unwrap(), MacState::ClockChanged);
    // A fresh one does.
    r.apple.lock().unwrap().sync_signed = Some(fixed - MINUTE_MS);
    assert_eq!(days_left(next.restore().unwrap()), 6);
    assert_eq!(next.guard_record().high_water_ms, fixed);
    // The relaunch keeps it.
    assert_eq!(days_left(r.relaunch().refresh()), 6);
}

#[test]
fn check_again_with_the_clock_set_back_to_an_old_apple_date_gives_no_days_back() {
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);
    r.clocks.sleep(6 * DAY_MS);
    r.adapter.refresh();
    // The clock goes back to the moment Apple signed the trial, and the store is online.
    r.clocks.set_wall(T0 + MINUTE_MS);
    let next = r.relaunch();
    assert_eq!(next.refresh(), MacState::ClockChanged);
    assert_eq!(next.restore().unwrap(), MacState::ClockChanged);
    // The same when the store hands the old date back as if it were new.
    r.apple.lock().unwrap().sync_signed = Some(T0);
    assert_eq!(next.restore().unwrap(), MacState::ClockChanged);
}

#[test]
fn check_again_while_offline_cannot_clear_the_clock_lock() {
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);
    r.clocks.sleep(3 * DAY_MS);
    r.adapter.refresh();
    let wall = r.clocks.now_ms();
    r.clocks.set_wall(wall - 2 * DAY_MS);
    let next = r.relaunch();
    {
        let mut a = r.apple.lock().unwrap();
        a.products_fail = true;
        a.sync_fail = true;
    }
    assert_eq!(next.refresh(), MacState::ClockChanged);
    assert_eq!(next.restore(), Err(BridgeError::NoConnection));
    assert_eq!(next.mac_state(), MacState::ClockChanged);
}

#[test]
fn a_stored_look_answers_from_the_keychain_without_asking_the_store() {
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);
    r.clocks.sleep(DAY_MS);
    r.adapter.refresh();
    // The next launch, with the store slow: the record alone gives the trial.
    let next = r.relaunch();
    r.apple.lock().unwrap().entitlements_fail = true;
    assert_eq!(days_left(next.revalidate(RevalidateReason::Stored)), 6);
}

#[test]
fn the_clock_tick_writes_the_keychain_at_most_every_five_minutes() {
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);
    let writes = r.apple.lock().unwrap().keychain_writes;
    for _ in 0..10 {
        r.clocks.pass(5_000);
        r.adapter.revalidate(RevalidateReason::ClockTick);
    }
    assert_eq!(r.apple.lock().unwrap().keychain_writes, writes);
    r.clocks.pass(5 * MINUTE_MS);
    r.adapter.revalidate(RevalidateReason::ClockTick);
    assert_eq!(r.apple.lock().unwrap().keychain_writes, writes + 1);
}

#[test]
fn an_environment_that_could_not_be_read_is_asked_again_and_wipes_nothing() {
    #[derive(Clone)]
    struct Late {
        inner: FakeBridge,
        known: Arc<Mutex<Option<StoreEnvironment>>>,
    }
    impl StoreKitBridge for Late {
        fn current_entitlements(&self) -> Result<Vec<VerifiedTransaction>, BridgeError> {
            self.inner.current_entitlements()
        }
        fn purchase(&self, p: &str) -> Result<PurchaseOutcome, BridgeError> {
            self.inner.purchase(p)
        }
        fn app_transaction(&self) -> Result<AppTransactionInfo, BridgeError> {
            self.inner.app_transaction()
        }
        fn environment(&self) -> StoreEnvironment {
            StoreEnvironment::Production
        }
        fn known_environment(&self) -> Option<StoreEnvironment> {
            *self.known.lock().unwrap()
        }
        fn load_products(&self) -> Result<StoreProducts, BridgeError> {
            self.inner.load_products()
        }
        fn load_guard(&self) -> Result<Option<String>, BridgeError> {
            self.inner.load_guard()
        }
        fn save_guard(&self, record_json: &str) -> Result<(), BridgeError> {
            self.inner.save_guard(record_json)
        }
    }
    // Apple's reviewer: a sandbox purchase, and the environment read failed at launch.
    let mut t = lifetime(T0);
    t.environment = SANDBOX;
    let apple = Arc::new(Mutex::new(Apple {
        entitlements: vec![t],
        keychain: Some(r#"{"v":1,"env":"sandbox","lifetime_seen":true}"#.to_string()),
        ..Apple::default()
    }));
    let clocks = Clocks::new(T0);
    let known = Arc::new(Mutex::new(None));
    let adapter = MacAppStoreAdapter::new(
        Late {
            inner: FakeBridge {
                apple: apple.clone(),
                env: SANDBOX,
            },
            known: known.clone(),
        },
        Box::new(clocks.clone()),
        Box::new(clocks.clone()),
    );
    adapter.refresh();
    assert!(
        apple
            .lock()
            .unwrap()
            .keychain
            .as_deref()
            .unwrap()
            .contains("lifetime_seen\":true"),
        "the record survives while the environment is only a guess"
    );
    *known.lock().unwrap() = Some(SANDBOX);
    assert_eq!(
        adapter.revalidate(RevalidateReason::Resume),
        MacState::Lifetime
    );
}

#[test]
fn only_restore_runs_app_store_sync() {
    let r = rig(T0);
    r.adapter.refresh();
    for reason in [
        RevalidateReason::Launch,
        RevalidateReason::Resume,
        RevalidateReason::Wake,
        RevalidateReason::StoreKitChanged,
        RevalidateReason::Timer,
        RevalidateReason::ClockTick,
        RevalidateReason::Stored,
    ] {
        r.adapter.revalidate(reason);
    }
    assert_eq!(r.apple.lock().unwrap().sync_calls, 0);
    r.adapter.restore().unwrap();
    assert_eq!(r.apple.lock().unwrap().sync_calls, 1);
}

#[test]
fn restore_on_a_new_mac_finds_the_purchase_with_an_empty_guard() {
    let r = rig(T0 + 3 * DAY_MS);
    r.adapter.refresh();
    r.apple.lock().unwrap().after_sync = Some(vec![lifetime(T0)]);
    assert_eq!(r.adapter.restore().unwrap(), MacState::Lifetime);
    assert!(r.adapter.guard_record().lifetime_seen);
}

#[test]
fn a_second_trial_purchase_is_never_attempted_and_never_restarts_the_trial() {
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);
    r.clocks.sleep(2 * DAY_MS);
    r.apple
        .lock()
        .unwrap()
        .purchases
        .push(Ok(PurchaseOutcome::Success(tx(
            TRIAL_PRODUCT_ID,
            "2000000003",
            T0 + 2 * DAY_MS,
        ))));
    let end = r.adapter.purchase_trial().unwrap();
    assert_eq!(days_left(end.state()), 5);
    assert_eq!(
        r.apple.lock().unwrap().purchase_calls.len(),
        1,
        "no second sheet"
    );
}

#[test]
fn a_second_apple_account_on_the_same_mac_keeps_the_start_on_this_mac() {
    // ADR-0040: a second account gets its own trial on another Mac or macOS user. On one Keychain
    // the start never moves later (rule 7).
    let r = rig(T0);
    r.adapter.refresh();
    r.start_trial(T0);
    r.clocks.sleep(8 * DAY_MS);
    r.apple.lock().unwrap().entitlements =
        vec![tx(TRIAL_PRODUCT_ID, "3000000001", T0 + 8 * DAY_MS)];
    assert_eq!(
        r.adapter.revalidate(RevalidateReason::StoreKitChanged),
        MacState::TrialEnded
    );
}

#[test]
fn a_deleted_keychain_item_with_a_new_account_gets_more_time_which_is_accepted() {
    let r = rig(T0);
    r.apple.lock().unwrap().entitlements = vec![trial(T0)];
    r.apple.lock().unwrap().keychain = None;
    assert_eq!(days_left(r.adapter.refresh()), 7);
}

#[test]
fn a_purchase_that_waits_grants_nothing_and_says_so_and_a_cancel_changes_nothing() {
    let r = rig(T0);
    r.adapter.refresh();
    r.apple
        .lock()
        .unwrap()
        .purchases
        .push(Ok(PurchaseOutcome::Pending));
    let answer = ChannelAdapter::start_trial(&r.adapter).unwrap();
    assert_eq!(
        answer.unwrap_err().message,
        super::adapter::PURCHASE_PENDING
    );
    assert_eq!(
        r.adapter.mac_state(),
        MacState::TrialNotStarted {
            purchase_pending: true
        }
    );
    r.apple
        .lock()
        .unwrap()
        .purchases
        .push(Ok(PurchaseOutcome::Cancelled));
    let answer = ChannelAdapter::buy_full_version(&r.adapter).unwrap();
    assert_eq!(answer, Ok(EntitlementState::TrialNotStarted));
}

#[test]
fn a_purchase_for_another_product_environment_or_shape_is_refused() {
    let r = rig(T0);
    r.adapter.refresh();
    r.apple
        .lock()
        .unwrap()
        .purchases
        .push(Ok(PurchaseOutcome::Success(lifetime(T0))));
    assert_eq!(r.adapter.purchase_trial(), Err(BridgeError::Unverified));
    let mut sandbox = lifetime(T0);
    sandbox.environment = SANDBOX;
    r.apple
        .lock()
        .unwrap()
        .purchases
        .push(Ok(PurchaseOutcome::Success(sandbox)));
    assert_eq!(r.adapter.purchase_lifetime(), Err(BridgeError::Unverified));
    r.apple
        .lock()
        .unwrap()
        .purchases
        .push(Err(BridgeError::PurchasesNotAllowed));
    let answer = ChannelAdapter::buy_full_version(&r.adapter).unwrap();
    assert!(answer.unwrap_err().message.contains("Screen Time"));
}

#[test]
fn a_sandbox_build_accepts_sandbox_purchases_and_a_production_build_ignores_them() {
    let mut t = lifetime(T0);
    t.environment = SANDBOX;
    let apple = Arc::new(Mutex::new(Apple {
        entitlements: vec![t],
        ..Apple::default()
    }));
    let clocks = Clocks::new(T0);
    assert_eq!(
        adapter_over(&apple, &clocks, SANDBOX).refresh(),
        MacState::Lifetime
    );
    // Same Keychain: what the sandbox build remembered does not count in the production build.
    assert!(apple.lock().unwrap().keychain.is_some());
    let prod = adapter_over(&apple, &clocks, PROD);
    assert_ne!(prod.refresh(), MacState::Lifetime);
    assert!(!prod.guard_record().lifetime_seen);
    assert_eq!(prod.last_anomalies().environment_mismatch, 1);
}

#[test]
fn a_corrupted_guard_record_is_counted_and_never_grants_or_crashes() {
    for bad in ["not json", r#"{"high_water_ms": 1"#] {
        let r = rig(T0);
        r.apple.lock().unwrap().keychain = Some(bad.to_string());
        assert_eq!(
            r.adapter.refresh(),
            MacState::TrialNotStarted {
                purchase_pending: false
            }
        );
        assert_eq!(r.adapter.guard_anomalies(), 1);
        // The record is replaced by a good one.
        let stored = r.apple.lock().unwrap().keychain.clone().unwrap();
        assert!(!GuardRecord::parse(Some(&stored)).corrupt);
    }
}

#[test]
fn a_future_version_of_the_guard_record_is_read_and_keeps_its_version() {
    let r = rig(T0 + DAY_MS);
    r.apple.lock().unwrap().keychain = Some(format!(
        r#"{{"v": 9, "trial_start_ms": {T0}, "high_water_ms": {T0}, "newer": [1]}}"#
    ));
    assert_eq!(days_left(r.adapter.refresh()), 6);
    let stored = r.apple.lock().unwrap().keychain.clone().unwrap();
    assert_eq!(GuardRecord::parse(Some(&stored)).record.v, 9);
}

#[test]
fn a_keychain_that_does_not_answer_keeps_working_in_memory_and_saves_nothing_over_it() {
    let r = rig(T0);
    r.apple.lock().unwrap().keychain_fail = true;
    r.adapter.refresh();
    r.start_trial(T0);
    assert_eq!(days_left(r.adapter.mac_state()), 7);
    assert_eq!(r.apple.lock().unwrap().keychain_writes, 0);
    // It answers later: the two are joined and saved, earliest start and highest floor.
    {
        let mut a = r.apple.lock().unwrap();
        a.keychain_fail = false;
        a.keychain = Some(
            serde_json::to_string(&GuardRecord {
                trial_start_ms: Some(T0 - DAY_MS),
                high_water_ms: T0 - HOUR_MS,
                ..GuardRecord::default()
            })
            .unwrap(),
        );
    }
    assert_eq!(days_left(r.adapter.revalidate(RevalidateReason::Resume)), 6);
    let stored = r.apple.lock().unwrap().keychain.clone().unwrap();
    assert_eq!(
        GuardRecord::parse(Some(&stored)).record.trial_start_ms,
        Some(T0 - DAY_MS)
    );
}

#[test]
fn an_unchanged_guard_record_is_not_written_again() {
    let r = rig(T0);
    r.adapter.refresh();
    let writes = r.apple.lock().unwrap().keychain_writes;
    r.adapter.revalidate(RevalidateReason::Timer);
    assert_eq!(
        r.apple.lock().unwrap().keychain_writes,
        writes,
        "the clock did not move"
    );
    r.clocks.pass(MINUTE_MS);
    r.adapter.revalidate(RevalidateReason::Timer);
    assert_eq!(r.apple.lock().unwrap().keychain_writes, writes + 1);
}

#[test]
fn unverified_results_are_dropped_and_counted_by_the_contract_filter() {
    let set = keep_verified([
        RawVerification::Verified(lifetime(T0)),
        RawVerification::Unverified {
            product_id: LIFETIME_PRODUCT_ID.to_string(),
        },
        RawVerification::Verified({
            let mut t = trial(T0);
            t.purchase_date_ms = 0;
            t
        }),
    ]);
    assert_eq!(set.transactions.len(), 1);
    assert_eq!(set.unverified, 1);
    assert_eq!(set.malformed, 1);
}

#[test]
fn the_channel_adapter_reports_the_purchase_answer() {
    let r = rig(T0);
    assert_eq!(r.adapter.purchase_status(0), PurchaseStatus::NotPurchased);
    r.apple.lock().unwrap().entitlements = vec![lifetime(T0)];
    r.adapter.revalidate(RevalidateReason::Resume);
    assert_eq!(r.adapter.purchase_status(0), PurchaseStatus::Purchased);
    assert_eq!(r.adapter.limitation(), None);
}

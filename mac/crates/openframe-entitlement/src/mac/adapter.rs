//! The Mac App Store channel adapter: StoreKit 2 decided by [`MacTrialPolicy`], fully offline after
//! the first online start, with no licence server and no device binding (ADR-0040).
//!
//! What is authoritative and what is not:
//!
//! - **Authoritative:** transactions StoreKit verified (through [`StoreKitBridge`]).
//! - **Session memory only:** the latest verified `currentEntitlements` list and whether the App
//!   Store answered in this evaluation (`Product.products(for:)` returned both products).
//!   `currentEntitlements` does not fail offline, so that second signal is what tells an empty list
//!   from "no trial yet".
//! - **The guard record:** kept in the Keychain by the Swift shell (see
//!   [`super::clock_guard`]). It raises floors and remembers a start and a purchase on this Mac.
//!
//! The Swift shell asks for a check at launch, when the app comes to the front, after a wake, on
//! `Transaction.updates` and `Storefront.updates`. The core's worker adds one every 5 minutes while
//! a trial runs and one at the trial's end. `AppStore.sync()` runs only for "Restore purchase" and
//! "Check again", because it can show Apple's sign-in window.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::bridge::{
    BridgeError, LIFETIME_PRODUCT_ID, PurchaseOutcome, StoreEnvironment, StoreKitBridge,
    TRIAL_PRODUCT_ID, VerifiedTransaction,
};
use super::clock_guard::GuardRecord;
use super::policy::{
    Anomalies, CLOCK_TOLERANCE_MS, EntitlementsView, MacState, MacTrialPolicy, TrialInputs,
    cap_high_water_at_apple_date, recover_from_future_clock,
};
use super::unavailable::UnavailableBridge;
use crate::adapters::{ChannelAdapter, PurchaseFailure};
use crate::{Channel, EntitlementState, PurchaseStatus};

/// Which way rule G4 recovers from a wrong future clock. True: a freshly signed Apple date within
/// 10 minutes of the clock resets the high water mark. False: the fallback, capping the high water
/// mark at the last Apple signature plus 2 days. ADR-0040 records why the first is in use and that
/// it still has to be confirmed in the sandbox.
pub const G4_USES_FRESH_SIGNATURE: bool = true;

/// How often the worker's clock look saves a higher reading to the Keychain.
const GUARD_SAVE_EVERY_MS: i64 = 5 * 60 * 1000;

/// Plain words for a purchase that waits for approval (Ask to Buy, or a payment step).
pub const PURCHASE_PENDING: &str = "This purchase is waiting for approval. OpenFrame Studio updates by itself once it is approved.";

/// A monotonic clock: time since an arbitrary origin that never follows the wall clock.
pub trait MonotonicClock: Send + Sync {
    fn now(&self) -> Duration;
}

/// The production clock: `std::time::Instant` since the adapter was built.
#[derive(Debug)]
pub struct InstantClock {
    origin: Instant,
}

impl InstantClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for InstantClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for InstantClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// The Mac's wall clock in Unix milliseconds UTC. An input of the policy since ADR-0040.
pub trait WallClock: Send + Sync {
    fn now_ms(&self) -> i64;
}

/// The production wall clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemWallClock;

impl WallClock for SystemWallClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .unwrap_or(0)
    }
}

/// Why the desktop shell or the worker asks for a new check. The reason is logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RevalidateReason {
    /// The app started.
    Launch,
    /// The app came to the front, or the Mac woke from sleep.
    Resume,
    /// StoreKit reported a change: a `Transaction.updates` element (a purchase, a refund, a
    /// revocation) or an App Store account change.
    StoreKitChanged,
    /// The 5 minute timer while a trial runs, or the moment the trial ends.
    Timer,
    /// The Mac woke from sleep. Never left out as a repeat: the monotonic clock stood still.
    Wake,
    /// The worker's 5 second look at the clock. Asks StoreKit nothing.
    ClockTick,
    /// The first look at launch: what the Keychain record says, before StoreKit has answered, so a
    /// buyer or a running trial is not held back while the store is slow.
    Stored,
}

#[derive(Default)]
struct Session {
    /// The latest verified `currentEntitlements` list in this launch.
    entitlements: Option<Vec<VerifiedTransaction>>,
    /// `Product.products(for:)` returned both products in the latest evaluation.
    store_reachable: bool,
    /// `Product.displayPrice` of the lifetime product, from the latest answer.
    lifetime_price: Option<String>,
    purchase_pending: bool,
    /// The guard record. Merged in from the Keychain once it has been read.
    record: GuardRecord,
    /// The Keychain answered in this launch, so saving cannot overwrite a record not yet seen.
    record_loaded: bool,
    /// What the Keychain holds as far as this launch knows, to skip writes that change nothing.
    saved: Option<GuardRecord>,
    /// A monotonic and wall reading taken together, for rule G3.
    wall_anchor: Option<(Duration, i64)>,
    /// The largest monotonic reading seen, so a reading that goes back is ignored.
    max_now: Duration,
    refreshed: bool,
    anomalies: Anomalies,
    /// Stored guard records that could not be read.
    guard_anomalies: u32,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the desktop shell hands over to connect the Mac App Store channel.
pub struct MacInputs {
    /// The Swift StoreKit 2 bridge, which also keeps the guard record in the Keychain.
    pub bridge: Box<dyn StoreKitBridge>,
    /// The monotonic session clock, normally [`InstantClock`].
    pub clock: Box<dyn MonotonicClock>,
    /// The Mac's wall clock, normally [`SystemWallClock`].
    pub wall: Box<dyn WallClock>,
}

/// The adapter as the app holds it, over whatever bridge the shell supplied.
pub type DynMacAppStoreAdapter = MacAppStoreAdapter<Box<dyn StoreKitBridge>>;

/// The Mac App Store adapter.
pub struct MacAppStoreAdapter<B: StoreKitBridge> {
    bridge: B,
    clock: Box<dyn MonotonicClock>,
    wall: Box<dyn WallClock>,
    /// False when the build has no App Store connection ([`Self::unavailable`]).
    store_connected: bool,
    /// Rebuilt when the environment could only be guessed at first (see `confirm_environment`).
    policy: Mutex<MacTrialPolicy>,
    /// The environment was really read, not guessed.
    environment_known: Mutex<bool>,
    session: Mutex<Session>,
    /// Serialises refreshes and purchases so two never interleave.
    refresh_lock: Mutex<()>,
}

impl MacAppStoreAdapter<UnavailableBridge> {
    /// The adapter for a build without an App Store connection: a Windows build, or a Mac shell
    /// that does not supply [`MacInputs`]. The state is always NeedsOnlineCheck, so opening and
    /// exporting work, editing is locked and nothing is ever granted.
    pub fn unavailable() -> Self {
        let mut adapter = Self::new(
            UnavailableBridge,
            Box::new(InstantClock::new()),
            Box::new(SystemWallClock),
        );
        adapter.store_connected = false;
        adapter
    }
}

impl DynMacAppStoreAdapter {
    /// The adapter over the bridge and clocks the desktop shell supplied.
    pub fn from_inputs(inputs: MacInputs) -> Self {
        Self::new(inputs.bridge, inputs.clock, inputs.wall)
    }
}

impl<B: StoreKitBridge> MacAppStoreAdapter<B> {
    pub fn new(bridge: B, clock: Box<dyn MonotonicClock>, wall: Box<dyn WallClock>) -> Self {
        let known = bridge.known_environment();
        let policy = MacTrialPolicy::new(known.unwrap_or_else(|| bridge.environment()));
        Self {
            bridge,
            clock,
            wall,
            store_connected: true,
            policy: Mutex::new(policy),
            environment_known: Mutex::new(known.is_some()),
            session: Mutex::new(Session::default()),
            refresh_lock: Mutex::new(()),
        }
    }

    pub fn environment(&self) -> StoreEnvironment {
        lock(&self.policy).app_environment()
    }

    /// A read that failed at launch sticks to Production, and Apple's reviewer tests a production
    /// build in the sandbox: ask again until the environment is really known.
    fn confirm_environment(&self) {
        if *lock(&self.environment_known) {
            return;
        }
        if let Some(env) = self.bridge.known_environment() {
            *lock(&self.policy) = MacTrialPolicy::new(env);
            *lock(&self.environment_known) = true;
            // The record could not be read against the right environment until now.
            lock(&self.session).record_loaded = false;
        }
    }

    /// The launch check. The same as `revalidate(RevalidateReason::Launch)`.
    pub fn refresh(&self) -> MacState {
        self.revalidate(RevalidateReason::Launch)
    }

    /// Check again. Asks StoreKit for the products and the entitlements (never `AppStore.sync()`),
    /// derives the state and saves the guard record when it changed. [`RevalidateReason::ClockTick`]
    /// only looks at the clock again.
    pub fn revalidate(&self, reason: RevalidateReason) -> MacState {
        let _serial = lock(&self.refresh_lock);
        tracing::debug!(?reason, "entitlement check");
        match reason {
            RevalidateReason::ClockTick => {}
            RevalidateReason::Stored => self.load_guard(),
            _ => self.ask_store(),
        }
        self.finish(reason == RevalidateReason::ClockTick)
    }

    /// The state now, from this session's verified data and the clocks. Makes no calls, so a trial
    /// that runs out while the app is open ends here.
    pub fn mac_state(&self) -> MacState {
        let mut session = lock(&self.session);
        self.derive(&mut session)
    }

    /// The counts of transactions and answers the last derivation ignored.
    pub fn last_anomalies(&self) -> Anomalies {
        lock(&self.session).anomalies
    }

    /// Stored guard records that could not be read since launch.
    pub fn guard_anomalies(&self) -> u32 {
        lock(&self.session).guard_anomalies
    }

    /// The guard record as it stands now.
    pub fn guard_record(&self) -> GuardRecord {
        lock(&self.session).record.clone()
    }

    /// `Product.displayPrice` of the lifetime product, as the App Store formats it for this
    /// storefront. `None` until both products loaded.
    pub fn lifetime_price(&self) -> Option<String> {
        lock(&self.session).lifetime_price.clone()
    }

    /// Buy the zero-price trial product. A trial that already exists is never bought again.
    pub fn purchase_trial(&self) -> Result<PurchaseEnd, BridgeError> {
        self.purchase(TRIAL_PRODUCT_ID)
    }

    /// Buy the lifetime unlock.
    pub fn purchase_lifetime(&self) -> Result<PurchaseEnd, BridgeError> {
        self.purchase(LIFETIME_PRODUCT_ID)
    }

    /// "Restore purchase" and "Check again": `AppStore.sync()`, then a full check. When the clock
    /// guard is what stands in the way, the freshly signed Apple date may clear it (rule G4). When
    /// the sync fails the check still runs and the error is returned for the message.
    pub fn restore(&self) -> Result<MacState, BridgeError> {
        let _serial = lock(&self.refresh_lock);
        self.load_guard();
        let known_signed = lock(&self.session).record.last_apple_signed_ms;
        let synced = self.bridge.sync();
        self.ask_store();
        if let Ok(fresh) = &synced {
            self.recover_clock(*fresh, known_signed);
        }
        let state = self.finish(false);
        synced.map(|_| state)
    }

    // ------------------------------------------------------------------ internals

    fn now(&self, session: &mut Session) -> Duration {
        let now = self.clock.now().max(session.max_now);
        session.max_now = now;
        now
    }

    /// Rule G3: true when the wall clock moved back against the monotonic clock while the app ran.
    /// The anchor moves forward with the wall clock, because the monotonic clock stands still while
    /// the Mac sleeps.
    fn wall_moved_back(&self, session: &mut Session, wall: i64) -> bool {
        let mono = self.now(session);
        let Some((anchor_mono, anchor_wall)) = session.wall_anchor else {
            session.wall_anchor = Some((mono, wall));
            return false;
        };
        let ran = i64::try_from(mono.saturating_sub(anchor_mono).as_millis()).unwrap_or(i64::MAX);
        let expected = anchor_wall.saturating_add(ran);
        if wall >= expected {
            session.wall_anchor = Some((mono, wall));
            return false;
        }
        wall < expected.saturating_sub(CLOCK_TOLERANCE_MS)
    }

    fn derive(&self, session: &mut Session) -> MacState {
        let wall = self.wall.now_ms();
        let wall_moved_back = self.wall_moved_back(session, wall);
        let entitlements = match &session.entitlements {
            Some(txs) => EntitlementsView::Known(txs),
            None => EntitlementsView::Unavailable,
        };
        let derived = lock(&self.policy).derive_state(TrialInputs {
            entitlements,
            store_reachable: session.store_reachable,
            now_wall_ms: wall,
            wall_moved_back,
            clock_guard: &session.record,
            purchase_pending: session.purchase_pending,
        });
        session.anomalies = derived.anomalies;
        session.record = derived.clock_guard;
        derived.state
    }

    /// Read the Keychain until it has answered once. A failure keeps what this launch knows, and
    /// nothing is saved over a record not yet seen.
    fn load_guard(&self) {
        if lock(&self.session).record_loaded {
            return;
        }
        match self.bridge.load_guard() {
            Ok(stored) => {
                let mut parsed = GuardRecord::parse(stored.as_deref());
                let env = self.environment();
                // Nothing is judged, and nothing saved over it, while the environment is a guess.
                if !*lock(&self.environment_known) {
                    return;
                }
                if parsed.record.env.is_some_and(|written| written != env) {
                    // Written by a build for another environment: test and real never mix.
                    parsed.record = GuardRecord::default();
                }
                parsed.record.env = Some(env);
                let mut session = lock(&self.session);
                if parsed.corrupt {
                    session.guard_anomalies += 1;
                    tracing::warn!("the trial guard record could not be read; it starts empty");
                }
                let merged = merge(&session.record, &parsed.record);
                session.saved = stored.is_some().then_some(parsed.record);
                session.record = merged;
                session.record_loaded = true;
            }
            Err(e) => tracing::warn!(error = ?e, "the trial guard record is not available"),
        }
    }

    /// Ask StoreKit. On success the list replaces the session list; on failure the session list
    /// from earlier in this launch stays, so a lifetime purchase keeps working.
    fn ask_store(&self) {
        self.confirm_environment();
        self.load_guard();
        let products = self.bridge.load_products();
        let entitlements = self.bridge.current_entitlements();
        let mut session = lock(&self.session);
        session.refreshed = true;
        match products {
            Ok(p) => {
                session.store_reachable = true;
                session.lifetime_price = Some(p.lifetime_display_price);
            }
            Err(e) => {
                session.store_reachable = false;
                tracing::debug!(error = ?e, "the App Store products did not load");
            }
        }
        match entitlements {
            Ok(txs) => {
                // The bridge must pass only verified, well-formed transactions. Check again so a
                // faulty bridge can never hand the policy a corrupt one.
                let (kept, dropped): (Vec<_>, Vec<_>) = txs
                    .into_iter()
                    .partition(VerifiedTransaction::is_well_formed);
                if !dropped.is_empty() {
                    tracing::warn!(
                        count = dropped.len(),
                        "StoreKit bridge passed transactions that are not well formed; ignored"
                    );
                }
                session.entitlements = Some(kept);
            }
            Err(e) => tracing::warn!(error = ?e, "StoreKit current entitlements unavailable"),
        }
    }

    /// Rule G4, when the state is ClockChanged and the store answered. Only a signature newer than
    /// any seen before `known_signed` counts: an old record's date proves nothing, or a clock set
    /// back to the day a transaction was signed would pass.
    fn recover_clock(&self, fresh_signed_ms: Option<i64>, known_signed: i64) {
        let mut session = lock(&self.session);
        if !session.store_reachable || self.derive(&mut session) != MacState::ClockChanged {
            return;
        }
        let wall = self.wall.now_ms();
        let read_now = session
            .entitlements
            .as_deref()
            .map_or(0, |txs| lock(&self.policy).relevant(txs).latest_signed_ms);
        let freshest = fresh_signed_ms.unwrap_or(0).max(read_now);
        let changed = if G4_USES_FRESH_SIGNATURE {
            recover_from_future_clock(
                &mut session.record,
                Some(freshest).filter(|t| *t > known_signed),
                wall,
            )
        } else {
            cap_high_water_at_apple_date(&mut session.record)
        };
        if changed {
            let mono = self.now(&mut session);
            session.wall_anchor = Some((mono, wall));
            tracing::info!("the clock guard was reset from a fresh Apple date");
        }
    }

    /// Derive the state and keep the guard record safe. A tick saves a higher clock reading at most
    /// every 5 minutes (rule G3); anything else that changed goes to the Keychain at once.
    fn finish(&self, tick: bool) -> MacState {
        let mut session = lock(&self.session);
        let state = self.derive(&mut session);
        let a = session.anomalies;
        if a != Anomalies::default() {
            tracing::warn!(
                environment_mismatch = a.environment_mismatch,
                malformed = a.malformed,
                extra_trial_purchases = a.extra_trial_purchases,
                "entitlement anomalies ignored"
            );
        }
        let due = match &session.saved {
            Some(saved) if tick => {
                let only_the_clock = GuardRecord {
                    high_water_ms: saved.high_water_ms,
                    ..session.record.clone()
                } == *saved;
                !only_the_clock
                    || session.record.high_water_ms - saved.high_water_ms >= GUARD_SAVE_EVERY_MS
            }
            Some(saved) => *saved != session.record,
            None => true,
        };
        if self.store_connected
            && session.record_loaded
            && due
            && let Some(text) = session.record.to_stored()
        {
            match self.bridge.save_guard(&text) {
                Ok(()) => session.saved = Some(session.record.clone()),
                Err(e) => tracing::warn!(error = ?e, "the trial guard record could not be saved"),
            }
        }
        state
    }

    fn purchase(&self, product_id: &'static str) -> Result<PurchaseEnd, BridgeError> {
        let _serial = lock(&self.refresh_lock);
        self.load_guard();
        {
            let mut session = lock(&self.session);
            let known = session.entitlements.as_deref().unwrap_or_default();
            let policy = *lock(&self.policy);
            let relevant = policy.relevant(known);
            let owned = if product_id == TRIAL_PRODUCT_ID {
                !relevant.trials.is_empty()
                    || relevant.active_lifetime.is_some()
                    || session.record.trial_start_ms.is_some()
            } else {
                relevant.active_lifetime.is_some()
            };
            if owned {
                // Nothing to buy: a second trial purchase would never extend the trial.
                drop(session);
                return Ok(PurchaseEnd::Finished(self.finish(false)));
            }
            if product_id == TRIAL_PRODUCT_ID {
                session.purchase_pending = false;
            }
        }
        match self.bridge.purchase(product_id)? {
            PurchaseOutcome::Success(tx) => {
                let env = self.environment();
                if !(tx.is_well_formed() && tx.product_id == product_id && tx.environment == env) {
                    tracing::warn!("purchase returned a transaction that does not fit; ignored");
                    return Err(BridgeError::Unverified);
                }
                let mut session = lock(&self.session);
                let list = session.entitlements.get_or_insert_with(Vec::new);
                if !list.iter().any(|t| t.transaction_id == tx.transaction_id) {
                    list.push(tx);
                }
                session.purchase_pending = false;
            }
            PurchaseOutcome::Pending => {
                if product_id == TRIAL_PRODUCT_ID {
                    lock(&self.session).purchase_pending = true;
                }
                return Ok(PurchaseEnd::Pending(self.finish(false)));
            }
            PurchaseOutcome::Cancelled => {}
        }
        Ok(PurchaseEnd::Finished(self.finish(false)))
    }
}

/// How a purchase attempt ended, with the state after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseEnd {
    /// It finished or the person cancelled. The state says which.
    Finished(MacState),
    /// It waits for approval. Nothing is granted.
    Pending(MacState),
}

impl PurchaseEnd {
    pub const fn state(self) -> MacState {
        match self {
            PurchaseEnd::Finished(s) | PurchaseEnd::Pending(s) => s,
        }
    }
}

/// What the session learned before the Keychain answered, joined with what the Keychain holds.
/// Floors rise, the start stays the earliest, and a purchase seen in either place counts.
fn merge(session: &GuardRecord, stored: &GuardRecord) -> GuardRecord {
    let start = match (session.trial_start_ms, stored.trial_start_ms) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    GuardRecord {
        v: session.v.max(stored.v),
        env: stored.env,
        trial_start_ms: start,
        trial_original_tx: if start == stored.trial_start_ms {
            stored.trial_original_tx.clone()
        } else {
            session.trial_original_tx.clone()
        },
        lifetime_seen: session.lifetime_seen || stored.lifetime_seen,
        high_water_ms: session.high_water_ms.max(stored.high_water_ms),
        last_apple_signed_ms: session
            .last_apple_signed_ms
            .max(stored.last_apple_signed_ms),
    }
}

impl<B: StoreKitBridge> ChannelAdapter for MacAppStoreAdapter<B> {
    fn channel(&self) -> Channel {
        Channel::MacAppStore
    }

    /// The purchase answer only. The Mac trial is decided by [`MacAppStoreAdapter::mac_state`]
    /// (see [`ChannelAdapter::channel_state`]).
    fn purchase_status(&self, _ignored: i64) -> PurchaseStatus {
        to_purchase_status(self.launch_state())
    }

    fn restore_purchase(&self, _ignored: i64) -> PurchaseStatus {
        to_purchase_status(self.restore_state())
    }

    fn limitation(&self) -> Option<&'static str> {
        (!self.store_connected)
            .then_some("The App Store connection is not available in this build.")
    }

    /// The whole state.
    fn channel_state(&self) -> Option<EntitlementState> {
        Some(self.launch_state().to_entitlement_state())
    }

    fn restore_channel_state(&self) -> Option<EntitlementState> {
        Some(self.restore_state().to_entitlement_state())
    }

    /// StoreKit's products and `currentEntitlements`, without `AppStore.sync()`, which can show
    /// Apple's sign-in window and is for "Restore purchase" and "Check again" only.
    fn revalidate_channel_state(&self, reason: RevalidateReason) -> Option<EntitlementState> {
        Some(self.revalidate(reason).to_entitlement_state())
    }

    /// Apple's own payment sheet for the lifetime unlock (`Product.purchase()`).
    fn buy_full_version(&self) -> Option<Result<EntitlementState, PurchaseFailure>> {
        Some(purchase_answer(self.purchase_lifetime()))
    }

    /// The zero-price trial product. A trial that already exists is never bought again.
    fn start_trial(&self) -> Option<Result<EntitlementState, PurchaseFailure>> {
        Some(purchase_answer(self.purchase_trial()))
    }

    fn trial_ends_at_ms(&self) -> Option<i64> {
        self.mac_state().trial_ends_at_ms()
    }

    fn trial_time_left(&self) -> Option<Duration> {
        match self.mac_state() {
            MacState::TrialActive {
                remaining_seconds, ..
            } => u64::try_from(remaining_seconds)
                .ok()
                .map(Duration::from_secs),
            _ => None,
        }
    }

    fn lifetime_price(&self) -> Option<String> {
        MacAppStoreAdapter::lifetime_price(self)
    }
}

fn purchase_answer(
    answer: Result<PurchaseEnd, BridgeError>,
) -> Result<EntitlementState, PurchaseFailure> {
    match answer {
        Ok(PurchaseEnd::Finished(state)) => Ok(state.to_entitlement_state()),
        Ok(PurchaseEnd::Pending(_)) => Err(PurchaseFailure {
            message: PURCHASE_PENDING,
        }),
        Err(e) => Err(purchase_failure(e)),
    }
}

/// Plain words for a StoreKit failure during a purchase or a trial start.
pub fn purchase_failure(e: BridgeError) -> PurchaseFailure {
    let message = match e {
        BridgeError::NotSignedIn => "Sign in to the App Store, then try again.",
        BridgeError::NoConnection => {
            "OpenFrame could not reach the App Store. Connect to the internet, then try again."
        }
        BridgeError::StoreUnavailable => {
            "The App Store is not available right now. Try again later."
        }
        BridgeError::ProductNotFound => {
            "The App Store could not find this purchase right now. Try again later."
        }
        BridgeError::PurchasesNotAllowed => {
            "Purchases are turned off on this Mac. Check Screen Time, then try again."
        }
        BridgeError::Unverified => {
            "The App Store could not confirm this purchase. Choose Restore purchase to check again."
        }
        BridgeError::Other => "The purchase did not finish. Try again later.",
    };
    PurchaseFailure { message }
}

impl<B: StoreKitBridge> MacAppStoreAdapter<B> {
    /// The launch check on the first call, then the state from this session's data.
    fn launch_state(&self) -> MacState {
        let refreshed = lock(&self.session).refreshed;
        if refreshed {
            self.mac_state()
        } else {
            self.refresh()
        }
    }

    /// "Restore purchase", or the state now when the App Store sync failed.
    fn restore_state(&self) -> MacState {
        self.restore().unwrap_or_else(|_| self.mac_state())
    }
}

fn to_purchase_status(state: MacState) -> PurchaseStatus {
    match state {
        MacState::Lifetime => PurchaseStatus::Purchased,
        MacState::Revoked => PurchaseStatus::Revoked,
        MacState::NeedsOnlineCheck { .. } | MacState::ClockChanged => PurchaseStatus::Unknown,
        MacState::TrialActive { .. } | MacState::TrialEnded | MacState::TrialNotStarted { .. } => {
            PurchaseStatus::NotPurchased
        }
    }
}

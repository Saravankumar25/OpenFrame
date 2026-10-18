//! The Mac App Store channel adapter: StoreKit 2 plus the licensing server, decided by
//! [`MacTrialPolicy`].
//!
//! What is authoritative and what is not:
//!
//! - **Authoritative:** transactions StoreKit verified (through [`StoreKitBridge`]) and responses
//!   from the licensing server whose signature, nonce and environment were checked.
//! - **Session memory only:** the latest verified `currentEntitlements` list and the latest
//!   confirmed server verdict, with the monotonic time it arrived. Both live in memory for this
//!   launch and are never written to disk.
//! - **Cache only, never grants:** `<app data>/entitlement/mac-cache.json`. It records the last
//!   state so the app can pick the right words when it cannot connect. It is never read to decide
//!   a state, so restoring an old copy, copying one from another Mac or editing it changes nothing.
//!
//! The trial counts down on a monotonic session clock ([`MonotonicClock`], backed by
//! `std::time::Instant` in the app). A reading that goes backwards is ignored, so the countdown
//! never goes back up.
//!
//! Lease and revalidation. Each confirmed verdict is good for `min(leaseSeconds, remainingSeconds)`
//! of monotonic time. The desktop shell calls [`MacAppStoreAdapter::revalidate`] at launch, after
//! the Mac wakes or the window comes back after a long idle, when StoreKit reports a change
//! (`Transaction.updates` or an account change), and when the lease runs out
//! ([`MacAppStoreAdapter::time_until_check`] says when). A revalidation that cannot reach the
//! server keeps the current countdown until its lease ends; then editing locks until a check
//! succeeds. A successful one takes the server's seconds left, which the server computes as
//! Apple's purchase date plus 168 hours minus its own clock, so no sequence of leases, sleeps or
//! relaunches adds up to more than 168 hours of use. A verified lifetime purchase comes from
//! StoreKit only and is never taken away because the server cannot be reached.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

use super::bridge::{
    BridgeError, LIFETIME_PRODUCT_ID, PurchaseOutcome, StoreEnvironment, StoreKitBridge,
    TRIAL_PRODUCT_ID, VerifiedTransaction, is_compact_jws,
};
use super::policy::{
    Anomalies, EntitlementsView, MacState, MacTrialPolicy, OnlineCheckReason, ServerView,
};
use super::server::{
    LicenceServer, MAX_REQUEST_TRANSACTIONS, NONCE_BYTES, ResponseBody, SCHEMA, ServerError,
    ValidateRequest, nonce_hex, verify_response,
};
use super::unavailable::{NoNonces, UnavailableBridge, UnavailableServer};
use crate::adapters::{ChannelAdapter, PurchaseFailure};
use crate::{Channel, EntitlementState, PurchaseStatus};

/// Upper bound for the cache file; larger files are ignored unread.
pub const CACHE_MAX_BYTES: usize = 4096;

/// Current cache schema.
pub const CACHE_SCHEMA: u32 = 1;

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

/// Source of request nonces. The desktop shell supplies the operating system's secure random
/// generator; this crate has no random source of its own. `None` means no random bytes were
/// available, and the request is not sent.
pub trait NonceSource: Send + Sync {
    fn nonce(&self) -> Option<[u8; NONCE_BYTES]>;
}

/// Why the desktop shell asks for a new check. The reason is logged; every reason runs the same
/// check: StoreKit first, then the licensing server when a trial needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RevalidateReason {
    /// The app started.
    Launch,
    /// The Mac woke from sleep, or the window came back after a long idle.
    Resume,
    /// StoreKit reported a change: a `Transaction.updates` element (a purchase, a refund, a
    /// revocation) or an App Store account change.
    StoreKitChanged,
    /// The lease of the last confirmed verdict ran out (see [`MacAppStoreAdapter::time_until_check`]).
    LeaseExpired,
}

/// What the first-launch screen shows before the user starts the free trial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialOffer {
    pub duration_days: u32,
    /// What stops working when the trial ends without a purchase.
    pub what_locks: Vec<&'static str>,
    /// The lifetime price as the App Store formats it for the user's storefront.
    pub lifetime_price_display: String,
}

/// The kind of state last seen, for the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CachedKind {
    Lifetime,
    TrialActive,
    TrialEnded,
    TrialNotStarted,
    NeedsOnlineCheck,
    Revoked,
}

impl From<MacState> for CachedKind {
    fn from(s: MacState) -> Self {
        match s {
            MacState::Lifetime => CachedKind::Lifetime,
            MacState::TrialActive { .. } => CachedKind::TrialActive,
            MacState::TrialEnded => CachedKind::TrialEnded,
            MacState::TrialNotStarted { .. } => CachedKind::TrialNotStarted,
            MacState::NeedsOnlineCheck { .. } => CachedKind::NeedsOnlineCheck,
            MacState::Revoked => CachedKind::Revoked,
        }
    }
}

/// The cache file. It holds no transaction ids, no dates and no account data, and it never grants
/// anything: it only picks the words of the connectivity message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CachedState {
    pub schema: u32,
    /// Always true. Marks the file as a cache for anyone who opens it.
    pub cache_only: bool,
    pub environment: StoreEnvironment,
    pub last_state: CachedKind,
    /// The last state that was not [`CachedKind::NeedsOnlineCheck`], kept across launches that
    /// could not check, so the message still says "continue" after two offline launches.
    #[serde(default)]
    pub last_confirmed: Option<CachedKind>,
}

struct Validation {
    body: ResponseBody,
    /// Monotonic time the verdict arrived.
    at: Duration,
}

#[derive(Default)]
struct Session {
    /// The latest verified `currentEntitlements` list in this launch.
    entitlements: Option<Vec<VerifiedTransaction>>,
    /// The latest confirmed server verdict in this launch.
    validation: Option<Validation>,
    /// Why the latest server call failed, when no verdict exists.
    server_failure: Option<OnlineCheckReason>,
    purchase_pending: bool,
    /// The largest monotonic reading seen, so a reading that goes back is ignored.
    max_now: Duration,
    refreshed: bool,
    last_nonce: Option<[u8; NONCE_BYTES]>,
    anomalies: Anomalies,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the desktop shell hands over to connect the Mac App Store channel.
pub struct MacInputs {
    /// The Swift StoreKit 2 bridge.
    pub bridge: Box<dyn StoreKitBridge>,
    /// The HTTPS client for the licensing server.
    pub server: Box<dyn LicenceServer>,
    /// The operating system's secure random generator.
    pub nonces: Box<dyn NonceSource>,
    /// The monotonic session clock, normally [`InstantClock`].
    pub clock: Box<dyn MonotonicClock>,
    /// The licensing server's Ed25519 response key. `None` until the owner creates it.
    pub response_key: Option<VerifyingKey>,
}

/// The adapter as the app holds it, over whatever bridge and server the shell supplied.
pub type DynMacAppStoreAdapter =
    MacAppStoreAdapter<Box<dyn StoreKitBridge>, Box<dyn LicenceServer>>;

/// The Mac App Store adapter.
pub struct MacAppStoreAdapter<B: StoreKitBridge, S: LicenceServer> {
    bridge: B,
    server: S,
    nonces: Box<dyn NonceSource>,
    clock: Box<dyn MonotonicClock>,
    /// The licensing server's response key. `None` until the owner creates it; a trial can then
    /// never be confirmed, and the state says so instead of guessing.
    response_key: Option<VerifyingKey>,
    /// False when the build has no App Store connection ([`Self::unavailable`]). Such an adapter
    /// learns nothing, so it never writes the cache.
    store_connected: bool,
    policy: MacTrialPolicy,
    cache_path: PathBuf,
    session: Mutex<Session>,
    /// Serialises refreshes and purchases so two never interleave.
    refresh_lock: Mutex<()>,
}

impl MacAppStoreAdapter<UnavailableBridge, UnavailableServer> {
    /// The adapter for a build without an App Store connection: a Windows build, or a Mac shell
    /// that does not supply [`MacInputs`] yet. The state is always NeedsOnlineCheck, so opening
    /// and exporting work, editing is locked and nothing is ever granted.
    pub fn unavailable(app_data_dir: &Path) -> Self {
        let mut adapter = Self::new(
            UnavailableBridge,
            UnavailableServer,
            None,
            app_data_dir,
            Box::new(NoNonces),
            Box::new(InstantClock::new()),
        );
        adapter.store_connected = false;
        adapter
    }
}

impl DynMacAppStoreAdapter {
    /// The adapter over the bridge, server, nonce source and clock the desktop shell supplied.
    pub fn from_inputs(inputs: MacInputs, app_data_dir: &Path) -> Self {
        Self::new(
            inputs.bridge,
            inputs.server,
            inputs.response_key,
            app_data_dir,
            inputs.nonces,
            inputs.clock,
        )
    }
}

impl<B: StoreKitBridge, S: LicenceServer> MacAppStoreAdapter<B, S> {
    pub fn new(
        bridge: B,
        server: S,
        response_key: Option<VerifyingKey>,
        app_data_dir: &Path,
        nonces: Box<dyn NonceSource>,
        clock: Box<dyn MonotonicClock>,
    ) -> Self {
        let policy = MacTrialPolicy::new(bridge.environment());
        Self {
            bridge,
            server,
            nonces,
            clock,
            response_key,
            store_connected: true,
            policy,
            cache_path: app_data_dir.join("entitlement").join("mac-cache.json"),
            session: Mutex::new(Session::default()),
            refresh_lock: Mutex::new(()),
        }
    }

    pub fn cache_path(&self) -> &Path {
        &self.cache_path
    }

    pub fn environment(&self) -> StoreEnvironment {
        self.policy.app_environment()
    }

    /// The launch check: ask StoreKit, ask the licensing server when a trial needs it, derive the
    /// state and update the cache. The same as `revalidate(RevalidateReason::Launch)`.
    pub fn refresh(&self) -> MacState {
        self.revalidate(RevalidateReason::Launch)
    }

    /// Check again: ask StoreKit, then the licensing server when a trial needs it, and derive the
    /// state.
    ///
    /// - A revocation StoreKit reports locks at once, whether or not the server answers.
    /// - When the server cannot be reached, a verdict confirmed earlier in this session keeps
    ///   counting down until its lease ends, then editing locks
    ///   ([`OnlineCheckReason::LeaseExpired`]).
    /// - A successful check replaces the verdict with the server's, so the time left is never more
    ///   than the server says.
    /// - A verified lifetime purchase stays unlocked whatever the server does.
    pub fn revalidate(&self, reason: RevalidateReason) -> MacState {
        let _serial = lock(&self.refresh_lock);
        tracing::debug!(?reason, "entitlement check");
        self.load_entitlements();
        self.validate_online();
        self.finish()
    }

    /// How long until the current verdict's lease runs out, measured on the monotonic session
    /// clock. `None` when no trial countdown is running (no verdict, lifetime, ended, or already
    /// waiting for a check). The shell schedules `revalidate(RevalidateReason::LeaseExpired)` for
    /// that moment.
    pub fn time_until_check(&self) -> Option<Duration> {
        let mut session = lock(&self.session);
        if !matches!(self.derive(&mut session), MacState::TrialActive { .. }) {
            return None;
        }
        let now = self.now(&mut session);
        let v = session.validation.as_ref()?;
        let verdict = v.body.trial.as_ref()?;
        let window = u64::try_from(MacTrialPolicy::lease_window(verdict)).unwrap_or(0);
        Some(Duration::from_secs(window).saturating_sub(now.saturating_sub(v.at)))
    }

    /// The state now, from this session's verified data and the monotonic clock. Makes no calls,
    /// so a trial that runs out while the app is open ends here.
    pub fn mac_state(&self) -> MacState {
        let mut session = lock(&self.session);
        self.derive(&mut session)
    }

    /// The counts of transactions and answers the last derivation ignored.
    pub fn last_anomalies(&self) -> Anomalies {
        lock(&self.session).anomalies
    }

    /// What the first-launch screen shows before the trial starts. Shows nothing itself.
    pub fn start_trial(&self) -> Result<TrialOffer, BridgeError> {
        let lifetime = self.bridge.product_info(LIFETIME_PRODUCT_ID)?;
        Ok(TrialOffer {
            duration_days: 7,
            what_locks: vec![
                "Editing your projects",
                "Creating new projects",
                "Importing files",
                "Assistant changes to your projects",
            ],
            lifetime_price_display: lifetime.display_price,
        })
    }

    /// Buy the zero-price trial product. A trial that already exists is never bought again.
    pub fn purchase_trial(&self) -> Result<MacState, BridgeError> {
        self.purchase(TRIAL_PRODUCT_ID)
    }

    /// Buy the lifetime unlock.
    pub fn purchase_lifetime(&self) -> Result<MacState, BridgeError> {
        self.purchase(LIFETIME_PRODUCT_ID)
    }

    /// "Restore purchases": `AppStore.sync()`, then the launch check. When the sync fails the
    /// check still runs and the error is returned for the message.
    pub fn restore(&self) -> Result<MacState, BridgeError> {
        let synced = self.bridge.sync();
        let state = self.refresh();
        synced.map(|()| state)
    }

    /// Plain words for a [`MacState::NeedsOnlineCheck`] reason. The cache only picks the wording.
    pub fn online_check_message(&self, reason: OnlineCheckReason) -> &'static str {
        let last = self.read_cache().and_then(|c| c.last_confirmed);
        match (reason, last) {
            (OnlineCheckReason::NoConnection, Some(CachedKind::TrialActive)) => {
                "OpenFrame needs the internet to continue your free trial. Connect and open OpenFrame again. Your projects stay open and can be exported."
            }
            (OnlineCheckReason::NoConnection, _) => {
                "OpenFrame needs the internet to check your free trial. Connect and open OpenFrame again. Your projects stay open and can be exported."
            }
            (OnlineCheckReason::LeaseExpired, _) => {
                "OpenFrame needs to check your free trial again. Connect to the internet, then choose Check again. Your projects stay open and can be exported."
            }
            (OnlineCheckReason::CouldNotConfirm, _) => {
                "OpenFrame could not confirm your free trial with the App Store. Try again later. Your projects stay open and can be exported."
            }
            (OnlineCheckReason::AppStoreUnavailable, Some(CachedKind::Lifetime)) => {
                "OpenFrame could not confirm your purchase with the App Store. Check that you are signed in to the App Store, then open OpenFrame again. Your projects stay open and can be exported."
            }
            (OnlineCheckReason::AppStoreUnavailable, _) => {
                "OpenFrame could not reach the App Store. Check that you are signed in to the App Store, then open OpenFrame again. Your projects stay open and can be exported."
            }
        }
    }

    /// Read the cache. `None` when missing, unreadable, too large, of another schema, not marked
    /// as a cache, or written by a build for another environment.
    pub fn read_cache(&self) -> Option<CachedState> {
        let file = std::fs::File::open(&self.cache_path).ok()?;
        let mut bytes = Vec::with_capacity(256);
        file.take(CACHE_MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() > CACHE_MAX_BYTES {
            return None;
        }
        let cached: CachedState = serde_json::from_slice(&bytes).ok()?;
        (cached.schema == CACHE_SCHEMA
            && cached.cache_only
            && cached.environment == self.environment())
        .then_some(cached)
    }

    // ------------------------------------------------------------------ internals

    fn now(&self, session: &mut Session) -> Duration {
        let now = self.clock.now().max(session.max_now);
        session.max_now = now;
        now
    }

    fn derive(&self, session: &mut Session) -> MacState {
        let now = self.now(session);
        let (server, elapsed) = match (&session.validation, session.server_failure) {
            (Some(v), _) => (ServerView::Confirmed(&v.body), now.saturating_sub(v.at)),
            (None, Some(reason)) => (ServerView::Failed(reason), Duration::ZERO),
            (None, None) => (ServerView::NotAsked, Duration::ZERO),
        };
        let entitlements = match &session.entitlements {
            Some(txs) => EntitlementsView::Known(txs),
            None => EntitlementsView::Unavailable,
        };
        let derived =
            self.policy
                .derive_state(entitlements, server, elapsed, session.purchase_pending);
        session.anomalies = derived.anomalies;
        derived.state
    }

    /// Ask StoreKit. On success the list replaces the session list; on failure the session list
    /// from earlier in this launch stays, so a lifetime purchase keeps working.
    fn load_entitlements(&self) {
        match self.bridge.current_entitlements() {
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
                let mut session = lock(&self.session);
                session.entitlements = Some(kept);
                session.refreshed = true;
            }
            Err(e) => {
                tracing::warn!(error = ?e, "StoreKit current entitlements unavailable");
                lock(&self.session).refreshed = true;
            }
        }
    }

    /// Ask the licensing server when a trial needs it. A failure keeps any verdict confirmed
    /// earlier in this launch, so the countdown continues and is never restarted or extended.
    fn validate_online(&self) {
        let (request, nonce) = {
            let mut session = lock(&self.session);
            let Some(txs) = session.entitlements.clone() else {
                return;
            };
            let relevant = self.policy.relevant(&txs);
            if relevant.active_lifetime.is_some() || relevant.trials.is_empty() {
                session.server_failure = None;
                return;
            }
            if self.response_key.is_none() {
                session.server_failure = Some(OnlineCheckReason::CouldNotConfirm);
                return;
            }
            let nonce = match self.nonces.nonce() {
                Some(n) if Some(n) != session.last_nonce && n != [0; NONCE_BYTES] => n,
                _ => {
                    tracing::warn!("no fresh random nonce; the trial check was not sent");
                    session.server_failure = Some(OnlineCheckReason::CouldNotConfirm);
                    return;
                }
            };
            session.last_nonce = Some(nonce);
            let env = self.environment();
            let transaction_jws = txs
                .iter()
                .filter(|t| t.environment == env && t.is_well_formed())
                .take(MAX_REQUEST_TRANSACTIONS)
                .map(|t| t.signed_jws.clone())
                .collect();
            (
                ValidateRequest {
                    schema: SCHEMA,
                    nonce: nonce_hex(&nonce),
                    environment: env,
                    app_transaction_jws: None,
                    transaction_jws,
                },
                nonce,
            )
        };
        let env = self.environment();
        let mut request = request;
        request.app_transaction_jws = self
            .bridge
            .app_transaction()
            .ok()
            .filter(|a| a.environment == env && is_compact_jws(&a.signed_jws))
            .map(|a| a.signed_jws);

        let outcome = match self.server.validate(request) {
            Err(ServerError::NoConnection) => Err(OnlineCheckReason::NoConnection),
            Err(ServerError::Rejected | ServerError::Unavailable) => {
                Err(OnlineCheckReason::CouldNotConfirm)
            }
            Ok(resp) => {
                // Checked above; the key cannot have gone away.
                let key = self.response_key.as_ref();
                match key.map(|k| verify_response(&resp, k, &nonce_hex(&nonce))) {
                    Some(Ok(body)) if body.environment == env => Ok(body),
                    Some(Ok(_)) => {
                        tracing::warn!("licence response for another environment; refused");
                        Err(OnlineCheckReason::CouldNotConfirm)
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = ?e, "licence response refused");
                        Err(OnlineCheckReason::CouldNotConfirm)
                    }
                    None => Err(OnlineCheckReason::CouldNotConfirm),
                }
            }
        };

        let mut session = lock(&self.session);
        match outcome {
            Ok(body) => {
                let at = self.now(&mut session);
                session.validation = Some(Validation { body, at });
                session.server_failure = None;
            }
            Err(reason) => {
                if session.validation.is_none() {
                    session.server_failure = Some(reason);
                }
            }
        }
    }

    fn finish(&self) -> MacState {
        let state = {
            let mut session = lock(&self.session);
            let state = self.derive(&mut session);
            let a = session.anomalies;
            if a != Anomalies::default() {
                tracing::warn!(
                    environment_mismatch = a.environment_mismatch,
                    malformed = a.malformed,
                    extra_trial_purchases = a.extra_trial_purchases,
                    server_lifetime_without_storekit = a.server_lifetime_without_storekit,
                    "entitlement anomalies ignored"
                );
            }
            state
        };
        if !self.store_connected {
            return state;
        }
        if let Err(e) = self.write_cache(state) {
            tracing::warn!(kind = ?e.kind(), "the entitlement cache could not be saved");
        }
        state
    }

    fn write_cache(&self, state: MacState) -> std::io::Result<()> {
        let last_state = CachedKind::from(state);
        let previous = self.read_cache();
        let last_confirmed = if last_state == CachedKind::NeedsOnlineCheck {
            previous.as_ref().and_then(|c| c.last_confirmed)
        } else {
            Some(last_state)
        };
        let cached = CachedState {
            schema: CACHE_SCHEMA,
            cache_only: true,
            environment: self.environment(),
            last_state,
            last_confirmed,
        };
        // Checks repeat on a timer; an unchanged cache is not written again.
        if previous.as_ref() == Some(&cached) {
            return Ok(());
        }
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(&cached)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        openframe_platform::atomic::replace_bytes(&self.cache_path, &json)
    }

    fn purchase(&self, product_id: &'static str) -> Result<MacState, BridgeError> {
        let _serial = lock(&self.refresh_lock);
        {
            let mut session = lock(&self.session);
            if let Some(txs) = &session.entitlements {
                let relevant = self.policy.relevant(txs);
                let owned = if product_id == TRIAL_PRODUCT_ID {
                    !relevant.trials.is_empty() || relevant.active_lifetime.is_some()
                } else {
                    relevant.active_lifetime.is_some()
                };
                if owned {
                    // Nothing to buy: a second trial purchase would never extend the trial.
                    drop(session);
                    return Ok(self.finish());
                }
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
                {
                    let mut session = lock(&self.session);
                    let list = session.entitlements.get_or_insert_with(Vec::new);
                    if !list.iter().any(|t| t.transaction_id == tx.transaction_id) {
                        list.push(tx);
                    }
                    session.purchase_pending = false;
                }
                self.validate_online();
            }
            PurchaseOutcome::Pending => {
                if product_id == TRIAL_PRODUCT_ID {
                    lock(&self.session).purchase_pending = true;
                }
            }
            PurchaseOutcome::Cancelled => {}
        }
        Ok(self.finish())
    }
}

impl<B: StoreKitBridge, S: LicenceServer> ChannelAdapter for MacAppStoreAdapter<B, S> {
    fn channel(&self) -> Channel {
        Channel::MacAppStore
    }

    /// The purchase answer only. The Mac trial is decided by [`MacAppStoreAdapter::mac_state`]
    /// (see [`ChannelAdapter::channel_state`]), never by the local trial record.
    fn purchase_status(&self, _ignored: i64) -> PurchaseStatus {
        to_purchase_status(self.launch_state())
    }

    fn restore_purchase(&self, _ignored: i64) -> PurchaseStatus {
        to_purchase_status(self.restore_state())
    }

    fn limitation(&self) -> Option<&'static str> {
        if !self.store_connected {
            return Some("The App Store connection is not available in this build.");
        }
        self.response_key.is_none().then_some(
            "The App Store trial check is not set up yet. Opening and exporting your projects keep working.",
        )
    }

    /// The whole state. The local trial record is not used on the Mac.
    fn channel_state(&self) -> Option<EntitlementState> {
        Some(self.launch_state().to_entitlement_state())
    }

    fn restore_channel_state(&self) -> Option<EntitlementState> {
        Some(self.restore_state().to_entitlement_state())
    }

    /// StoreKit's `currentEntitlements` and the licensing server, without `AppStore.sync()`,
    /// which can show Apple's sign-in window and is for "Restore purchase" only.
    fn revalidate_channel_state(&self, reason: RevalidateReason) -> Option<EntitlementState> {
        Some(self.revalidate(reason).to_entitlement_state())
    }

    /// Apple's own payment sheet for the lifetime unlock (`Product.purchase()`).
    fn buy_full_version(&self) -> Option<Result<EntitlementState, PurchaseFailure>> {
        Some(
            self.purchase_lifetime()
                .map(MacState::to_entitlement_state)
                .map_err(purchase_failure),
        )
    }

    /// The zero-price trial product. A trial that already exists is never bought again.
    fn start_trial(&self) -> Option<Result<EntitlementState, PurchaseFailure>> {
        Some(
            self.purchase_trial()
                .map(MacState::to_entitlement_state)
                .map_err(purchase_failure),
        )
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

impl<B: StoreKitBridge, S: LicenceServer> MacAppStoreAdapter<B, S> {
    /// The launch check on the first call, then the state from this session's data.
    fn launch_state(&self) -> MacState {
        let refreshed = lock(&self.session).refreshed;
        if refreshed {
            self.mac_state()
        } else {
            self.refresh()
        }
    }

    /// "Restore purchases", or the state now when the App Store sync failed.
    fn restore_state(&self) -> MacState {
        self.restore().unwrap_or_else(|_| self.mac_state())
    }
}

fn to_purchase_status(state: MacState) -> PurchaseStatus {
    match state {
        MacState::Lifetime => PurchaseStatus::Purchased,
        MacState::Revoked => PurchaseStatus::Revoked,
        MacState::NeedsOnlineCheck { .. } => PurchaseStatus::Unknown,
        MacState::TrialActive { .. } | MacState::TrialEnded | MacState::TrialNotStarted { .. } => {
            PurchaseStatus::NotPurchased
        }
    }
}

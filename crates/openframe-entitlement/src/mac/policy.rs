//! The Mac App Store trial and unlock rules, as pure functions (ADR-0040).
//!
//! Inputs are StoreKit's verified transactions, whether the App Store answered, the Mac's wall
//! clock in UTC and the clock guard record. Nothing here reads a clock, a file or the network. The
//! wall clock is an input now that no server keeps the time, so the guard rules below stop simple
//! clock tricks.
//!
//! Rules, in the order they are applied:
//!
//! 1. Transactions from another environment than the build's (a sandbox purchase in a production
//!    build, for example) are ignored and counted as an anomaly. So are transactions that are not
//!    well formed.
//! 2. A lifetime purchase that StoreKit lists without a revocation date wins over everything.
//! 3. A lifetime purchase that StoreKit reports as refunded or revoked gives Revoked.
//! 4. No trial transaction, the App Store answered, and the guard knows no trial: not started.
//! 5. A trial transaction that Apple revoked gives Revoked.
//! 6. The App Store did not answer and no verified transaction came back. `currentEntitlements`
//!    does not fail offline, so an empty list then proves nothing and never reads as "no trial
//!    yet". A buyer stays unlocked (`lifetime_seen`), a known trial goes on from the guard's start,
//!    and anything else needs an online check.
//! 7. The start is the earlier of `purchaseDate` and `originalPurchaseDate` of the earliest verified
//!    trial transaction, or the guard's start when that is earlier. A restore or a second trial
//!    transaction never moves the start later. The end is the start plus 168 hours.
//! 8. Clock guard. The trusted floor is the latest of the guard's high water mark, the last time
//!    Apple signed a record, and the signed date of any transaction read now. A clock more than 5
//!    minutes behind the floor, or a wall clock that moved back while the app was running, gives
//!    ClockChanged. Otherwise the high water mark rises to the clock.
//! 9. At or after the end: TrialEnded. Before it: TrialActive.
//!
//! A buyer is decided by rules 2 and 6 before the clock guard, so a wrong clock never locks a buyer.

use super::bridge::{StoreEnvironment, VerifiedTransaction};
use super::clock_guard::GuardRecord;
use crate::{EntitlementState, Permission};

/// Seconds in one day.
const DAY_SECONDS: i64 = 24 * 3600;

/// How far the clock may sit behind the trusted floor. Absorbs normal clock corrections (rule 8).
pub const CLOCK_TOLERANCE_MS: i64 = 5 * 60 * 1000;

/// How close a freshly signed Apple date must be to the clock for "Check again" to trust the clock
/// and reset the high water mark (rule G4).
pub const FRESH_SIGNATURE_WINDOW_MS: i64 = 10 * 60 * 1000;

/// Why the app has to check online before editing is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlineCheckReason {
    /// The App Store could not be reached and this Mac knows no purchase and no trial.
    NoConnection,
    /// StoreKit did not answer (signed out of the App Store, App Store not available).
    AppStoreUnavailable,
}

/// The Mac App Store channel's view of the user's right to use the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacState {
    /// The paid lifetime purchase is verified, or was seen on this Mac and the store is silent.
    Lifetime,
    /// The trial is running. `remaining_seconds` is at least 1.
    TrialActive {
        remaining_seconds: i64,
        ends_at_ms: i64,
    },
    /// The 168 hours are over and there is no lifetime purchase. Nothing is permitted until a
    /// purchase or a restore (ADR-0022).
    TrialEnded,
    /// No trial was started. `purchase_pending` is true while a trial purchase waits for approval
    /// (Ask to Buy). Nothing is granted either way.
    TrialNotStarted { purchase_pending: bool },
    /// The store cannot be reached and this Mac knows nothing yet. Open and Export only.
    NeedsOnlineCheck { reason: OnlineCheckReason },
    /// Apple refunded or revoked the purchase. Nothing is permitted until a purchase or a
    /// restore (ADR-0022).
    Revoked,
    /// The Mac's clock is behind a date the app already trusted. Nothing is permitted until the
    /// clock is right (rule 8).
    ClockChanged,
}

impl MacState {
    /// Whether `perm` is allowed. TrialEnded, Revoked and ClockChanged allow nothing: the Mac build
    /// locks the whole app. In every other state Open and Export are allowed; Edit needs a verified
    /// lifetime purchase or a running trial.
    pub const fn permits(self, perm: Permission) -> bool {
        match perm {
            Permission::Open | Permission::Export => !matches!(
                self,
                MacState::TrialEnded | MacState::Revoked | MacState::ClockChanged
            ),
            Permission::Edit => {
                matches!(self, MacState::Lifetime | MacState::TrialActive { .. })
            }
        }
    }

    /// The shared entitlement state for this Mac state. Every Mac state has one, and each permits
    /// exactly what the Mac state permits.
    pub fn to_entitlement_state(self) -> EntitlementState {
        match self {
            MacState::Lifetime => EntitlementState::Purchased,
            MacState::TrialActive {
                remaining_seconds, ..
            } => {
                let days = (remaining_seconds.max(1) + DAY_SECONDS - 1) / DAY_SECONDS;
                let days_left = u32::try_from(days.clamp(1, 7)).unwrap_or(1);
                EntitlementState::TrialActive { days_left }
            }
            MacState::TrialEnded => EntitlementState::TrialEnded,
            MacState::TrialNotStarted { .. } => EntitlementState::TrialNotStarted,
            MacState::NeedsOnlineCheck { .. } => EntitlementState::NeedsOnlineCheck,
            MacState::Revoked => EntitlementState::Revoked,
            MacState::ClockChanged => EntitlementState::ClockChanged,
        }
    }

    /// The moment the running trial ends, Unix milliseconds UTC.
    pub const fn trial_ends_at_ms(self) -> Option<i64> {
        match self {
            MacState::TrialActive { ends_at_ms, .. } => Some(ends_at_ms),
            _ => None,
        }
    }
}

/// Things the policy saw and ignored. Logged as counts; never shown with any identifier.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Anomalies {
    /// Transactions from another environment than the build's.
    pub environment_mismatch: u32,
    /// Transactions that are not well formed.
    pub malformed: u32,
    /// Trial transactions beyond the first (the earliest one is used).
    pub extra_trial_purchases: u32,
}

/// What StoreKit's `currentEntitlements` gave the policy.
#[derive(Debug, Clone, Copy)]
pub enum EntitlementsView<'a> {
    /// The verified transactions from the latest successful call in this session.
    Known(&'a [VerifiedTransaction]),
    /// StoreKit has not answered in this session.
    Unavailable,
}

/// Where the trial starts. One small interface, so that a different source (for example the first
/// download date, should Apple have no free price) is a change to one file (ADR-0040).
pub trait TrialStartSource: Send + Sync {
    /// The trial start from the verified trial transactions, earliest first. `None` when they say
    /// nothing about a start.
    fn start_ms(&self, trials: &[&VerifiedTransaction]) -> Option<i64>;
}

/// The trial starts at Apple's date on the zero-price trial item: the earlier of `purchaseDate` and
/// `originalPurchaseDate` of the earliest transaction. Never the app's first download date.
#[derive(Debug, Clone, Copy, Default)]
pub struct TrialItemPurchaseDate;

impl TrialStartSource for TrialItemPurchaseDate {
    fn start_ms(&self, trials: &[&VerifiedTransaction]) -> Option<i64> {
        trials
            .first()
            .map(|t| t.purchase_date_ms.min(t.original_purchase_date_ms))
    }
}

/// The transactions that count for this build, sorted out.
#[derive(Debug, Clone, Default)]
pub struct Relevant<'a> {
    pub active_lifetime: Option<&'a VerifiedTransaction>,
    pub revoked_lifetime: bool,
    /// Trial transactions, earliest Apple date first.
    pub trials: Vec<&'a VerifiedTransaction>,
    /// The latest time Apple signed any of them, or 0.
    pub latest_signed_ms: i64,
    pub anomalies: Anomalies,
}

/// Everything one evaluation looks at.
#[derive(Debug, Clone, Copy)]
pub struct TrialInputs<'a> {
    pub entitlements: EntitlementsView<'a>,
    /// `Product.products(for:)` returned both products in this evaluation.
    pub store_reachable: bool,
    /// The Mac's clock, Unix milliseconds UTC.
    pub now_wall_ms: i64,
    /// The wall clock moved back against the session's monotonic clock (rule G3).
    pub wall_moved_back: bool,
    pub clock_guard: &'a GuardRecord,
    /// A trial purchase waits for approval (Ask to Buy).
    pub purchase_pending: bool,
}

/// The result of [`MacTrialPolicy::derive_state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    pub state: MacState,
    /// The record to keep. Differs from the input when something new was learned.
    pub clock_guard: GuardRecord,
    pub anomalies: Anomalies,
}

/// The Mac App Store trial rules for one build environment.
#[derive(Clone, Copy)]
pub struct MacTrialPolicy {
    app_environment: StoreEnvironment,
    start_source: &'static dyn TrialStartSource,
}

impl MacTrialPolicy {
    /// Length of the trial: exactly 168 hours from the start.
    pub const TRIAL_SECONDS: i64 = 168 * 3600;

    pub const fn new(app_environment: StoreEnvironment) -> Self {
        Self {
            app_environment,
            start_source: &TrialItemPurchaseDate,
        }
    }

    pub const fn app_environment(&self) -> StoreEnvironment {
        self.app_environment
    }

    /// Sort `txs` into what counts for this build (rule 1).
    pub fn relevant<'a>(&self, txs: &'a [VerifiedTransaction]) -> Relevant<'a> {
        let mut r = Relevant::default();
        for tx in txs {
            if !tx.is_well_formed() {
                r.anomalies.malformed += 1;
                continue;
            }
            if tx.environment != self.app_environment {
                r.anomalies.environment_mismatch += 1;
                continue;
            }
            r.latest_signed_ms = r.latest_signed_ms.max(tx.signed_date_ms);
            if tx.is_lifetime() {
                if tx.is_revoked() {
                    r.revoked_lifetime = true;
                } else if r.active_lifetime.is_none() {
                    r.active_lifetime = Some(tx);
                }
            } else if tx.is_trial() {
                r.trials.push(tx);
            }
        }
        // Earliest Apple date first; the transaction id breaks ties so the choice is the same on
        // every Mac.
        r.trials.sort_by(|a, b| {
            (
                a.purchase_date_ms.min(a.original_purchase_date_ms),
                &a.transaction_id,
            )
                .cmp(&(
                    b.purchase_date_ms.min(b.original_purchase_date_ms),
                    &b.transaction_id,
                ))
        });
        r.anomalies.extra_trial_purchases =
            u32::try_from(r.trials.len().saturating_sub(1)).unwrap_or(u32::MAX);
        r
    }

    /// Derive the state and the guard record to keep.
    pub fn derive_state(&self, input: TrialInputs<'_>) -> Derived {
        let (txs, answered): (&[VerifiedTransaction], bool) = match input.entitlements {
            EntitlementsView::Known(txs) => (txs, true),
            EntitlementsView::Unavailable => (&[], false),
        };
        let relevant = self.relevant(txs);
        let mut record = input.clock_guard.clone();
        // Only ever rises (rule G1).
        record.last_apple_signed_ms = record.last_apple_signed_ms.max(relevant.latest_signed_ms);
        let state = self.decide(&relevant, answered, input, &mut record);
        Derived {
            state,
            clock_guard: record,
            anomalies: relevant.anomalies,
        }
    }

    fn decide(
        &self,
        r: &Relevant<'_>,
        answered: bool,
        input: TrialInputs<'_>,
        record: &mut GuardRecord,
    ) -> MacState {
        // Rules 2 and 3.
        if r.active_lifetime.is_some() {
            record.lifetime_seen = true;
            self.remember_trial(r, record);
            advance(input, record);
            return MacState::Lifetime;
        }
        if r.revoked_lifetime {
            // A refund is final until a new purchase: a later offline launch must not read as
            // "bought".
            record.lifetime_seen = false;
            advance(input, record);
            return MacState::Revoked;
        }
        // Rule 5. A revoked trial is listed with its date, so check before the rest.
        if r.trials.iter().any(|t| t.is_revoked()) {
            advance(input, record);
            return MacState::Revoked;
        }
        if r.trials.is_empty() {
            // A person who bought on this Mac stays unlocked while the store shows nothing for
            // this account, whether or not the network is up (ADR-0040): a refund arrives as a
            // transaction with a revocation date and is handled above.
            if record.lifetime_seen {
                advance(input, record);
                return MacState::Lifetime;
            }
            if record.trial_start_ms.is_none() {
                if input.store_reachable && answered {
                    // Rule 4.
                    advance(input, record);
                    return MacState::TrialNotStarted {
                        purchase_pending: input.purchase_pending,
                    };
                }
                // Rule 6.
                return MacState::NeedsOnlineCheck {
                    reason: if answered {
                        OnlineCheckReason::NoConnection
                    } else {
                        OnlineCheckReason::AppStoreUnavailable
                    },
                };
            }
        }
        // Rule 7.
        self.remember_trial(r, record);
        let Some(start) = record.trial_start_ms else {
            return MacState::NeedsOnlineCheck {
                reason: OnlineCheckReason::NoConnection,
            };
        };
        // Rule 8.
        if clock_is_behind(input, record) {
            return MacState::ClockChanged;
        }
        record.high_water_ms = record.high_water_ms.max(input.now_wall_ms);
        // Rule 9.
        let end = start.saturating_add(Self::TRIAL_SECONDS * 1000);
        if input.now_wall_ms >= end {
            return MacState::TrialEnded;
        }
        let left_ms = end - input.now_wall_ms;
        MacState::TrialActive {
            remaining_seconds: (left_ms + 999) / 1000,
            ends_at_ms: end,
        }
    }

    /// Keep the earliest start ever seen (rule 7). Never moves later.
    fn remember_trial(&self, r: &Relevant<'_>, record: &mut GuardRecord) {
        let Some(found) = self.start_source.start_ms(&r.trials) else {
            return;
        };
        if record.trial_start_ms.is_none_or(|kept| found < kept) {
            record.trial_start_ms = Some(found);
            record.trial_original_tx = r.trials.first().map(|t| t.original_transaction_id.clone());
        }
    }
}

/// Rule 8: the clock sits behind what the app already trusted.
fn clock_is_behind(input: TrialInputs<'_>, record: &GuardRecord) -> bool {
    let floor = record.high_water_ms.max(record.last_apple_signed_ms);
    input.wall_moved_back || input.now_wall_ms < floor.saturating_sub(CLOCK_TOLERANCE_MS)
}

/// Raise the high water mark when the clock passes the check. Used in states the clock does not
/// decide, so a later trial still has a floor.
fn advance(input: TrialInputs<'_>, record: &mut GuardRecord) {
    if !clock_is_behind(input, record) {
        record.high_water_ms = record.high_water_ms.max(input.now_wall_ms);
    }
}

/// Rule G4: "Check again" with a wrong future clock since fixed. When Apple signed something within
/// 10 minutes of the Mac's clock, Apple's time confirms the clock, and the high water mark comes
/// back to it. Returns true when the record changed. This is the only place a high water mark ever
/// goes down.
pub fn recover_from_future_clock(
    record: &mut GuardRecord,
    fresh_signed_ms: Option<i64>,
    now_wall_ms: i64,
) -> bool {
    let Some(signed) = fresh_signed_ms else {
        return false;
    };
    if signed.abs_diff(now_wall_ms) > FRESH_SIGNATURE_WINDOW_MS.unsigned_abs() {
        return false;
    }
    let changed = record.high_water_ms > now_wall_ms;
    record.high_water_ms = record.high_water_ms.min(now_wall_ms);
    record.last_apple_signed_ms = record.last_apple_signed_ms.max(signed);
    changed
}

/// The fallback for rule G4, should a refreshed transaction turn out not to carry a fresh signed
/// date (only the sandbox can show this, see ADR-0040): cap the high water mark at the last Apple
/// signature plus 2 days. Not in use while [`crate::mac::adapter::G4_USES_FRESH_SIGNATURE`] is
/// true.
pub fn cap_high_water_at_apple_date(record: &mut GuardRecord) -> bool {
    let cap = record
        .last_apple_signed_ms
        .saturating_add(2 * DAY_SECONDS * 1000);
    let changed = record.last_apple_signed_ms > 0 && record.high_water_ms > cap;
    if changed {
        record.high_water_ms = cap;
    }
    changed
}

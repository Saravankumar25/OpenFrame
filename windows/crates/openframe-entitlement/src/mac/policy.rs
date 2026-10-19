//! The Mac App Store trial and unlock rules, as pure functions.
//!
//! Inputs are StoreKit's verified transactions, the licensing server's verified verdict and the
//! time elapsed on the monotonic session clock since that verdict arrived. Nothing here reads a
//! clock, a file or the network. The wall clock, the time zone, the install date and any local
//! file are never inputs, so changing them cannot change the answer.
//!
//! Rules, in the order they are applied:
//!
//! 1. Transactions from another environment than the build's (a sandbox purchase in a production
//!    build, for example) are ignored and counted as an anomaly. So are transactions that are not
//!    well formed.
//! 2. A lifetime purchase that StoreKit lists without a revocation date wins over everything.
//! 3. A lifetime purchase that StoreKit reports as refunded or revoked gives Revoked.
//! 4. No trial transaction: the trial has not started.
//! 5. A trial transaction that Apple revoked gives Revoked.
//! 6. A trial transaction with no verified server verdict: the app must check online. It never
//!    reads as "not started", which would offer a second trial.
//! 7. The verdict must name the earliest trial transaction and must not end later than its Apple
//!    purchase date plus exactly 168 hours. A second trial purchase never moves the start. An end
//!    that is earlier (the server used the transaction's earlier original purchase date) only
//!    shortens the trial and is accepted.
//! 8. Seconds left come from the server only and count down on the monotonic session clock.
//! 9. A verdict is good for `min(leaseSeconds, remainingSeconds)` of monotonic session time. When
//!    the trial time is used up the state is TrialEnded; when only the lease is used up the state
//!    is NeedsOnlineCheck with [`OnlineCheckReason::LeaseExpired`] until a new check succeeds.

use std::time::Duration;

use super::bridge::{StoreEnvironment, VerifiedTransaction};
use super::server::{ResponseBody, TrialVerdict};
use crate::{EntitlementState, Permission};

/// Seconds in one day.
const DAY_SECONDS: i64 = 24 * 3600;

/// Why the app has to check online before editing is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlineCheckReason {
    /// The licensing server could not be reached.
    NoConnection,
    /// The server or StoreKit answered, but the answer could not be confirmed: a rejected JWS, a
    /// response with a bad signature or another request's nonce, or a verdict that does not match
    /// Apple's purchase date.
    CouldNotConfirm,
    /// StoreKit did not answer (signed out of the App Store, App Store not available).
    AppStoreUnavailable,
    /// The trial was confirmed earlier in this session, but the confirmation's lease has run
    /// out. Editing waits for a new successful check.
    LeaseExpired,
}

/// The Mac App Store channel's view of the user's right to use the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacState {
    /// The paid lifetime purchase is verified.
    Lifetime,
    /// The trial is running. `remaining_seconds` is at least 1.
    TrialActive { remaining_seconds: i64 },
    /// The 168 hours are over and there is no lifetime purchase. Nothing is permitted until a
    /// purchase or a restore (ADR-0022).
    TrialEnded,
    /// No trial transaction exists. `purchase_pending` is true while a trial purchase waits for
    /// approval (Ask to Buy). Nothing is granted either way.
    TrialNotStarted { purchase_pending: bool },
    /// A trial exists but could not be confirmed online. Open and Export only.
    NeedsOnlineCheck { reason: OnlineCheckReason },
    /// Apple refunded or revoked the purchase. Nothing is permitted until a purchase or a
    /// restore (ADR-0022).
    Revoked,
}

impl MacState {
    /// Whether `perm` is allowed. TrialEnded and Revoked allow nothing: the Mac build locks the
    /// whole app until a purchase or a restore (owner decision 2026-10-07, ADR-0022). In every
    /// other state Open and Export are allowed (a failed online check keeps them, ADR-0018
    /// decision 6); Edit needs a verified lifetime purchase or a running, server-confirmed trial.
    pub const fn permits(self, perm: Permission) -> bool {
        match perm {
            Permission::Open | Permission::Export => {
                !matches!(self, MacState::TrialEnded | MacState::Revoked)
            }
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
            MacState::TrialActive { remaining_seconds } => {
                let days = (remaining_seconds.max(1) + DAY_SECONDS - 1) / DAY_SECONDS;
                let days_left = u32::try_from(days.clamp(1, 7)).unwrap_or(1);
                EntitlementState::TrialActive { days_left }
            }
            MacState::TrialEnded => EntitlementState::TrialEnded,
            MacState::TrialNotStarted { .. } => EntitlementState::TrialNotStarted,
            MacState::NeedsOnlineCheck { .. } => EntitlementState::NeedsOnlineCheck,
            MacState::Revoked => EntitlementState::Revoked,
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
    /// The server reported a lifetime purchase that StoreKit did not list.
    pub server_lifetime_without_storekit: u32,
}

/// What StoreKit's `currentEntitlements` gave the policy.
#[derive(Debug, Clone, Copy)]
pub enum EntitlementsView<'a> {
    /// The verified transactions from the latest successful call in this session.
    Known(&'a [VerifiedTransaction]),
    /// StoreKit has not answered in this session.
    Unavailable,
}

/// What the licensing server gave the policy.
#[derive(Debug, Clone, Copy)]
pub enum ServerView<'a> {
    /// No server call was made or needed.
    NotAsked,
    /// A response whose signature, nonce and environment were checked.
    Confirmed(&'a ResponseBody),
    /// The call failed and no confirmed verdict exists in this session.
    Failed(OnlineCheckReason),
}

/// The transactions that count for this build, sorted out.
#[derive(Debug, Clone, Default)]
pub struct Relevant<'a> {
    pub active_lifetime: Option<&'a VerifiedTransaction>,
    pub revoked_lifetime: bool,
    /// Trial transactions, earliest Apple purchase date first.
    pub trials: Vec<&'a VerifiedTransaction>,
    pub anomalies: Anomalies,
}

impl Relevant<'_> {
    /// The trial transaction that decides the trial: the earliest Apple purchase date.
    pub fn earliest_trial(&self) -> Option<&VerifiedTransaction> {
        self.trials.first().copied()
    }
}

/// The result of [`MacTrialPolicy::derive_state`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Derived {
    pub state: MacState,
    pub anomalies: Anomalies,
}

/// The Mac App Store trial rules for one build environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacTrialPolicy {
    app_environment: StoreEnvironment,
}

impl MacTrialPolicy {
    /// Length of the trial: exactly 168 hours, counted by the server from Apple's purchase date.
    pub const TRIAL_SECONDS: i64 = 168 * 3600;

    pub const fn new(app_environment: StoreEnvironment) -> Self {
        Self { app_environment }
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
        // Earliest Apple purchase date first; the transaction id breaks ties so the choice is the
        // same on every Mac.
        r.trials.sort_by(|a, b| {
            (a.purchase_date_ms, &a.transaction_id).cmp(&(b.purchase_date_ms, &b.transaction_id))
        });
        r.anomalies.extra_trial_purchases =
            u32::try_from(r.trials.len().saturating_sub(1)).unwrap_or(u32::MAX);
        r
    }

    /// Derive the state.
    ///
    /// `elapsed_since_validation` is the monotonic session time since the confirmed verdict in
    /// `server` arrived. `purchase_pending` is true while a trial purchase waits for approval.
    pub fn derive_state(
        &self,
        entitlements: EntitlementsView<'_>,
        server: ServerView<'_>,
        elapsed_since_validation: Duration,
        purchase_pending: bool,
    ) -> Derived {
        let EntitlementsView::Known(txs) = entitlements else {
            return Derived {
                state: MacState::NeedsOnlineCheck {
                    reason: OnlineCheckReason::AppStoreUnavailable,
                },
                anomalies: Anomalies::default(),
            };
        };
        let mut relevant = self.relevant(txs);
        if let ServerView::Confirmed(body) = server
            && body.lifetime
            && relevant.active_lifetime.is_none()
        {
            relevant.anomalies.server_lifetime_without_storekit += 1;
        }
        let state = self.decide(
            &relevant,
            server,
            elapsed_since_validation,
            purchase_pending,
        );
        Derived {
            state,
            anomalies: relevant.anomalies,
        }
    }

    fn decide(
        &self,
        r: &Relevant<'_>,
        server: ServerView<'_>,
        elapsed: Duration,
        purchase_pending: bool,
    ) -> MacState {
        // Rules 2 and 3.
        if r.active_lifetime.is_some() {
            return MacState::Lifetime;
        }
        if r.revoked_lifetime {
            return MacState::Revoked;
        }
        // Rule 4.
        let Some(earliest) = r.earliest_trial() else {
            return MacState::TrialNotStarted { purchase_pending };
        };
        // Rule 5.
        if r.trials.iter().any(|t| t.is_revoked()) {
            return MacState::Revoked;
        }
        // Rule 6.
        let body = match server {
            ServerView::Confirmed(body) => body,
            ServerView::Failed(reason) => return MacState::NeedsOnlineCheck { reason },
            ServerView::NotAsked => {
                return MacState::NeedsOnlineCheck {
                    reason: OnlineCheckReason::CouldNotConfirm,
                };
            }
        };
        if body.environment != self.app_environment {
            return could_not_confirm();
        }
        if body.revoked {
            return MacState::Revoked;
        }
        let Some(verdict) = &body.trial else {
            return could_not_confirm();
        };
        // Rule 7.
        if !self.verdict_matches(verdict, body.server_time_ms, earliest) {
            return could_not_confirm();
        }
        // Rule 8.
        let elapsed_secs = i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX);
        let left = verdict.remaining_seconds.saturating_sub(elapsed_secs);
        if left <= 0 {
            return MacState::TrialEnded;
        }
        // Rule 9.
        if elapsed_secs >= Self::lease_window(verdict) {
            return MacState::NeedsOnlineCheck {
                reason: OnlineCheckReason::LeaseExpired,
            };
        }
        MacState::TrialActive {
            remaining_seconds: left,
        }
    }

    /// Seconds of monotonic session time a verdict may be counted down: its lease, never more
    /// than its seconds left (rule 9).
    pub fn lease_window(verdict: &TrialVerdict) -> i64 {
        verdict
            .lease_seconds
            .clamp(0, verdict.remaining_seconds.max(0))
    }

    /// Whether the server's verdict belongs to `earliest` and is consistent with itself.
    fn verdict_matches(
        &self,
        verdict: &TrialVerdict,
        server_time: i64,
        earliest: &VerifiedTransaction,
    ) -> bool {
        let names_earliest = verdict.apple_transaction_id == earliest.transaction_id
            || verdict.apple_transaction_id == earliest.original_transaction_id;
        let latest_end = earliest
            .purchase_date_ms
            .checked_add(Self::TRIAL_SECONDS * 1000);
        let end_matches = latest_end.is_some_and(|end| verdict.trial_ends_at_ms <= end);
        let in_range = (0..=Self::TRIAL_SECONDS).contains(&verdict.remaining_seconds);
        // The seconds left must agree with the server's own end and clock, within one second for
        // rounding. A verdict that grants more than its own numbers allow is refused.
        let by_server_clock = verdict.trial_ends_at_ms.saturating_sub(server_time).max(0) / 1000;
        let consistent = (verdict.remaining_seconds - by_server_clock).abs() <= 1;
        names_earliest && end_matches && in_range && consistent
    }
}

const fn could_not_confirm() -> MacState {
    MacState::NeedsOnlineCheck {
        reason: OnlineCheckReason::CouldNotConfirm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mac::bridge::{LIFETIME_PRODUCT_ID, TRIAL_PRODUCT_ID};
    use crate::mac::server::SCHEMA;

    const P0: i64 = 1_791_244_800_000;
    const PROD: StoreEnvironment = StoreEnvironment::Production;

    fn tx(product: &str, id: &str, purchase: i64) -> VerifiedTransaction {
        VerifiedTransaction {
            product_id: product.to_string(),
            transaction_id: id.to_string(),
            original_transaction_id: id.to_string(),
            app_transaction_id: Some("705".to_string()),
            purchase_date_ms: purchase,
            revocation_date_ms: None,
            environment: PROD,
            signed_jws: "a.b.c".to_string(),
        }
    }

    fn verdict_body(id: &str, purchase: i64, server_time: i64) -> ResponseBody {
        let ends = purchase + MacTrialPolicy::TRIAL_SECONDS * 1000;
        ResponseBody {
            schema: SCHEMA,
            nonce: "n".to_string(),
            environment: PROD,
            server_time_ms: server_time,
            trial: Some(TrialVerdict {
                trial_ends_at_ms: ends,
                remaining_seconds: ((ends - server_time) / 1000).max(0),
                apple_transaction_id: id.to_string(),
                lease_seconds: crate::mac::server::DEFAULT_LEASE_SECONDS,
            }),
            lifetime: false,
            revoked: false,
        }
    }

    fn derive(txs: &[VerifiedTransaction], server: ServerView<'_>, elapsed: u64) -> MacState {
        MacTrialPolicy::new(PROD)
            .derive_state(
                EntitlementsView::Known(txs),
                server,
                Duration::from_secs(elapsed),
                false,
            )
            .state
    }

    #[test]
    fn lifetime_wins_over_any_trial_and_any_server_answer() {
        let txs = [
            tx(TRIAL_PRODUCT_ID, "t1", P0),
            tx(LIFETIME_PRODUCT_ID, "l1", P0 + 1000),
        ];
        assert_eq!(
            derive(&txs, ServerView::Failed(OnlineCheckReason::NoConnection), 0),
            MacState::Lifetime
        );
        assert_eq!(derive(&txs, ServerView::NotAsked, 0), MacState::Lifetime);
    }

    #[test]
    fn a_trial_without_a_confirmed_verdict_is_never_not_started() {
        let txs = [tx(TRIAL_PRODUCT_ID, "t1", P0)];
        assert_eq!(
            derive(&txs, ServerView::NotAsked, 0),
            MacState::NeedsOnlineCheck {
                reason: OnlineCheckReason::CouldNotConfirm
            }
        );
    }

    #[test]
    fn a_verdict_that_does_not_match_apples_date_or_grants_too_much_is_refused() {
        let txs = [tx(TRIAL_PRODUCT_ID, "t1", P0)];
        let now = P0 + 3600 * 1000;
        let good = verdict_body("t1", P0, now);
        assert_eq!(
            derive(&txs, ServerView::Confirmed(&good), 0),
            MacState::TrialActive {
                remaining_seconds: MacTrialPolicy::TRIAL_SECONDS - 3600
            }
        );

        // Computed from a later start.
        let later = verdict_body("t1", P0 + 86_400_000, now);
        assert_eq!(
            derive(&txs, ServerView::Confirmed(&later), 0),
            could_not_confirm()
        );
        // Computed from an earlier original purchase date: shorter, so accepted.
        let earlier = verdict_body("t1", P0 - 3_600_000, now);
        assert_eq!(
            derive(&txs, ServerView::Confirmed(&earlier), 0),
            MacState::TrialActive {
                remaining_seconds: MacTrialPolicy::TRIAL_SECONDS - 7200
            }
        );
        // More seconds than its own end allows.
        let mut greedy = good.clone();
        greedy.trial.as_mut().unwrap().remaining_seconds += 3600;
        assert_eq!(
            derive(&txs, ServerView::Confirmed(&greedy), 0),
            could_not_confirm()
        );
        // Another transaction.
        let other = verdict_body("t9", P0, now);
        assert_eq!(
            derive(&txs, ServerView::Confirmed(&other), 0),
            could_not_confirm()
        );
        // No trial verdict at all.
        let mut empty = good.clone();
        empty.trial = None;
        assert_eq!(
            derive(&txs, ServerView::Confirmed(&empty), 0),
            could_not_confirm()
        );
        // A verdict for the sandbox in a production build.
        let mut sandbox = good;
        sandbox.environment = StoreEnvironment::Sandbox;
        assert_eq!(
            derive(&txs, ServerView::Confirmed(&sandbox), 0),
            could_not_confirm()
        );
    }

    #[test]
    fn a_verdict_is_good_for_its_lease_and_never_beyond_the_seconds_left() {
        let txs = [tx(TRIAL_PRODUCT_ID, "t1", P0)];
        let now = P0 + 3600 * 1000;
        let mut body = verdict_body("t1", P0, now);
        body.trial.as_mut().unwrap().lease_seconds = 600;
        let left = MacTrialPolicy::TRIAL_SECONDS - 3600;
        assert_eq!(
            derive(&txs, ServerView::Confirmed(&body), 599),
            MacState::TrialActive {
                remaining_seconds: left - 599
            }
        );
        let lease_over = MacState::NeedsOnlineCheck {
            reason: OnlineCheckReason::LeaseExpired,
        };
        assert_eq!(derive(&txs, ServerView::Confirmed(&body), 600), lease_over);
        assert!(!lease_over.permits(Permission::Edit));
        assert_eq!(
            lease_over.to_entitlement_state(),
            EntitlementState::NeedsOnlineCheck
        );

        // A lease longer than the trial: the trial end comes first and reads as ended.
        let end = P0 + MacTrialPolicy::TRIAL_SECONDS * 1000;
        let mut short = verdict_body("t1", P0, end - 30_000);
        short.trial.as_mut().unwrap().lease_seconds = 3600;
        assert_eq!(
            MacTrialPolicy::lease_window(short.trial.as_ref().unwrap()),
            30
        );
        assert_eq!(
            derive(&txs, ServerView::Confirmed(&short), 29),
            MacState::TrialActive {
                remaining_seconds: 1
            }
        );
        assert_eq!(
            derive(&txs, ServerView::Confirmed(&short), 30),
            MacState::TrialEnded
        );
    }

    #[test]
    fn unavailable_storekit_needs_an_online_check() {
        let d = MacTrialPolicy::new(PROD).derive_state(
            EntitlementsView::Unavailable,
            ServerView::NotAsked,
            Duration::ZERO,
            false,
        );
        assert_eq!(
            d.state,
            MacState::NeedsOnlineCheck {
                reason: OnlineCheckReason::AppStoreUnavailable
            }
        );
    }

    #[test]
    fn only_lifetime_and_a_running_trial_permit_editing_and_an_ended_or_revoked_one_permits_nothing()
     {
        use MacState::*;
        for s in [
            Lifetime,
            TrialActive {
                remaining_seconds: 1,
            },
            TrialEnded,
            TrialNotStarted {
                purchase_pending: true,
            },
            NeedsOnlineCheck {
                reason: OnlineCheckReason::NoConnection,
            },
            Revoked,
        ] {
            let open = !matches!(s, TrialEnded | Revoked);
            assert_eq!(s.permits(Permission::Open), open, "{s:?}");
            assert_eq!(s.permits(Permission::Export), open, "{s:?}");
            let edit = matches!(s, Lifetime | TrialActive { .. });
            assert_eq!(s.permits(Permission::Edit), edit, "{s:?}");
            let shared = s.to_entitlement_state();
            let mac = crate::Channel::MacAppStore;
            assert_eq!(shared.permits(mac, Permission::Edit), edit, "{s:?}");
            assert_eq!(shared.permits(mac, Permission::Open), open, "{s:?}");
            assert_eq!(shared.permits(mac, Permission::Export), open, "{s:?}");
        }
        assert_eq!(
            TrialActive {
                remaining_seconds: 1
            }
            .to_entitlement_state(),
            EntitlementState::TrialActive { days_left: 1 }
        );
        assert_eq!(
            TrialActive {
                remaining_seconds: MacTrialPolicy::TRIAL_SECONDS
            }
            .to_entitlement_state(),
            EntitlementState::TrialActive { days_left: 7 }
        );
        assert_eq!(
            TrialNotStarted {
                purchase_pending: false
            }
            .to_entitlement_state(),
            EntitlementState::TrialNotStarted
        );
    }
}

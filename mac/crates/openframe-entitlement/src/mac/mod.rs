//! Mac App Store channel: StoreKit 2 non-subscription trial and lifetime unlock (owner decision
//! 2026-10-06; ADR-0018).
//!
//! The model:
//!
//! - A zero-price non-consumable, "7-day Trial" ([`bridge::TRIAL_PRODUCT_ID`]), starts the trial.
//! - A paid non-consumable, "Lifetime Access" ([`bridge::LIFETIME_PRODUCT_ID`]), unlocks the app.
//! - No subscription. Family Sharing off.
//!
//! Only transactions StoreKit verified, and licensing server answers whose signature and nonce
//! were checked, decide anything. The install date, the wall clock, the time zone, preferences,
//! the database, files, the Keychain and the interface never do. The local file this module writes
//! is a cache that only picks the words of a message.
//!
//! The trial ends exactly 168 hours after Apple's recorded purchase date of the earliest trial
//! transaction (never the app's first download date, `AppTransaction.originalPurchaseDate`). The
//! licensing server computes that in UTC and returns the seconds left on its own clock and a lease;
//! the app counts down on a monotonic session clock for at most the lease, then checks again.
//! Without a confirmed answer, or once the lease has run out, the state is
//! [`policy::MacState::NeedsOnlineCheck`]: opening and exporting stay allowed, editing locks, and
//! the trial is neither restarted nor extended. Apple's `appTransactionID` is optional (macOS 15.4
//! and later) and only links an Apple Account to the app.
//!
//! Nothing in this module calls StoreKit or the network. The desktop shell implements
//! [`bridge::StoreKitBridge`] in Swift, [`server::LicenceServer`] over HTTPS and
//! [`adapter::NonceSource`] with the system's secure random generator, and hands them over as
//! [`adapter::MacInputs`]. Until it does, the adapter runs over [`unavailable::UnavailableBridge`]
//! and [`unavailable::UnavailableServer`] and the state is `NeedsOnlineCheck`.

pub mod adapter;
pub mod bridge;
pub mod policy;
pub mod server;
pub mod unavailable;

#[cfg(test)]
mod tests;

pub use adapter::{
    CachedKind, CachedState, DynMacAppStoreAdapter, InstantClock, MacAppStoreAdapter, MacInputs,
    MonotonicClock, NonceSource, RevalidateReason, TrialOffer,
};
pub use bridge::{
    AppTransactionInfo, BridgeError, LIFETIME_PRODUCT_ID, ProductInfo, PurchaseOutcome,
    RawVerification, StoreEnvironment, StoreKitBridge, TRIAL_PRODUCT_ID, VerifiedSet,
    VerifiedTransaction, keep_verified,
};
pub use policy::{
    Anomalies, Derived, EntitlementsView, MacState, MacTrialPolicy, OnlineCheckReason, ServerView,
};
pub use server::{
    DEFAULT_LEASE_SECONDS, LicenceServer, RESPONSE_DOMAIN, ResponseBody, ResponseError,
    ServerError, TrialVerdict, ValidateRequest, ValidateResponse, verify_response,
};
pub use unavailable::{NoNonces, UnavailableBridge, UnavailableServer};

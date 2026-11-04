//! Mac App Store channel: StoreKit 2 non-subscription trial and lifetime unlock, counted offline on
//! this Mac (owner decision 2026-11-03; ADR-0040).
//!
//! The model:
//!
//! - A zero-price non-consumable, "7-day Trial" ([`bridge::TRIAL_PRODUCT_ID`]), starts the trial.
//! - A paid non-consumable, "Lifetime Access" ([`bridge::LIFETIME_PRODUCT_ID`]), unlocks the app.
//! - No subscription. Family Sharing off. No licence server, no device binding, no analytics.
//!
//! Only transactions StoreKit verified decide anything. The trial ends exactly 168 hours after
//! Apple's date on the earliest trial transaction (never the app's first download date,
//! `AppTransaction.originalPurchaseDate`). The Mac's clock is an input, so the guard record
//! ([`clock_guard`]) and the rules in [`policy`] stop simple clock tricks. Apple's `appTransactionID`
//! is optional (macOS 15.4 and later) and only links an Apple Account to the app.
//!
//! Nothing in this module calls StoreKit, the Keychain or the network. The desktop shell implements
//! [`bridge::StoreKitBridge`] in Swift and hands it over as [`adapter::MacInputs`]. Until it does,
//! the adapter runs over [`unavailable::UnavailableBridge`] and the state is `NeedsOnlineCheck`.

pub mod adapter;
pub mod bridge;
pub mod clock_guard;
pub mod policy;
pub mod unavailable;

#[cfg(test)]
mod tests;

pub use adapter::{
    DynMacAppStoreAdapter, InstantClock, MacAppStoreAdapter, MacInputs, MonotonicClock,
    PURCHASE_PENDING, PurchaseEnd, RevalidateReason, SystemWallClock, WallClock,
};
pub use bridge::{
    AppTransactionInfo, BridgeError, LIFETIME_PRODUCT_ID, PurchaseOutcome, RawVerification,
    StoreEnvironment, StoreKitBridge, StoreProducts, TRIAL_PRODUCT_ID, VerifiedSet,
    VerifiedTransaction, keep_verified,
};
pub use clock_guard::{GuardRecord, ParsedGuard};
pub use policy::{
    Anomalies, Derived, EntitlementsView, MacState, MacTrialPolicy, OnlineCheckReason, TrialInputs,
    TrialItemPurchaseDate, TrialStartSource,
};
pub use unavailable::UnavailableBridge;

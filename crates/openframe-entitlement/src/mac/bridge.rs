//! The contract between the Rust core and the StoreKit 2 code on the Mac.
//!
//! The Swift side lives in the desktop shell and implements [`StoreKitBridge`]. This file holds
//! only the types and the rules both sides follow; nothing here calls StoreKit, so it compiles and
//! is tested on every operating system.
//!
//! The bridge contract, which the Swift implementation must keep:
//!
//! 1. **Only verified transactions cross the bridge.** StoreKit hands every transaction over as a
//!    `VerificationResult`. The bridge passes on `.verified` results only. Every `.unverified`
//!    result is dropped and counted, never passed on in any form. [`keep_verified`] is the one
//!    function the shell uses to apply this rule, so the Rust side and the tests agree on it.
//! 2. **Apple's dates.** `purchase_date_ms`, `original_purchase_date_ms`, `signed_date_ms` and
//!    `revocation_date_ms` are the dates Apple recorded in the signed transaction, converted to Unix
//!    milliseconds in UTC. `signed_date_ms` is the time Apple signed the record, so the real time is
//!    never earlier than it (the clock guard's floor).
//! 3. **The environment is reported on every transaction** (Sandbox, Production, Xcode) so test
//!    purchases and real purchases are never mixed.
//! 4. **`appTransactionID` is optional.** Apple provides it on macOS 15.4 and later only. It links
//!    an Apple Account to this app and nothing else; it is never a trial date. The trial starts at
//!    the trial transaction's own Apple date. `AppTransaction.originalPurchaseDate` (the date the
//!    app was first downloaded) is never carried across the bridge and never used.

use serde::{Deserialize, Serialize};

/// The zero-price non-consumable that starts the 7-day trial. Never change this after the first
/// release: a new id is a different product with no purchases.
pub const TRIAL_PRODUCT_ID: &str = "app.openframestudio.studio.trial7";

/// The paid non-consumable that unlocks the app for good. Never change this after the first release.
pub const LIFETIME_PRODUCT_ID: &str = "app.openframestudio.studio.lifetime";

/// Upper bound for an identifier field (transaction ids, app transaction id, product id).
pub const MAX_ID_BYTES: usize = 128;

/// Where a transaction was made. Test purchases and real purchases are never mixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreEnvironment {
    /// App Store sandbox, including TestFlight and Apple's own App Review.
    Sandbox,
    /// The real App Store.
    Production,
    /// StoreKit testing inside Xcode.
    Xcode,
}

/// One transaction that StoreKit verified on this Mac.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTransaction {
    pub product_id: String,
    pub transaction_id: String,
    pub original_transaction_id: String,
    /// Apple's `appTransactionID`: the same for every transaction of one Apple Account and app.
    /// `None` before macOS 15.4, where StoreKit does not provide it.
    #[serde(default)]
    pub app_transaction_id: Option<String>,
    /// The purchase date Apple recorded, Unix milliseconds UTC.
    pub purchase_date_ms: i64,
    /// The original purchase date Apple recorded for this transaction, Unix milliseconds UTC.
    pub original_purchase_date_ms: i64,
    /// The time Apple signed this record, Unix milliseconds UTC.
    pub signed_date_ms: i64,
    /// Set when Apple refunded or revoked the transaction, Unix milliseconds UTC.
    pub revocation_date_ms: Option<i64>,
    pub environment: StoreEnvironment,
}

impl VerifiedTransaction {
    /// Whether the transaction has the shape the core accepts: one of OpenFrame's two products,
    /// bounded non-empty identifiers (the optional `appTransactionID` too, when present), positive
    /// Apple dates and a revocation date that is not before the purchase. A transaction that fails
    /// this is treated as corrupt and never reaches the policy.
    pub fn is_well_formed(&self) -> bool {
        let id_ok = |s: &str| !s.is_empty() && s.len() <= MAX_ID_BYTES && s.is_ascii();
        (self.product_id == TRIAL_PRODUCT_ID || self.product_id == LIFETIME_PRODUCT_ID)
            && id_ok(&self.transaction_id)
            && id_ok(&self.original_transaction_id)
            && self.app_transaction_id.as_deref().is_none_or(id_ok)
            && self.purchase_date_ms > 0
            && self.original_purchase_date_ms > 0
            && self.signed_date_ms > 0
            && self
                .revocation_date_ms
                .is_none_or(|r| r >= self.purchase_date_ms)
    }

    pub fn is_trial(&self) -> bool {
        self.product_id == TRIAL_PRODUCT_ID
    }

    pub fn is_lifetime(&self) -> bool {
        self.product_id == LIFETIME_PRODUCT_ID
    }

    pub fn is_revoked(&self) -> bool {
        self.revocation_date_ms.is_some()
    }
}

/// What StoreKit's `AppTransaction.shared` returned, already verified by StoreKit.
///
/// It carries no date on purpose: the app's first download date is not the trial start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTransactionInfo {
    /// `None` before macOS 15.4, where StoreKit does not provide it.
    #[serde(default)]
    pub app_transaction_id: Option<String>,
    pub environment: StoreEnvironment,
}

/// `Product.products(for:)` returned both products, which is the signal that the App Store is
/// reachable in this evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreProducts {
    /// StoreKit's `displayPrice` of the lifetime product, already formatted for the user's
    /// storefront and currency. Read at run time, never typed in code.
    pub lifetime_display_price: String,
}

/// What a purchase attempt ended with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseOutcome {
    /// Apple completed the purchase and StoreKit verified the transaction.
    Success(VerifiedTransaction),
    /// The purchase waits for someone else (Ask to Buy) or for a payment step. Nothing is granted.
    Pending,
    /// The user cancelled. Nothing changes.
    Cancelled,
}

/// Why the bridge could not answer. No variant carries user data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeError {
    /// No Apple Account is signed in to the App Store.
    NotSignedIn,
    /// The Mac has no connection to the App Store.
    NoConnection,
    /// The App Store answered with an error or is not available right now.
    StoreUnavailable,
    /// The product id is not known to the App Store (setup error in App Store Connect).
    ProductNotFound,
    /// Purchases are switched off on this Mac (Screen Time or a device policy).
    PurchasesNotAllowed,
    /// The purchase returned a transaction that failed StoreKit's verification. It was dropped.
    Unverified,
    /// Anything else.
    Other,
}

/// The StoreKit 2 and Keychain calls the core needs. The Swift side implements this through the
/// desktop shell.
///
/// Every method must return within a bounded time and must never pass on an unverified
/// transaction (see the module notes).
pub trait StoreKitBridge: Send + Sync {
    /// `Transaction.currentEntitlements`, plus the latest record of each product so a refund shows
    /// up, verified results only.
    fn current_entitlements(&self) -> Result<Vec<VerifiedTransaction>, BridgeError>;

    /// `Product.purchase()` for `product_id`.
    fn purchase(&self, product_id: &str) -> Result<PurchaseOutcome, BridgeError>;

    /// `AppTransaction.shared`, verified.
    fn app_transaction(&self) -> Result<AppTransactionInfo, BridgeError>;

    /// The environment this build talks to, from the verified `AppTransaction`. Production when it
    /// cannot be read, so transactions from another environment are refused.
    fn environment(&self) -> StoreEnvironment;

    /// The environment when it was really read, `None` when [`Self::environment`] is only a guess.
    fn known_environment(&self) -> Option<StoreEnvironment> {
        Some(self.environment())
    }

    /// `Product.products(for:)` for both products. An error, or fewer than two products, means the
    /// App Store is not reachable in this evaluation.
    fn load_products(&self) -> Result<StoreProducts, BridgeError>;

    /// `AppStore.sync()`, used only for "Restore purchase" and "Check again". Answers the time
    /// Apple signed a freshly fetched record, when the store gave one, for the clock guard.
    fn sync(&self) -> Result<Option<i64>, BridgeError> {
        Ok(None)
    }

    /// The guard record text from the Keychain, `None` when there is none yet.
    fn load_guard(&self) -> Result<Option<String>, BridgeError>;

    /// Store the guard record text in the Keychain.
    fn save_guard(&self, record_json: &str) -> Result<(), BridgeError>;

    /// How many unverified results this bridge has dropped since launch (contract rule 1).
    fn unverified_dropped(&self) -> u64 {
        0
    }
}

/// A boxed bridge, so the desktop shell can hand one over without naming its type.
impl<T: StoreKitBridge + ?Sized> StoreKitBridge for Box<T> {
    fn current_entitlements(&self) -> Result<Vec<VerifiedTransaction>, BridgeError> {
        (**self).current_entitlements()
    }
    fn purchase(&self, product_id: &str) -> Result<PurchaseOutcome, BridgeError> {
        (**self).purchase(product_id)
    }
    fn app_transaction(&self) -> Result<AppTransactionInfo, BridgeError> {
        (**self).app_transaction()
    }
    fn environment(&self) -> StoreEnvironment {
        (**self).environment()
    }
    fn known_environment(&self) -> Option<StoreEnvironment> {
        (**self).known_environment()
    }
    fn load_products(&self) -> Result<StoreProducts, BridgeError> {
        (**self).load_products()
    }
    fn sync(&self) -> Result<Option<i64>, BridgeError> {
        (**self).sync()
    }
    fn load_guard(&self) -> Result<Option<String>, BridgeError> {
        (**self).load_guard()
    }
    fn save_guard(&self, record_json: &str) -> Result<(), BridgeError> {
        (**self).save_guard(record_json)
    }
    fn unverified_dropped(&self) -> u64 {
        (**self).unverified_dropped()
    }
}

/// A StoreKit verification result as the shell receives it, before the contract filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawVerification {
    Verified(VerifiedTransaction),
    /// StoreKit could not verify the transaction. Only the product id is kept, for the count.
    Unverified {
        product_id: String,
    },
}

/// The verified transactions after the contract filter, and how many were dropped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerifiedSet {
    pub transactions: Vec<VerifiedTransaction>,
    /// Unverified results dropped.
    pub unverified: u64,
    /// Results StoreKit called verified that still failed [`VerifiedTransaction::is_well_formed`].
    pub malformed: u64,
}

/// Apply contract rule 1: keep verified, well-formed transactions and count the rest.
pub fn keep_verified(results: impl IntoIterator<Item = RawVerification>) -> VerifiedSet {
    let mut set = VerifiedSet::default();
    for r in results {
        match r {
            RawVerification::Verified(tx) if tx.is_well_formed() => set.transactions.push(tx),
            RawVerification::Verified(_) => set.malformed += 1,
            RawVerification::Unverified { .. } => set.unverified += 1,
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx() -> VerifiedTransaction {
        VerifiedTransaction {
            product_id: TRIAL_PRODUCT_ID.to_string(),
            transaction_id: "2000000001".to_string(),
            original_transaction_id: "2000000001".to_string(),
            app_transaction_id: Some("705000000001".to_string()),
            purchase_date_ms: 1_791_244_800_000,
            original_purchase_date_ms: 1_791_244_800_000,
            signed_date_ms: 1_791_244_800_000,
            revocation_date_ms: None,
            environment: StoreEnvironment::Production,
        }
    }

    #[test]
    fn the_store_identity_never_changes_after_the_first_release() {
        assert_eq!(TRIAL_PRODUCT_ID, "app.openframestudio.studio.trial7");
        assert_eq!(LIFETIME_PRODUCT_ID, "app.openframestudio.studio.lifetime");
    }

    #[test]
    fn well_formed_rejects_foreign_products_bad_ids_and_bad_dates() {
        assert!(tx().is_well_formed());
        let mut t = tx();
        t.product_id = "com.other.app.pro".to_string();
        assert!(!t.is_well_formed());
        let mut t = tx();
        t.transaction_id.clear();
        assert!(!t.is_well_formed());
        let mut t = tx();
        t.app_transaction_id = Some("x".repeat(MAX_ID_BYTES + 1));
        assert!(!t.is_well_formed());
        let mut t = tx();
        t.app_transaction_id = Some(String::new());
        assert!(!t.is_well_formed());
        let mut t = tx();
        t.purchase_date_ms = 0;
        assert!(!t.is_well_formed());
        let mut t = tx();
        t.original_purchase_date_ms = 0;
        assert!(!t.is_well_formed());
        let mut t = tx();
        t.signed_date_ms = -1;
        assert!(!t.is_well_formed());
        let mut t = tx();
        t.revocation_date_ms = Some(t.purchase_date_ms - 1);
        assert!(!t.is_well_formed());
    }

    #[test]
    fn a_missing_app_transaction_id_is_well_formed_because_apple_sends_it_from_macos_15_4_only() {
        let mut t = tx();
        t.app_transaction_id = None;
        assert!(t.is_well_formed());
        // The Swift side may leave the field out entirely.
        let mut json = serde_json::to_value(tx()).unwrap();
        json.as_object_mut().unwrap().remove("appTransactionId");
        let parsed: VerifiedTransaction = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.app_transaction_id, None);
        assert!(parsed.is_well_formed());
        let app: AppTransactionInfo =
            serde_json::from_str(r#"{"environment":"production"}"#).unwrap();
        assert_eq!(app.app_transaction_id, None);
    }

    #[test]
    fn a_transaction_without_a_signed_date_does_not_parse() {
        let mut json = serde_json::to_value(tx()).unwrap();
        json.as_object_mut().unwrap().remove("signedDateMs");
        assert!(serde_json::from_value::<VerifiedTransaction>(json).is_err());
    }
}

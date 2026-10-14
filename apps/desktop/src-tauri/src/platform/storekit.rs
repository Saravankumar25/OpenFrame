//! The JSON contract of the Swift StoreKit 2 bridge and its mapping onto the entitlement crate's
//! types. Pure Rust with no operating system code: compiled for macOS, where
//! `macos_storekit.rs` uses it, and for the tests on every system.
//!
//! The bridge only reports what StoreKit says; it makes no entitlement decision and compares no
//! dates. Unverified transactions never reach Rust (the Swift side counts them), and
//! [`convert`] passes every transaction through the crate's `keep_verified` filter again.

// On Windows only the tests use this module.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use serde::Deserialize;

pub use openframe_entitlement::mac::{LIFETIME_PRODUCT_ID, TRIAL_PRODUCT_ID};

/// Largest answer the bridge may return. A larger answer is refused, not parsed.
pub const MAX_JSON_BYTES: usize = 256 * 1024;

/// A StoreKit 2 transaction that StoreKit reported as `.verified`. Unverified transactions never
/// reach Rust; the bridge only counts them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StoreTransaction {
    #[serde(rename = "productID")]
    pub product_id: String,
    pub id: String,
    #[serde(rename = "originalID")]
    pub original_id: String,
    /// Present on macOS 15.4 and later.
    #[serde(rename = "appTransactionID", default)]
    pub app_transaction_id: Option<String>,
    #[serde(rename = "purchaseDateMs")]
    pub purchase_date_ms: i64,
    #[serde(rename = "revocationDateMs", default)]
    pub revocation_date_ms: Option<i64>,
    pub environment: String,
    /// The signed transaction, sent unchanged to the licence server.
    #[serde(rename = "jwsRepresentation")]
    pub jws_representation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entitlements {
    pub transactions: Vec<StoreTransaction>,
    pub unverified_count: u32,
    /// Verified transactions that are not ours to use: another product, or Family Sharing.
    pub skipped_count: u32,
}

/// How a purchase ended. Ask to Buy (deferred) and a cancelled sheet are different outcomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseOutcome {
    Purchased(StoreTransaction),
    /// Waiting for approval (Ask to Buy) or for a payment step. Not an entitlement.
    Pending,
    Cancelled,
    /// StoreKit returned a transaction it could not verify. Not an entitlement.
    Unverified,
    /// A StoreKit result this version does not know.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTransactionInfo {
    pub verified: bool,
    #[serde(rename = "appTransactionID", default)]
    pub app_transaction_id: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub jws_representation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreEnvironment {
    Production,
    Sandbox,
    /// Local StoreKit testing in Xcode. The licence server refuses these transactions.
    Xcode,
}

/// For the price shown on the trial sheet only. Never used for a decision.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPrice {
    pub id: String,
    pub display_price: String,
}

/// A change StoreKit pushed while the app runs, for example a refund that revokes a purchase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreUpdate {
    Transaction(StoreTransaction),
    Unverified,
    /// The App Store storefront changed, which happens when another Apple Account signs in to the
    /// App Store. macOS has no direct "account changed" notice, so this stands in for it.
    StorefrontChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreKitError {
    /// The argument was refused before it reached StoreKit.
    BadArgument,
    /// The bridge returned nothing, or text that is not the expected JSON.
    BadAnswer,
    /// The answer was larger than [`MAX_JSON_BYTES`].
    TooLarge,
    /// StoreKit reported a failure. The code comes from the bridge, for example `timeout`.
    Store(String),
}

/// Parsing of the bridge's JSON answers.
pub(crate) mod parse {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde_json::Value;

    /// Turn the bridge's JSON text into `T`. Every answer has `ok`; a failed one has `error`.
    pub(crate) fn parse_answer<T: DeserializeOwned>(text: &str) -> Result<T, StoreKitError> {
        if text.len() > MAX_JSON_BYTES {
            return Err(StoreKitError::TooLarge);
        }
        let value: Value = serde_json::from_str(text).map_err(|_| StoreKitError::BadAnswer)?;
        match value.get("ok") {
            Some(Value::Bool(true)) => {
                serde_json::from_value(value).map_err(|_| StoreKitError::BadAnswer)
            }
            Some(Value::Bool(false)) => {
                let code = value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("store_error");
                // Codes are short identifiers from the bridge; anything else is not passed on.
                let code = if code.len() <= 64
                    && code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                {
                    code
                } else {
                    "store_error"
                };
                Err(StoreKitError::Store(code.to_string()))
            }
            _ => Err(StoreKitError::BadAnswer),
        }
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PurchaseAnswer {
        outcome: String,
        #[serde(default)]
        transaction: Option<StoreTransaction>,
    }

    pub(crate) fn parse_purchase(text: &str) -> Result<PurchaseOutcome, StoreKitError> {
        let answer: PurchaseAnswer = parse_answer(text)?;
        Ok(match (answer.outcome.as_str(), answer.transaction) {
            ("purchased", Some(t)) => PurchaseOutcome::Purchased(t),
            ("purchased", None) => return Err(StoreKitError::BadAnswer),
            ("pending", _) => PurchaseOutcome::Pending,
            ("cancelled", _) => PurchaseOutcome::Cancelled,
            ("unverified", _) => PurchaseOutcome::Unverified,
            _ => PurchaseOutcome::Unknown,
        })
    }

    #[derive(Deserialize)]
    struct EnvironmentAnswer {
        environment: String,
    }

    pub(crate) fn parse_environment(text: &str) -> Result<StoreEnvironment, StoreKitError> {
        let answer: EnvironmentAnswer = parse_answer(text)?;
        match answer.environment.as_str() {
            "Production" => Ok(StoreEnvironment::Production),
            "Sandbox" => Ok(StoreEnvironment::Sandbox),
            "Xcode" => Ok(StoreEnvironment::Xcode),
            _ => Err(StoreKitError::BadAnswer),
        }
    }

    #[derive(Deserialize)]
    struct ProductsAnswer {
        products: Vec<ProductPrice>,
    }

    pub(crate) fn parse_products(text: &str) -> Result<Vec<ProductPrice>, StoreKitError> {
        Ok(parse_answer::<ProductsAnswer>(text)?.products)
    }

    #[derive(Deserialize)]
    struct UpdateAnswer {
        kind: String,
        #[serde(default)]
        transaction: Option<StoreTransaction>,
    }

    /// Updates carry no `ok` field; they are pushed, not answered.
    pub(crate) fn parse_update(text: &str) -> Result<StoreUpdate, StoreKitError> {
        if text.len() > MAX_JSON_BYTES {
            return Err(StoreKitError::TooLarge);
        }
        let answer: UpdateAnswer =
            serde_json::from_str(text).map_err(|_| StoreKitError::BadAnswer)?;
        match (answer.kind.as_str(), answer.transaction) {
            ("transaction", Some(t)) => Ok(StoreUpdate::Transaction(t)),
            ("unverified", _) => Ok(StoreUpdate::Unverified),
            ("storefront", _) => Ok(StoreUpdate::StorefrontChanged),
            _ => Err(StoreKitError::BadAnswer),
        }
    }

    /// Only the two known products may be bought or priced.
    pub(crate) fn is_known_product(id: &str) -> bool {
        id == TRIAL_PRODUCT_ID || id == LIFETIME_PRODUCT_ID
    }
}

/// Mapping of the bridge's answers onto the entitlement crate's types. Every failure becomes one
/// [`BridgeError`](ent::BridgeError) that carries no user data.
pub(crate) mod convert {
    use super::*;
    use openframe_entitlement::mac as ent;

    /// StoreKit's `Environment.rawValue`.
    pub(crate) fn env_from_name(raw: &str) -> Option<ent::StoreEnvironment> {
        match raw {
            "Production" => Some(ent::StoreEnvironment::Production),
            "Sandbox" => Some(ent::StoreEnvironment::Sandbox),
            "Xcode" => Some(ent::StoreEnvironment::Xcode),
            _ => None,
        }
    }

    pub(crate) fn store_environment(e: StoreEnvironment) -> ent::StoreEnvironment {
        match e {
            StoreEnvironment::Production => ent::StoreEnvironment::Production,
            StoreEnvironment::Sandbox => ent::StoreEnvironment::Sandbox,
            StoreEnvironment::Xcode => ent::StoreEnvironment::Xcode,
        }
    }

    /// One transaction in the crate's shape, or `None` when its environment is unknown. Before
    /// macOS 15.4 StoreKit has no `appTransactionID`, and the field stays empty.
    pub(crate) fn transaction(t: StoreTransaction) -> Option<ent::VerifiedTransaction> {
        let environment = env_from_name(&t.environment)?;
        Some(ent::VerifiedTransaction {
            app_transaction_id: t.app_transaction_id.filter(|id| !id.is_empty()),
            product_id: t.product_id,
            transaction_id: t.id,
            original_transaction_id: t.original_id,
            purchase_date_ms: t.purchase_date_ms,
            revocation_date_ms: t.revocation_date_ms,
            environment,
            signed_jws: t.jws_representation,
        })
    }

    /// `currentEntitlements` after the crate's verification filter, with what was dropped.
    #[derive(Debug, Default, PartialEq, Eq)]
    pub(crate) struct Filtered {
        pub transactions: Vec<ent::VerifiedTransaction>,
        /// Unverified results the Swift side dropped.
        pub unverified: u64,
        /// Verified results that are not well formed or have an unknown environment.
        pub malformed: u64,
        /// Other products or Family Sharing, skipped by the Swift side.
        pub skipped: u64,
    }

    pub(crate) fn entitlements(e: Entitlements) -> Filtered {
        let total = e.transactions.len();
        let raw: Vec<ent::RawVerification> = e
            .transactions
            .into_iter()
            .filter_map(transaction)
            .map(ent::RawVerification::Verified)
            .collect();
        let unknown_environment = (total - raw.len()) as u64;
        let set = ent::keep_verified(raw);
        Filtered {
            transactions: set.transactions,
            unverified: u64::from(e.unverified_count) + set.unverified,
            malformed: set.malformed + unknown_environment,
            skipped: u64::from(e.skipped_count),
        }
    }

    pub(crate) fn purchase(o: PurchaseOutcome) -> Result<ent::PurchaseOutcome, ent::BridgeError> {
        match o {
            PurchaseOutcome::Purchased(t) => transaction(t)
                .filter(ent::VerifiedTransaction::is_well_formed)
                .map(ent::PurchaseOutcome::Success)
                .ok_or(ent::BridgeError::Unverified),
            PurchaseOutcome::Pending => Ok(ent::PurchaseOutcome::Pending),
            PurchaseOutcome::Cancelled => Ok(ent::PurchaseOutcome::Cancelled),
            PurchaseOutcome::Unverified => Err(ent::BridgeError::Unverified),
            PurchaseOutcome::Unknown => Err(ent::BridgeError::Other),
        }
    }

    /// The verified `AppTransaction`. It has no id before macOS 15.4.
    pub(crate) fn app_transaction(
        a: AppTransactionInfo,
    ) -> Result<ent::AppTransactionInfo, ent::BridgeError> {
        if !a.verified {
            return Err(ent::BridgeError::Unverified);
        }
        let environment = a
            .environment
            .as_deref()
            .and_then(env_from_name)
            .ok_or(ent::BridgeError::Other)?;
        let signed_jws = a.jws_representation.ok_or(ent::BridgeError::Other)?;
        Ok(ent::AppTransactionInfo {
            app_transaction_id: a.app_transaction_id.filter(|id| !id.is_empty()),
            environment,
            signed_jws,
        })
    }

    pub(crate) fn product_info(
        products: Vec<ProductPrice>,
        id: &str,
    ) -> Result<ent::ProductInfo, ent::BridgeError> {
        products
            .into_iter()
            .find(|p| p.id == id)
            .map(|p| ent::ProductInfo {
                product_id: p.id,
                display_price: p.display_price,
            })
            .ok_or(ent::BridgeError::ProductNotFound)
    }

    /// The bridge's error codes (see `StoreKitBridge.swift`) as the crate's errors.
    pub(crate) fn bridge_error(e: StoreKitError) -> ent::BridgeError {
        match e {
            StoreKitError::BadArgument | StoreKitError::BadAnswer | StoreKitError::TooLarge => {
                ent::BridgeError::Other
            }
            StoreKitError::Store(code) => match code.as_str() {
                "timeout" => ent::BridgeError::NoConnection,
                "not_found" => ent::BridgeError::ProductNotFound,
                "unverified" => ent::BridgeError::Unverified,
                "store_error" => ent::BridgeError::StoreUnavailable,
                _ => ent::BridgeError::Other,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse::*;
    use super::*;

    const TX: &str = r#"{"productID":"com.openframe.studio.trial7","id":"2","originalID":"1",
        "appTransactionID":"7","purchaseDateMs":1790000000000,"environment":"Sandbox",
        "jwsRepresentation":"a.b.c"}"#;

    #[test]
    fn entitlements_parse_and_unverified_ones_are_only_counted() {
        let text =
            format!(r#"{{"ok":true,"transactions":[{TX}],"unverifiedCount":2,"skippedCount":1}}"#);
        let e: Entitlements = parse_answer(&text).unwrap();
        assert_eq!(e.transactions.len(), 1);
        assert_eq!(e.transactions[0].original_id, "1");
        assert_eq!(e.transactions[0].revocation_date_ms, None);
        assert_eq!(e.unverified_count, 2);
    }

    #[test]
    fn deferred_and_cancelled_purchases_are_different_outcomes() {
        assert_eq!(
            parse_purchase(r#"{"ok":true,"outcome":"pending"}"#),
            Ok(PurchaseOutcome::Pending)
        );
        assert_eq!(
            parse_purchase(r#"{"ok":true,"outcome":"cancelled"}"#),
            Ok(PurchaseOutcome::Cancelled)
        );
        assert_eq!(
            parse_purchase(r#"{"ok":true,"outcome":"unverified"}"#),
            Ok(PurchaseOutcome::Unverified)
        );
        assert_eq!(
            parse_purchase(r#"{"ok":true,"outcome":"purchased"}"#),
            Err(StoreKitError::BadAnswer)
        );
        let bought = format!(r#"{{"ok":true,"outcome":"purchased","transaction":{TX}}}"#);
        assert!(matches!(
            parse_purchase(&bought),
            Ok(PurchaseOutcome::Purchased(_))
        ));
    }

    #[test]
    fn failures_oversized_and_garbage_answers_map_to_plain_errors() {
        assert_eq!(
            parse_answer::<Entitlements>(r#"{"ok":false,"error":"timeout"}"#),
            Err(StoreKitError::Store("timeout".into()))
        );
        assert_eq!(
            parse_answer::<Entitlements>(r#"{"ok":false,"error":"Not\nA code"}"#),
            Err(StoreKitError::Store("store_error".into()))
        );
        assert_eq!(
            parse_answer::<Entitlements>("not json"),
            Err(StoreKitError::BadAnswer)
        );
        assert_eq!(
            parse_answer::<Entitlements>(r#"{"transactions":[]}"#),
            Err(StoreKitError::BadAnswer)
        );
        let huge = "x".repeat(MAX_JSON_BYTES + 1);
        assert_eq!(
            parse_answer::<Entitlements>(&huge),
            Err(StoreKitError::TooLarge)
        );
        assert_eq!(
            parse_environment(r#"{"ok":true,"environment":"Other"}"#),
            Err(StoreKitError::BadAnswer)
        );
        assert_eq!(
            parse_environment(r#"{"ok":true,"environment":"Sandbox"}"#),
            Ok(StoreEnvironment::Sandbox)
        );
    }

    #[test]
    fn revocation_updates_reach_rust_with_their_date() {
        let text = TX.replace(
            r#""environment""#,
            r#""revocationDateMs":1790000500000,"environment""#,
        );
        let update =
            parse_update(&format!(r#"{{"kind":"transaction","transaction":{text}}}"#)).unwrap();
        match update {
            StoreUpdate::Transaction(t) => {
                assert_eq!(t.revocation_date_ms, Some(1_790_000_500_000))
            }
            StoreUpdate::Unverified | StoreUpdate::StorefrontChanged => {
                panic!("expected a transaction")
            }
        }
        assert_eq!(
            parse_update(r#"{"kind":"unverified"}"#),
            Ok(StoreUpdate::Unverified)
        );
        assert_eq!(
            parse_update(r#"{"kind":"transaction"}"#),
            Err(StoreKitError::BadAnswer)
        );
    }

    #[test]
    fn only_the_two_products_are_known() {
        assert!(is_known_product(TRIAL_PRODUCT_ID));
        assert!(is_known_product(LIFETIME_PRODUCT_ID));
        assert!(!is_known_product("com.openframe.studio.other"));
    }

    // ---------------------------------------------------------------- mapping onto the crate

    use openframe_entitlement::mac as ent;

    fn store_tx(product: &str, app_id: Option<&str>) -> StoreTransaction {
        StoreTransaction {
            product_id: product.to_string(),
            id: "2000000002".to_string(),
            original_id: "2000000001".to_string(),
            app_transaction_id: app_id.map(str::to_string),
            purchase_date_ms: 1_790_000_000_000,
            revocation_date_ms: None,
            environment: "Sandbox".to_string(),
            jws_representation: "aGVhZA.Ym9keQ.c2ln".to_string(),
        }
    }

    #[test]
    fn transactions_map_onto_the_crate_and_pass_its_filter() {
        let mut odd_env = store_tx(TRIAL_PRODUCT_ID, Some("705"));
        odd_env.environment = "Other".to_string();
        let mut bad_jws = store_tx(LIFETIME_PRODUCT_ID, Some("705"));
        bad_jws.jws_representation = "not a jws".to_string();
        let filtered = convert::entitlements(Entitlements {
            transactions: vec![store_tx(TRIAL_PRODUCT_ID, Some("705")), odd_env, bad_jws],
            unverified_count: 2,
            skipped_count: 1,
        });
        assert_eq!(filtered.transactions.len(), 1);
        let t = &filtered.transactions[0];
        assert_eq!(t.transaction_id, "2000000002");
        assert_eq!(t.original_transaction_id, "2000000001");
        assert_eq!(t.app_transaction_id.as_deref(), Some("705"));
        assert_eq!(t.environment, ent::StoreEnvironment::Sandbox);
        assert!(t.is_trial() && t.is_well_formed());
        assert_eq!(
            (filtered.unverified, filtered.malformed, filtered.skipped),
            (2, 2, 1)
        );
    }

    #[test]
    fn before_macos_15_4_the_app_transaction_id_is_absent_and_that_is_well_formed() {
        for missing in [None, Some("")] {
            let t = convert::transaction(store_tx(TRIAL_PRODUCT_ID, missing)).unwrap();
            assert_eq!(t.app_transaction_id, None);
            assert!(t.is_well_formed());
        }
    }

    #[test]
    fn purchase_outcomes_and_errors_map_without_granting_anything_extra() {
        assert!(matches!(
            convert::purchase(PurchaseOutcome::Purchased(store_tx(LIFETIME_PRODUCT_ID, None))),
            Ok(ent::PurchaseOutcome::Success(t)) if t.is_lifetime()
        ));
        let mut foreign = store_tx(LIFETIME_PRODUCT_ID, None);
        foreign.product_id = "com.other.app".to_string();
        assert_eq!(
            convert::purchase(PurchaseOutcome::Purchased(foreign)),
            Err(ent::BridgeError::Unverified)
        );
        assert_eq!(
            convert::purchase(PurchaseOutcome::Pending),
            Ok(ent::PurchaseOutcome::Pending)
        );
        assert_eq!(
            convert::purchase(PurchaseOutcome::Cancelled),
            Ok(ent::PurchaseOutcome::Cancelled)
        );
        assert_eq!(
            convert::purchase(PurchaseOutcome::Unverified),
            Err(ent::BridgeError::Unverified)
        );
        assert_eq!(
            convert::purchase(PurchaseOutcome::Unknown),
            Err(ent::BridgeError::Other)
        );
        assert_eq!(
            convert::bridge_error(StoreKitError::Store("timeout".into())),
            ent::BridgeError::NoConnection
        );
        assert_eq!(
            convert::bridge_error(StoreKitError::Store("main_thread".into())),
            ent::BridgeError::Other
        );
        assert_eq!(
            convert::bridge_error(StoreKitError::TooLarge),
            ent::BridgeError::Other
        );
    }

    #[test]
    fn app_transaction_needs_verification_an_environment_and_a_signed_form() {
        let ok = AppTransactionInfo {
            verified: true,
            app_transaction_id: None,
            environment: Some("Production".to_string()),
            jws_representation: Some("a.b.c".to_string()),
        };
        let mapped = convert::app_transaction(ok.clone()).unwrap();
        assert_eq!(mapped.environment, ent::StoreEnvironment::Production);
        assert_eq!(mapped.app_transaction_id, None);
        assert_eq!(
            convert::app_transaction(AppTransactionInfo {
                verified: false,
                ..ok.clone()
            }),
            Err(ent::BridgeError::Unverified)
        );
        assert_eq!(
            convert::app_transaction(AppTransactionInfo {
                jws_representation: None,
                ..ok
            }),
            Err(ent::BridgeError::Other)
        );
    }

    #[test]
    fn storefront_updates_and_unknown_products_are_handled() {
        assert_eq!(
            parse_update(r#"{"kind":"storefront"}"#),
            Ok(StoreUpdate::StorefrontChanged)
        );
        assert_eq!(
            convert::product_info(
                vec![ProductPrice {
                    id: LIFETIME_PRODUCT_ID.to_string(),
                    display_price: "$49.99".to_string()
                }],
                TRIAL_PRODUCT_ID
            ),
            Err(ent::BridgeError::ProductNotFound)
        );
    }
}

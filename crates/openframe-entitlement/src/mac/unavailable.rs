//! A stand-in for a build without an App Store connection: a Windows build, or a Mac shell that does
//! not supply the StoreKit bridge.
//!
//! Every call answers with the module's "not available" error, so the adapter derives
//! [`super::MacState::NeedsOnlineCheck`]: opening and exporting keep working, editing is locked,
//! and neither the trial nor the lifetime unlock is ever granted this way.

use super::bridge::{
    AppTransactionInfo, BridgeError, PurchaseOutcome, StoreEnvironment, StoreKitBridge,
    StoreProducts, VerifiedTransaction,
};

/// A StoreKit bridge that is not there. Every call fails with [`BridgeError::StoreUnavailable`].
#[derive(Debug, Default, Clone, Copy)]
pub struct UnavailableBridge;

impl StoreKitBridge for UnavailableBridge {
    fn current_entitlements(&self) -> Result<Vec<VerifiedTransaction>, BridgeError> {
        Err(BridgeError::StoreUnavailable)
    }

    fn purchase(&self, _product_id: &str) -> Result<PurchaseOutcome, BridgeError> {
        Err(BridgeError::StoreUnavailable)
    }

    fn app_transaction(&self) -> Result<AppTransactionInfo, BridgeError> {
        Err(BridgeError::StoreUnavailable)
    }

    fn environment(&self) -> StoreEnvironment {
        StoreEnvironment::Production
    }

    fn known_environment(&self) -> Option<StoreEnvironment> {
        None
    }

    fn load_products(&self) -> Result<StoreProducts, BridgeError> {
        Err(BridgeError::StoreUnavailable)
    }

    fn sync(&self) -> Result<Option<i64>, BridgeError> {
        Err(BridgeError::StoreUnavailable)
    }

    fn load_guard(&self) -> Result<Option<String>, BridgeError> {
        Err(BridgeError::StoreUnavailable)
    }

    fn save_guard(&self, _record_json: &str) -> Result<(), BridgeError> {
        Err(BridgeError::StoreUnavailable)
    }
}

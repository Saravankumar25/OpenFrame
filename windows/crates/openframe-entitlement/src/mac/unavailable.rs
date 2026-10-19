//! Stand-ins for a build without an App Store connection: a Windows build, or a Mac shell that
//! does not supply the StoreKit bridge and the licensing server client yet.
//!
//! Every call answers with the module's "not available" error, so the adapter derives
//! [`super::MacState::NeedsOnlineCheck`]: opening and exporting keep working, editing is locked,
//! and neither the trial nor the lifetime unlock is ever granted this way.

use super::adapter::NonceSource;
use super::bridge::{
    AppTransactionInfo, BridgeError, ProductInfo, PurchaseOutcome, StoreEnvironment,
    StoreKitBridge, VerifiedTransaction,
};
use super::server::{LicenceServer, NONCE_BYTES, ServerError, ValidateRequest, ValidateResponse};

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

    /// Production, so a cache written by a real build is never mistaken for this one's.
    fn environment(&self) -> StoreEnvironment {
        StoreEnvironment::Production
    }

    fn product_info(&self, _product_id: &str) -> Result<ProductInfo, BridgeError> {
        Err(BridgeError::StoreUnavailable)
    }

    fn sync(&self) -> Result<(), BridgeError> {
        Err(BridgeError::StoreUnavailable)
    }
}

/// A licensing server that is not there. Every call fails with [`ServerError::Unavailable`].
#[derive(Debug, Default, Clone, Copy)]
pub struct UnavailableServer;

impl LicenceServer for UnavailableServer {
    fn validate(&self, _req: ValidateRequest) -> Result<ValidateResponse, ServerError> {
        Err(ServerError::Unavailable)
    }
}

/// A nonce source with no random bytes, so no request is ever sent.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoNonces;

impl NonceSource for NoNonces {
    fn nonce(&self) -> Option<[u8; NONCE_BYTES]> {
        None
    }
}

//! One adapter per release channel. Both store channels decide the whole state themselves
//! ([`ChannelAdapter::channel_state`]): the store owns the 7-day trial and the local trial record
//! is not used (ADR-0023). The installer is an internal build that is never sold, so the core
//! asks no adapter for it. Adapters never see project data.
//!
//! State of each adapter:
//!
//! - [`MicrosoftStoreAdapter`]: the Store licence (`GetAppLicenseAsync`: IsActive, IsTrial,
//!   TrialTimeRemaining) and the Store purchase dialog (`RequestPurchaseAsync`) behind
//!   [`StoreLicenceSource`]. The state mapping and the purchase path are built and tested with a
//!   fake source. The real calls need the Partner Center product identity (owner task); until the
//!   shell supplies them, [`for_channel`] builds [`MicrosoftStoreAdapter::unavailable`] and the
//!   state is NeedsOnlineCheck.
//! - [`crate::mac::MacAppStoreAdapter`]: StoreKit 2 trial and lifetime unlock, decided by the
//!   licensing server's signed verdict. Built and tested with a fake bridge and a fake server.
//!   Until the desktop shell supplies the Swift bridge and the HTTPS client ([`MacInputs`]),
//!   [`for_channel`] builds it over [`crate::mac::UnavailableBridge`], so the state is
//!   NeedsOnlineCheck: opening and exporting work, editing is locked, nothing is granted.

use std::path::Path;

use crate::mac::{MacAppStoreAdapter, MacInputs};
use crate::{Channel, EntitlementState, PurchaseStatus};

/// What a channel must be able to answer.
pub trait ChannelAdapter: Send + Sync {
    fn channel(&self) -> Channel;

    /// The channel's purchase answer at `now_ms`. Must return quickly and must never panic; use
    /// [`PurchaseStatus::Unknown`] for anything the channel cannot answer right now.
    fn purchase_status(&self, now_ms: i64) -> PurchaseStatus;

    /// Ask the channel to restore a purchase (a Store sign-in, for example). Returns the
    /// new status. The default re-runs [`Self::purchase_status`].
    fn restore_purchase(&self, now_ms: i64) -> PurchaseStatus {
        self.purchase_status(now_ms)
    }

    /// Plain words for why the adapter cannot answer, shown in Settings, or `None` when it works.
    fn limitation(&self) -> Option<&'static str> {
        None
    }

    /// The whole state, for a channel whose store decides the trial itself (the Mac App Store and
    /// the Microsoft Store, ADR-0023). `None`, the default, means the channel decides nothing
    /// and the state is Unknown.
    fn channel_state(&self) -> Option<EntitlementState> {
        None
    }

    /// Like [`Self::channel_state`], after asking the channel to restore a purchase.
    fn restore_channel_state(&self) -> Option<EntitlementState> {
        None
    }

    /// Buy the full version through the store's own purchase window (the Mac App Store's payment
    /// sheet, the Microsoft Store's purchase dialog), then return the new state. The store shows
    /// the price; OpenFrame never does (ADR-0023). `None`, the default, means the channel cannot
    /// sell from this build (the internal installer build, or a store build whose store
    /// connection is not wired). Blocks while the store answers, so it runs off the main thread.
    /// No payment detail ever passes through OpenFrame.
    fn buy_full_version(&self) -> Option<Result<EntitlementState, PurchaseFailure>> {
        None
    }

    /// Start the free trial for a channel where the user starts it (the Mac App Store's
    /// zero-price trial product). `None`, the default, means the trial starts by itself at first
    /// launch.
    fn start_trial(&self) -> Option<Result<EntitlementState, PurchaseFailure>> {
        None
    }
}

/// Why a purchase or trial start through the channel did not finish, in plain words for the
/// screen. Carries no store identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PurchaseFailure {
    pub message: &'static str,
}

/// What the Microsoft Store says about this app's licence: `StoreContext.GetAppLicenseAsync`
/// (Windows.Services.Store), which answers from the cached offline licence when the Store cannot
/// be reached. The Store owns the 7-day trial (Partner Center); OpenFrame keeps no trial timer of
/// its own for this channel (owner decision 2026-10-07, ADR-0023).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreAppLicence {
    /// `StoreAppLicense.IsActive`: the trial is running or the app is bought.
    pub is_active: bool,
    /// `StoreAppLicense.IsTrial`.
    pub is_trial: bool,
    /// `StoreAppLicense.TrialTimeRemaining` in whole seconds. Read only while `is_trial`.
    pub trial_seconds_left: u64,
}

/// Why no licence could be read: the Store cannot be reached and there is no cached licence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreLicenceError {
    Unreachable,
}

/// The answer of `StoreContext.RequestPurchaseAsync` (`StorePurchaseStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorePurchaseStatus {
    Succeeded,
    AlreadyPurchased,
    /// The user closed the Store's purchase dialog.
    NotPurchased,
    NetworkError,
    ServerError,
}

/// The Microsoft Store calls, behind a seam so the state mapping and the lock are tested with a
/// fake. The real implementation lives in the Windows shell and needs the Partner Center product
/// identity, which the owner has not given yet. Calls block while the Store answers, so the
/// core makes them off the main thread.
pub trait StoreLicenceSource: Send + Sync {
    /// `StoreContext.GetAppLicenseAsync`. Called at launch, on "Restore purchase" and when the
    /// shell sees `StoreContext.OfflineLicensesChanged`.
    fn app_licence(&self) -> Result<StoreAppLicence, StoreLicenceError>;
    /// `StoreContext.RequestPurchaseAsync` for this app, with the app window's handle so the
    /// Store's dialog opens over it.
    fn request_purchase(&self) -> StorePurchaseStatus;
}

/// The source for a build without the Partner Center identity: nothing can be read or bought.
#[derive(Debug, Default)]
pub struct UnavailableStoreLicence;

impl StoreLicenceSource for UnavailableStoreLicence {
    fn app_licence(&self) -> Result<StoreAppLicence, StoreLicenceError> {
        Err(StoreLicenceError::Unreachable)
    }
    fn request_purchase(&self) -> StorePurchaseStatus {
        StorePurchaseStatus::NetworkError
    }
}

const DAY_SECONDS: u64 = 24 * 60 * 60;

/// The state for a Store licence answer (ADR-0023):
///
/// - active and not a trial: Purchased;
/// - active trial: TrialActive with `TrialTimeRemaining` in whole days, rounded up, 1 to 7;
/// - not active (the trial is over and nothing was bought): TrialEnded, which locks the app;
/// - no answer and no cached licence: NeedsOnlineCheck. Editing locks, opening and exporting
///   stay (the rule of ADR-0018 decision 6), so a failed check never locks anyone out. A user
///   who bought or is in a running trial has a cached licence, which the Store answers from.
pub fn store_licence_state(answer: Result<StoreAppLicence, StoreLicenceError>) -> EntitlementState {
    match answer {
        Ok(l) if l.is_active && !l.is_trial => EntitlementState::Purchased,
        Ok(l) if l.is_active => {
            let days = l.trial_seconds_left.div_ceil(DAY_SECONDS).clamp(1, 7);
            EntitlementState::TrialActive {
                days_left: u32::try_from(days).unwrap_or(1),
            }
        }
        Ok(_) => EntitlementState::TrialEnded,
        Err(StoreLicenceError::Unreachable) => EntitlementState::NeedsOnlineCheck,
    }
}

/// Microsoft Store channel: the Store's licence and purchase dialog through a
/// [`StoreLicenceSource`].
pub struct MicrosoftStoreAdapter {
    source: Box<dyn StoreLicenceSource>,
    connected: bool,
}

impl MicrosoftStoreAdapter {
    /// The adapter over the shell's Store calls.
    pub fn new(source: Box<dyn StoreLicenceSource>) -> Self {
        Self {
            source,
            connected: true,
        }
    }

    /// A build without the Store calls wired: NeedsOnlineCheck and nothing can be bought. A gap
    /// in the build, not a state a user of a finished Store build sees.
    pub fn unavailable() -> Self {
        Self {
            source: Box::new(UnavailableStoreLicence),
            connected: false,
        }
    }

    fn current(&self) -> EntitlementState {
        store_licence_state(self.source.app_licence())
    }
}

impl std::fmt::Debug for MicrosoftStoreAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicrosoftStoreAdapter")
            .field("connected", &self.connected)
            .finish()
    }
}

/// Plain words when the Microsoft Store connection is missing from the build.
pub const MICROSOFT_STORE_NOT_CONNECTED: &str =
    "The Microsoft Store connection is not available in this build.";

impl ChannelAdapter for MicrosoftStoreAdapter {
    fn channel(&self) -> Channel {
        Channel::MicrosoftStore
    }
    fn purchase_status(&self, _now_ms: i64) -> PurchaseStatus {
        match self.current() {
            EntitlementState::Purchased => PurchaseStatus::Purchased,
            EntitlementState::TrialActive { .. } | EntitlementState::TrialEnded => {
                PurchaseStatus::NotPurchased
            }
            _ => PurchaseStatus::Unknown,
        }
    }
    fn limitation(&self) -> Option<&'static str> {
        (!self.connected).then_some(MICROSOFT_STORE_NOT_CONNECTED)
    }
    fn channel_state(&self) -> Option<EntitlementState> {
        Some(self.current())
    }
    fn restore_channel_state(&self) -> Option<EntitlementState> {
        Some(self.current())
    }
    fn buy_full_version(&self) -> Option<Result<EntitlementState, PurchaseFailure>> {
        if !self.connected {
            return None;
        }
        Some(match self.source.request_purchase() {
            StorePurchaseStatus::Succeeded
            | StorePurchaseStatus::AlreadyPurchased
            | StorePurchaseStatus::NotPurchased => Ok(self.current()),
            StorePurchaseStatus::NetworkError => Err(PurchaseFailure {
                message: "OpenFrame could not reach the Microsoft Store. Connect to the internet, then try again.",
            }),
            StorePurchaseStatus::ServerError => Err(PurchaseFailure {
                message: "The Microsoft Store could not finish the purchase. Try again later.",
            }),
        })
    }
}

/// The adapter for a channel, with the production inputs the desktop shell supplies.
///
/// `mac` carries the StoreKit bridge, the licensing server client, the nonce source and the
/// monotonic clock for the Mac App Store channel. `None` (a Windows build, or a Mac shell without
/// the StoreKit bridge yet) builds the Mac adapter over [`crate::mac::UnavailableBridge`] and
/// [`crate::mac::UnavailableServer`]: the state is NeedsOnlineCheck and neither the trial nor the
/// lifetime unlock is ever granted that way. `store` carries the Microsoft Store calls; `None`
/// builds [`MicrosoftStoreAdapter::unavailable`]. Both `None` cases are gaps in the build, not
/// states a user of a finished store build sees.
///
/// `None` for the internal team builds ([`Channel::DirectInstaller`]): they are never sold, so
/// there is no channel to ask (ADR-0023, ADR-0024).
pub fn for_channel(
    channel: Channel,
    app_data_dir: &Path,
    mac: Option<MacInputs>,
    store: Option<Box<dyn StoreLicenceSource>>,
) -> Option<Box<dyn ChannelAdapter>> {
    match channel {
        Channel::MicrosoftStore => Some(Box::new(match store {
            Some(source) => MicrosoftStoreAdapter::new(source),
            None => MicrosoftStoreAdapter::unavailable(),
        })),
        Channel::MacAppStore => Some(match mac {
            Some(inputs) => Box::new(MacAppStoreAdapter::from_inputs(inputs, app_data_dir)),
            None => Box::new(MacAppStoreAdapter::unavailable(app_data_dir)),
        }),
        Channel::DirectInstaller => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    /// A Store whose licence answer and purchase result the test sets.
    struct FakeStore {
        licence: Mutex<Result<StoreAppLicence, StoreLicenceError>>,
        purchase: Mutex<StorePurchaseStatus>,
        after_purchase: Mutex<Option<StoreAppLicence>>,
    }

    impl FakeStore {
        fn new(licence: Result<StoreAppLicence, StoreLicenceError>) -> Self {
            Self {
                licence: Mutex::new(licence),
                purchase: Mutex::new(StorePurchaseStatus::NotPurchased),
                after_purchase: Mutex::new(None),
            }
        }
    }

    impl StoreLicenceSource for std::sync::Arc<FakeStore> {
        fn app_licence(&self) -> Result<StoreAppLicence, StoreLicenceError> {
            *self.licence.lock().unwrap()
        }
        fn request_purchase(&self) -> StorePurchaseStatus {
            let status = *self.purchase.lock().unwrap();
            if let Some(next) = self.after_purchase.lock().unwrap().take() {
                *self.licence.lock().unwrap() = Ok(next);
            }
            status
        }
    }

    const fn lic(is_active: bool, is_trial: bool, trial_seconds_left: u64) -> StoreAppLicence {
        StoreAppLicence {
            is_active,
            is_trial,
            trial_seconds_left,
        }
    }

    #[test]
    fn store_licence_answers_map_to_every_state() {
        let day = DAY_SECONDS;
        assert_eq!(
            store_licence_state(Ok(lic(true, false, 0))),
            EntitlementState::Purchased
        );
        assert_eq!(
            store_licence_state(Ok(lic(true, false, 5 * day))),
            EntitlementState::Purchased,
            "trial time is ignored once bought"
        );
        for (secs, days) in [
            (7 * day, 7),
            (7 * day - 1, 7),
            (6 * day + 1, 7),
            (6 * day, 6),
            (2 * day + 30, 3),
            (day, 1),
            (1, 1),
            (0, 1),
            (400 * day, 7),
        ] {
            assert_eq!(
                store_licence_state(Ok(lic(true, true, secs))),
                EntitlementState::TrialActive { days_left: days },
                "{secs} seconds left"
            );
        }
        assert_eq!(
            store_licence_state(Ok(lic(false, true, 0))),
            EntitlementState::TrialEnded
        );
        assert_eq!(
            store_licence_state(Ok(lic(false, false, 0))),
            EntitlementState::TrialEnded
        );
        assert_eq!(
            store_licence_state(Err(StoreLicenceError::Unreachable)),
            EntitlementState::NeedsOnlineCheck
        );
    }

    #[test]
    fn microsoft_store_reads_the_store_licence_and_buys_through_the_store_dialog() {
        let store = std::sync::Arc::new(FakeStore::new(Ok(lic(true, true, 3 * DAY_SECONDS))));
        let a = for_channel(
            Channel::MicrosoftStore,
            Path::new("unused"),
            None,
            Some(Box::new(store.clone())),
        )
        .unwrap();
        assert_eq!(a.channel(), Channel::MicrosoftStore);
        assert_eq!(a.limitation(), None);
        assert_eq!(
            a.channel_state(),
            Some(EntitlementState::TrialActive { days_left: 3 })
        );
        assert_eq!(a.purchase_status(0), PurchaseStatus::NotPurchased);
        assert_eq!(a.start_trial(), None, "the Store starts the trial itself");

        // The trial ends while the app is open; the shell checks again on OfflineLicensesChanged.
        *store.licence.lock().unwrap() = Ok(lic(false, true, 0));
        assert_eq!(
            a.restore_channel_state(),
            Some(EntitlementState::TrialEnded)
        );

        // The user closes the Store's dialog: nothing changes and no failure is shown.
        assert_eq!(a.buy_full_version(), Some(Ok(EntitlementState::TrialEnded)));
        // The Store cannot be reached, then the purchase succeeds.
        *store.purchase.lock().unwrap() = StorePurchaseStatus::NetworkError;
        let failed = a.buy_full_version().unwrap().unwrap_err();
        assert!(
            failed
                .message
                .contains("could not reach the Microsoft Store"),
            "{failed:?}"
        );
        *store.purchase.lock().unwrap() = StorePurchaseStatus::ServerError;
        assert!(a.buy_full_version().unwrap().is_err());
        *store.purchase.lock().unwrap() = StorePurchaseStatus::Succeeded;
        *store.after_purchase.lock().unwrap() = Some(lic(true, false, 0));
        assert_eq!(a.buy_full_version(), Some(Ok(EntitlementState::Purchased)));
        assert_eq!(a.purchase_status(0), PurchaseStatus::Purchased);
        // Bought before on another computer: the Store says so and the licence is active.
        *store.purchase.lock().unwrap() = StorePurchaseStatus::AlreadyPurchased;
        assert_eq!(a.buy_full_version(), Some(Ok(EntitlementState::Purchased)));

        // Offline with no cached licence: a check is needed, which never locks opening.
        *store.licence.lock().unwrap() = Err(StoreLicenceError::Unreachable);
        assert_eq!(a.channel_state(), Some(EntitlementState::NeedsOnlineCheck));
        assert_eq!(a.purchase_status(0), PurchaseStatus::Unknown);
    }

    #[test]
    fn microsoft_store_without_the_store_calls_needs_a_check_and_cannot_sell() {
        let a = for_channel(Channel::MicrosoftStore, Path::new("unused"), None, None).unwrap();
        assert_eq!(a.channel_state(), Some(EntitlementState::NeedsOnlineCheck));
        assert_eq!(
            a.restore_channel_state(),
            Some(EntitlementState::NeedsOnlineCheck)
        );
        assert_eq!(a.buy_full_version(), None);
        assert_eq!(a.limitation(), Some(MICROSOFT_STORE_NOT_CONNECTED));
        // The fallback source is never asked to sell.
        assert_eq!(
            UnavailableStoreLicence.request_purchase(),
            StorePurchaseStatus::NetworkError
        );
    }

    #[test]
    fn mac_channel_without_inputs_needs_an_online_check_and_grants_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let a = for_channel(Channel::MacAppStore, dir.path(), None, None).unwrap();
        assert_eq!(a.channel(), Channel::MacAppStore);
        assert_eq!(a.channel_state(), Some(EntitlementState::NeedsOnlineCheck));
        assert_eq!(
            a.restore_channel_state(),
            Some(EntitlementState::NeedsOnlineCheck)
        );
        assert_eq!(a.purchase_status(0), PurchaseStatus::Unknown);
        assert_eq!(a.restore_purchase(0), PurchaseStatus::Unknown);
        assert_eq!(
            a.limitation(),
            Some("The App Store connection is not available in this build.")
        );
        assert!(
            !dir.path().join("entitlement").exists(),
            "nothing learned, nothing cached"
        );
    }

    #[test]
    fn the_internal_builds_have_no_channel_to_ask() {
        let dir = tempfile::tempdir().unwrap();
        assert!(for_channel(Channel::DirectInstaller, dir.path(), None, None).is_none());
        assert!(!dir.path().join("entitlement").exists(), "nothing written");
    }
}

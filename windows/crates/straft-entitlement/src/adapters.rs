//! One adapter per release channel. Both store channels decide the whole state themselves
//! ([`ChannelAdapter::channel_state`]): the store owns the 7-day trial and the local trial record
//! is not used (ADR-0023). The installer is an internal build that is never sold, so the core
//! asks no adapter for it. Adapters never see project data.
//!
//! State of each adapter:
//!
//! - [`MicrosoftStoreAdapter`]: the Store licence (`GetAppLicenseAsync`: IsActive, IsTrial,
//!   TrialTimeRemaining) and the Store purchase dialog (`RequestPurchaseAsync`) behind
//!   [`StoreLicenceSource`]. The state mapping with memory ([`read_store`], ADR-0039 item 5) and
//!   the purchase path are built and tested with a fake source. Until the shell supplies the
//!   source, [`for_channel`] builds [`MicrosoftStoreAdapter::unavailable`] and the state is
//!   NeedsOnlineCheck.
//! - [`crate::mac::MacAppStoreAdapter`]: StoreKit 2 trial and lifetime unlock, decided by the
//!   licensing server's signed verdict. Built and tested with a fake bridge and a fake server.
//!   Until the desktop shell supplies the Swift bridge and the HTTPS client ([`MacInputs`]),
//!   [`for_channel`] builds it over [`crate::mac::UnavailableBridge`], so the state is
//!   NeedsOnlineCheck: opening and exporting work, editing is locked, nothing is granted.

use std::path::Path;
use std::time::Instant;

use parking_lot::Mutex;

use crate::mac::{MacAppStoreAdapter, MacInputs};
use crate::notices::{LastState, NoticeRecord};
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
    /// the price; Straft never does (ADR-0023). `None`, the default, means the channel cannot
    /// sell from this build (the internal installer build, or a store build whose store
    /// connection is not wired). Blocks while the store answers, so it runs off the main thread.
    /// No payment detail ever passes through Straft.
    fn buy_full_version(&self) -> Option<Result<EntitlementState, PurchaseFailure>> {
        None
    }

    /// Start the free trial for a channel where the user starts it (the Mac App Store's
    /// zero-price trial product). `None`, the default, means the trial starts by itself at first
    /// launch.
    fn start_trial(&self) -> Option<Result<EntitlementState, PurchaseFailure>> {
        None
    }

    /// When the running trial ends, in milliseconds since 1970, from the last read: now plus the
    /// time the store reported. `None`, the default, when there is no running trial or the channel
    /// does not report one.
    fn trial_ends_at_ms(&self) -> Option<i64> {
        None
    }

    /// Hand the adapter what the app remembered from earlier runs. Called once, before the first
    /// read. The default ignores it.
    fn seed_memory(&self, _memory: StoreMemory) {}

    /// What the adapter remembers now, for the app to save. `None`, the default, when the
    /// channel keeps no memory.
    fn memory(&self) -> Option<StoreMemory> {
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
/// be reached. The Store owns the 7-day trial (Partner Center); Straft keeps no trial timer of
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

/// What the app remembers of the last good Store answer (ADR-0039 item 5). The first three
/// fields are saved in `notices.json` (see [`StoreMemory::from_record`]); `read_at_mono` lives
/// only in this run and is the better clock inside it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StoreMemory {
    pub last_state: Option<LastState>,
    /// Trial seconds the Store reported at that answer.
    pub trial_seconds: Option<u64>,
    /// Wall-clock moment of that answer, milliseconds since 1970.
    pub read_at_ms: Option<i64>,
    /// Monotonic moment of that answer. Not saved; `None` after a restart.
    pub read_at_mono: Option<Instant>,
}

impl StoreMemory {
    /// The memory a saved record holds. The monotonic moment is unknown after a restart.
    pub fn from_record(record: &NoticeRecord) -> Self {
        Self {
            last_state: record.last_state,
            trial_seconds: record.last_trial_seconds,
            read_at_ms: record.last_trial_read_at_ms,
            read_at_mono: None,
        }
    }

    /// Copy the saved part into `record`. True when the record changed.
    pub fn write_to(&self, record: &mut NoticeRecord) -> bool {
        let before = (
            record.last_state,
            record.last_trial_seconds,
            record.last_trial_read_at_ms,
        );
        record.last_state = self.last_state;
        record.last_trial_seconds = self.trial_seconds;
        record.last_trial_read_at_ms = self.read_at_ms;
        before != (self.last_state, self.trial_seconds, self.read_at_ms)
    }
}

/// One Store read after the table of ADR-0039 item 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreReading {
    pub state: EntitlementState,
    /// When the trial ends, from this read: now plus the remaining time. `None` outside a trial.
    pub trial_ends_at_ms: Option<i64>,
    /// The memory to keep after this read.
    pub memory: StoreMemory,
}

fn trial_days(seconds: u64) -> EntitlementState {
    let days = seconds.div_ceil(DAY_SECONDS).clamp(1, 7);
    EntitlementState::TrialActive {
        days_left: u32::try_from(days).unwrap_or(1),
    }
}

fn plus_seconds(now_ms: i64, seconds: u64) -> i64 {
    now_ms.saturating_add(i64::try_from(seconds.saturating_mul(1000)).unwrap_or(i64::MAX))
}

/// The state for a Store licence answer, with memory (ADR-0039 item 5, package file 03 W3):
///
/// | Store says | Last known | Result |
/// |---|---|---|
/// | active, not a trial | any | Purchased |
/// | active, a trial | any | TrialActive, days rounded up, 1 to 7 |
/// | not active | trial, ended or none | TrialEnded (remembered as ended) |
/// | not active | purchased or revoked | Revoked (remembered as revoked) |
/// | failed | purchased | Purchased |
/// | failed | ended | TrialEnded |
/// | failed | revoked | Revoked |
/// | failed | trial | TrialActive from the last good time left minus the time since, never more |
/// | failed | trial, time ran out | NeedsOnlineCheck |
/// | failed | none | NeedsOnlineCheck |
///
/// A lock the Store confirmed holds through failed reads, also after a restart (it is saved), and
/// only a good Store answer lifts it: any active answer overwrites the memory.
///
/// The time since is the monotonic clock inside one run and the wall clock (floored at zero, so a
/// clock turned back gives no extra time) after a restart. NeedsOnlineCheck never locks: opening
/// and exporting stay (ADR-0018 decision 6). Pure: the caller keeps `memory` and the clocks.
pub fn read_store(
    answer: Result<StoreAppLicence, StoreLicenceError>,
    memory: &StoreMemory,
    now_ms: i64,
    now_mono: Instant,
) -> StoreReading {
    let reading = |state, trial_ends_at_ms, memory| StoreReading {
        state,
        trial_ends_at_ms,
        memory,
    };
    match answer {
        Ok(l) if l.is_active && !l.is_trial => reading(
            EntitlementState::Purchased,
            None,
            StoreMemory {
                last_state: Some(LastState::Purchased),
                ..StoreMemory::default()
            },
        ),
        Ok(l) if l.is_active => reading(
            trial_days(l.trial_seconds_left),
            Some(plus_seconds(now_ms, l.trial_seconds_left)),
            StoreMemory {
                last_state: Some(LastState::Trial),
                trial_seconds: Some(l.trial_seconds_left),
                read_at_ms: Some(now_ms),
                read_at_mono: Some(now_mono),
            },
        ),
        // A purchase is gone. The lock is remembered, so a failed read cannot lift it; only an
        // active answer does.
        Ok(_)
            if matches!(
                memory.last_state,
                Some(LastState::Purchased | LastState::Revoked)
            ) =>
        {
            reading(
                EntitlementState::Revoked,
                None,
                StoreMemory {
                    last_state: Some(LastState::Revoked),
                    ..StoreMemory::default()
                },
            )
        }
        // The trial is over: its time left is spent and the lock is remembered the same way.
        Ok(_) => reading(
            EntitlementState::TrialEnded,
            None,
            StoreMemory {
                last_state: Some(LastState::Ended),
                ..StoreMemory::default()
            },
        ),
        Err(StoreLicenceError::Unreachable) => {
            let state_and_end = match (memory.last_state, memory.trial_seconds) {
                (Some(LastState::Purchased), _) => Some((EntitlementState::Purchased, None)),
                (Some(LastState::Revoked), _) => Some((EntitlementState::Revoked, None)),
                (Some(LastState::Ended), _) => Some((EntitlementState::TrialEnded, None)),
                (Some(LastState::Trial), Some(seconds)) => {
                    let elapsed_ms = match (memory.read_at_mono, memory.read_at_ms) {
                        (Some(mono), _) => {
                            i64::try_from(now_mono.saturating_duration_since(mono).as_millis()).ok()
                        }
                        (None, Some(at)) => Some(now_ms.saturating_sub(at).max(0)),
                        (None, None) => None,
                    };
                    elapsed_ms.and_then(|elapsed| {
                        let left_ms = i64::try_from(seconds.saturating_mul(1000))
                            .unwrap_or(i64::MAX)
                            .saturating_sub(elapsed);
                        (left_ms > 0).then(|| {
                            let left_s = u64::try_from(left_ms).unwrap_or(0).div_ceil(1000);
                            (trial_days(left_s), Some(plus_seconds(now_ms, left_s)))
                        })
                    })
                }
                _ => None,
            };
            let (state, ends) = state_and_end.unwrap_or((EntitlementState::NeedsOnlineCheck, None));
            reading(state, ends, *memory)
        }
    }
}

/// The wall clock and the monotonic clock, read together. Injected in tests.
pub type Clock = Box<dyn Fn() -> (i64, Instant) + Send + Sync>;

fn system_clock() -> (i64, Instant) {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    (ms, Instant::now())
}

/// The last read, kept between reads.
#[derive(Default)]
struct Remembered {
    memory: StoreMemory,
    trial_ends_at_ms: Option<i64>,
}

/// Microsoft Store channel: the Store's licence and purchase dialog through a
/// [`StoreLicenceSource`].
pub struct MicrosoftStoreAdapter {
    source: Box<dyn StoreLicenceSource>,
    connected: bool,
    clock: Clock,
    remembered: Mutex<Remembered>,
}

impl MicrosoftStoreAdapter {
    /// The adapter over the shell's Store calls.
    pub fn new(source: Box<dyn StoreLicenceSource>) -> Self {
        Self::with_clock(source, Box::new(system_clock))
    }

    /// [`Self::new`] with the clocks given (tests).
    pub fn with_clock(source: Box<dyn StoreLicenceSource>, clock: Clock) -> Self {
        Self {
            source,
            connected: true,
            clock,
            remembered: Mutex::default(),
        }
    }

    /// A build without the Store calls wired: NeedsOnlineCheck and nothing can be bought. A gap
    /// in the build, not a state a user of a finished Store build sees.
    pub fn unavailable() -> Self {
        Self {
            source: Box::new(UnavailableStoreLicence),
            connected: false,
            clock: Box::new(system_clock),
            remembered: Mutex::default(),
        }
    }

    /// Ask the Store and keep the answer. The lock is not held while the Store works, which can
    /// take seconds.
    fn current(&self) -> EntitlementState {
        let answer = self.source.app_licence();
        let (now_ms, now_mono) = (self.clock)();
        let mut kept = self.remembered.lock();
        let read = read_store(answer, &kept.memory, now_ms, now_mono);
        kept.memory = read.memory;
        kept.trial_ends_at_ms = read.trial_ends_at_ms;
        read.state
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
            EntitlementState::Revoked => PurchaseStatus::Revoked,
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
    fn trial_ends_at_ms(&self) -> Option<i64> {
        self.remembered.lock().trial_ends_at_ms
    }
    fn seed_memory(&self, mut memory: StoreMemory) {
        // The Store trial lasts 7 days, so a longer remembered time is an edited file.
        memory.trial_seconds = memory.trial_seconds.map(|s| s.min(7 * DAY_SECONDS));
        self.remembered.lock().memory = memory;
    }
    fn memory(&self) -> Option<StoreMemory> {
        Some(self.remembered.lock().memory)
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
                message: "Straft could not reach the Microsoft Store. Connect to the internet, then try again.",
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

    /// A first read: nothing remembered.
    fn store_licence_state(answer: Result<StoreAppLicence, StoreLicenceError>) -> EntitlementState {
        read_store(answer, &StoreMemory::default(), 0, Instant::now()).state
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

        // A buyer is never locked by a network error.
        *store.licence.lock().unwrap() = Err(StoreLicenceError::Unreachable);
        assert_eq!(a.channel_state(), Some(EntitlementState::Purchased));
        assert_eq!(a.purchase_status(0), PurchaseStatus::Purchased);

        // Offline with no cached licence and nothing remembered: a check is needed, which never
        // locks opening.
        let fresh = for_channel(
            Channel::MicrosoftStore,
            Path::new("unused"),
            None,
            Some(Box::new(store)),
        )
        .unwrap();
        assert_eq!(
            fresh.channel_state(),
            Some(EntitlementState::NeedsOnlineCheck)
        );
        assert_eq!(fresh.purchase_status(0), PurchaseStatus::Unknown);
    }

    const T0: i64 = 1_800_000_000_000;
    const DAY_MS: i64 = 86_400_000;

    fn trial_memory(seconds: u64, at_ms: i64, mono: Option<Instant>) -> StoreMemory {
        StoreMemory {
            last_state: Some(LastState::Trial),
            trial_seconds: Some(seconds),
            read_at_ms: Some(at_ms),
            read_at_mono: mono,
        }
    }

    const GONE: Result<StoreAppLicence, StoreLicenceError> = Err(StoreLicenceError::Unreachable);

    #[test]
    fn an_active_answer_refreshes_the_memory_and_names_the_end_of_the_trial() {
        let base = Instant::now();
        let r = read_store(
            Ok(lic(true, true, 3 * DAY_SECONDS)),
            &StoreMemory::default(),
            T0,
            base,
        );
        assert_eq!(r.state, EntitlementState::TrialActive { days_left: 3 });
        assert_eq!(r.trial_ends_at_ms, Some(T0 + 3 * DAY_MS));
        assert_eq!(r.memory, trial_memory(3 * DAY_SECONDS, T0, Some(base)));

        // Bought at any moment: the trial time is dropped and the end moment is gone.
        let r = read_store(Ok(lic(true, false, 0)), &r.memory, T0 + 1, base);
        assert_eq!(r.state, EntitlementState::Purchased);
        assert_eq!(r.trial_ends_at_ms, None);
        assert_eq!(
            r.memory,
            StoreMemory {
                last_state: Some(LastState::Purchased),
                ..StoreMemory::default()
            }
        );
    }

    #[test]
    fn a_store_confirmed_lock_holds_through_failed_reads_and_only_a_good_answer_lifts_it() {
        let base = Instant::now();
        let bought = StoreMemory {
            last_state: Some(LastState::Purchased),
            ..StoreMemory::default()
        };
        let revoked = StoreMemory {
            last_state: Some(LastState::Revoked),
            ..StoreMemory::default()
        };
        let ended = StoreMemory {
            last_state: Some(LastState::Ended),
            ..StoreMemory::default()
        };
        // A refund, whether the Store calls the licence a trial or not, and a second inactive
        // answer after it: Revoked, remembered as revoked.
        for (before, inactive) in [
            (bought, lic(false, false, 0)),
            (bought, lic(false, true, 0)),
            (revoked, lic(false, false, 0)),
        ] {
            let r = read_store(Ok(inactive), &before, T0, base);
            assert_eq!(r.state, EntitlementState::Revoked);
            assert_eq!(r.trial_ends_at_ms, None);
            assert_eq!(r.memory, revoked);
        }
        // A failed read after the refund stays Revoked, again and again.
        let mut mem = revoked;
        for step in 1..=3 {
            let r = read_store(GONE, &mem, T0 + step, base);
            assert_eq!(r.state, EntitlementState::Revoked);
            assert_eq!(r.memory, revoked);
            mem = r.memory;
        }
        // Not active while the last answer was a trial, ended, or nothing: the trial ended. The
        // time left is spent and a failed read afterwards stays TrialEnded.
        for before in [
            trial_memory(86_400, T0, Some(base)),
            StoreMemory::default(),
            ended,
        ] {
            let r = read_store(Ok(lic(false, true, 0)), &before, T0 + DAY_MS, base);
            assert_eq!(r.state, EntitlementState::TrialEnded);
            assert_eq!(r.memory, ended, "no purchase invented");
            let after = read_store(GONE, &r.memory, T0 + DAY_MS + 1, base);
            assert_eq!(after.state, EntitlementState::TrialEnded);
            assert_eq!(after.memory, ended);
        }
        // Only a good answer lifts a lock: a restore or a purchase, or a new trial.
        for lock in [revoked, ended] {
            let r = read_store(Ok(lic(true, false, 0)), &lock, T0, base);
            assert_eq!(r.state, EntitlementState::Purchased);
            assert_eq!(r.memory, bought);
        }
        let r = read_store(Ok(lic(true, true, 2 * DAY_SECONDS)), &ended, T0, base);
        assert_eq!(r.state, EntitlementState::TrialActive { days_left: 2 });
        assert_eq!(r.memory, trial_memory(2 * DAY_SECONDS, T0, Some(base)));
        // The Store sees the purchase again (restore, a new computer): Purchased is back.
        let r = read_store(Ok(lic(true, false, 0)), &bought, T0, base);
        assert_eq!(r.state, EntitlementState::Purchased);
    }

    #[test]
    fn a_failed_read_keeps_a_purchase_and_counts_a_trial_down_but_never_extends_it() {
        let base = Instant::now();
        let secs = |s: u64| base + std::time::Duration::from_secs(s);
        let mem = trial_memory(3 * DAY_SECONDS, T0, Some(base));

        // Inside one run the monotonic clock counts, whatever the wall clock says.
        for (elapsed, days) in [
            (0, 3),
            (DAY_SECONDS, 2),
            (DAY_SECONDS + 1, 2),
            (2 * DAY_SECONDS, 1),
            (3 * DAY_SECONDS - 1, 1),
        ] {
            let r = read_store(GONE, &mem, T0 + 1, secs(elapsed));
            assert_eq!(
                r.state,
                EntitlementState::TrialActive { days_left: days },
                "{elapsed} s after the last good read"
            );
            assert_eq!(r.memory, mem, "a failed read learns nothing");
        }
        let r = read_store(GONE, &mem, T0 + 1000, secs(DAY_SECONDS));
        assert_eq!(r.trial_ends_at_ms, Some(T0 + 1000 + 2 * DAY_MS));
        assert_eq!(
            read_store(GONE, &mem, T0, secs(3 * DAY_SECONDS)).state,
            EntitlementState::NeedsOnlineCheck,
            "the time ran out"
        );
        // Wall clock turned back inside a run changes nothing.
        let r = read_store(GONE, &mem, T0 - 30 * DAY_MS, secs(DAY_SECONDS));
        assert_eq!(r.state, EntitlementState::TrialActive { days_left: 2 });

        // After a restart only the wall clock is left. Forward counts, back gives no extra time.
        let restarted = trial_memory(3 * DAY_SECONDS, T0, None);
        assert_eq!(
            read_store(GONE, &restarted, T0 + DAY_MS, base).state,
            EntitlementState::TrialActive { days_left: 2 }
        );
        let back = read_store(GONE, &restarted, T0 - 5 * DAY_MS, base);
        assert_eq!(back.state, EntitlementState::TrialActive { days_left: 3 });
        assert_eq!(back.trial_ends_at_ms, Some(T0 - 5 * DAY_MS + 3 * DAY_MS));
        assert_eq!(
            read_store(GONE, &restarted, T0 + 4 * DAY_MS, base).state,
            EntitlementState::NeedsOnlineCheck
        );

        // A buyer stays a buyer; a damaged trial memory and no memory need a check.
        let bought = StoreMemory {
            last_state: Some(LastState::Purchased),
            ..StoreMemory::default()
        };
        assert_eq!(
            read_store(GONE, &bought, T0, base).state,
            EntitlementState::Purchased
        );
        for none in [
            StoreMemory::default(),
            StoreMemory {
                last_state: Some(LastState::Trial),
                ..StoreMemory::default()
            },
            StoreMemory {
                last_state: Some(LastState::Trial),
                trial_seconds: Some(DAY_SECONDS),
                ..StoreMemory::default()
            },
        ] {
            let r = read_store(GONE, &none, T0, base);
            assert_eq!(r.state, EntitlementState::NeedsOnlineCheck, "{none:?}");
            assert_eq!(r.trial_ends_at_ms, None);
        }
    }

    /// The adapter with a clock the test moves.
    fn adapter_with_clock(
        store: &std::sync::Arc<FakeStore>,
    ) -> (MicrosoftStoreAdapter, std::sync::Arc<Mutex<(i64, Instant)>>) {
        let now = std::sync::Arc::new(Mutex::new((T0, Instant::now())));
        let clock = now.clone();
        let a = MicrosoftStoreAdapter::with_clock(
            Box::new(store.clone()),
            Box::new(move || *clock.lock().unwrap()),
        );
        (a, now)
    }

    fn advance(now: &Mutex<(i64, Instant)>, seconds: u64) {
        let mut t = now.lock().unwrap();
        t.0 += i64::try_from(seconds).unwrap() * 1000;
        t.1 += std::time::Duration::from_secs(seconds);
    }

    #[test]
    fn the_adapter_walks_every_row_with_a_trial_buy_refund_and_a_restart() {
        let store = std::sync::Arc::new(FakeStore::new(Ok(lic(true, true, 7 * DAY_SECONDS))));
        let (a, now) = adapter_with_clock(&store);
        assert_eq!(a.trial_ends_at_ms(), None, "nothing read yet");

        // Day 1: buy at once.
        assert_eq!(
            a.channel_state(),
            Some(EntitlementState::TrialActive { days_left: 7 })
        );
        assert_eq!(a.trial_ends_at_ms(), Some(T0 + 7 * DAY_MS));
        *store.purchase.lock().unwrap() = StorePurchaseStatus::Succeeded;
        *store.after_purchase.lock().unwrap() = Some(lic(true, false, 0));
        assert_eq!(a.buy_full_version(), Some(Ok(EntitlementState::Purchased)));
        assert_eq!(a.trial_ends_at_ms(), None);
        assert_eq!(a.memory().unwrap().last_state, Some(LastState::Purchased));

        // The Store is unreachable for a day: still bought. Then a refund.
        advance(&now, DAY_SECONDS);
        *store.licence.lock().unwrap() = GONE;
        assert_eq!(a.channel_state(), Some(EntitlementState::Purchased));
        *store.licence.lock().unwrap() = Ok(lic(false, false, 0));
        assert_eq!(a.restore_channel_state(), Some(EntitlementState::Revoked));
        assert_eq!(a.purchase_status(0), PurchaseStatus::Revoked);
        // The buyer buys again from the lock screen.
        *store.after_purchase.lock().unwrap() = Some(lic(true, false, 0));
        assert_eq!(a.buy_full_version(), Some(Ok(EntitlementState::Purchased)));

        // A trial on day 4 that drops offline, counts down, then the app restarts offline.
        let store = std::sync::Arc::new(FakeStore::new(Ok(lic(true, true, 4 * DAY_SECONDS))));
        let (a, now) = adapter_with_clock(&store);
        assert_eq!(
            a.channel_state(),
            Some(EntitlementState::TrialActive { days_left: 4 })
        );
        let saved = a.memory().unwrap();
        *store.licence.lock().unwrap() = GONE;
        advance(&now, DAY_SECONDS + 5);
        assert_eq!(
            a.channel_state(),
            Some(EntitlementState::TrialActive { days_left: 3 })
        );
        assert_eq!(a.trial_ends_at_ms(), Some(T0 + 4 * DAY_MS));

        // Restart: a new adapter seeded from the saved memory (no monotonic moment), the wall
        // clock set back two days before the last good read.
        let (b, now_b) = adapter_with_clock(&store);
        b.seed_memory(StoreMemory {
            read_at_mono: None,
            ..saved
        });
        now_b.lock().unwrap().0 = T0 - 2 * DAY_MS;
        assert_eq!(
            b.channel_state(),
            Some(EntitlementState::TrialActive { days_left: 4 }),
            "a clock turned back gives at most what the Store last said"
        );
        now_b.lock().unwrap().0 = T0 + 2 * DAY_MS;
        assert_eq!(
            b.channel_state(),
            Some(EntitlementState::TrialActive { days_left: 2 })
        );
        now_b.lock().unwrap().0 = T0 + 5 * DAY_MS;
        assert_eq!(b.channel_state(), Some(EntitlementState::NeedsOnlineCheck));

        // The trial ends while the app is open, with the Store answering.
        *store.licence.lock().unwrap() = Ok(lic(false, true, 0));
        assert_eq!(b.channel_state(), Some(EntitlementState::TrialEnded));
        assert_eq!(b.trial_ends_at_ms(), None);
    }

    /// A restart: a new adapter over the same Store, seeded from what the old one saved in a
    /// notice record (so the text form of the memory is covered too).
    fn restarted(
        old: &MicrosoftStoreAdapter,
        store: &std::sync::Arc<FakeStore>,
    ) -> MicrosoftStoreAdapter {
        let mut record = NoticeRecord::default();
        old.memory().unwrap().write_to(&mut record);
        let json = serde_json::to_string(&record).unwrap();
        let record: NoticeRecord = serde_json::from_str(&json).unwrap();
        let (fresh, _) = adapter_with_clock(store);
        fresh.seed_memory(StoreMemory::from_record(&record));
        fresh
    }

    #[test]
    fn a_refund_or_an_ended_trial_stays_locked_through_failed_reads_and_a_restart() {
        // Bought, then refunded, then the Store cannot be reached: still Revoked, also after a
        // restart; a good Store answer brings the purchase back.
        let store = std::sync::Arc::new(FakeStore::new(Ok(lic(true, false, 0))));
        let (a, _) = adapter_with_clock(&store);
        assert_eq!(a.channel_state(), Some(EntitlementState::Purchased));
        *store.licence.lock().unwrap() = Ok(lic(false, false, 0));
        assert_eq!(a.channel_state(), Some(EntitlementState::Revoked));
        *store.licence.lock().unwrap() = GONE;
        assert_eq!(a.channel_state(), Some(EntitlementState::Revoked));
        let b = restarted(&a, &store);
        assert_eq!(b.channel_state(), Some(EntitlementState::Revoked));
        assert_eq!(b.restore_channel_state(), Some(EntitlementState::Revoked));
        assert_eq!(b.purchase_status(0), PurchaseStatus::Revoked);
        *store.licence.lock().unwrap() = Ok(lic(true, false, 0));
        assert_eq!(b.channel_state(), Some(EntitlementState::Purchased));

        // A trial that ended, then no Store answer: still TrialEnded, also after a restart; a
        // good answer with a running trial lifts it.
        let store = std::sync::Arc::new(FakeStore::new(Ok(lic(true, true, DAY_SECONDS))));
        let (a, _) = adapter_with_clock(&store);
        assert_eq!(
            a.channel_state(),
            Some(EntitlementState::TrialActive { days_left: 1 })
        );
        *store.licence.lock().unwrap() = Ok(lic(false, true, 0));
        assert_eq!(a.channel_state(), Some(EntitlementState::TrialEnded));
        *store.licence.lock().unwrap() = GONE;
        assert_eq!(a.channel_state(), Some(EntitlementState::TrialEnded));
        let b = restarted(&a, &store);
        assert_eq!(b.channel_state(), Some(EntitlementState::TrialEnded));
        *store.licence.lock().unwrap() = Ok(lic(true, true, 3 * DAY_SECONDS));
        assert_eq!(
            b.channel_state(),
            Some(EntitlementState::TrialActive { days_left: 3 })
        );
    }

    #[test]
    fn the_memory_round_trips_through_the_notice_record() {
        let mut record = NoticeRecord::default();
        let memory = trial_memory(DAY_SECONDS, T0, Some(Instant::now()));
        assert!(memory.write_to(&mut record));
        assert!(
            !memory.write_to(&mut record),
            "the same memory changes nothing"
        );
        assert_eq!(
            StoreMemory::from_record(&record),
            StoreMemory {
                read_at_mono: None,
                ..memory
            }
        );
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

    /// An edited `notices.json` cannot extend a trial: the remembered seconds are capped at the
    /// 7 days the Store trial lasts.
    #[test]
    fn an_edited_trial_memory_never_gives_more_than_seven_days() {
        let store = std::sync::Arc::new(FakeStore::new(GONE));
        let (a, now) = adapter_with_clock(&store);
        a.seed_memory(StoreMemory {
            last_state: Some(LastState::Trial),
            trial_seconds: Some(400 * DAY_SECONDS),
            read_at_ms: Some(T0),
            read_at_mono: None,
        });
        advance(&now, 1);
        assert_eq!(
            a.channel_state(),
            Some(EntitlementState::TrialActive { days_left: 7 })
        );
        assert_eq!(a.trial_ends_at_ms(), Some(T0 + 7 * DAY_MS));
        advance(&now, 7 * DAY_SECONDS);
        assert_eq!(a.channel_state(), Some(EntitlementState::NeedsOnlineCheck));
    }
}

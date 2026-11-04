//! Licence and free-trial status (build plan area "Payment and free trial in all three channels",
//! items "Shared entitlement interface" and "No data loss on expiry"; ADR-0018).
//!
//! The core holds one [`EntitlementService`]: the release channel's adapter and the last computed
//! [`EntitlementState`]. Startup never fails: anything the adapter cannot answer is logged and the
//! state is Unknown.
//!
//! **Channels (owner decisions 2026-10-07, ADR-0022 and ADR-0023).** Both stores own the 7-day
//! trial and decide the whole state ([`ChannelAdapter::channel_state`]); Straft keeps no trial
//! timer of its own. The Microsoft Store answers through `GetAppLicenseAsync` behind
//! [`straft_entitlement::adapters::StoreLicenceSource`] (supplied by the Windows shell as
//! [`AppConfig::microsoft_store`]), checked at startup, on "Restore purchase" and when the shell
//! sees `OfflineLicensesChanged` ([`EntitlementService::refresh`]). The Windows installer is an
//! internal build for the owner's team, never sold: its state is InternalBuild and nothing is
//! ever asked, shown or locked.
//!
//! On the Mac, StoreKit and the licensing server decide the trial.
//! When the macOS shell supplies the StoreKit bridge and the licence server client
//! ([`AppConfig::mac_inputs`]), the service does not ask them at startup, because both block and
//! startup runs on the main thread. The state starts as NeedsOnlineCheck and the shell runs
//! [`EntitlementService::revalidate`] on a background thread: at launch, after resume or wake,
//! when StoreKit reports a change, and when the lease ([`EntitlementService::lease`]) runs out.
//! Without the inputs (a Windows build, or a Mac build without the bridge) the Mac adapter runs
//! over the "unavailable" bridge and the state is NeedsOnlineCheck: open and export only, nothing
//! granted. A Mac build without the bridge is a gap in the build (the Swift bridge is not wired
//! yet), not a state a user of a finished build is meant to see.
//!
//! **The store lock (ADR-0022 for the Mac, ADR-0023 for the Microsoft Store).** In either store
//! build, when the trial ended without a purchase or the purchase was refunded or revoked, the
//! whole app waits for a purchase or a restore: [`EntitlementService::check_operation`] refuses every operation except
//! the few the lock screen itself needs ([`ALLOWED_WHILE_LOCKED`] and `entitlement.*`). It runs in
//! [`crate::registry::Registry::dispatch`], the one path every operation takes (the interface, the
//! assistant's change sets and package imports), so the webview cannot get around it. The
//! projects stay on disk untouched. The internal installer build never locks, and
//! NeedsOnlineCheck never locks the app (ADR-0018 decision 6). It locks editing alone, as does
//! TrialNotStarted: [`EntitlementService::check_edit`] runs in the same dispatch path and refuses
//! the operations that change a project, while opening, exporting, closing, app settings and
//! `entitlement.*` keep working. The status reports the same answer as `can_edit`.
//!
//! The trial, purchase and first-run safety screens (owner mocks 182 to 198) are driven from here:
//! `entitlement.notice` says which screen is due ([`straft_entitlement::notices::due`]),
//! `entitlement.record_notice` remembers a reminder day or a finished screen in
//! `entitlement/notices.json`, `entitlement.buy` hands off to the store's own purchase window
//! (Apple's payment sheet or the Microsoft Store purchase dialog, which show the price; Straft
//! never names a price, ADR-0023) and `entitlement.start_trial` starts the Mac trial. The notice record is held in memory too, so a
//! disk that cannot be written never traps the user on a mandatory screen.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use straft_domain::auth::ActorOrigin;
use straft_domain::{Actor, AppError, AppResult, now_ms};
use straft_entitlement::adapters::{self, StoreLicenceSource, StoreMemory};
use straft_entitlement::mac::{MacInputs, RevalidateReason};
use straft_entitlement::notices::{
    self, CalendarDay, LastState, Notice, NoticeRecord, NoticeStore,
};
use straft_entitlement::team_gate::{TeamGate, UnlockAnswer};
use straft_entitlement::{
    Channel, ChannelAdapter, EntitlementState, Permission, PurchaseFailure, TRIAL_DAYS,
};
use ts_rs::TS;

use crate::core::{AppConfig, AppCore};
use crate::registry::Registry;

/// Why the assistant never sees these operations (published in the AI tool coverage matrix).
pub const HIDDEN_REASON: &str = "Licence and free-trial status belong to the person at this computer and the release channel; the assistant never reads, restores or changes them.";

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M};
    r.module("Licence and trial");
    r.query("entitlement.status", status).meta(
        M::app_query(
            "Licence and free-trial status for Settings: channel, days left and whether editing is allowed.",
        )
        .hidden(HIDDEN_REASON),
    );
    // Asks the store again; may write only the Mac adapter's cache in the app data folder.
    r.command("entitlement.restore_purchase", restore_purchase).meta(
        M::app_command(
            "Ask the release channel to restore a purchase, then check the licence and trial again.",
        )
        .fs(Fs::ProjectStorage)
        .hidden(HIDDEN_REASON),
    );
    r.query("entitlement.notice", notice).meta(
        M::app_query(
            "Which trial reminder or first-run screen to show now: a reminder in the last 3 trial days, the data note, the file safety tutorial and the welcome after a purchase, or the store screens (the lock after the trial, the licence check, the Mac first launch).",
        )
        .hidden(HIDDEN_REASON),
    );
    // Writes only `entitlement/notices.json` in the app data folder; no project changes.
    r.command("entitlement.record_notice", record_notice).meta(
        M::app_command(
            "Remember that a trial reminder was shown today, or that the data note, the tutorial or the welcome was finished.",
        )
        .fs(Fs::ProjectStorage)
        .hidden(HIDDEN_REASON),
    );
    // Hands over to the channel's own purchase. No payment detail passes through Straft.
    r.command("entitlement.buy", buy).meta(
        M::app_command(
            "Buy the full version through the store's own purchase window: Apple's payment sheet on the Mac, the Microsoft Store purchase dialog on Windows. The internal installer build never sells.",
        )
        .fs(Fs::ProjectStorage)
        .hidden(HIDDEN_REASON),
    );
    // The lock screen has stored what the editors held: fold the write-ahead logs into the
    // database files and end the flush window (ADR-0039 item 9).
    r.command("entitlement.lock_ready", lock_ready).meta(
        M::app_command(
            "The lock screen has saved what the editors held: write the open project and the vault to their files and end the short window in which saving is still allowed.",
        )
        .fs(Fs::ProjectStorage)
        .hidden(HIDDEN_REASON),
    );
    r.command("entitlement.start_trial", start_trial).meta(
        M::app_command(
            "Start the free 7-day trial from the App Store on the Mac. Other channels start the trial by themselves.",
        )
        .fs(Fs::ProjectStorage)
        .hidden(HIDDEN_REASON),
    );
    r.query("team_gate.status", team_gate_status).meta(
        M::app_query(
            "Internal builds: whether the team password must still be entered, and how long the next try waits.",
        )
        .hidden(TEAM_GATE_HIDDEN_REASON),
    );
    // Writes only `entitlement/team-gate.json` (the count of wrong tries) in the app data folder.
    r.command("team_gate.unlock", team_gate_unlock).meta(
        M::app_command(
            "Internal builds: check the team password typed at launch and open the app when it is right.",
        )
        .fs(Fs::ProjectStorage)
        .hidden(TEAM_GATE_HIDDEN_REASON),
    );
}

/// Why the assistant never sees the team password operations.
pub const TEAM_GATE_HIDDEN_REASON: &str = "The team password of an internal build belongs to the person at this computer; the assistant never reads or types it.";

/// Development and end-to-end builds only (see `AppConfig::entitlement_debug_controls`): set the
/// licence state and channel by hand so the trial and first-run screens can be driven end to end.
/// Never registered in a release build.
pub fn register_debug(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Licence and trial");
    r.command("entitlement.debug_set", debug_set).meta(
        M::app_command(
            "Development builds only: set the licence state and release channel by hand for testing.",
        )
        .hidden(HIDDEN_REASON),
    );
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntitlementStatusArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestorePurchaseArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockReadyArgs {}

/// What Settings shows about the licence.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementStatus {
    pub channel: Channel,
    pub state: EntitlementState,
    /// Plain words for the state, for example "Free trial: 4 days left.".
    pub summary: String,
    /// Why the channel cannot answer yet (for example, the store check is not connected).
    pub limitation: Option<String>,
    /// Length of the free trial in days.
    pub trial_days: u32,
    /// Whether the state permits editing; when false, operations that change a project are
    /// refused in Rust ([`EntitlementService::check_edit`]).
    pub can_edit: bool,
    /// When the running trial ends, in milliseconds since 1970: the time the store last reported
    /// plus now. `null` outside a trial. The shell formats it in local time; the core never reads
    /// the time zone (ADR-0039 item 7).
    #[ts(type = "number | null")]
    pub trial_ends_at_ms: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntitlementNoticeArgs {
    /// The user's local calendar day, `YYYY-MM-DD`. A missing or invalid day shows no reminder.
    #[serde(default)]
    pub today: Option<String>,
}

/// A screen the user finished or saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum NoticeKind {
    /// A trial reminder was shown on `today`.
    Reminder,
    /// The data note was read to the end, ticked and confirmed.
    DataNote,
    /// The file safety tutorial was finished with all three boxes ticked.
    Tutorial,
    /// The welcome popup was closed.
    Welcome,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordNoticeArgs {
    pub notice: NoticeKind,
    /// Required for `reminder`: the user's local calendar day, `YYYY-MM-DD`.
    #[serde(default)]
    pub today: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuyArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartTrialArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamGateStatusArgs {}

/// The typed team password. Its `Debug` never prints the password.
#[derive(Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamGateUnlockArgs {
    pub password: String,
}

impl std::fmt::Debug for TeamGateUnlockArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeamGateUnlockArgs").finish_non_exhaustive()
    }
}

/// What the team password screen shows (ADR-0024).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TeamGateStatus {
    /// True in an internal build with an embedded team password hash.
    pub required: bool,
    /// True once the password was right in this launch (always true when not required).
    pub unlocked: bool,
    /// Seconds until the next try is allowed; 0 when a try is allowed now.
    pub wait_seconds: u32,
    /// Wrong tries left before the next wait.
    pub tries_before_wait: u32,
}

/// The answer to one team password try.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TeamGateAnswer {
    pub status: TeamGateStatus,
    /// Plain words when the try did not open the app.
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DebugSetArgs {
    pub channel: Channel,
    pub state: EntitlementState,
    /// Forget the reminder day and the three first-run flags.
    #[serde(default)]
    pub reset_notices: bool,
    /// The end moment of the trial to report, milliseconds since 1970.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub trial_ends_at_ms: Option<i64>,
}

/// The licence status and the screen due now.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementNotice {
    pub status: EntitlementStatus,
    pub notice: Notice,
    /// The one folder to back up (`AppConfig::data_root`), shown on the data note and in the
    /// tutorial. "Show in File Explorer" reveals it through the shell, never from this text.
    pub data_root: String,
    /// True in a debug build. A release build hides the social rows that have no address yet
    /// (ADR-0039 item 12).
    pub debug_build: bool,
}

/// Where "Buy full version" took the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", tag = "route")]
pub enum BuyRoute {
    /// The store's own purchase window ran (Apple's payment sheet, the Microsoft Store purchase
    /// dialog); `status` has the result.
    Store,
    /// The build cannot sell right now; `message` says why.
    Unavailable,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BuyOutcome {
    pub route: BuyRoute,
    pub status: EntitlementStatus,
    /// Plain words when the purchase or the trial start did not finish.
    pub message: Option<String>,
}

// ------------------------------------------------------------------ service

/// The state's name for the log. Carries no days, dates or identifiers.
const fn state_name(state: EntitlementState) -> &'static str {
    match state {
        EntitlementState::TrialActive { .. } => "trialActive",
        EntitlementState::TrialEnded => "trialEnded",
        EntitlementState::Purchased => "purchased",
        EntitlementState::Revoked => "revoked",
        EntitlementState::Unknown => "unknown",
        EntitlementState::NeedsOnlineCheck => "needsOnlineCheck",
        EntitlementState::TrialNotStarted => "trialNotStarted",
        EntitlementState::InternalBuild => "internalBuild",
        EntitlementState::ClockChanged => "clockChanged",
    }
}

/// What the page shows from the licence, for deciding whether to tell it something changed: the
/// state (with its days left) and the day the trial ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChangeMarker {
    channel: Channel,
    state: EntitlementState,
    ends_on_day: Option<i64>,
}

/// How long operations still pass after the app locks while it is running, so the lock screen
/// can store what the editors hold (ADR-0039 item 9).
pub const LOCK_FLUSH_WINDOW: Duration = Duration::from_secs(20);

/// The entitlement state owned by [`AppCore`].
pub struct EntitlementService {
    channel: Channel,
    app_data_dir: PathBuf,
    /// The channel adapter (none for the internal team builds, which are never sold). Built at
    /// startup, except for the Mac App Store with the shell's inputs: building that one asks
    /// StoreKit for the environment, which blocks, so it is built by the first check, which the
    /// shell runs off the main thread.
    adapter: OnceLock<Option<Box<dyn ChannelAdapter>>>,
    /// The shell's StoreKit bridge and licence server client, until the adapter is built.
    mac_inputs: Mutex<Option<MacInputs>>,
    /// The shell's Microsoft Store calls, until the adapter is built.
    microsoft_store: Mutex<Option<Box<dyn StoreLicenceSource>>>,
    state: Mutex<EntitlementState>,
    /// Serialises checks so two never interleave. Held while the adapter works; `state` is locked
    /// only to read or store the answer, so [`Self::status`] never waits for a store or the
    /// network.
    checking: Mutex<()>,
    /// Held while a store purchase window is open, so a second "Buy" never opens a second window.
    /// The check lock is not held: reads must keep running while the person decides, or a trial that ends
    /// meanwhile would go unseen.
    buying: Mutex<()>,
    /// The first check runs on the shell's worker after startup (the stores block), so the page
    /// may already hold typed text when it finds a lock.
    deferred: bool,
    /// The reminder day and the three first-run flags, kept in memory and saved to
    /// `entitlement/notices.json` after each change.
    notices: Mutex<NoticeRecord>,
    notice_store: NoticeStore,
    /// `entitlement.debug_set` only (never set in a release build): the channel and state to
    /// report instead of the real ones.
    /// The third part is the trial end moment to report.
    forced: Mutex<Option<(Channel, EntitlementState, Option<i64>)>>,
    /// Until this moment operations still pass although the state locks (see
    /// [`LOCK_FLUSH_WINDOW`]). Set when a running app moves from not locked to locked.
    flush_until: Mutex<Option<Instant>>,
    /// Length of the flush window in milliseconds; a test makes it short.
    flush_window_ms: AtomicU64,
    /// The window passed an operation and said so in the log (once per window).
    flush_logged: AtomicBool,
    /// The first check has finished. When that check runs inside `start` (before any page), a lock
    /// found by it is a lock at launch and opens no window.
    first_check_done: AtomicBool,
    /// `AppConfig::debug_build`.
    debug_build: bool,
    /// The internal builds' team password gate (ADR-0024). `None` in store packages.
    team_gate: Option<TeamGate>,
}

/// The lock an earlier run saved: an ended trial or a revoked purchase. Nothing else is trusted
/// from the file to start with; the Store's first answer decides the rest.
fn saved_lock(record: &NoticeRecord) -> Option<EntitlementState> {
    match record.last_state {
        Some(LastState::Ended) => Some(EntitlementState::TrialEnded),
        Some(LastState::Revoked) => Some(EntitlementState::Revoked),
        _ => None,
    }
}

/// Plain words when a store build cannot reach its store from this build (no store connection
/// wired, or a state set by hand for testing).
const fn store_not_connected(channel: Channel) -> &'static str {
    match channel {
        Channel::MicrosoftStore => adapters::MICROSOFT_STORE_NOT_CONNECTED,
        _ => "The App Store connection is not available in this build.",
    }
}

/// A second "Buy" while the store's window is already open.
const PURCHASE_WINDOW_OPEN: &str =
    "The purchase window is already open. Finish or close it, then try again.";

/// "Buy full version" on the internal installer build, which is never sold (ADR-0023). The
/// interface hides the button there; this answers a call that gets through anyway.
const INTERNAL_BUILD_NOT_FOR_SALE: &str = "This is an internal build and is not for sale.";

impl EntitlementService {
    /// Build the service for `config`. Never fails: any problem is logged and the state is
    /// Unknown.
    ///
    /// With `mac_inputs` on the Mac App Store channel nothing is asked here: the state is
    /// NeedsOnlineCheck (open and export only) until the shell's first
    /// [`Self::revalidate`] answers. Every other channel computes the state once, now.
    pub fn start(
        config: &AppConfig,
        mac_inputs: Option<MacInputs>,
        microsoft_store: Option<Box<dyn StoreLicenceSource>>,
    ) -> Self {
        // Both stores block while they answer and startup runs on the main thread, so the shell
        // runs the first read on a worker (`AppCore::refresh_entitlement`). Until it answers the
        // Mac says NeedsOnlineCheck and the Microsoft Store says Unknown, which shows nothing.
        let mac_deferred = config.channel == Channel::MacAppStore && mac_inputs.is_some();
        let deferred = mac_deferred
            || (config.channel == Channel::MicrosoftStore && microsoft_store.is_some());
        let notice_store = NoticeStore::in_app_data(&config.app_data_dir);
        let notices = notice_store.load().record;
        let service = Self {
            channel: config.channel,
            app_data_dir: config.app_data_dir.clone(),
            adapter: OnceLock::new(),
            mac_inputs: Mutex::new(mac_inputs),
            microsoft_store: Mutex::new(microsoft_store),
            // A lock saved by an earlier run holds from the first moment, so a Store that hangs for
            // 40 seconds at launch cannot let the app open unlocked (ADR-0039 item 5).
            state: Mutex::new(if mac_deferred {
                EntitlementState::NeedsOnlineCheck
            } else if deferred {
                saved_lock(&notices).unwrap_or(EntitlementState::Unknown)
            } else {
                EntitlementState::Unknown
            }),
            checking: Mutex::new(()),
            buying: Mutex::new(()),
            deferred,
            notices: Mutex::new(notices),
            notice_store,
            forced: Mutex::new(None),
            flush_until: Mutex::new(None),
            flush_window_ms: AtomicU64::new(duration_ms(LOCK_FLUSH_WINDOW)),
            flush_logged: AtomicBool::new(false),
            first_check_done: AtomicBool::new(false),
            debug_build: config.debug_build,
            team_gate: config
                .team_password_hash
                .as_ref()
                .map(|hash| TeamGate::new(hash.clone(), &config.app_data_dir, now_ms())),
        };
        if deferred {
            tracing::info!("the store check runs in the background after startup");
        } else {
            service.refresh();
        }
        service
    }

    /// The channel adapter, built on first use (see the `adapter` field).
    fn adapter(&self) -> Option<&dyn ChannelAdapter> {
        self.adapter
            .get_or_init(|| {
                let mac = self.mac_inputs.lock().take();
                let store = self.microsoft_store.lock().take();
                let built = adapters::for_channel(self.channel, &self.app_data_dir, mac, store);
                if let Some(adapter) = &built {
                    adapter.seed_memory(StoreMemory::from_record(&self.notices.lock()));
                }
                built
            })
            .as_deref()
    }

    /// The last computed state.
    pub fn state(&self) -> EntitlementState {
        match *self.forced.lock() {
            Some((_, state, _)) => state,
            None => *self.state.lock(),
        }
    }

    /// The release channel (or the one `entitlement.debug_set` chose).
    pub fn channel(&self) -> Channel {
        match *self.forced.lock() {
            Some((channel, _, _)) => channel,
            None => self.channel,
        }
    }

    /// Ask the channel again (startup, and the Microsoft Store's `OfflineLicensesChanged`).
    pub fn refresh(&self) -> EntitlementState {
        self.evaluate(Check::Launch)
    }

    /// Ask the channel to restore a purchase, then check again.
    pub fn restore_purchase(&self) -> EntitlementState {
        self.evaluate(Check::Restore)
    }

    /// Check again because something changed (see [`RevalidateReason`]). Blocks while StoreKit
    /// and the licence server answer, so the shell calls it from a background thread only.
    ///
    /// Today this runs the channel's restore check (`restore_channel_state`), which on the Mac
    /// asks StoreKit's `currentEntitlements` and, during a trial, the licence server. When
    /// `ChannelAdapter` gains the Mac adapter's `revalidate(reason)`, only
    /// [`Self::evaluate`]'s `Check::Revalidate` branch changes.
    pub fn revalidate(&self, reason: RevalidateReason) -> EntitlementState {
        let state = self.evaluate(Check::Revalidate);
        tracing::info!(
            reason = ?reason,
            state = state_name(state),
            "licence and trial checked again"
        );
        state
    }

    /// How long the last answer may be used before the shell checks again. `None` for the
    /// Windows channels, which check at startup and on "Restore purchase" only.
    ///
    /// Fixed intervals by state until `ChannelAdapter` exposes the Mac adapter's
    /// `time_until_check` (the licence server's own lease): a running trial is confirmed hourly (so its end and a refund are seen within the
    /// hour), a failed check is retried every 15 minutes, a lifetime purchase daily, and every
    /// other state every 6 hours. StoreKit changes and resume trigger a check sooner.
    pub fn lease(&self) -> Option<Duration> {
        if self.channel != Channel::MacAppStore {
            return None;
        }
        const MINUTE: u64 = 60;
        let secs = match self.state() {
            EntitlementState::TrialActive { .. } => 60 * MINUTE,
            EntitlementState::NeedsOnlineCheck | EntitlementState::Unknown => 15 * MINUTE,
            EntitlementState::Purchased => 24 * 60 * MINUTE,
            EntitlementState::TrialEnded
            | EntitlementState::ClockChanged
            | EntitlementState::TrialNotStarted
            | EntitlementState::Revoked
            | EntitlementState::InternalBuild => 6 * 60 * MINUTE,
        };
        Some(Duration::from_secs(secs))
    }

    fn evaluate(&self, check: Check) -> EntitlementState {
        let _serial = self.checking.lock();
        if let Some((_, forced, _)) = *self.forced.lock() {
            return forced;
        }
        let was_locked = self.locks_the_app();
        let first = !self.first_check_done.swap(true, Ordering::SeqCst);
        if self.channel == Channel::DirectInstaller {
            // The internal build is never sold and asks nothing (ADR-0023).
            *self.state.lock() = EntitlementState::InternalBuild;
            return EntitlementState::InternalBuild;
        }
        let computed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let Some(adapter) = self.adapter() else {
                return EntitlementState::Unknown;
            };
            // The store decides the whole state; Straft keeps no trial timer of its own.
            let channel_state = match check {
                Check::Launch => adapter.channel_state(),
                Check::Restore | Check::Revalidate => adapter.restore_channel_state(),
            };
            channel_state.unwrap_or(EntitlementState::Unknown)
        }))
        .unwrap_or_else(|_| {
            tracing::warn!("the licence check failed unexpectedly; the licence state is unknown");
            EntitlementState::Unknown
        });
        *self.state.lock() = computed;
        self.remember_store();
        self.note_transition(was_locked, !first || self.deferred);
        computed
    }

    /// Keep what the adapter learned from the Store (the last good answer) in the notice record,
    /// saved only when it changed. A failed save is logged; the memory still holds in this run.
    fn remember_store(&self) {
        let Some(memory) = self
            .adapter
            .get()
            .and_then(|a| a.as_deref())
            .and_then(|a| a.memory())
        else {
            return;
        };
        let mut record = self.notices.lock();
        if memory.write_to(&mut record)
            && let Err(e) = self.notice_store.write(&record)
        {
            tracing::warn!(kind = ?e.kind(), "the licence memory could not be saved");
        }
    }

    /// The state just changed. When a running app went from not locked to locked, open the flush
    /// window (ADR-0039 item 9); a lock found by the first check, or one that cleared, leaves none.
    fn note_transition(&self, was_locked: bool, check_done_before: bool) {
        let mut until = self.flush_until.lock();
        if !self.locks_the_app() {
            *until = None;
        } else if !was_locked && check_done_before {
            let window = Duration::from_millis(self.flush_window_ms.load(Ordering::SeqCst));
            *until = Some(Instant::now() + window);
            self.flush_logged.store(false, Ordering::SeqCst);
            tracing::info!("the app locked while running; saving stays open for a short while");
        }
    }

    /// True while the flush window lets operations through although the state locks.
    pub fn flush_window_open(&self) -> bool {
        self.flush_until.lock().is_some_and(|t| Instant::now() < t)
    }

    /// End the flush window now (the lock screen has stored everything).
    pub fn end_flush_window(&self) {
        *self.flush_until.lock() = None;
    }

    /// Make the flush window short or long. Tests only.
    #[doc(hidden)]
    pub fn set_flush_window_for_tests(&self, window: Duration) {
        self.flush_window_ms
            .store(duration_ms(window), Ordering::SeqCst);
    }

    /// What the page shows, to compare before and after a check.
    pub(crate) fn change_marker(&self) -> ChangeMarker {
        let status = self.status();
        ChangeMarker {
            channel: status.channel,
            state: status.state,
            ends_on_day: status.trial_ends_at_ms.map(|ms| ms.div_euclid(86_400_000)),
        }
    }

    /// The Settings view of the current state. Never waits for a check that is running.
    pub fn status(&self) -> EntitlementStatus {
        let state = self.state();
        EntitlementStatus {
            channel: self.channel(),
            state,
            summary: state.summary(self.channel()),
            limitation: if self.channel() == Channel::DirectInstaller {
                None
            } else {
                self.adapter
                    .get()
                    .and_then(|a| a.as_deref().and_then(|a| a.limitation()))
                    .map(str::to_string)
            },
            trial_days: TRIAL_DAYS,
            can_edit: state.permits(self.channel(), Permission::Edit),
            trial_ends_at_ms: match *self.forced.lock() {
                Some((_, _, ends)) => ends,
                None if matches!(state, EntitlementState::TrialActive { .. }) => self
                    .adapter
                    .get()
                    .and_then(|a| a.as_deref())
                    .and_then(|a| a.trial_ends_at_ms()),
                None => None,
            },
        }
    }
}

/// Operations outside `entitlement.*` that still run while the Mac lock holds: what the lock
/// screen itself needs to draw (the platform name for "Show in Finder" and the user's theme), and
/// the team password check the window asks first on every launch (a store package answers that
/// no password is needed; refusing it left a locked store user on a "could not start the
/// password check" screen instead of the lock). None of them reads or writes a project.
pub const ALLOWED_WHILE_LOCKED: &[&str] = &["app.info", "appearance.get", "team_gate.status"];

/// Operations that run before the team password is right: the password screen's own, and the
/// user's theme so the screen looks like the rest of the app. Nothing else opens (ADR-0024).
pub const TEAM_GATE_ALLOWED: &[&str] = &["team_gate.status", "team_gate.unlock", "appearance.get"];

/// Plain words for an operation refused before the team password.
pub const TEAM_GATE_LOCKED_MESSAGE: &str = "Enter the team password to open Straft Studio.";

/// Plain words for an operation refused by the Mac lock.
pub const LOCKED_MESSAGE: &str =
    "Buy Straft Studio or restore your purchase to open your projects again. Nothing is deleted.";

impl EntitlementService {
    /// True while the whole app waits for a purchase or a restore (the Mac lock, ADR-0022).
    pub fn locks_the_app(&self) -> bool {
        self.state().locks_the_app(self.channel())
    }

    /// The Mac lock (ADR-0022): refuse `op` while the app waits for a purchase or a restore,
    /// unless it is an `entitlement.*` operation or in [`ALLOWED_WHILE_LOCKED`]. Called for every
    /// operation before its handler runs.
    pub fn check_operation(&self, op: &str) -> AppResult<()> {
        if self.team_gate_locked() && !TEAM_GATE_ALLOWED.contains(&op) {
            return Err(AppError::new(
                "permission.team_password_required",
                TEAM_GATE_LOCKED_MESSAGE,
            )
            .with_detail(format!("{op} refused before the team password")));
        }
        if !self.locks_the_app()
            || op.starts_with("entitlement.")
            || ALLOWED_WHILE_LOCKED.contains(&op)
        {
            return Ok(());
        }
        if self.flush_window_open() {
            if !self.flush_logged.swap(true, Ordering::SeqCst) {
                tracing::info!(op, "an operation passed the lock's flush window");
            }
            return Ok(());
        }
        Err(AppError::new("permission.licence_required", LOCKED_MESSAGE)
            .with_detail(format!("{op} refused: {}", state_name(self.state()))))
    }
}

/// Plain words for an edit refused while the purchase could not be checked.
pub const EDIT_NEEDS_CHECK_MESSAGE: &str = "Straft could not check your purchase, so editing is paused. You can still open your projects and export them. Connect to the internet, then try again.";

/// Plain words for an edit refused before the free trial has started.
pub const EDIT_NEEDS_TRIAL_MESSAGE: &str =
    "Start your free trial to edit. You can still open your projects and export them.";

impl EntitlementService {
    /// Editing is locked in NeedsOnlineCheck and TrialNotStarted (`Permission::Edit`, package
    /// file 02): refuse an operation that changes a project (`changes_project`, decided by the
    /// registry from its metadata). Opening, exporting, closing and the app's own settings pass.
    /// A full lock is [`Self::check_operation`]'s job and its flush window must not be cut short
    /// here, so a locked app passes this check.
    pub fn check_edit(&self, op: &str, changes_project: bool) -> AppResult<()> {
        if !changes_project
            || self.locks_the_app()
            || self.state().permits(self.channel(), Permission::Edit)
        {
            return Ok(());
        }
        let message = if self.state() == EntitlementState::TrialNotStarted {
            EDIT_NEEDS_TRIAL_MESSAGE
        } else {
            EDIT_NEEDS_CHECK_MESSAGE
        };
        Err(
            AppError::new("permission.licence_required", message).with_detail(format!(
                "{op} refused: editing locked in {}",
                state_name(self.state())
            )),
        )
    }

    /// True while an internal build waits for the team password. Always false in store packages.
    pub fn team_gate_locked(&self) -> bool {
        self.team_gate.as_ref().is_some_and(|g| !g.is_unlocked())
    }

    /// The team password screen's view.
    pub fn team_gate_status(&self) -> TeamGateStatus {
        match &self.team_gate {
            None => TeamGateStatus {
                required: false,
                unlocked: true,
                wait_seconds: 0,
                tries_before_wait: 0,
            },
            Some(gate) => {
                let view = gate.view(now_ms());
                TeamGateStatus {
                    required: true,
                    unlocked: view.unlocked,
                    wait_seconds: seconds(view.wait_ms),
                    tries_before_wait: view.tries_before_wait,
                }
            }
        }
    }

    /// One try of the team password at `now`. Returns the plain words to show when it fails, or
    /// `None` when the app opens (or has no gate).
    pub fn team_gate_unlock(&self, password: &str, now: i64) -> Option<String> {
        let gate = self.team_gate.as_ref()?;
        match gate.try_unlock(password, now) {
            UnlockAnswer::Unlocked => {
                tracing::info!("team password accepted");
                None
            }
            UnlockAnswer::Wrong {
                wait_ms: 0,
                tries_before_wait,
            } => {
                tracing::info!("team password refused");
                Some(format!(
                    "That password is not right. {} left before a wait.",
                    count(tries_before_wait, "try", "tries")
                ))
            }
            UnlockAnswer::Wrong { wait_ms, .. } => {
                tracing::info!("team password refused; the next try waits");
                Some(format!(
                    "That password is not right. Too many wrong tries: wait {} before you try again.",
                    duration_words(wait_ms)
                ))
            }
            UnlockAnswer::Waiting { wait_ms } => Some(format!(
                "Too many wrong tries. Try again in {}.",
                duration_words(wait_ms)
            )),
        }
    }

    /// The screen due now for the user's local calendar day (see [`notices::due`]).
    pub fn notice(&self, today: Option<&CalendarDay>) -> Notice {
        let record = self.notices.lock().clone();
        notices::due(self.channel(), self.state(), &record, today)
    }

    /// Remember a screen. Refuses one that is out of order: a reminder only during the trial,
    /// the data note only after a purchase, the tutorial only after the data note and the welcome
    /// only after the tutorial. The change holds in memory at once; when the file cannot be
    /// written the screen may show again at the next launch, and the user is never stuck.
    pub fn record_notice(&self, kind: NoticeKind, today: Option<&CalendarDay>) -> AppResult<()> {
        let state = self.state();
        let mut record = self.notices.lock();
        match kind {
            NoticeKind::Reminder => {
                let today = today.ok_or_else(|| {
                    AppError::invalid_input("The date for the reminder is missing or not valid.")
                })?;
                if !matches!(state, EntitlementState::TrialActive { .. }) {
                    return Err(AppError::conflict(
                        "There is no running free trial to remind you about.",
                    ));
                }
                record.reminder_shown_on = Some(today.as_str().to_string());
            }
            NoticeKind::DataNote => {
                if state != EntitlementState::Purchased {
                    return Err(AppError::conflict(
                        "The data note is shown after you buy the full version.",
                    ));
                }
                record.data_note_acknowledged = true;
            }
            NoticeKind::Tutorial => {
                if state != EntitlementState::Purchased || !record.data_note_acknowledged {
                    return Err(AppError::conflict(
                        "The file safety tutorial follows the data note.",
                    ));
                }
                record.tutorial_completed = true;
            }
            NoticeKind::Welcome => {
                if state != EntitlementState::Purchased || !record.tutorial_completed {
                    return Err(AppError::conflict(
                        "The welcome follows the file safety tutorial.",
                    ));
                }
                record.welcome_shown = true;
            }
        }
        if let Err(e) = self.notice_store.write(&record) {
            tracing::warn!(
                kind = ?e.kind(),
                "the notice record could not be saved; the screen may show again next time"
            );
        }
        Ok(())
    }

    /// "Buy full version" for the current channel: the store's own purchase window, which shows
    /// the price (ADR-0023). The internal installer build never sells.
    pub fn buy(&self) -> (BuyRoute, Option<String>) {
        match self.channel() {
            Channel::DirectInstaller => (
                BuyRoute::Unavailable,
                Some(INTERNAL_BUILD_NOT_FOR_SALE.to_string()),
            ),
            Channel::MicrosoftStore | Channel::MacAppStore => {
                self.store_purchase(|a| a.buy_full_version())
            }
        }
    }

    /// "Start free trial" on the Mac. Other channels start the trial by themselves.
    pub fn start_trial(&self) -> (BuyRoute, Option<String>) {
        if self.channel() != Channel::MacAppStore {
            return (BuyRoute::Unavailable, None);
        }
        self.store_purchase(|a| a.start_trial())
    }

    /// Run a purchase through the channel adapter and keep its answer as the new state.
    fn store_purchase(
        &self,
        run: impl FnOnce(&dyn ChannelAdapter) -> Option<Result<EntitlementState, PurchaseFailure>>,
    ) -> (BuyRoute, Option<String>) {
        if self.forced.lock().is_some() {
            // A state set by hand has no store behind it.
            return (
                BuyRoute::Unavailable,
                Some(store_not_connected(self.channel()).to_string()),
            );
        }
        // One purchase window at a time. `checking` is not held while the store's window is open:
        // reads go on, so a trial that ends meanwhile is seen. It is taken only to store the answer.
        let Some(_window) = self.buying.try_lock() else {
            return (BuyRoute::Store, Some(PURCHASE_WINDOW_OPEN.to_string()));
        };
        let was_locked = self.locks_the_app();
        let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.adapter().and_then(run)
        }))
        .unwrap_or_else(|_| {
            tracing::warn!("the purchase failed unexpectedly");
            Some(Err(PurchaseFailure {
                message: "The purchase did not finish. Try again later.",
            }))
        });
        let _serial = self.checking.lock();
        match answer {
            Some(Ok(state)) => {
                *self.state.lock() = state;
                self.remember_store();
                self.note_transition(was_locked, self.first_check_done.load(Ordering::SeqCst));
                tracing::info!(
                    state = state_name(state),
                    "purchase or trial start finished"
                );
                (BuyRoute::Store, None)
            }
            Some(Err(failure)) => (BuyRoute::Store, Some(failure.message.to_string())),
            None => (
                BuyRoute::Unavailable,
                Some(store_not_connected(self.channel).to_string()),
            ),
        }
    }

    /// `entitlement.debug_set` only: report `channel` and `state` until the app closes.
    fn force(
        &self,
        channel: Channel,
        state: EntitlementState,
        trial_ends_at_ms: Option<i64>,
        reset_notices: bool,
    ) {
        let was_locked = self.locks_the_app();
        *self.forced.lock() = Some((channel, state, trial_ends_at_ms));
        // A hand-set state counts as a running app, also for the first call.
        self.first_check_done.store(true, Ordering::SeqCst);
        self.note_transition(was_locked, true);
        if reset_notices {
            let mut record = self.notices.lock();
            *record = NoticeRecord::default();
            if let Err(e) = self.notice_store.write(&record) {
                tracing::warn!(kind = ?e.kind(), "the notice record could not be reset");
            }
        }
    }
}

/// Which check [`EntitlementService::evaluate`] runs.
#[derive(Debug, Clone, Copy)]
enum Check {
    /// The check at startup (or a later refresh).
    Launch,
    /// "Restore purchase".
    Restore,
    /// A check the shell asked for (resume, StoreKit change, lease).
    Revalidate,
}

impl std::fmt::Debug for EntitlementService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntitlementService")
            .field("channel", &self.channel)
            .field("state", &self.state())
            .finish_non_exhaustive()
    }
}

// ------------------------------------------------------------------ operations

fn status(
    core: &AppCore,
    _actor: &Actor,
    _args: EntitlementStatusArgs,
) -> AppResult<EntitlementStatus> {
    Ok(core.entitlement.status())
}

/// Only the person at this computer buys, starts a trial or confirms a screen; never the
/// assistant or a package.
fn require_local(actor: &Actor, message: &str) -> AppResult<()> {
    if matches!(actor.origin, ActorOrigin::Local) {
        Ok(())
    } else {
        Err(AppError::new("permission.denied", message))
    }
}

fn parse_day(text: Option<&str>) -> Option<CalendarDay> {
    text.and_then(CalendarDay::parse)
}

fn notice_view(core: &AppCore, today: Option<&CalendarDay>) -> EntitlementNotice {
    EntitlementNotice {
        status: core.entitlement.status(),
        notice: core.entitlement.notice(today),
        data_root: core.data_root().to_string_lossy().into_owned(),
        debug_build: core.entitlement.debug_build,
    }
}

fn notice(
    core: &AppCore,
    _actor: &Actor,
    args: EntitlementNoticeArgs,
) -> AppResult<EntitlementNotice> {
    let today = parse_day(args.today.as_deref());
    Ok(notice_view(core, today.as_ref()))
}

fn record_notice(
    core: &AppCore,
    actor: &Actor,
    args: RecordNoticeArgs,
) -> AppResult<EntitlementNotice> {
    require_local(
        actor,
        "Only the person using this computer can confirm this screen.",
    )?;
    let today = parse_day(args.today.as_deref());
    core.entitlement
        .record_notice(args.notice, today.as_ref())?;
    Ok(notice_view(core, today.as_ref()))
}

fn buy(core: &AppCore, actor: &Actor, _args: BuyArgs) -> AppResult<BuyOutcome> {
    require_local(
        actor,
        "Only the person using this computer can buy Straft Studio.",
    )?;
    let before = core.entitlement.change_marker();
    let (route, message) = core.entitlement.buy();
    core.announce_entitlement_change(before);
    Ok(BuyOutcome {
        route,
        status: core.entitlement.status(),
        message,
    })
}

fn start_trial(core: &AppCore, actor: &Actor, _args: StartTrialArgs) -> AppResult<BuyOutcome> {
    require_local(
        actor,
        "Only the person using this computer can start the free trial.",
    )?;
    let before = core.entitlement.change_marker();
    let (route, message) = core.entitlement.start_trial();
    core.announce_entitlement_change(before);
    Ok(BuyOutcome {
        route,
        status: core.entitlement.status(),
        message,
    })
}

fn debug_set(core: &AppCore, actor: &Actor, args: DebugSetArgs) -> AppResult<EntitlementStatus> {
    require_local(actor, "Only the person using this computer can do this.")?;
    core.entitlement.force(
        args.channel,
        args.state,
        args.trial_ends_at_ms,
        args.reset_notices,
    );
    core.emit_entitlement_changed();
    Ok(core.entitlement.status())
}

fn lock_ready(core: &AppCore, actor: &Actor, _args: LockReadyArgs) -> AppResult<()> {
    require_local(actor, "Only the person using this computer can do this.")?;
    core.checkpoint_open_stores("lock");
    core.entitlement.end_flush_window();
    Ok(())
}

/// Milliseconds of `d`, capped.
fn duration_ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// Whole seconds, rounded up, for a wait in milliseconds.
fn seconds(ms: i64) -> u32 {
    u32::try_from((ms.max(0) + 999) / 1000).unwrap_or(u32::MAX)
}

fn count(n: u32, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// "1 minute", "30 minutes", "45 seconds".
fn duration_words(ms: i64) -> String {
    let secs = seconds(ms);
    if secs >= 60 {
        count(secs.div_ceil(60), "minute", "minutes")
    } else {
        count(secs.max(1), "second", "seconds")
    }
}

fn team_gate_status(
    core: &AppCore,
    _actor: &Actor,
    _args: TeamGateStatusArgs,
) -> AppResult<TeamGateStatus> {
    Ok(core.entitlement.team_gate_status())
}

fn team_gate_unlock(
    core: &AppCore,
    actor: &Actor,
    args: TeamGateUnlockArgs,
) -> AppResult<TeamGateAnswer> {
    require_local(
        actor,
        "Only the person using this computer can enter the team password.",
    )?;
    let was_locked = core.entitlement.team_gate_locked();
    let message = core.entitlement.team_gate_unlock(&args.password, now_ms());
    drop(args);
    if was_locked && !core.entitlement.team_gate_locked() {
        // What app start does once the gate opens (the Global Idea Vault's Recently Deleted).
        core.startup_maintenance();
    }
    Ok(TeamGateAnswer {
        status: core.entitlement.team_gate_status(),
        message,
    })
}

fn restore_purchase(
    core: &AppCore,
    actor: &Actor,
    _args: RestorePurchaseArgs,
) -> AppResult<EntitlementStatus> {
    // A purchase belongs to the person at this computer, never to the assistant or a package.
    if !matches!(actor.origin, ActorOrigin::Local) {
        return Err(AppError::new(
            "permission.denied",
            "Only the person using this computer can restore a purchase.",
        ));
    }
    let before = core.entitlement.change_marker();
    core.entitlement.restore_purchase();
    core.announce_entitlement_change(before);
    Ok(core.entitlement.status())
}

//! Entitlement interface for OpenFrame Studio (build plan area "Payment and free trial in all three
//! channels"; ADR-0018).
//!
//! The product is paid with a 7-day free trial and no credit card to start. This crate answers,
//! for the rest of the app, exactly three questions: is the trial running and for how many days,
//! is the app unlocked, and what may the user do right now. It never talks to a store itself; one
//! adapter per release channel does that behind the [`ChannelAdapter`] trait:
//!
//! | Channel | Adapter | State |
//! |---|---|---|
//! | Microsoft Store (`.msix`) | [`adapters::MicrosoftStoreAdapter`] | stub: waits for the Partner Center account |
//! | Mac App Store (`.pkg`) | [`mac::MacAppStoreAdapter`] | StoreKit 2 trial and lifetime unlock, built and tested with fakes; the Swift bridge and the licensing server are not connected yet |
//! | Internal team builds (`.exe`, `.dmg`) | none | never sold: no trial, no purchase, no lock (ADR-0023, ADR-0024); the team password gate is [`team_gate`] |
//!
//! Rules that hold in every channel (see `docs/engineering/36-entitlement-and-trial.md`):
//!
//! - **Unknown never blocks.** When a store cannot be reached or an adapter is not wired, the
//!   state is [`EntitlementState::Unknown`] and the user can open, edit and export. A paid app
//!   must never hold a filmmaker's work hostage to a licence server.
//! - **Expiry never loses data.** Nothing is ever deleted. In the two store builds (Mac App Store
//!   and Microsoft Store) an ended trial or a refunded purchase locks opening, editing and
//!   exporting until a purchase or a restore (owner decisions 2026-10-07, ADR-0022 and ADR-0023);
//!   the projects stay on disk untouched. A failed online check never locks opening or
//!   exporting (ADR-0018 decision 6).
//! - **The internal builds are never sold.** The Windows installer and the Mac disk image are
//!   builds for the owner's team: no trial, no purchase, no lock
//!   ([`EntitlementState::InternalBuild`], ADR-0023 and ADR-0024), and a team password on every
//!   launch ([`team_gate`]).
//! - **The stores keep the trial clock.** OpenFrame keeps no trial timer of its own, so a clock
//!   turned back on this computer gives no days back.
//! - **No project data in any check.** Adapters receive nothing but the channel's own proof of
//!   purchase.

pub mod adapters;
pub mod mac;
pub mod notices;
pub mod team_gate;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use adapters::{ChannelAdapter, PurchaseFailure};
pub use notices::{CalendarDay, Notice, NoticeRecord, NoticeStore};

/// Length of the free trial in both stores, in days. The stores run the trial clock (ADR-0022,
/// ADR-0023); OpenFrame keeps no trial timer of its own.
pub const TRIAL_DAYS: u32 = 7;

/// The release channel this build was made for. Set once at build time by the desktop shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    MicrosoftStore,
    MacAppStore,
    /// The internal team builds: the Windows installer and the Mac disk image (ADR-0023,
    /// ADR-0024). Never sold.
    DirectInstaller,
}

impl Channel {
    pub const fn name(self) -> &'static str {
        match self {
            Channel::MicrosoftStore => "microsoft-store",
            Channel::MacAppStore => "mac-app-store",
            Channel::DirectInstaller => "direct-installer",
        }
    }
}

/// What the app knows about the user's right to use it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum EntitlementState {
    /// The free trial is running. `days_left` counts whole days still available, at least 1.
    TrialActive { days_left: u32 },
    /// The 7 days are over and no purchase is known.
    TrialEnded,
    /// A purchase (or an active Store licence) is known.
    Purchased,
    /// The channel reported a refund or a revoked licence.
    Revoked,
    /// The channel could not answer (offline, store outage, adapter not wired). Never blocks.
    Unknown,
    /// A store build whose licence could not be confirmed: on the Mac a trial that was not
    /// confirmed online (`mac::MacState::NeedsOnlineCheck`), on the Microsoft Store no Store
    /// answer and no cached licence. Open and Export only (ADR-0018 decision 6); the trial is
    /// neither restarted nor extended. A build with no store connection wired also reports this
    /// state; that is a gap in the build, not something a user of a finished build sees.
    NeedsOnlineCheck,
    /// Mac App Store only: no trial has been started from the App Store yet
    /// (`mac::MacState::TrialNotStarted`). Open and Export only until the user starts it.
    TrialNotStarted,
    /// The internal team builds (Windows installer, Mac disk image), never sold (ADR-0023,
    /// ADR-0024).
    /// Everything is allowed and no trial, reminder or purchase screen ever shows.
    InternalBuild,
}

/// Things the user may want to do that the entitlement can gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// Open an existing project and read it.
    Open,
    /// Export or print an existing project.
    Export,
    /// Change content: edit, create, import, use the assistant to propose changes.
    Edit,
}

impl EntitlementState {
    /// Whether `perm` is allowed in this state on `channel`.
    ///
    /// - **A store build, TrialEnded or Revoked:** nothing. Opening, editing and exporting all
    ///   wait for a purchase or a restore (ADR-0022 for the Mac, ADR-0023 for the Microsoft
    ///   Store). The application core refuses every project operation in this case, see
    ///   [`Self::locks_the_app`].
    /// - **Everywhere else:** Open and Export are always allowed. Edit is locked in TrialEnded,
    ///   Revoked, NeedsOnlineCheck and TrialNotStarted.
    pub const fn permits(self, channel: Channel, perm: Permission) -> bool {
        if self.locks_the_app(channel) {
            return false;
        }
        match perm {
            Permission::Open | Permission::Export => true,
            Permission::Edit => matches!(
                self,
                EntitlementState::TrialActive { .. }
                    | EntitlementState::Purchased
                    | EntitlementState::Unknown
                    | EntitlementState::InternalBuild
            ),
        }
    }

    /// True when the whole app waits for a purchase or a restore: a store build (Mac App Store,
    /// ADR-0022; Microsoft Store, ADR-0023) after the trial ended without a purchase, or after a
    /// refund or revocation. The internal installer build never locks. NeedsOnlineCheck never
    /// locks either: a failed check keeps opening and exporting (ADR-0018 decision 6).
    pub const fn locks_the_app(self, channel: Channel) -> bool {
        matches!(channel, Channel::MacAppStore | Channel::MicrosoftStore)
            && matches!(
                self,
                EntitlementState::TrialEnded | EntitlementState::Revoked
            )
    }

    /// Plain words for the Settings screen and the trial banner.
    pub fn summary(self, channel: Channel) -> String {
        if self.locks_the_app(channel) {
            let first = if self == EntitlementState::Revoked {
                "This purchase was refunded or revoked."
            } else {
                "Your free trial has ended."
            };
            return format!(
                "{first} Buy OpenFrame Studio or restore your purchase to open your projects again. Nothing is deleted."
            );
        }
        match self {
            EntitlementState::TrialActive { days_left: 1 } => "Free trial: 1 day left.".to_string(),
            EntitlementState::TrialActive { days_left } => {
                format!("Free trial: {days_left} days left.")
            }
            EntitlementState::TrialEnded => {
                "Your free trial has ended. You can still open and export your projects. Buy OpenFrame Studio to keep editing."
                    .to_string()
            }
            EntitlementState::Purchased => "OpenFrame Studio is unlocked. Thank you.".to_string(),
            EntitlementState::Revoked => {
                "This purchase was refunded or revoked. You can still open and export your projects."
                    .to_string()
            }
            EntitlementState::Unknown => {
                "OpenFrame could not check your licence right now. Everything keeps working; it will check again later."
                    .to_string()
            }
            EntitlementState::NeedsOnlineCheck => {
                let store = if channel == Channel::MicrosoftStore {
                    "the Microsoft Store"
                } else {
                    "the App Store"
                };
                format!(
                    "OpenFrame needs to check your licence with {store}. Connect to the internet and open OpenFrame again. Your projects stay open and can be exported."
                )
            }
            EntitlementState::TrialNotStarted => {
                "Start your free 7-day trial from the App Store to begin editing. Your projects can be opened and exported."
                    .to_string()
            }
            EntitlementState::InternalBuild => "Internal build, not for sale.".to_string(),
        }
    }
}

/// What a channel adapter knows about a purchase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseStatus {
    /// The channel confirms a valid purchase or licence.
    Purchased,
    /// The channel confirms there is no purchase (so the trial rules decide).
    NotPurchased,
    /// The channel confirms a refund or revocation.
    Revoked,
    /// The channel could not answer.
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [EntitlementState; 8] = [
        EntitlementState::TrialActive { days_left: 3 },
        EntitlementState::TrialEnded,
        EntitlementState::Purchased,
        EntitlementState::Revoked,
        EntitlementState::Unknown,
        EntitlementState::NeedsOnlineCheck,
        EntitlementState::TrialNotStarted,
        EntitlementState::InternalBuild,
    ];

    #[test]
    fn the_internal_installer_build_never_locks_anything() {
        let internal = Channel::DirectInstaller;
        for s in ALL {
            assert!(!s.locks_the_app(internal), "{s:?}");
            assert!(s.permits(internal, Permission::Open), "{s:?}");
            assert!(s.permits(internal, Permission::Export), "{s:?}");
        }
        for channel in [
            Channel::DirectInstaller,
            Channel::MicrosoftStore,
            Channel::MacAppStore,
        ] {
            for perm in [Permission::Open, Permission::Export, Permission::Edit] {
                assert!(EntitlementState::InternalBuild.permits(channel, perm));
            }
        }
        assert_eq!(
            EntitlementState::InternalBuild.summary(internal),
            "Internal build, not for sale."
        );
    }

    #[test]
    fn both_stores_lock_everything_only_after_the_trial_ended_or_a_refund() {
        for store in [Channel::MacAppStore, Channel::MicrosoftStore] {
            for s in ALL {
                let locked = matches!(s, EntitlementState::TrialEnded | EntitlementState::Revoked);
                assert_eq!(s.locks_the_app(store), locked, "{store:?} {s:?}");
                for perm in [Permission::Open, Permission::Export] {
                    assert_eq!(s.permits(store, perm), !locked, "{store:?} {s:?} {perm:?}");
                }
            }
            // A failed online check keeps opening and exporting (ADR-0018 decision 6).
            assert!(EntitlementState::NeedsOnlineCheck.permits(store, Permission::Export));
            assert!(EntitlementState::NeedsOnlineCheck.permits(store, Permission::Open));
        }
        assert!(EntitlementState::TrialNotStarted.permits(Channel::MacAppStore, Permission::Open));
    }

    #[test]
    fn only_ended_revoked_needs_online_check_and_not_started_lock_editing() {
        for channel in [
            Channel::MicrosoftStore,
            Channel::DirectInstaller,
            Channel::MacAppStore,
        ] {
            let edit = |s: EntitlementState| s.permits(channel, Permission::Edit);
            assert!(edit(EntitlementState::TrialActive { days_left: 1 }));
            assert!(edit(EntitlementState::Purchased));
            assert!(edit(EntitlementState::Unknown));
            assert!(!edit(EntitlementState::TrialEnded));
            assert!(!edit(EntitlementState::Revoked));
            assert!(!edit(EntitlementState::NeedsOnlineCheck));
            assert!(!edit(EntitlementState::TrialNotStarted));
            assert!(edit(EntitlementState::InternalBuild));
        }
    }

    #[test]
    fn summaries_are_plain_and_have_no_em_dash() {
        let mac = Channel::MacAppStore;
        let win = Channel::DirectInstaller;
        for channel in [Channel::MicrosoftStore, win, mac] {
            for s in ALL
                .into_iter()
                .chain([EntitlementState::TrialActive { days_left: 1 }])
            {
                let text = s.summary(channel);
                assert!(!text.contains('\u{2014}'), "{text}");
                assert!(text.ends_with('.'), "{text}");
            }
        }
        assert_eq!(
            EntitlementState::TrialActive { days_left: 1 }.summary(win),
            "Free trial: 1 day left."
        );
        // The lock in either store says how to get back in; the price is never named.
        assert_eq!(
            EntitlementState::TrialEnded.summary(Channel::MicrosoftStore),
            EntitlementState::TrialEnded.summary(mac)
        );
        assert!(
            EntitlementState::NeedsOnlineCheck
                .summary(Channel::MicrosoftStore)
                .contains("with the Microsoft Store")
        );
        assert_eq!(
            EntitlementState::TrialEnded.summary(mac),
            "Your free trial has ended. Buy OpenFrame Studio or restore your purchase to open your projects again. Nothing is deleted."
        );
        assert_eq!(
            EntitlementState::Revoked.summary(mac),
            "This purchase was refunded or revoked. Buy OpenFrame Studio or restore your purchase to open your projects again. Nothing is deleted."
        );
        assert_eq!(
            EntitlementState::NeedsOnlineCheck.summary(mac),
            "OpenFrame needs to check your licence with the App Store. Connect to the internet and open OpenFrame again. Your projects stay open and can be exported."
        );
        assert_eq!(
            EntitlementState::TrialNotStarted.summary(mac),
            "Start your free 7-day trial from the App Store to begin editing. Your projects can be opened and exported."
        );
    }
}

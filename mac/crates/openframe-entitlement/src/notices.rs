//! Trial reminders and the once-only screens after a purchase (owner mocks 182 to 198, "Trial,
//! purchase and first-run safety screens"; `docs/engineering/36-entitlement-and-trial.md`
//! section 10).
//!
//! Two things live here:
//!
//! - [`NoticeRecord`], stored next to the trial record as `entitlement/notices.json`: the calendar
//!   day the last trial reminder was shown, and the three flags "data note acknowledged",
//!   "tutorial completed" and "welcome shown". The file only decides whether a screen appears
//!   again. It never grants or removes anything, so a broken, edited or unreadable file is read as
//!   an empty record and never stops the app: the worst case is that a reminder or the data note
//!   is shown once more.
//! - [`due`], the single rule that says which screen the app shows now, from the release channel,
//!   the entitlement state, the record and the user's local calendar day.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Channel, EntitlementState};

/// Upper bound for the record file; larger files are read as an empty record.
pub const MAX_FILE_BYTES: usize = 4096;

/// Trial reminders appear while this many days or fewer are left (trial days 5, 6 and 7 of 7).
pub const REMINDER_DAYS_LEFT: u32 = 3;

/// A calendar day on the user's computer, `YYYY-MM-DD`. The interface sends the local date, so
/// "once per calendar day" follows the user's own midnight. Only real dates from 2000 to 9999 are
/// accepted; anything else is refused before it reaches the record.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CalendarDay(String);

impl CalendarDay {
    pub fn parse(text: &str) -> Option<Self> {
        let b = text.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let digits = |r: std::ops::Range<usize>| -> Option<u32> {
            b[r].iter().try_fold(0u32, |acc, c| {
                c.is_ascii_digit().then(|| acc * 10 + u32::from(c - b'0'))
            })
        };
        let (year, month, day) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
        if !(2000..=9999).contains(&year) || !(1..=12).contains(&month) || day == 0 {
            return None;
        }
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let days_in_month = match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        (day <= days_in_month).then(|| Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What the app remembers about the trial reminders and the screens after a purchase.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NoticeRecord {
    /// The calendar day (`YYYY-MM-DD`) the last trial reminder was shown.
    pub reminder_shown_on: Option<String>,
    /// The user ticked the box and pressed OK on "Your data, your responsibility".
    pub data_note_acknowledged: bool,
    /// The user finished the 4-step file safety tutorial.
    pub tutorial_completed: bool,
    /// The welcome popup was shown and closed.
    pub welcome_shown: bool,
}

impl NoticeRecord {
    /// The stored reminder day, or `None` when it is missing or not a real date.
    pub fn reminder_day(&self) -> Option<CalendarDay> {
        self.reminder_shown_on
            .as_deref()
            .and_then(CalendarDay::parse)
    }
}

/// The screen the app shows now. At most one at a time, in the order of [`due`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Notice {
    /// Nothing to show.
    None,
    /// Trial reminder (mocks 184, 185, 189). `days_left` is 3, 2 or 1.
    Reminder {
        #[serde(rename = "daysLeft")]
        days_left: u32,
    },
    /// "Your data, your responsibility" (mock 193). Must be acknowledged.
    DataNote,
    /// The 4-step file safety tutorial (mocks 194 to 197). Must be finished.
    Tutorial,
    /// "Welcome to the OpenFrame Studio family" (mock 198). Shown once.
    Welcome,
    /// Mac App Store: first launch, free trial or buy (mock 187). Blocks until the user chooses.
    MacFirstLaunch,
    /// Either store: the trial is over or the purchase was refunded (mock 190). The whole app is
    /// locked until a purchase or a restore, and the screen cannot be closed (ADR-0022 for the
    /// Mac, ADR-0023 for the Microsoft Store). The core refuses every project operation
    /// underneath it.
    Locked,
    /// Either store: the licence could not be confirmed online (mock 192). The screen can be
    /// closed: opening and exporting keep working (ADR-0018 decision 6).
    CheckNeeded,
}

/// Which screen is due.
///
/// - **The internal installer build:** nothing, ever (ADR-0023).
/// - **Either store first:** TrialEnded and Revoked show the lock, NeedsOnlineCheck shows the
///   purchase check, and on the Mac TrialNotStarted shows the first-launch choice.
/// - **After a purchase, either store, once each and in order:** the data note, then the
///   tutorial, then the welcome popup.
/// - **During the trial:** a reminder when 3, 2 or 1 days are left (trial days 5, 6 and 7), at
///   most once per calendar day. A stored day later than `today` (the clock was turned back)
///   counts as shown, so moving the clock back and forth never shows it twice.
/// - Without `today` no reminder is shown, because "once per calendar day" cannot be kept.
pub fn due(
    channel: Channel,
    state: EntitlementState,
    record: &NoticeRecord,
    today: Option<&CalendarDay>,
) -> Notice {
    if channel == Channel::DirectInstaller {
        return Notice::None;
    }
    match state {
        EntitlementState::TrialNotStarted if channel == Channel::MacAppStore => {
            Notice::MacFirstLaunch
        }
        EntitlementState::TrialEnded | EntitlementState::Revoked => Notice::Locked,
        EntitlementState::NeedsOnlineCheck => Notice::CheckNeeded,
        EntitlementState::Purchased => {
            if !record.data_note_acknowledged {
                Notice::DataNote
            } else if !record.tutorial_completed {
                Notice::Tutorial
            } else if !record.welcome_shown {
                Notice::Welcome
            } else {
                Notice::None
            }
        }
        EntitlementState::TrialActive { days_left } if days_left <= REMINDER_DAYS_LEFT => {
            let Some(today) = today else {
                return Notice::None;
            };
            match record.reminder_day() {
                Some(shown) if shown >= *today => Notice::None,
                _ => Notice::Reminder {
                    days_left: days_left.max(1),
                },
            }
        }
        _ => Notice::None,
    }
}

/// Reads and writes `entitlement/notices.json` under the app data folder.
pub struct NoticeStore {
    path: PathBuf,
}

/// A record after [`NoticeStore::load`], with a flag for a file that was there but unusable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedNotices {
    pub record: NoticeRecord,
    /// True when a file existed but could not be read or was not a record. The record is then
    /// empty, which can only show a screen again.
    pub replaced: bool,
}

impl NoticeStore {
    /// `app_data_dir/entitlement/notices.json`.
    pub fn in_app_data(app_data_dir: &Path) -> Self {
        Self {
            path: app_data_dir.join("entitlement").join("notices.json"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the record. Never fails: missing means first run; locked, too large, not JSON or the
    /// wrong shape gives an empty record with `replaced: true`.
    pub fn load(&self) -> LoadedNotices {
        let file = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                return LoadedNotices {
                    record: NoticeRecord::default(),
                    replaced: false,
                };
            }
            Err(e) => {
                tracing::warn!(kind = ?e.kind(), "the notice record could not be opened; screens may show again");
                return LoadedNotices {
                    record: NoticeRecord::default(),
                    replaced: true,
                };
            }
        };
        let mut bytes = Vec::with_capacity(256);
        let read = file.take(MAX_FILE_BYTES as u64 + 1).read_to_end(&mut bytes);
        let record = match read {
            Ok(_) if bytes.len() <= MAX_FILE_BYTES => {
                serde_json::from_slice::<NoticeRecord>(&bytes).ok()
            }
            _ => None,
        };
        match record {
            Some(record) => LoadedNotices {
                record,
                replaced: false,
            },
            None => {
                tracing::warn!("the notice record is not readable; screens may show again");
                LoadedNotices {
                    record: NoticeRecord::default(),
                    replaced: true,
                }
            }
        }
    }

    /// Save the record with the shared atomic replace, so a crash never leaves half a file.
    pub fn write(&self, record: &NoticeRecord) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        openframe_platform::atomic::replace_bytes(&self.path, &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str) -> CalendarDay {
        CalendarDay::parse(s).unwrap()
    }

    fn trial(days_left: u32) -> EntitlementState {
        EntitlementState::TrialActive { days_left }
    }

    const WIN: Channel = Channel::MicrosoftStore;
    const DIRECT: Channel = Channel::DirectInstaller;
    const MAC: Channel = Channel::MacAppStore;

    #[test]
    fn calendar_days_are_strict_real_dates() {
        assert_eq!(day("2026-10-07").as_str(), "2026-10-07");
        assert!(CalendarDay::parse("2028-02-29").is_some(), "leap day");
        assert!(CalendarDay::parse("2000-02-29").is_some(), "leap century");
        for bad in [
            "",
            "2026-10-7",
            "2026/10/07",
            "2026-13-01",
            "2026-00-10",
            "2026-02-29",
            "2100-02-29",
            "2026-04-31",
            "1999-12-31",
            "２０２６-10-07",
            "2026-10-07T00:00",
            "+026-10-07",
            "2026-1a-07",
        ] {
            assert!(CalendarDay::parse(bad).is_none(), "{bad:?}");
        }
        assert!(day("2026-10-07") < day("2026-10-08"));
        assert!(day("2026-12-31") < day("2027-01-01"));
    }

    #[test]
    fn the_internal_installer_build_never_shows_a_screen() {
        let today = day("2026-10-07");
        let r = NoticeRecord::default();
        for state in [
            trial(1),
            trial(3),
            EntitlementState::TrialEnded,
            EntitlementState::Revoked,
            EntitlementState::Purchased,
            EntitlementState::NeedsOnlineCheck,
            EntitlementState::TrialNotStarted,
            EntitlementState::Unknown,
            EntitlementState::InternalBuild,
        ] {
            assert_eq!(
                due(DIRECT, state, &r, Some(&today)),
                Notice::None,
                "{state:?}"
            );
        }
    }

    #[test]
    fn reminders_show_on_trial_days_five_six_and_seven_only() {
        let r = NoticeRecord::default();
        let today = day("2026-10-07");
        for (days_left, expected) in [
            (7, Notice::None),
            (6, Notice::None),
            (5, Notice::None),
            (4, Notice::None),
            (3, Notice::Reminder { days_left: 3 }),
            (2, Notice::Reminder { days_left: 2 }),
            (1, Notice::Reminder { days_left: 1 }),
        ] {
            for channel in [WIN, MAC] {
                assert_eq!(
                    due(channel, trial(days_left), &r, Some(&today)),
                    expected,
                    "{channel:?} with {days_left} days left"
                );
            }
        }
    }

    #[test]
    fn a_reminder_shows_at_most_once_per_calendar_day() {
        let mut r = NoticeRecord {
            reminder_shown_on: Some("2026-10-07".to_string()),
            ..Default::default()
        };
        assert_eq!(
            due(WIN, trial(3), &r, Some(&day("2026-10-07"))),
            Notice::None
        );
        assert_eq!(
            due(WIN, trial(2), &r, Some(&day("2026-10-08"))),
            Notice::Reminder { days_left: 2 }
        );
        r.reminder_shown_on = Some("2026-10-08".to_string());
        assert_eq!(
            due(WIN, trial(2), &r, Some(&day("2026-10-08"))),
            Notice::None
        );
        // No local day from the interface: no reminder rather than one too many.
        assert_eq!(due(WIN, trial(1), &r, None), Notice::None);
    }

    #[test]
    fn a_clock_moved_back_never_shows_the_reminder_twice() {
        let r = NoticeRecord {
            reminder_shown_on: Some("2026-10-09".to_string()),
            ..Default::default()
        };
        assert_eq!(
            due(WIN, trial(2), &r, Some(&day("2026-10-08"))),
            Notice::None
        );
        assert_eq!(
            due(WIN, trial(2), &r, Some(&day("2026-01-01"))),
            Notice::None
        );
        // A broken stored day is ignored instead of blocking every later reminder.
        let broken = NoticeRecord {
            reminder_shown_on: Some("9999-99-99".to_string()),
            ..Default::default()
        };
        assert_eq!(
            due(WIN, trial(1), &broken, Some(&day("2026-10-09"))),
            Notice::Reminder { days_left: 1 }
        );
    }

    #[test]
    fn after_a_purchase_the_note_tutorial_and_welcome_follow_in_order_once() {
        let today = day("2026-10-07");
        let mut r = NoticeRecord::default();
        for channel in [WIN, MAC] {
            assert_eq!(
                due(channel, EntitlementState::Purchased, &r, Some(&today)),
                Notice::DataNote
            );
        }
        r.data_note_acknowledged = true;
        assert_eq!(
            due(MAC, EntitlementState::Purchased, &r, None),
            Notice::Tutorial
        );
        r.tutorial_completed = true;
        assert_eq!(
            due(WIN, EntitlementState::Purchased, &r, None),
            Notice::Welcome
        );
        r.welcome_shown = true;
        assert_eq!(
            due(WIN, EntitlementState::Purchased, &r, Some(&today)),
            Notice::None
        );
    }

    #[test]
    fn a_purchase_mid_trial_skips_the_reminder_and_trial_users_skip_the_first_run_screens() {
        let r = NoticeRecord::default();
        let today = day("2026-10-07");
        assert_eq!(
            due(WIN, trial(2), &r, Some(&today)),
            Notice::Reminder { days_left: 2 }
        );
        assert_eq!(
            due(WIN, EntitlementState::Purchased, &r, Some(&today)),
            Notice::DataNote,
            "purchase wins over the reminder at once"
        );
        for state in [trial(7), trial(4), EntitlementState::Unknown] {
            assert_eq!(due(WIN, state, &r, Some(&today)), Notice::None, "{state:?}");
        }
    }

    #[test]
    fn both_stores_lock_after_the_trial_or_a_refund_and_ask_for_a_check_when_offline() {
        let r = NoticeRecord::default();
        for channel in [WIN, MAC] {
            for (state, expected) in [
                (EntitlementState::TrialEnded, Notice::Locked),
                (EntitlementState::Revoked, Notice::Locked),
                (EntitlementState::NeedsOnlineCheck, Notice::CheckNeeded),
                (EntitlementState::Unknown, Notice::None),
            ] {
                assert_eq!(
                    due(channel, state, &r, None),
                    expected,
                    "{channel:?} {state:?}"
                );
            }
            // The lock wins over a pending first-run screen: a refund after the purchase locks.
            let done = NoticeRecord {
                data_note_acknowledged: true,
                ..Default::default()
            };
            assert_eq!(
                due(channel, EntitlementState::Revoked, &done, None),
                Notice::Locked
            );
        }
        // Only the Mac has a trial the user starts.
        assert_eq!(
            due(MAC, EntitlementState::TrialNotStarted, &r, None),
            Notice::MacFirstLaunch
        );
        assert_eq!(
            due(WIN, EntitlementState::TrialNotStarted, &r, None),
            Notice::None
        );
    }

    #[test]
    fn the_record_round_trips_and_a_broken_file_is_an_empty_record_not_a_crash() {
        let dir = tempfile::tempdir().unwrap();
        let store = NoticeStore::in_app_data(dir.path());
        assert_eq!(
            store.load(),
            LoadedNotices {
                record: NoticeRecord::default(),
                replaced: false
            },
            "first run"
        );
        let r = NoticeRecord {
            reminder_shown_on: Some("2026-10-07".to_string()),
            data_note_acknowledged: true,
            tutorial_completed: false,
            welcome_shown: false,
        };
        store.write(&r).unwrap();
        assert_eq!(store.load().record, r);
        let text = std::fs::read_to_string(store.path()).unwrap();
        assert!(text.contains("\"dataNoteAcknowledged\": true"), "{text}");

        for broken in [
            &b"not json"[..],
            b"{\"dataNoteAcknowledged\": \"yes\"}",
            b"[1,2,3]",
            b"\xff\xfe\x00",
        ] {
            std::fs::write(store.path(), broken).unwrap();
            let loaded = store.load();
            assert_eq!(loaded.record, NoticeRecord::default());
            assert!(loaded.replaced);
        }
        // Unknown fields from a later version and missing fields are fine.
        std::fs::write(store.path(), br#"{"welcomeShown": true, "later": 1}"#).unwrap();
        let loaded = store.load();
        assert!(loaded.record.welcome_shown && !loaded.replaced);

        // A huge file is never loaded whole.
        std::fs::write(store.path(), vec![b' '; MAX_FILE_BYTES + 10]).unwrap();
        assert!(store.load().replaced);
    }

    #[test]
    fn a_path_through_a_file_is_a_first_run_and_an_unwritable_folder_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        // `entitlement` is a file, so the record path goes through a file.
        std::fs::write(dir.path().join("entitlement"), b"x").unwrap();
        let store = NoticeStore::in_app_data(dir.path());
        assert!(!store.load().replaced);
        assert!(store.write(&NoticeRecord::default()).is_err());
    }
}

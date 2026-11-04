//! When the Microsoft Store build asks the Store about the licence again (ADR-0039 item 6).
//!
//! One background thread, `straft-entitlement`, makes every read, so the Store is never asked
//! on the main thread and two reads never overlap. A read happens:
//!
//! - at launch, right after setup;
//! - when the Store says the offline licences changed (a purchase, a refund, a sign-in);
//! - when the window gets focus back more than [`FOCUS_AFTER`] after the last read;
//! - when the computer wakes from sleep;
//! - every [`REGULAR`] (15 minutes), at local midnight, and one second after the trial ends.
//!
//! Requests that queue up during a read are answered by one read. The schedule is plain
//! functions, tested on every system; the thread and the hooks are Windows only. An internal
//! installer build starts nothing (see [`starts_worker`]).

// Everything but the pure schedule is Windows only; the schedule is built everywhere for tests.
#![cfg_attr(not(windows), allow(dead_code))]

use std::time::Duration;

use straft_entitlement::Channel;

/// The longest the worker sleeps between reads.
pub const REGULAR: Duration = Duration::from_secs(15 * 60);

/// A focus return counts only when the last read is older than this.
pub const FOCUS_AFTER: Duration = Duration::from_secs(5 * 60);

/// The wait after a trial's end moment before asking, so the Store has moved on by then.
const EXPIRY_GRACE_MS: i64 = 1_000;

const DAY_MS: i64 = 24 * 60 * 60 * 1_000;

/// Whether this channel gets a refresh worker: only the Microsoft Store build. The internal
/// installer build is never sold and keeps no Store connection.
pub fn starts_worker(channel: Channel) -> bool {
    channel == Channel::MicrosoftStore
}

/// How long the worker may sleep after a read at `last_read_ms`: until the soonest of the next
/// regular read, local midnight and one second after the trial ends. Never negative, never more
/// than [`REGULAR`]. All moments are milliseconds since 1970. A trial end that the last read
/// already saw pass is ignored, so a Store that keeps reporting a past end cannot make the
/// worker spin.
pub fn next_wake(
    now_ms: i64,
    last_read_ms: i64,
    trial_ends_at_ms: Option<i64>,
    next_local_midnight_ms: i64,
) -> Duration {
    let regular = i64::try_from(REGULAR.as_millis()).unwrap_or(i64::MAX);
    let mut due = last_read_ms
        .saturating_add(regular)
        .min(next_local_midnight_ms);
    if let Some(end) = trial_ends_at_ms {
        let at = end.saturating_add(EXPIRY_GRACE_MS);
        if at > last_read_ms {
            due = due.min(at);
        }
    }
    let wait = due.saturating_sub(now_ms).clamp(0, regular);
    Duration::from_millis(u64::try_from(wait).unwrap_or(0))
}

/// Whether a focus return asks for a read: only after more than [`FOCUS_AFTER`] since the last
/// one, or when there has been none.
pub fn focus_needs_read(focused: bool, since_last_read: Option<Duration>) -> bool {
    focused && since_last_read.is_none_or(|d| d > FOCUS_AFTER)
}

/// Milliseconds from a local time of day to the next local midnight, always 1 to 86,400,000.
/// A day with a clock change is an hour out at most; the 15 minute read covers it.
pub fn ms_until_midnight(hour: u16, minute: u16, second: u16, milli: u16) -> i64 {
    let since = ((i64::from(hour) * 60 + i64::from(minute)) * 60 + i64::from(second)) * 1_000
        + i64::from(milli);
    (DAY_MS - since).clamp(1, DAY_MS)
}

#[cfg(windows)]
pub use worker::{on_resume, on_window_event, start_entitlement_checks};

#[cfg(not(windows))]
pub use none::{on_resume, on_window_event, start_entitlement_checks};

/// The twins for a system with no Microsoft Store: nothing to start, nothing to react to.
#[cfg(not(windows))]
mod none {
    use std::sync::Arc;

    use straft_application::AppCore;
    use tauri::WindowEvent;

    pub fn start_entitlement_checks(_core: &Arc<AppCore>) {}
    pub fn on_window_event(_event: &WindowEvent) {}
    pub fn on_resume() {}
}

#[cfg(windows)]
mod worker {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
    use std::sync::{Arc, OnceLock, Weak};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use straft_application::AppCore;
    use tauri::WindowEvent;
    use windows::Win32::System::SystemInformation::GetLocalTime;

    use super::{focus_needs_read, ms_until_midnight, next_wake, starts_worker};

    /// Why the worker is asked. Only a focus return is weighed against the last read.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Request {
        Launch,
        /// `StoreContext.OfflineLicensesChanged`.
        Changed,
        Focus,
        Resume,
    }

    static QUEUE: OnceLock<Sender<Request>> = OnceLock::new();

    /// Ask the worker for a read. Never blocks; does nothing before the worker starts.
    pub fn request(request: Request) {
        if let Some(queue) = QUEUE.get() {
            let _ = queue.send(request);
        }
    }

    /// Start the worker and the launch read. Called once, from setup, after the core exists.
    pub fn start_entitlement_checks(core: &Arc<AppCore>) {
        if !starts_worker(core.entitlement.channel()) {
            return;
        }
        let (tx, rx) = mpsc::channel();
        if QUEUE.set(tx).is_err() {
            return;
        }
        let core = Arc::downgrade(core);
        let spawned = std::thread::Builder::new()
            .name("straft-entitlement".into())
            .spawn(move || run(core, rx));
        if spawned.is_err() {
            tracing::warn!("the licence check thread could not start; the state stays as it is");
            return;
        }
        request(Request::Launch);
    }

    pub fn on_window_event(event: &WindowEvent) {
        if let WindowEvent::Focused(true) = event {
            request(Request::Focus);
        }
    }

    /// The computer woke from sleep.
    pub fn on_resume() {
        request(Request::Resume);
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .unwrap_or(0)
    }

    /// The next local midnight as a moment since 1970.
    fn next_local_midnight_ms(now_ms: i64) -> i64 {
        // SAFETY: GetLocalTime takes no input and returns a plain struct by value.
        let t = unsafe { GetLocalTime() };
        now_ms + ms_until_midnight(t.wHour, t.wMinute, t.wSecond, t.wMilliseconds)
    }

    fn run(core: Weak<AppCore>, rx: Receiver<Request>) {
        // (when on the monotonic clock, when on the wall clock) of the last read.
        let mut last_read: Option<(Instant, i64)> = None;
        let mut trial_ends_at_ms: Option<i64> = None;
        loop {
            let wait = match last_read {
                None => std::time::Duration::ZERO,
                Some((_, at)) => {
                    let now = now_ms();
                    next_wake(now, at, trial_ends_at_ms, next_local_midnight_ms(now))
                }
            };
            let due = match rx.recv_timeout(wait) {
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => true,
                Ok(first) => {
                    // One read answers everything that queued up while we waited or read.
                    let since = last_read.map(|(mono, _)| mono.elapsed());
                    std::iter::once(first)
                        .chain(rx.try_iter())
                        .any(|r| r != Request::Focus || focus_needs_read(true, since))
                }
            };
            if !due {
                continue;
            }
            let Some(core) = core.upgrade() else { return };
            // The licence call must never take the app down: a panic ends this read only.
            let read = catch_unwind(AssertUnwindSafe(|| {
                core.refresh_entitlement();
                core.entitlement.status().trial_ends_at_ms
            }));
            drop(core);
            match read {
                Ok(ends) => trial_ends_at_ms = ends,
                Err(_) => tracing::error!("the licence check failed unexpectedly"),
            }
            last_read = Some((Instant::now(), now_ms()));
        }
    }
}

#[cfg(windows)]
pub(crate) use worker::{Request, request};

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;
    const NOON: i64 = 12 * 60 * MIN;
    const MIDNIGHT: i64 = 24 * 60 * MIN;

    fn wake(now: i64, last: i64, ends: Option<i64>, midnight: i64) -> Duration {
        next_wake(now, last, ends, midnight)
    }

    #[test]
    fn the_regular_read_is_fifteen_minutes_after_the_last() {
        assert_eq!(wake(NOON, NOON, None, MIDNIGHT), REGULAR);
        assert_eq!(
            wake(NOON + 4 * MIN, NOON, None, MIDNIGHT),
            Duration::from_secs(11 * 60)
        );
    }

    #[test]
    fn midnight_wins_when_it_is_sooner() {
        let now = MIDNIGHT - 3 * MIN;
        assert_eq!(wake(now, now, None, MIDNIGHT), Duration::from_secs(3 * 60));
    }

    #[test]
    fn one_second_after_the_trial_ends_wins_when_it_is_sooner() {
        let ends = NOON + 2 * MIN;
        assert_eq!(
            wake(NOON, NOON, Some(ends), MIDNIGHT),
            Duration::from_millis(2 * 60_000 + 1_000)
        );
        // A trial end after the regular read changes nothing.
        assert_eq!(
            wake(NOON, NOON, Some(NOON + 3 * 60 * MIN), MIDNIGHT),
            REGULAR
        );
    }

    #[test]
    fn an_end_the_last_read_already_saw_pass_is_ignored() {
        let ends = NOON - MIN;
        assert_eq!(wake(NOON, NOON, Some(ends), MIDNIGHT), REGULAR);
    }

    #[test]
    fn the_wait_is_never_negative_and_never_more_than_regular() {
        assert_eq!(wake(NOON + 20 * MIN, NOON, None, MIDNIGHT), Duration::ZERO);
        assert_eq!(wake(NOON, NOON, None, MIDNIGHT + 99 * MIN), REGULAR);
        // The wall clock set back since the last read still waits at most one interval.
        assert_eq!(wake(NOON - 60 * MIN, NOON, None, MIDNIGHT), REGULAR);
        assert_eq!(
            wake(i64::MAX, i64::MIN, Some(i64::MAX), i64::MAX),
            Duration::ZERO
        );
    }

    #[test]
    fn focus_asks_for_a_read_only_after_five_minutes() {
        assert!(!focus_needs_read(false, None), "losing focus never reads");
        assert!(focus_needs_read(true, None), "no read yet");
        assert!(!focus_needs_read(true, Some(Duration::from_secs(60))));
        assert!(
            !focus_needs_read(true, Some(FOCUS_AFTER)),
            "exactly five minutes is not more"
        );
        assert!(focus_needs_read(
            true,
            Some(FOCUS_AFTER + Duration::from_secs(1))
        ));
    }

    #[test]
    fn midnight_is_counted_from_the_local_time_of_day() {
        assert_eq!(
            ms_until_midnight(0, 0, 0, 0),
            86_400_000,
            "at midnight, the next one"
        );
        assert_eq!(ms_until_midnight(23, 59, 59, 999), 1);
        assert_eq!(ms_until_midnight(12, 0, 0, 0), 12 * 60 * 60 * 1_000);
        assert_eq!(
            ms_until_midnight(99, 99, 99, 9999),
            1,
            "a bad value cannot go negative"
        );
    }

    #[test]
    fn only_the_microsoft_store_build_starts_a_worker() {
        assert!(starts_worker(Channel::MicrosoftStore));
        assert!(!starts_worker(Channel::DirectInstaller));
        assert!(!starts_worker(Channel::MacAppStore));
    }
}

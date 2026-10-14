//! When the Mac App Store build checks the licence and trial again.
//!
//! One background thread (the entitlement worker) runs every check, so StoreKit and the licence
//! server are never asked on the main thread and two checks never overlap. It checks:
//!
//! - at launch, right after the window is set up (the state is NeedsOnlineCheck until then);
//! - when the app resumes or is reopened, and when it wakes from sleep;
//! - when the window regains focus after more than [`LONG_ABSENCE`] without it;
//! - when StoreKit reports a transaction (a purchase, refund or revocation) or a storefront
//!   change, which is how another Apple Account signing in shows up;
//! - when the lease of the last answer runs out (`EntitlementService::lease`).
//!
//! The decisions here (focus, wake, debounce) are plain functions, tested on every system. The
//! thread and the Tauri hooks exist only on macOS. The log carries the reason and the state name,
//! never an identifier.

use std::time::{Duration, Instant};

/// Focus regained after this long counts as coming back to the app.
pub(crate) const LONG_ABSENCE: Duration = Duration::from_secs(30 * 60);

/// A resume this soon after the last check adds nothing (Tauri reports a resume at startup too).
pub(crate) const RESUME_DEBOUNCE: Duration = Duration::from_secs(60);

/// Wall time running ahead of the monotonic clock by more than this means the Mac slept. On
/// macOS `Instant` stops during sleep and the wall clock does not.
pub(crate) const SLEEP_GAP: Duration = Duration::from_secs(120);

/// Tracks how long the window has been without focus.
#[derive(Debug, Default)]
pub(crate) struct FocusTracker {
    unfocused_since: Option<Instant>,
}

impl FocusTracker {
    /// Record a focus change. True when focus came back after more than [`LONG_ABSENCE`].
    pub(crate) fn changed(&mut self, focused: bool, now: Instant) -> bool {
        if !focused {
            self.unfocused_since.get_or_insert(now);
            return false;
        }
        self.unfocused_since
            .take()
            .is_some_and(|since| now.saturating_duration_since(since) > LONG_ABSENCE)
    }
}

/// Whether the Mac slept between two worker ticks: the wall clock moved on further than the
/// monotonic clock. A wall clock set forward by hand looks the same and only costs one check.
pub(crate) fn slept(monotonic: Duration, wall: Option<Duration>) -> bool {
    wall.is_some_and(|wall| wall.saturating_sub(monotonic) > SLEEP_GAP)
}

/// Whether a resume request can be skipped because a check just ran.
pub(crate) fn resume_is_redundant(since_last_check: Option<Duration>) -> bool {
    since_last_check.is_some_and(|d| d < RESUME_DEBOUNCE)
}

#[cfg(target_os = "macos")]
pub use worker::{on_run_event, on_window_event, start_entitlement_checks};

#[cfg(target_os = "macos")]
mod worker {
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
    use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};
    use std::time::{Duration, Instant, SystemTime};

    use openframe_application::AppCore;
    use openframe_entitlement::mac::RevalidateReason;
    use tauri::{AppHandle, RunEvent, WindowEvent};

    use super::{FocusTracker, resume_is_redundant, slept};
    use crate::platform::macos_storekit::StoreKitFfi;
    use crate::platform::storekit::StoreUpdate;

    /// How often the worker wakes to look for a sleep it missed and for an expired lease.
    const TICK: Duration = Duration::from_secs(60);

    static QUEUE: OnceLock<Sender<RevalidateReason>> = OnceLock::new();
    static FOCUS: Mutex<FocusTracker> = Mutex::new(FocusTracker {
        unfocused_since: None,
    });

    /// Ask the worker for a check. Never blocks; does nothing before the worker starts.
    fn request(reason: RevalidateReason) {
        if let Some(queue) = QUEUE.get() {
            let _ = queue.send(reason);
        }
    }

    /// Start the worker, the StoreKit listener and the launch check. Called once, from setup.
    pub fn start_entitlement_checks(core: &Arc<AppCore>) {
        let (tx, rx) = mpsc::channel();
        if QUEUE.set(tx).is_err() {
            return;
        }
        let core = Arc::downgrade(core);
        let spawned = std::thread::Builder::new()
            .name("openframe-entitlement".into())
            .spawn(move || run(core, rx));
        if spawned.is_err() {
            tracing::warn!("the licence check thread could not start; the state stays as it is");
            return;
        }
        request(RevalidateReason::Launch);
    }

    /// `Resumed` and, on macOS, `Reopen` (the Dock icon was clicked).
    pub fn on_run_event(_app: &AppHandle, event: &RunEvent) {
        if matches!(event, RunEvent::Resumed | RunEvent::Reopen { .. }) {
            request(RevalidateReason::Resume);
        }
    }

    pub fn on_window_event(event: &WindowEvent) {
        if let WindowEvent::Focused(focused) = event {
            let back = FOCUS
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .changed(*focused, Instant::now());
            if back {
                request(RevalidateReason::Resume);
            }
        }
    }

    /// Forwarded from the Swift `Transaction.updates` and `Storefront.updates` listeners.
    fn on_store_update(update: Result<StoreUpdate, crate::platform::storekit::StoreKitError>) {
        let kind = match &update {
            Ok(StoreUpdate::Transaction(t)) if t.revocation_date_ms.is_some() => "revocation",
            Ok(StoreUpdate::Transaction(_)) => "transaction",
            Ok(StoreUpdate::Unverified) => "unverified",
            Ok(StoreUpdate::StorefrontChanged) => "storefront",
            Err(_) => "unreadable",
        };
        tracing::info!(kind, "StoreKit reported a change");
        request(RevalidateReason::StoreKitChanged);
    }

    fn run(core: Weak<AppCore>, rx: Receiver<RevalidateReason>) {
        if StoreKitFfi.start_updates(on_store_update).is_err() {
            tracing::warn!("StoreKit updates are not available; checks run on the other triggers");
        }
        let mut last_check: Option<Instant> = None;
        let mut lease_ends: Option<Instant> = None;
        let mut tick_mono = Instant::now();
        let mut tick_wall = SystemTime::now();
        loop {
            let wait = lease_ends
                .map_or(TICK, |end| end.saturating_duration_since(Instant::now()))
                .min(TICK);
            let asked = match rx.recv_timeout(wait) {
                Ok(reason) => Some(reason),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            };
            let now = Instant::now();
            let wall = SystemTime::now();
            let woke = slept(
                now.saturating_duration_since(tick_mono),
                wall.duration_since(tick_wall).ok(),
            );
            (tick_mono, tick_wall) = (now, wall);
            let reason = asked
                .or(woke.then_some(RevalidateReason::Resume))
                .or(lease_ends
                    .is_some_and(|end| now >= end)
                    .then_some(RevalidateReason::LeaseExpired));
            let Some(mut reason) = reason else { continue };
            // Requests that queued up (for example during the last check) are answered by this
            // one check. Any reason other than a resume is kept, so it is never debounced away.
            for queued in rx.try_iter() {
                if reason == RevalidateReason::Resume {
                    reason = queued;
                }
            }
            if reason == RevalidateReason::Resume
                && resume_is_redundant(last_check.map(|t| now.saturating_duration_since(t)))
            {
                continue;
            }
            let Some(core) = core.upgrade() else { return };
            core.entitlement.revalidate(reason);
            let done = Instant::now();
            last_check = Some(done);
            lease_ends = core.entitlement.lease().map(|lease| done + lease);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_back_after_a_long_absence_triggers_a_check_and_short_ones_do_not() {
        let t0 = Instant::now();
        let mut f = FocusTracker::default();
        assert!(!f.changed(true, t0), "focus without a loss first");
        assert!(!f.changed(false, t0));
        assert!(
            !f.changed(false, t0 + Duration::from_secs(10)),
            "first loss counts"
        );
        assert!(
            !f.changed(true, t0 + LONG_ABSENCE),
            "exactly 30 minutes is not more"
        );
        assert!(!f.changed(false, t0));
        assert!(f.changed(true, t0 + LONG_ABSENCE + Duration::from_secs(1)));
        assert!(!f.changed(true, t0 + 2 * LONG_ABSENCE), "already counted");
    }

    #[test]
    fn a_wall_clock_far_ahead_of_the_monotonic_clock_means_the_mac_slept() {
        let minute = Duration::from_secs(60);
        assert!(!slept(minute, Some(minute)));
        assert!(!slept(minute, Some(minute + SLEEP_GAP)));
        assert!(slept(
            minute,
            Some(minute + SLEEP_GAP + Duration::from_secs(1))
        ));
        assert!(!slept(minute, Some(Duration::ZERO)), "a clock set back");
        assert!(!slept(minute, None), "a clock set before the last tick");
    }

    #[test]
    fn a_resume_right_after_a_check_is_skipped() {
        assert!(!resume_is_redundant(None), "no check yet");
        assert!(resume_is_redundant(Some(Duration::from_secs(5))));
        assert!(!resume_is_redundant(Some(RESUME_DEBOUNCE)));
    }
}

//! Window close handshake (Windows phase W2, "protect writers' work").
//!
//! The close button, Alt+F4 and the system all reach the window as a close request. The first
//! request is held: the page is asked to store the text its editors still hold
//! (`of://close-requested`, answered by `apps/desktop/src/app/closeRequest.ts` through the
//! `of_close_window` command). The page answers "flushing" at once, then "close" when the text is
//! stored, or "held" when the user chose to stay. A page that never answers (stuck or crashed)
//! must not keep the app open, so the window closes by itself after [`ANSWER_WAIT`].

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};
use std::time::Duration;

/// The event that asks the page to store pending text.
pub const EVENT: &str = "of://close-requested";

/// How long the page has to answer before the window closes anyway.
pub const ANSWER_WAIT: Duration = Duration::from_secs(3);

/// What to do with one close request.
#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    /// Let the window close (the page said it may, or it never answered).
    Close,
    /// Keep the window open. `ask` is true for the first request of a round: send the event and
    /// start the fallback timer for `round`.
    Hold { ask: bool, round: u64 },
}

#[derive(Default)]
pub struct CloseGate {
    released: AtomicBool,
    asked: AtomicBool,
    answered: AtomicBool,
    round: AtomicU64,
}

impl CloseGate {
    /// A close request arrived.
    pub fn on_request(&self) -> Request {
        if self.released.load(SeqCst) {
            return Request::Close;
        }
        let ask = !self.asked.swap(true, SeqCst);
        if ask {
            self.answered.store(false, SeqCst);
        }
        Request::Hold {
            ask,
            round: self.round.load(SeqCst),
        }
    }

    /// The page received the request and is storing its text.
    pub fn answered(&self) {
        self.answered.store(true, SeqCst);
    }

    /// The user chose to stay: the window stays open and the next request starts a new round.
    pub fn held(&self) {
        self.round.fetch_add(1, SeqCst);
        self.asked.store(false, SeqCst);
    }

    /// The window may close now.
    pub fn release(&self) {
        self.released.store(true, SeqCst);
    }

    /// The fallback timer of `round` ran out: close when the page never answered that round.
    pub fn should_force(&self, round: u64) -> bool {
        !self.released.load(SeqCst)
            && self.asked.load(SeqCst)
            && !self.answered.load(SeqCst)
            && self.round.load(SeqCst) == round
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_request_asks_the_page_and_later_ones_wait() {
        let g = CloseGate::default();
        assert_eq!(
            g.on_request(),
            Request::Hold {
                ask: true,
                round: 0
            }
        );
        // A second click on the close button while the page saves does not ask again.
        assert_eq!(
            g.on_request(),
            Request::Hold {
                ask: false,
                round: 0
            }
        );
        g.answered();
        g.release();
        assert_eq!(g.on_request(), Request::Close);
    }

    #[test]
    fn a_page_that_never_answers_cannot_keep_the_window_open() {
        let g = CloseGate::default();
        let Request::Hold { round, .. } = g.on_request() else {
            panic!("held")
        };
        assert!(g.should_force(round));
        g.answered();
        assert!(
            !g.should_force(round),
            "a page that answered is given time to save"
        );
    }

    #[test]
    fn staying_starts_a_new_round_and_the_old_timer_does_nothing() {
        let g = CloseGate::default();
        let Request::Hold { round: first, .. } = g.on_request() else {
            panic!("held")
        };
        g.answered();
        g.held();
        assert!(!g.should_force(first), "the user chose to stay");
        assert_eq!(
            g.on_request(),
            Request::Hold {
                ask: true,
                round: 1
            }
        );
        assert!(
            !g.should_force(first),
            "a timer from the earlier round never closes the window"
        );
        assert!(g.should_force(1));
    }
}

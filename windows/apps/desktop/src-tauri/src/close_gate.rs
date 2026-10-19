//! Window close handshake (Windows phase W2, "protect writers' work").
//!
//! The close button, Alt+F4 and the system all reach the window as a close request. The first
//! request is held: the page is asked to store the text its editors still hold
//! (`of://close-requested`, answered by `apps/desktop/src/app/closeRequest.ts` through the
//! `of_close_window` command). The page answers "flushing" at once, then "close" when the text is
//! stored, or "held" when the user chose to stay. A page that never answers (stuck or crashed)
//! must not keep the app open: the window closes by itself after [`ANSWER_WAIT`] without an
//! answer, or [`FLUSH_WAIT`] after "flushing" without a "close" or "held". The page's own saves
//! give up after 4 s and then ask the user, so these waits only end a page that has stopped.
//!
//! One lock holds the whole state, so a forced close can never override the user's "stay".

use std::time::Duration;

use parking_lot::Mutex;

/// The event that asks the page to store pending text.
pub const EVENT: &str = "of://close-requested";

/// How long the page has to answer the request at all. Long enough for a busy page to get to it.
pub const ANSWER_WAIT: Duration = Duration::from_secs(10);

/// How long the page has, once it answered, to finish storing and say "close" or "held".
pub const FLUSH_WAIT: Duration = Duration::from_secs(30);

/// What to do with one close request.
#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    /// Let the window close (the page said it may, or it stopped answering).
    Close,
    /// Keep the window open. `ask` is true for the first request of a round: send the event and
    /// start the fallback timer for `round`.
    Hold { ask: bool, round: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Asked,
    Flushing,
    Released,
}

#[derive(Debug)]
struct State {
    phase: Phase,
    round: u64,
}

#[derive(Debug)]
pub struct CloseGate(Mutex<State>);

impl Default for CloseGate {
    fn default() -> Self {
        Self(Mutex::new(State {
            phase: Phase::Idle,
            round: 0,
        }))
    }
}

impl CloseGate {
    /// A close request arrived.
    pub fn on_request(&self) -> Request {
        let mut s = self.0.lock();
        match s.phase {
            Phase::Released => Request::Close,
            Phase::Idle => {
                s.phase = Phase::Asked;
                Request::Hold {
                    ask: true,
                    round: s.round,
                }
            }
            Phase::Asked | Phase::Flushing => Request::Hold {
                ask: false,
                round: s.round,
            },
        }
    }

    /// The page received the request and is storing its text.
    pub fn answered(&self) {
        let mut s = self.0.lock();
        if s.phase == Phase::Asked {
            s.phase = Phase::Flushing;
        }
    }

    /// The user chose to stay: the window stays open and the next request starts a new round.
    pub fn held(&self) {
        let mut s = self.0.lock();
        if s.phase != Phase::Released {
            s.round += 1;
            s.phase = Phase::Idle;
        }
    }

    /// The window may close now.
    pub fn release(&self) {
        self.0.lock().phase = Phase::Released;
    }

    /// Closing failed after [`CloseGate::release`]: the next request goes through the page again.
    pub fn unrelease(&self) {
        let mut s = self.0.lock();
        if s.phase == Phase::Released {
            s.round += 1;
            s.phase = Phase::Idle;
        }
    }

    /// A fallback timer of `round` ran out. Releases and returns true when the page still has
    /// not answered (`waited_for_answer`) or, after answering, still has not finished; returns
    /// false when the user chose to stay, a new round began, or the window is already closing.
    pub fn try_force(&self, round: u64, waited_for_answer: bool) -> bool {
        let mut s = self.0.lock();
        let stuck = s.round == round
            && match s.phase {
                Phase::Asked => true,
                Phase::Flushing => !waited_for_answer,
                Phase::Idle | Phase::Released => false,
            };
        if stuck {
            s.phase = Phase::Released;
        }
        stuck
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
        assert!(g.try_force(round, true));
        assert_eq!(g.on_request(), Request::Close);
    }

    #[test]
    fn a_page_that_answered_gets_the_longer_wait_then_cannot_keep_the_window_open() {
        let g = CloseGate::default();
        let Request::Hold { round, .. } = g.on_request() else {
            panic!("held")
        };
        g.answered();
        assert!(
            !g.try_force(round, true),
            "a page that answered is given time to save"
        );
        assert!(
            g.try_force(round, false),
            "a page stuck after answering is closed"
        );
    }

    #[test]
    fn staying_starts_a_new_round_and_the_old_timers_do_nothing() {
        let g = CloseGate::default();
        let Request::Hold { round: first, .. } = g.on_request() else {
            panic!("held")
        };
        g.answered();
        g.held();
        assert!(!g.try_force(first, true), "the user chose to stay");
        assert!(!g.try_force(first, false), "the user chose to stay");
        assert_eq!(
            g.on_request(),
            Request::Hold {
                ask: true,
                round: 1
            }
        );
        assert!(
            !g.try_force(first, true),
            "a timer from the earlier round never closes the window"
        );
        assert!(g.try_force(1, true));
    }

    #[test]
    fn a_close_that_failed_goes_through_the_page_again() {
        let g = CloseGate::default();
        g.on_request();
        g.answered();
        g.release();
        g.unrelease();
        assert_eq!(
            g.on_request(),
            Request::Hold {
                ask: true,
                round: 1
            }
        );
    }
}

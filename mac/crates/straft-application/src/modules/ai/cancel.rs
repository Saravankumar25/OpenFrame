//! Stopping a running request (`ai.cancel`; harness plan 5.15 and 7).
//!
//! `ai.ask` registers a flag for its conversation while it runs and removes it when it ends, by any
//! road. `ai.cancel` sets the flag of the person's own running conversation. The request looks at
//! the flag between steps and calls: a model call already running finishes or times out as usual,
//! and the request then stops at the next check point. Neither operation holds a lock the other
//! needs, so a stop can arrive while the model is working.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

use parking_lot::Mutex;

type Key = (String, String);

static RUNNING: LazyLock<Mutex<HashMap<Key, Arc<AtomicBool>>>> = LazyLock::new(Default::default);

/// The registration of one running request; dropping it removes the flag.
pub(super) struct Running {
    key: Key,
    flag: Arc<AtomicBool>,
}

impl Running {
    pub fn stopped(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let mut map = RUNNING.lock();
        // A newer request of the same conversation may have taken the place; leave it.
        if map
            .get(&self.key)
            .is_some_and(|f| Arc::ptr_eq(f, &self.flag))
        {
            map.remove(&self.key);
        }
    }
}

/// Register a request of `conversation` for `user`.
pub(super) fn begin(user: &str, conversation: &str) -> Running {
    let key = (user.to_string(), conversation.to_string());
    let flag = Arc::new(AtomicBool::new(false));
    RUNNING.lock().insert(key.clone(), flag.clone());
    Running { key, flag }
}

/// Ask the running request of this conversation to stop. False when none is running.
pub(super) fn stop(user: &str, conversation: &str) -> bool {
    match RUNNING
        .lock()
        .get(&(user.to_string(), conversation.to_string()))
    {
        Some(flag) => {
            flag.store(true, Ordering::SeqCst);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stop_reaches_only_the_running_request_of_that_person_and_conversation() {
        let run = begin("u1", "c1");
        assert!(!run.stopped());
        assert!(!stop("u2", "c1"), "another person's conversation");
        assert!(!stop("u1", "c2"), "another conversation");
        assert!(!run.stopped());
        assert!(stop("u1", "c1"));
        assert!(run.stopped());
    }

    #[test]
    fn the_flag_is_removed_when_the_request_ends_however_it_ends() {
        {
            let _run = begin("u1", "c-gone");
            assert!(stop("u1", "c-gone"));
        }
        assert!(!stop("u1", "c-gone"));
        // A newer request of the same conversation keeps its own flag when the older one ends.
        let old = begin("u1", "c-twice");
        let new = begin("u1", "c-twice");
        drop(old);
        assert!(stop("u1", "c-twice"));
        assert!(new.stopped());
    }
}

//! Team password gate for the internal builds (ADR-0024; build plan area 20 box "Team password
//! gate").
//!
//! The internal Windows installer and the internal Mac disk image ask the team password on every
//! launch, before anything opens. The build embeds only an Argon2id hash with a random salt
//! ([`build_hash`]); the password itself is never in the source, the repository, the logs or the
//! built app. Store packages carry neither the gate nor a hash: the desktop shell builds a
//! [`TeamGate`] only when its build script embedded a hash.
//!
//! Wrong tries: after 5 wrong passwords in a row the next try waits 1 minute; every further run
//! of 5 wrong tries doubles the wait, up to 30 minutes ([`wait_after`]). The count is saved in
//! `app_data_dir/entitlement/team-gate.json` and survives a restart. It is saved *before* the
//! password is checked, so closing or killing the app during the check cannot undo a wrong try.
//! A right password resets it. A damaged count file starts at the first wait instead of at zero.
//! A clock turned back never makes a wait longer than its own length: the wait starts again from
//! the earlier time, so it still ends after its own length of real time. A clock turned forward
//! ends a wait early, which is accepted: the gate slows casual use of a shared copy, and ADR-0024
//! says the hash can be guessed offline by a person who holds the file.

pub mod build_hash;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub use build_hash::{
    GatePlan, MAX_PASSWORD_BYTES, TEAM_PASSWORD_ENV, build_plan, hash_for_build, hash_with_costs,
    verify_team_password,
};

/// Wrong tries allowed before a wait.
pub const TRIES_BEFORE_WAIT: u32 = 5;
/// The first wait, after 5 wrong tries.
pub const FIRST_WAIT_MS: i64 = 60 * 1000;
/// The longest wait.
pub const MAX_WAIT_MS: i64 = 30 * 60 * 1000;
/// The count file is a few dozen bytes; anything larger is damaged.
const MAX_FILE_BYTES: u64 = 4096;

/// How long the next try waits after `failures` wrong tries in a row: nothing until a run of 5 is
/// complete, then 1 minute, 2, 4, 8, 16 and 30 minutes from then on.
pub fn wait_after(failures: u32) -> i64 {
    if failures == 0 || !failures.is_multiple_of(TRIES_BEFORE_WAIT) {
        return 0;
    }
    let doublings = (failures / TRIES_BEFORE_WAIT - 1).min(16);
    (FIRST_WAIT_MS << doublings).min(MAX_WAIT_MS)
}

/// The saved count of wrong tries and the wait they started.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GateRecord {
    version: u32,
    failures: u32,
    /// When the current wait started, by the computer's clock.
    wait_started_ms: Option<i64>,
    /// Length of the current wait.
    wait_ms: i64,
}

impl GateRecord {
    fn after_wrong_try(&self, now_ms: i64) -> GateRecord {
        let failures = self.failures.saturating_add(1);
        let wait = wait_after(failures);
        GateRecord {
            version: 1,
            failures,
            wait_started_ms: (wait > 0).then_some(now_ms),
            wait_ms: wait,
        }
    }

    /// Milliseconds left of the current wait at `now_ms`, never more than the wait's length.
    fn wait_left(&self, now_ms: i64) -> i64 {
        match self.wait_started_ms {
            None => 0,
            Some(start) if now_ms < start => self.wait_ms,
            Some(start) => (self.wait_ms - (now_ms - start)).clamp(0, self.wait_ms),
        }
    }

    /// A clock turned back to before the wait started: start the wait again at `now_ms`, so it
    /// ends after its own length of real time instead of when the clock catches up. Returns true
    /// when the record changed and must be saved.
    fn restart_wait_if_clock_went_back(&mut self, now_ms: i64) -> bool {
        match self.wait_started_ms {
            Some(start) if now_ms < start && self.wait_ms > 0 => {
                self.wait_started_ms = Some(now_ms);
                true
            }
            _ => false,
        }
    }

    fn tries_before_wait(&self) -> u32 {
        TRIES_BEFORE_WAIT - self.failures % TRIES_BEFORE_WAIT
    }
}

/// What the lock screen shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateView {
    pub unlocked: bool,
    /// Milliseconds until the next try is allowed; 0 when a try is allowed now.
    pub wait_ms: i64,
    /// Wrong tries left before the next wait.
    pub tries_before_wait: u32,
}

/// The answer to one try.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockAnswer {
    /// The password is right; the app opens.
    Unlocked,
    /// The password is wrong. `wait_ms` is the wait before the next try (0 when there is none).
    Wrong {
        tries_before_wait: u32,
        wait_ms: i64,
    },
    /// A wait is running; the password was not checked and the try does not count.
    Waiting { wait_ms: i64 },
}

/// The gate of one running app. Unlocked lives in memory only, so every launch asks again.
#[derive(Debug)]
pub struct TeamGate {
    hash: String,
    path: PathBuf,
    record: Mutex<GateRecord>,
    unlocked: AtomicBool,
}

impl TeamGate {
    /// The gate over the embedded hash, with the count saved in `app_data_dir`. Never fails: a
    /// hash that cannot be read matches no password, so the gate stays shut.
    pub fn new(hash: impl Into<String>, app_data_dir: &Path, now_ms: i64) -> Self {
        let path = app_data_dir.join("entitlement").join("team-gate.json");
        let record = load(&path, now_ms);
        Self {
            hash: hash.into(),
            path,
            record: Mutex::new(record),
            unlocked: AtomicBool::new(false),
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.unlocked.load(Ordering::SeqCst)
    }

    pub fn view(&self, now_ms: i64) -> GateView {
        let mut record = self.record.lock();
        if record.restart_wait_if_clock_went_back(now_ms) {
            self.save(&record);
        }
        GateView {
            unlocked: self.is_unlocked(),
            wait_ms: record.wait_left(now_ms),
            tries_before_wait: record.tries_before_wait(),
        }
    }

    /// One try. Tries are serialised; the count is saved before the slow check runs.
    pub fn try_unlock(&self, password: &str, now_ms: i64) -> UnlockAnswer {
        if self.is_unlocked() {
            return UnlockAnswer::Unlocked;
        }
        let mut record = self.record.lock();
        if record.restart_wait_if_clock_went_back(now_ms) {
            self.save(&record);
        }
        let wait = record.wait_left(now_ms);
        if wait > 0 {
            return UnlockAnswer::Waiting { wait_ms: wait };
        }
        let counted = record.after_wrong_try(now_ms);
        self.save(&counted);
        if verify_team_password(&self.hash, password) {
            *record = GateRecord {
                version: 1,
                ..GateRecord::default()
            };
            self.save(&record);
            self.unlocked.store(true, Ordering::SeqCst);
            return UnlockAnswer::Unlocked;
        }
        *record = counted;
        UnlockAnswer::Wrong {
            tries_before_wait: record.tries_before_wait(),
            wait_ms: record.wait_left(now_ms),
        }
    }

    /// Save the count with the shared atomic replace. A failure is logged; the count still holds
    /// in memory for this launch.
    fn save(&self, record: &GateRecord) {
        let result = (|| -> std::io::Result<()> {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_vec(record)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            openframe_platform::atomic::replace_bytes(&self.path, &json)
        })();
        if let Err(e) = result {
            tracing::warn!(kind = ?e.kind(), "the team password try count could not be saved");
        }
    }
}

/// Read the saved count. Missing means no wrong tries yet. Anything else that cannot be read
/// (locked, too large, not a record) starts at the first wait, never at zero, so damaging the
/// file does not give tries back.
fn load(path: &Path, now_ms: i64) -> GateRecord {
    let damaged = || GateRecord {
        version: 1,
        failures: TRIES_BEFORE_WAIT,
        wait_started_ms: Some(now_ms),
        wait_ms: FIRST_WAIT_MS,
    };
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return GateRecord {
                version: 1,
                ..GateRecord::default()
            };
        }
        Err(_) => return damaged(),
    };
    let mut bytes = Vec::with_capacity(128);
    match file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes) {
        Ok(n) if n as u64 <= MAX_FILE_BYTES => {}
        _ => return damaged(),
    }
    match serde_json::from_slice::<GateRecord>(&bytes) {
        Ok(r) if r.version == 1 && r.wait_ms >= 0 && r.wait_ms <= MAX_WAIT_MS => r,
        _ => damaged(),
    }
}

#[cfg(test)]
mod tests;

//! The Mac trial's clock guard record (ADR-0040, rules G1 to G4).
//!
//! The trial is counted on this Mac from Apple-signed StoreKit data and the Mac's clock, so a clock
//! turned back must not give days back. One small record remembers the latest time the app has
//! trusted. The Swift shell stores it in the data protection Keychain (a file in the container is
//! easy to restore from a backup; the Keychain item survives deleting and reinstalling the app) and
//! the core reads and writes it through the StoreKit bridge.
//!
//! The record never unlocks anything on its own account, except that a buyer stays unlocked while
//! the App Store cannot be reached (`lifetime_seen`, rule 6). A person who deletes the item, or uses
//! a second Apple Account, gets more trial time; the owner accepts this (no device binding).
//!
//! Reading is tolerant by design: missing fields get defaults, unknown fields are ignored, and a
//! record that cannot be read at all is treated as empty and counted as an anomaly. An empty record
//! can only lose the memory of the clock; it can never grant a purchase.

use serde::{Deserialize, Serialize};

use super::bridge::StoreEnvironment;

/// The current record version.
pub const GUARD_VERSION: u32 = 1;

/// Largest stored record. A larger one is treated as corrupt.
pub const GUARD_MAX_BYTES: usize = 4096;

/// What the Mac remembers about the trial and the clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GuardRecord {
    pub v: u32,
    /// The store environment that wrote the record. A record from another environment (a TestFlight
    /// build before the real one, for example) is not used, so test and real purchases never mix.
    pub env: Option<StoreEnvironment>,
    /// The trial start in Unix milliseconds UTC. Never moves later.
    pub trial_start_ms: Option<i64>,
    /// The original transaction id of the trial transaction that set the start.
    pub trial_original_tx: Option<String>,
    /// A verified lifetime purchase was seen on this Mac and not refunded since.
    pub lifetime_seen: bool,
    /// The latest wall clock reading the app has trusted, Unix milliseconds UTC.
    pub high_water_ms: i64,
    /// The latest time Apple signed a transaction this app read, Unix milliseconds UTC.
    pub last_apple_signed_ms: i64,
}

impl Default for GuardRecord {
    fn default() -> Self {
        Self {
            v: GUARD_VERSION,
            env: None,
            trial_start_ms: None,
            trial_original_tx: None,
            lifetime_seen: false,
            high_water_ms: 0,
            last_apple_signed_ms: 0,
        }
    }
}

/// A record after [`GuardRecord::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGuard {
    pub record: GuardRecord,
    /// True when a stored value could not be read and the record is empty as a result.
    pub corrupt: bool,
}

impl GuardRecord {
    /// Read a stored record. `None` is a first run. Never fails.
    pub fn parse(stored: Option<&str>) -> ParsedGuard {
        let Some(text) = stored else {
            return ParsedGuard {
                record: Self::default(),
                corrupt: false,
            };
        };
        let parsed = (text.len() <= GUARD_MAX_BYTES)
            .then(|| serde_json::from_str::<Self>(text).ok())
            .flatten();
        match parsed {
            Some(record) => ParsedGuard {
                record: record.sanitized(),
                corrupt: false,
            },
            None => ParsedGuard {
                record: Self::default(),
                corrupt: true,
            },
        }
    }

    /// Text for the Keychain.
    pub fn to_stored(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }

    /// Values that make no sense are dropped, so a damaged record cannot move a date backwards.
    /// A record from a later version keeps its version number: only its known fields are used.
    fn sanitized(mut self) -> Self {
        self.v = self.v.max(GUARD_VERSION);
        self.trial_start_ms = self.trial_start_ms.filter(|t| *t > 0);
        self.trial_original_tx = self
            .trial_original_tx
            .filter(|t| !t.is_empty() && t.len() <= super::bridge::MAX_ID_BYTES && t.is_ascii());
        self.high_water_ms = self.high_water_ms.max(0);
        self.last_apple_signed_ms = self.last_apple_signed_ms.max(0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_record_is_a_first_run_and_not_an_anomaly() {
        let p = GuardRecord::parse(None);
        assert_eq!(p.record, GuardRecord::default());
        assert!(!p.corrupt);
    }

    #[test]
    fn the_record_round_trips() {
        let r = GuardRecord {
            trial_start_ms: Some(1_000),
            trial_original_tx: Some("2000000001".into()),
            lifetime_seen: true,
            high_water_ms: 5_000,
            last_apple_signed_ms: 4_000,
            ..GuardRecord::default()
        };
        let p = GuardRecord::parse(r.to_stored().as_deref());
        assert_eq!(p.record, r);
        assert!(!p.corrupt);
    }

    #[test]
    fn missing_fields_get_defaults_and_unknown_fields_are_ignored() {
        let p = GuardRecord::parse(Some(r#"{"high_water_ms": 9, "later": {"x": 1}}"#));
        assert_eq!(p.record.high_water_ms, 9);
        assert_eq!(p.record.v, GUARD_VERSION);
        assert!(!p.corrupt);
    }

    #[test]
    fn a_later_version_keeps_its_known_fields_and_its_version() {
        let p = GuardRecord::parse(Some(
            r#"{"v": 7, "trial_start_ms": 11, "lifetime_seen": true, "new": "x"}"#,
        ));
        assert_eq!(p.record.v, 7);
        assert_eq!(p.record.trial_start_ms, Some(11));
        assert!(p.record.lifetime_seen);
        assert!(!p.corrupt);
    }

    #[test]
    fn truncated_garbage_and_oversized_records_read_as_empty_and_count_as_an_anomaly() {
        let big = format!(r#"{{"pad":"{}"}}"#, "x".repeat(GUARD_MAX_BYTES));
        for bad in [
            r#"{"high_water_ms": 9"#,
            "not json",
            "[1,2,3]",
            r#"{"high_water_ms": "soon"}"#,
            big.as_str(),
        ] {
            let p = GuardRecord::parse(Some(bad));
            assert_eq!(p.record, GuardRecord::default(), "{bad}");
            assert!(p.corrupt, "{bad}");
        }
    }

    #[test]
    fn nonsense_values_are_dropped() {
        let p = GuardRecord::parse(Some(
            r#"{"trial_start_ms": -5, "trial_original_tx": "", "high_water_ms": -1, "last_apple_signed_ms": -9}"#,
        ));
        assert_eq!(p.record, GuardRecord::default());
        assert!(!p.corrupt);
    }
}

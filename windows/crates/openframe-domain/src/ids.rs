//! Stable opaque identities (Domain spec §2).
//!
//! Identities are UUIDv7 strings: globally unique (safe across exchange
//! packages exchanged between computers), time-sortable for stable tie-breaking,
//! and never derived from display numbers.

/// Canonical identity type. Stored as TEXT in SQLite and serialized as a string.
pub type Id = String;

/// Create a new stable identity.
pub fn new_id() -> Id {
    uuid::Uuid::now_v7().to_string()
}

/// Validate that a string is a well-formed identity (defends IPC/package inputs).
pub fn is_valid_id(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok()
}

/// Milliseconds since the Unix epoch (UTC). All persisted timestamps use this.
pub fn now_ms() -> i64 {
    let now = time::OffsetDateTime::now_utc();
    (now.unix_timestamp_nanos() / 1_000_000) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_valid() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        assert!(is_valid_id(&a));
        assert!(!is_valid_id("scene-12"));
    }

    #[test]
    fn ids_sort_by_creation_time() {
        let a = new_id();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = new_id();
        assert!(a < b);
    }
}

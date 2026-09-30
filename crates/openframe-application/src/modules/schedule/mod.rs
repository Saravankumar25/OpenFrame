//! Shooting Schedule, Daily View, Call Sheets, Sides, Reports and Budget
//! (FSD §35–39, §56–58, §104–105, §109–111, §125–126; FSD-SCH-*, FSD-CALL-*).
//!
//! Tables: migrations/project/0007_schedule.sql. Reads hub tables owned by the
//! screenplay and production modules (read-only; never rewrites them).
//!
//! Submodules:
//! - `source`    — read-only projection of the Production Source (scenes, pages, cast, locations)
//! - `board`     — the in-memory schedule (days, strips, markers), derived values and warnings
//! - `ops`       — `schedule.*` operations and the daily view
//! - `callsheet` — `callsheets.*` operations
//! - `docs`      — `sides.*` and `reports.*`
//! - `budget`    — `budget.*`

pub mod board;
pub mod budget;
pub mod callsheet;
pub mod docs;
pub mod fmt;
pub mod ops;
pub mod source;

use crate::registry::Registry;

pub fn register(r: &mut Registry) {
    ops::register(r);
    callsheet::register(r);
    docs::register(r);
    budget::register(r);
}

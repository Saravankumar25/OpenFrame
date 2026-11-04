//! Shooting Schedule, Daily View, Day Out of Days, Call Sheets, Sides, Reports and Budget
//! (FSD sections 35 to 39, 56 to 58, 104 to 105, 109 to 111, 125 to 126; FSD-SCH-*, FSD-CALL-*;
//! Screenplay and Production Depth spec sections 25 to 28).
//!
//! Tables: migrations/project/0007_schedule.sql and 0015_schedule_timeline.sql. Reads hub
//! tables owned by the screenplay and production modules (read-only; never rewrites them).
//!
//! Submodules:
//! - `source`: read-only projection of the Production Source (scenes, pages, cast, locations)
//! - `board`: the in-memory schedule (days, strips, markers), derived values and warnings
//! - `timeline`: the derived day timeline (crew call, starts, breaks, estimated wrap)
//! - `availability`: dated cast and location availability as the schedule reads it
//! - `ops`: `schedule.*` operations and the daily view
//! - `dood`: the Day Out of Days report and its manual corrections
//! - `callsheet`: `callsheets.*` operations
//! - `docs`: `sides.*` and `reports.*`
//! - `budget`: `budget.*`

pub mod availability;
pub mod board;
pub mod budget;
pub mod callsheet;
pub mod callsheet_source;
pub mod docs;
pub mod dood;
pub mod fmt;
pub mod ops;
pub mod source;
pub mod timeline;

use crate::registry::Registry;

pub fn register(r: &mut Registry) {
    ops::register(r);
    dood::register(r);
    callsheet::register(r);
    docs::register(r);
    budget::register(r);
}

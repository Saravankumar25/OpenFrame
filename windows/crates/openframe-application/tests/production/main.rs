//! Production: breakdown and catalog, schedule, exports, PDF, packages.
//!
//! One test binary per area instead of one per file: each binary links the whole application
//! crate, so fewer binaries build faster and use less disk (Windows phase W1, storage hygiene).
//! Run one file with `cargo test -p openframe-application --test production <file>::`.

mod exports;
mod packages;
mod pdf_regression;
mod production;
mod production_depth;
mod schedule;
mod schedule_depth;

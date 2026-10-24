//! Production: breakdown and catalog, schedule, exports, PDF, packages.
//!
//! One test binary per area instead of one per file: each binary links the whole application
//! crate, so fewer binaries build faster and use less disk (Windows phase W1, storage hygiene).
//! Run one file with `cargo test -p openframe-application --test production <file>::`.

mod breakdown_design;
mod call_sheet_design;
mod daily_view_design;
mod dood_design;
mod exports;
mod packages;
mod pdf_design;
mod pdf_regression;
mod pdf_samples;
mod production;
mod production_depth;
mod schedule;
mod schedule_depth;
mod schedule_design;

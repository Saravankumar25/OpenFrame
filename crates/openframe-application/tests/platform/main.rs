//! Platform: pipeline, migrations, failure injection, recovery, security review, entitlement and trial.
//!
//! One test binary per area instead of one per file: each binary links the whole application
//! crate, so fewer binaries build faster and use less disk (Windows phase W1, storage hygiene).
//! Run one file with `cargo test -p openframe-application --test platform <file>::`.

mod core_pipeline;
mod diagnostics;
mod e2e_flows;
mod entitlement;
mod failure_injection;
mod migration_failures;
mod migration_fixtures;
mod project_lifecycle;
mod property_invariants;
mod recently_deleted_purge;
mod security_review;
mod soak;
mod team_gate;
mod trial_first_run;

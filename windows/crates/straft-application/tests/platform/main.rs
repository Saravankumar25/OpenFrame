//! Platform: pipeline, migrations, failure injection, recovery, security review, entitlement and trial.
//!
//! One test binary per area instead of one per file: each binary links the whole application
//! crate, so fewer binaries build faster and use less disk (Windows phase W1, storage hygiene).
//! Run one file with `cargo test -p straft-application --test platform <file>::`.

mod config_files;
mod core_pipeline;
mod diagnostics;
mod e2e_flows;
mod entitlement;
mod failure_injection;
mod library_setup;
mod migration_failures;
mod migration_fixtures;
mod permission_matrix;
mod project_lifecycle;
mod property_invariants;
mod recently_deleted_purge;
mod rename_migration;
mod security_review;
mod soak;
mod storage;
mod team_gate;
mod trial_first_run;
mod update_from_release_baseline;
mod vault_recover;

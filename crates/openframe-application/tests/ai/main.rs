//! Offline AI: assistant, agent, retrieval, toolbox, stale proposals and the real-model checks.
//!
//! One test binary per area instead of one per file: each binary links the whole application
//! crate, so fewer binaries build faster and use less disk (Windows phase W1, storage hygiene).
//! Run one file with `cargo test -p openframe-application --test ai <file>::`.

mod ai;
mod ai_agent;
mod ai_apply_recovery;
mod ai_index_stress;
mod ai_lanes;
mod ai_real_model;
mod ai_retrieval;
mod ai_retrieval_perf;
mod ai_stale_proposals;
mod ai_tool_coverage;
mod ai_toolbox;

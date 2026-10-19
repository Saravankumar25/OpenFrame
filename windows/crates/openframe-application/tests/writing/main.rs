//! Writing: Idea Vault, Story, Screenplay, notes, comments, visual planning, appearance and interchange.
//!
//! One test binary per area instead of one per file: each binary links the whole application
//! crate, so fewer binaries build faster and use less disk (Windows phase W1, storage hygiene).
//! Run one file with `cargo test -p openframe-application --test writing <file>::`.

mod appearance;
mod comments;
mod idea_vault;
mod interchange;
mod notes;
mod screenplay;
mod screenplay_delete;
mod screenplay_engine;
mod screenplay_formats;
mod story;
mod story_cards;
mod vault_research;
mod visual;
mod writing_comfort;

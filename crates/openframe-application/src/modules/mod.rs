//! Module registry. Each module registers its operations here.

use crate::registry::Registry;

pub mod ai;
pub mod appearance;
pub mod comments;
pub mod diagnostics;
pub mod entitlement;
pub mod exports;
pub mod files;
pub mod history;
pub mod idea_vault;
pub mod interchange;
pub mod notes;
pub mod packages;
pub mod production;
pub mod project;
pub mod retention;
pub mod schedule;
pub mod screenplay;
pub mod search;
pub mod story;
pub mod visual;
pub mod writing;

pub fn register_all(r: &mut Registry) {
    project::register(r);
    history::register(r);
    search::register(r);
    files::register(r);
    visual::register(r);
    idea_vault::register(r);
    story::register(r);
    comments::register(r);
    screenplay::register(r);
    production::register(r);
    notes::register(r);
    schedule::register(r);
    ai::register(r);
    interchange::register(r);
    packages::register(r);
    exports::register(r);
    diagnostics::register(r);
    entitlement::register(r);
    writing::register(r);
    appearance::register(r);
}

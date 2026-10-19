//! The production of "Low Tide Radio", in the order a real team works: settings and the Idea
//! Vault, the Story Board, the screenplay and its drafts, then production planning, the shoot
//! paperwork, the Offline AI, and the exchange of packages with collaborators.

pub mod ai;
pub mod app;
pub mod exports;
pub mod packages;
pub mod production;
pub mod schedule;
pub mod screenplay;
pub mod story;
pub mod vault;
pub mod visual;
pub mod workspace;

use crate::Ctx;
use crate::gold::Gold;

pub fn run_all(g: &Gold, ctx: &mut Ctx) {
    app::settings(g, ctx);
    app::project(g, ctx);
    vault::run(g, ctx);
    story::board(g, ctx);
    screenplay::build_and_write(g, ctx);
    story::after_script(g, ctx);
    screenplay::drafts_and_review(g, ctx);
    production::people_and_places(g, ctx);
    screenplay::lock_for_production(g, ctx);
    packages::producer_review(g, ctx);
    production::breakdown(g, ctx);
    visual::run(g, ctx);
    schedule::run(g, ctx);
    schedule::paperwork(g, ctx);
    production::budget(g, ctx);
    workspace::notes_tasks_templates_files(g, ctx);
    screenplay::blue_revision(g, ctx);
    production::after_revision(g, ctx);
    schedule::after_revision(g, ctx);
    workspace::comments_history_trash_search(g, ctx);
    ai::run(g, ctx);
    screenplay::teaser(g, ctx);
    exports::run(g, ctx);
    packages::run(g, ctx);
    app::finish(g, ctx);
}

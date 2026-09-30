//! Backup, Full Project Packages and Exchange / Review / Response packages
//! (FSD §44.5–44.8, §45, §47, §50, §112, §118, §121; FSD-COL-001/002, FSD-OFF-007/008;
//! Import/Export spec §7–§20; Security spec §8, §22–§25).
//!
//! Collaboration is file-based only (LAN collaboration is out of scope): an
//! exchange package is a snapshot of a selected subset, sent by normal file
//! transfer, reviewed on another machine, and returned as a response package
//! that the author previews and applies deliberately.
//!
//! - [`format`]   container: manifest + checksums, safe write/validate/extract
//! - [`backup`]   Backup Package and Full Project Package (create / inspect / open / restore)
//! - [`exchange`] exchange package export (content model shared with import/review)
//! - [`import`]   Import Sessions: preview → apply (all-or-nothing) → report; Review Queue; review records
//! - [`review`]   the reviewer's read-only viewer and Response package export

pub mod backup;
pub mod exchange;
pub mod format;
pub mod import;
pub mod review;

use openframe_domain::{Actor, AppResult, Capability, new_id, now_ms};
use rusqlite::params;
use serde::Serialize;
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::{MutationMeta, Store};

pub use backup::{
    BackupPreview, BackupReport, PackageOpenMode, PackageProjectImportReport,
    build_project_package, import_project_package,
};
pub use format::{PackageManifest, PackageType};

/// A labelled count in package summaries ("Scenes 44", "Comments 23").
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageCount {
    pub label: String,
    pub count: u32,
}

/// A background package task was started; progress/result arrive as task events.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageTaskStarted {
    pub task_id: String,
    pub path: String,
}

#[derive(Debug, serde::Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageEmptyArgs {}

pub fn register(r: &mut Registry) {
    // Backup + Full Project Package.
    r.query("packages.backup_preview", backup_preview);
    r.command("packages.create_backup", create_backup);
    r.command("packages.export_project", export_project);
    r.query("packages.inspect_package", backup::inspect);
    r.command("packages.import_project", backup::start_import_project);
    // Exchange export.
    r.query("packages.exchange_sources", exchange_sources);
    r.query("packages.exchange_scenes", exchange::scenes);
    r.query("packages.exchange_preview", exchange::preview);
    r.command("packages.export_exchange", exchange::export);
    // Import sessions.
    r.command("packages.open_exchange", import::open_exchange);
    r.query("packages.session", import::get_session);
    r.query("packages.sessions", import::list_sessions);
    r.command("packages.apply_import", import::apply_import);
    r.command("packages.cancel_import", import::cancel_import);
    r.command("packages.undo_import", import::undo_import);
    r.query("packages.log", import::package_log);
    r.query("packages.review_queue", import::review_queue);
    r.command("packages.queue_attach", import::queue_attach);
    r.query("packages.records", import::records);
    r.command("packages.record_delete", import::record_delete);
    // Reviewer workspace and read-only viewers.
    r.command("packages.review_open", review::review_open);
    r.query("packages.review_list", review::review_list);
    r.query("packages.review_get", review::review_get);
    r.command("packages.review_comment", review::review_comment);
    r.command(
        "packages.review_delete_comment",
        review::review_delete_comment,
    );
    r.command(
        "packages.review_export_response",
        review::review_export_response,
    );
    r.query("packages.session_view", review::session_view);
    r.query("packages.record_view", review::record_view);
    import::register_data(r);
}

/// Record an exported/imported package in the project's package log and Activity.
pub(crate) fn log_package(
    store: &Store,
    actor: &Actor,
    direction: &str,
    ty: PackageType,
    package_id: &str,
    file_name: &str,
    summary: &str,
    session_id: Option<&str>,
) -> AppResult<()> {
    let origin = match &actor.origin {
        openframe_domain::auth::ActorOrigin::Local => "local",
        openframe_domain::auth::ActorOrigin::Exchange { .. } => "exchange",
        openframe_domain::auth::ActorOrigin::Ai { .. } => "ai",
    };
    store.mutate(
        actor,
        MutationMeta::new("packages.log", summary.to_string(), Capability::View)
            .not_undoable()
            .quiet(),
        |tx| {
            let now = now_ms();
            tx.conn().execute(
                "INSERT INTO sys_package_log(id, direction, package_type, package_id, file_name, summary, session_id, actor_id, actor_name, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    new_id(),
                    direction,
                    ty.as_str(),
                    package_id,
                    file_name,
                    summary,
                    session_id,
                    actor.user_id,
                    actor.display_name,
                    now
                ],
            )?;
            // Package operations touch no canonical rows, so the pipeline records no
            // activity by itself; the project's Activity still shows them.
            tx.conn().execute(
                "INSERT INTO sys_activity(id, at, actor_id, actor_name, action, summary, origin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    new_id(),
                    now,
                    actor.user_id,
                    actor.display_name,
                    format!("packages.{direction}"),
                    summary,
                    origin
                ],
            )?;
            Ok(())
        },
    )
}

fn backup_preview(core: &AppCore, actor: &Actor, _: PackageEmptyArgs) -> AppResult<BackupPreview> {
    backup::backup_preview(core, actor)
}

fn create_backup(
    core: &AppCore,
    actor: &Actor,
    a: backup::BackupCreateArgs,
) -> AppResult<PackageTaskStarted> {
    backup::start_project_package(core, actor, a, PackageType::Backup)
}

fn export_project(
    core: &AppCore,
    actor: &Actor,
    a: backup::BackupCreateArgs,
) -> AppResult<PackageTaskStarted> {
    backup::start_project_package(core, actor, a, PackageType::Project)
}

fn exchange_sources(
    core: &AppCore,
    actor: &Actor,
    _: PackageEmptyArgs,
) -> AppResult<exchange::PackageExchangeSources> {
    exchange::sources(core, actor)
}

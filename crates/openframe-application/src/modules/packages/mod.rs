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
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    use openframe_domain::Capability as Cap;
    r.module("Packages");
    // Backup + Full Project Package.
    r.query("packages.backup_preview", backup_preview).meta(
        M::compute("What a backup or full project package would contain.").hidden(h::UI_FLOW),
    );
    r.command("packages.create_backup", create_backup).meta(
        M::command(
            Cap::CreatePackage,
            "Write a backup package to a location the user picked.",
        )
        .fs(Fs::WritesUserFile)
        .long_running()
        .hidden(h::USER_PATH),
    );
    r.command("packages.export_project", export_project).meta(
        M::command(
            Cap::CreatePackage,
            "Write a full project package to a location the user picked.",
        )
        .fs(Fs::WritesUserFile)
        .long_running()
        .hidden(h::USER_PATH),
    );
    r.query("packages.inspect_package", backup::inspect).meta(
        M::app_query("Inspect a package file the user picked.")
            .fs(Fs::ReadsUserFile)
            .hidden(h::USER_PATH),
    );
    r.command("packages.import_project", backup::start_import_project)
        .meta(
            M::app_command("Open or restore a project from a package file the user picked.")
                .fs(Fs::ProjectStorage)
                .long_running()
                .destructive()
                .confirm()
                .hidden(h::USER_PATH),
        );
    // Exchange export.
    r.query("packages.exchange_sources", exchange_sources).meta(
        M::read("Drafts and scenes that can be sent in an exchange package (dialog helper).")
            .hidden(h::UI_FLOW),
    );
    r.query("packages.exchange_scenes", exchange::scenes).meta(
        M::read("Scenes of a draft for an exchange package (dialog helper).").hidden(h::UI_FLOW),
    );
    r.query("packages.exchange_preview", exchange::preview)
        .meta(M::compute("Preview of an exchange package before writing it.").hidden(h::UI_FLOW));
    r.command("packages.export_exchange", exchange::export)
        .meta(
            M::command(
                Cap::CreatePackage,
                "Write an exchange/review package to a location the user picked.",
            )
            .fs(Fs::WritesUserFile)
            .hidden(h::USER_PATH),
        );
    // Import sessions.
    r.command("packages.open_exchange", import::open_exchange)
        .meta(
            M::read("Open a received exchange package file the user picked for review.")
                .fs(Fs::ReadsUserFile)
                .class(crate::registry::OpClass::Mutate)
                .hidden(h::PACKAGE_REVIEW),
        );
    r.query("packages.session", import::get_session).meta(
        M::read("One package import session with its change preview.").hidden(h::PACKAGE_REVIEW),
    );
    r.query("packages.sessions", import::list_sessions)
        .meta(M::read(
            "Package import sessions (received packages and their status).",
        ));
    r.command("packages.apply_import", import::apply_import)
        .meta(
            M::command(
                Cap::Import,
                "Apply reviewed changes from a received package.",
            )
            .destructive()
            .hidden(h::PACKAGE_REVIEW),
        );
    r.command("packages.cancel_import", import::cancel_import)
        .meta(M::command(Cap::View, "Cancel a package import session.").hidden(h::PACKAGE_REVIEW));
    r.command("packages.undo_import", import::undo_import).meta(
        M::command(Cap::Import, "Undo an applied package import.")
            .destructive()
            .hidden(h::PACKAGE_REVIEW),
    );
    r.query("packages.log", import::package_log)
        .meta(M::read("Package history: packages exported and imported."));
    r.query("packages.review_queue", import::review_queue)
        .meta(M::read(
            "Review queue: received comments that need a place in the project.",
        ));
    r.command("packages.queue_attach", import::queue_attach)
        .meta(
            M::command(
                Cap::Comment,
                "Attach a queued received comment to a project object.",
            )
            .hidden(h::PACKAGE_REVIEW),
        );
    r.query("packages.records", import::records)
        .meta(M::read("Saved exchange review records."));
    r.command("packages.record_delete", import::record_delete)
        .meta(
            M::command(Cap::SoftDelete, "Delete a saved review record.")
                .destructive()
                .irreversible()
                .confirm()
                .hidden(h::DELETE_FOREVER),
        );
    // Reviewer workspace and read-only viewers.
    r.command("packages.review_open", review::review_open).meta(
        M::app_command("Open a review package file the user picked in the reviewer workspace.")
            .fs(Fs::ReadsUserFile)
            .hidden(h::USER_PATH),
    );
    r.query("packages.review_list", review::review_list).meta(
        M::app_query("Review packages open in the reviewer workspace.").hidden(h::PACKAGE_REVIEW),
    );
    r.query("packages.review_get", review::review_get).meta(
        M::app_query("One review package in the reviewer workspace.").hidden(h::PACKAGE_REVIEW),
    );
    r.command("packages.review_comment", review::review_comment)
        .meta(
            M::app_command("Add a reviewer comment inside a review package.")
                .hidden(h::PACKAGE_REVIEW),
        );
    r.command(
        "packages.review_delete_comment",
        review::review_delete_comment,
    )
    .meta(
        M::app_command("Delete a reviewer comment inside a review package.")
            .destructive()
            .hidden(h::PACKAGE_REVIEW),
    );
    r.command(
        "packages.review_export_response",
        review::review_export_response,
    )
    .meta(
        M::app_command("Write a review response package to a location the user picked.")
            .fs(Fs::WritesUserFile)
            .hidden(h::USER_PATH),
    );
    r.query("packages.session_view", review::session_view)
        .meta(M::read("Read-only viewer of a received package session.").hidden(h::PACKAGE_REVIEW));
    r.query("packages.record_view", review::record_view)
        .meta(M::read("Read-only viewer of a saved review record.").hidden(h::PACKAGE_REVIEW));
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

//! Project lifecycle (FSD §4, §6, §44–45; Release/Migration spec).
//!
//! create / open (lock → integrity → safety-backup → migrate → crash detection)
//! / save checkpoint / close / recover / archive / duplicate / delete-to-recycle-bin,
//! plus Project Home, settings, status and the "Continue" location.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use openframe_domain::enums::{ProjectStatus, ProjectType};
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_project_format::{self as pf, ProjectLayout, ProjectLock, ProjectManifest};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::core::{AppCore, LocalProfile, ProjectSession, RecoveryOffer};
use crate::events::{AppEvent, StoreKind};
use crate::registry::Registry;
use crate::schema::PROJECT_MIGRATIONS;
use crate::store::{MutationMeta, Store};
use crate::util::{optional_text, required_text};

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    r.module("Project");
    r.query("app.info", app_info).meta(
        M::app_query("Application version, folders and the local profile.").hidden(h::ASSET_PATH),
    );
    r.command("app.set_display_name", set_display_name)
        .meta(M::app_command("Change the local user's display name.").hidden(h::SESSION_CONTROL));
    r.query("app.save_state", save_state)
        .meta(M::app_query("Save indicator state for the status bar.").hidden(h::UI_FLOW));
    r.command("app.cancel_task", cancel_task)
        .meta(M::app_command("Cancel a running background task.").hidden(h::SESSION_CONTROL));
    r.query("project.list_recent", list_recent).meta(
        M::app_query("Recent projects on this computer (Home project list).")
            .fs(Fs::ProjectStorage)
            .hidden(h::APP_LIFECYCLE),
    );
    r.command("project.create", create).meta(
        M::app_command("Create a new project folder and open it.")
            .fs(Fs::ProjectStorage)
            .hidden(h::APP_LIFECYCLE),
    );
    r.command("project.open", open).meta(
        M::app_command("Open a project from disk (lock, integrity check, migrate, recover).")
            .fs(Fs::ProjectStorage)
            .hidden(h::APP_LIFECYCLE),
    );
    r.command("project.close", close)
        .meta(M::app_command("Close the open project.").hidden(h::APP_LIFECYCLE));
    r.query("project.current", current)
        .meta(M::app_query("Summary of the open project, if any.").hidden(h::APP_LIFECYCLE));
    r.query("project.home", home).meta(M::compute(
        "Project Home: headline counts, recent activity and continue location.",
    ));
    r.command("project.update_settings", update_settings)
        .meta(M::command(
            Capability::ManageProject,
            "Change project settings (title, type, language, genre, creator, logline).",
        ));
    r.command("project.set_status", set_status).meta(M::command(
        Capability::ManageProject,
        "Change the project status (Idea, Development, …).",
    ));
    r.command("project.save", save_checkpoint)
        .meta(M::edit("Save a checkpoint of the open project now.").hidden(h::SESSION_CONTROL));
    r.command("project.resolve_recovery", resolve_recovery)
        .meta(
            M::command(
                Capability::Edit,
                "Keep or discard recovered changes after a crash.",
            )
            .fs(Fs::ProjectStorage)
            .destructive()
            .confirm()
            .hidden(h::APP_LIFECYCLE),
        );
    r.command("project.set_archived", set_archived).meta(
        M::command(
            Capability::ManageProject,
            "Archive or unarchive a project in the project list.",
        )
        .hidden(h::APP_LIFECYCLE),
    );
    r.command("project.duplicate", duplicate).meta(
        M::command(
            Capability::ManageProject,
            "Duplicate a project folder on disk.",
        )
        .fs(Fs::ProjectStorage)
        .hidden(h::APP_LIFECYCLE),
    );
    r.command("project.delete", delete_project).meta(
        M::command(
            Capability::PermanentDelete,
            "Delete a project (moves its folder to the Recycle Bin).",
        )
        .fs(Fs::ProjectStorage)
        .destructive()
        .irreversible()
        .confirm()
        .hidden(h::DELETE_FOREVER),
    );
    r.command("project.remove_recent", remove_recent).meta(
        M::app_command("Remove a project from the recent-projects list.").hidden(h::APP_LIFECYCLE),
    );
    r.command("project.locate", locate).meta(
        M::app_command("Point a recent project at its moved folder.")
            .fs(Fs::ProjectStorage)
            .hidden(h::USER_PATH),
    );
    r.command("project.set_last_location", set_last_location)
        .meta(
            M::command(Capability::View, "Remember where the user was (Continue).")
                .hidden(h::VIEW_STATE),
        );
    r.query("project.members", members)
        .meta(M::read("People on this project and their roles."));
    r.command("project.rename", rename).meta(
        M::command(
            Capability::ManageProject,
            "Rename a project from the project list.",
        )
        .hidden(h::APP_LIFECYCLE),
    );
    r.command("project.set_pinned", set_pinned).meta(
        M::app_command("Pin a project in the recent-projects list.").hidden(h::APP_LIFECYCLE),
    );
    r.query("project.recovery_state", recovery_state).meta(
        M::read("Crash-recovery offer for the open project.")
            .fs(Fs::ProjectStorage)
            .hidden(h::APP_LIFECYCLE),
    );
    r.query("project.get_settings", get_settings).meta(M::read(
        "Project settings: title, type, status, language, genre, creator, logline.",
    ));
    r.query("project.deleted_items", deleted_items)
        .meta(M::read(
            "Recently Deleted: items that can still be restored.",
        ));
}

/// Keys of `sys_settings` (application database).
const SETTING_DISPLAY_NAME_SET: &str = "profile.display_name_confirmed";
/// Maximum size of the free-form project settings object.
const MAX_SETTINGS_BYTES: usize = 64 * 1024;

// ------------------------------------------------------------------- DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub app_version: String,
    pub projects_dir: String,
    pub global_vault_dir: String,
    pub profile: LocalProfile,
    pub platform: String,
    pub schema_version: u32,
    /// False until the user has confirmed their display name once (first-run prompt).
    pub display_name_set: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub title: String,
    pub project_type: ProjectType,
    pub status: String,
    pub language: Option<String>,
    pub genre: Option<String>,
    pub creator: Option<String>,
    pub logline: Option<String>,
    pub archived: bool,
    pub path: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub project_id: String,
    pub path: String,
    pub title: String,
    pub project_type: String,
    pub status: String,
    pub archived: bool,
    pub pinned: bool,
    #[ts(type = "number")]
    pub last_opened_at: i64,
    /// When the project's data last changed on disk (None when unavailable).
    #[ts(type = "number | null")]
    pub modified_at: Option<i64>,
    /// False when the folder (e.g. on an external drive) can't be found right now.
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectResult {
    pub project: ProjectSummary,
    pub recovery: Option<RecoveryOffer>,
    pub migrated_from: Option<u32>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub id: String,
    #[ts(type = "number")]
    pub at: i64,
    pub actor_name: Option<String>,
    pub action: String,
    pub summary: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub origin: String,
    /// Where "Open" goes, when the affected object still exists and is searchable.
    #[ts(type = "unknown")]
    pub nav: Option<Value>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct HomeCount {
    pub key: String,
    pub label: String,
    #[ts(type = "number")]
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHome {
    pub project: ProjectSummary,
    /// FSD §6.3: a brand-new project shows exactly three starting actions and no fake metrics.
    pub is_empty: bool,
    pub counts: Vec<HomeCount>,
    pub recent_activity: Vec<ActivityEntry>,
    #[ts(type = "unknown")]
    pub continue_location: Option<Value>,
    /// Last meaningful location per workspace, most recent first (FSD §6.2).
    #[ts(type = "Array<Record<string, unknown>>")]
    pub continue_locations: Vec<Value>,
    pub recovery: Option<RecoveryOffer>,
    /// When this session opened the project ("Last opened today 10:42").
    #[ts(type = "number")]
    pub opened_at: i64,
}

/// Crash-recovery and last-confirmed-save information (FSD §44, mocks 016/017/019).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryState {
    /// Present after an abnormal shutdown until the user chooses a version.
    pub offer: Option<RecoveryOffer>,
    pub has_checkpoint: bool,
    /// Time of the last confirmed save (checkpoint).
    #[ts(type = "number | null")]
    pub checkpoint_at: Option<i64>,
    /// Time of the latest recorded change.
    #[ts(type = "number | null")]
    pub last_change_at: Option<i64>,
    /// Recorded changes newer than the last confirmed save ("9 changes newer").
    #[ts(type = "number | null")]
    pub changes_since_checkpoint: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettings {
    pub project: ProjectSummary,
    pub project_notes: Option<String>,
    #[ts(type = "Record<string, unknown>")]
    pub settings: Value,
}

/// A Recently Deleted row with human context (FSD §52, mock 157).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DeletedItemView {
    pub id: String,
    pub object_type: String,
    pub object_id: String,
    pub title: Option<String>,
    /// e.g. "Scene Card", "File".
    pub type_label: String,
    /// e.g. "Story Board · Act “Act One”", "Files".
    pub was_in: String,
    /// The original container no longer exists: restore goes to Unassigned / the top level.
    pub parent_missing: bool,
    #[ts(type = "number")]
    pub deleted_at: i64,
    pub deleted_by_name: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameProjectArgs {
    pub project_id: String,
    pub title: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetPinnedArgs {
    pub project_id: String,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub user_id: String,
    pub display_name: String,
    pub role: String,
    pub professional_label: Option<String>,
    pub is_you: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateProjectArgs {
    pub title: String,
    pub project_type: ProjectType,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub creator: Option<String>,
    /// Parent folder; defaults to Documents/OpenFrame/Projects.
    #[serde(default)]
    pub parent_dir: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenProjectArgs {
    pub path: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSettingsArgs {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub language: Option<Option<String>>,
    #[serde(default)]
    pub genre: Option<Option<String>>,
    #[serde(default)]
    pub creator: Option<Option<String>>,
    #[serde(default)]
    pub logline: Option<Option<String>>,
    #[serde(default)]
    pub project_type: Option<ProjectType>,
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub settings: Option<Value>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetStatusArgs {
    pub status: ProjectStatus,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectIdArgs {
    pub project_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetArchivedArgs {
    pub project_id: String,
    pub archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocateArgs {
    pub project_id: String,
    pub path: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryArgs {
    /// "latest" = keep the latest autosaved state; "checkpoint" = open the last confirmed save.
    pub choice: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListRecentArgs {
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetLocationArgs {
    #[ts(type = "unknown")]
    pub location: Value,
}

#[derive(Debug, Deserialize, Default, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Empty {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisplayNameArgs {
    pub display_name: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskIdArgs {
    pub task_id: String,
}

// ------------------------------------------------------------- app-level ops

fn app_info(core: &AppCore, _a: &Actor, _: Empty) -> AppResult<AppInfo> {
    let display_name_set = core.with_app_db(|c| {
        Ok(c.query_row(
            "SELECT EXISTS(SELECT 1 FROM sys_settings WHERE key=?1)",
            [SETTING_DISPLAY_NAME_SET],
            |r| r.get(0),
        )?)
    })?;
    Ok(AppInfo {
        app_version: core.config.app_version.clone(),
        projects_dir: core.config.projects_dir.to_string_lossy().into_owned(),
        global_vault_dir: core.config.global_vault_dir.to_string_lossy().into_owned(),
        profile: core.profile(),
        platform: std::env::consts::OS.to_string(),
        schema_version: crate::schema::project_schema_version(),
        display_name_set,
    })
}

fn set_display_name(
    core: &AppCore,
    actor: &Actor,
    args: DisplayNameArgs,
) -> AppResult<LocalProfile> {
    // The local profile belongs to the person at this computer, never to someone whose changes arrive in a package.
    if !matches!(actor.origin, openframe_domain::auth::ActorOrigin::Local) {
        return Err(AppError::new(
            "permission.denied",
            "Only the person using this computer can change their name.",
        ));
    }
    let name = required_text(&args.display_name, "Your name", 80)?;
    let profile = core.set_display_name(&name)?;
    core.with_app_db(|c| {
        c.execute(
            "INSERT INTO sys_settings(key, value_json, updated_at) VALUES (?1, 'true', ?2)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
            params![SETTING_DISPLAY_NAME_SET, now_ms()],
        )?;
        Ok(())
    })?;
    if let Some(s) = core.project_opt() {
        s.store.with_writer(|c| {
            c.execute(
                "UPDATE project_member SET display_name=?1, updated_at=?2 WHERE user_id=?3",
                params![name, now_ms(), profile.user_id],
            )?;
            Ok(())
        })?;
    }
    Ok(profile)
}

fn save_state(core: &AppCore, _a: &Actor, _: Empty) -> AppResult<crate::events::SaveState> {
    Ok(core.save.snapshot())
}

fn cancel_task(core: &AppCore, _a: &Actor, args: TaskIdArgs) -> AppResult<bool> {
    Ok(core.tasks.cancel(&args.task_id))
}

// ------------------------------------------------------------------ recents

fn list_recent(core: &AppCore, _a: &Actor, args: ListRecentArgs) -> AppResult<Vec<RecentProject>> {
    core.with_app_db(|c| {
        let mut stmt = c.prepare(
            "SELECT project_id, path, title, project_type, status, archived, pinned, last_opened_at
             FROM sys_recent_project WHERE (?1 OR archived = 0) ORDER BY pinned DESC, last_opened_at DESC",
        )?;
        let rows = stmt
            .query_map([args.include_archived], |r| {
                let path: String = r.get(1)?;
                let layout = ProjectLayout::new(&path);
                let available = layout.looks_like_project();
                let modified_at = if available { data_modified_at(&layout) } else { None };
                Ok(RecentProject {
                    modified_at,
                    project_id: r.get(0)?,
                    path,
                    title: r.get(2)?,
                    project_type: r.get(3)?,
                    status: r.get(4)?,
                    archived: r.get::<_, i64>(5)? != 0,
                    pinned: r.get::<_, i64>(6)? != 0,
                    last_opened_at: r.get(7)?,
                    available,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

fn file_mtime_ms(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
}

/// Last time the project's database changed on disk (database file or its write-ahead log).
fn data_modified_at(layout: &ProjectLayout) -> Option<i64> {
    let db = layout.database();
    let wal = PathBuf::from(format!("{}-wal", db.display()));
    [file_mtime_ms(&db), file_mtime_ms(&wal)]
        .into_iter()
        .flatten()
        .max()
}

fn record_recent(core: &AppCore, summary: &ProjectSummary) -> AppResult<()> {
    let now = now_ms();
    core.with_app_db(|c| {
        c.execute(
            "INSERT INTO sys_recent_project(project_id, path, title, project_type, status, archived, pinned, last_opened_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?7)
             ON CONFLICT(project_id) DO UPDATE SET path=excluded.path, title=excluded.title,
                 project_type=excluded.project_type, status=excluded.status, archived=excluded.archived,
                 last_opened_at=excluded.last_opened_at, updated_at=excluded.updated_at",
            params![
                summary.id,
                summary.path,
                summary.title,
                summary.project_type.as_str(),
                summary.status,
                summary.archived as i64,
                now
            ],
        )?;
        Ok(())
    })
}

fn refresh_recent(core: &AppCore) -> AppResult<()> {
    if let Some(s) = core.project_opt() {
        let summary = load_summary(&s)?;
        core.with_app_db(|c| {
            c.execute(
                "UPDATE sys_recent_project SET title=?1, project_type=?2, status=?3, archived=?4, updated_at=?5 WHERE project_id=?6",
                params![
                    summary.title,
                    summary.project_type.as_str(),
                    summary.status,
                    summary.archived as i64,
                    now_ms(),
                    summary.id
                ],
            )?;
            Ok(())
        })?;
    }
    Ok(())
}

fn recent_path(core: &AppCore, project_id: &str) -> AppResult<PathBuf> {
    core.with_app_db(|c| {
        c.query_row(
            "SELECT path FROM sys_recent_project WHERE project_id=?1",
            [project_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .map(PathBuf::from)
        .ok_or_else(|| AppError::not_found("project"))
    })
}

// ---------------------------------------------------------------- lifecycle

pub fn load_summary(s: &ProjectSession) -> AppResult<ProjectSummary> {
    let path = s.layout.root().to_string_lossy().into_owned();
    s.store.read(|c| summary_from(c, &path))
}

fn summary_from(c: &Connection, path: &str) -> AppResult<ProjectSummary> {
    c.query_row(
        "SELECT id, title, project_type, status, language, genre, creator, logline, archived, created_at, updated_at, rev
         FROM project LIMIT 1",
        [],
        |r| {
            let t: String = r.get(2)?;
            Ok(ProjectSummary {
                id: r.get(0)?,
                title: r.get(1)?,
                project_type: ProjectType::parse(&t).unwrap_or(ProjectType::FeatureFilm),
                status: r.get(3)?,
                language: r.get(4)?,
                genre: r.get(5)?,
                creator: r.get(6)?,
                logline: r.get(7)?,
                archived: r.get::<_, i64>(8)? != 0,
                path: path.to_string(),
                created_at: r.get(9)?,
                updated_at: r.get(10)?,
                rev: r.get(11)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::project_format("This project's information is missing."))
}

fn create(core: &AppCore, actor: &Actor, args: CreateProjectArgs) -> AppResult<OpenProjectResult> {
    let title = required_text(&args.title, "Title", 200)?;
    let language = optional_text(args.language, "Language", 80)?;
    let genre = optional_text(args.genre, "Genre", 80)?;
    let creator = optional_text(args.creator, "Creator", 200)?;
    let parent = match args.parent_dir {
        Some(p) => PathBuf::from(p.trim()),
        None => core.config.projects_dir.clone(),
    };
    if pf::is_network_path(&parent) {
        return Err(network_location_error());
    }
    // Absolute, local, no device/reserved names, not inside the open project, app data
    // or the Global Idea Vault (PATH-03).
    crate::util::check_project_parent(core, &parent).map_err(network_or)?;
    std::fs::create_dir_all(&parent)?;
    let root = pf::unique_project_dir(&parent, &title);
    let layout = ProjectLayout::new(&root);
    let result = (|| -> AppResult<()> {
        layout.create_dirs()?;
        let project_id = new_id();
        let profile = core.profile();
        let mut conn = openframe_persistence::open_connection(&layout.database(), true)?;
        openframe_persistence::migrate::apply(&mut conn, PROJECT_MIGRATIONS)?;
        let now = now_ms();
        conn.execute(
            "INSERT INTO project(id, title, project_type, status, language, genre, creator, owner_user_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'Idea', ?4, ?5, ?6, ?7, ?8, ?8)",
            params![project_id, title, args.project_type.as_str(), language, genre, creator, profile.user_id, now],
        )?;
        conn.execute(
            "INSERT INTO project_member(id, user_id, display_name, role, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'Owner', ?4, ?4)",
            params![new_id(), profile.user_id, profile.display_name, now],
        )?;
        conn.execute(
            "INSERT INTO sys_activity(id, at, actor_id, actor_name, action, summary) VALUES (?1, ?2, ?3, ?4, 'project.create', ?5)",
            params![new_id(), now, profile.user_id, profile.display_name, format!("Created project “{title}”")],
        )?;
        openframe_persistence::checkpoint(&conn)?;
        drop(conn);
        let manifest = ProjectManifest::new(
            &project_id,
            &title,
            &core.config.app_version,
            crate::schema::project_schema_version(),
        );
        pf::write_manifest(&layout, &manifest)?;
        Ok(())
    })();
    if let Err(e) = result {
        // Never leave a half-created project folder behind.
        let _ = std::fs::remove_dir_all(&root);
        return Err(e);
    }
    open_at(core, actor, &root)
}

fn open(core: &AppCore, actor: &Actor, args: OpenProjectArgs) -> AppResult<OpenProjectResult> {
    let mut path = PathBuf::from(args.path.trim());
    if path.file_name().and_then(|n| n.to_str()) == Some(pf::MANIFEST_FILE) {
        path = path.parent().map(Path::to_path_buf).unwrap_or(path);
    }
    open_at(core, actor, &path)
}

/// Keep the project-specific wording for network locations.
fn network_or(e: AppError) -> AppError {
    if e.is("project_format.network_location") {
        network_location_error()
    } else {
        e
    }
}

fn network_location_error() -> AppError {
    AppError::new(
        "project_format.network_location",
        "OpenFrame projects can't be edited directly from a network folder. Copy the project to this computer, or share it with others using a review or project package.",
    )
}

/// Open a project folder (FSD §4, Release/Migration: safety backup before migration).
pub fn open_at(core: &AppCore, _actor: &Actor, root: &Path) -> AppResult<OpenProjectResult> {
    // Syntax first: never probe a device, share or reserved name (PATH-03).
    if pf::is_network_path(root) {
        return Err(network_location_error());
    }
    openframe_security::check_user_path(root, openframe_security::NetworkPaths::Refuse)
        .map_err(network_or)?;
    if !root.exists() {
        return Err(AppError::new(
            "not_found.project_folder",
            "This project can't be found. If it's on an external drive, reconnect the drive and try again.",
        ));
    }
    if pf::is_network_path(root) {
        return Err(network_location_error());
    }
    let layout = ProjectLayout::new(root);
    let manifest = pf::read_manifest(&layout)?;
    // Opening the same project again is a no-op; switching projects closes the current one first.
    if let Some(current) = core.project_opt() {
        if current.project_id() == manifest.project_id {
            let summary = load_summary(&current)?;
            let recovery = current.recovery.lock().clone();
            return Ok(OpenProjectResult {
                project: summary,
                recovery,
                migrated_from: None,
            });
        }
        close_current(core)?;
    }
    let lock = ProjectLock::acquire(&layout)?;
    let unclean = pf::previous_unclean_session(&layout);
    let (store, report) = Store::open(
        StoreKind::Project,
        root,
        &layout.database(),
        PROJECT_MIGRATIONS,
        false,
        &layout.safety_backups(),
        core.registry.clone(),
        core.events.clone(),
        core.save.clone(),
    )?;
    let db_project_id: String =
        store.read(|c| Ok(c.query_row("SELECT id FROM project LIMIT 1", [], |r| r.get(0))?))?;
    if db_project_id != manifest.project_id {
        return Err(AppError::project_format(
            "This project's identity information doesn't match its data.",
        ));
    }
    let recovery = unclean.map(|m| {
        let last_autosave_at: Option<i64> = store
            .read(|c| Ok(c.query_row("SELECT MAX(at) FROM sys_activity", [], |r| r.get(0))?))
            .ok()
            .flatten();
        let checkpoint = layout.checkpoint();
        let checkpoint_at = std::fs::metadata(&checkpoint)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64);
        RecoveryOffer {
            previous_session_started_at: m.started_at,
            last_autosave_at,
            has_checkpoint: checkpoint.is_file(),
            checkpoint_at,
        }
    });
    pf::write_session_marker(&layout, &core.config.app_version)?;
    // Ensure the local user is a member (projects received from others may not list us).
    let profile = core.profile();
    store.with_writer(|c| {
        let exists: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM project_member WHERE user_id=?1)",
            [&profile.user_id],
            |r| r.get(0),
        )?;
        if !exists {
            // A project folder on this computer is a user-owned working asset (FSD §45,
            // Security §2): whoever opens their local copy is its Owner. Collaborator
            // roles apply to exchange packages and exports, not to local files.
            c.execute(
                "INSERT INTO project_member(id, user_id, display_name, role, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'Owner', ?4, ?4)",
                params![new_id(), profile.user_id, profile.display_name, now_ms()],
            )?;
        }
        Ok(())
    })?;
    let mut manifest = manifest;
    if manifest.schema_version != report.schema_version
        || manifest.app_version != core.config.app_version
    {
        manifest.schema_version = report.schema_version;
        manifest.app_version = core.config.app_version.clone();
        pf::write_manifest(&layout, &manifest)?;
    }
    let session = Arc::new(ProjectSession::new(
        layout,
        store,
        manifest,
        lock,
        recovery.clone(),
    ));
    let search_empty: bool = session.store.read(|c| {
        Ok(
            c.query_row("SELECT NOT EXISTS(SELECT 1 FROM search_doc)", [], |r| {
                r.get(0)
            })?,
        )
    })?;
    if (search_empty || report.migrated_from.is_some())
        && let Err(e) = session.store.rebuild_search_index()
    {
        tracing::warn!(
            code = e.code_str(),
            "search index rebuild failed; search may be incomplete"
        );
    }
    let summary = load_summary(&session)?;
    core.set_project(Some(session));
    record_recent(core, &summary)?;
    core.emit(AppEvent::ProjectOpened {
        project_id: summary.id.clone(),
    });
    Ok(OpenProjectResult {
        project: summary,
        recovery,
        migrated_from: report.migrated_from,
    })
}

/// Clean close: fold the WAL, refresh the "last confirmed" checkpoint, clear the crash marker.
pub fn close_current(core: &AppCore) -> AppResult<()> {
    let Some(session) = core.set_project(None) else {
        return Ok(());
    };
    let id = session.project_id();
    let result = (|| -> AppResult<()> {
        session.store.with_writer(|c| {
            openframe_persistence::checkpoint(c)?;
            openframe_persistence::backup_to(c, &session.layout.checkpoint())
        })?;
        pf::clear_session_marker(&session.layout)
    })();
    if let Err(e) = &result {
        tracing::error!(
            code = e.code_str(),
            "project did not close cleanly; recovery will be offered next time"
        );
    }
    drop(session);
    core.emit(AppEvent::ProjectClosed { project_id: id });
    result
}

fn close(core: &AppCore, _a: &Actor, _: Empty) -> AppResult<()> {
    close_current(core)
}

fn current(core: &AppCore, _a: &Actor, _: Empty) -> AppResult<Option<ProjectSummary>> {
    match core.project_opt() {
        Some(s) => Ok(Some(load_summary(&s)?)),
        None => Ok(None),
    }
}

/// Explicit Save (FSD §44.2): everything is already persisted; this folds the
/// WAL and records the "last confirmed saved state" used by crash recovery.
fn save_checkpoint(core: &AppCore, actor: &Actor, _: Empty) -> AppResult<i64> {
    actor.require(Capability::Edit, "save this project")?;
    let s = core.project()?;
    core.save.saving();
    let result = s.store.with_writer(|c| {
        openframe_persistence::checkpoint(c)?;
        openframe_persistence::backup_to(c, &s.layout.checkpoint())
    });
    match &result {
        Ok(()) => core.save.saved(),
        Err(e) => core.save.failed(e),
    }
    result.map(|_| now_ms())
}

fn resolve_recovery(
    core: &AppCore,
    actor: &Actor,
    args: RecoveryArgs,
) -> AppResult<OpenProjectResult> {
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    match args.choice.as_str() {
        "latest" => {
            *s.recovery.lock() = None;
            let summary = load_summary(&s)?;
            Ok(OpenProjectResult {
                project: summary,
                recovery: None,
                migrated_from: None,
            })
        }
        "checkpoint" => {
            actor.require(Capability::Edit, "restore this project")?;
            let layout = s.layout.clone();
            if !layout.checkpoint().is_file() {
                return Err(AppError::new(
                    "not_found.checkpoint",
                    "There is no confirmed saved state to return to.",
                ));
            }
            // Close without writing a new checkpoint (that would overwrite the one we restore).
            // Every handle to the session must be released so the database files are closed.
            drop(s);
            let session = core.set_project(None).expect("project is open");
            drop(session);
            std::fs::create_dir_all(layout.safety_backups())?;
            let keep = layout
                .safety_backups()
                .join(format!("before-recovery-{}.sqlite", now_ms()));
            {
                let conn = openframe_persistence::open_connection(&layout.database(), false)?;
                openframe_persistence::backup_to(&conn, &keep)?;
            }
            for suffix in ["-wal", "-shm"] {
                let p = PathBuf::from(format!("{}{}", layout.database().display(), suffix));
                if p.exists() {
                    std::fs::remove_file(p)?;
                }
            }
            let tmp = layout.database().with_extension("restoring");
            std::fs::copy(layout.checkpoint(), &tmp)?;
            std::fs::rename(&tmp, layout.database())?;
            pf::clear_session_marker(&layout)?;
            let mut result = open_at(core, actor, &root)?;
            result.recovery = None;
            Ok(result)
        }
        _ => Err(AppError::invalid_input("Unknown recovery choice.")),
    }
}

// ---------------------------------------------------------------- settings

fn update_settings(
    core: &AppCore,
    actor: &Actor,
    args: UpdateSettingsArgs,
) -> AppResult<ProjectSummary> {
    let s = core.project()?;
    let title = args
        .title
        .map(|t| required_text(&t, "Title", 200))
        .transpose()?;
    let fix =
        |v: Option<Option<String>>, what: &str, max: usize| -> AppResult<Option<Option<String>>> {
            v.map(|inner| optional_text(inner, what, max)).transpose()
        };
    let language = fix(args.language, "Language", 80)?;
    let genre = fix(args.genre, "Genre", 80)?;
    let creator = fix(args.creator, "Creator", 200)?;
    let logline = fix(args.logline, "Logline", 1000)?;
    if let Some(v) = &args.settings {
        if !v.is_object() {
            return Err(AppError::invalid_input("Settings must be an object."));
        }
        if v.to_string().len() > MAX_SETTINGS_BYTES {
            return Err(AppError::invalid_input("Project settings are too large."));
        }
        if let Some(notes) = v.get("projectNotes")
            && !(notes.is_null()
                || notes
                    .as_str()
                    .map(|s| s.chars().count() <= 20_000)
                    .unwrap_or(false))
        {
            return Err(AppError::invalid_input(
                "Project notes are too long (maximum 20000 characters).",
            ));
        }
    }
    s.store.mutate(
        actor,
        MutationMeta::new(
            "project.update_settings",
            "Updated project settings",
            Capability::ManageProject,
        ),
        |tx| {
            use openframe_persistence::rows::{opt_text, text, update_fields};
            let id: String = tx
                .conn()
                .query_row("SELECT id FROM project", [], |r| r.get(0))?;
            let mut fields = Vec::new();
            if let Some(t) = &title {
                fields.push(("title", text(t.clone())));
            }
            if let Some(v) = language {
                fields.push(("language", opt_text(v)));
            }
            if let Some(v) = genre {
                fields.push(("genre", opt_text(v)));
            }
            if let Some(v) = creator {
                fields.push(("creator", opt_text(v)));
            }
            if let Some(v) = logline {
                fields.push(("logline", opt_text(v)));
            }
            if let Some(t) = args.project_type {
                fields.push(("project_type", text(t.as_str())));
            }
            if let Some(v) = &args.settings {
                let current: String =
                    tx.conn()
                        .query_row("SELECT settings_json FROM project", [], |r| r.get(0))?;
                let mut merged: Value = serde_json::from_str(&current)
                    .unwrap_or_else(|_| Value::Object(Default::default()));
                if let (Some(m), Some(n)) = (merged.as_object_mut(), v.as_object()) {
                    for (k, val) in n {
                        m.insert(k.clone(), val.clone());
                    }
                }
                fields.push(("settings_json", text(merged.to_string())));
            }
            update_fields(
                tx.conn(),
                "project",
                &id,
                &fields,
                &[
                    "title",
                    "language",
                    "genre",
                    "creator",
                    "logline",
                    "project_type",
                    "settings_json",
                ],
                None,
                "project",
            )?;
            Ok(())
        },
    )?;
    if let Some(t) = &title {
        let mut m = s.manifest.lock();
        m.title = t.clone();
        pf::write_manifest(&s.layout, &m)?;
    }
    refresh_recent(core)?;
    load_summary(&s)
}

fn set_status(core: &AppCore, actor: &Actor, args: SetStatusArgs) -> AppResult<ProjectSummary> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "project.set_status",
            format!("Changed project status to {}", args.status),
            Capability::ManageProject,
        ),
        |tx| {
            tx.conn().execute(
                "UPDATE project SET status=?1, status_user_selected=1, updated_at=?2, rev=rev+1",
                params![args.status.as_str(), now_ms()],
            )?;
            Ok(())
        },
    )?;
    refresh_recent(core)?;
    load_summary(&s)
}

fn set_archived(core: &AppCore, actor: &Actor, args: SetArchivedArgs) -> AppResult<()> {
    actor.require(Capability::ManageProject, "archive projects")?;
    // Archive is stored in the project itself (so opening an archived project later
    // doesn't silently restore it) and in the Home registry.
    let open_here = core
        .project_opt()
        .filter(|s| s.project_id() == args.project_id);
    if open_here.is_none() {
        let root = recent_path(core, &args.project_id)?;
        // A project on a disconnected drive can still be archived in the Home list;
        // its own flag is updated the next time it is available.
        if ProjectLayout::new(&root).looks_like_project() {
            let summary = if args.archived {
                "Archived project"
            } else {
                "Restored project from archive"
            };
            with_closed_project(
                core,
                actor,
                &args.project_id,
                "project.archive",
                summary,
                |c| {
                    c.execute(
                        "UPDATE project SET archived=?1, updated_at=?2, rev=rev+1",
                        params![args.archived as i64, now_ms()],
                    )?;
                    Ok(())
                },
            )?;
        }
    }
    if let Some(s) = open_here {
        {
            s.store.mutate(
                actor,
                MutationMeta::new(
                    "project.archive",
                    if args.archived {
                        "Archived project"
                    } else {
                        "Restored project from archive"
                    },
                    Capability::ManageProject,
                ),
                |tx| {
                    tx.conn().execute(
                        "UPDATE project SET archived=?1, updated_at=?2, rev=rev+1",
                        params![args.archived as i64, now_ms()],
                    )?;
                    Ok(())
                },
            )?;
        }
    }
    let n = core.with_app_db(|c| {
        Ok(c.execute(
            "UPDATE sys_recent_project SET archived=?1, updated_at=?2 WHERE project_id=?3",
            params![args.archived as i64, now_ms(), args.project_id],
        )?)
    })?;
    if n == 0 {
        return Err(AppError::not_found("project"));
    }
    Ok(())
}

fn remove_recent(core: &AppCore, _a: &Actor, args: ProjectIdArgs) -> AppResult<()> {
    core.with_app_db(|c| {
        c.execute(
            "DELETE FROM sys_recent_project WHERE project_id=?1",
            [&args.project_id],
        )?;
        Ok(())
    })
}

fn locate(core: &AppCore, actor: &Actor, args: LocateArgs) -> AppResult<OpenProjectResult> {
    let mut path = PathBuf::from(args.path.trim());
    if path.file_name().and_then(|n| n.to_str()) == Some(pf::MANIFEST_FILE) {
        path = path.parent().map(Path::to_path_buf).unwrap_or(path);
    }
    // Validate before touching the filesystem or updating Recent Projects.
    if pf::is_network_path(&path) {
        return Err(network_location_error());
    }
    openframe_security::check_user_path(&path, openframe_security::NetworkPaths::Refuse)
        .map_err(network_or)?;
    let manifest = pf::read_manifest(&ProjectLayout::new(&path))?;
    if manifest.project_id != args.project_id {
        return Err(AppError::new(
            "validation.different_project",
            "That folder contains a different project. Choose the folder for this project.",
        ));
    }
    core.with_app_db(|c| {
        c.execute(
            "UPDATE sys_recent_project SET path=?1, updated_at=?2 WHERE project_id=?3",
            params![path.to_string_lossy(), now_ms(), args.project_id],
        )?;
        Ok(())
    })?;
    open_at(core, actor, &path)
}

/// Duplicate Project (FSD §4.4): an independent project with a new identity.
fn duplicate(core: &AppCore, actor: &Actor, args: ProjectIdArgs) -> AppResult<RecentProject> {
    actor.require(Capability::ManageProject, "duplicate projects")?;
    let src_root = recent_path(core, &args.project_id)?;
    let src = ProjectLayout::new(&src_root);
    let manifest = pf::read_manifest(&src)?;
    let title = format!("Copy of {}", manifest.title);
    let parent = src_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| core.config.projects_dir.clone());
    let dest_root = pf::unique_project_dir(&parent, &title);
    let dest = ProjectLayout::new(&dest_root);
    let result = (|| -> AppResult<RecentProject> {
        dest.create_dirs()?;
        // Consistent database copy even when the source is open (online backup API).
        match core
            .project_opt()
            .filter(|s| s.project_id() == args.project_id)
        {
            Some(s) => s
                .store
                .with_writer(|c| openframe_persistence::backup_to(c, &dest.database()))?,
            None => {
                let c = openframe_persistence::open_connection(&src.database(), false)?;
                openframe_persistence::backup_to(&c, &dest.database())?;
            }
        }
        copy_dir(&src.assets(), &dest.assets())?;
        let new_id = new_id();
        let now = now_ms();
        let c = openframe_persistence::open_connection(&dest.database(), false)?;
        c.execute(
            "UPDATE project SET id=?1, title=?2, archived=0, updated_at=?3",
            params![new_id, title, now],
        )?;
        c.execute("DELETE FROM sys_undo", [])?;
        c.execute(
            "INSERT INTO sys_activity(id, at, actor_id, actor_name, action, summary) VALUES (?1, ?2, ?3, ?4, 'project.duplicate', ?5)",
            params![
                openframe_domain::new_id(),
                now,
                actor.user_id,
                actor.display_name,
                format!("Duplicated from “{}”", manifest.title)
            ],
        )?;
        openframe_persistence::checkpoint(&c)?;
        drop(c);
        let mut m = ProjectManifest::new(
            &new_id,
            &title,
            &core.config.app_version,
            manifest.schema_version,
        );
        m.created_at = now;
        pf::write_manifest(&dest, &m)?;
        let status: String = manifest_status(&dest)?;
        let rp = RecentProject {
            project_id: new_id.clone(),
            path: dest_root.to_string_lossy().into_owned(),
            title: title.clone(),
            project_type: project_type_of(&dest)?,
            status,
            archived: false,
            pinned: false,
            last_opened_at: now,
            modified_at: data_modified_at(&dest),
            available: true,
        };
        core.with_app_db(|c| {
            c.execute(
                "INSERT INTO sys_recent_project(project_id, path, title, project_type, status, archived, pinned, last_opened_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, ?6)",
                params![rp.project_id, rp.path, rp.title, rp.project_type, rp.status, now],
            )?;
            Ok(())
        })?;
        Ok(rp)
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&dest_root);
    }
    result
}

fn manifest_status(layout: &ProjectLayout) -> AppResult<String> {
    let c = openframe_persistence::open_connection(&layout.database(), false)?;
    Ok(c.query_row("SELECT status FROM project", [], |r| r.get(0))?)
}

fn project_type_of(layout: &ProjectLayout) -> AppResult<String> {
    let c = openframe_persistence::open_connection(&layout.database(), false)?;
    Ok(c.query_row("SELECT project_type FROM project", [], |r| r.get(0))?)
}

fn copy_dir(src: &Path, dest: &Path) -> AppResult<()> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let target = dest.join(entry.file_name());
        if ft.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if ft.is_file() {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Delete a project: moves the whole project folder to the Recycle Bin, so it
/// stays recoverable by the operating system (deliberate confirmation happens in the UI).
fn delete_project(core: &AppCore, actor: &Actor, args: ProjectIdArgs) -> AppResult<()> {
    actor.require(Capability::PermanentDelete, "delete projects")?;
    if core
        .project_opt()
        .map(|s| s.project_id() == args.project_id)
        .unwrap_or(false)
    {
        return Err(AppError::conflict("Close this project before deleting it."));
    }
    let root = recent_path(core, &args.project_id)?;
    if root.exists() {
        let manifest = pf::read_manifest(&ProjectLayout::new(&root))?;
        if manifest.project_id != args.project_id {
            return Err(AppError::conflict(
                "The folder on disk no longer matches this project, so it was not deleted.",
            ));
        }
        let _lock = ProjectLock::acquire(&ProjectLayout::new(&root))?;
        drop(_lock);
        trash::delete(&root).map_err(|e| {
            AppError::new(
                "storage.delete_failed",
                "OpenFrame couldn't move this project to the Recycle Bin. Nothing was deleted.",
            )
            .with_detail(e.to_string())
        })?;
    }
    core.with_app_db(|c| {
        c.execute(
            "DELETE FROM sys_recent_project WHERE project_id=?1",
            [&args.project_id],
        )?;
        Ok(())
    })
}

// ------------------------------------------------------------ project home

fn table_exists(c: &Connection, table: &str) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [table],
        |r| r.get(0),
    )?)
}

/// Count live rows if the (module-owned) table exists. Home is a derived view.
fn count_live(c: &Connection, table: &str, extra_where: &str) -> AppResult<i64> {
    if !table_exists(c, table)? {
        return Ok(0);
    }
    let has_deleted: bool = c.query_row(
        &format!(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name='deleted_at')"
        ),
        [],
        |r| r.get(0),
    )?;
    let mut sql = format!("SELECT count(*) FROM \"{table}\" WHERE 1=1");
    if has_deleted {
        sql.push_str(" AND deleted_at IS NULL");
    }
    if !extra_where.is_empty() {
        sql.push_str(" AND ");
        sql.push_str(extra_where);
    }
    Ok(c.query_row(&sql, [], |r| r.get(0)).unwrap_or(0))
}

pub fn recent_activity(c: &Connection, limit: i64) -> AppResult<Vec<ActivityEntry>> {
    // "Open" targets come from the search projection of the affected object, which
    // disappears when the object is deleted — so deleted objects never offer "Open".
    let mut stmt = c.prepare(
        "SELECT a.id, a.at, a.actor_name, a.action, a.summary, a.target_type, a.target_id, a.origin,
                (SELECT d.nav_json FROM search_doc d
                  WHERE d.source_table = a.target_type AND d.entity_id = a.target_id AND d.owner_user_id IS NULL)
         FROM sys_activity a ORDER BY a.at DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map([limit], |r| {
            let nav: Option<String> = r.get(8)?;
            Ok(ActivityEntry {
                id: r.get(0)?,
                at: r.get(1)?,
                actor_name: r.get(2)?,
                action: r.get(3)?,
                summary: r.get(4)?,
                target_type: r.get(5)?,
                target_id: r.get(6)?,
                origin: r.get(7)?,
                nav: nav.and_then(|n| serde_json::from_str(&n).ok()),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn home(core: &AppCore, actor: &Actor, _: Empty) -> AppResult<ProjectHome> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    let project = load_summary(&s)?;
    let recovery = s.recovery.lock().clone();
    let user = actor.user_id.clone();
    s.store.read(|c| {
        // (key, label, table, extra filter). Tables are owned by their modules; missing tables count as 0.
        let specs: [(&str, &str, &str, &str); 10] = [
            ("ideas", "Ideas", "vault_item", ""),
            ("sceneCards", "Scene cards", "story_scene_card", ""),
            ("characters", "Characters", "story_character", ""),
            ("drafts", "Screenplay drafts", "screenplay_draft", ""),
            ("scenes", "Screenplay scenes", "screenplay_scene", ""),
            ("breakdownItems", "Breakdown items", "breakdown_element", "confirmation_state IN ('Confirmed','Manual')"),
            ("locations", "Locations", "location", ""),
            ("shots", "Shots", "shot", ""),
            ("shootingDays", "Shooting days", "shooting_day", ""),
            ("callSheets", "Call sheets", "call_sheet", ""),
        ];
        let mut counts = Vec::new();
        for (key, label, table, filter) in specs {
            let n = count_live(c, table, filter)?;
            counts.push(HomeCount { key: key.to_string(), label: label.to_string(), count: n });
        }
        let content: i64 = counts.iter().filter(|h| h.key != "drafts").map(|h| h.count).sum::<i64>()
            + count_live(c, "story_act", "")?
            + count_live(c, "story_beat", "")?
            + count_live(c, "project_file", "")?
            + count_live(c, "project_note", "")?
            + count_live(c, "task", "")?;
        let continue_location: Option<Value> = c
            .query_row(
                "SELECT value_json FROM sys_view_state WHERE user_id=?1 AND key='continue'",
                [&user],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .and_then(|s| serde_json::from_str(&s).ok());
        let continue_locations: Vec<Value> = {
            let mut stmt = c.prepare(
                "SELECT value_json FROM sys_view_state WHERE user_id=?1 AND key LIKE 'continue:%' ORDER BY updated_at DESC LIMIT 8",
            )?;
            let rows: Vec<String> = stmt.query_map([&user], |r| r.get(0))?.collect::<Result<_, _>>()?;
            rows.iter().filter_map(|s| serde_json::from_str(s).ok()).collect()
        };
        Ok(ProjectHome {
            project,
            is_empty: content == 0,
            counts: counts.into_iter().filter(|h| h.count > 0).collect(),
            recent_activity: recent_activity(c, 12)?,
            continue_location,
            continue_locations,
            recovery,
            opened_at: s.opened_at,
        })
    })
}

/// Workspaces that can be "continued" (FSD §6.2). Keys are UI workspace ids.
const CONTINUE_WORKSPACES: &[&str] = &[
    "vault",
    "story",
    "screenplay",
    "breakdown",
    "production",
    "callsheets",
    "files",
    "notes",
];

/// FSD §6.2: remember the last meaningful location per project and user.
fn set_last_location(core: &AppCore, actor: &Actor, args: SetLocationArgs) -> AppResult<()> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    if !args.location.is_object() {
        return Err(AppError::invalid_input("Location must be an object."));
    }
    let json = args.location.to_string();
    if json.len() > 4096 {
        return Err(AppError::invalid_input("Location is too large."));
    }
    let workspace = args
        .location
        .get("workspace")
        .and_then(Value::as_str)
        .unwrap_or("");
    let per_workspace = CONTINUE_WORKSPACES
        .contains(&workspace)
        .then(|| format!("continue:{workspace}"));
    s.store.with_writer(|c| {
        let now = now_ms();
        for key in std::iter::once("continue".to_string()).chain(per_workspace) {
            c.execute(
                "INSERT INTO sys_view_state(user_id, key, value_json, updated_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(user_id, key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
                params![actor.user_id, key, json, now],
            )?;
        }
        Ok(())
    })
}

fn members(core: &AppCore, actor: &Actor, _: Empty) -> AppResult<Vec<Member>> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    s.store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT user_id, display_name, role, professional_label FROM project_member WHERE deleted_at IS NULL ORDER BY created_at",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let uid: String = r.get(0)?;
                Ok(Member {
                    is_you: uid == actor.user_id,
                    user_id: uid,
                    display_name: r.get(1)?,
                    role: r.get(2)?,
                    professional_label: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

// ------------------------------------------------- closed-project maintenance

/// Run a small maintenance edit directly on a project that is NOT open (Home
/// actions such as Rename and Archive). Takes the single-writer lock, refuses
/// projects written by a newer OpenFrame, records an activity entry and
/// checkpoints so the change is durable. Never used for an open project.
fn with_closed_project<R>(
    core: &AppCore,
    actor: &Actor,
    project_id: &str,
    action: &str,
    summary: &str,
    f: impl FnOnce(&Connection) -> AppResult<R>,
) -> AppResult<R> {
    if core
        .project_opt()
        .map(|s| s.project_id() == project_id)
        .unwrap_or(false)
    {
        return Err(AppError::internal(
            "with_closed_project called for the open project",
        ));
    }
    let root = recent_path(core, project_id)?;
    let layout = ProjectLayout::new(&root);
    if !layout.looks_like_project() {
        return Err(AppError::new(
            "not_found.project_folder",
            "This project can't be found. If it's on an external drive, reconnect the drive and try again.",
        ));
    }
    let manifest = pf::read_manifest(&layout)?;
    if manifest.project_id != project_id {
        return Err(AppError::conflict(
            "The folder on disk no longer matches this project, so it was not changed.",
        ));
    }
    let _lock = ProjectLock::acquire(&layout)?;
    let mut conn = openframe_persistence::open_connection(&layout.database(), false)?;
    let version = openframe_persistence::migrate::current_version(&conn)?;
    let supported = crate::schema::project_schema_version();
    if version > supported {
        return Err(openframe_persistence::migrate::too_new_error(
            version, supported,
        ));
    }
    let tx = conn.transaction()?;
    let out = f(&tx)?;
    tx.execute(
        "INSERT INTO sys_activity(id, at, actor_id, actor_name, action, summary) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![new_id(), now_ms(), actor.user_id, actor.display_name, action, summary],
    )?;
    tx.commit()?;
    openframe_persistence::checkpoint(&conn)?;
    Ok(out)
}

/// Rename a project (Home card "Rename" F2, FSD §3.5). Works on the open project
/// (undoable, through the pipeline) or on a closed one (title + manifest + Home list).
fn rename(core: &AppCore, actor: &Actor, args: RenameProjectArgs) -> AppResult<RecentProject> {
    actor.require(Capability::ManageProject, "rename projects")?;
    let title = required_text(&args.title, "Title", 200)?;
    let open_here = core
        .project_opt()
        .filter(|s| s.project_id() == args.project_id);
    if let Some(s) = open_here {
        s.store.mutate(
            actor,
            MutationMeta::new(
                "project.rename",
                format!("Renamed project to “{title}”"),
                Capability::ManageProject,
            ),
            |tx| {
                let id: String = tx
                    .conn()
                    .query_row("SELECT id FROM project", [], |r| r.get(0))?;
                openframe_persistence::rows::update_fields(
                    tx.conn(),
                    "project",
                    &id,
                    &[("title", openframe_persistence::rows::text(title.clone()))],
                    &["title"],
                    None,
                    "project",
                )?;
                Ok(())
            },
        )?;
        {
            let mut m = s.manifest.lock();
            m.title = title.clone();
            pf::write_manifest(&s.layout, &m)?;
        }
        refresh_recent(core)?;
    } else {
        let root = recent_path(core, &args.project_id)?;
        with_closed_project(
            core,
            actor,
            &args.project_id,
            "project.rename",
            &format!("Renamed project to “{title}”"),
            |c| {
                c.execute(
                    "UPDATE project SET title=?1, updated_at=?2, rev=rev+1",
                    params![title, now_ms()],
                )?;
                Ok(())
            },
        )?;
        let layout = ProjectLayout::new(&root);
        let mut manifest = pf::read_manifest(&layout)?;
        manifest.title = title.clone();
        pf::write_manifest(&layout, &manifest)?;
        core.with_app_db(|c| {
            c.execute(
                "UPDATE sys_recent_project SET title=?1, updated_at=?2 WHERE project_id=?3",
                params![title, now_ms(), args.project_id],
            )?;
            Ok(())
        })?;
    }
    recent_entry(core, &args.project_id)
}

fn recent_entry(core: &AppCore, project_id: &str) -> AppResult<RecentProject> {
    list_recent(
        core,
        &core.local_actor(),
        ListRecentArgs {
            include_archived: true,
        },
    )?
    .into_iter()
    .find(|p| p.project_id == project_id)
    .ok_or_else(|| AppError::not_found("project"))
}

/// Pin / Unpin on the Home screen (a view preference; the project is not changed).
fn set_pinned(core: &AppCore, actor: &Actor, args: SetPinnedArgs) -> AppResult<RecentProject> {
    if !matches!(actor.origin, openframe_domain::auth::ActorOrigin::Local) {
        return Err(AppError::new(
            "permission.denied",
            "Only the person using this computer can change its project list.",
        ));
    }
    let n = core.with_app_db(|c| {
        Ok(c.execute(
            "UPDATE sys_recent_project SET pinned=?1, updated_at=?2 WHERE project_id=?3",
            params![args.pinned as i64, now_ms(), args.project_id],
        )?)
    })?;
    if n == 0 {
        return Err(AppError::not_found("project"));
    }
    recent_entry(core, &args.project_id)
}

fn recovery_state(core: &AppCore, actor: &Actor, _: Empty) -> AppResult<RecoveryState> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    let checkpoint = s.layout.checkpoint();
    let checkpoint_at = if checkpoint.is_file() {
        file_mtime_ms(&checkpoint)
    } else {
        None
    };
    let (last_change_at, changes_since_checkpoint): (Option<i64>, Option<i64>) =
        s.store.read(|c| {
            let last: Option<i64> =
                c.query_row("SELECT MAX(at) FROM sys_activity", [], |r| r.get(0))?;
            let newer: Option<i64> = match checkpoint_at {
                Some(at) => Some(c.query_row(
                    "SELECT count(*) FROM sys_activity WHERE at > ?1",
                    [at],
                    |r| r.get(0),
                )?),
                None => None,
            };
            Ok((last, newer))
        })?;
    Ok(RecoveryState {
        offer: s.recovery.lock().clone(),
        has_checkpoint: checkpoint_at.is_some(),
        checkpoint_at,
        last_change_at,
        changes_since_checkpoint,
    })
}

fn get_settings(core: &AppCore, actor: &Actor, _: Empty) -> AppResult<ProjectSettings> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    let project = load_summary(&s)?;
    let raw: String = s
        .store
        .read(|c| Ok(c.query_row("SELECT settings_json FROM project", [], |r| r.get(0))?))?;
    let settings: Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| Value::Object(Default::default()));
    let project_notes = settings
        .get("projectNotes")
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(ProjectSettings {
        project,
        project_notes,
        settings,
    })
}

// ------------------------------------------------------ Recently Deleted view

/// Which workspace a table belongs to, for "Was in" (mock 157).
fn area_for_table(table: &str) -> &'static str {
    const AREAS: &[(&str, &str)] = &[
        ("vault_", "Idea Vault"),
        ("story_character", "Characters"),
        ("story_", "Story Board"),
        ("screenplay_", "Screenplay"),
        ("breakdown_", "Breakdown"),
        ("production_source", "Breakdown"),
        ("catalog_item", "Catalog"),
        ("location", "Locations"),
        ("cast_member", "Cast & Crew"),
        ("crew_member", "Cast & Crew"),
        ("moodboard", "Moodboards"),
        ("storyboard", "Storyboards"),
        ("shot", "Shot Lists"),
        ("shooting_day", "Shooting Schedule"),
        ("schedule", "Shooting Schedule"),
        ("strip", "Shooting Schedule"),
        ("call_sheet", "Call Sheets"),
        ("project_file", "Files"),
        ("project_note", "Notes & Tasks"),
        ("task", "Notes & Tasks"),
        ("template", "Templates"),
        ("comment", "Comments"),
        ("private_note", "Private Notes"),
        ("season", "Series"),
        ("episode", "Series"),
    ];
    AREAS
        .iter()
        .find(|(p, _)| table.starts_with(p))
        .map(|(_, a)| *a)
        .unwrap_or("Project")
}

fn humanize(object_type: &str) -> String {
    let words: Vec<String> = object_type
        .trim_start_matches("story_")
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect();
    if words.is_empty() {
        "Item".into()
    } else {
        words.join(" ")
    }
}

/// A row's display name and whether it is live (exists and is not deleted).
/// Tables are module-owned, so columns are discovered rather than assumed.
pub(crate) fn describe_row(
    c: &Connection,
    table: &str,
    id: &str,
) -> AppResult<Option<(Option<String>, bool)>> {
    if !table_exists(c, table)? {
        return Ok(None);
    }
    let cols: Vec<String> = {
        let mut stmt = c.prepare(&format!(
            "SELECT name FROM pragma_table_info('{}')",
            table.replace('\'', "''")
        ))?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    let name_col = ["title", "name", "display_name", "heading"]
        .into_iter()
        .find(|n| cols.iter().any(|c| c == n));
    let name_sql = name_col
        .map(|n| format!("\"{n}\""))
        .unwrap_or_else(|| "NULL".into());
    let deleted_sql = if cols.iter().any(|c| c == "deleted_at") {
        "deleted_at"
    } else {
        "NULL"
    };
    let t = table.replace('"', "");
    let row: Option<(Option<String>, Option<i64>)> = c
        .query_row(
            &format!("SELECT {name_sql}, {deleted_sql} FROM \"{t}\" WHERE id=?1"),
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(row.map(|(name, deleted)| (name.filter(|n| !n.trim().is_empty()), deleted.is_none())))
}

type DeletedRaw = (
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
    Option<String>,
);

fn deleted_items(
    core: &AppCore,
    actor: &Actor,
    args: crate::modules::history::StoreArgs,
) -> AppResult<Vec<DeletedItemView>> {
    actor.require(Capability::View, "view deleted items")?;
    let registry = core.registry.clone();
    crate::util::with_store(core, args.store, |s| {
        s.read(|c| {
            let rows: Vec<DeletedRaw> = {
                let mut stmt = c.prepare(
                    "SELECT id, object_type, object_id, table_name, title, parent_type, parent_id, deleted_at, deleted_by
                     FROM deleted_item ORDER BY deleted_at DESC",
                )?;
                stmt.query_map([], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?))
                })?
                .collect::<Result<_, _>>()?
            };
            let has_members = table_exists(c, "project_member")?;
            let mut out = Vec::with_capacity(rows.len());
            for (id, object_type, object_id, table, title, parent_type, parent_id, deleted_at, deleted_by) in rows {
                // Private notes in the trash are visible only to their owner (Security §8).
                if object_type == "private_note" {
                    let owner: Option<String> = c
                        .query_row("SELECT owner_user_id FROM private_note WHERE id=?1", [&object_id], |r| r.get(0))
                        .optional()?;
                    if owner.as_deref() != Some(actor.user_id.as_str()) {
                        continue;
                    }
                }
                let type_label = registry.trash_for(&object_type).map(|h| h.label.to_string()).unwrap_or_else(|| humanize(&object_type));
                let area = area_for_table(&table);
                let mut was_in = area.to_string();
                let mut parent_missing = false;
                if let (Some(pt), Some(pid)) = (&parent_type, &parent_id) {
                    let (ptable, plabel) = match registry.trash_for(pt) {
                        Some(h) => (h.table.to_string(), h.label.to_string()),
                        None => (pt.clone(), humanize(pt)),
                    };
                    match describe_row(c, &ptable, pid)? {
                        Some((name, live)) => {
                            parent_missing = !live;
                            let name = name.map(|n| format!(" “{n}”")).unwrap_or_default();
                            was_in = if area == plabel { format!("{plabel}{name}") } else { format!("{area} · {plabel}{name}") };
                        }
                        None => parent_missing = table_exists(c, &ptable)?,
                    }
                }
                let deleted_by_name: Option<String> = match (&deleted_by, has_members) {
                    (Some(uid), true) => c
                        .query_row("SELECT display_name FROM project_member WHERE user_id=?1", [uid], |r| r.get(0))
                        .optional()?,
                    _ => None,
                };
                out.push(DeletedItemView {
                    id,
                    object_type,
                    object_id,
                    title,
                    type_label,
                    was_in,
                    parent_missing,
                    deleted_at,
                    deleted_by_name,
                });
            }
            Ok(out)
        })
    })
}

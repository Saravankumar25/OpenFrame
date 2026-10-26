//! Backup Package (`.ofbackup`, FSD §44.5–44.8) and Full Project Package
//! (`.ofproject`, FSD §45): create, inspect, restore / open (as the same
//! project, as a copy, or replacing an existing project after a safety backup).
//!
//! Both are whole-project containers: a consistent online copy of the project
//! database (SQLite backup API), every managed asset, the project manifest and
//! an external-reference manifest. Linked external files are copied only when
//! the user asks for a portable package; otherwise they are listed: a package
//! never claims to contain files it does not contain (Import/Export §8.3).

use std::fs;
use std::path::{Path, PathBuf};

use openframe_domain::{
    Actor, AppError, AppResult, Capability, PACKAGE_FORMAT_VERSION, new_id, now_ms,
};
use openframe_persistence::undo::quote_ident;
use openframe_project_format::{self as pf, ProjectLayout, ProjectLock, ProjectManifest};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::format::{
    self, EntryData, PackageManifest, PackageScope, PackageType, PackageUser, Staging, WriteOutcome,
};
use super::{PackageCount, PackageTaskStarted, log_package};
use crate::core::{AppCore, ProjectSession};
use crate::store::Store;

const EXTERNAL_REFS_ENTRY: &str = "external-references.json";
const PROJECT_DIR: &str = "project";

// ------------------------------------------------------------------- DTOs

/// One file in a backup/project package summary (FSD §44.7, Import/Export §8.3).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BackupFileEntry {
    pub asset_id: String,
    pub name: String,
    /// "Included" (stored in the project), "Copied" (linked file copied into the
    /// package), "External" (linked, not copied) or "Missing".
    pub status: String,
    /// Location of a linked external file (never shown for managed files).
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub project_title: String,
    /// Project name + local date/time, e.g. "BLACK RAIN 2026-09-29 1042".
    pub base_name: String,
    pub default_dir: String,
    /// Files stored in the project that will be included.
    pub included_files: u32,
    #[ts(type = "number")]
    pub included_bytes: i64,
    /// Linked external files that exist on this computer.
    pub external: Vec<BackupFileEntry>,
    /// Files that are referenced but can't be found (never claimed as backed up).
    pub missing: Vec<BackupFileEntry>,
    /// Workspace groups included in a Full Project Package (mockup 170 checklist).
    pub groups: Vec<PackageCount>,
    /// The current user's private notes (included only in private transfers/backups).
    pub private_note_count: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BackupReport {
    pub path: String,
    pub file_name: String,
    pub package_id: String,
    pub package_type: PackageType,
    #[ts(type = "number")]
    pub bytes: i64,
    pub included_files: u32,
    pub copied_external: u32,
    pub external: Vec<BackupFileEntry>,
    pub missing: Vec<BackupFileEntry>,
    pub summary: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupCreateArgs {
    /// Full path chosen in the save dialog.
    pub path: String,
    /// Optional user label appended to the name (FSD §44.8).
    #[serde(default)]
    pub label: Option<String>,
    /// Portable package: copy linked external files into the package (FSD §44.7).
    #[serde(default)]
    pub include_external: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum PackageOpenMode {
    /// Open with the package's identity (no project with that identity exists here).
    Open,
    /// Open as an independent copy with a new project identity (default on collision).
    Copy,
    /// Replace the existing project with the same identity after a safety backup.
    Replace,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageImportProjectArgs {
    pub path: String,
    pub mode: PackageOpenMode,
    /// Folder for the opened project; defaults to Documents/OpenFrame/Projects.
    #[serde(default)]
    pub parent_dir: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackagePathArgs {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageCollision {
    pub project_id: String,
    pub title: String,
    pub path: String,
    pub is_open: bool,
}

/// What a package is, before anything is imported (FSD-COL-001: inspect a
/// package in another installation; FSD §45.3 import step "Validate").
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageInspection {
    pub manifest: PackageManifest,
    pub type_label: String,
    pub file_name: String,
    #[ts(type = "number")]
    pub size_bytes: i64,
    /// A project with the same identity already exists on this computer.
    pub collision: Option<PackageCollision>,
    /// Suggested choice: Open (no collision) or Copy (collision; favours preservation).
    pub default_mode: Option<PackageOpenMode>,
    /// Linked files of a project/backup package and their status on this computer.
    pub external: Vec<BackupFileEntry>,
    /// Exchange packages: what the package contains.
    pub content_summary: Vec<PackageCount>,
    /// The package came from the project that is open now.
    pub from_open_project: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageProjectImportReport {
    pub project_id: String,
    pub title: String,
    pub path: String,
    pub mode: PackageOpenMode,
    pub identity_kept: bool,
    pub safety_backup: Option<String>,
    pub migrated_from: Option<u32>,
    pub missing_external: Vec<BackupFileEntry>,
    pub copied_external: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExternalRef {
    asset_id: String,
    original_name: String,
    external_path: Option<String>,
    status: String,
    packaged_path: Option<String>,
}

// ------------------------------------------------------------------ helpers

/// Local "YYYY-MM-DD HHmm" stamp for package names.
pub(crate) fn local_stamp() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let fmt = time::macros::format_description!("[year]-[month]-[day] [hour][minute]");
    now.format(&fmt).unwrap_or_default()
}

struct AssetRow {
    id: String,
    mode: String,
    rel_path: Option<String>,
    external_path: Option<String>,
    original_name: String,
}

fn asset_rows(c: &Connection) -> AppResult<Vec<AssetRow>> {
    let mut stmt = c.prepare(
        "SELECT id, storage_mode, rel_path, external_path, original_name FROM asset ORDER BY created_at, id",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(AssetRow {
                id: r.get(0)?,
                mode: r.get(1)?,
                rel_path: r.get(2)?,
                external_path: r.get(3)?,
                original_name: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

struct Classified {
    /// (entry name, file on disk)
    managed: Vec<(String, PathBuf)>,
    included_bytes: u64,
    external: Vec<BackupFileEntry>,
    missing: Vec<BackupFileEntry>,
    copies: Vec<(String, PathBuf)>,
    refs: Vec<ExternalRef>,
}

fn classify(root: &Path, rows: &[AssetRow], include_external: bool) -> Classified {
    let mut out = Classified {
        managed: vec![],
        included_bytes: 0,
        external: vec![],
        missing: vec![],
        copies: vec![],
        refs: vec![],
    };
    for a in rows {
        let entry = |status: &str, path: Option<String>| BackupFileEntry {
            asset_id: a.id.clone(),
            name: a.original_name.clone(),
            status: status.to_string(),
            path,
        };
        if a.mode == "managed" {
            let rel = a.rel_path.clone().unwrap_or_default();
            match openframe_security::confine(root, &rel) {
                Ok(p) if p.is_file() => {
                    out.included_bytes += fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    out.managed
                        .push((format!("{PROJECT_DIR}/{}", rel.replace('\\', "/")), p));
                }
                _ => out.missing.push(entry("Missing", None)),
            }
            continue;
        }
        let ext_path = a.external_path.clone().unwrap_or_default();
        let exists = !ext_path.is_empty()
            && openframe_security::is_local_disk_path(Path::new(&ext_path))
            && Path::new(&ext_path).is_file();
        let mut r = ExternalRef {
            asset_id: a.id.clone(),
            original_name: a.original_name.clone(),
            external_path: a.external_path.clone(),
            status: "external".into(),
            packaged_path: None,
        };
        if !exists {
            r.status = "missing".into();
            out.missing.push(entry("Missing", Some(ext_path)));
        } else if include_external && openframe_domain::ids::is_valid_id(&a.id) {
            let name = format!(
                "external/{}/{}",
                a.id,
                openframe_security::sanitize_file_name(&a.original_name)
            );
            r.status = "copied".into();
            r.packaged_path = Some(name.clone());
            out.copies.push((name, PathBuf::from(&ext_path)));
            out.external.push(entry("Copied", Some(ext_path)));
        } else {
            out.external.push(entry("External", Some(ext_path)));
        }
        out.refs.push(r);
    }
    out
}

/// Row count of every project table (format 2 manifests). The full-text index and its shadow
/// tables are derived and left out; restore rebuilds them.
fn table_counts(c: &Connection) -> AppResult<std::collections::BTreeMap<String, i64>> {
    let mut out = std::collections::BTreeMap::new();
    for t in super::coverage::schema_columns(c)?.into_keys() {
        if t.starts_with("search_fts") || t.starts_with("sqlite_") {
            continue;
        }
        let n: i64 = c.query_row(
            &format!("SELECT count(*) FROM {}", quote_ident(&t)),
            [],
            |r| r.get(0),
        )?;
        out.insert(t, n);
    }
    Ok(out)
}

/// Format 2 packages: the database must hold exactly the rows its manifest counted. A
/// database swapped or damaged after export, with recomputed checksums, is refused.
fn check_table_counts(c: &Connection, m: &PackageManifest) -> AppResult<()> {
    if m.table_counts.is_empty() {
        return Ok(());
    }
    let here = table_counts(c)?;
    let mut wrong: Vec<String> = m
        .table_counts
        .iter()
        .filter(|(t, n)| here.get(*t) != Some(*n))
        .map(|(t, _)| t.clone())
        .collect();
    wrong.extend(
        here.keys()
            .filter(|t| !m.table_counts.contains_key(*t))
            .cloned(),
    );
    if wrong.is_empty() {
        Ok(())
    } else {
        Err(
            format::incomplete("the project database does not match its package manifest")
                .with_detail(format!("tables: {}", wrong.join(", "))),
        )
    }
}

fn table_exists(c: &Connection, t: &str) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [t],
        |r| r.get(0),
    )?)
}

fn count_live(c: &Connection, tables: &[&str]) -> AppResult<u32> {
    let mut n = 0i64;
    for t in tables {
        if !table_exists(c, t)? {
            continue;
        }
        let has_deleted: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name='deleted_at')",
            [&t],
            |r| r.get(0),
        )?;
        let sql = if has_deleted {
            format!(
                "SELECT count(*) FROM {} WHERE deleted_at IS NULL",
                quote_ident(t)
            )
        } else {
            format!("SELECT count(*) FROM {}", quote_ident(t))
        };
        n += c.query_row(&sql, [], |r| r.get::<_, i64>(0))?;
    }
    Ok(n as u32)
}

fn package_groups(c: &Connection) -> AppResult<Vec<PackageCount>> {
    let g = |label: &str, tables: &[&str]| -> AppResult<PackageCount> {
        Ok(PackageCount {
            label: label.to_string(),
            count: count_live(c, tables)?,
        })
    };
    Ok(vec![
        g("Idea Vault (project)", &["vault_item"])?,
        g(
            "Beat Board, Characters, Timeline",
            &[
                "story_act",
                "story_sequence",
                "story_beat",
                "story_scene_card",
                "story_character",
            ],
        )?,
        g(
            "Screenplay drafts, reviews and comments",
            &["screenplay_draft", "comment"],
        )?,
        g(
            "Breakdown, Catalog, Locations, Cast & Crew",
            &[
                "breakdown_element",
                "catalog_item",
                "location",
                "cast_member",
                "crew_member",
            ],
        )?,
        g(
            "Moodboards, Storyboards, Shot Lists",
            &["moodboard", "storyboard", "shot"],
        )?,
        g(
            "Schedule, Call Sheets, Sides, Notes, Tasks, Activity",
            &["shooting_day", "call_sheet", "side", "project_note", "task"],
        )?,
        g("Project Files", &["project_file"])?,
    ])
}

// ------------------------------------------------------------------- preview

pub(crate) fn backup_preview(core: &AppCore, actor: &Actor) -> AppResult<BackupPreview> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    let title = s.manifest.lock().title.clone();
    s.store.read(|c| {
        let rows = asset_rows(c)?;
        let cl = classify(&root, &rows, false);
        let private_note_count: i64 = c.query_row(
            "SELECT count(*) FROM private_note WHERE owner_user_id=?1 AND deleted_at IS NULL",
            [&actor.user_id],
            |r| r.get(0),
        )?;
        Ok(BackupPreview {
            base_name: openframe_security::sanitize_file_name(&format!(
                "{title} {}",
                local_stamp()
            )),
            project_title: title.clone(),
            default_dir: core
                .projects_dir()
                .join("Backups")
                .to_string_lossy()
                .into_owned(),
            included_files: cl.managed.len() as u32,
            included_bytes: cl.included_bytes as i64,
            external: cl.external,
            missing: cl.missing,
            groups: package_groups(c)?,
            private_note_count: private_note_count as u32,
        })
    })
}

// -------------------------------------------------------------------- create

/// Where the project data comes from.
pub enum DbSource<'a> {
    /// The open project (consistent online copy while the user keeps working).
    Store(&'a Store),
    /// A closed project whose lock the caller holds.
    Closed,
}

pub struct ProjectPackageSource<'a> {
    pub layout: ProjectLayout,
    pub manifest: ProjectManifest,
    pub db: DbSource<'a>,
}

impl<'a> ProjectPackageSource<'a> {
    pub fn open_session(session: &'a ProjectSession) -> Self {
        ProjectPackageSource {
            layout: session.layout.clone(),
            manifest: session.manifest.lock().clone(),
            db: DbSource::Store(&session.store),
        }
    }
}

pub struct ProjectPackageOptions {
    pub kind: PackageType,
    pub dest: PathBuf,
    pub label: Option<String>,
    pub include_external: bool,
    pub user: PackageUser,
    pub app_version: String,
}

/// Build a backup / full project package. Returns `None` when `cancelled`
/// reported true at any checkpoint; in that case no file (partial or
/// otherwise) is left at the destination.
pub fn build_project_package(
    src: &ProjectPackageSource<'_>,
    opts: &ProjectPackageOptions,
    progress: &dyn Fn(f64, &str),
    cancelled: &dyn Fn() -> bool,
) -> AppResult<Option<BackupReport>> {
    if !opts.kind.is_project_container() {
        return Err(AppError::internal("not a project container package type"));
    }
    if cancelled() {
        return Ok(None);
    }
    progress(0.02, "Copying project data");
    let staging = Staging::create(&src.layout.cache(), "package-staging")?;
    let db_copy = staging.path().join("project.sqlite");
    match src.db {
        DbSource::Store(store) => {
            store.with_writer(|c| openframe_persistence::backup_to(c, &db_copy))?
        }
        DbSource::Closed => {
            let c = openframe_persistence::open_connection(&src.layout.database(), false)?;
            openframe_persistence::backup_to(&c, &db_copy)?;
        }
    }
    if cancelled() {
        return Ok(None);
    }
    if opts.kind == PackageType::Project {
        // A Full Project Package is a transfer to another installation (often another
        // person): it carries the exporting user's own private notes only, never other
        // members' notes, and no undo history (row images of old text). A Backup Package
        // is the owner's private recovery copy and keeps everything (Security spec §8.7;
        // review finding PN-02). The per-table rules live in `coverage`.
        let c = openframe_persistence::open_connection(&db_copy, false)?;
        super::coverage::scrub_for_transfer(&c, &opts.user.user_id)?;
        openframe_persistence::checkpoint(&c)?;
    }
    let (rows, schema_version, table_counts) = {
        let c = openframe_persistence::open_sqlite(&db_copy, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let v = openframe_persistence::migrate::current_version(&c)?;
        (asset_rows(&c)?, v, table_counts(&c)?)
    };
    progress(0.08, "Collecting project files");
    let cl = classify(src.layout.root(), &rows, opts.include_external);
    let mut entries: Vec<(String, EntryData)> = vec![
        (
            format!("{PROJECT_DIR}/{}", pf::DB_FILE),
            EntryData::File(db_copy.clone()),
        ),
        (
            format!("{PROJECT_DIR}/{}", pf::MANIFEST_FILE),
            EntryData::Bytes(format::to_json_bytes(&src.manifest)?),
        ),
        (
            EXTERNAL_REFS_ENTRY.to_string(),
            EntryData::Bytes(format::to_json_bytes(&cl.refs)?),
        ),
    ];
    for (name, path) in &cl.managed {
        entries.push((name.clone(), EntryData::File(path.clone())));
    }
    for (name, path) in &cl.copies {
        entries.push((name.clone(), EntryData::File(path.clone())));
    }
    let manifest = PackageManifest {
        format: format::FORMAT_NAME.into(),
        package_type: opts.kind,
        format_version: PACKAGE_FORMAT_VERSION,
        package_id: new_id(),
        source_project_id: src.manifest.project_id.clone(),
        source_project_title: src.manifest.title.clone(),
        exported_at: now_ms(),
        app_version: opts.app_version.clone(),
        schema_version: Some(schema_version),
        source_draft: None,
        source_versions: vec![],
        included_object_ids: vec![],
        scope: PackageScope {
            kind: "project".into(),
            label: "Whole project".into(),
            ids: vec![],
        },
        comments_included: true,
        attachments_included: true,
        private_notes_included: true,
        base_snapshot: Default::default(),
        originating_user: opts.user.clone(),
        label: opts.label.clone(),
        responds_to: None,
        table_counts,
        carried: vec![],
    };
    let outcome = format::write_package(&opts.dest, &manifest, entries, progress, cancelled)?;
    drop(staging);
    let WriteOutcome::Written { bytes } = outcome else {
        return Ok(None);
    };
    let copied = cl.copies.len() as u32;
    let mut summary = format!("{} files stored in the project included", cl.managed.len());
    if copied > 0 {
        summary.push_str(&format!(" · {copied} linked files copied"));
    }
    let not_copied = cl
        .external
        .iter()
        .filter(|e| e.status == "External")
        .count();
    if not_copied > 0 {
        summary.push_str(&format!(
            " · {not_copied} linked files not copied (external)"
        ));
    }
    if !cl.missing.is_empty() {
        summary.push_str(&format!(" · {} missing", cl.missing.len()));
    }
    Ok(Some(BackupReport {
        path: opts.dest.to_string_lossy().into_owned(),
        file_name: opts
            .dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        package_id: manifest.package_id,
        package_type: opts.kind,
        bytes: bytes as i64,
        included_files: cl.managed.len() as u32,
        copied_external: copied,
        external: cl.external,
        missing: cl.missing,
        summary,
    }))
}

/// Keeps the "Saved with pending package operation" state for as long as the
/// background package task runs, even if it fails or panics.
struct PendingExternal(std::sync::Arc<crate::store::SaveTracker>);
impl Drop for PendingExternal {
    fn drop(&mut self) {
        self.0.external_finished();
    }
}

/// `packages.create_backup` / `packages.export_project`: background task with
/// progress and cancellation (FSD §44.5, §45.1).
pub(crate) fn start_project_package(
    core: &AppCore,
    actor: &Actor,
    args: BackupCreateArgs,
    kind: PackageType,
) -> AppResult<PackageTaskStarted> {
    actor.require(
        Capability::CreatePackage,
        "create packages from this project",
    )?;
    let session = core.project()?;
    let dest = format::package_destination(&args.path, kind)?;
    crate::util::check_output_file(core, &dest)?;
    let label = crate::util::optional_text(args.label, "Backup label", 80)?;
    let opts = ProjectPackageOptions {
        kind,
        dest: dest.clone(),
        label,
        include_external: args.include_external,
        user: PackageUser {
            user_id: actor.user_id.clone(),
            display_name: actor.display_name.clone(),
        },
        app_version: core.config.app_version.clone(),
    };
    core.save.external_started();
    let pending = PendingExternal(core.save.clone());
    let actor = actor.clone();
    let task_label = match kind {
        PackageType::Backup => "Creating backup",
        _ => "Exporting project package",
    };
    let task_id = core.spawn_task("package.export", task_label, move |_core, h| {
        let _pending = pending;
        let src = ProjectPackageSource::open_session(&session);
        let report =
            build_project_package(&src, &opts, &|f, m| h.progress(f, m), &|| h.is_cancelled())?
                .ok_or_else(AppError::cancelled)?;
        let verb = if kind == PackageType::Backup {
            "Created backup"
        } else {
            "Exported project package"
        };
        log_package(
            &session.store,
            &actor,
            "export",
            kind,
            &report.package_id,
            &report.file_name,
            &format!("{verb} “{}” ({})", report.file_name, report.summary),
            None,
        )?;
        serde_json::to_value(&report).map_err(|e| AppError::internal(e.to_string()))
    });
    Ok(PackageTaskStarted {
        task_id,
        path: dest.to_string_lossy().into_owned(),
    })
}

// -------------------------------------------------------------------- inspect

fn find_collision(core: &AppCore, project_id: &str) -> AppResult<Option<PackageCollision>> {
    if let Some(s) = core.project_opt()
        && s.project_id() == project_id
    {
        return Ok(Some(PackageCollision {
            project_id: project_id.to_string(),
            title: s.manifest.lock().title.clone(),
            path: s.layout.root().to_string_lossy().into_owned(),
            is_open: true,
        }));
    }
    let row: Option<(String, String)> = core.with_app_db(|c| {
        Ok(c.query_row(
            "SELECT path, title FROM sys_recent_project WHERE project_id=?1",
            [project_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
    })?;
    Ok(row.and_then(|(path, title)| {
        let layout = ProjectLayout::new(&path);
        let same = layout.looks_like_project()
            && pf::read_manifest(&layout)
                .map(|m| m.project_id == project_id)
                .unwrap_or(false);
        same.then(|| PackageCollision {
            project_id: project_id.to_string(),
            title,
            path,
            is_open: false,
        })
    }))
}

fn external_status_here(refs: Vec<ExternalRef>) -> Vec<BackupFileEntry> {
    refs.into_iter()
        .map(|r| {
            let status = match r.status.as_str() {
                "copied" => "Copied",
                // Paths inside a received package are untrusted: only local-disk paths are
                // probed (a UNC path would leak the user's NTLM hash on inspection, PKG-01).
                _ if r
                    .external_path
                    .as_deref()
                    .map(|p| {
                        openframe_security::is_local_disk_path(Path::new(p))
                            && Path::new(p).is_file()
                    })
                    .unwrap_or(false) =>
                {
                    "External"
                }
                _ => "Missing",
            };
            BackupFileEntry {
                asset_id: r.asset_id,
                name: r.original_name,
                status: status.to_string(),
                path: r.external_path,
            }
        })
        .collect()
}

pub(crate) fn inspect(
    core: &AppCore,
    _actor: &Actor,
    args: PackagePathArgs,
) -> AppResult<PackageInspection> {
    let path = PathBuf::from(args.path.trim());
    let (manifest, _) = format::read_header(&path)?;
    let size_bytes = fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(0);
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let from_open_project = core
        .project_opt()
        .map(|s| s.project_id() == manifest.source_project_id)
        .unwrap_or(false);
    if manifest.package_type.is_project_container() {
        let collision = find_collision(core, &manifest.source_project_id)?;
        let refs: Vec<ExternalRef> =
            openframe_security::archive::read_entry(&path, EXTERNAL_REFS_ENTRY, 64 << 20)?
                .map(|b| serde_json::from_slice(&b))
                .transpose()
                .map_err(|e| format::incomplete(format!("external references: {e}")))?
                .unwrap_or_default();
        return Ok(PackageInspection {
            type_label: manifest.package_type.label().to_string(),
            default_mode: Some(if collision.is_some() {
                PackageOpenMode::Copy
            } else {
                PackageOpenMode::Open
            }),
            collision,
            external: external_status_here(refs),
            content_summary: vec![],
            manifest,
            file_name,
            size_bytes,
            from_open_project,
        });
    }
    let content = openframe_security::archive::read_entry(&path, format::CONTENT_ENTRY, 256 << 20)?
        .ok_or_else(|| format::incomplete("content.json missing"))?;
    let content: serde_json::Value = serde_json::from_slice(&content)
        .map_err(|e| format::incomplete(format!("content: {e}")))?;
    Ok(PackageInspection {
        type_label: manifest.package_type.label().to_string(),
        collision: None,
        default_mode: None,
        external: vec![],
        content_summary: super::exchange::content_summary(manifest.package_type, &content),
        manifest,
        file_name,
        size_bytes,
        from_open_project,
    })
}

// --------------------------------------------------------------- import/open

/// ADR-0012 amendment of 2026-10-08: the person restoring a backup or Full Project Package
/// becomes the owner of every private note it holds, so they can see and edit them. Whoever
/// holds the file already holds the notes, so leaving them with an owner who is not here
/// protected nothing. The notes stay private: only the new owner sees them, in lists, search
/// and the assistant alike. Returns how many notes changed owner.
///
/// `from_owner` limits it to the notes of one owner (the Owner taking over what a former member
/// left, `project.adopt_orphan_notes`); `None` takes every note, as a restore does.
pub(crate) fn adopt_private_notes(
    c: &Connection,
    user_id: &str,
    from_owner: Option<&str>,
) -> AppResult<u32> {
    // Undo entries hold full row images with the old owner; undoing one of them would take
    // the note away from its new owner again. Same text match as the purge scrub (PN-02).
    c.execute(
        "DELETE FROM sys_undo WHERE EXISTS (
             SELECT 1 FROM private_note p
             WHERE p.owner_user_id <> ?1 AND (?2 IS NULL OR p.owner_user_id = ?2)
               AND instr(sys_undo.changes_json, p.id) > 0)",
        params![user_id, from_owner],
    )?;
    // The search index carries the owner too (Global Search shows own private notes only).
    c.execute(
        "UPDATE search_doc SET owner_user_id=?1
         WHERE source_table='private_note' AND owner_user_id IS NOT ?1
           AND (?2 IS NULL OR owner_user_id = ?2)",
        params![user_id, from_owner],
    )?;
    let moved = c.execute(
        "UPDATE private_note SET owner_user_id=?1
         WHERE owner_user_id <> ?1 AND (?2 IS NULL OR owner_user_id = ?2)",
        params![user_id, from_owner],
    )?;
    Ok(u32::try_from(moved).unwrap_or(u32::MAX))
}

pub(crate) fn start_import_project(
    core: &AppCore,
    actor: &Actor,
    args: PackageImportProjectArgs,
) -> AppResult<PackageTaskStarted> {
    let path = PathBuf::from(args.path.trim());
    // Validate structure, version and metadata synchronously so the user gets
    // an immediate, precise answer; the heavy extraction runs in the task.
    let (manifest, _) = format::read_header(&path)?;
    if !manifest.package_type.is_project_container() {
        return Err(format::wrong_kind(
            manifest.package_type,
            "a project or backup package",
        ));
    }
    let parent = args.parent_dir.map(PathBuf::from);
    let actor = actor.clone();
    let mode = args.mode;
    let label = if manifest.package_type == PackageType::Backup {
        "Restoring backup"
    } else {
        "Opening project package"
    };
    let task_id = core.spawn_task("package.import_project", label, move |core, h| {
        let report = import_project_package(
            &core,
            &actor,
            &path,
            mode,
            parent,
            &|f, m| h.progress(f, m),
            &|| h.is_cancelled(),
        )?
        .ok_or_else(AppError::cancelled)?;
        serde_json::to_value(&report).map_err(|e| AppError::internal(e.to_string()))
    });
    Ok(PackageTaskStarted {
        task_id,
        path: args.path,
    })
}

/// Validate, extract and open a backup / full project package (FSD §45.3–45.5,
/// Import/Export §8.6: Open Package → Validate → Project Identity → Collision
/// Choice → File/Reference Report → Import). Nothing existing is changed until
/// the package has been fully validated; a replaced project is first saved as
/// a safety backup and restored if the new copy can't be opened.
pub fn import_project_package(
    core: &AppCore,
    actor: &Actor,
    path: &Path,
    mode: PackageOpenMode,
    parent: Option<PathBuf>,
    progress: &dyn Fn(f64, &str),
    cancelled: &dyn Fn() -> bool,
) -> AppResult<Option<PackageProjectImportReport>> {
    let (manifest, _) = format::read_header(path)?;
    if !manifest.package_type.is_project_container() {
        return Err(format::wrong_kind(
            manifest.package_type,
            "a project or backup package",
        ));
    }
    let collision = find_collision(core, &manifest.source_project_id)?;
    match (mode, &collision) {
        (PackageOpenMode::Open, Some(_)) => {
            return Err(AppError::conflict(
                "A project with this identity already exists on this computer. Choose Open as Copy or Replace Existing After Backup.",
            ));
        }
        (PackageOpenMode::Replace, None) => {
            return Err(AppError::conflict(
                "There is no existing project to replace. Open the package instead.",
            ));
        }
        _ => {}
    }
    let parent = match (&collision, mode) {
        (Some(c), PackageOpenMode::Replace) => Path::new(&c.path)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| core.projects_dir()),
        _ => parent.unwrap_or_else(|| core.projects_dir()),
    };
    if pf::is_network_path(&parent) {
        return Err(AppError::new(
            "project_format.network_location",
            "OpenFrame projects can't be edited directly from a network folder. Choose a folder on this computer.",
        ));
    }
    // Absolute, local, no device/reserved names, not inside protected folders (PATH-03).
    crate::util::check_project_parent(core, &parent)?;
    fs::create_dir_all(&parent)?;
    if cancelled() {
        return Ok(None);
    }
    progress(0.05, "Validating package");
    let opened = format::open_package(
        path,
        &parent,
        &[PackageType::Backup, PackageType::Project],
        "a project or backup package",
    )?;
    if cancelled() {
        return Ok(None);
    }
    progress(0.5, "Checking project data");
    let root = opened.root().join(PROJECT_DIR);
    let layout = ProjectLayout::new(&root);
    let inner = pf::read_manifest(&layout)?;
    if !layout.database().is_file() {
        return Err(format::incomplete("project database missing"));
    }
    let mut conn = openframe_persistence::open_connection(&layout.database(), false)?;
    let version = openframe_persistence::migrate::current_version(&conn)?;
    let supported = crate::schema::project_schema_version();
    if version > supported {
        return Err(openframe_persistence::migrate::too_new_error(
            version, supported,
        ));
    }
    let problems = openframe_persistence::integrity_problems(&conn, false)?;
    if !problems.is_empty() {
        return Err(
            format::incomplete("project database is damaged").with_detail(problems.join("; "))
        );
    }
    // The database inside a package is untrusted: no foreign triggers/views, no hostile
    // object names (they would run or be interpolated inside OpenFrame; PKG-03).
    let sql: Vec<&str> = crate::schema::PROJECT_MIGRATIONS
        .iter()
        .map(|m| m.sql)
        .collect();
    let problems = openframe_persistence::untrusted_schema_problems(&conn, &sql)?;
    if !problems.is_empty() {
        return Err(
            format::incomplete("project database has an unexpected schema")
                .with_detail(problems.join("; ")),
        );
    }
    check_table_counts(&conn, &opened.manifest)?;
    let db_id: String = conn
        .query_row("SELECT id FROM project LIMIT 1", [], |r| r.get(0))
        .optional()?
        .ok_or_else(|| format::incomplete("project row missing"))?;
    if db_id != inner.project_id || db_id != manifest.source_project_id {
        return Err(format::incomplete("project identity mismatch"));
    }
    layout.create_dirs()?;

    // Linked files: portable copies become project-owned files; the rest keep
    // their reference and are reported when missing here (IEX-026/027).
    let refs_path = opened.root().join(EXTERNAL_REFS_ENTRY);
    // Same cap as inspection (the staged copy is untrusted package content).
    if fs::metadata(&refs_path).is_ok_and(|m| m.len() > 64 << 20) {
        return Err(format::incomplete("external references too large"));
    }
    let refs: Vec<ExternalRef> = match fs::read(&refs_path) {
        Ok(b) => serde_json::from_slice(&b)
            .map_err(|e| format::incomplete(format!("external references: {e}")))?,
        Err(_) => vec![],
    };
    let mut copied_external = 0u32;
    let mut missing_external = vec![];
    let tx = conn.transaction()?;
    for r in &refs {
        match (r.status.as_str(), &r.packaged_path) {
            ("copied", Some(packaged)) if openframe_domain::ids::is_valid_id(&r.asset_id) => {
                let src = opened.entry_path(packaged)?;
                let ext = Path::new(&r.original_name)
                    .extension()
                    .and_then(|e| e.to_str());
                let rel = ProjectLayout::new_asset_rel(&r.asset_id, ext);
                let dest = layout.asset_path(&rel)?;
                if let Some(p) = dest.parent() {
                    fs::create_dir_all(p)?;
                }
                // Move when the staged copy is on the same volume; otherwise copy through the
                // shared atomic replace so a failed copy never leaves a half-written asset.
                fs::rename(&src, &dest).or_else(|_| {
                    let mut input = fs::File::open(&src)?;
                    openframe_platform::atomic::replace_with(&dest, |out| {
                        std::io::copy(&mut input, out).map(|_| ())
                    })
                })?;
                let sha = openframe_security::sha256_file(&dest)?;
                tx.execute(
                    "UPDATE asset SET storage_mode='managed', rel_path=?1, external_path=NULL, sha256=?2, updated_at=?3 WHERE id=?4",
                    params![rel, sha, now_ms(), r.asset_id],
                )?;
                copied_external += 1;
            }
            _ => {
                let here = r
                    .external_path
                    .as_deref()
                    .map(|p| {
                        openframe_security::is_local_disk_path(Path::new(p))
                            && Path::new(p).is_file()
                    })
                    .unwrap_or(false);
                if !here {
                    missing_external.push(BackupFileEntry {
                        asset_id: r.asset_id.clone(),
                        name: r.original_name.clone(),
                        status: "Missing".into(),
                        path: r.external_path.clone(),
                    });
                }
            }
        }
    }
    // Same transaction as the rest of the restore, on the staged copy: a crash before the
    // commit leaves no project at all, never one with half its notes moved.
    let adopted_notes = adopt_private_notes(&tx, &actor.user_id, None)?;
    let now = now_ms();
    let file_name = opened.file_name.clone();
    let (project_id, title) = if mode == PackageOpenMode::Copy {
        let new_pid = new_id();
        let title = format!("{} (copy)", inner.title);
        tx.execute(
            "UPDATE project SET id=?1, title=?2, archived=0, updated_at=?3",
            params![new_pid, title, now],
        )?;
        tx.execute("DELETE FROM sys_undo", [])?;
        // Without history an unfinished AI apply can no longer be compensated (ADR-0019).
        crate::modules::ai::change_set::abandon_open_journals(&tx)?;
        (new_pid, title)
    } else {
        (inner.project_id.clone(), inner.title.clone())
    };
    let what = match (manifest.package_type, mode) {
        (_, PackageOpenMode::Copy) => format!("Opened “{file_name}” as a copy with a new identity"),
        (PackageType::Backup, _) => format!("Restored from backup “{file_name}”"),
        _ => format!("Opened from project package “{file_name}”"),
    };
    tx.execute(
        "INSERT INTO sys_activity(id, at, actor_id, actor_name, action, summary) VALUES (?1, ?2, ?3, ?4, 'packages.import_project', ?5)",
        params![new_id(), now, actor.user_id, actor.display_name, what],
    )?;
    tx.commit()?;
    openframe_persistence::checkpoint(&conn)?;
    drop(conn);
    if mode == PackageOpenMode::Copy {
        let mut m = ProjectManifest::new(&project_id, &title, &core.config.app_version, version);
        m.created_at = now;
        pf::write_manifest(&layout, &m)?;
    }
    if cancelled() {
        return Ok(None);
    }
    progress(0.8, "Opening project");

    let (dest, safety_backup, displaced) = match (mode, &collision) {
        (PackageOpenMode::Replace, Some(existing)) => {
            let existing_root = PathBuf::from(&existing.path);
            if existing.is_open {
                crate::modules::project::close_current(core)?;
            }
            let existing_layout = ProjectLayout::new(&existing_root);
            let backup_path = {
                let lock = ProjectLock::acquire(&existing_layout)?;
                let backups = core.projects_dir().join("Backups");
                fs::create_dir_all(&backups)?;
                let name = openframe_security::sanitize_file_name(&format!(
                    "{} before replace {}",
                    existing.title,
                    local_stamp()
                ));
                let dest = format::package_destination(
                    &backups.join(format!("{name}.ofbackup")).to_string_lossy(),
                    PackageType::Backup,
                )?;
                let src = ProjectPackageSource {
                    manifest: pf::read_manifest(&existing_layout)?,
                    layout: existing_layout.clone(),
                    db: DbSource::Closed,
                };
                let opts = ProjectPackageOptions {
                    kind: PackageType::Backup,
                    dest: dest.clone(),
                    label: Some("before replace".into()),
                    include_external: false,
                    user: PackageUser {
                        user_id: actor.user_id.clone(),
                        display_name: actor.display_name.clone(),
                    },
                    app_version: core.config.app_version.clone(),
                };
                build_project_package(&src, &opts, &|_, _| {}, &|| false)?
                    .ok_or_else(|| AppError::internal("safety backup did not complete"))?;
                // Verify the safety backup is a readable, well-formed package before the
                // existing project is touched (never replace on an unverified backup).
                let (bm, _) = format::read_header(&dest)?;
                if bm.package_type != PackageType::Backup
                    || bm.source_project_id != existing.project_id
                {
                    return Err(AppError::internal("safety backup failed verification"));
                }
                drop(lock);
                dest
            };
            let displaced = existing_root.with_file_name(format!(
                ".{}.replaced-{}",
                existing_root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                new_id()
            ));
            fs::rename(&existing_root, &displaced).map_err(|e| {
                AppError::new(
                    "storage.replace_failed",
                    "OpenFrame couldn't replace the existing project. It was not changed.",
                )
                .with_detail(e.to_string())
            })?;
            if let Err(e) = fs::rename(&root, &existing_root) {
                if let Err(back) = put_back(&displaced, &existing_root, &existing.title) {
                    tracing::error!(error = %back, "the replaced project could not be put back");
                    return Err(
                        stranded_error(&displaced, &existing_root).with_detail(e.to_string())
                    );
                }
                return Err(AppError::new(
                    "storage.replace_failed",
                    "OpenFrame couldn't replace the existing project. It was not changed.",
                )
                .with_detail(e.to_string()));
            }
            (existing_root, Some(backup_path), Some(displaced))
        }
        _ => {
            let dest = pf::unique_project_dir(&parent, &title);
            fs::rename(&root, &dest)?;
            (dest, None, None)
        }
    };
    drop(opened);
    let replaced_title = collision
        .as_ref()
        .map(|c| c.title.clone())
        .unwrap_or_default();
    let opened_result = if displaced.is_some() && fault::open_fails(&replaced_title) {
        Err(AppError::internal("simulated open failure (test hook)"))
    } else {
        crate::modules::project::open_at(core, actor, &dest)
    };
    let result = match opened_result {
        Ok(r) => r,
        Err(e) => {
            // Never leave a half-imported project; put a replaced project back.
            if let Err(rm) = fs::remove_dir_all(&dest)
                && rm.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!(error = %rm, "a half-imported project could not be removed");
            }
            if let Some(old) = &displaced
                && let Err(back) = put_back(old, &dest, &replaced_title)
            {
                tracing::error!(error = %back, "the replaced project could not be put back");
                return Err(stranded_error(old, &dest).with_detail(e.message));
            }
            return Err(e);
        }
    };
    // The old copy stays hidden next to the project if it cannot be removed; say so, never silently.
    let leftover = displaced.and_then(|old| match fs::remove_dir_all(&old) {
        Ok(()) => None,
        Err(e) => {
            tracing::warn!(error = %e, "the replaced project's old copy could not be removed");
            Some(old)
        }
    });
    if let Some(s) = core.project_opt() {
        let local = core.local_actor();
        if let Err(e) = log_package(
            &s.store,
            &local,
            "import",
            manifest.package_type,
            &manifest.package_id,
            &file_name,
            &what,
            None,
        ) {
            // The project is open and whole; only the Activity line is missing.
            tracing::warn!(code = %e.code.0, "the package import could not be added to Activity");
        }
    }
    let mut message = match (mode, &collision) {
        (PackageOpenMode::Copy, Some(_)) => {
            "The project package was imported as a copy. The original project was not replaced."
                .to_string()
        }
        (PackageOpenMode::Copy, None) => {
            "The project was opened as a copy with a new identity.".to_string()
        }
        (PackageOpenMode::Replace, _) => {
            "The existing project was replaced. A safety backup of it was saved first.".to_string()
        }
        (PackageOpenMode::Open, _) => "The project was opened on this computer.".to_string(),
    };
    if let Some(old) = &leftover {
        message.push_str(&format!(
            " A hidden copy of the old project is still in \"{}\"; you can delete it.",
            old.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    if adopted_notes > 0 {
        message.push_str(&format!(
            " {adopted_notes} private note{} from the package {} now yours.",
            if adopted_notes == 1 { "" } else { "s" },
            if adopted_notes == 1 { "is" } else { "are" }
        ));
    }
    if !missing_external.is_empty() {
        message.push_str(&format!(
            " {} linked external file{} missing on this PC (the project still opens).",
            missing_external.len(),
            if missing_external.len() == 1 {
                " is"
            } else {
                "s are"
            }
        ));
    }
    Ok(Some(PackageProjectImportReport {
        project_id: result.project.id.clone(),
        title: result.project.title.clone(),
        path: dest.to_string_lossy().into_owned(),
        mode,
        identity_kept: mode != PackageOpenMode::Copy,
        safety_backup: safety_backup.map(|p| p.to_string_lossy().into_owned()),
        migrated_from: result.migrated_from,
        missing_external,
        copied_external,
        message,
    }))
}

/// Put a replaced project back where it was after a replace failed.
fn put_back(displaced: &Path, original: &Path, title: &str) -> std::io::Result<()> {
    if fault::restore_fails(title) {
        return Err(std::io::Error::other(
            "simulated rename failure (test hook)",
        ));
    }
    fs::rename(displaced, original)
}

/// The replaced project could not be put back. It is whole, in a hidden folder next to where it
/// was; the message names that folder and says how to get the project back, so nothing is lost.
fn stranded_error(displaced: &Path, original: &Path) -> AppError {
    let name = |p: &Path| {
        p.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    };
    let place = displaced
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    AppError::new(
        "storage.replace_rollback_failed",
        format!(
            "OpenFrame couldn't replace the existing project and couldn't put it back in place. \
             Your project is safe in the hidden folder \"{}\" in {place}. Rename that folder to \"{}\" to use it again.",
            name(displaced),
            name(original)
        ),
    )
}

/// Test hooks for the replace path (debug builds only). They act only on the project with the
/// given title, so other tests running at the same time are not touched. Release builds compile
/// both checks to `false`.
pub mod fault {
    #[cfg(debug_assertions)]
    static STRAND: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

    /// Make replacing the project with this title fail after the old folder was moved aside, and
    /// make putting it back fail too (`None` disarms).
    #[cfg(debug_assertions)]
    pub fn strand_replace_of(title: Option<&str>) {
        *STRAND
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = title.map(str::to_string);
    }

    fn armed(title: &str) -> bool {
        #[cfg(debug_assertions)]
        {
            STRAND
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_deref()
                == Some(title)
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = title;
            false
        }
    }

    pub(super) fn open_fails(title: &str) -> bool {
        armed(title)
    }

    pub(super) fn restore_fails(title: &str) -> bool {
        armed(title)
    }
}

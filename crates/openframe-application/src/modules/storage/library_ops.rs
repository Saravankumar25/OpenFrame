//! The OpenFrame library (doc 40 section 1, ADR-0038): one folder that holds `library.json`,
//! `Projects/` and `Global Idea Vault/`. The first start asks where it is; nothing is created
//! before the person confirms. The choice is remembered in `app.sqlite` and in a pointer file
//! outside the app data (`%APPDATA%\OpenFrame\library-pointer.json`), so a reinstall finds it.
//!
//! Operations: `library.status`, `library.inspect`, `library.use_default`, `library.create_at`,
//! `library.connect` and `library.move`. All are direct user actions, hidden from the assistant.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use openframe_domain::{Actor, AppError, AppResult, now_ms};
use openframe_project_format::{self as pf, ProjectLayout};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::relocate::{
    MoveToFolderArgs, PROJECT_SKIP, check_destination_free, check_project_copy, confirm_open_at,
    rollback_failed, same_place_error, staging_path,
};
use super::vault_archive::check_vault_database;
use super::{StagingNote, StorageTaskStarted, checked_folder, library, probe_writable, tree};
use crate::core::{AppConfig, AppCore, StorageOp, TaskHandle, storage_busy};
use crate::modules::packages::format::Staging;
use crate::modules::project::{self, Empty};
use crate::registry::Registry;

pub const LIBRARY_FOLDER: &str = "OpenFrame";
pub const PROJECTS_FOLDER: &str = "Projects";
pub const VAULT_FOLDER: &str = "Global Idea Vault";
pub const POINTER_FILE: &str = "library-pointer.json";
/// `sys_settings` key that remembers the library folder in `app.sqlite`.
const ROOT_SETTING: &str = "storage.library_root";
const POINTER_FORMAT: u32 = 1;
const POINTER_MAX_BYTES: u64 = 64 * 1024;
/// Folders looked at when counting projects or finding a library one level down.
const SCAN_LIMIT: usize = 2_000;

/// Where the first start looks and what it suggests.
#[derive(Debug, Clone)]
pub struct LibrarySetup {
    /// `%APPDATA%\OpenFrame\library-pointer.json`; on macOS
    /// `~/Library/Application Support/OpenFrame Library/library-pointer.json`.
    pub pointer_file: PathBuf,
    /// `<home>\OpenFrame`: outside Documents, so OneDrive does not sync it.
    pub suggested_root: PathBuf,
    /// Where earlier versions kept the library (`Documents\OpenFrame`, also when Documents is
    /// redirected to OneDrive).
    pub earlier_roots: Vec<PathBuf>,
}

impl LibrarySetup {
    pub fn for_current_user(app_data_dir: &Path) -> Option<Self> {
        use openframe_platform::dirs;
        let home = dirs::home_dir()?;
        let pointer_file = dirs::library_pointer_dir()
            .unwrap_or_else(|| app_data_dir.to_path_buf())
            .join(POINTER_FILE);
        let mut earlier: Vec<PathBuf> = dirs::documents_dir()
            .map(|d| d.join(LIBRARY_FOLDER))
            .into_iter()
            .collect();
        earlier.push(home.join("Documents").join(LIBRARY_FOLDER));
        if cfg!(windows) {
            earlier.push(home.join("OneDrive").join("Documents").join(LIBRARY_FOLDER));
        }
        Some(Self {
            pointer_file,
            suggested_root: home.join(LIBRARY_FOLDER),
            earlier_roots: earlier,
        })
    }

    /// Everything inside `root` (tests and the end-to-end switch).
    pub fn under(root: &Path) -> Self {
        Self {
            pointer_file: root.join("roaming").join(LIBRARY_FOLDER).join(POINTER_FILE),
            suggested_root: root.join("home").join(LIBRARY_FOLDER),
            earlier_roots: vec![root.join("Documents").join(LIBRARY_FOLDER)],
        }
    }
}

/// The library in use: its folder, its file and its own default folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPlace {
    pub root: PathBuf,
    pub file: Option<PathBuf>,
    pub projects_dir: PathBuf,
    pub vault_dir: PathBuf,
}

impl LibraryPlace {
    /// The standard layout at `root`.
    pub fn at(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            file: Some(root.join(library::LIBRARY_FILE_NAME)),
            projects_dir: root.join(PROJECTS_FOLDER),
            vault_dir: root.join(VAULT_FOLDER),
        }
    }

    /// The same library after its folder moved to `root`.
    pub fn moved_to(&self, root: &Path) -> Self {
        let follow =
            |p: &Path| library::rebase(p, &self.root, root).unwrap_or_else(|| p.to_path_buf());
        Self {
            root: root.to_path_buf(),
            file: self.file.as_deref().map(follow),
            projects_dir: follow(&self.projects_dir),
            vault_dir: follow(&self.vault_dir),
        }
    }

    /// Tests and the end-to-end data root: set up from the start, laid out as the config says.
    fn fixed(config: &AppConfig) -> Self {
        let root = config
            .library_file
            .as_deref()
            .and_then(Path::parent)
            .or_else(|| config.projects_dir.parent())
            .unwrap_or(&config.projects_dir)
            .to_path_buf();
        Self {
            root,
            file: config.library_file.clone(),
            projects_dir: config.projects_dir.clone(),
            vault_dir: config.global_vault_dir.clone(),
        }
    }
}

// ------------------------------------------------------------- remembering

/// The library at start: the remembered folder when it is there. In a test configuration the
/// library is always set up (at the remembered folder after a move, else its fixed folder).
pub(crate) fn place_at_start(conn: &Connection, config: &AppConfig) -> Option<LibraryPlace> {
    let from_db = read_db_root(conn);
    let Some(setup) = &config.library_setup else {
        let fixed = LibraryPlace::fixed(config);
        return Some(match from_db.filter(|r| r.is_dir()) {
            Some(r) if !library::same_place(&r, &fixed.root) => fixed.moved_to(&r),
            _ => fixed,
        });
    };
    let from_pointer = read_pointer(&setup.pointer_file);
    let root = from_db.clone().or_else(|| from_pointer.clone())?;
    if !root.is_dir() {
        return None;
    }
    // Keep both records in step (best effort): a reinstall has only the pointer, an older
    // pointer may be missing.
    if from_db.is_none() && write_db_root(conn, &root).is_err() {
        tracing::warn!("the library folder could not be recorded in the app database");
    }
    let pointer_ok = from_pointer.is_some_and(|p| library::same_place(&p, &root));
    if !pointer_ok && write_pointer(&setup.pointer_file, &root).is_err() {
        tracing::warn!("the library pointer could not be written");
    }
    Some(LibraryPlace::at(&root))
}

/// The folder the app last used, whether or not it is there now.
fn remembered_root(core: &AppCore) -> Option<PathBuf> {
    let from_db = core.with_app_db(|c| Ok(read_db_root(c))).ok().flatten();
    from_db.or_else(|| {
        core.config
            .library_setup
            .as_ref()
            .and_then(|s| read_pointer(&s.pointer_file))
    })
}

/// Record the library folder in `app.sqlite` and in the pointer. Both or neither: a pointer that
/// cannot be written puts the database value back.
pub(crate) fn remember_root(core: &AppCore, root: Option<&Path>) -> AppResult<()> {
    let Some(root) = root else {
        return Ok(());
    };
    let before = core.with_app_db(|c| Ok(read_db_root(c)))?;
    core.with_app_db(|c| write_db_root(c, root))?;
    if let Some(setup) = &core.config.library_setup
        && let Err(e) = write_pointer(&setup.pointer_file, root)
    {
        let _ = core.with_app_db(|c| match &before {
            Some(b) => write_db_root(c, b),
            None => Ok(c
                .execute("DELETE FROM sys_settings WHERE key=?1", [ROOT_SETTING])
                .map(|_| ())?),
        });
        return Err(AppError::new(
            "storage.not_writable",
            "OpenFrame could not save where your library is. Nothing was changed. Try again.",
        )
        .with_detail(e.message));
    }
    Ok(())
}

fn usable_root(raw: &str) -> Option<PathBuf> {
    let p = PathBuf::from(raw);
    (p.is_absolute()
        && openframe_security::check_user_path(&p, openframe_security::NetworkPaths::Refuse)
            .is_ok())
    .then_some(p)
}

fn read_db_root(conn: &Connection) -> Option<PathBuf> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value_json FROM sys_settings WHERE key=?1",
            [ROOT_SETTING],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let text: String = serde_json::from_str(&raw?).ok()?;
    usable_root(&text)
}

fn write_db_root(conn: &Connection, root: &Path) -> AppResult<()> {
    let value = serde_json::Value::String(root.to_string_lossy().into_owned()).to_string();
    conn.execute(
        "INSERT INTO sys_settings(key, value_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
        params![ROOT_SETTING, value, now_ms()],
    )?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pointer {
    format: u32,
    library_root: String,
    updated_at: i64,
}

/// The pointer file names an absolute local folder; anything else is ignored (logged).
pub fn read_pointer(file: &Path) -> Option<PathBuf> {
    if fs::metadata(file).ok()?.len() > POINTER_MAX_BYTES {
        tracing::warn!("the library pointer is too large and is ignored");
        return None;
    }
    let parsed: Option<Pointer> = fs::read_to_string(file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok());
    let root = parsed
        .filter(|p| p.format == POINTER_FORMAT)
        .and_then(|p| usable_root(&p.library_root));
    if root.is_none() {
        tracing::warn!("the library pointer is not valid and is ignored");
    }
    root
}

fn write_pointer(file: &Path, root: &Path) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(&Pointer {
        format: POINTER_FORMAT,
        library_root: root.to_string_lossy().into_owned(),
        updated_at: now_ms(),
    })
    .map_err(|e| AppError::internal(e.to_string()))?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    openframe_platform::atomic::replace_bytes(file, &bytes)?;
    Ok(())
}

// ------------------------------------------------------------------ the gate

/// Until the library is set up only these run: the setup screen, the team password and licence
/// screens, and what draws the window.
pub(crate) fn allowed_before_setup(op: &str) -> bool {
    ["library.", "team_gate.", "entitlement.", "appearance."]
        .iter()
        .any(|p| op.starts_with(p))
        || matches!(op, "app.info" | "app.save_state")
}

pub(crate) fn setup_required() -> AppError {
    AppError::new(
        "library.setup_required",
        "Choose where OpenFrame keeps your work first. Your OpenFrame library is not set up yet.",
    )
}

// ------------------------------------------------------------------- DTOs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum LibraryState {
    Ready,
    NeedsSetup,
    /// The remembered library folder is not there (a drive that is not connected, a moved folder).
    Missing,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    pub state: LibraryState,
    /// Ready: the library in use. Missing: where it was expected.
    pub root: Option<String>,
    /// The suggested place for a new library (`<home>\OpenFrame`).
    pub suggested: String,
    /// The service that syncs the library in use, when one does ("OneDrive").
    pub cloud_service: Option<String>,
    /// Existing libraries found on this computer (not Ready only).
    pub found: Vec<LibraryInspection>,
}

/// What a folder the person picked turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum LibraryPick {
    /// The library folder itself.
    LibraryRoot,
    /// The folder that holds the library (for example the home folder).
    LibraryParent,
    /// A folder inside the library (`Projects`, the vault).
    InsideLibrary,
    /// One project folder.
    Project,
    /// A Global Idea Vault on its own.
    Vault,
    Empty,
    Other,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInspection {
    /// The folder that was looked at.
    pub path: String,
    pub kind: LibraryPick,
    /// The library folder, when it could be identified safely. Connect uses this one.
    pub library_root: Option<String>,
    pub has_library_file: bool,
    /// The name in the library file, so the person can tell whose library it is.
    pub owner_name: Option<String>,
    pub project_count: u32,
    pub vault_found: bool,
    #[ts(type = "number | null")]
    pub idea_count: Option<i64>,
    pub cloud_service: Option<String>,
    /// What was found, in plain words.
    pub message: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryDirArgs {
    pub dir: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryCreateArgs {
    /// The folder that will hold the new `OpenFrame` library folder.
    pub parent: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMoveReport {
    pub path: String,
    /// Projects inside the library that moved with it.
    pub projects_moved: u32,
    pub vault_moved: bool,
    /// Set when the old folder still holds something OpenFrame did not copy.
    pub source_left_at: Option<String>,
}

// -------------------------------------------------------------- registration

pub(super) fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    r.query("library.status", status).meta(
        M::app_query(
            "Whether the OpenFrame library is set up, and libraries found on this computer.",
        )
        .fs(Fs::ProjectStorage)
        .hidden(h::USER_PATH),
    );
    r.query("library.inspect", inspect).meta(
        M::app_query("What a folder holds: an OpenFrame library, a project, a vault, or nothing.")
            .fs(Fs::ProjectStorage)
            .hidden(h::USER_PATH),
    );
    r.command("library.use_default", use_default).meta(
        M::app_command("Set up the OpenFrame library in the suggested folder.")
            .fs(Fs::ProjectStorage)
            .hidden(h::USER_PATH),
    );
    r.command("library.create_at", create_at).meta(
        M::app_command("Set up a new OpenFrame library in a folder the person picked.")
            .fs(Fs::ProjectStorage)
            .hidden(h::USER_PATH),
    );
    r.command("library.connect", connect).meta(
        M::app_command("Connect an OpenFrame library that already exists.")
            .fs(Fs::ProjectStorage)
            .hidden(h::USER_PATH),
    );
    r.command("library.move", move_library_op).meta(
        M::app_command("Move the whole OpenFrame library to another folder.")
            .fs(Fs::ProjectStorage)
            .long_running()
            .destructive()
            .confirm()
            .hidden(h::USER_PATH),
    );
}

// -------------------------------------------------------------------- status

fn status(core: &AppCore, _a: &Actor, _: Empty) -> AppResult<LibraryStatus> {
    Ok(library_status(core))
}

fn cloud_name(p: &Path) -> Option<String> {
    openframe_platform::cloud::cloud_service(p).map(|s| s.name().to_string())
}

pub(crate) fn library_status(core: &AppCore) -> LibraryStatus {
    let text = |p: &Path| p.to_string_lossy().into_owned();
    let suggested = core
        .config
        .library_setup
        .as_ref()
        .map(|s| s.suggested_root.clone());
    if let Some(place) = core.library_place() {
        return LibraryStatus {
            state: LibraryState::Ready,
            cloud_service: cloud_name(&place.root),
            suggested: text(suggested.as_deref().unwrap_or(&place.root)),
            root: Some(text(&place.root)),
            found: Vec::new(),
        };
    }
    let expected = remembered_root(core);
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(s) = &core.config.library_setup {
        candidates.push(s.suggested_root.clone());
        candidates.extend(s.earlier_roots.iter().cloned());
    }
    let mut seen = std::collections::HashSet::new();
    let found = candidates
        .into_iter()
        .filter(|c| seen.insert(library::place_key(c)) && is_library_root(c))
        .filter_map(|c| inspect_dir(core, &c).ok())
        .collect();
    LibraryStatus {
        state: if expected.is_some() {
            LibraryState::Missing
        } else {
            LibraryState::NeedsSetup
        },
        root: expected.as_deref().map(text),
        suggested: text(suggested.as_deref().unwrap_or(&core.config.projects_dir)),
        cloud_service: None,
        found,
    }
}

// -------------------------------------------------------------------- inspect

/// `library.json`, or both `Projects` and the vault folder.
pub fn is_library_root(dir: &Path) -> bool {
    dir.join(library::LIBRARY_FILE_NAME).is_file()
        || (dir.join(PROJECTS_FOLDER).is_dir() && dir.join(VAULT_FOLDER).is_dir())
}

fn checked_dir(raw: &str) -> AppResult<PathBuf> {
    let dir = PathBuf::from(raw.trim());
    openframe_security::check_user_path(&dir, openframe_security::NetworkPaths::Refuse)
        .map_err(project::network_or)?;
    if !dir.is_dir() {
        return Err(AppError::new(
            "not_found.folder",
            "That folder can't be found. If it's on an external drive, reconnect it and try again.",
        ));
    }
    Ok(dir)
}

fn inspect(core: &AppCore, _a: &Actor, args: LibraryDirArgs) -> AppResult<LibraryInspection> {
    inspect_dir(core, &checked_dir(&args.dir)?)
}

/// Look at `dir` without changing anything: classify it, find the library folder when that can
/// be done safely, and count what the library holds.
pub(crate) fn inspect_dir(core: &AppCore, dir: &Path) -> AppResult<LibraryInspection> {
    let parent_root = dir
        .parent()
        .filter(|p| is_library_root(p))
        .map(Path::to_path_buf);
    let (kind, root) = if is_library_root(dir) {
        (LibraryPick::LibraryRoot, Some(dir.to_path_buf()))
    } else if dir.join(pf::MANIFEST_FILE).is_file() {
        let grand = dir
            .parent()
            .and_then(Path::parent)
            .filter(|g| is_library_root(g));
        (LibraryPick::Project, grand.map(Path::to_path_buf))
    } else if let Some(root) = parent_root {
        (LibraryPick::InsideLibrary, Some(root))
    } else if dir.join("vault.sqlite").is_file() {
        (LibraryPick::Vault, None)
    } else if let Some(child) = one_library_inside(dir) {
        (LibraryPick::LibraryParent, Some(child))
    } else if super::tree::is_empty_but_for_finder_files(dir)? {
        (LibraryPick::Empty, None)
    } else {
        (LibraryPick::Other, None)
    };
    // The app's own folder is never a library.
    let root = root.filter(|r| !library::is_inside(&core.config.app_data_dir, r));
    let text = |p: &Path| p.to_string_lossy().into_owned();
    let mut out = LibraryInspection {
        path: text(dir),
        kind,
        library_root: root.as_deref().map(text),
        has_library_file: false,
        owner_name: None,
        project_count: 0,
        vault_found: false,
        idea_count: None,
        cloud_service: cloud_name(root.as_deref().unwrap_or(dir)),
        message: String::new(),
    };
    if let Some(root) = &root {
        let file = root.join(library::LIBRARY_FILE_NAME);
        out.has_library_file = file.is_file();
        let lib = match library::read(&file, &core.config.app_data_dir) {
            library::Read::Valid(l) => Some(l),
            _ => None,
        };
        out.owner_name = lib.as_ref().map(|l| l.display_name.clone());
        let projects = lib
            .as_ref()
            .and_then(|l| l.projects_dir.clone())
            .unwrap_or_else(|| root.join(PROJECTS_FOLDER));
        let vault = lib
            .and_then(|l| l.vault_dir)
            .unwrap_or_else(|| root.join(VAULT_FOLDER));
        out.project_count = count_projects(&projects);
        let db = vault.join("vault.sqlite");
        out.vault_found = db.is_file();
        out.idea_count = out.vault_found.then(|| count_ideas(&db)).flatten();
    } else if kind == LibraryPick::Vault {
        out.vault_found = true;
        out.idea_count = count_ideas(&dir.join("vault.sqlite"));
    }
    out.message = message(&out);
    Ok(out)
}

/// The one library directly inside `dir` (`OpenFrame` first), when there is exactly one.
fn one_library_inside(dir: &Path) -> Option<PathBuf> {
    let named = dir.join(LIBRARY_FOLDER);
    if is_library_root(&named) {
        return Some(named);
    }
    let found: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .take(SCAN_LIMIT)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .filter(|p| is_library_root(p))
        .take(2)
        .collect();
    (found.len() == 1).then(|| found[0].clone())
}

/// Project folders in `dir` and one level below (as Home's "Found on this PC" looks).
fn count_projects(dir: &Path) -> u32 {
    let mut count = 0u32;
    let mut level = vec![dir.to_path_buf()];
    let mut visited = 0usize;
    for _ in 0..2 {
        let mut next = Vec::new();
        for parent in &level {
            let Ok(entries) = fs::read_dir(parent) else {
                continue;
            };
            for e in entries.flatten() {
                if !e.file_type().is_ok_and(|t| t.is_dir()) {
                    continue;
                }
                visited += 1;
                if visited > SCAN_LIMIT {
                    return count;
                }
                if e.path().join(pf::MANIFEST_FILE).is_file() {
                    count += 1;
                } else {
                    next.push(e.path());
                }
            }
        }
        level = next;
    }
    count
}

/// Ideas in a vault, read without writing anything. `None` when it cannot be read.
fn count_ideas(db: &Path) -> Option<i64> {
    // `immutable=1`: a plain read-only open of a WAL database still creates `-wal` and `-shm`
    // beside it. shortcut: changes still in a `-wal` (after a crash) are not counted; the
    // number is only a preview.
    let mut path = db.to_string_lossy().replace('\\', "/");
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    let path = path
        .replace('%', "%25")
        .replace('?', "%3f")
        .replace('#', "%23");
    let conn = Connection::open_with_flags(
        format!("file://{path}?immutable=1"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .ok()?;
    conn.query_row(
        "SELECT count(*) FROM vault_item WHERE deleted_at IS NULL",
        [],
        |r| r.get(0),
    )
    .ok()
}

fn message(i: &LibraryInspection) -> String {
    let what = || {
        let projects = match i.project_count {
            1 => "1 project".to_string(),
            n => format!("{n} projects"),
        };
        let vault = match (i.vault_found, i.idea_count) {
            (true, Some(1)) => "a Global Idea Vault with 1 idea".to_string(),
            (true, Some(n)) => format!("a Global Idea Vault with {n} ideas"),
            (true, None) => "a Global Idea Vault".to_string(),
            (false, _) => "no Global Idea Vault yet".to_string(),
        };
        format!("{projects} and {vault}")
    };
    let root = i.library_root.as_deref().unwrap_or_default();
    match i.kind {
        LibraryPick::LibraryRoot => format!("This is an OpenFrame library with {}.", what()),
        LibraryPick::LibraryParent => format!(
            "This folder holds an OpenFrame library in “{root}”, with {}. That folder will be connected.",
            what()
        ),
        LibraryPick::InsideLibrary => format!(
            "This folder is inside an OpenFrame library. The library is “{root}”, with {}. That folder will be connected.",
            what()
        ),
        LibraryPick::Project if i.library_root.is_some() => format!(
            "This folder is one project. It belongs to the OpenFrame library “{root}”, with {}. That library will be connected.",
            what()
        ),
        LibraryPick::Project => "This folder is one project, not a library. Choose the folder that holds your library, or set up a library first and then open this project from Home.".to_string(),
        LibraryPick::Vault => "This folder is a Global Idea Vault on its own, not a library. Set up a library first, then connect this vault in Settings > Storage with Locate Existing Vault.".to_string(),
        LibraryPick::Empty => "This folder is empty. There is no OpenFrame library here.".to_string(),
        LibraryPick::Other => "No OpenFrame library was found in this folder. Choose the folder named OpenFrame that holds library.json and the Projects folder.".to_string(),
    }
}

// ---------------------------------------------------------- set up and connect

fn refuse_when_ready(core: &AppCore) -> AppResult<()> {
    if core.library_ready() {
        return Err(AppError::conflict(
            "Your OpenFrame library is already set up. To use another folder, move the library or locate an existing one in Settings > Storage.",
        ));
    }
    Ok(())
}

fn use_default(core: &AppCore, _a: &Actor, _: Empty) -> AppResult<LibraryStatus> {
    refuse_when_ready(core)?;
    let Some(setup) = &core.config.library_setup else {
        return Err(setup_required());
    };
    let root = setup.suggested_root.clone();
    if is_library_root(&root) {
        return connect_root(core, &root);
    }
    let parent = root
        .parent()
        .ok_or_else(|| AppError::invalid_input("That folder can't hold a library."))?;
    fs::create_dir_all(parent)?;
    create_root(core, &root)
}

fn create_at(core: &AppCore, _a: &Actor, args: LibraryCreateArgs) -> AppResult<LibraryStatus> {
    refuse_when_ready(core)?;
    let parent = checked_folder(core, &args.parent)?;
    if let Some(lib) = parent.ancestors().find(|a| is_library_root(a)) {
        return Err(AppError::invalid_input(format!(
            "That folder is inside the OpenFrame library “{}”. Choose a folder outside it, or use Locate existing OpenFrame data to connect that library.",
            lib.display()
        )));
    }
    create_root(core, &parent.join(LIBRARY_FOLDER))
}

/// A new, empty library at `root`. Refuses a folder that already holds anything; what this
/// created is removed again when setting it up fails.
fn create_root(core: &AppCore, root: &Path) -> AppResult<LibraryStatus> {
    if is_library_root(root) {
        return Err(AppError::new(
            "conflict.exists",
            format!(
                "“{}” already holds an OpenFrame library. Use Locate existing OpenFrame data to connect it.",
                root.display()
            ),
        ));
    }
    check_destination_free(root, true)?;
    let made_root = !root.exists();
    fs::create_dir_all(root)?;
    let done = (|| {
        probe_writable(root)?;
        let place = LibraryPlace::at(root);
        fs::create_dir_all(&place.projects_dir)?;
        fs::create_dir_all(&place.vault_dir)?;
        core.use_library(place)
    })();
    if let Err(e) = done {
        let place = LibraryPlace::at(root);
        if let Some(f) = &place.file {
            let _ = fs::remove_file(f);
        }
        let _ = fs::remove_dir(&place.projects_dir);
        let _ = fs::remove_dir(&place.vault_dir);
        if made_root {
            let _ = fs::remove_dir(root);
        }
        return Err(e);
    }
    core.startup_maintenance();
    Ok(library_status(core))
}

fn connect(core: &AppCore, _a: &Actor, args: LibraryDirArgs) -> AppResult<LibraryStatus> {
    let dir = checked_dir(&args.dir)?;
    let found = inspect_dir(core, &dir)?;
    let Some(root) = found.library_root.as_deref().map(PathBuf::from) else {
        return Err(AppError::new("not_found.library", found.message));
    };
    connect_root(core, &root)
}

/// Start using the existing library at `root`. Never creates or replaces anything in it except
/// its library file, which is written with the identity in use.
fn connect_root(core: &AppCore, root: &Path) -> AppResult<LibraryStatus> {
    let _guard = core.begin_storage_op(StorageOp::Vault)?;
    if core
        .library_place()
        .is_some_and(|p| library::same_place(&p.root, root))
    {
        return Ok(library_status(core));
    }
    if library::is_inside(&core.config.app_data_dir, root) {
        return Err(AppError::invalid_input(
            "That is OpenFrame's own app data folder, not a library.",
        ));
    }
    probe_writable(root)?;
    let vault = root.join(VAULT_FOLDER).join("vault.sqlite");
    if vault.is_file() {
        check_vault_database(&vault)?;
    }
    core.use_library(LibraryPlace::at(root))?;
    drop(_guard);
    core.startup_maintenance();
    Ok(library_status(core))
}

// ---------------------------------------------------------------------- move

fn move_library_op(
    core: &AppCore,
    actor: &Actor,
    args: MoveToFolderArgs,
) -> AppResult<StorageTaskStarted> {
    // From here until the task ends no project can be opened, created or closed.
    let guard = core.begin_storage_op(StorageOp::LibraryMove)?;
    let place = core.library_place().ok_or_else(setup_required)?;
    let root = place.root.clone();
    let Some(name) = root.file_name().map(OsString::from) else {
        return Err(AppError::invalid_input(
            "The library is the top folder of a drive, so it can't be moved.",
        ));
    };
    if !root.is_dir() {
        return Err(AppError::new(
            "not_found.folder",
            "Your OpenFrame library can't be found. If it's on an external drive, reconnect it and try again.",
        ));
    }
    let parent = checked_folder(core, &args.dest_parent)?;
    let dest = parent.join(&name);
    if library::same_place(&dest, &root) {
        return Err(same_place_error("Your OpenFrame library"));
    }
    if library::is_inside(&root, &dest) || library::is_inside(&dest, &root) {
        return Err(AppError::invalid_input(
            "Choose a folder outside your OpenFrame library.",
        ));
    }
    check_destination_free(&dest, true)?;
    tree::ensure_space(&parent, tree::walk(&root, &[])?.bytes)?;
    let actor = actor.clone();
    let task_id = core.spawn_task(
        "storage.library_move",
        "Moving the library",
        move |core, h| {
            let _guard = guard;
            let report = move_library(&core, &actor, &root, &parent, &dest, &name, &h)?;
            serde_json::to_value(&report).map_err(|e| AppError::internal(e.to_string()))
        },
    );
    Ok(StorageTaskStarted { task_id })
}

/// Recent Projects inside `from`, to follow the library and to put back after a failure.
fn recents_inside(core: &AppCore, from: &Path) -> AppResult<Vec<(String, PathBuf)>> {
    core.with_app_db(|c| {
        let mut stmt = c.prepare("SELECT project_id, path FROM sys_recent_project")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows
            .into_iter()
            .map(|(id, p)| (id, PathBuf::from(p)))
            .filter(|(_, p)| library::is_inside(from, p))
            .collect())
    })
}

fn set_recents(core: &AppCore, rows: &[(String, PathBuf)]) -> AppResult<()> {
    core.with_app_db(|c| {
        let tx = c.unchecked_transaction()?;
        for (id, path) in rows {
            tx.execute(
                "UPDATE sys_recent_project SET path = ?1, updated_at = ?2 WHERE project_id = ?3",
                params![path.to_string_lossy(), now_ms(), id],
            )?;
        }
        tx.commit()?;
        Ok(())
    })
}

/// Projects in the copied tree: the folder (relative) and its id, read from the source.
fn projects_in(source: &tree::Tree, root: &Path) -> Vec<(String, String)> {
    source
        .files
        .iter()
        .filter_map(|f| {
            let dir = if f.rel == pf::MANIFEST_FILE {
                ""
            } else {
                f.rel.strip_suffix(&format!("/{}", pf::MANIFEST_FILE))?
            };
            let id = pf::read_manifest(&ProjectLayout::new(root.join(dir)))
                .ok()?
                .project_id;
            Some((dir.to_string(), id))
        })
        .collect()
}

/// Everything in the library, and its projects. Like a project move, each project's `.lock`
/// (it belongs to this place) and `cache/` (rebuilt on demand) are not copied.
fn walk_library(root: &Path) -> AppResult<(tree::Tree, Vec<(String, String)>)> {
    let mut source = tree::walk(root, &[])?;
    let projects = projects_in(&source, root);
    let skipped: Vec<String> = projects
        .iter()
        .flat_map(|(dir, _)| {
            PROJECT_SKIP.iter().map(move |s| {
                if dir.is_empty() {
                    s.to_lowercase()
                } else {
                    format!("{dir}/{s}").to_lowercase()
                }
            })
        })
        .collect();
    let keep = |rel: &str| {
        let rel = rel.to_lowercase();
        !skipped
            .iter()
            .any(|s| rel == *s || rel.starts_with(&format!("{s}/")))
    };
    source.files.retain(|f| keep(&f.rel));
    source.dirs.retain(|d| keep(d));
    Ok((source, projects))
}

/// After a verified move: what [`walk_library`] left out of each project in the old folder.
fn remove_project_leftovers(root: &Path, projects: &[(String, String)]) {
    for (dir, _) in projects {
        for name in PROJECT_SKIP {
            let path = root.join(dir).join(name);
            match fs::symlink_metadata(&path) {
                Ok(m) if m.is_dir() => {
                    let _ = fs::remove_dir_all(&path);
                }
                Ok(_) => {
                    let _ = fs::remove_file(&path);
                }
                Err(_) => {}
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
fn move_library(
    core: &AppCore,
    actor: &Actor,
    root: &Path,
    parent: &Path,
    dest: &Path,
    name: &OsString,
    h: &TaskHandle,
) -> AppResult<LibraryMoveReport> {
    let progress = |f: f64, m: &str| h.progress(f, m);
    let cancelled = || h.is_cancelled();
    // The open project, when it lives in the library, is closed for the copy and reopened from
    // its new place.
    let open_inside = core
        .project_opt()
        .filter(|s| library::is_inside(root, s.layout.root()))
        .map(|s| s.layout.root().to_path_buf());
    let reopen = |path: &Path| {
        if let Err(e) = project::open_at_unguarded(core, actor, path) {
            tracing::error!(
                code = e.code_str(),
                "the project could not be reopened after a library move"
            );
        }
    };
    if open_inside.is_some() {
        progress(0.02, "Saving and closing the project");
        let session = core.project()?;
        if let Err(e) = project::close_current(core) {
            tracing::warn!(
                code = e.code_str(),
                "the project did not close cleanly before a move"
            );
        }
        let give_up = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while Arc::strong_count(&session) > 1 {
            if std::time::Instant::now() >= give_up {
                drop(session);
                if let Some(p) = &open_inside {
                    reopen(p);
                }
                return Err(storage_busy(
                    "The project is in use right now. Wait a moment and try again. Nothing was moved.",
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    let previous = core.locations_snapshot();
    let vault_inside = library::is_inside(root, &core.vault_dir());
    let staging_dir = staging_path(parent, name);
    // Declared before the staging folder, so it is dropped after it.
    let _note = StagingNote::record(core, &staging_dir);
    let staging = Staging::at(staging_dir);
    let mut old_recents = Vec::new();
    progress(0.03, "Closing the vault");
    let switched = core.with_vault_closed(|| {
        if !core
            .library_place()
            .is_some_and(|p| library::same_place(&p.root, root))
        {
            return Err(storage_busy(
                "Your library was changed while this move was starting. Nothing was changed. Try the move again.",
            ));
        }
        let (source, projects) = walk_library(root)?;
        fs::create_dir_all(staging.path())?;
        let copied = tree::copy(&source, staging.path(), &progress, &cancelled)?;
        tree::verify(staging.path(), &copied, &progress, &cancelled)?;
        for (dir, id) in &projects {
            check_project_copy(&staging.path().join(dir), id)?;
        }
        if vault_inside
            && let Some(v) = library::rebase(&core.vault_dir(), root, staging.path())
            && v.join("vault.sqlite").is_file()
        {
            check_vault_database(&v.join("vault.sqlite"))?;
        }
        if h.is_cancelled() {
            return Err(AppError::cancelled());
        }
        check_destination_free(dest, true)?;
        if dest.exists() {
            super::tree::clear_finder_files(dest);
            fs::remove_dir(dest)?;
        }
        fs::rename(staging.path(), dest)?;
        // The switch: pointer and app.sqlite, then library.json and the places. The folder for
        // new projects is always the moved library's own (owner decision); a vault inside the
        // library follows it, one outside stays.
        let switch = (|| {
            remember_root(core, Some(dest))?;
            let rebase = |p: &Path| library::rebase(p, root, dest).unwrap_or_else(|| p.to_path_buf());
            core.change_locations(|l| {
                l.library = l.library.as_ref().map(|p| p.moved_to(dest));
                l.projects_dir = None;
                l.vault_dir = l.vault_dir.as_deref().map(rebase);
            })?;
            old_recents = recents_inside(core, root)?;
            let moved: Vec<(String, PathBuf)> = old_recents
                .iter()
                .map(|(id, p)| (id.clone(), rebase(p)))
                .collect();
            set_recents(core, &moved)
        })();
        if let Err(e) = switch {
            if let Err(r) = undo_switch(core, &previous, &old_recents) {
                return Err(both_places_error(root, dest, r.message));
            }
            tree::discard(dest);
            return Err(e);
        }
        Ok((source, projects))
    });
    let (source, projects) = match switched {
        Ok(v) => v,
        Err(e) => {
            drop(staging);
            if let Some(p) = &open_inside {
                reopen(p);
            }
            return Err(e);
        }
    };
    progress(0.92, "Opening your work in its new place");
    let new_open = open_inside
        .as_deref()
        .and_then(|p| library::rebase(p, root, dest));
    let opened = (|| {
        if let Some(p) = &new_open {
            project::open_at_unguarded(core, actor, p).and_then(|_| confirm_open_at(core, p))?;
        }
        if vault_inside && core.vault_dir().join("vault.sqlite").is_file() {
            core.global_store()?;
        }
        Ok::<(), AppError>(())
    })();
    if let Err(e) = opened {
        if let Some(p) = &new_open
            && core
                .project_opt()
                .is_some_and(|s| library::same_place(s.layout.root(), p))
            && let Err(c) = project::close_current(core)
        {
            tracing::warn!(code = c.code_str(), "the project did not close cleanly");
        }
        let undone = core.with_vault_closed(|| undo_switch(core, &previous, &old_recents));
        if let Some(p) = &open_inside {
            reopen(p);
        }
        return Err(match undone {
            Ok(()) => {
                tree::discard(dest);
                e
            }
            Err(r) => both_places_error(root, dest, r.message),
        });
    }
    progress(0.96, "Removing the old folder");
    remove_project_leftovers(root, &projects);
    let left = !tree::remove_source(root, &source, &[], &[]);
    Ok(LibraryMoveReport {
        path: dest.to_string_lossy().into_owned(),
        projects_moved: u32::try_from(projects.len()).unwrap_or(u32::MAX),
        vault_moved: vault_inside,
        source_left_at: left.then(|| root.to_string_lossy().into_owned()),
    })
}

fn undo_switch(
    core: &AppCore,
    previous: &crate::core::Locations,
    old_recents: &[(String, PathBuf)],
) -> AppResult<()> {
    set_recents(core, old_recents)?;
    core.restore_locations(previous.clone())
}

fn both_places_error(old: &Path, new: &Path, detail: String) -> AppError {
    rollback_failed(
        format!(
            "Your OpenFrame library was copied to “{}” but OpenFrame could not switch to it or back. Your work is safe in both “{}” and “{}”. Go to Settings > Storage and use Locate Existing Library to choose the one you want.",
            new.display(),
            old.display(),
            new.display()
        ),
        detail,
    )
}

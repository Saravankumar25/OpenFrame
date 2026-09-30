//! Local project format (docs/engineering/12-project-file-format.md, ADR-0003).
//!
//! A project is one folder named `<Title>.openframe`, presented to filmmakers
//! as a single project. Its internals are an implementation detail:
//!
//! ```text
//! <Title>.openframe/
//! ├─ openframe.json        manifest (format/schema/app versions, project identity)
//! ├─ project.sqlite        canonical structured data (+ -wal/-shm while open)
//! ├─ assets/               project-owned media, content-addressed by id
//! ├─ cache/                rebuildable thumbnails/previews (never canonical)
//! ├─ recovery/             session marker, last confirmed checkpoint
//! ├─ backups/              automatic safety backups (pre-migration, pre-replace)
//! └─ .lock                 single-writer lock (prevents two apps writing at once)
//! ```

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;
use openframe_domain::{AppError, AppResult, PROJECT_FORMAT_VERSION, now_ms};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const PROJECT_EXTENSION: &str = "openframe";
pub const MANIFEST_FILE: &str = "openframe.json";
pub const DB_FILE: &str = "project.sqlite";
pub const MANIFEST_FORMAT: &str = "openframe-project";

/// Paths inside a project folder.
#[derive(Debug, Clone)]
pub struct ProjectLayout {
    root: PathBuf,
}

impl ProjectLayout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn manifest(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }
    pub fn database(&self) -> PathBuf {
        self.root.join(DB_FILE)
    }
    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }
    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }
    pub fn thumbnails(&self) -> PathBuf {
        self.cache().join("thumbnails")
    }
    pub fn recovery(&self) -> PathBuf {
        self.root.join("recovery")
    }
    pub fn checkpoint(&self) -> PathBuf {
        self.recovery().join("checkpoint.sqlite")
    }
    pub fn session_marker(&self) -> PathBuf {
        self.recovery().join("session.json")
    }
    pub fn safety_backups(&self) -> PathBuf {
        self.root.join("backups")
    }
    pub fn lock_file(&self) -> PathBuf {
        self.root.join(".lock")
    }

    /// Absolute path of a project-owned asset from its stored relative path.
    pub fn asset_path(&self, rel: &str) -> AppResult<PathBuf> {
        openframe_security::confine(&self.root, rel)
    }

    /// Relative storage path for a new managed asset: `assets/<aa>/<id>.<ext>`.
    pub fn new_asset_rel(id: &str, extension: Option<&str>) -> String {
        let shard: String = id
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .rev()
            .take(2)
            .collect();
        let ext = extension
            .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
            .filter(|e| {
                !e.is_empty() && e.len() <= 10 && e.chars().all(|c| c.is_ascii_alphanumeric())
            });
        match ext {
            Some(ext) => format!("assets/{shard}/{id}.{ext}"),
            None => format!("assets/{shard}/{id}"),
        }
    }

    pub fn create_dirs(&self) -> AppResult<()> {
        for d in [
            self.root.clone(),
            self.assets(),
            self.cache(),
            self.recovery(),
            self.safety_backups(),
        ] {
            fs::create_dir_all(d)?;
        }
        Ok(())
    }

    pub fn looks_like_project(&self) -> bool {
        self.manifest().is_file() && self.database().is_file()
    }
}

/// `openframe.json` — human-inspectable identity and version metadata.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub format: String,
    pub format_version: u32,
    pub project_id: String,
    pub title: String,
    #[ts(type = "number")]
    pub created_at: i64,
    /// App version that last wrote this project.
    pub app_version: String,
    /// Database schema version (mirrors `PRAGMA user_version`).
    pub schema_version: u32,
    pub ai_compatibility_version: u32,
}

impl ProjectManifest {
    pub fn new(project_id: &str, title: &str, app_version: &str, schema_version: u32) -> Self {
        Self {
            format: MANIFEST_FORMAT.to_string(),
            format_version: PROJECT_FORMAT_VERSION,
            project_id: project_id.to_string(),
            title: title.to_string(),
            created_at: now_ms(),
            app_version: app_version.to_string(),
            schema_version,
            ai_compatibility_version: openframe_domain::AI_COMPATIBILITY_VERSION,
        }
    }
}

pub fn read_manifest(layout: &ProjectLayout) -> AppResult<ProjectManifest> {
    // The manifest is a few hundred bytes; a huge one (e.g. inside a received package)
    // is refused before it is read into memory.
    if fs::metadata(layout.manifest()).is_ok_and(|m| m.len() > 1 << 20) {
        return Err(
            AppError::project_format("This project's information file is damaged.")
                .with_detail("manifest larger than 1 MiB"),
        );
    }
    let bytes = fs::read(layout.manifest()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::project_format(
                "This folder isn't an OpenFrame project (its project information file is missing).",
            )
        } else {
            e.into()
        }
    })?;
    let manifest: ProjectManifest = serde_json::from_slice(&bytes).map_err(|e| {
        AppError::project_format("This project's information file is damaged.")
            .with_detail(e.to_string())
    })?;
    if manifest.format != MANIFEST_FORMAT {
        return Err(AppError::project_format(
            "This folder isn't an OpenFrame project.",
        ));
    }
    if manifest.format_version > PROJECT_FORMAT_VERSION {
        return Err(AppError::new(
            "project_format.too_new",
            "This project was saved by a newer version of OpenFrame. Update OpenFrame to open it — the project has not been changed.",
        ));
    }
    if !openframe_domain::ids::is_valid_id(&manifest.project_id) {
        return Err(AppError::project_format(
            "This project's identity information is damaged.",
        ));
    }
    Ok(manifest)
}

pub fn write_manifest(layout: &ProjectLayout, manifest: &ProjectManifest) -> AppResult<()> {
    let bytes =
        serde_json::to_vec_pretty(manifest).map_err(|e| AppError::internal(e.to_string()))?;
    atomic_write(&layout.manifest(), &bytes)
}

/// Crash-safe file replacement: write temp sibling, fsync, rename over target.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::internal("atomic_write needs a parent directory"))?;
    fs::create_dir_all(parent)?;
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("file")
    ));
    {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })?;
    Ok(())
}

fn in_use_error() -> AppError {
    AppError::new(
        "project_format.in_use",
        "This project is already open in another OpenFrame window. Close it there first — OpenFrame won't open two editable copies at once.",
    )
}

/// Exclusive single-writer lock held for as long as a project is open.
#[derive(Debug)]
pub struct ProjectLock {
    file: File,
    path: PathBuf,
}

impl ProjectLock {
    pub fn acquire(layout: &ProjectLayout) -> AppResult<ProjectLock> {
        let path = layout.lock_file();
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(true) => Ok(ProjectLock { file, path }),
            Ok(false) => Err(in_use_error()),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Err(in_use_error()),
            Err(e) => Err(e.into()),
        }
    }
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
        tracing::debug!(path = %self.path.display(), "project lock released");
    }
}

/// Session marker used for crash detection (Autosave/Recovery spec).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SessionMarker {
    pub pid: u32,
    #[ts(type = "number")]
    pub started_at: i64,
    pub app_version: String,
}

/// If a marker from a previous session exists, that session did not close cleanly.
pub fn previous_unclean_session(layout: &ProjectLayout) -> Option<SessionMarker> {
    let bytes = fs::read(layout.session_marker()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn write_session_marker(layout: &ProjectLayout, app_version: &str) -> AppResult<()> {
    let marker = SessionMarker {
        pid: std::process::id(),
        started_at: now_ms(),
        app_version: app_version.to_string(),
    };
    atomic_write(
        &layout.session_marker(),
        &serde_json::to_vec(&marker).map_err(|e| AppError::internal(e.to_string()))?,
    )
}

pub fn clear_session_marker(layout: &ProjectLayout) -> AppResult<()> {
    match fs::remove_file(layout.session_marker()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Choose a new, non-existing `<Title>.openframe` folder under `parent`.
pub fn unique_project_dir(parent: &Path, title: &str) -> PathBuf {
    let base = openframe_security::sanitize_file_name(title);
    let mut candidate = parent.join(format!("{base}.{PROJECT_EXTENSION}"));
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{base} ({n}).{PROJECT_EXTENSION}"));
        n += 1;
    }
    candidate
}

/// SQLite WAL is unsafe on network file systems, so projects are never edited
/// over a network share; sharing happens through packages. Detects UNC paths.
pub fn is_network_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    (s.starts_with("\\\\") && !s.starts_with("\\\\?\\"))
        || s.starts_with("\\\\?\\UNC\\")
        || s.starts_with("//")
}

fn documents_dir() -> PathBuf {
    directories::UserDirs::new()
        .and_then(|u| u.document_dir().map(|d| d.to_path_buf()))
        .unwrap_or_else(std::env::temp_dir)
}

/// Default location for new projects: `Documents/OpenFrame/Projects`.
pub fn default_projects_dir() -> PathBuf {
    documents_dir().join("OpenFrame").join("Projects")
}

/// Default location for the filmmaker's Global Idea Vault (user content, so it lives in Documents).
pub fn default_global_vault_dir() -> PathBuf {
    documents_dir().join("OpenFrame").join("Global Idea Vault")
}

/// Application-private data (`%LOCALAPPDATA%/OpenFrame`): settings, recents, models, logs.
pub fn app_data_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|b| b.data_local_dir().to_path_buf())
        .unwrap_or_else(std::env::temp_dir)
        .join("OpenFrame")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trip_and_validation() {
        let dir = tempfile::tempdir().unwrap();
        let layout = ProjectLayout::new(dir.path().join("Film.openframe"));
        layout.create_dirs().unwrap();
        let id = openframe_domain::new_id();
        let m = ProjectManifest::new(&id, "Film", "0.1.0", 1);
        write_manifest(&layout, &m).unwrap();
        let back = read_manifest(&layout).unwrap();
        assert_eq!(back.project_id, id);

        let mut newer = m.clone();
        newer.format_version = PROJECT_FORMAT_VERSION + 1;
        write_manifest(&layout, &newer).unwrap();
        assert_eq!(
            read_manifest(&layout).unwrap_err().code_str(),
            "project_format.too_new"
        );

        fs::write(layout.manifest(), b"{garbage").unwrap();
        assert!(read_manifest(&layout).unwrap_err().is("project_format"));
    }

    #[test]
    fn lock_is_exclusive() {
        let dir = tempfile::tempdir().unwrap();
        let layout = ProjectLayout::new(dir.path());
        let first = ProjectLock::acquire(&layout).unwrap();
        assert_eq!(
            ProjectLock::acquire(&layout).unwrap_err().code_str(),
            "project_format.in_use"
        );
        drop(first);
        assert!(ProjectLock::acquire(&layout).is_ok());
    }

    #[test]
    fn session_marker_detects_unclean_close() {
        let dir = tempfile::tempdir().unwrap();
        let layout = ProjectLayout::new(dir.path());
        layout.create_dirs().unwrap();
        assert!(previous_unclean_session(&layout).is_none());
        write_session_marker(&layout, "0.1.0").unwrap();
        assert!(previous_unclean_session(&layout).is_some());
        clear_session_marker(&layout).unwrap();
        assert!(previous_unclean_session(&layout).is_none());
    }

    #[test]
    fn unique_dirs_and_assets() {
        let dir = tempfile::tempdir().unwrap();
        let a = unique_project_dir(dir.path(), "My: Film");
        assert!(a.ends_with("My_ Film.openframe"));
        fs::create_dir_all(&a).unwrap();
        let b = unique_project_dir(dir.path(), "My: Film");
        assert!(b.ends_with("My_ Film (2).openframe"));
        let rel =
            ProjectLayout::new_asset_rel("0192f0c1-aaaa-7bbb-8ccc-0123456789ab", Some(".PNG"));
        assert!(rel.starts_with("assets/ba/") && rel.ends_with(".png"));
        let bad =
            ProjectLayout::new_asset_rel("0192f0c1-aaaa-7bbb-8ccc-0123456789ab", Some("../../x"));
        assert!(!bad.contains(".."));
    }

    #[test]
    fn network_paths_detected() {
        assert!(is_network_path(Path::new(r"\\server\share\Film.openframe")));
        assert!(!is_network_path(Path::new(r"C:\Users\x\Film.openframe")));
        assert!(!is_network_path(Path::new(r"\\?\C:\long\path")));
    }
}

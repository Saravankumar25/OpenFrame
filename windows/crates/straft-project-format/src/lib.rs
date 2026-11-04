//! Local project format (docs/engineering/12-project-file-format.md, ADR-0003).
//!
//! A project is one folder named `<Title>.straft`, presented to users
//! as a single project. Its internals are an implementation detail:
//!
//! ```text
//! <Title>.straft/
//! ├─ straft.json        manifest (format/schema/app versions, project identity)
//! ├─ project.sqlite        canonical structured data (+ -wal/-shm while open)
//! ├─ assets/               project-owned media, content-addressed by id
//! ├─ cache/                rebuildable thumbnails/previews (never canonical)
//! ├─ recovery/             session marker, last confirmed checkpoint
//! ├─ backups/              automatic safety backups (pre-migration, pre-replace)
//! └─ .lock                 single-writer lock (prevents two apps writing at once)
//! ```
//!
//! Projects made before the rename keep their folder name (`straft_platform::legacy_names`) and may
//! still hold the earlier manifest file. One rule: `straft.json` wins whenever it exists. The earlier
//! file is read only when there is no `straft.json`, and every manifest write creates `straft.json`
//! first and removes the earlier file only once the new one is on the drive, so the two can never
//! disagree.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;
use straft_domain::{AppError, AppResult, PROJECT_FORMAT_VERSION, now_ms};
use straft_platform::legacy_names;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const PROJECT_EXTENSION: &str = "straft";
pub const MANIFEST_FILE: &str = "straft.json";
pub const DB_FILE: &str = "project.sqlite";
pub const MANIFEST_FORMAT: &str = "straft-project";

/// True for the manifest file's name, in a project made by this version or an earlier one.
pub fn is_manifest_name(name: &str) -> bool {
    name == MANIFEST_FILE || name == legacy_names::MANIFEST_FILE
}

/// True when `dir` holds a project manifest, written by this version or an earlier one.
pub fn is_project_folder(dir: &Path) -> bool {
    dir.join(MANIFEST_FILE).is_file() || dir.join(legacy_names::MANIFEST_FILE).is_file()
}

/// True for a folder named like a project (`<Title>.straft`, or the earlier extension). Any case.
pub fn has_project_extension(name: &str) -> bool {
    let name = name.to_lowercase();
    [PROJECT_EXTENSION, legacy_names::PROJECT_EXTENSION]
        .iter()
        .any(|ext| {
            name.strip_suffix(ext)
                .is_some_and(|stem| stem.ends_with('.'))
        })
}

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
    /// Where the manifest is written. Read it with [`read_manifest`].
    pub fn manifest(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }
    /// The manifest file of a project an earlier version wrote.
    pub fn legacy_manifest(&self) -> PathBuf {
        self.root.join(legacy_names::MANIFEST_FILE)
    }
    /// True when the folder holds a manifest under either name.
    pub fn has_manifest(&self) -> bool {
        is_project_folder(&self.root)
    }
    /// The file [`read_manifest`] reads: `straft.json` whenever it is there.
    fn manifest_to_read(&self) -> PathBuf {
        let current = self.manifest();
        if current.exists() {
            return current;
        }
        let earlier = self.legacy_manifest();
        if earlier.is_file() { earlier } else { current }
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
        straft_security::confine(&self.root, rel)
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
        self.has_manifest() && self.database().is_file()
    }
}

/// `straft.json`: human-inspectable identity and version metadata.
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
            ai_compatibility_version: straft_domain::AI_COMPATIBILITY_VERSION,
        }
    }
}

pub fn read_manifest(layout: &ProjectLayout) -> AppResult<ProjectManifest> {
    // The manifest is a few hundred bytes; a huge one (e.g. inside a received package)
    // is refused before it is read into memory.
    let path = layout.manifest_to_read();
    if fs::metadata(&path).is_ok_and(|m| m.len() > 1 << 20) {
        return Err(
            AppError::project_format("This project's information file is damaged.")
                .with_detail("manifest larger than 1 MiB"),
        );
    }
    let bytes = fs::read(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::project_format(
                "This folder isn't a Straft Studio project (its project information file is missing).",
            )
        } else {
            e.into()
        }
    })?;
    let mut manifest: ProjectManifest = serde_json::from_slice(&bytes).map_err(|e| {
        AppError::project_format("This project's information file is damaged.")
            .with_detail(e.to_string())
    })?;
    if manifest.format != MANIFEST_FORMAT && manifest.format != legacy_names::MANIFEST_FORMAT {
        return Err(AppError::project_format(
            "This folder isn't a Straft Studio project.",
        ));
    }
    // The next write records the current name, whichever file this one came from.
    manifest.format = MANIFEST_FORMAT.to_string();
    if manifest.format_version > PROJECT_FORMAT_VERSION {
        return Err(AppError::new(
            "project_format.too_new",
            "This project was saved by a newer version of Straft Studio. Update Straft Studio to open it. The project has not been changed.",
        ));
    }
    if !straft_domain::ids::is_valid_id(&manifest.project_id) {
        return Err(AppError::project_format(
            "This project's identity information is damaged.",
        ));
    }
    Ok(manifest)
}

/// Write `straft.json` atomically. An earlier manifest file in the folder is removed only after the
/// new file is on the drive. If that removal fails nothing is wrong: `straft.json` wins on every read
/// and the next write tries again.
pub fn write_manifest(layout: &ProjectLayout, manifest: &ProjectManifest) -> AppResult<()> {
    let bytes =
        serde_json::to_vec_pretty(manifest).map_err(|e| AppError::internal(e.to_string()))?;
    atomic_write(&layout.manifest(), &bytes)?;
    let earlier = layout.legacy_manifest();
    if earlier.exists() {
        straft_platform::atomic::sync_folder(layout.root());
        if let Err(e) = fs::remove_file(&earlier) {
            // The path holds the project name: never log it.
            tracing::warn!(error = %e.kind(), "the earlier project manifest could not be removed yet");
        }
    }
    Ok(())
}

/// Crash-safe file replacement through the shared atomic replace
/// (`straft_platform::atomic`): unique temp file in the same folder, fsync, rename over the
/// target, temp removed on failure. The folder is created when missing.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| AppError::internal("atomic_write needs a parent directory"))?;
    fs::create_dir_all(parent)?;
    straft_platform::atomic::replace_bytes(path, bytes)?;
    Ok(())
}

/// Remove staging files left by a write that a crash or power loss interrupted, in the project
/// folder and the project's own subfolders that receive atomic writes. Best effort; only files
/// older than `straft_platform::atomic::STALE_AFTER` are removed. Returns how many were removed.
pub fn remove_stale_temps(layout: &ProjectLayout) -> usize {
    [
        layout.root().to_path_buf(),
        layout.recovery(),
        layout.safety_backups(),
    ]
    .iter()
    .map(|d| straft_platform::atomic::remove_stale_temps(d))
    .sum()
}

fn in_use_error() -> AppError {
    AppError::new(
        "project_format.in_use",
        "This project is already open in another Straft Studio window. Close it there first. Straft Studio won't open two editable copies at once.",
    )
}

/// Exclusive single-writer lock held for as long as a project is open.
#[derive(Debug)]
pub struct ProjectLock {
    file: File,
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
            Ok(true) => Ok(ProjectLock { file }),
            Ok(false) => Err(in_use_error()),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Err(in_use_error()),
            Err(e) => Err(e.into()),
        }
    }
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
        // The path holds the project name: never log it (no project content in logs, log redaction test).
        tracing::debug!("project lock released");
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

/// Choose a new, non-existing `<Title>.straft` folder under `parent`.
pub fn unique_project_dir(parent: &Path, title: &str) -> PathBuf {
    let base = straft_security::sanitize_file_name(title);
    let mut candidate = parent.join(format!("{base}.{PROJECT_EXTENSION}"));
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{base} ({n}).{PROJECT_EXTENSION}"));
        n += 1;
    }
    candidate
}

/// SQLite WAL is unsafe on network file systems, so projects are never edited
/// over a network share; sharing happens through packages. Detects UNC paths, mapped network
/// drives and local paths that resolve to a share through a link (network drive refusal); see
/// [`straft_security::is_network_location`].
pub fn is_network_path(path: &Path) -> bool {
    straft_security::is_network_location(path)
}

// The folders below come from the platform boundary (`straft_platform::dirs`) and are `None`
// when the system cannot name them. There is no temp-folder fallback: projects and settings must
// never land where the system cleans up. `AppConfig::for_current_user` refuses to start instead.

/// Default location for new projects: `Documents/Straft Studio/Projects`.
pub fn default_projects_dir() -> Option<PathBuf> {
    straft_platform::dirs::user_content_dir().map(|d| d.join("Projects"))
}

/// Default location for the user's Global Idea Vault (user content, so it lives in Documents).
pub fn default_global_vault_dir() -> Option<PathBuf> {
    straft_platform::dirs::user_content_dir().map(|d| d.join("Global Idea Vault"))
}

/// Application-private data (`%LOCALAPPDATA%\Straft` on Windows,
/// `~/Library/Application Support/Straft` on macOS): settings, recents, models, logs.
pub fn app_data_dir() -> Option<PathBuf> {
    straft_platform::dirs::app_data_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trip_and_validation() {
        let dir = tempfile::tempdir().unwrap();
        let layout = ProjectLayout::new(dir.path().join("Film.straft"));
        layout.create_dirs().unwrap();
        let id = straft_domain::new_id();
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

    /// A manifest write that fails (the file is read-only) keeps the old manifest and leaves no
    /// staging file behind (session 1 defect: partial output on failure).
    #[cfg(windows)]
    #[test]
    fn failed_manifest_write_leaves_no_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let layout = ProjectLayout::new(dir.path().join("Film.straft"));
        layout.create_dirs().unwrap();
        let m = ProjectManifest::new(&straft_domain::new_id(), "Film", "0.1.0", 1);
        write_manifest(&layout, &m).unwrap();
        let before = fs::read(layout.manifest()).unwrap();
        let set_ro = |on: bool| {
            let mut p = fs::metadata(layout.manifest()).unwrap().permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            p.set_readonly(on);
            fs::set_permissions(layout.manifest(), p).unwrap();
        };
        set_ro(true);
        let mut renamed = m.clone();
        renamed.title = "Renamed".into();
        let result = write_manifest(&layout, &renamed);
        set_ro(false);
        assert!(result.is_err(), "a read-only manifest was replaced");
        assert_eq!(fs::read(layout.manifest()).unwrap(), before);
        let names: Vec<String> = fs::read_dir(layout.root())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(straft_platform::atomic::TEMP_SUFFIX) || n.ends_with(".tmp"))
            .collect();
        assert!(names.is_empty(), "{names:?}");
    }

    #[test]
    fn manifest_write_leaves_no_staging_file_and_stale_ones_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let layout = ProjectLayout::new(dir.path().join("Film.straft"));
        layout.create_dirs().unwrap();
        let m = ProjectManifest::new(&straft_domain::new_id(), "Film", "0.1.0", 1);
        write_manifest(&layout, &m).unwrap();
        write_session_marker(&layout, "0.1.0").unwrap();
        let staging = |d: &Path| -> Vec<String> {
            fs::read_dir(d)
                .unwrap()
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| {
                    n.ends_with(straft_platform::atomic::TEMP_SUFFIX) || n.ends_with(".tmp")
                })
                .collect()
        };
        assert!(staging(layout.root()).is_empty());
        assert!(staging(&layout.recovery()).is_empty());

        // A crash mid-write leaves a staging file; opening the project removes it.
        let left =
            straft_platform::atomic::unique_temp_path(&layout.recovery(), &layout.checkpoint())
                .unwrap();
        fs::write(&left, b"half").unwrap();
        let left_root =
            straft_platform::atomic::unique_temp_path(layout.root(), &layout.manifest())
                .unwrap();
        fs::write(&left_root, b"half").unwrap();
        // A staging file this new may belong to a write in progress and is kept.
        assert_eq!(remove_stale_temps(&layout), 0);
        let old = std::time::SystemTime::now()
            - straft_platform::atomic::STALE_AFTER
            - std::time::Duration::from_secs(60);
        for p in [&left, &left_root] {
            OpenOptions::new()
                .write(true)
                .open(p)
                .unwrap()
                .set_modified(old)
                .unwrap();
        }
        assert_eq!(remove_stale_temps(&layout), 2);
        assert!(!left.exists() && !left_root.exists());
        assert!(layout.manifest().is_file());
        assert!(layout.session_marker().is_file());
    }

    #[test]
    fn folders_delegate_to_the_platform_boundary() {
        assert_eq!(app_data_dir(), straft_platform::dirs::app_data_dir());
        assert_eq!(
            default_projects_dir(),
            straft_platform::dirs::user_content_dir().map(|d| d.join("Projects"))
        );
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
        assert!(a.ends_with("My_ Film.straft"));
        fs::create_dir_all(&a).unwrap();
        let b = unique_project_dir(dir.path(), "My: Film");
        assert!(b.ends_with("My_ Film (2).straft"));
        let rel =
            ProjectLayout::new_asset_rel("0192f0c1-aaaa-7bbb-8ccc-0123456789ab", Some(".PNG"));
        assert!(rel.starts_with("assets/ba/") && rel.ends_with(".png"));
        let bad =
            ProjectLayout::new_asset_rel("0192f0c1-aaaa-7bbb-8ccc-0123456789ab", Some("../../x"));
        assert!(!bad.contains(".."));
    }

    #[test]
    fn network_paths_detected() {
        assert!(is_network_path(Path::new(r"\\server\share\Film.straft")));
        assert!(!is_network_path(Path::new(r"C:\Users\x\Film.straft")));
        assert!(!is_network_path(Path::new(r"\\?\C:\long\path")));
        assert!(is_network_path(Path::new(
            r"\\?\UNC\server\share\Film.straft"
        )));
        assert!(is_network_path(Path::new("//server/share/Film.straft")));
        // A real local folder (existing or not yet created) is not a network location.
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_network_path(dir.path()));
        assert!(!is_network_path(
            &dir.path().join("New").join("Film.straft")
        ));
    }

    /// A manifest an earlier version wrote, in a folder with the earlier extension.
    fn earlier_project(dir: &Path) -> (ProjectLayout, String) {
        let root = dir.join(format!("Film.{}", legacy_names::PROJECT_EXTENSION));
        let layout = ProjectLayout::new(&root);
        layout.create_dirs().unwrap();
        let id = straft_domain::new_id();
        let mut json = serde_json::to_value(ProjectManifest::new(&id, "Film", "0.1.0", 7)).unwrap();
        json["format"] = legacy_names::MANIFEST_FORMAT.into();
        fs::write(
            layout.legacy_manifest(),
            serde_json::to_vec_pretty(&json).unwrap(),
        )
        .unwrap();
        fs::write(layout.database(), b"").unwrap();
        (layout, id)
    }

    #[test]
    fn a_project_from_an_earlier_version_reads_and_counts_as_a_project() {
        let dir = tempfile::tempdir().unwrap();
        let (layout, id) = earlier_project(dir.path());
        assert!(!layout.manifest().exists());
        assert!(layout.has_manifest() && layout.looks_like_project());
        assert!(is_project_folder(layout.root()));
        let m = read_manifest(&layout).unwrap();
        assert_eq!(m.project_id, id);
        assert_eq!(m.format, MANIFEST_FORMAT, "normalised for the next write");
        // Reading never writes: a read-only place stays as it was.
        assert!(!layout.manifest().exists());
        assert!(layout.legacy_manifest().is_file());
    }

    #[test]
    fn the_first_write_replaces_the_earlier_manifest_and_only_after_the_new_one_exists() {
        let dir = tempfile::tempdir().unwrap();
        let (layout, id) = earlier_project(dir.path());
        let mut m = read_manifest(&layout).unwrap();
        m.title = "Renamed".into();
        write_manifest(&layout, &m).unwrap();
        assert!(layout.manifest().is_file());
        assert!(!layout.legacy_manifest().exists());
        let back = read_manifest(&layout).unwrap();
        assert_eq!((back.title.as_str(), back.project_id.as_str()), ("Renamed", id.as_str()));
        let on_disk: serde_json::Value =
            serde_json::from_slice(&fs::read(layout.manifest()).unwrap()).unwrap();
        assert_eq!(on_disk["format"], MANIFEST_FORMAT);
    }

    /// A crash between the two steps leaves both files. There is one answer: the new file wins, an
    /// edit in the earlier file never comes back, and the next write removes it.
    #[test]
    fn with_both_manifests_the_new_one_wins_and_the_next_write_tidies_up() {
        let dir = tempfile::tempdir().unwrap();
        let (layout, id) = earlier_project(dir.path());
        let mut new = read_manifest(&layout).unwrap();
        new.title = "New title".into();
        atomic_write(
            &layout.manifest(),
            &serde_json::to_vec_pretty(&new).unwrap(),
        )
        .unwrap();
        assert!(layout.legacy_manifest().is_file());
        assert_eq!(read_manifest(&layout).unwrap().title, "New title");
        write_manifest(&layout, &new).unwrap();
        assert!(!layout.legacy_manifest().exists());
        assert_eq!(read_manifest(&layout).unwrap().project_id, id);

        // A damaged new file is reported; it never silently falls back to older facts.
        let (layout, _) = earlier_project(&dir.path().join("second"));
        fs::write(layout.manifest(), b"{ nope").unwrap();
        assert!(read_manifest(&layout).unwrap_err().is("project_format"));
    }

    #[test]
    fn a_manifest_with_some_other_format_is_still_refused() {
        let dir = tempfile::tempdir().unwrap();
        let (layout, _) = earlier_project(dir.path());
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(layout.legacy_manifest()).unwrap()).unwrap();
        json["format"] = "another-app-project".into();
        fs::write(layout.legacy_manifest(), serde_json::to_vec(&json).unwrap()).unwrap();
        assert!(read_manifest(&layout).unwrap_err().is("project_format"));
    }

    #[test]
    fn project_names_are_recognised_under_both_extensions() {
        let earlier = legacy_names::PROJECT_EXTENSION;
        assert!(has_project_extension("Film.straft"));
        assert!(has_project_extension("FILM.STRAFT"));
        assert!(has_project_extension(&format!("Film.{earlier}")));
        assert!(!has_project_extension("straft"));
        assert!(!has_project_extension(earlier));
        assert!(!has_project_extension("Film.sqlite"));
        assert!(is_manifest_name(MANIFEST_FILE));
        assert!(is_manifest_name(legacy_names::MANIFEST_FILE));
        assert!(!is_manifest_name("manifest.json"));
    }
}

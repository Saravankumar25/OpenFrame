//! Package container format (Import/Export §9.3, §13; Security threat model).
//!
//! Every OpenFrame package is a ZIP written through
//! `openframe_security::archive` (validated entry names, atomic write) with:
//!
//! ```text
//! manifest.json    PackageManifest (format "openframe-package", type, version, identities…)
//! checksums.json   { "algorithm": "sha256", "files": { "<entry>": "<hex>" } } for every other entry
//! content.json     exchange packages: the selected snapshot (JSON)
//! project/…        backup / full project packages: database, project manifest, managed assets
//! external/…       backup / project packages: linked external files copied for portability
//! ```
//!
//! Reading follows the strict validation order of the spec: structure (safe
//! archive pre-flight: zip-slip, zip bombs, symlinks) → supported format version
//! → required metadata → extraction into a private staging folder → checksum of
//! every entry. Nothing in the project is touched before all of that succeeds.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use openframe_domain::{
    AppError, AppResult, OLDEST_PACKAGE_FORMAT_VERSION, PACKAGE_FORMAT_VERSION, new_id,
};
use openframe_security::archive::{self, ArchiveLimits, EntrySource};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

pub const FORMAT_NAME: &str = "openframe-package";
pub const MANIFEST_ENTRY: &str = "manifest.json";
pub const CHECKSUMS_ENTRY: &str = "checksums.json";
pub const CONTENT_ENTRY: &str = "content.json";
/// Metadata entries are small; anything larger is not a real manifest.
const MAX_META_BYTES: u64 = 4 << 20;
/// `content.json` of an exchange package is text: a feature screenplay is well under 1 MB, a
/// large Story Board or schedule a few MB. A bigger file is refused before it is parsed, so
/// a hostile package can't make the app build a multi-gigabyte JSON tree in memory.
pub const MAX_CONTENT_BYTES: u64 = 64 << 20;

/// The package classes. They are never interchangeable (Import/Export §1):
/// backup, full project transfer, exchange (review/collaboration) and response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum PackageType {
    Backup,
    Project,
    ScriptReview,
    Story,
    Breakdown,
    Shots,
    Schedule,
    CallReview,
    Response,
}

impl PackageType {
    pub const ALL: [PackageType; 9] = [
        PackageType::Backup,
        PackageType::Project,
        PackageType::ScriptReview,
        PackageType::Story,
        PackageType::Breakdown,
        PackageType::Shots,
        PackageType::Schedule,
        PackageType::CallReview,
        PackageType::Response,
    ];

    pub fn extension(self) -> &'static str {
        match self {
            PackageType::Backup => "ofbackup",
            PackageType::Project => "ofproject",
            PackageType::ScriptReview => "ofscriptreview",
            PackageType::Story => "ofstory",
            PackageType::Breakdown => "ofbreakdown",
            PackageType::Shots => "ofshots",
            PackageType::Schedule => "ofschedule",
            PackageType::CallReview => "ofcallreview",
            PackageType::Response => "ofresponse",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PackageType::Backup => "Backup Package",
            PackageType::Project => "Full Project Package",
            PackageType::ScriptReview => "Screenplay review package",
            PackageType::Story => "Story Board package",
            PackageType::Breakdown => "Breakdown package",
            PackageType::Shots => "Shot List package",
            PackageType::Schedule => "Schedule package",
            PackageType::CallReview => "Call Sheet review package",
            PackageType::Response => "Response package",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PackageType::Backup => "backup",
            PackageType::Project => "project",
            PackageType::ScriptReview => "scriptReview",
            PackageType::Story => "story",
            PackageType::Breakdown => "breakdown",
            PackageType::Shots => "shots",
            PackageType::Schedule => "schedule",
            PackageType::CallReview => "callReview",
            PackageType::Response => "response",
        }
    }

    pub fn parse(s: &str) -> Option<PackageType> {
        PackageType::ALL.into_iter().find(|t| t.as_str() == s)
    }

    /// Whole-project containers (may include private project data).
    pub fn is_project_container(self) -> bool {
        matches!(self, PackageType::Backup | PackageType::Project)
    }

    /// Exchange packages (selected subset; never private notes).
    pub fn is_exchange(self) -> bool {
        !self.is_project_container()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageUser {
    pub user_id: String,
    pub display_name: String,
}

/// Source screenplay draft of a review package.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageSourceDraft {
    pub id: String,
    pub screenplay_id: String,
    pub name: String,
    pub status: String,
}

/// Any other source version the package was built from (production source, schedule…).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageSourceVersion {
    pub kind: String,
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageScope {
    /// e.g. "project", "full", "scenes", "act", "schedule", "callSheet".
    pub kind: String,
    /// Human description, e.g. "Full script" or "Scenes 3, 4, 9".
    pub label: String,
    #[serde(default)]
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageRef {
    pub package_id: String,
    pub package_type: PackageType,
}

/// `manifest.json`: the package metadata required by Import/Export §9.3.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageManifest {
    pub format: String,
    pub package_type: PackageType,
    pub format_version: u32,
    pub package_id: String,
    pub source_project_id: String,
    pub source_project_title: String,
    #[ts(type = "number")]
    pub exported_at: i64,
    pub app_version: String,
    /// Project database schema version (backup / full project packages).
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub source_draft: Option<PackageSourceDraft>,
    #[serde(default)]
    pub source_versions: Vec<PackageSourceVersion>,
    #[serde(default)]
    pub included_object_ids: Vec<String>,
    pub scope: PackageScope,
    pub comments_included: bool,
    pub attachments_included: bool,
    /// Always false for exchange packages (Security §8.6).
    pub private_notes_included: bool,
    /// Row revisions of the included objects at export time (`table:id` → rev),
    /// used to detect stale packages and conflicts.
    #[serde(default)]
    #[ts(type = "Record<string, number>")]
    pub base_snapshot: BTreeMap<String, i64>,
    pub originating_user: PackageUser,
    /// Optional user label (backup naming, FSD §44.8).
    #[serde(default)]
    pub label: Option<String>,
    /// Response packages: the review package this responds to.
    #[serde(default)]
    pub responds_to: Option<PackageRef>,
    /// Format 2, backup and full project packages: live row count of every project table at
    /// export time. Restore refuses a database whose counts differ. Empty in format 1.
    #[serde(default)]
    #[ts(type = "Record<string, number>")]
    pub table_counts: BTreeMap<String, i64>,
    /// Format 2, exchange packages: the `table.column` values the writer carried (see
    /// `coverage`). A reader treats a field missing from this list as not sent rather than
    /// empty. Empty in format 1.
    #[serde(default)]
    pub carried: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksums {
    pub algorithm: String,
    pub files: BTreeMap<String, String>,
}

// ------------------------------------------------------------------ errors

pub fn not_package() -> AppError {
    AppError::import(
        "not_package",
        "This file isn't an OpenFrame package, so nothing was imported. Your project was not changed.",
    )
}

pub fn incomplete(detail: impl Into<String>) -> AppError {
    AppError::import(
        "package_incomplete",
        "Package validation failed because the package is incomplete. Your project was not changed.",
    )
    .with_detail(detail)
}

pub fn checksum_mismatch(detail: impl Into<String>) -> AppError {
    AppError::import(
        "checksum_mismatch",
        "Package validation failed because the package is damaged or was changed after it was exported. Your project was not changed.",
    )
    .with_detail(detail)
}

pub fn too_new(found: u32) -> AppError {
    AppError::import(
        "package_too_new",
        "This package was created by a newer version of OpenFrame. Update OpenFrame to open it. Nothing was changed.",
    )
    .with_detail(format!(
        "package format {found} > supported {PACKAGE_FORMAT_VERSION}"
    ))
}

pub fn wrong_kind(found: PackageType, wanted: &str) -> AppError {
    AppError::import(
        "wrong_package_kind",
        format!(
            "This is a {}, not {wanted}. Nothing was imported.",
            found.label()
        ),
    )
}

// ------------------------------------------------------------------ staging

/// A private temporary folder removed when dropped (never left behind on errors).
pub struct Staging {
    path: PathBuf,
    keep: bool,
}

impl Staging {
    /// Reserve (but do not create) a unique folder under `parent`.
    pub fn reserve(parent: &Path, prefix: &str) -> AppResult<Staging> {
        fs::create_dir_all(parent)?;
        Ok(Staging {
            path: parent.join(format!(".{prefix}-{}", new_id())),
            keep: false,
        })
    }
    pub fn create(parent: &Path, prefix: &str) -> AppResult<Staging> {
        let s = Staging::reserve(parent, prefix)?;
        fs::create_dir_all(&s.path)?;
        Ok(s)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Keep the folder (its contents were moved into place by the caller).
    pub fn keep(mut self) {
        self.keep = true;
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if !self.keep && self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

// ------------------------------------------------------------------ writing

/// Bytes or a file on disk to put into the package.
pub enum EntryData {
    Bytes(Vec<u8>),
    File(PathBuf),
}

impl EntryData {
    fn len(&self) -> u64 {
        match self {
            EntryData::Bytes(b) => b.len() as u64,
            EntryData::File(p) => fs::metadata(p).map(|m| m.len()).unwrap_or(0),
        }
    }
    fn sha256(&self) -> AppResult<String> {
        match self {
            EntryData::Bytes(b) => Ok(openframe_security::sha256_bytes(b)),
            EntryData::File(p) => openframe_security::sha256_file(p),
        }
    }
}

pub fn to_json_bytes<T: Serialize>(v: &T) -> AppResult<Vec<u8>> {
    serde_json::to_vec_pretty(v).map_err(|e| AppError::internal(e.to_string()))
}

/// Destination for a package the user chose in a save dialog: absolute, a
/// file (not a folder), in an existing folder, with the package extension.
pub fn package_destination(path: &str, ty: PackageType) -> AppResult<PathBuf> {
    let raw = path.trim();
    if raw.is_empty() {
        return Err(AppError::required("Save to"));
    }
    let mut p = PathBuf::from(raw);
    if !p.is_absolute() {
        return Err(AppError::invalid_input(
            "Choose where to save the package with Browse….",
        ));
    }
    if p.is_dir() {
        return Err(AppError::invalid_input(
            "Choose a file name for the package, not a folder.",
        ));
    }
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .map(openframe_security::sanitize_file_name)
        .ok_or_else(|| AppError::invalid_input("Choose a file name for the package."))?;
    let ext_ok = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(ty.extension()))
        .unwrap_or(false);
    let name = if ext_ok {
        name
    } else {
        format!("{name}.{}", ty.extension())
    };
    p.set_file_name(name);
    let parent = p
        .parent()
        .ok_or_else(|| AppError::invalid_input("Choose a folder for the package."))?;
    if !parent.is_dir() {
        return Err(AppError::new(
            "not_found.folder",
            "That folder can't be found. If it's on an external drive, reconnect it and try again.",
        ));
    }
    Ok(p)
}

/// Outcome of [`write_package`].
pub enum WriteOutcome {
    Written { bytes: u64 },
    Cancelled,
}

/// Write a package: every entry is hashed into `checksums.json`, the ZIP is
/// written to a uniquely named staging file in the destination folder and moved into place
/// with the shared atomic replace (`openframe_platform::atomic`) only when complete.
/// `cancelled` is polled between entries; a cancelled write leaves no file.
pub fn write_package(
    dest: &Path,
    manifest: &PackageManifest,
    entries: Vec<(String, EntryData)>,
    progress: &dyn Fn(f64, &str),
    cancelled: &dyn Fn() -> bool,
) -> AppResult<WriteOutcome> {
    let manifest_bytes = to_json_bytes(manifest)?;
    let mut files = BTreeMap::new();
    files.insert(
        MANIFEST_ENTRY.to_string(),
        openframe_security::sha256_bytes(&manifest_bytes),
    );
    let total: u64 = entries.iter().map(|(_, d)| d.len()).sum::<u64>().max(1);
    for (i, (name, data)) in entries.iter().enumerate() {
        if cancelled() {
            return Ok(WriteOutcome::Cancelled);
        }
        if name == MANIFEST_ENTRY || name == CHECKSUMS_ENTRY {
            return Err(AppError::internal("reserved package entry name"));
        }
        openframe_security::validate_relative(name)?;
        if files.insert(name.clone(), data.sha256()?).is_some() {
            return Err(AppError::internal(format!("duplicate package entry {i}")));
        }
    }
    let checksum_bytes = to_json_bytes(&Checksums {
        algorithm: "sha256".into(),
        files,
    })?;
    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| AppError::internal("package destination has no folder"))?;
    let tmp = openframe_platform::atomic::unique_temp_path(parent, dest)?;
    let mut stopped = false;
    let mut written: u64 = 0;
    let result = {
        let head = vec![
            (
                MANIFEST_ENTRY.to_string(),
                EntrySource::Bytes(&manifest_bytes),
            ),
            (
                CHECKSUMS_ENTRY.to_string(),
                EntrySource::Bytes(&checksum_bytes),
            ),
        ];
        let body = entries.iter().map_while(|(name, data)| {
            if cancelled() {
                stopped = true;
                return None;
            }
            written += data.len();
            progress(
                0.1 + 0.85 * (written as f64 / total as f64),
                "Writing package",
            );
            Some((
                name.clone(),
                match data {
                    EntryData::Bytes(b) => EntrySource::Bytes(b.as_slice()),
                    EntryData::File(p) => EntrySource::File(p.as_path()),
                },
            ))
        });
        archive::write_zip(&tmp, head.into_iter().chain(body))
    };
    if let Err(e) = result {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if stopped || cancelled() {
        let _ = fs::remove_file(&tmp);
        return Ok(WriteOutcome::Cancelled);
    }
    let bytes = fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    // The rename replaces an existing package in one step. No delete first: that left a crash
    // window with no file at `dest`, and `remove_file` also deletes a read-only file on
    // Windows, so a backup "completed" over a file the user had protected. A read-only or
    // locked destination makes the rename fail, the old file stays as it was and the staging
    // file is removed.
    openframe_platform::atomic::replace_from_staged(&tmp, dest)?;
    Ok(WriteOutcome::Written { bytes })
}

// ------------------------------------------------------------------ reading

/// Parse and check `manifest.json` bytes: format name, supported format
/// version (too new → refuse clearly), then the required metadata.
pub fn parse_manifest(bytes: &[u8]) -> AppResult<PackageManifest> {
    let raw: Value = serde_json::from_slice(bytes).map_err(|_| not_package())?;
    if raw.get("format").and_then(Value::as_str) != Some(FORMAT_NAME) {
        return Err(not_package());
    }
    let version = raw
        .get("formatVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| incomplete("manifest has no format version"))?;
    if version > PACKAGE_FORMAT_VERSION as u64 {
        return Err(too_new(version.min(u32::MAX as u64) as u32));
    }
    if version < OLDEST_PACKAGE_FORMAT_VERSION as u64 {
        return Err(incomplete(format!("unknown package format {version}")));
    }
    let m: PackageManifest =
        serde_json::from_value(raw).map_err(|e| incomplete(format!("manifest: {e}")))?;
    let valid = openframe_domain::ids::is_valid_id;
    if !valid(&m.package_id) || !valid(&m.source_project_id) {
        return Err(incomplete("manifest identities are not valid"));
    }
    if m.source_project_title.trim().is_empty() || m.originating_user.display_name.len() > 400 {
        return Err(incomplete("manifest metadata is missing"));
    }
    if m.package_type.is_exchange() && m.private_notes_included {
        // Private notes are never allowed in exchange packages (Security §8.6).
        return Err(incomplete("exchange package claims private notes"));
    }
    if m.package_type == PackageType::Response && m.responds_to.is_none() {
        return Err(incomplete("response package without its review package"));
    }
    Ok(m)
}

fn limits_for(ty: PackageType) -> ArchiveLimits {
    if ty.is_project_container() {
        ArchiveLimits::PACKAGE
    } else {
        ArchiveLimits::DOCUMENT
    }
}

/// Validate structure + version + metadata without extracting (Inspect).
/// Returns the manifest and the entry names.
pub fn read_header(path: &Path) -> AppResult<(PackageManifest, Vec<String>)> {
    // A package is a file the user picked: absolute, no device/ADS/reserved names.
    openframe_security::validate_input_file(path, u64::MAX).map_err(|e| {
        if e.is("security") {
            e
        } else {
            AppError::new(
                "not_found.file",
                "This package can't be found. If it's on an external drive, reconnect it and try again.",
            )
        }
    })?;
    if !path.is_file() {
        return Err(AppError::new(
            "not_found.file",
            "This package can't be found. If it's on an external drive, reconnect it and try again.",
        ));
    }
    let names = archive::inspect(path, ArchiveLimits::PACKAGE)?;
    let manifest_bytes =
        archive::read_entry(path, MANIFEST_ENTRY, MAX_META_BYTES)?.ok_or_else(|| {
            if names.is_empty() {
                not_package()
            } else {
                incomplete("manifest.json missing")
            }
        })?;
    let manifest = parse_manifest(&manifest_bytes)?;
    if manifest.package_type.is_exchange() {
        archive::inspect(path, limits_for(manifest.package_type))?;
    }
    let checksum_bytes = archive::read_entry(path, CHECKSUMS_ENTRY, MAX_META_BYTES)?
        .ok_or_else(|| incomplete("checksums.json missing"))?;
    let sums: Checksums = serde_json::from_slice(&checksum_bytes)
        .map_err(|e| incomplete(format!("checksums: {e}")))?;
    if sums.algorithm != "sha256" {
        return Err(incomplete("unsupported checksum algorithm"));
    }
    let listed: BTreeSet<&str> = sums.files.keys().map(String::as_str).collect();
    let present: BTreeSet<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|n| *n != CHECKSUMS_ENTRY)
        .collect();
    if listed != present {
        let missing: Vec<&&str> = listed.difference(&present).collect();
        return if missing.is_empty() {
            Err(checksum_mismatch("unlisted entries present"))
        } else {
            Err(incomplete(format!(
                "{} listed entries missing",
                missing.len()
            )))
        };
    }
    if manifest.package_type.is_exchange() && !present.contains(CONTENT_ENTRY) {
        return Err(incomplete("content.json missing"));
    }
    Ok((manifest, names))
}

/// A fully validated, extracted package. The staging folder is removed on drop.
pub struct OpenedPackage {
    pub manifest: PackageManifest,
    pub sha256: String,
    pub file_name: String,
    pub staging: Staging,
}

impl OpenedPackage {
    pub fn root(&self) -> &Path {
        self.staging.path()
    }
    pub fn entry_path(&self, name: &str) -> AppResult<PathBuf> {
        Ok(self
            .staging
            .path()
            .join(openframe_security::validate_relative(name)?))
    }
    pub fn content(&self) -> AppResult<Value> {
        let p = self.entry_path(CONTENT_ENTRY)?;
        let len = fs::metadata(&p)
            .map_err(|_| incomplete("content.json missing"))?
            .len();
        if len > MAX_CONTENT_BYTES {
            return Err(incomplete(format!("content.json is {len} bytes")));
        }
        let bytes = fs::read(&p).map_err(|_| incomplete("content.json missing"))?;
        serde_json::from_slice(&bytes).map_err(|e| incomplete(format!("content: {e}")))
    }
}

/// The full validation pipeline (Import/Export §13 steps 1–4): structure,
/// format version, metadata, then extraction into `staging_parent` with
/// archive limits and a checksum check of every entry.
pub fn open_package(
    path: &Path,
    staging_parent: &Path,
    accept: &[PackageType],
    wanted: &str,
) -> AppResult<OpenedPackage> {
    let (manifest, _) = read_header(path)?;
    if !accept.contains(&manifest.package_type) {
        return Err(wrong_kind(manifest.package_type, wanted));
    }
    let staging = Staging::reserve(staging_parent, "openframe-package")?;
    archive::extract_all(path, staging.path(), limits_for(manifest.package_type))?;
    let sums: Checksums = serde_json::from_slice(
        &fs::read(staging.path().join(CHECKSUMS_ENTRY))
            .map_err(|_| incomplete("checksums.json missing"))?,
    )
    .map_err(|e| incomplete(format!("checksums: {e}")))?;
    for (name, expected) in &sums.files {
        let p = staging
            .path()
            .join(openframe_security::validate_relative(name)?);
        if !p.is_file() {
            return Err(incomplete(format!("entry missing: {name}")));
        }
        let actual = openframe_security::sha256_file(&p)?;
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(checksum_mismatch(format!("entry changed: {name}")));
        }
    }
    // The manifest that was verified is the one we act on.
    let manifest = parse_manifest(&fs::read(staging.path().join(MANIFEST_ENTRY))?)?;
    let sha256 = openframe_security::sha256_file(path)?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("package")
        .to_string();
    Ok(OpenedPackage {
        manifest,
        sha256,
        file_name,
        staging,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(ty: PackageType) -> PackageManifest {
        PackageManifest {
            format: FORMAT_NAME.into(),
            package_type: ty,
            format_version: PACKAGE_FORMAT_VERSION,
            package_id: new_id(),
            source_project_id: new_id(),
            source_project_title: "Black Rain".into(),
            exported_at: 1,
            app_version: "test".into(),
            schema_version: None,
            source_draft: None,
            source_versions: vec![],
            included_object_ids: vec![],
            scope: PackageScope {
                kind: "full".into(),
                label: "Full".into(),
                ids: vec![],
            },
            comments_included: false,
            attachments_included: false,
            private_notes_included: false,
            base_snapshot: BTreeMap::new(),
            originating_user: PackageUser {
                user_id: new_id(),
                display_name: "Nisha".into(),
            },
            label: None,
            responds_to: None,
            table_counts: BTreeMap::new(),
            carried: vec![],
        }
    }

    #[test]
    fn package_round_trip_and_cancel_leaves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("p.ofstory");
        let m = manifest(PackageType::Story);
        let out = write_package(
            &dest,
            &m,
            vec![(CONTENT_ENTRY.into(), EntryData::Bytes(b"{}".to_vec()))],
            &|_, _| {},
            &|| false,
        )
        .unwrap();
        assert!(matches!(out, WriteOutcome::Written { .. }));
        let opened = open_package(&dest, dir.path(), &[PackageType::Story], "x").unwrap();
        assert_eq!(opened.manifest.package_id, m.package_id);
        let staging = opened.root().to_path_buf();
        drop(opened);
        assert!(!staging.exists());

        let dest2 = dir.path().join("q.ofstory");
        let out = write_package(
            &dest2,
            &m,
            vec![(CONTENT_ENTRY.into(), EntryData::Bytes(b"{}".to_vec()))],
            &|_, _| {},
            &|| true,
        )
        .unwrap();
        assert!(matches!(out, WriteOutcome::Cancelled));
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            !names.iter().any(|n| n.starts_with("q.")
                || n.starts_with(".q.")
                || n.contains(".tmp")
                || n.contains(".partial")
                || n.ends_with(openframe_platform::atomic::TEMP_SUFFIX)),
            "{names:?}"
        );
    }

    /// A write that fails part way (an entry file vanished) leaves neither the package nor a
    /// staging file, and an existing package at the destination is kept.
    #[test]
    fn failed_write_leaves_no_partial_package() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("p.ofbackup");
        fs::write(&dest, b"previous package").unwrap();
        let m = manifest(PackageType::Backup);
        let gone = dir.path().join("vanished.bin");
        let err = write_package(
            &dest,
            &m,
            vec![
                ("a.bin".into(), EntryData::Bytes(vec![1; 1024])),
                ("b.bin".into(), EntryData::File(gone)),
            ],
            &|_, _| {},
            &|| false,
        );
        assert!(err.is_err());
        assert_eq!(fs::read(&dest).unwrap(), b"previous package");
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["p.ofbackup".to_string()]);
    }

    #[test]
    fn format_versions_one_and_two_are_read_and_zero_is_refused() {
        for v in [1, PACKAGE_FORMAT_VERSION] {
            let mut m = manifest(PackageType::Story);
            m.format_version = v;
            assert!(parse_manifest(&serde_json::to_vec(&m).unwrap()).is_ok());
        }
        // A format 1 manifest has neither of the fields format 2 added.
        let mut raw = serde_json::to_value(manifest(PackageType::Backup)).unwrap();
        raw.as_object_mut().unwrap().remove("tableCounts");
        raw.as_object_mut().unwrap().remove("carried");
        raw["formatVersion"] = 1.into();
        let m = parse_manifest(&serde_json::to_vec(&raw).unwrap()).unwrap();
        assert!(m.table_counts.is_empty() && m.carried.is_empty());
        raw["formatVersion"] = 0.into();
        assert_eq!(
            parse_manifest(&serde_json::to_vec(&raw).unwrap())
                .unwrap_err()
                .code_str(),
            "import.package_incomplete"
        );
    }

    #[test]
    fn manifest_checks() {
        let mut m = manifest(PackageType::Story);
        m.format_version = 99;
        let err = parse_manifest(&serde_json::to_vec(&m).unwrap()).unwrap_err();
        assert_eq!(err.code_str(), "import.package_too_new");
        let mut m = manifest(PackageType::Story);
        m.private_notes_included = true;
        assert!(parse_manifest(&serde_json::to_vec(&m).unwrap()).is_err());
        assert_eq!(
            parse_manifest(b"{\"format\":\"other\"}")
                .unwrap_err()
                .code_str(),
            "import.not_package"
        );
    }
}

/// Hostile input for the package reader (security standard: bounded parsers, confined paths).
/// Every case must be refused with a stable error code, never panic, and leave nothing in the
/// staging folder or outside it.
#[cfg(test)]
mod hostile {
    use super::*;
    use std::io::Write;

    /// Stored ZIP writer that allows any name, duplicate names and a Unix mode (symlinks).
    fn raw_zip(path: &Path, entries: &[(&str, &[u8], Option<u32>)]) {
        fn crc32(data: &[u8]) -> u32 {
            let mut crc = 0xFFFF_FFFFu32;
            for &b in data {
                crc ^= b as u32;
                for _ in 0..8 {
                    crc = if crc & 1 != 0 {
                        (crc >> 1) ^ 0xEDB8_8320
                    } else {
                        crc >> 1
                    };
                }
            }
            !crc
        }
        let mut out: Vec<u8> = vec![];
        let mut central: Vec<u8> = vec![];
        for (name, data, mode) in entries {
            let offset = out.len() as u32;
            let crc = crc32(data);
            let len = data.len() as u32;
            out.extend(0x0403_4b50u32.to_le_bytes());
            out.extend(20u16.to_le_bytes());
            out.extend([0u8; 8]);
            out.extend(crc.to_le_bytes());
            out.extend(len.to_le_bytes());
            out.extend(len.to_le_bytes());
            out.extend((name.len() as u16).to_le_bytes());
            out.extend(0u16.to_le_bytes());
            out.extend(name.as_bytes());
            out.extend(*data);
            central.extend(0x0201_4b50u32.to_le_bytes());
            // "Made by" Unix (3) when a mode is given, so readers honour the mode bits.
            let made_by: u16 = if mode.is_some() { (3 << 8) | 20 } else { 20 };
            central.extend(made_by.to_le_bytes());
            central.extend(20u16.to_le_bytes());
            central.extend([0u8; 8]);
            central.extend(crc.to_le_bytes());
            central.extend(len.to_le_bytes());
            central.extend(len.to_le_bytes());
            central.extend((name.len() as u16).to_le_bytes());
            central.extend([0u8; 8]);
            central.extend((mode.unwrap_or(0) << 16).to_le_bytes());
            central.extend(offset.to_le_bytes());
            central.extend(name.as_bytes());
        }
        let cd_offset = out.len() as u32;
        let cd_size = central.len() as u32;
        out.extend(central);
        out.extend(0x0605_4b50u32.to_le_bytes());
        out.extend([0u8; 4]);
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend(cd_size.to_le_bytes());
        out.extend(cd_offset.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        fs::write(path, out).unwrap();
    }

    fn manifest_json(ty: &str) -> Value {
        serde_json::json!({
            "format": FORMAT_NAME, "packageType": ty, "formatVersion": PACKAGE_FORMAT_VERSION,
            "packageId": new_id(), "sourceProjectId": new_id(), "sourceProjectTitle": "Black Rain",
            "exportedAt": 1, "appVersion": "1", "scope": { "kind": "full", "label": "Full" },
            "commentsIncluded": false, "attachmentsIncluded": false, "privateNotesIncluded": false,
            "originatingUser": { "userId": new_id(), "displayName": "Mallory" }
        })
    }

    fn checksums(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let files: serde_json::Map<String, Value> = entries
            .iter()
            .map(|(n, b)| {
                (
                    n.to_string(),
                    Value::from(openframe_security::sha256_bytes(b)),
                )
            })
            .collect();
        serde_json::to_vec(&serde_json::json!({ "algorithm": "sha256", "files": files })).unwrap()
    }

    /// A well-formed Story Board package whose checksums match, plus `extra` raw entries.
    fn package(path: &Path, manifest: &[u8], content: &[u8], extra: &[(&str, &[u8], Option<u32>)]) {
        let mut listed: Vec<(&str, &[u8])> =
            vec![(MANIFEST_ENTRY, manifest), (CONTENT_ENTRY, content)];
        listed.extend(extra.iter().map(|(n, b, _)| (*n, *b)));
        let sums = checksums(&listed);
        let mut all: Vec<(&str, &[u8], Option<u32>)> = vec![
            (MANIFEST_ENTRY, manifest, None),
            (CONTENT_ENTRY, content, None),
        ];
        all.extend_from_slice(extra);
        all.push((CHECKSUMS_ENTRY, &sums, None));
        raw_zip(path, &all);
    }

    struct Case {
        what: &'static str,
        codes: &'static [&'static str],
    }

    fn refuse(dir: &Path, file: &Path, case: Case) {
        let staging = dir.join("staging");
        fs::create_dir_all(&staging).unwrap();
        let result = open_package(file, &staging, &PackageType::ALL, "a package");
        let err = match result {
            Ok(_) => panic!("{}: a hostile package was opened", case.what),
            Err(e) => e,
        };
        assert!(
            case.codes.contains(&err.code_str()),
            "{}: got {} ({:?})",
            case.what,
            err.code_str(),
            err.detail
        );
        // The message is for the writer: no paths, no parser jargon.
        assert!(
            !err.message.contains('\\') && !err.message.contains("serde"),
            "{}",
            err.message
        );
        let left: Vec<_> = fs::read_dir(&staging)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .collect();
        assert!(left.is_empty(), "{}: staging kept {left:?}", case.what);
        fs::remove_dir_all(&staging).unwrap();
        // Nothing was written next to the package or above it.
        for name in ["escape.txt", "evil", "pwned.txt"] {
            assert!(!dir.join(name).exists(), "{}: wrote {name}", case.what);
            assert!(
                !dir.parent().unwrap().join(name).exists(),
                "{}: wrote {name}",
                case.what
            );
        }
    }

    const UNSAFE: &[&str] = &["import.unsafe_archive"];
    const NOT_PACKAGE: &[&str] = &["import.not_package", "import.unsafe_archive"];
    const INCOMPLETE: &[&str] = &["import.package_incomplete"];
    const DAMAGED: &[&str] = &["import.checksum_mismatch"];

    #[test]
    fn files_that_are_not_packages_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("in");
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("p.ofstory");

        fs::write(&f, b"").unwrap();
        refuse(
            &dir,
            &f,
            Case {
                what: "empty file",
                codes: NOT_PACKAGE,
            },
        );

        let noise: Vec<u8> = (0..8192u32)
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
            .collect();
        fs::write(&f, &noise).unwrap();
        refuse(
            &dir,
            &f,
            Case {
                what: "random bytes",
                codes: NOT_PACKAGE,
            },
        );

        let m = serde_json::to_vec(&manifest_json("story")).unwrap();
        package(&f, &m, b"{}", &[]);
        let whole = fs::read(&f).unwrap();
        fs::write(&f, &whole[..whole.len() / 2]).unwrap();
        refuse(
            &dir,
            &f,
            Case {
                what: "truncated package",
                codes: NOT_PACKAGE,
            },
        );

        // The end-of-directory record claims 65535 entries in a tiny file.
        let mut lying = whole.clone();
        let eocd = lying.len() - 22;
        lying[eocd + 8..eocd + 12].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        fs::write(&f, &lying).unwrap();
        refuse(
            &dir,
            &f,
            Case {
                what: "lying entry count",
                codes: NOT_PACKAGE,
            },
        );

        raw_zip(&f, &[]);
        refuse(
            &dir,
            &f,
            Case {
                what: "empty archive",
                codes: NOT_PACKAGE,
            },
        );

        raw_zip(&f, &[("hello.txt", b"hi", None)]);
        refuse(
            &dir,
            &f,
            Case {
                what: "archive without a manifest",
                codes: INCOMPLETE,
            },
        );
    }

    #[test]
    fn hostile_manifests_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("in");
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("p.ofstory");
        let with = |edit: &dyn Fn(&mut Value)| {
            let mut m = manifest_json("story");
            edit(&mut m);
            serde_json::to_vec(&m).unwrap()
        };

        package(&f, b"not json at all", b"{}", &[]);
        refuse(
            &dir,
            &f,
            Case {
                what: "manifest is not JSON",
                codes: &["import.not_package"],
            },
        );

        // 100,000 nested arrays: the parser's depth limit refuses it without a stack overflow.
        let deep = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
        package(&f, deep.as_bytes(), b"{}", &[]);
        refuse(
            &dir,
            &f,
            Case {
                what: "deeply nested manifest",
                codes: &["import.not_package"],
            },
        );

        let big = vec![b' '; (MAX_META_BYTES + 1) as usize];
        package(&f, &big, b"{}", &[]);
        refuse(
            &dir,
            &f,
            Case {
                what: "oversized manifest",
                codes: UNSAFE,
            },
        );

        let cases: Vec<(&str, Vec<u8>, &[&str])> = vec![
            (
                "other format name",
                with(&|m| m["format"] = "zip-bundle".into()),
                &["import.not_package"],
            ),
            (
                "no format version",
                with(&|m| {
                    m.as_object_mut().unwrap().remove("formatVersion");
                }),
                INCOMPLETE,
            ),
            (
                "format version as text",
                with(&|m| m["formatVersion"] = "2".into()),
                INCOMPLETE,
            ),
            (
                "negative format version",
                with(&|m| m["formatVersion"] = (-1).into()),
                INCOMPLETE,
            ),
            (
                "enormous format version",
                with(&|m| m["formatVersion"] = u64::MAX.into()),
                &["import.package_too_new"],
            ),
            (
                "unknown package type",
                with(&|m| m["packageType"] = "hologram".into()),
                INCOMPLETE,
            ),
            (
                "package id is not an id",
                with(&|m| m["packageId"] = "../../etc".into()),
                INCOMPLETE,
            ),
            (
                "project id is not an id",
                with(&|m| m["sourceProjectId"] = "".into()),
                INCOMPLETE,
            ),
            (
                "blank project title",
                with(&|m| m["sourceProjectTitle"] = "   ".into()),
                INCOMPLETE,
            ),
            (
                "very long sender name",
                with(&|m| m["originatingUser"]["displayName"] = "x".repeat(5000).into()),
                INCOMPLETE,
            ),
            (
                "exchange package claims private notes",
                with(&|m| m["privateNotesIncluded"] = true.into()),
                INCOMPLETE,
            ),
            (
                "response without its review package",
                with(&|m| m["packageType"] = "response".into()),
                INCOMPLETE,
            ),
            (
                "table counts of the wrong type",
                with(&|m| m["tableCounts"] = serde_json::json!({ "scene": "many" })),
                INCOMPLETE,
            ),
        ];
        for (what, bytes, codes) in cases {
            package(&f, &bytes, b"{}", &[]);
            refuse(&dir, &f, Case { what, codes });
        }
    }

    #[test]
    fn damaged_or_incomplete_checksum_lists_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("in");
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("p.ofstory");
        let m = serde_json::to_vec(&manifest_json("story")).unwrap();
        let content = b"{\"cards\":[]}";

        raw_zip(
            &f,
            &[(MANIFEST_ENTRY, &m, None), (CONTENT_ENTRY, content, None)],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "no checksum list",
                codes: INCOMPLETE,
            },
        );

        raw_zip(
            &f,
            &[
                (MANIFEST_ENTRY, &m, None),
                (CONTENT_ENTRY, content, None),
                (CHECKSUMS_ENTRY, b"[1,2", None),
            ],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "checksum list is not JSON",
                codes: INCOMPLETE,
            },
        );

        let md5 =
            serde_json::to_vec(&serde_json::json!({ "algorithm": "md5", "files": {} })).unwrap();
        raw_zip(
            &f,
            &[
                (MANIFEST_ENTRY, &m, None),
                (CONTENT_ENTRY, content, None),
                (CHECKSUMS_ENTRY, &md5, None),
            ],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "weak checksum algorithm",
                codes: INCOMPLETE,
            },
        );

        // Lists an entry the archive does not hold.
        let sums = checksums(&[
            (MANIFEST_ENTRY, &m),
            (CONTENT_ENTRY, content),
            ("assets/photo.jpg", b"x"),
        ]);
        raw_zip(
            &f,
            &[
                (MANIFEST_ENTRY, &m, None),
                (CONTENT_ENTRY, content, None),
                (CHECKSUMS_ENTRY, &sums, None),
            ],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "listed entry missing",
                codes: INCOMPLETE,
            },
        );

        // Holds an entry the list does not name (smuggled file).
        let sums = checksums(&[(MANIFEST_ENTRY, &m), (CONTENT_ENTRY, content)]);
        raw_zip(
            &f,
            &[
                (MANIFEST_ENTRY, &m, None),
                (CONTENT_ENTRY, content, None),
                ("extra.bin", b"x", None),
                (CHECKSUMS_ENTRY, &sums, None),
            ],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "unlisted entry",
                codes: DAMAGED,
            },
        );

        // Content changed after export.
        raw_zip(
            &f,
            &[
                (MANIFEST_ENTRY, &m, None),
                (CONTENT_ENTRY, b"{\"cards\":[1]}", None),
                (CHECKSUMS_ENTRY, &sums, None),
            ],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "changed content",
                codes: DAMAGED,
            },
        );

        // Manifest swapped after export (a different package id).
        let other = serde_json::to_vec(&manifest_json("story")).unwrap();
        raw_zip(
            &f,
            &[
                (MANIFEST_ENTRY, &other, None),
                (CONTENT_ENTRY, content, None),
                (CHECKSUMS_ENTRY, &sums, None),
            ],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "swapped manifest",
                codes: DAMAGED,
            },
        );

        // An exchange package without its content.
        let sums = checksums(&[(MANIFEST_ENTRY, &m)]);
        raw_zip(
            &f,
            &[(MANIFEST_ENTRY, &m, None), (CHECKSUMS_ENTRY, &sums, None)],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "exchange package without content",
                codes: INCOMPLETE,
            },
        );
    }

    #[test]
    fn unsafe_entry_names_links_and_bombs_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("in");
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("p.ofstory");
        let m = serde_json::to_vec(&manifest_json("story")).unwrap();
        let names = [
            "../escape.txt",
            "../../pwned.txt",
            "assets/../../escape.txt",
            "..\\escape.txt",
            "/escape.txt",
            "\\\\server\\share\\escape.txt",
            "C:/escape.txt",
            "C:escape.txt",
            "content.json:hidden",
            "assets/CON",
            "assets/nul.txt",
            "assets/./photo.jpg",
            "assets//photo.jpg",
            "assets/photo.jpg.",
            "assets/photo\u{0}.jpg",
            "CONTEN~1.JSO",
        ];
        for name in names {
            package(&f, &m, b"{}", &[(name, b"pwned", None)]);
            refuse(
                &dir,
                &f,
                Case {
                    what: "unsafe entry name",
                    codes: UNSAFE,
                },
            );
        }

        // Two entries that land on one file on Windows and default macOS volumes.
        package(
            &f,
            &m,
            b"{}",
            &[
                ("assets/Photo.jpg", b"a", None),
                ("assets/photo.JPG", b"b", None),
            ],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "case collision",
                codes: UNSAFE,
            },
        );
        // A file and a folder with one name.
        package(
            &f,
            &m,
            b"{}",
            &[("assets", b"a", None), ("assets/photo.jpg", b"b", None)],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "file and folder collide",
                codes: UNSAFE,
            },
        );
        // The same name twice: readers disagree about which one is real.
        package(
            &f,
            &m,
            b"{}",
            &[
                ("assets/photo.jpg", b"a", None),
                ("assets/photo.jpg", b"b", None),
            ],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "duplicate entry",
                codes: UNSAFE,
            },
        );
        // A symbolic link entry pointing outside the package.
        package(
            &f,
            &m,
            b"{}",
            &[(
                "assets/evil",
                b"../../../../Windows/System32",
                Some(0o120777),
            )],
        );
        refuse(
            &dir,
            &f,
            Case {
                what: "symlink entry",
                codes: UNSAFE,
            },
        );

        // Decompression bomb: 16 MB of zeros deflate to a few KB.
        {
            let mut w = zip::ZipWriter::new(fs::File::create(&f).unwrap());
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            w.start_file(MANIFEST_ENTRY, opts).unwrap();
            w.write_all(&m).unwrap();
            w.start_file("assets/zeros.bin", opts).unwrap();
            let zeros = vec![0u8; 1 << 20];
            for _ in 0..16 {
                w.write_all(&zeros).unwrap();
            }
            w.finish().unwrap();
        }
        refuse(
            &dir,
            &f,
            Case {
                what: "compression bomb",
                codes: UNSAFE,
            },
        );
    }

    #[test]
    fn oversized_or_misshapen_content_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("in");
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("p.ofscriptreview");
        let m = serde_json::to_vec(&manifest_json("scriptReview")).unwrap();
        let staging = dir.join("staging");
        fs::create_dir_all(&staging).unwrap();

        // Content of the wrong shape parses as JSON but is refused by the content model.
        for bad in [
            &br#"{"scenes":"all of them"}"#[..],
            br#"{"scenes":[{"number":-4}]}"#,
            br#"{"scenes":[{"elements":[{"text":7}]}]}"#,
            br#""a string""#,
        ] {
            package(&f, &m, bad, &[]);
            let opened = open_package(&f, &staging, &PackageType::ALL, "a package").unwrap();
            let content = opened.content().unwrap();
            let err =
                super::super::exchange::parse_content::<super::super::exchange::ScriptContent>(
                    &content,
                )
                .unwrap_err();
            assert_eq!(err.code_str(), "import.package_incomplete");
        }

        // Content larger than any real exchange package is refused before parsing.
        let mut huge = b"{\"scenes\":[], \"pad\":\"".to_vec();
        huge.resize(MAX_CONTENT_BYTES as usize + 16, b'a');
        huge.extend(b"\"}");
        package(&f, &m, &huge, &[]);
        let opened = open_package(&f, &staging, &PackageType::ALL, "a package").unwrap();
        assert_eq!(
            opened.content().unwrap_err().code_str(),
            "import.package_incomplete"
        );
        let root_of = opened.root().to_path_buf();
        drop(opened);
        assert!(
            !root_of.exists(),
            "staging folder kept after the package was dropped"
        );
    }

    /// Wrong kind: a Story Board package offered where a backup is expected is refused by kind,
    /// before anything is extracted.
    #[test]
    fn a_package_of_another_kind_is_refused_before_extraction() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("in");
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("p.ofbackup");
        let m = serde_json::to_vec(&manifest_json("story")).unwrap();
        package(&f, &m, b"{}", &[]);
        let staging = dir.join("staging");
        fs::create_dir_all(&staging).unwrap();
        let err = match open_package(&f, &staging, &[PackageType::Backup], "a backup") {
            Ok(_) => panic!("a Story Board package was opened as a backup"),
            Err(e) => e,
        };
        assert_eq!(err.code_str(), "import.wrong_package_kind");
        assert_eq!(fs::read_dir(&staging).unwrap().count(), 0);
    }
}

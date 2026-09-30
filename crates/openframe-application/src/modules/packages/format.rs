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

use openframe_domain::{AppError, AppResult, PACKAGE_FORMAT_VERSION, new_id};
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

/// `manifest.json` — the package metadata required by Import/Export §9.3.
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
        "This package was created by a newer version of OpenFrame. Update OpenFrame to open it — nothing was changed.",
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
/// written to a temporary sibling and renamed into place only when complete.
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
    let file_name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("package")
        .to_string();
    let tmp = dest.with_file_name(format!(".{file_name}.{}.tmp", new_id()));
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
    if dest.exists()
        && let Err(e) = fs::remove_file(dest)
    {
        let _ = fs::remove_file(&tmp);
        return Err(e.into());
    }
    if let Err(e) = fs::rename(&tmp, dest) {
        let _ = fs::remove_file(&tmp);
        return Err(e.into());
    }
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
            !names
                .iter()
                .any(|n| n.starts_with("q.") || n.contains(".tmp") || n.contains(".partial")),
            "{names:?}"
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

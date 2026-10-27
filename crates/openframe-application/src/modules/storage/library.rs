//! The library file (`library.json`, ADR-0038, doc 40 section 2): the person's identity and the
//! places they chose, kept next to their projects so an app data reset or a reinstall does not
//! make them a stranger to their own private notes.
//!
//! Reading never fails the app: a file that is missing, damaged or wrong is not trusted (logged)
//! and the app starts as it would without one. A file that is there but unusable is never
//! overwritten at start (see [`Read`]).

use std::path::{Path, PathBuf};

use openframe_domain::{AppError, AppResult};
use serde::{Deserialize, Serialize};

pub const LIBRARY_FILE_NAME: &str = "library.json";
const FORMAT: u32 = 1;
/// The file holds a few hundred bytes; anything bigger is not ours.
const MAX_BYTES: u64 = 64 * 1024;
const MAX_NAME_CHARS: usize = 200;
/// An unusable file is kept beside the original as `library.json.unreadable-<ms>`.
const UNREADABLE_MARKER: &str = ".unreadable-";

/// What the file holds on disk, in the order and spelling of the contract.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Raw {
    format: u32,
    user_id: String,
    display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    projects_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vault_dir: Option<String>,
    updated_at: i64,
}

/// A library file that passed every check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    pub user_id: String,
    pub display_name: String,
    /// The folder the person chose for new projects, when they chose one.
    pub projects_dir: Option<PathBuf>,
    /// The Global Idea Vault folder, when they moved it.
    pub vault_dir: Option<PathBuf>,
}

/// What reading the library file found. `Unusable` is not the same as `Absent`: a file that is
/// there but cannot be trusted (a read error that may pass, a half-written file, a newer format)
/// may still hold the person's chosen folders, so it must never be overwritten with defaults.
#[derive(Debug, PartialEq, Eq)]
pub enum Read {
    Absent,
    Valid(Library),
    Unusable,
}

/// Read and validate the file at `path`. A folder that is invalid is dropped on its own (the
/// identity stays); an invalid identity makes the whole file `Unusable`.
pub fn read(path: &Path, app_data_dir: &Path) -> Read {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Read::Absent,
        Err(e) => {
            tracing::warn!(error = %e, "the library file could not be read and is left as it is");
            return Read::Unusable;
        }
    };
    if meta.len() > MAX_BYTES {
        tracing::warn!("the library file is too large and is left as it is");
        return Read::Unusable;
    }
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "the library file could not be read and is left as it is");
            return Read::Unusable;
        }
    };
    match parse(&text, app_data_dir) {
        Ok(library) => Read::Valid(library),
        Err(why) => {
            tracing::warn!(
                reason = why,
                "the library file is not valid and is left as it is"
            );
            Read::Unusable
        }
    }
}

/// Copy an unusable library file to `library.json.unreadable-<ms>` beside it, so whatever it
/// held can be recovered by hand. Best effort. A file over 1 MB is not ours and is not copied. A
/// copy that already holds the same bytes is reused, so a file that stays unusable does not add
/// a copy at every start.
pub fn preserve_unusable(path: &Path) -> Option<PathBuf> {
    let bytes = match std::fs::metadata(path) {
        Ok(m) if m.len() <= 1 << 20 => std::fs::read(path).ok()?,
        _ => return None,
    };
    let dir = path.parent()?;
    let prefix = format!("{}{UNREADABLE_MARKER}", path.file_name()?.to_string_lossy());
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        if entry.file_name().to_string_lossy().starts_with(&prefix)
            && std::fs::read(entry.path()).is_ok_and(|b| b == bytes)
        {
            return Some(entry.path());
        }
    }
    let stamp = openframe_domain::now_ms();
    for n in 0..100 {
        let copy = dir.join(format!("{prefix}{}", stamp + n));
        let created = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&copy);
        if let Ok(mut f) = created {
            use std::io::Write;
            return f.write_all(&bytes).ok().map(|()| copy);
        }
    }
    None
}

/// Parse and validate the text of a library file. The error is a short reason for the log.
pub fn parse(text: &str, app_data_dir: &Path) -> Result<Library, &'static str> {
    let raw: Raw = serde_json::from_str(text).map_err(|_| "it does not parse")?;
    if raw.format != FORMAT {
        return Err("unknown format");
    }
    if uuid::Uuid::parse_str(&raw.user_id).is_err() {
        return Err("the user id is not a UUID");
    }
    let name = raw.display_name.trim();
    if name.is_empty()
        || name.chars().count() > MAX_NAME_CHARS
        || name.chars().any(char::is_control)
    {
        return Err("the display name is not usable");
    }
    Ok(Library {
        user_id: raw.user_id,
        display_name: name.to_string(),
        projects_dir: raw
            .projects_dir
            .and_then(|p| usable_dir(&p, app_data_dir, "projectsDir")),
        vault_dir: raw
            .vault_dir
            .and_then(|p| usable_dir(&p, app_data_dir, "vaultDir")),
    })
}

/// A chosen folder from the file: absolute, a well-formed local path, not inside the app data
/// folder. Existence is not checked here: a drive that is not connected is not a bad file.
fn usable_dir(raw: &str, app_data_dir: &Path, field: &str) -> Option<PathBuf> {
    let path = PathBuf::from(raw);
    let verdict = if !path.is_absolute() {
        Err("is not an absolute path")
    } else if openframe_security::check_user_path(&path, openframe_security::NetworkPaths::Refuse)
        .is_err()
    {
        Err("is not an allowed location")
    } else if is_inside(app_data_dir, &path) {
        Err("is inside the app data folder")
    } else {
        Ok(())
    };
    match verdict {
        Ok(()) => Some(path),
        Err(why) => {
            tracing::warn!(field, why, "a folder in the library file is ignored");
            None
        }
    }
}

/// Lexical comparison that ignores case and slash direction (Windows paths). No file system
/// access, so it works for folders that do not exist.
pub fn same_place(a: &Path, b: &Path) -> bool {
    place_key(a) == place_key(b)
}

/// True when `path` is `root` or lies inside it (lexically).
pub fn is_inside(root: &Path, path: &Path) -> bool {
    let (root, path) = (place_key(root), place_key(path));
    path == root || path.starts_with(&format!("{root}/"))
}

pub fn place_key(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/").to_lowercase();
    s.trim_end_matches('/').to_string()
}

/// Write the file atomically. The folder is created when it does not exist yet.
pub fn write(
    path: &Path,
    user_id: &str,
    display_name: &str,
    projects_dir: Option<&Path>,
    vault_dir: Option<&Path>,
) -> AppResult<()> {
    let raw = Raw {
        format: FORMAT,
        user_id: user_id.to_string(),
        display_name: display_name.to_string(),
        projects_dir: projects_dir.map(|p| p.to_string_lossy().into_owned()),
        vault_dir: vault_dir.map(|p| p.to_string_lossy().into_owned()),
        updated_at: openframe_domain::now_ms(),
    };
    let bytes = serde_json::to_vec_pretty(&raw).map_err(|e| AppError::internal(e.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    openframe_platform::atomic::replace_bytes(path, &bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0192f0a1-7b3c-7d4e-8f50-123456789abc";

    fn app_data() -> PathBuf {
        std::env::temp_dir().join("of-library-appdata")
    }

    fn text(extra: &str) -> String {
        format!(
            r#"{{ "format": 1, "userId": "{ID}", "displayName": "Meera", "updatedAt": 5 {extra} }}"#
        )
    }

    fn quoted(p: &Path) -> String {
        serde_json::to_string(&p.to_string_lossy()).unwrap()
    }

    #[test]
    fn a_minimal_file_gives_identity_and_default_places() {
        let lib = parse(&text(""), &app_data()).unwrap();
        assert_eq!(lib.user_id, ID);
        assert_eq!(lib.display_name, "Meera");
        assert_eq!(lib.projects_dir, None);
        assert_eq!(lib.vault_dir, None);
    }

    #[test]
    fn chosen_folders_are_kept_when_absolute() {
        let films = std::env::temp_dir().join("Films");
        let vault = films.join("Vault");
        let extra = format!(
            ", \"projectsDir\": {}, \"vaultDir\": {}",
            quoted(&films),
            quoted(&vault)
        );
        let lib = parse(&text(&extra), &app_data()).unwrap();
        assert_eq!(lib.projects_dir, Some(films));
        assert_eq!(lib.vault_dir, Some(vault));
    }

    #[test]
    fn a_bad_identity_drops_the_whole_file() {
        let app = app_data();
        assert!(parse("not json", &app).is_err());
        assert!(parse("{}", &app).is_err());
        assert!(parse(&text("").replace("\"format\": 1", "\"format\": 2"), &app).is_err());
        assert!(parse(&text("").replace(ID, "meera@example.com"), &app).is_err());
        assert!(parse(&text("").replace("Meera", "  "), &app).is_err());
    }

    #[test]
    fn a_bad_folder_is_dropped_but_the_identity_stays() {
        let app = app_data();
        let lib = parse(
            &text(r#", "projectsDir": "relative/films", "vaultDir": "" "#),
            &app,
        )
        .unwrap();
        assert_eq!(lib.user_id, ID);
        assert_eq!((lib.projects_dir, lib.vault_dir), (None, None));
        // Inside the app data folder is refused as well.
        let extra = format!(", \"vaultDir\": {}", quoted(&app.join("vault")));
        let lib = parse(&text(&extra), &app).unwrap();
        assert_eq!(lib.vault_dir, None);
    }

    #[test]
    fn places_compare_without_case_or_slash_direction() {
        assert!(same_place(
            Path::new("C:/Docs/Vault/"),
            Path::new("c:\\docs\\vault")
        ));
        assert!(is_inside(Path::new("C:/a"), Path::new("c:\\A\\b")));
        assert!(!is_inside(Path::new("C:/a"), Path::new("C:/ab")));
    }

    #[test]
    fn what_is_written_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sub").join(LIBRARY_FILE_NAME);
        let films = dir.path().join("Films");
        write(&file, ID, "Meera", Some(&films), None).unwrap();
        let Read::Valid(lib) = read(&file, &app_data()) else {
            panic!("the file reads back");
        };
        assert_eq!(lib.user_id, ID);
        assert_eq!(lib.projects_dir, Some(films));
        assert_eq!(lib.vault_dir, None);
        assert_eq!(
            read(&dir.path().join("none.json"), &app_data()),
            Read::Absent
        );
    }

    #[test]
    fn a_file_that_is_there_but_cannot_be_trusted_is_unusable_not_absent() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(LIBRARY_FILE_NAME);
        for bad in [
            text("").replace("\"format\": 1", "\"format\": 2"),
            text("")[..30].to_string(),
            "x".repeat(70_000),
        ] {
            std::fs::write(&file, &bad).unwrap();
            assert_eq!(read(&file, &app_data()), Read::Unusable);
        }
        // A folder where the file should be is a read error, not a missing file.
        let as_folder = dir.path().join("folder");
        std::fs::create_dir(&as_folder).unwrap();
        assert_eq!(read(&as_folder, &app_data()), Read::Unusable);
    }

    #[test]
    fn an_unusable_file_is_copied_aside_once_per_content() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(LIBRARY_FILE_NAME);
        std::fs::write(&file, b"{ half a file").unwrap();
        let first = preserve_unusable(&file).expect("copied");
        assert_eq!(std::fs::read(&first).unwrap(), b"{ half a file");
        assert_eq!(
            preserve_unusable(&file),
            Some(first.clone()),
            "no second copy"
        );
        std::fs::write(&file, b"{ another").unwrap();
        let second = preserve_unusable(&file).expect("a different content is kept too");
        assert_ne!(second, first);
        assert_eq!(std::fs::read(&first).unwrap(), b"{ half a file");
    }
}

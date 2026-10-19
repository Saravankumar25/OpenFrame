//! Cloud-synced folders (Windows phase W2). A project folder that OneDrive, Dropbox, Google Drive
//! or iCloud Drive syncs while OpenFrame writes to it can be locked, replaced by an older copy or
//! split into conflict copies. This names the service that syncs a folder, so the app can warn in
//! plain words and offer a folder on this computer instead. It never moves anything.
//!
//! A folder is synced when one of its parent folders is:
//! - a OneDrive root named by Windows (the `OneDrive`, `OneDriveConsumer` and `OneDriveCommercial`
//!   environment variables; Documents redirected to OneDrive lands under them);
//! - named like a sync root (`OneDrive`, `OneDrive - Company`, `Dropbox`, `Dropbox (Team)`,
//!   `Google Drive`, `My Drive`, `iCloudDrive`, `iCloud Drive`, and on the Mac
//!   `Library/CloudStorage/<Service>-...` and `Library/Mobile Documents`);
//! - marked by the service (`.dropbox` or `.dropbox.cache` inside the Dropbox root).

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudService {
    OneDrive,
    Dropbox,
    GoogleDrive,
    ICloudDrive,
}

impl CloudService {
    /// The name people know the service by.
    pub fn name(self) -> &'static str {
        match self {
            Self::OneDrive => "OneDrive",
            Self::Dropbox => "Dropbox",
            Self::GoogleDrive => "Google Drive",
            Self::ICloudDrive => "iCloud Drive",
        }
    }
}

/// The service that syncs `path`, if any.
pub fn cloud_service(path: &Path) -> Option<CloudService> {
    cloud_service_with(path, |name| std::env::var_os(name))
}

/// [`cloud_service`] with the environment given (tests).
pub fn cloud_service_with(
    path: &Path,
    env: impl Fn(&str) -> Option<OsString>,
) -> Option<CloudService> {
    let lower = |p: &Path| p.to_string_lossy().replace('\\', "/").to_lowercase();
    let here = lower(path);
    for var in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        if let Some(root) = env(var).filter(|r| !r.is_empty()) {
            let root = lower(Path::new(&root));
            let root = root.trim_end_matches('/');
            if !root.is_empty() && (here == root || here.starts_with(&format!("{root}/"))) {
                return Some(CloudService::OneDrive);
            }
        }
    }
    let mut dir = PathBuf::new();
    let mut previous = String::new();
    for part in path.components() {
        dir.push(part);
        let Component::Normal(name) = part else {
            continue;
        };
        let name = name.to_string_lossy().to_lowercase();
        let named = if name == "onedrive" || name.starts_with("onedrive - ") {
            Some(CloudService::OneDrive)
        } else if name == "dropbox" || (name.starts_with("dropbox (") && name.ends_with(')')) {
            Some(CloudService::Dropbox)
        } else if name == "google drive" || name == "my drive" {
            Some(CloudService::GoogleDrive)
        } else if name == "iclouddrive" || name == "icloud drive" || name == "com~apple~clouddocs" {
            Some(CloudService::ICloudDrive)
        } else if previous == "cloudstorage" {
            // macOS File Provider roots: Library/CloudStorage/OneDrive-Personal, GoogleDrive-me@x, ...
            [
                ("onedrive", CloudService::OneDrive),
                ("dropbox", CloudService::Dropbox),
                ("googledrive", CloudService::GoogleDrive),
            ]
            .into_iter()
            .find(|(prefix, _)| name.starts_with(prefix))
            .map(|(_, s)| s)
        } else if previous == "library" && name == "mobile documents" {
            Some(CloudService::ICloudDrive)
        } else {
            None
        };
        if named.is_some() {
            return named;
        }
        if dir.join(".dropbox").exists() || dir.join(".dropbox.cache").exists() {
            return Some(CloudService::Dropbox);
        }
        previous = name;
    }
    None
}

/// A projects folder on this computer that no service syncs: `<home>/OpenFrame Projects`, offered
/// when the default (inside Documents) is synced. `None` when the home folder is unknown or synced.
pub fn local_projects_dir() -> Option<PathBuf> {
    let home = directories::BaseDirs::new()?.home_dir().to_path_buf();
    let dir = home.join("OpenFrame Projects");
    cloud_service(&dir).is_none().then_some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_ENV: fn(&str) -> Option<OsString> = |_| None;

    #[test]
    fn documents_redirected_to_onedrive_is_found_through_the_environment() {
        let root = tempfile::tempdir().unwrap();
        let onedrive = root.path().join("Personal Cloud");
        let projects = onedrive
            .join("Documents")
            .join("OpenFrame")
            .join("Projects");
        let env = |name: &str| (name == "OneDrive").then(|| onedrive.clone().into_os_string());
        assert_eq!(
            cloud_service_with(&projects, env),
            Some(CloudService::OneDrive)
        );
        // A sibling folder that only starts with the same letters is not inside it.
        let beside = root.path().join("Personal Cloud Backup").join("x");
        assert_eq!(cloud_service_with(&beside, env), None);
    }

    #[test]
    fn a_fake_onedrive_root_and_the_other_services_are_found_by_name() {
        let root = tempfile::tempdir().unwrap();
        let cases = [
            ("OneDrive/Documents/OpenFrame", Some(CloudService::OneDrive)),
            (
                "OneDrive - Contoso Films/Scripts",
                Some(CloudService::OneDrive),
            ),
            ("Dropbox/Writing", Some(CloudService::Dropbox)),
            ("Dropbox (Studio)/Writing", Some(CloudService::Dropbox)),
            (
                "Google Drive/My Drive/Films",
                Some(CloudService::GoogleDrive),
            ),
            ("iCloudDrive/Films", Some(CloudService::ICloudDrive)),
            (
                "Library/CloudStorage/OneDrive-Personal/Films",
                Some(CloudService::OneDrive),
            ),
            (
                "Library/CloudStorage/GoogleDrive-me@example.com/Films",
                Some(CloudService::GoogleDrive),
            ),
            (
                "Library/Mobile Documents/com~apple~CloudDocs/Films",
                Some(CloudService::ICloudDrive),
            ),
            ("Documents/OpenFrame/Projects", None),
            ("OneDriveNotes/Films", None),
        ];
        for (rel, want) in cases {
            assert_eq!(
                cloud_service_with(&root.path().join(rel), NO_ENV),
                want,
                "{rel}"
            );
        }
    }

    #[test]
    fn a_dropbox_root_with_another_name_is_found_by_its_marker() {
        let root = tempfile::tempdir().unwrap();
        let sync_root = root.path().join("Team Files");
        std::fs::create_dir_all(sync_root.join("Films")).unwrap();
        std::fs::write(sync_root.join(".dropbox"), b"{}").unwrap();
        assert_eq!(
            cloud_service_with(&sync_root.join("Films"), NO_ENV),
            Some(CloudService::Dropbox)
        );
    }

    #[test]
    fn the_local_alternative_is_not_synced() {
        if let Some(dir) = local_projects_dir() {
            assert!(dir.ends_with("OpenFrame Projects"));
            assert_eq!(cloud_service(&dir), None);
        }
    }
}

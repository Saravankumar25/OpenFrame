//! Application folders. Windows: `%LOCALAPPDATA%\OpenFrame` and `Documents\OpenFrame`.
//! Every function returns `None` when the operating system cannot name the folder. There is no
//! fallback to a temp folder: settings, the trial record and projects must never land somewhere
//! the system cleans up. Callers decide what to do (refuse to start, or ask the user).
//! macOS: `~/Library/Application Support/OpenFrame` and `~/Documents/OpenFrame`. Inside the Mac
//! App Sandbox the home folder is the container, so the same calls land in the container; the
//! security-scoped access to user-picked project folders is handled by the desktop shell.

use std::path::PathBuf;

const APP_FOLDER: &str = "OpenFrame";

/// Application-private data: settings, recents, models, logs. Never user content.
pub fn app_data_dir() -> Option<PathBuf> {
    base_data_dir().map(|b| b.join(APP_FOLDER))
}

/// The user's Documents folder, where projects and the Global Idea Vault live by default.
pub fn documents_dir() -> Option<PathBuf> {
    directories::UserDirs::new().and_then(|u| u.document_dir().map(|d| d.to_path_buf()))
}

/// `Documents/OpenFrame`: the root for user-visible OpenFrame content.
pub fn user_content_dir() -> Option<PathBuf> {
    documents_dir().map(|d| d.join(APP_FOLDER))
}

/// The operating system's per-user local data folder without the OpenFrame suffix
/// (`%LOCALAPPDATA%` on Windows, `~/Library/Application Support` on macOS). The desktop shell uses
/// it to find the package-private data folder of an MSIX install.
pub fn base_data_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.data_local_dir().to_path_buf())
}

/// The read-only folder holding the files installed with the app (the Tauri bundle resources),
/// for example the bundled Offline AI (ADR-0025).
///
/// - Windows (installer, MSIX package and development builds): the folder of the executable. MSIX
///   installs it under `WindowsApps`, which is read-only for the app.
/// - macOS: `OpenFrame Studio.app/Contents/Resources` when running from an app bundle (read-only
///   and covered by the code signature, also inside the App Sandbox); the executable's folder for
///   an unbundled development build. Needs a Mac to verify.
///
/// `None` when the operating system cannot name the running executable.
pub fn app_resources_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    resources_dir_for(&exe)
}

/// [`app_resources_dir`] for a given executable path (separate for tests).
pub fn resources_dir_for(exe: &std::path::Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    if cfg!(target_os = "macos")
        && dir.file_name().is_some_and(|n| n == "MacOS")
        && let Some(contents) = dir
            .parent()
            .filter(|c| c.file_name().is_some_and(|n| n == "Contents"))
    {
        return Some(contents.join("Resources"));
    }
    Some(dir.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resources_sit_next_to_the_executable_outside_a_mac_app_bundle() {
        let exe = std::path::Path::new("/opt/of/openframe-desktop");
        assert_eq!(
            resources_dir_for(exe).unwrap(),
            std::path::Path::new("/opt/of")
        );
        assert!(app_resources_dir().is_some_and(|d| d.is_absolute()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mac_app_bundle_resources_are_in_contents_resources() {
        let exe = std::path::Path::new(
            "/Applications/OpenFrame Studio.app/Contents/MacOS/openframe-desktop",
        );
        assert_eq!(
            resources_dir_for(exe).unwrap(),
            std::path::Path::new("/Applications/OpenFrame Studio.app/Contents/Resources")
        );
    }

    #[test]
    fn folders_end_with_openframe_and_are_absolute() {
        let data = app_data_dir().expect("a home folder exists on the test machine");
        assert!(data.is_absolute());
        assert_eq!(data.file_name().unwrap(), APP_FOLDER);
        let content = user_content_dir().expect("a Documents folder exists on the test machine");
        assert!(content.is_absolute());
        assert_eq!(content.file_name().unwrap(), APP_FOLDER);
    }

    #[cfg(windows)]
    #[test]
    fn windows_data_folder_is_under_local_app_data() {
        let local = std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA is set on Windows");
        assert!(app_data_dir().unwrap().starts_with(local));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mac_data_folder_is_under_application_support() {
        let s = app_data_dir().unwrap().to_string_lossy().into_owned();
        assert!(s.contains("Library/Application Support"), "{s}");
    }
}

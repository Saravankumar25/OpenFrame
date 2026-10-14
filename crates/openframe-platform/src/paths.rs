//! Long paths. A project in a deep folder must open on both systems.
//!
//! Windows adapter: SQLite's default VFS refuses paths over 260 characters ("unable to open
//! database file"). Such a path is opened through SQLite's `win32-longpath` VFS with the absolute
//! extended-length form (`\\?\C:\...`, `\\?\UNC\...`). Short paths are passed as they are, so
//! error details and behaviour stay the same for everyone else. macOS adapter: paths up to
//! `PATH_MAX` (1024 bytes) open normally, so no other form is needed.

use std::path::{Path, PathBuf};

/// Paths this long (UTF-16 units) go through the long-path VFS. The `-wal`, `-shm` and
/// `-journal` siblings add up to 8 characters, so the switch happens well below `MAX_PATH`.
pub const LONG_PATH_THRESHOLD: usize = 240;

/// How SQLite must open `path`: `Some((path to pass, VFS name))` when the default way fails for
/// long paths on this system, `None` to open it as it is.
pub fn sqlite_long_path(path: &Path) -> Option<(PathBuf, &'static str)> {
    imp::sqlite_long_path(path)
}

/// The absolute extended-length (`\\?\`) form of `path`, or `None` when it cannot be formed.
/// Pure text work, the same on every system (used by the Windows adapter and its tests).
pub fn extended_length(path: &Path) -> Option<PathBuf> {
    let s = path.to_str()?;
    if s.starts_with(r"\\?\") {
        return Some(path.to_path_buf());
    }
    let abs = std::path::absolute(path).ok()?;
    let abs = abs.to_str()?;
    if abs.starts_with(r"\\?\") {
        Some(abs.into())
    } else if let Some(unc) = abs.strip_prefix(r"\\") {
        Some(format!(r"\\?\UNC\{unc}").into())
    } else {
        Some(format!(r"\\?\{abs}").into())
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    pub fn sqlite_long_path(path: &Path) -> Option<(PathBuf, &'static str)> {
        if path.as_os_str().encode_wide().count() >= super::LONG_PATH_THRESHOLD {
            super::extended_length(path).map(|p| (p, "win32-longpath"))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::{Path, PathBuf};

    pub fn sqlite_long_path(_path: &Path) -> Option<(PathBuf, &'static str)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_paths_open_as_they_are() {
        assert_eq!(sqlite_long_path(Path::new("project.sqlite")), None);
    }

    #[cfg(windows)]
    #[test]
    fn long_windows_paths_use_the_extended_form_and_the_long_path_vfs() {
        let deep = format!(r"C:\Films\{}\project.sqlite", "x".repeat(260));
        let (p, vfs) = sqlite_long_path(Path::new(&deep)).expect("long path");
        assert_eq!(vfs, "win32-longpath");
        assert_eq!(p, PathBuf::from(format!(r"\\?\{deep}")));
        let unc = format!(r"\\nas\share\{}\project.sqlite", "y".repeat(260));
        let (p, _) = sqlite_long_path(Path::new(&unc)).unwrap();
        assert!(p.to_string_lossy().starts_with(r"\\?\UNC\nas\share\"));
        let already = format!(r"\\?\C:\{}", "z".repeat(260));
        assert_eq!(
            sqlite_long_path(Path::new(&already)).unwrap().0,
            PathBuf::from(&already)
        );
    }
}

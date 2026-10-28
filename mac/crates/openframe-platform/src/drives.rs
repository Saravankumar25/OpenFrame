//! Drive classification for the network-location refusal (security standard: projects never live
//! on a network share, because SQLite's write-ahead log is unsafe there).
//!
//! Windows adapter: drive letters exist, and `GetDriveTypeW` reports a mapped network drive as
//! `DRIVE_REMOTE`. The call reads the local drive-letter mapping only; it does not open the path
//! or connect to a share. macOS adapter: there are no drive letters; every path starts at `/`.
//! A share is mounted somewhere in the tree (usually under `/Volumes`), so [`is_network_volume`]
//! asks `statfs` for the volume that holds the path's nearest existing folder and counts it as a
//! network volume unless the system marks it local (`MNT_LOCAL`). SMB, AFP, NFS and WebDAV mounts
//! do not carry that flag.

/// What the operating system reports for the volume behind a drive letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveKind {
    /// A mapped network drive (`net use Z: \\server\share`).
    Remote,
    /// Fixed, removable, optical, RAM disk, unknown or not present.
    Other,
}

/// Whether absolute paths on this system start with a drive letter or a UNC prefix (Windows), so
/// a path without one is malformed.
pub const DRIVE_LETTERS: bool = cfg!(windows);

/// Classify a drive letter. Always [`DriveKind::Other`] where drive letters do not exist.
pub fn drive_kind(letter: u8) -> DriveKind {
    imp::drive_kind(letter)
}

/// True when `path` lies on a network volume. macOS only: the nearest existing folder of `path`
/// (the path itself when it exists) is looked up with `statfs`, which follows links, so a local
/// link into a share counts as the share. Nothing is opened. `false` where drive letters decide
/// instead (Windows) and when the system cannot say.
pub fn is_network_volume(path: &std::path::Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        let Some(existing) = path
            .ancestors()
            .find(|a| std::fs::symlink_metadata(a).is_ok())
        else {
            return false;
        };
        mount_flags(existing).is_some_and(mount_flags_are_remote)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        false
    }
}

/// A volume is remote unless its mount flags carry `MNT_LOCAL`.
#[cfg(target_os = "macos")]
pub fn mount_flags_are_remote(flags: u32) -> bool {
    flags & libc::MNT_LOCAL as u32 == 0
}

#[cfg(target_os = "macos")]
fn mount_flags(path: &std::path::Path) -> Option<u32> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c` is a valid NUL-terminated path and `st` a valid out structure for the call.
    (unsafe { libc::statfs(c.as_ptr(), &mut st) } == 0).then_some(st.f_flags)
}

#[cfg(windows)]
mod imp {
    use super::DriveKind;

    pub fn drive_kind(letter: u8) -> DriveKind {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetDriveTypeW(root_path_name: *const u16) -> u32;
        }
        /// `DRIVE_REMOTE` from `winbase.h`.
        const DRIVE_REMOTE: u32 = 4;
        if !letter.is_ascii_alphabetic() {
            return DriveKind::Other;
        }
        let root: [u16; 4] = [u16::from(letter), u16::from(b':'), u16::from(b'\\'), 0];
        // SAFETY: `root` is a NUL-terminated UTF-16 string ("X:\") that outlives the call;
        // GetDriveTypeW only reads it and keeps no pointer to it.
        let kind = unsafe { GetDriveTypeW(root.as_ptr()) };
        if kind == DRIVE_REMOTE {
            DriveKind::Remote
        } else {
            DriveKind::Other
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::DriveKind;

    pub fn drive_kind(_letter: u8) -> DriveKind {
        DriveKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_letters_are_never_remote() {
        for b in [b'1', b':', b'\\', 0u8, 0xFF] {
            assert_eq!(drive_kind(b), DriveKind::Other);
        }
    }

    #[cfg(windows)]
    #[test]
    fn the_system_drive_is_local() {
        let sys = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        assert_eq!(drive_kind(sys.as_bytes()[0]), DriveKind::Other);
        const { assert!(DRIVE_LETTERS) };
    }
}

/// Bytes free for this user on the volume that holds `path` (the folder must exist), or `None`
/// when the system cannot say. Used by the AI download preflight (ADR-0026 decision 5).
pub fn free_bytes(path: &std::path::Path) -> Option<u64> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        use windows::core::PCWSTR;
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut free = 0u64;
        // SAFETY: `wide` is a NUL-terminated path that outlives the call; `free` is a valid out pointer.
        unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut free), None, None) }.ok()?;
        Some(free)
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;
        let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
        let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
        // SAFETY: `c` is a valid C string and `st` a valid out structure for the call.
        (unsafe { libc::statvfs(c.as_ptr(), &mut st) } == 0)
            .then(|| st.f_bavail as u64 * st.f_frsize as u64)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = path;
        None
    }
}

#[cfg(all(test, target_os = "macos"))]
mod network_volume_tests {
    use super::*;

    #[test]
    fn a_local_temp_folder_and_a_missing_folder_inside_it_are_not_network_volumes() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_network_volume(dir.path()));
        assert!(!is_network_volume(&dir.path().join("not yet").join("made")));
        assert!(!is_network_volume(std::path::Path::new("/")));
    }

    #[test]
    fn a_volume_without_the_local_flag_is_remote() {
        let local = libc::MNT_LOCAL as u32;
        assert!(!mount_flags_are_remote(local));
        assert!(!mount_flags_are_remote(local | libc::MNT_NOSUID as u32));
        // An SMB, AFP or NFS mount: no MNT_LOCAL, whatever else is set.
        assert!(mount_flags_are_remote(0));
        assert!(mount_flags_are_remote(
            libc::MNT_NOSUID as u32 | libc::MNT_NODEV as u32
        ));
    }
}

#[cfg(test)]
mod free_space_tests {
    #[test]
    fn the_temp_folder_reports_free_space() {
        let free = super::free_bytes(&std::env::temp_dir());
        if cfg!(any(windows, target_os = "macos")) {
            assert!(free.is_some_and(|b| b > 0));
        }
        let missing = std::path::Path::new(r"Z:\no\such\folder");
        assert!(super::free_bytes(missing).is_none() || cfg!(not(windows)));
    }
}

//! Drive classification for the network-location refusal (security standard: projects never live
//! on a network share, because SQLite's write-ahead log is unsafe there).
//!
//! Windows adapter: drive letters exist, and `GetDriveTypeW` reports a mapped network drive as
//! `DRIVE_REMOTE`. The call reads the local drive-letter mapping only; it does not open the path
//! or connect to a share. macOS adapter: there are no drive letters; every path starts at `/`.
//! Network mounts under `/Volumes` are not classified yet (unverified without Mac hardware; see
//! `docs/engineering/37-platform-boundary.md` section 6).

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

//! Opening a file so that nobody can change it while it is open.
//!
//! Windows adapter: the file is opened with `FILE_SHARE_READ` only, so another program can read it
//! but cannot write, rename or delete it until the handle is closed. macOS adapter: files carry no
//! share mode; the file is opened for reading and later checks (size and modification time) catch
//! a change.

use std::fs::File;
use std::io;
use std::path::Path;

/// Open `path` for reading while other programs may only read it (Windows), or for reading (macOS).
pub fn open_read_only_shared(path: &Path) -> io::Result<File> {
    let mut o = std::fs::OpenOptions::new();
    o.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        o.share_mode(1); // FILE_SHARE_READ
    }
    o.open(path)
}

/// True when `path` is a regular file (a link is not followed) that its owner may write.
/// Always false where file modes do not exist (Windows).
pub fn is_owner_writable_file(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::symlink_metadata(path)
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o200 != 0)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

/// Add the owner's write permission to the regular file at `path`. The file is opened without
/// following a link (`O_NOFOLLOW`) and changed through that descriptor (`fchmod`), so a link put
/// in its place between a check and the change can never redirect the change to another file.
/// Anything that is not a regular file is left alone. A no-op where file modes do not exist.
pub fn add_owner_write_no_follow(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)?;
        let meta = file.metadata()?;
        let mode = meta.permissions().mode();
        if meta.is_file() && mode & 0o200 == 0 {
            file.set_permissions(std::fs::Permissions::from_mode(mode | 0o200))?;
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_read_and_on_windows_cannot_be_written_while_held() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("m.gguf");
        std::fs::write(&p, b"model").unwrap();
        let held = open_read_only_shared(&p).unwrap();
        assert_eq!(
            std::fs::read(&p).unwrap(),
            b"model",
            "others may still read it"
        );
        if cfg!(windows) {
            assert!(std::fs::OpenOptions::new().write(true).open(&p).is_err());
        }
        drop(held);
        std::fs::write(&p, b"other").unwrap();
    }

    /// A side file that is a link is never followed: the file it points to keeps its permissions.
    /// Only a regular file gains the owner's write permission.
    #[cfg(unix)]
    #[test]
    fn permissions_change_only_on_the_regular_file_itself() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        let target = dir.path().join("elsewhere.txt");
        std::fs::write(&target, b"not ours").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444)).unwrap();
        let link = dir.path().join("project.sqlite-wal");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let _ = add_owner_write_no_follow(&link);
        assert_eq!(mode(&target), 0o444, "the link was followed");

        let shm = dir.path().join("project.sqlite-shm");
        std::fs::write(&shm, b"").unwrap();
        std::fs::set_permissions(&shm, std::fs::Permissions::from_mode(0o444)).unwrap();
        add_owner_write_no_follow(&shm).unwrap();
        assert_eq!(mode(&shm), 0o644);

        let fifo = dir.path().join("fifo-wal");
        let c = std::ffi::CString::new(fifo.as_os_str().to_str().unwrap()).unwrap();
        // SAFETY: a valid NUL-terminated path for mkfifo.
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o444) }, 0);
        let _ = add_owner_write_no_follow(&fifo);
        assert_eq!(mode(&fifo), 0o444, "a FIFO is not a regular file");
    }
}

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
}

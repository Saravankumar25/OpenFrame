//! Fast "is this a regular file" check for list views with hundreds of assets.
//!
//! On Windows `std::fs::metadata` opens a handle, which wakes on-access antivirus scanning and
//! costs about 0.1 ms per file. `GetFileAttributesW` reads the directory entry only. Links and
//! very long paths and relative paths fall back to the std check so the answer stays correct.

use std::path::Path;

/// `Path::is_file`, answered with one attribute query where the platform allows it.
pub fn file_exists(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetFileAttributesW(file_name: *const u16) -> u32;
        }
        const INVALID_FILE_ATTRIBUTES: u32 = u32::MAX;
        const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        let os = path.as_os_str();
        if path.is_absolute() && os.len() < 240 {
            let wide: Vec<u16> = os.encode_wide().chain(std::iter::once(0)).collect();
            if !wide[..wide.len() - 1].contains(&0) {
                // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives the call.
                let attrs = unsafe { GetFileAttributesW(wide.as_ptr()) };
                if attrs == INVALID_FILE_ATTRIBUTES {
                    return false;
                }
                if attrs & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
                    return attrs & FILE_ATTRIBUTE_DIRECTORY == 0;
                }
            }
        }
    }
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_folders_and_missing_paths() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.png");
        std::fs::write(&f, b"x").unwrap();
        assert!(file_exists(&f));
        assert!(!file_exists(dir.path()));
        assert!(!file_exists(&dir.path().join("missing.png")));
    }

    #[test]
    fn long_paths_use_the_fallback_and_stay_correct() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = dir.path().to_path_buf();
        for _ in 0..6 {
            p.push("a folder with a long name for the long path test");
        }
        std::fs::create_dir_all(&p).unwrap();
        let f = p.join("deep.txt");
        std::fs::write(&f, b"x").unwrap();
        assert!(f.as_os_str().len() > 240);
        assert!(file_exists(&f));
        assert!(!file_exists(&p.join("nope.txt")));
    }
}

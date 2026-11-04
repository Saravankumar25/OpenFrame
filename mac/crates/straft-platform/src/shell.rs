//! Open a file with its default program, show it in the file manager, or open a web link in the
//! browser. Callers decide what may be opened (the desktop shell resolves paths from ids, refuses
//! network locations and shows dangerous file types instead of opening them); this module only
//! asks the operating system.
//!
//! Windows adapter: `ShellExecuteW` ("open") for files and links, and
//! `SHOpenFolderAndSelectItems` for "show in File Explorer", which selects the item without
//! starting a program by name. macOS adapter: `/usr/bin/open` and `/usr/bin/open -R` (compiles,
//! verified only on Mac hardware; inside the App Sandbox it works for paths the user granted).

use std::io;
use std::path::Path;

/// Open `path` with the program the user chose for its type.
pub fn open_path(path: &Path) -> io::Result<()> {
    check(path)?;
    imp::open(path.as_os_str())
}

/// Show `path` selected in the file manager (File Explorer or Finder).
pub fn reveal_path(path: &Path) -> io::Result<()> {
    check(path)?;
    imp::reveal(path)
}

/// Open a web link in the default browser. Only `http://` and `https://` links are accepted; the
/// caller validates the link further.
pub fn open_url(url: &str) -> io::Result<()> {
    let lower = url.trim_start().to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) || url.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "only web links can be opened",
        ));
    }
    imp::open(std::ffi::OsStr::new(url))
}

/// Only absolute paths without NUL characters reach the operating system.
fn check(path: &Path) -> io::Result<()> {
    if !path.is_absolute() || path.to_string_lossy().contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "an absolute path is needed",
        ));
    }
    Ok(())
}

#[cfg(windows)]
mod imp {
    use std::ffi::{OsStr, c_void};
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteW(
            hwnd: *mut c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
        fn SHParseDisplayName(
            name: *const u16,
            bind_ctx: *mut c_void,
            pidl: *mut *mut c_void,
            sfgao_in: u32,
            sfgao_out: *mut u32,
        ) -> i32;
        fn SHOpenFolderAndSelectItems(
            folder: *const c_void,
            count: u32,
            items: *const *const c_void,
            flags: u32,
        ) -> i32;
        fn ILFree(pidl: *mut c_void);
    }
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> i32;
        fn CoUninitialize();
    }

    const SW_SHOWNORMAL: i32 = 1;
    const COINIT_APARTMENTTHREADED: u32 = 0x2;

    fn wide(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn open(target: &OsStr) -> io::Result<()> {
        let verb = wide(OsStr::new("open"));
        let file = wide(target);
        // SAFETY: both strings are NUL-terminated UTF-16 buffers that outlive the call.
        let rc = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        // Values above 32 mean success.
        if rc > 32 {
            Ok(())
        } else {
            Err(io::Error::other(format!("ShellExecuteW returned {rc}")))
        }
    }

    pub fn reveal(path: &Path) -> io::Result<()> {
        let name = wide(path.as_os_str());
        // SAFETY: COM is initialised for this call only (balanced below unless another mode was
        // already set); the item id list is freed after use.
        unsafe {
            let co = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
            let mut pidl: *mut c_void = std::ptr::null_mut();
            let mut out = 0u32;
            let hr =
                SHParseDisplayName(name.as_ptr(), std::ptr::null_mut(), &mut pidl, 0, &mut out);
            let result = if hr < 0 || pidl.is_null() {
                Err(io::Error::other(format!(
                    "SHParseDisplayName failed {hr:#x}"
                )))
            } else {
                let hr = SHOpenFolderAndSelectItems(pidl, 0, std::ptr::null(), 0);
                ILFree(pidl);
                if hr < 0 {
                    Err(io::Error::other(format!(
                        "SHOpenFolderAndSelectItems failed {hr:#x}"
                    )))
                } else {
                    Ok(())
                }
            };
            // S_OK (0) or S_FALSE (1) must be balanced; RPC_E_CHANGED_MODE must not.
            if co == 0 || co == 1 {
                CoUninitialize();
            }
            result
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::OsStr;
    use std::io;
    use std::path::Path;
    use std::process::Command;

    fn run(args: &[&OsStr]) -> io::Result<()> {
        let status = Command::new("/usr/bin/open").args(args).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("open exited with {status}")))
        }
    }

    pub fn open(target: &OsStr) -> io::Result<()> {
        run(&[OsStr::new("--"), target])
    }

    pub fn reveal(path: &Path) -> io::Result<()> {
        run(&[OsStr::new("-R"), OsStr::new("--"), path.as_os_str()])
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    use std::ffi::OsStr;
    use std::io;
    use std::path::Path;

    pub fn open(_target: &OsStr) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    pub fn reveal(_path: &Path) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_and_nul_characters_never_reach_the_system() {
        for bad in ["relative.png", "a\0b"] {
            assert_eq!(
                open_path(Path::new(bad)).unwrap_err().kind(),
                io::ErrorKind::InvalidInput,
                "{bad:?}"
            );
            assert_eq!(
                reveal_path(Path::new(bad)).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }

    #[test]
    fn only_web_links_are_opened_in_the_browser() {
        for bad in [
            "file:///C:/Windows/System32/calc.exe",
            r"C:\Windows\System32\calc.exe",
            "javascript:alert(1)",
            "ms-settings:privacy",
            "https://example.com/\0x",
            "",
        ] {
            assert_eq!(
                open_url(bad).unwrap_err().kind(),
                io::ErrorKind::InvalidInput,
                "{bad:?}"
            );
        }
    }
}

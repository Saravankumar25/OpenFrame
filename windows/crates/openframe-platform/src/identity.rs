//! How this process was installed, as far as the operating system can tell.
//!
//! Windows: the MSIX package family name when the process runs from a Store package, otherwise
//! `None`. macOS: the bundle identifier is read by the desktop shell from the bundle; the App
//! Store receipt check belongs to the entitlement adapter, not here.

/// The MSIX package family name of this process, or `None` when it runs unpackaged or not on
/// Windows.
pub fn package_family_name() -> Option<String> {
    imp::package_family_name()
}

#[cfg(windows)]
mod imp {
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_INSUFFICIENT_BUFFER: i32 = 122;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        /// Windows 8 and later. Returns APPMODEL_ERROR_NO_PACKAGE (15700) for unpackaged processes.
        fn GetCurrentPackageFamilyName(length: *mut u32, name: *mut u16) -> i32;
    }

    pub fn package_family_name() -> Option<String> {
        let mut len: u32 = 128;
        let mut buf: Vec<u16> = vec![0; len as usize];
        for _ in 0..2 {
            // SAFETY: `buf` holds `len` u16s; the API writes at most `len` units including the
            // terminator and updates `len` to the required size.
            let rc = unsafe { GetCurrentPackageFamilyName(&mut len, buf.as_mut_ptr()) };
            match rc {
                ERROR_SUCCESS => {
                    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                    let name = String::from_utf16_lossy(&buf[..end]);
                    return (!name.is_empty()).then_some(name);
                }
                ERROR_INSUFFICIENT_BUFFER => buf = vec![0; len as usize],
                _ => return None,
            }
        }
        None
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn package_family_name() -> Option<String> {
        None
    }
}

/// Validate a package family name before building a path from it: `<name>_<publisher id>` where
/// the name is 3 to 50 ASCII letters, digits, `.` or `-`, neither starting nor ending with a dot,
/// and the publisher id is exactly 13 lower-case base32 characters (as Windows generates it).
pub fn is_valid_package_family_name(family: &str) -> bool {
    let Some((name, publisher)) = family.rsplit_once('_') else {
        return false;
    };
    let name_ok = (3..=50).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
        && !name.starts_with('.')
        && !name.ends_with('.')
        && !name.contains("..");
    let publisher_ok = publisher.len() == 13
        && publisher
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    name_ok && publisher_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpackaged_test_process_has_no_family_name() {
        // `cargo test` never runs from an MSIX package.
        assert_eq!(package_family_name(), None);
    }

    #[test]
    fn family_name_validation() {
        assert!(is_valid_package_family_name(
            "OpenFrame.Studio_8wekyb3d8bbwe"
        ));
        assert!(!is_valid_package_family_name(""));
        assert!(!is_valid_package_family_name("NoUnderscore"));
        assert!(!is_valid_package_family_name("..\\..\\escape_x"));
        assert!(!is_valid_package_family_name(".hidden_x"));
        assert!(!is_valid_package_family_name("has space_x"));
        assert!(!is_valid_package_family_name("Trailing._8wekyb3d8bbwe"));
        assert!(!is_valid_package_family_name("OpenFrame.Studio_short"));
        assert!(!is_valid_package_family_name(
            "OpenFrame.Studio_8WEKYB3D8BBWE"
        ));
        assert!(!is_valid_package_family_name("a_8wekyb3d8bbwe"));
        // Package names never contain an underscore; only the publisher separator does.
        assert!(!is_valid_package_family_name("Name_x_8wekyb3d8bbwe"));
    }
}

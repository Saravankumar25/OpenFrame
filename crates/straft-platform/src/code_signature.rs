//! Code signature check for program files (store build, macOS).
//!
//! A Mac App Store build signs the engine program and each library, and Apple signs them again at
//! distribution, so their bytes no longer match the hashes pinned in the lock. The engine check
//! then asks this module instead: does the file carry a valid signature that chains to an Apple
//! root? Developer ID and Mac App Store certificates both do. A planted or edited file does not.
//! On any other system the answer is always no, because only the Mac store build asks.

use std::path::Path;

/// `true` when `path` is a signed program or library whose signature is intact (strict check) and
/// chains to Apple ("anchor apple generic"). A missing file, an unsigned file, a file changed
/// after signing and any error all give `false`.
pub fn is_apple_signed(path: &Path) -> bool {
    imp::is_apple_signed(path)
}

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::{CString, c_char, c_void};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    type CFTypeRef = *const c_void;
    type OSStatus = i32;

    /// `kSecCSDefaultFlags`.
    const DEFAULT_FLAGS: u32 = 0;
    /// `kSecCSStrictValidate`: also reject a signed file with extra or altered resources.
    const STRICT_VALIDATE: u32 = 1 << 4;
    const UTF8: u32 = 0x0800_0100;

    // SAFETY: these declarations match the system headers (CFURL.h, CFString.h, CFBase.h).
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFURLCreateFromFileSystemRepresentation(
            allocator: CFTypeRef,
            buffer: *const u8,
            len: isize,
            is_directory: u8,
        ) -> CFTypeRef;
        fn CFStringCreateWithCString(
            allocator: CFTypeRef,
            text: *const c_char,
            encoding: u32,
        ) -> CFTypeRef;
        fn CFRelease(object: CFTypeRef);
    }

    // SAFETY: these declarations match the system headers (SecStaticCode.h, SecRequirement.h).
    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        fn SecStaticCodeCreateWithPath(
            path: CFTypeRef,
            flags: u32,
            static_code: *mut CFTypeRef,
        ) -> OSStatus;
        fn SecRequirementCreateWithString(
            text: CFTypeRef,
            flags: u32,
            requirement: *mut CFTypeRef,
        ) -> OSStatus;
        fn SecStaticCodeCheckValidity(
            static_code: CFTypeRef,
            flags: u32,
            requirement: CFTypeRef,
        ) -> OSStatus;
    }

    /// Releases every Core Foundation object it holds, on every exit path.
    struct Owned(Vec<CFTypeRef>);
    impl Drop for Owned {
        fn drop(&mut self) {
            for object in self.0.drain(..).filter(|o| !o.is_null()) {
                // SAFETY: each object came from a Create function and is released once.
                unsafe { CFRelease(object) };
            }
        }
    }

    pub fn is_apple_signed(path: &Path) -> bool {
        // A missing path would also fail below; checking first keeps the answer plain.
        if !path.is_file() {
            return false;
        }
        let bytes = path.as_os_str().as_bytes();
        let Ok(len) = isize::try_from(bytes.len()) else {
            return false;
        };
        let Ok(text) = CString::new("anchor apple generic") else {
            return false;
        };
        let mut held = Owned(Vec::new());
        // SAFETY: the buffer and length describe `bytes`, which outlives the call; every created
        // object is pushed to `held` and released when it drops; the out pointers are valid.
        unsafe {
            let url =
                CFURLCreateFromFileSystemRepresentation(std::ptr::null(), bytes.as_ptr(), len, 0);
            held.0.push(url);
            if url.is_null() {
                return false;
            }
            let mut code: CFTypeRef = std::ptr::null();
            if SecStaticCodeCreateWithPath(url, DEFAULT_FLAGS, &mut code) != 0 {
                return false;
            }
            held.0.push(code);
            let requirement_text = CFStringCreateWithCString(std::ptr::null(), text.as_ptr(), UTF8);
            held.0.push(requirement_text);
            if requirement_text.is_null() {
                return false;
            }
            let mut requirement: CFTypeRef = std::ptr::null();
            if SecRequirementCreateWithString(requirement_text, DEFAULT_FLAGS, &mut requirement)
                != 0
            {
                return false;
            }
            held.0.push(requirement);
            SecStaticCodeCheckValidity(code, DEFAULT_FLAGS | STRICT_VALIDATE, requirement) == 0
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use std::path::Path;

    pub fn is_apple_signed(_path: &Path) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn a_system_program_is_signed_by_apple() {
        assert!(is_apple_signed(Path::new("/bin/ls")));
    }

    #[test]
    fn garbage_bytes_a_copy_with_garbage_and_a_missing_path_are_not_signed() {
        let tmp = tempfile::tempdir().unwrap();
        let junk = tmp.path().join("llama-server");
        std::fs::write(&junk, b"not a program").unwrap();
        assert!(!is_apple_signed(&junk));
        assert!(!is_apple_signed(&tmp.path().join("missing.dylib")));
        assert!(!is_apple_signed(tmp.path()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_signed_program_with_changed_bytes_is_not_signed() {
        let tmp = tempfile::tempdir().unwrap();
        let copy = tmp.path().join("ls");
        std::fs::copy("/bin/ls", &copy).unwrap();
        let mut bytes = std::fs::read(&copy).unwrap();
        let at = bytes.len() / 2;
        bytes[at] ^= 0xff;
        std::fs::write(&copy, bytes).unwrap();
        assert!(!is_apple_signed(&copy));
    }
}

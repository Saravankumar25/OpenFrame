//! Distribution channel and MSIX packaging context
//! (docs/engineering/27-microsoft-store.md §5, docs/engineering/24-windows-packaging-updater.md §3).
//!
//! OpenFrame ships through two channels:
//!
//! * **direct**: NSIS/MSI installers from the OpenFrame release host (the default build);
//! * **store**: an MSIX package that the Microsoft Store distributes, signs and updates.
//!
//! The channel is fixed at build time with `OPENFRAME_DISTRIBUTION=store|direct`. `build.rs`
//! also sets `cfg(openframe_store)`, so code that must never ship in Store builds (the in-app
//! updater) can be compiled out. Independently of the build flag, the process detects at runtime
//! whether it runs with MSIX package identity. A packaged process is always updated by Windows
//! (Store or App Installer) and never by an in-app updater: its install folder is read-only, and an
//! NSIS/MSI update would install a second, unpackaged copy next to it.

use std::path::{Path, PathBuf};

/// Where end users obtain the Evergreen WebView2 Runtime when it is missing (Microsoft's page).
pub const WEBVIEW2_DOWNLOAD_URL: &str =
    "https://developer.microsoft.com/microsoft-edge/webview2/consumer/";

/// Build-channel marker. It is logged at startup, so it stays in the executable, and
/// `scripts/build-msix.mjs --store` refuses to package an executable that doesn't contain the
/// `store` marker.
pub const CHANNEL_MARKER: &str = concat!(
    "openframe-distribution-channel:",
    env!("OPENFRAME_DISTRIBUTION")
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Direct,
    Store,
}

impl Channel {
    /// The channel this binary was built for (`OPENFRAME_DISTRIBUTION`, see `build.rs`).
    pub const fn current() -> Self {
        if cfg!(openframe_store) {
            Channel::Store
        } else {
            Channel::Direct
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Channel::Direct => "direct",
            Channel::Store => "store",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Distribution {
    pub channel: Channel,
    /// MSIX package family name when the process runs with package identity.
    pub package_family_name: Option<String>,
}

impl Distribution {
    pub fn detect() -> Self {
        Self {
            channel: Channel::current(),
            package_family_name: current_package_family_name(),
        }
    }

    pub fn is_packaged(&self) -> bool {
        self.package_family_name.is_some()
    }

    /// Whether an in-app updater may run at all. It may run only in direct builds that are not
    /// running from an MSIX package. Store builds are updated exclusively by the Microsoft Store.
    /// Any future updater plugin must be compiled under `#[cfg(not(openframe_store))]` and
    /// registered only when this returns `true`.
    pub fn in_app_updates_allowed(&self) -> bool {
        self.channel == Channel::Direct && !self.is_packaged()
    }

    /// Resolve the app-private data folder (settings, recents, logs, downloaded AI runtime and
    /// models).
    ///
    /// Unpackaged builds use `%LOCALAPPDATA%\OpenFrame`. A packaged process uses the package's own
    /// data folder (`%LOCALAPPDATA%\Packages\<family>\LocalState\OpenFrame`, the Win32 path of
    /// `ApplicationData.Current.LocalFolder`). The path is real and not virtualized, so Explorer
    /// ("Open logs folder") and child processes (the local AI runtime) see exactly the same files,
    /// and Windows removes the folder when the app is uninstalled. Without this, MSIX would silently
    /// redirect new files under `%LOCALAPPDATA%\OpenFrame` to a private location that other
    /// processes cannot see at the original path.
    ///
    /// Projects and the Global Idea Vault live in Documents and are unaffected: MSIX does not
    /// virtualize Documents.
    pub fn app_data_dir(&self, unpackaged_default: PathBuf) -> PathBuf {
        match (&self.package_family_name, local_app_data_dir()) {
            (Some(family), Some(local)) => {
                packaged_app_data_dir(&local, family).unwrap_or(unpackaged_default)
            }
            _ => unpackaged_default,
        }
    }
}

/// `<local_app_data>\Packages\<family>\LocalState\OpenFrame`, or `None` when `family` is not a
/// well-formed package family name (never build a path from unexpected input).
pub fn packaged_app_data_dir(local_app_data: &Path, family: &str) -> Option<PathBuf> {
    let valid = !family.is_empty()
        && family.len() <= 128
        && family
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && family.contains('_')
        && !family.starts_with('.');
    valid.then(|| {
        local_app_data
            .join("Packages")
            .join(family)
            .join("LocalState")
            .join("OpenFrame")
    })
}

fn local_app_data_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.data_local_dir().to_path_buf())
}

/// Check that the WebView2 Runtime is installed before any window is created. Windows 11 ships it
/// and nearly every Windows 10 device has it. MSIX packages cannot run the WebView2 bootstrapper
/// that the NSIS/MSI installers use. When it is missing, explain what to install and offer to
/// open Microsoft's download page instead of failing silently (release builds have no console).
pub fn ensure_webview2() -> bool {
    match tauri::webview_version() {
        Ok(version) => {
            tracing::info!(webview2 = %version, "WebView2 runtime available");
            true
        }
        Err(e) => {
            tracing::error!(error = %e, "WebView2 runtime not available");
            let open = ask_yes_no(
                "OpenFrame Studio",
                "OpenFrame Studio needs the Microsoft Edge WebView2 Runtime, which isn't installed on this PC.\n\n\
                 Open Microsoft's download page now? Install the \"Evergreen Bootstrapper\", then start OpenFrame Studio again.",
            );
            if open
                && let Err(e) = tauri_plugin_opener::open_url(WEBVIEW2_DOWNLOAD_URL, None::<&str>)
            {
                tracing::warn!(error = %e, "could not open the WebView2 download page");
            }
            false
        }
    }
}

#[cfg(windows)]
mod win {
    //! Minimal Win32 bindings (kernel32/user32 are always linked on Windows).

    pub const ERROR_SUCCESS: i32 = 0;
    pub const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
    pub const MB_YESNO: u32 = 0x0000_0004;
    pub const MB_ICONERROR: u32 = 0x0000_0010;
    pub const MB_SETFOREGROUND: u32 = 0x0001_0000;
    pub const IDYES: i32 = 6;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        /// Windows 8+. Returns APPMODEL_ERROR_NO_PACKAGE (15700) for unpackaged processes.
        pub fn GetCurrentPackageFamilyName(length: *mut u32, name: *mut u16) -> i32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        pub fn MessageBoxW(
            hwnd: *mut core::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }

    pub fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

/// The MSIX package family name of this process, or `None` when it runs unpackaged.
#[cfg(windows)]
pub fn current_package_family_name() -> Option<String> {
    let mut len: u32 = 128;
    let mut buf: Vec<u16> = vec![0; len as usize];
    for _ in 0..2 {
        // SAFETY: `buf` holds `len` u16s; the API writes at most `len` units (incl. NUL) and
        // updates `len` to the required size.
        let rc = unsafe { win::GetCurrentPackageFamilyName(&mut len, buf.as_mut_ptr()) };
        match rc {
            win::ERROR_SUCCESS => {
                let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                let name = String::from_utf16_lossy(&buf[..end]);
                return (!name.is_empty()).then_some(name);
            }
            win::ERROR_INSUFFICIENT_BUFFER => buf = vec![0; len as usize],
            _ => return None,
        }
    }
    None
}

#[cfg(not(windows))]
pub fn current_package_family_name() -> Option<String> {
    None
}

#[cfg(windows)]
fn ask_yes_no(title: &str, text: &str) -> bool {
    let (t, c) = (win::wide(text), win::wide(title));
    // SAFETY: both buffers are NUL-terminated UTF-16 and outlive the call; no owner window.
    let r = unsafe {
        win::MessageBoxW(
            std::ptr::null_mut(),
            t.as_ptr(),
            c.as_ptr(),
            win::MB_YESNO | win::MB_ICONERROR | win::MB_SETFOREGROUND,
        )
    };
    r == win::IDYES
}

#[cfg(not(windows))]
fn ask_yes_no(_title: &str, text: &str) -> bool {
    eprintln!("{text}");
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(channel: Channel, family: Option<&str>) -> Distribution {
        Distribution {
            channel,
            package_family_name: family.map(str::to_string),
        }
    }

    #[test]
    fn store_builds_never_allow_in_app_updates() {
        assert!(!dist(Channel::Store, None).in_app_updates_allowed());
        assert!(
            !dist(Channel::Store, Some("OpenFrame.Studio_8wekyb3d8bbwe")).in_app_updates_allowed()
        );
    }

    #[test]
    fn packaged_direct_builds_never_allow_in_app_updates() {
        // e.g. a sideloaded MSIX built without OPENFRAME_DISTRIBUTION=store
        assert!(
            !dist(Channel::Direct, Some("OpenFrame.Studio_8wekyb3d8bbwe")).in_app_updates_allowed()
        );
    }

    #[test]
    fn unpackaged_direct_builds_may_update_in_app() {
        assert!(dist(Channel::Direct, None).in_app_updates_allowed());
    }

    #[test]
    fn packaged_data_dir_is_the_package_local_state_folder() {
        let local = Path::new(r"C:\Users\A\AppData\Local");
        assert_eq!(
            packaged_app_data_dir(local, "OpenFrame.Studio_8wekyb3d8bbwe").unwrap(),
            local
                .join("Packages")
                .join("OpenFrame.Studio_8wekyb3d8bbwe")
                .join("LocalState")
                .join("OpenFrame")
        );
    }

    #[test]
    fn malformed_family_names_are_rejected() {
        let local = Path::new(r"C:\Users\A\AppData\Local");
        for bad in ["", "..", r"..\x_y", "a/b_c", "no-publisher-id", "a b_c"] {
            assert!(packaged_app_data_dir(local, bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn unpackaged_process_keeps_default_data_dir() {
        let default = PathBuf::from(r"C:\Users\A\AppData\Local\OpenFrame");
        assert_eq!(
            dist(Channel::Store, None).app_data_dir(default.clone()),
            default
        );
    }

    #[test]
    fn channel_matches_build_flag() {
        // build.rs exports the channel as a compile-time env var and as cfg(openframe_store).
        assert_eq!(env!("OPENFRAME_DISTRIBUTION"), Channel::current().as_str());
        assert!(CHANNEL_MARKER.ends_with(Channel::current().as_str()));
    }

    #[cfg(windows)]
    #[test]
    fn test_process_is_not_packaged() {
        assert_eq!(current_package_family_name(), None);
    }
}

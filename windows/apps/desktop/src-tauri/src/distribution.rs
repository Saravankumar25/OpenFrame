//! Distribution channel and MSIX packaging context
//! (docs/engineering/27-microsoft-store.md §5, docs/engineering/24-windows-packaging-updater.md §3).
//!
//! Straft ships through two channels:
//!
//! * **direct**: NSIS/MSI installers from the Straft release host (the default build);
//! * **store**: an MSIX package that the Microsoft Store distributes, signs and updates.
//!
//! The channel is fixed at build time with `STRAFT_DISTRIBUTION=store|direct`. `build.rs`
//! also sets `cfg(straft_store)`, so code that must never ship in Store builds (the in-app
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
pub const CHANNEL_MARKER: &str =
    concat!("straft-distribution-channel:", env!("STRAFT_DISTRIBUTION"));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Direct,
    Store,
}

impl Channel {
    /// The channel this binary was built for (`STRAFT_DISTRIBUTION`, see `build.rs`).
    pub const fn current() -> Self {
        if cfg!(straft_store) {
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
    /// Any future updater plugin must be compiled under `#[cfg(not(straft_store))]` and
    /// registered only when this returns `true`.
    pub fn in_app_updates_allowed(&self) -> bool {
        self.channel == Channel::Direct && !self.is_packaged()
    }

    /// Resolve the app-private data folder (settings, recents, logs, downloaded AI runtime and
    /// models).
    ///
    /// Unpackaged builds use `%LOCALAPPDATA%\Straft`. A packaged process uses the package's own
    /// data folder (`%LOCALAPPDATA%\Packages\<family>\LocalState\Straft`, the Win32 path of
    /// `ApplicationData.Current.LocalFolder`). The path is real and not virtualized, so Explorer
    /// ("Open logs folder") and child processes (the local AI runtime) see exactly the same files,
    /// and Windows removes the folder when the app is uninstalled. Without this, MSIX would silently
    /// redirect new files under `%LOCALAPPDATA%\Straft` to a private location that other
    /// processes cannot see at the original path.
    ///
    /// Projects and the Global Idea Vault live in Documents and are unaffected: MSIX does not
    /// virtualize Documents.
    pub fn app_data_dir(&self, unpackaged_default: PathBuf) -> PathBuf {
        match (
            &self.package_family_name,
            straft_platform::dirs::base_data_dir(),
        ) {
            (Some(family), Some(local)) => {
                packaged_app_data_dir(&local, family).unwrap_or(unpackaged_default)
            }
            _ => unpackaged_default,
        }
    }
}

/// `<local_app_data>\Packages\<family>\LocalState\Straft`, or `None` when `family` is not a
/// well-formed package family name (never build a path from unexpected input). The check is the
/// platform boundary's `is_valid_package_family_name`.
pub fn packaged_app_data_dir(local_app_data: &Path, family: &str) -> Option<PathBuf> {
    straft_platform::identity::is_valid_package_family_name(family).then(|| {
        local_app_data
            .join("Packages")
            .join(family)
            .join("LocalState")
            // "Straft": never change after the first release. The Store keeps people's settings,
            // logs and downloaded AI in this folder, and a new name would orphan them (package
            // file 05 U3, golden test below).
            .join("Straft")
    })
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
                "Straft Studio",
                "Straft Studio needs the Microsoft Edge WebView2 Runtime, which isn't installed on this PC.\n\n\
                 Open Microsoft's download page now? Install the \"Evergreen Bootstrapper\", then start Straft Studio again.",
            );
            if open && let Err(e) = straft_platform::shell::open_url(WEBVIEW2_DOWNLOAD_URL) {
                tracing::warn!(error = %e, "could not open the WebView2 download page");
            }
            false
        }
    }
}

#[cfg(windows)]
mod win {
    //! Minimal Win32 bindings (kernel32/user32 are always linked on Windows).

    pub const MB_OK: u32 = 0x0000_0000;
    pub const MB_YESNO: u32 = 0x0000_0004;
    pub const MB_ICONERROR: u32 = 0x0000_0010;
    pub const MB_SETFOREGROUND: u32 = 0x0001_0000;
    pub const IDYES: i32 = 6;

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

/// The MSIX package family name of this process, or `None` when it runs unpackaged (platform
/// boundary: `straft_platform::identity`).
pub fn current_package_family_name() -> Option<String> {
    straft_platform::identity::package_family_name()
}

/// A message box before any window exists (release builds have no console). Returns the button.
#[cfg(windows)]
fn message_box(title: &str, text: &str, buttons: u32) -> i32 {
    let (t, c) = (win::wide(text), win::wide(title));
    // SAFETY: both buffers are NUL-terminated UTF-16 and outlive the call; no owner window.
    unsafe {
        win::MessageBoxW(
            std::ptr::null_mut(),
            t.as_ptr(),
            c.as_ptr(),
            buttons | win::MB_ICONERROR | win::MB_SETFOREGROUND,
        )
    }
}

#[cfg(windows)]
fn ask_yes_no(title: &str, text: &str) -> bool {
    message_box(title, text, win::MB_YESNO) == win::IDYES
}

#[cfg(not(windows))]
fn ask_yes_no(_title: &str, text: &str) -> bool {
    eprintln!("{text}");
    false
}

/// Tell the user, in plain words, why Straft cannot start (no window can open yet).
pub fn show_startup_error(text: &str) {
    #[cfg(windows)]
    message_box("Straft Studio", text, win::MB_OK);
    #[cfg(not(windows))]
    eprintln!("{text}");
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
            !dist(Channel::Store, Some("Straft.Studio_8wekyb3d8bbwe")).in_app_updates_allowed()
        );
    }

    #[test]
    fn packaged_direct_builds_never_allow_in_app_updates() {
        // e.g. a sideloaded MSIX built without STRAFT_DISTRIBUTION=store
        assert!(
            !dist(Channel::Direct, Some("Straft.Studio_8wekyb3d8bbwe")).in_app_updates_allowed()
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
            packaged_app_data_dir(local, "Straft.Studio_8wekyb3d8bbwe").unwrap(),
            local
                .join("Packages")
                .join("Straft.Studio_8wekyb3d8bbwe")
                .join("LocalState")
                .join("Straft")
        );
    }

    /// Golden: the exact folder a packaged app keeps its data in. If this line has to change, the
    /// people who already installed lose their settings; do not edit it to make a build pass.
    #[cfg(windows)]
    #[test]
    fn packaged_data_dir_golden_path() {
        let dir = packaged_app_data_dir(Path::new(r"C:\L"), "Straft.Studio_8wekyb3d8bbwe");
        assert_eq!(
            dir.unwrap().to_str().unwrap(),
            r"C:\L\Packages\Straft.Studio_8wekyb3d8bbwe\LocalState\Straft"
        );
        // A name that is not a real family name (no 13 character publisher id) builds no path.
        assert!(packaged_app_data_dir(Path::new(r"C:\L"), "A.B_xyz").is_none());
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
        let default = PathBuf::from(r"C:\Users\A\AppData\Local\Straft");
        assert_eq!(
            dist(Channel::Store, None).app_data_dir(default.clone()),
            default
        );
    }

    #[test]
    fn channel_matches_build_flag() {
        // build.rs exports the channel as a compile-time env var and as cfg(straft_store).
        assert_eq!(env!("STRAFT_DISTRIBUTION"), Channel::current().as_str());
        assert!(CHANNEL_MARKER.ends_with(Channel::current().as_str()));
    }

    #[test]
    fn family_name_check_is_the_platform_rule() {
        let local = Path::new(r"C:\Users\A\AppData\Local");
        for name in [
            "Straft.Studio_8wekyb3d8bbwe",
            "",
            ".hidden_x",
            r"..\..\escape_x",
            "NoUnderscore",
            "a b_c",
        ] {
            assert_eq!(
                packaged_app_data_dir(local, name).is_some(),
                straft_platform::identity::is_valid_package_family_name(name),
                "{name}"
            );
        }
    }

    #[test]
    fn test_process_is_not_packaged() {
        assert_eq!(current_package_family_name(), None);
    }
}

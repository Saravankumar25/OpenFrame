//! Platform boundary for OpenFrame Studio (build plan area "Platform adapter layer for Windows and Mac").
//!
//! This crate is the only place outside the desktop shell (`apps/desktop/src-tauri`) that may
//! know which operating system OpenFrame runs on. Feature code calls these functions and never
//! writes `cfg(windows)`, `cfg(target_os = ...)`, `std::os::windows`, `std::os::unix` or the
//! `windows` and `libc` crates itself. The architecture lint (`scripts/architecture-lint.mjs`,
//! rule R6) enforces that.
//!
//! Two adapters live here, selected at compile time:
//!
//! - **Windows adapter:** `%LOCALAPPDATA%\OpenFrame`, `Documents\OpenFrame`, suspended creation
//!   into one-process kill-on-close Job Objects for managed child processes, `GetFileAttributesW`
//!   existence checks, MSIX package identity, `Ctrl` key labels, Windows spelling dictionaries,
//!   mapped network drive detection, the long-path SQLite form, sleep and wake notifications, and
//!   open or reveal through the shell.
//! - **macOS adapter:** `~/Library/Application Support/OpenFrame`, `~/Documents/OpenFrame`,
//!   process groups for managed child processes, `Command` key labels, `open` and `open -R`.
//!   Parts that need Mac hardware or the App Sandbox to verify are marked in each module.
//!
//! Behaviour that is the same on both systems (atomic file replace, temp names, cleanup rules) is
//! written once and tested once.

pub mod atomic;
pub mod cloud;
pub mod dirs;
pub mod drives;
pub mod exists;
pub mod identity;
pub mod keys;
pub mod paths;
pub mod power;
pub mod process;
pub mod shell;
pub mod spelling;

/// The operating system family this build runs on. Feature code should not branch on this; it
/// exists for diagnostics, logging and the few UI labels that differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    Other,
}

/// The operating system family of this build.
pub const fn current_os() -> Os {
    if cfg!(windows) {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::MacOs
    } else {
        Os::Other
    }
}

impl Os {
    /// Short name for diagnostics files and logs.
    pub const fn name(self) -> &'static str {
        match self {
            Os::Windows => "windows",
            Os::MacOs => "macos",
            Os::Other => "other",
        }
    }
}

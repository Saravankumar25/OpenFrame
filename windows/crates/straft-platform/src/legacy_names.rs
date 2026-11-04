//! The names the product used before it became Straft Studio, on disk and in files people already
//! have. Readers accept them so nobody loses data after the update; nothing new is ever written
//! under them. Every use of the old name in the code base goes through this module, except the few
//! lines marked `LEGACY` where a text pattern has to spell it out.

/// The product name, also the folder name of the app data, of the library pointer and
/// of the default library (`Documents\<name>`, `<home>\<name>`).
// LEGACY: Open Frame name kept for migration
pub const PRODUCT_NAME: &str = "OpenFrame";

/// The extension of a project folder (`<title>.<this>`).
// LEGACY: Open Frame name kept for migration
pub const PROJECT_EXTENSION: &str = "openframe";

/// The manifest file of a project folder, also inside older packages.
// LEGACY: Open Frame name kept for migration
pub const MANIFEST_FILE: &str = "openframe.json";

/// The `format` field of a project manifest.
// LEGACY: Open Frame name kept for migration
pub const MANIFEST_FORMAT: &str = "openframe-project";

/// The `format` field of a package manifest (`.ofproject`, `.ofbackup`, `.ofstory` and the rest).
// LEGACY: Open Frame name kept for migration
pub const PACKAGE_FORMAT: &str = "openframe-package";

/// The `format` field of a Global Idea Vault archive (`.ofvault`).
// LEGACY: Open Frame name kept for migration
pub const VAULT_FORMAT: &str = "openframe-vault";

/// The id of the default theme, stored in `sys_appearance_pref` and in exported theme files.
// LEGACY: Open Frame name kept for migration
pub const DEFAULT_THEME_ID: &str = "openframe-default";

/// The start of a daily log file name, before the date.
// LEGACY: Open Frame name kept for migration
pub const LOG_FILE_PREFIX: &str = "openframe.";

/// The start of a log line's target (the crate name), in log files of earlier versions.
// LEGACY: Open Frame name kept for migration
pub const LOG_TARGET_PREFIX: &str = "openframe";

/// The suffix of a staging file a crash left behind.
// LEGACY: Open Frame name kept for migration
pub const TEMP_SUFFIX: &str = ".openframe-tmp";

/// The marker in the name of a staging folder an interrupted move left behind.
// LEGACY: Open Frame name kept for migration
pub const STAGING_MARKER: &str = ".openframe-staging-";

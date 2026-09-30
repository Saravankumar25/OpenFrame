//! Stable error taxonomy (ESD §12).
//!
//! Every backend error carries a stable machine code (e.g. `storage.disk_full`)
//! used by the frontend for behaviour, a filmmaker-facing message (never raw
//! SQLite/OS text), optional technical detail for logs/diagnostics, and a
//! retry hint. The frontend must branch on `code`, never on `message`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A stable dotted error code such as `validation.required` or `conflict.stale`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ErrorCode(pub String);

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, thiserror::Error)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
#[error("{code}: {message}")]
pub struct AppError {
    pub code: ErrorCode,
    /// Human, filmmaker-oriented explanation. Safe to show in the UI.
    pub message: String,
    /// Technical detail for diagnostics. Callers must not put project content here.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub detail: Option<String>,
    pub retryable: bool,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode(code.into()),
            message: message.into(),
            detail: None,
            retryable: false,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }

    pub fn code_str(&self) -> &str {
        &self.code.0
    }

    /// True if the code equals `prefix` or is nested beneath it (`storage` matches `storage.disk_full`).
    pub fn is(&self, prefix: &str) -> bool {
        self.code.0 == prefix
            || (self.code.0.starts_with(prefix) && self.code.0[prefix.len()..].starts_with('.'))
    }

    // ---- validation.* -------------------------------------------------------
    pub fn validation(field: &str, message: impl Into<String>) -> Self {
        Self::new(format!("validation.{field}"), message)
    }
    pub fn required(what: &str) -> Self {
        Self::new("validation.required", format!("{what} is required."))
    }
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new("validation.invalid_input", message)
    }

    // ---- permission.* -------------------------------------------------------
    pub fn permission_denied(action: &str) -> Self {
        Self::new(
            "permission.denied",
            format!("You don't have permission to {action} in this project."),
        )
    }
    pub fn private_content() -> Self {
        Self::new(
            "permission.private",
            "The requested information is private and cannot be accessed in this context.",
        )
    }
    pub fn locked_draft() -> Self {
        Self::new(
            "permission.locked_draft",
            "This draft is locked as the shooting draft. Start a revision to make changes.",
        )
    }

    // ---- not_found.* --------------------------------------------------------
    pub fn not_found(what: &str) -> Self {
        Self::new(
            format!("not_found.{}", what.to_lowercase().replace(' ', "_")),
            format!("That {what} could not be found. It may have been deleted."),
        )
    }
    pub fn no_project_open() -> Self {
        Self::new("not_found.project_not_open", "No project is open.")
    }

    // ---- conflict.* ---------------------------------------------------------
    pub fn stale(what: &str) -> Self {
        Self::new(
            "conflict.stale",
            format!(
                "This {what} was changed by someone else. Review the latest version and try again."
            ),
        )
        .retryable()
    }
    pub fn undo_conflict() -> Self {
        Self::new(
            "conflict.undo",
            "This can't be undone because the same content was changed afterwards.",
        )
    }
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new("conflict.state", message)
    }

    // ---- storage.* ----------------------------------------------------------
    pub fn storage(detail: impl Into<String>) -> Self {
        Self::new(
            "storage.write_failed",
            "OpenFrame could not save this change yet. Your work is still open and has not been discarded.",
        )
        .with_detail(detail)
        .retryable()
    }
    pub fn disk_full() -> Self {
        Self::new(
            "storage.disk_full",
            "The drive is full, so OpenFrame could not save this change. Free up some space and try again — your work is still open.",
        )
        .retryable()
    }
    pub fn busy() -> Self {
        Self::new(
            "storage.busy",
            "OpenFrame could not save this change yet. Your work is still open and has not been discarded.",
        )
        .retryable()
    }
    pub fn drive_unavailable() -> Self {
        Self::new(
            "storage.drive_unavailable",
            "The drive holding this project is no longer available. Reconnect it to keep saving — OpenFrame will not create a separate copy.",
        )
        .retryable()
    }
    pub fn access_denied(detail: impl Into<String>) -> Self {
        Self::new(
            "storage.access_denied",
            "OpenFrame doesn't have permission to write to this location. Choose a different folder or check its permissions.",
        )
        .with_detail(detail)
    }

    // ---- project_format.* / migration.* ------------------------------------
    pub fn project_format(message: impl Into<String>) -> Self {
        Self::new("project_format.invalid", message)
    }
    pub fn migration(detail: impl Into<String>) -> Self {
        Self::new(
            "migration.failed",
            "OpenFrame couldn't upgrade this project. The original project was left untouched and a safety backup was kept.",
        )
        .with_detail(detail)
    }

    // ---- import.* / export.* -----------------------------------------------
    pub fn import(code: &str, message: impl Into<String>) -> Self {
        Self::new(format!("import.{code}"), message)
    }
    pub fn export(code: &str, message: impl Into<String>) -> Self {
        Self::new(format!("export.{code}"), message)
    }

    // ---- ai.* / network.* / security.* / internal.* -------------------------
    pub fn ai(code: &str, message: impl Into<String>) -> Self {
        Self::new(format!("ai.{code}"), message)
    }
    pub fn network(detail: impl Into<String>) -> Self {
        Self::new(
            "network.unavailable",
            "OpenFrame couldn't reach the internet. Everything else keeps working offline.",
        )
        .with_detail(detail)
        .retryable()
    }
    pub fn security(code: &str, message: impl Into<String>) -> Self {
        Self::new(format!("security.{code}"), message)
    }
    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(
            "internal.unexpected",
            "Something went wrong inside OpenFrame. Your project was not changed.",
        )
        .with_detail(detail)
    }
    pub fn cancelled() -> Self {
        Self::new("internal.cancelled", "The operation was cancelled.")
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind as K;
        // Windows: 21 ERROR_NOT_READY, 55 ERROR_DEV_NOT_EXIST, 1167 ERROR_DEVICE_NOT_CONNECTED,
        // 112 ERROR_DISK_FULL, 39 ERROR_HANDLE_DISK_FULL.
        match (err.kind(), err.raw_os_error()) {
            (_, Some(21 | 55 | 1167)) => AppError::drive_unavailable().with_detail(err.to_string()),
            (_, Some(112 | 39)) | (K::StorageFull | K::QuotaExceeded, _) => {
                AppError::disk_full().with_detail(err.to_string())
            }
            (K::PermissionDenied, _) => AppError::access_denied(err.to_string()),
            (K::NotFound, _) => AppError::new(
                "not_found.file",
                "A file OpenFrame expected could not be found.",
            )
            .with_detail(err.to_string()),
            _ => AppError::storage(err.to_string()),
        }
    }
}

#[cfg(feature = "sqlite")]
impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        use rusqlite::ffi::ErrorCode as C;
        let detail = err.to_string();
        match &err {
            rusqlite::Error::SqliteFailure(e, _) => match e.code {
                C::DiskFull => AppError::disk_full().with_detail(detail),
                C::DatabaseBusy | C::DatabaseLocked => AppError::busy().with_detail(detail),
                C::ReadOnly | C::PermissionDenied => AppError::access_denied(detail),
                C::DatabaseCorrupt | C::NotADatabase => AppError::new(
                    "project_format.corrupt",
                    "This project's data file is damaged. OpenFrame will not modify it. Restore from a backup or recovery point.",
                )
                .with_detail(detail),
                C::CannotOpen => AppError::drive_unavailable().with_detail(detail),
                C::SystemIoFailure => AppError::storage(detail),
                C::ConstraintViolation => AppError::conflict(
                    "That change conflicts with related project data and was not saved.",
                )
                .with_detail(detail),
                _ => AppError::storage(detail),
            },
            rusqlite::Error::QueryReturnedNoRows => AppError::new("not_found.record", "That item could not be found. It may have been deleted.")
                .with_detail(detail),
            _ => AppError::internal(detail),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::invalid_input("The request was not in the expected format.")
            .with_detail(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_matching() {
        let e = AppError::disk_full();
        assert!(e.is("storage"));
        assert!(e.is("storage.disk_full"));
        assert!(!e.is("stor"));
        assert!(e.retryable);
    }

    #[test]
    fn io_errors_map_to_human_messages() {
        let e: AppError = std::io::Error::from(std::io::ErrorKind::PermissionDenied).into();
        assert_eq!(e.code_str(), "storage.access_denied");
        let full: AppError = std::io::Error::from_raw_os_error(112).into();
        assert_eq!(full.code_str(), "storage.disk_full");
        let gone: AppError = std::io::Error::from_raw_os_error(21).into();
        assert_eq!(gone.code_str(), "storage.drive_unavailable");
    }
}

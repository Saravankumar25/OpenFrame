//! Writing comfort (build plan area "Writing comfort"; features SCRIPT-017 to SCRIPT-020).
//!
//! - `suggestions`: project values for the editor's smart suggestion list (Story Board
//!   characters, cast, production locations, Story Board scene headings). Values typed in the
//!   script itself are read by the editor from the live document, so unsaved typing counts too.
//! - `navigator`: optional scene navigator columns (page, notes, revision, production number).
//! - `proofing`: project spelling languages, the on and off switch, checking text against the
//!   operating system's dictionaries, and the personal dictionary.
//! - `shortcuts`: custom keyboard shortcuts (stored overrides of the built-in defaults).
//!
//! None of this uses an AI model. Personal preferences (shortcuts, personal dictionary) live in
//! the application database, never in a project.

pub mod navigator;
pub mod proofing;
pub mod shortcuts;
pub mod suggestions;

use crate::registry::Registry;

pub fn register(r: &mut Registry) {
    r.module("Writing");
    suggestions::register(r);
    navigator::register(r);
    proofing::register(r);
    shortcuts::register(r);
}

/// Serialises read-modify-write changes of personal preferences (the personal dictionary and
/// keyboard shortcuts), so two quick changes can never lose one of them.
pub(crate) fn settings_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Read a JSON value from the application settings table.
pub(crate) fn read_app_setting(
    core: &crate::core::AppCore,
    key: &str,
) -> openframe_domain::AppResult<Option<serde_json::Value>> {
    use rusqlite::OptionalExtension;
    core.with_app_db(|c| {
        let raw: Option<String> = c
            .query_row(
                "SELECT value_json FROM sys_settings WHERE key=?1",
                [key],
                |r| r.get(0),
            )
            .optional()?;
        // A damaged value reads as "not set" so the defaults apply; it is replaced on the next save.
        Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
    })
}

/// Write (or with `None`, remove) a JSON value in the application settings table.
pub(crate) fn write_app_setting(
    core: &crate::core::AppCore,
    key: &str,
    value: Option<&serde_json::Value>,
) -> openframe_domain::AppResult<()> {
    core.with_app_db(|c| {
        match value {
            Some(v) => {
                c.execute(
                    "INSERT INTO sys_settings(key, value_json, updated_at) VALUES (?1, ?2, ?3)
                     ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
                    rusqlite::params![key, v.to_string(), openframe_domain::now_ms()],
                )?;
            }
            None => {
                c.execute("DELETE FROM sys_settings WHERE key=?1", [key])?;
            }
        }
        Ok(())
    })
}

/// Personal preferences belong to the person at this computer, never to someone whose changes
/// arrive in a package.
pub(crate) fn require_local(
    actor: &openframe_domain::Actor,
    what: &str,
) -> openframe_domain::AppResult<()> {
    if !matches!(actor.origin, openframe_domain::auth::ActorOrigin::Local) {
        return Err(openframe_domain::AppError::new(
            "permission.denied",
            format!("Only the person using this computer can change {what}."),
        ));
    }
    Ok(())
}

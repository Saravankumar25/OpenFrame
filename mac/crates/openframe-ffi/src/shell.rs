//! Opening and revealing files for the Mac shell.
//!
//! The same rules as the Windows adapter (`apps/desktop/src-tauri/src/lib.rs`, commands
//! `of_open_asset`, `of_reveal_path`, `of_open_url`): paths are resolved by the core from ids, the
//! shell never passes an arbitrary path, the store lock and the team gate are checked first, and
//! files that would run as programs are shown in Finder instead of opened.
//!
//! The Windows adapter is frozen with the proved Windows build (handoff, W10), so these rules
//! live here as a second adapter instead of being moved into a shared module now. Moving both
//! adapters onto one shared module is a follow-up after W10 closes (Mac parity matrix notes).

use std::path::{Path, PathBuf};

use openframe_application::{AppCore, StoreKind};
use openframe_domain::{AppError, AppResult};

fn store_kind(global: bool) -> StoreKind {
    if global {
        StoreKind::Global
    } else {
        StoreKind::Project
    }
}

fn resolve_asset(core: &AppCore, asset_id: &str, global: bool) -> AppResult<PathBuf> {
    core.with_store(store_kind(global), |s| {
        let root = s.root().to_path_buf();
        s.read(|c| openframe_application::util::asset_file_path(c, &root, asset_id))
    })
}

fn unavailable() -> AppError {
    AppError::new(
        "not_found.file",
        "This file isn't available right now. If it's on an external drive, reconnect it or relink the file.",
    )
}

/// The managed folders previews may read: the open project's `assets/` and the Global Idea
/// Vault's `assets/`. Databases, `cache/` and recovery data are never answered.
fn allowed_asset_roots(core: &AppCore) -> Vec<PathBuf> {
    let mut roots = vec![core.config.global_vault_dir.join("assets")];
    if let Some(p) = core.project_opt() {
        roots.push(p.layout.assets());
    }
    roots
}

fn inside_any(path: &Path, roots: &[PathBuf]) -> bool {
    let Ok(real) = path.canonicalize() else {
        return false;
    };
    roots.iter().any(|root| {
        root.canonicalize()
            .map(|r| real.starts_with(&r))
            .unwrap_or(false)
    })
}

pub fn asset_file(core: &AppCore, asset_id: &str, global: bool) -> AppResult<PathBuf> {
    core.entitlement.check_operation("shell.open_asset")?;
    let path = resolve_asset(core, asset_id, global)?;
    if !path.is_file() {
        return Err(unavailable());
    }
    // A linked file outside the managed folders is opened by its app, never read for a preview.
    if !inside_any(&path, &allowed_asset_roots(core)) {
        return Err(AppError::not_found("file"));
    }
    Ok(path)
}

pub fn open_asset(core: &AppCore, asset_id: &str, global: bool, reveal: bool) -> AppResult<()> {
    core.entitlement.check_operation("shell.open_asset")?;
    let path = resolve_asset(core, asset_id, global)?;
    if !openframe_security::is_local_disk_path(&path) || !path.exists() {
        return Err(unavailable());
    }
    // Files from received packages can carry attacker-chosen names: anything that would run is
    // shown in Finder instead of opened (PKG-01).
    let reveal = reveal || openframe_security::is_dangerous_to_open(&path);
    open_path(&path, reveal)
}

pub fn reveal_location(
    core: &AppCore,
    kind: &str,
    id: Option<String>,
    path: Option<String>,
) -> AppResult<()> {
    // Before the team password nothing is reachable; under the store lock only the data folder
    // and the logs, which support needs (as on Windows).
    if core.entitlement.team_gate_locked() || !matches!(kind, "dataRoot" | "logs") {
        core.entitlement.check_operation("shell.reveal_path")?;
    }
    let target: PathBuf = match kind {
        "project" => match id {
            Some(pid) => core.with_app_db(|c| {
                Ok(PathBuf::from(c.query_row(
                    "SELECT path FROM sys_recent_project WHERE project_id=?1",
                    [pid],
                    |r| r.get::<_, String>(0),
                )?))
            })?,
            None => core.project()?.layout.root().to_path_buf(),
        },
        "globalVault" => core.config.global_vault_dir.clone(),
        "dataRoot" => {
            let dir = core.config.data_root();
            std::fs::create_dir_all(&dir).map_err(|e| {
                AppError::new(
                    "internal.open_failed",
                    "OpenFrame couldn't open that folder.",
                )
                .with_detail(e.to_string())
            })?;
            dir
        }
        "assetFolder" | "globalAssetFolder" => {
            let asset_id = id.unwrap_or_default();
            let file = resolve_asset(core, &asset_id, kind == "globalAssetFolder")?;
            let dir = file
                .parent()
                .filter(|d| openframe_security::is_local_disk_path(d) && d.is_dir())
                .ok_or_else(|| AppError::not_found("folder"))?;
            return open_path(dir, false);
        }
        "logs" => core.config.app_data_dir.join("logs"),
        // Only files OpenFrame itself wrote this session to a place the user chose (PATH-01).
        "exported" => {
            let raw = path.unwrap_or_default();
            let p = openframe_security::user_path(&raw, openframe_security::NetworkPaths::Allow)
                .map_err(|_| AppError::not_found("file"))?;
            if !openframe_application::util::is_remembered_output(core, &p) || !p.exists() {
                return Err(AppError::not_found("file"));
            }
            p
        }
        _ => return Err(AppError::invalid_input("Unknown location.")),
    };
    open_path(&target, true)
}

pub fn open_url(url: &str) -> AppResult<()> {
    let url = openframe_security::web_link(url)
        .ok_or_else(|| AppError::invalid_input("Only web links (http or https) can be opened."))?;
    openframe_platform::shell::open_url(url).map_err(|e| {
        AppError::new("internal.open_failed", "OpenFrame couldn't open that link.")
            .with_detail(e.to_string())
    })
}

fn open_path(path: &Path, reveal: bool) -> AppResult<()> {
    let r = if reveal {
        openframe_platform::shell::reveal_path(path)
    } else {
        openframe_platform::shell::open_path(path)
    };
    r.map_err(|e| {
        AppError::new("internal.open_failed", "OpenFrame couldn't open that file.")
            .with_detail(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_links_open() {
        assert!(open_url("file:///etc/passwd").is_err());
        assert!(open_url("javascript:alert(1)").is_err());
        assert!(open_url("smb://server/share").is_err());
    }

    #[test]
    fn a_path_outside_the_managed_folders_is_not_inside() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        let inside = assets.join("a.png");
        std::fs::write(&inside, b"x").unwrap();
        let outside = dir.path().join("project.sqlite");
        std::fs::write(&outside, b"x").unwrap();
        let roots = vec![assets.clone()];
        assert!(inside_any(&inside, &roots));
        assert!(!inside_any(&outside, &roots));
        // A symbolic link inside the folder that points out of it is refused too.
        let link = assets.join("escape.png");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert!(!inside_any(&link, &roots));
        // A missing file is never inside.
        assert!(!inside_any(&assets.join("missing.png"), &roots));
    }
}

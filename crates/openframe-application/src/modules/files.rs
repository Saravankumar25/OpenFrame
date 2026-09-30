//! Project Files cabinet (FSD §40, §112) and generic asset lifecycle helpers.

use std::path::PathBuf;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{next_position, opt_text, text, update_fields};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{
    AssetInfo, ingest_file, load_asset, optional_text, reference_external, required_text,
};

pub fn register(r: &mut Registry) {
    r.query("files.list", list);
    r.command("files.add", add);
    r.command("files.rename", rename);
    r.command("files.move", move_file);
    r.command("files.update_notes", update_notes);
    r.command("files.delete", delete);
    r.command("files.relink", relink);
    r.command("files.create_folder", create_folder);
    r.command("files.rename_folder", rename_folder);
    r.command("files.delete_folder", delete_folder);
    r.query("files.asset", asset);
    r.command("files.export_copy", export_copy);
    r.indexer("project_file", index_file);
    r.trash_handler(TrashHandler {
        object_type: "project_file",
        table: "project_file",
        label: "File",
        restore: Some(restore_file),
        purge: purge_file,
    });
    r.trash_handler(TrashHandler {
        object_type: "project_file_folder",
        table: "project_file_folder",
        label: "Folder",
        restore: None,
        purge: purge_folder,
    });
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFileDto {
    pub id: String,
    pub display_name: String,
    pub folder_id: Option<String>,
    pub notes: Option<String>,
    pub source: Option<String>,
    pub asset: AssetInfo,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FolderDto {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FilesListing {
    pub folders: Vec<FolderDto>,
    pub files: Vec<ProjectFileDto>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddFilesArgs {
    pub paths: Vec<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
    /// "copy" (store inside the project) or "link" (external reference).
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "copy".into()
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameArgs {
    pub id: String,
    pub name: String,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveArgs {
    pub id: String,
    pub folder_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NotesArgs {
    pub id: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelinkArgs {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FolderArgs {
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetArgs {
    pub asset_id: String,
    #[serde(default)]
    pub store: crate::util::StoreSel,
}

type FileRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    i64,
    i64,
    i64,
);

fn load_file(c: &Connection, root: &std::path::Path, id: &str) -> AppResult<ProjectFileDto> {
    let row: Option<FileRow> = c
        .query_row(
            "SELECT id, display_name, folder_id, notes, source, asset_id, created_at, updated_at, rev
             FROM project_file WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
        )
        .optional()?;
    let (id, display_name, folder_id, notes, source, asset_id, created_at, updated_at, rev) =
        row.ok_or_else(|| AppError::not_found("file"))?;
    Ok(ProjectFileDto {
        id,
        display_name,
        folder_id,
        notes,
        source,
        asset: load_asset(c, root, &asset_id)?,
        created_at,
        updated_at,
        rev,
    })
}

fn list(core: &AppCore, actor: &Actor, _: ListArgs) -> AppResult<FilesListing> {
    actor.require(Capability::View, "view project files")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| {
        let mut fs = c.prepare(
            "SELECT id, name, parent_id, rev FROM project_file_folder WHERE deleted_at IS NULL ORDER BY position, name",
        )?;
        let folders = fs
            .query_map([], |r| Ok(FolderDto { id: r.get(0)?, name: r.get(1)?, parent_id: r.get(2)?, rev: r.get(3)? }))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut stmt = c.prepare("SELECT id FROM project_file WHERE deleted_at IS NULL ORDER BY position, display_name")?;
        let ids: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        let files = ids.iter().map(|id| load_file(c, &root, id)).collect::<AppResult<Vec<_>>>()?;
        Ok(FilesListing { folders, files })
    })
}

fn check_folder(c: &Connection, folder_id: &Option<String>) -> AppResult<()> {
    if let Some(f) = folder_id {
        let ok: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM project_file_folder WHERE id=?1 AND deleted_at IS NULL)",
            [f],
            |r| r.get(0),
        )?;
        if !ok {
            return Err(AppError::not_found("folder"));
        }
    }
    Ok(())
}

/// Insert a Project File row for an already-ingested asset (used by other modules,
/// e.g. keeping an imported screenplay source file — FSD §19.9).
pub fn add_file_record(
    tx: &Tx<'_>,
    asset: &AssetInfo,
    folder_id: Option<&str>,
    source: Option<&str>,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    let pos = next_position(
        tx.conn(),
        "project_file",
        "folder_id IS ?1",
        &[opt_text(folder_id)],
    )?;
    tx.conn().execute(
        "INSERT INTO project_file(id, display_name, folder_id, asset_id, source, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![id, asset.original_name, folder_id, asset.id, source, pos, now],
    )?;
    Ok(id)
}

fn add(core: &AppCore, actor: &Actor, args: AddFilesArgs) -> AppResult<Vec<ProjectFileDto>> {
    if args.paths.is_empty() {
        return Ok(vec![]);
    }
    if args.paths.len() > 500 {
        return Err(AppError::invalid_input("Add at most 500 files at a time."));
    }
    let link = match args.mode.as_str() {
        "copy" => false,
        "link" => true,
        _ => return Err(AppError::invalid_input("Unknown storage mode.")),
    };
    let s = core.project()?;
    let summary = if args.paths.len() == 1 {
        format!(
            "Added file “{}”",
            PathBuf::from(&args.paths[0])
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
        )
    } else {
        format!("Added {} files", args.paths.len())
    };
    let ids = s.store.mutate(
        actor,
        MutationMeta::new("files.add", summary, Capability::Edit),
        |tx| {
            check_folder(tx.conn(), &args.folder_id)?;
            let mut ids = Vec::new();
            for p in &args.paths {
                let path = PathBuf::from(p);
                let asset = if link {
                    reference_external(tx, &path)?
                } else {
                    ingest_file(tx, &path)?
                };
                ids.push(add_file_record(
                    tx,
                    &asset,
                    args.folder_id.as_deref(),
                    None,
                )?);
            }
            Ok(ids)
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store
        .read(|c| ids.iter().map(|id| load_file(c, &root, id)).collect())
}

fn rename(core: &AppCore, actor: &Actor, args: RenameArgs) -> AppResult<ProjectFileDto> {
    let name = required_text(&args.name, "Name", 255)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "files.rename",
            format!("Renamed file to “{name}”"),
            Capability::Edit,
        )
        .target("project_file", &args.id),
        |tx| {
            update_fields(
                tx.conn(),
                "project_file",
                &args.id,
                &[("display_name", text(name.clone()))],
                &["display_name"],
                args.expected_rev,
                "file",
            )?;
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_file(c, &root, &args.id))
}

fn move_file(core: &AppCore, actor: &Actor, args: MoveArgs) -> AppResult<ProjectFileDto> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("files.move", "Moved file", Capability::Edit)
            .target("project_file", &args.id),
        |tx| {
            check_folder(tx.conn(), &args.folder_id)?;
            let pos = next_position(
                tx.conn(),
                "project_file",
                "folder_id IS ?1",
                &[opt_text(args.folder_id.clone())],
            )?;
            update_fields(
                tx.conn(),
                "project_file",
                &args.id,
                &[
                    ("folder_id", opt_text(args.folder_id.clone())),
                    ("position", openframe_persistence::rows::int(pos)),
                ],
                &["folder_id", "position"],
                None,
                "file",
            )?;
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_file(c, &root, &args.id))
}

fn update_notes(core: &AppCore, actor: &Actor, args: NotesArgs) -> AppResult<ProjectFileDto> {
    let notes = optional_text(args.notes, "Notes", 20_000)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("files.update_notes", "Edited file notes", Capability::Edit)
            .target("project_file", &args.id)
            .coalesce(format!("files.notes:{}", args.id)),
        |tx| {
            update_fields(
                tx.conn(),
                "project_file",
                &args.id,
                &[("notes", opt_text(notes.clone()))],
                &["notes"],
                None,
                "file",
            )?;
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_file(c, &root, &args.id))
}

fn delete(core: &AppCore, actor: &Actor, args: IdArgs) -> AppResult<()> {
    let s = core.project()?;
    let (name, folder, pos): (String, Option<String>, i64) = s.store.read(|c| {
        c.query_row(
            "SELECT display_name, folder_id, position FROM project_file WHERE id=?1",
            [&args.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("file"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "files.delete",
            format!("Deleted file “{name}”"),
            Capability::SoftDelete,
        )
        .target("project_file", &args.id),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "project_file",
                    table: "project_file",
                    id: &args.id,
                    title: Some(name.clone()),
                    parent_type: Some("project_file_folder"),
                    parent_id: folder.clone(),
                    position: Some(pos),
                },
            )
        },
    )
}

fn relink(core: &AppCore, actor: &Actor, args: RelinkArgs) -> AppResult<ProjectFileDto> {
    let s = core.project()?;
    s.store.mutate(actor, MutationMeta::new("files.relink", "Relinked file", Capability::Edit).target("project_file", &args.id), |tx| {
        let asset_id: String =
            tx.conn().query_row("SELECT asset_id FROM project_file WHERE id=?1", [&args.id], |r| r.get(0))?;
        let mode: String = tx.conn().query_row("SELECT storage_mode FROM asset WHERE id=?1", [&asset_id], |r| r.get(0))?;
        if mode != "external" {
            return Err(AppError::invalid_input("Only linked files can be relinked."));
        }
        let (abs, len) = crate::util::external_link_target(&PathBuf::from(args.path.trim()))?;
        let abs = abs.to_string_lossy().into_owned();
        // Relinking keeps the same logical asset identity (Domain §22).
        tx.conn().execute(
            "UPDATE asset SET external_path=?1, byte_size=?2, last_seen_at=?3, updated_at=?3, rev=rev+1 WHERE id=?4",
            params![abs, len as i64, now_ms(), asset_id],
        )?;
        tx.reindex("project_file", &args.id);
        Ok(())
    })?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_file(c, &root, &args.id))
}

fn create_folder(core: &AppCore, actor: &Actor, args: FolderArgs) -> AppResult<FolderDto> {
    let name = required_text(&args.name, "Folder name", 120)?;
    let s = core.project()?;
    let id = new_id();
    s.store.mutate(actor, MutationMeta::new("files.create_folder", format!("Created folder “{name}”"), Capability::Edit), |tx| {
        check_folder(tx.conn(), &args.parent_id)?;
        let now = now_ms();
        let pos = next_position(tx.conn(), "project_file_folder", "parent_id IS ?1", &[opt_text(args.parent_id.clone())])?;
        tx.conn().execute(
            "INSERT INTO project_file_folder(id, name, parent_id, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, name, args.parent_id, pos, now],
        )?;
        Ok(())
    })?;
    Ok(FolderDto {
        id,
        name,
        parent_id: args.parent_id,
        rev: 1,
    })
}

fn rename_folder(core: &AppCore, actor: &Actor, args: RenameArgs) -> AppResult<()> {
    let name = required_text(&args.name, "Folder name", 120)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "files.rename_folder",
            format!("Renamed folder to “{name}”"),
            Capability::Edit,
        ),
        |tx| {
            update_fields(
                tx.conn(),
                "project_file_folder",
                &args.id,
                &[("name", text(name.clone()))],
                &["name"],
                args.expected_rev,
                "folder",
            )?;
            Ok(())
        },
    )
}

/// Deleting a folder never deletes its files: they move to the top level first (FSD §52.3).
fn delete_folder(core: &AppCore, actor: &Actor, args: IdArgs) -> AppResult<()> {
    let s = core.project()?;
    let name: String = s.store.read(|c| {
        c.query_row(
            "SELECT name FROM project_file_folder WHERE id=?1 AND deleted_at IS NULL",
            [&args.id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("folder"))
    })?;
    s.store.mutate(actor, MutationMeta::new("files.delete_folder", format!("Deleted folder “{name}”"), Capability::SoftDelete), |tx| {
        let now = now_ms();
        tx.conn().execute(
            "UPDATE project_file SET folder_id=NULL, updated_at=?1, rev=rev+1 WHERE folder_id=?2",
            params![now, args.id],
        )?;
        tx.conn().execute(
            "UPDATE project_file_folder SET parent_id=NULL, updated_at=?1, rev=rev+1 WHERE parent_id=?2",
            params![now, args.id],
        )?;
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "project_file_folder",
                table: "project_file_folder",
                id: &args.id,
                title: Some(name.clone()),
                parent_type: None,
                parent_id: None,
                position: None,
            },
        )
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportCopyArgs {
    pub id: String,
    /// Destination chosen by the user in a save dialog.
    pub dest_path: String,
}

/// "Export/copy" (FSD §40.2): write a copy of a project file to a location the
/// user chose. The project and its stored file are not changed.
fn export_copy(core: &AppCore, actor: &Actor, args: ExportCopyArgs) -> AppResult<String> {
    actor.require(Capability::Export, "export files")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    let src = s.store.read(|c| {
        let asset_id: String = c
            .query_row(
                "SELECT asset_id FROM project_file WHERE id=?1 AND deleted_at IS NULL",
                [&args.id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("file"))?;
        crate::util::asset_file_path(c, &root, &asset_id)
    })?;
    if !src.is_file() {
        return Err(AppError::new(
            "not_found.file",
            "This file isn't available right now. If it's on an external drive, reconnect it or relink the file.",
        ));
    }
    let dest = PathBuf::from(args.dest_path.trim());
    if !dest.is_absolute() || dest.file_name().is_none() {
        return Err(AppError::invalid_input("Choose where to save the copy."));
    }
    if dest.parent().map(|p| !p.is_dir()).unwrap_or(true) {
        return Err(AppError::not_found("folder"));
    }
    // Never into the project/app-data/vault folders (it could overwrite openframe.json,
    // recovery data or managed assets), no device/ADS/reserved names (PATH-02).
    crate::util::check_output_file(core, &dest)?;
    let same = match (src.canonicalize(), dest.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if same {
        return Err(AppError::invalid_input(
            "Choose a different location for the copy.",
        ));
    }
    // Copy to a temporary sibling first so a failed copy never leaves a half-written file.
    // Unique temporary name: `with_extension` would clobber a sibling such as `a.openframe-part`.
    let tmp = dest.with_file_name(format!(
        ".{}.{}.openframe-part",
        dest.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        openframe_domain::new_id()
    ));
    if let Err(e) = std::fs::copy(&src, &tmp).and_then(|_| std::fs::rename(&tmp, &dest)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(AppError::from(e));
    }
    Ok(dest.to_string_lossy().into_owned())
}

fn asset(core: &AppCore, actor: &Actor, args: AssetArgs) -> AppResult<AssetInfo> {
    actor.require(Capability::View, "view files")?;
    crate::util::with_store(core, args.store, |s| {
        let root = s.root().to_path_buf();
        s.read(|c| load_asset(c, &root, &args.asset_id))
    })
}

fn index_file(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, String, Option<i64>)> = c
        .query_row(
            "SELECT f.display_name, f.notes, a.original_name, f.deleted_at FROM project_file f JOIN asset a ON a.id=f.asset_id WHERE f.id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    Ok(match row {
        Some((name, notes, original, None)) => Some(SearchDoc {
            entity_type: "project_file".into(),
            title: name,
            body: format!("{} {}", original, notes.unwrap_or_default()),
            context: "Files".into(),
            nav: json!({ "workspace": "files", "fileId": id }),
            owner_user_id: None,
        }),
        _ => None,
    })
}

fn restore_file(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    // Return to the original folder if it still exists; otherwise the top level.
    let folder_ok = match &row.parent_id {
        Some(f) => tx.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM project_file_folder WHERE id=?1 AND deleted_at IS NULL)",
            [f],
            |r| r.get::<_, bool>(0),
        )?,
        None => true,
    };
    let folder = if folder_ok {
        row.parent_id.clone()
    } else {
        None
    };
    tx.conn().execute(
        "UPDATE project_file SET deleted_at=NULL, folder_id=?1, updated_at=?2, rev=rev+1 WHERE id=?3",
        params![folder, now_ms(), row.object_id],
    )?;
    Ok(())
}

fn purge_file(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let asset_id: String = tx.conn().query_row(
        "SELECT asset_id FROM project_file WHERE id=?1",
        [&row.object_id],
        |r| r.get(0),
    )?;
    tx.conn()
        .execute("DELETE FROM project_file WHERE id=?1", [&row.object_id])?;
    purge_asset_if_unreferenced(tx, &asset_id)
}

fn purge_folder(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn().execute(
        "DELETE FROM project_file_folder WHERE id=?1",
        [&row.object_id],
    )?;
    Ok(())
}

/// Remove an asset row (and its managed file, after commit) when no table still
/// references it. References are discovered generically: every column named
/// `asset_id` or ending in `_asset_id` in any table.
pub fn purge_asset_if_unreferenced(tx: &Tx<'_>, asset_id: &str) -> AppResult<()> {
    let c = tx.conn();
    let tables: Vec<String> = {
        let mut stmt = c.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'search_%' AND name <> 'asset'",
        )?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    for t in tables {
        let cols: Vec<String> = {
            let mut stmt = c.prepare(&format!(
                "SELECT name FROM pragma_table_info('{}')",
                t.replace('\'', "''")
            ))?;
            stmt.query_map([], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        };
        for col in cols
            .iter()
            .filter(|n| *n == "asset_id" || n.ends_with("_asset_id"))
        {
            let used: bool = c.query_row(
                &format!(
                    "SELECT EXISTS(SELECT 1 FROM \"{}\" WHERE \"{}\" = ?1)",
                    t.replace('"', "\"\""),
                    col.replace('"', "\"\"")
                ),
                [asset_id],
                |r| r.get(0),
            )?;
            if used {
                return Ok(());
            }
        }
    }
    let rel: Option<(String, Option<String>)> = c
        .query_row(
            "SELECT storage_mode, rel_path FROM asset WHERE id=?1",
            [asset_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    c.execute("DELETE FROM asset WHERE id=?1", [asset_id])?;
    if let Some(("managed", Some(rel))) = rel.as_ref().map(|(m, r)| (m.as_str(), r.clone()))
        && let Ok(p) = openframe_security::confine(tx.root(), &rel)
    {
        tx.delete_file_after_commit(p);
    }
    Ok(())
}
